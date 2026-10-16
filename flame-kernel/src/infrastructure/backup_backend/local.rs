//! 本地目录备份后端：同机自定义目录（挂载盘 / NFS 等）

use crate::core::error::AppError;
use crate::infrastructure::backup_backend_shared::spawn_blocking;
use crate::infrastructure::backup_backend_traits::BackupBackend;
use crate::infrastructure::backup_backend_traits::RemoteObject;
use std::path::{Path, PathBuf};

/// 本地目录后端。所有对象落在 `root/` 下的 `remote_key`。
pub struct LocalBackend {
    root: PathBuf,
}

impl LocalBackend {
    /// 创建本地后端。目录不存在时在首次操作时惰性创建。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, remote_key: &str) -> Result<PathBuf, AppError> {
        // 防 key 路径穿越：拒绝绝对路径与 `..` 段
        if remote_key.starts_with('/') || remote_key.split('/').any(|seg| seg == "..") {
            return Err(AppError::BadRequest(format!(
                "非法备份对象 key: {}",
                remote_key
            )));
        }
        Ok(self.root.join(remote_key))
    }
}

#[async_trait::async_trait]
impl BackupBackend for LocalBackend {
    async fn upload(&self, local: &Path, remote_key: &str) -> Result<RemoteObject, AppError> {
        let target = self.resolve(remote_key)?;
        let source = local.to_path_buf();
        let target2 = target.clone();
        let size: u64 = spawn_blocking(move || -> Result<u64, AppError> {
            if let Some(parent) = target2.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let size = std::fs::metadata(&source)?.len();
            std::fs::copy(&source, &target2)?;
            Ok(size)
        })
        .await?;
        Ok(RemoteObject {
            key: remote_key.to_string(),
            size: size as i64,
        })
    }

    async fn list(&self) -> Result<Vec<RemoteObject>, AppError> {
        let root = self.root.clone();
        spawn_blocking(move || -> Result<Vec<RemoteObject>, AppError> {
            let mut out = Vec::new();
            if !root.exists() {
                return Ok(out);
            }
            let mut stack = vec![root.clone()];
            while let Some(dir) = stack.pop() {
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(rel) = p.strip_prefix(&root) {
                        let meta = entry.metadata()?;
                        out.push(RemoteObject {
                            key: rel.to_string_lossy().replace('\\', "/"),
                            size: meta.len() as i64,
                        });
                    }
                }
            }
            Ok(out)
        })
        .await
    }

    async fn download(&self, remote_key: &str, local: &Path) -> Result<(), AppError> {
        let source = self.resolve(remote_key)?;
        let target = local.to_path_buf();
        spawn_blocking(move || -> Result<(), AppError> {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source, &target)?;
            Ok(())
        })
        .await
    }

    async fn delete(&self, remote_key: &str) -> Result<(), AppError> {
        let target = self.resolve(remote_key)?;
        spawn_blocking(move || -> Result<(), AppError> {
            if target.exists() {
                std::fs::remove_file(&target)?;
            }
            Ok(())
        })
        .await
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        let root = self.root.clone();
        spawn_blocking(move || -> Result<(), AppError> {
            std::fs::create_dir_all(&root)?;
            // 写探针：确认目录可写（只读挂载在这里暴露）
            let probe = root.join(".flamepanel-probe");
            std::fs::write(&probe, b"ok")?;
            std::fs::remove_file(&probe)?;
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn upload_list_download_delete_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let src_file = src.path().join("a.db");
        std::fs::write(&src_file, b"hello backup").unwrap();

        let backend = LocalBackend::new(tmp.path().join("remote"));
        let obj = backend.upload(&src_file, "daily/a.db").await.unwrap();
        assert_eq!(obj.key, "daily/a.db");
        assert_eq!(obj.size, 12);

        let mut keys: Vec<String> = backend
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|o| o.key)
            .collect();
        keys.sort();
        assert_eq!(keys, vec!["daily/a.db"]);

        let dst = src.path().join("restored.db");
        backend.download("daily/a.db", &dst).await.unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello backup");

        backend.delete("daily/a.db").await.unwrap();
        assert!(backend.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_traversal_key() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(tmp.path());
        let src = tempfile::tempdir().unwrap();
        let f = src.path().join("x");
        std::fs::write(&f, b"x").unwrap();
        assert!(backend.upload(&f, "../evil").await.is_err());
        assert!(backend.upload(&f, "/abs").await.is_err());
        assert!(backend.download("../evil", &f).await.is_err());
        assert!(backend.delete("a/../..//etc").await.is_err());
    }

    #[tokio::test]
    async fn test_connection_ok_on_writable_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(tmp.path().join("sub/dir"));
        backend.test_connection().await.unwrap();
    }
}
