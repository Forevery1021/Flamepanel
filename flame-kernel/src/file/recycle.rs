//! 回收站（M-C 文件管理收尾）。
//!
//! 布局：`<根>/.fp-recycle/<id>/{meta.json, data}`——与被删条目同在根目录所在的文件系统，
//! 移入 / 还原都是一次 `rename`（目录再大也是瞬时完成）。条目保留 [`RETENTION_DAYS`] 天，
//! 过期条目在下一次列出或移入时顺带清理。普通文件接口看不到、也访问不了该目录。
use super::{FileService, RECYCLE_DIR};
use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use utoipa::ToSchema;

/// 回收站保留天数。
pub const RETENTION_DAYS: i64 = 30;
const META_FILE: &str = "meta.json";
const DATA_NAME: &str = "data";
/// 统计目录体积时最多遍历的条目数（超出后停止累加，`size_truncated = true`）。
const SIZE_WALK_LIMIT: usize = 100_000;

/// 回收站条目。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecycleItem {
    pub id: String,
    pub name: String,
    /// 删除前的根内相对路径
    pub original_path: String,
    pub is_dir: bool,
    /// 字节数（目录为递归合计）
    pub size: u64,
    /// 目录条目过多时体积只统计了前一部分
    #[serde(default)]
    pub size_truncated: bool,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub deleted_by: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct Recycle {
    dir: PathBuf,
    svc: FileService,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// 递归体积（不跟随符号链接），超过遍历上限返回 truncated。
fn dir_size(path: &Path) -> (u64, bool) {
    let mut total = 0u64;
    let mut seen = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&p) else {
            continue;
        };
        for entry in rd.flatten() {
            seen += 1;
            if seen > SIZE_WALK_LIMIT {
                return (total, true);
            }
            let Ok(meta) = entry.path().symlink_metadata() else {
                continue;
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total = total.saturating_add(meta.len());
            }
        }
    }
    (total, false)
}

impl Recycle {
    pub fn new(svc: &FileService) -> Self {
        Self {
            dir: svc.root_canonical().join(RECYCLE_DIR),
            svc: svc.clone(),
        }
    }

    fn item_dir(&self, id: &str) -> Result<PathBuf, AppError> {
        if !valid_id(id) {
            return Err(AppError::BadRequest(format!("非法回收站条目: {id}")));
        }
        let dir = self.dir.join(id);
        if !dir.join(META_FILE).is_file() {
            return Err(AppError::NotFound(format!("回收站条目不存在: {id}")));
        }
        Ok(dir)
    }

    async fn read_meta(dir: &Path) -> Option<RecycleItem> {
        let raw = fs::read_to_string(dir.join(META_FILE)).await.ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// 把已校验的条目（`target` 为根内真实路径，不跟随末级链接）移入回收站。
    pub async fn move_in(
        &self,
        target: &Path,
        original_path: &str,
        deleted_by: &str,
    ) -> Result<(), AppError> {
        self.purge_expired().await;
        fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| AppError::internal(format!("创建回收站目录失败: {e}")))?;
        let meta = fs::symlink_metadata(target)
            .await
            .map_err(|e| AppError::internal(format!("读取文件信息失败: {e}")))?;
        let is_dir = meta.is_dir();
        let (size, size_truncated) = if is_dir {
            let t = target.to_path_buf();
            tokio::task::spawn_blocking(move || dir_size(&t))
                .await
                .unwrap_or((0, true))
        } else {
            (meta.len(), false)
        };
        let now = chrono::Utc::now();
        let id = format!(
            "{}-{}",
            now.timestamp_millis(),
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );
        let item_dir = self.dir.join(&id);
        fs::create_dir(&item_dir)
            .await
            .map_err(|e| AppError::internal(format!("创建回收站条目失败: {e}")))?;
        let item = RecycleItem {
            id: id.clone(),
            name: target
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            original_path: original_path.to_string(),
            is_dir,
            size,
            size_truncated,
            deleted_at: now,
            deleted_by: deleted_by.to_string(),
            expires_at: now + chrono::Duration::days(RETENTION_DAYS),
        };
        let json = serde_json::to_vec_pretty(&item)
            .map_err(|e| AppError::internal(format!("写入回收站记录失败: {e}")))?;
        if let Err(e) = fs::write(item_dir.join(META_FILE), json).await {
            let _ = fs::remove_dir_all(&item_dir).await;
            return Err(AppError::internal(format!("写入回收站记录失败: {e}")));
        }
        if let Err(e) = fs::rename(target, item_dir.join(DATA_NAME)).await {
            let _ = fs::remove_dir_all(&item_dir).await;
            // EXDEV：条目是挂载点或位于其他文件系统，rename 做不到
            return Err(if e.raw_os_error() == Some(libc::EXDEV) {
                AppError::BadRequest(
                    "该条目位于其他文件系统（挂载点），无法移入回收站，请选择永久删除".into(),
                )
            } else {
                AppError::internal(format!("移入回收站失败: {e}"))
            });
        }
        Ok(())
    }

    /// 列出条目（新删除的在前），顺带清理过期条目与残缺条目。
    pub async fn list(&self) -> Result<Vec<RecycleItem>, AppError> {
        self.purge_expired().await;
        let mut items = Vec::new();
        let Ok(mut rd) = fs::read_dir(&self.dir).await else {
            return Ok(items);
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            if let Some(item) = Self::read_meta(&entry.path()).await {
                if entry.path().join(DATA_NAME).symlink_metadata().is_ok() {
                    items.push(item);
                }
            }
        }
        items.sort_by_key(|a| std::cmp::Reverse(a.deleted_at));
        Ok(items)
    }

    /// 还原到原位置；原位置已被占用返回 409（不覆盖），父目录缺失时自动重建。
    pub async fn restore(&self, id: &str) -> Result<RecycleItem, AppError> {
        let dir = self.item_dir(id)?;
        let item = Self::read_meta(&dir)
            .await
            .ok_or_else(|| AppError::internal("回收站记录损坏"))?;
        let data = dir.join(DATA_NAME);
        if data.symlink_metadata().is_err() {
            return Err(AppError::NotFound(format!("回收站条目数据缺失: {id}")));
        }
        // 父目录可能已被删除：先在根内重建（经 sanitize_write_target 校验每一层）
        let parent_rel = Path::new(&item.original_path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/".into());
        let mut acc = String::new();
        for part in parent_rel.split('/').filter(|s| !s.is_empty()) {
            acc.push('/');
            acc.push_str(part);
            let p = self.svc.sanitize_write_target(&acc)?;
            match fs::symlink_metadata(&p).await {
                Ok(m) if m.is_dir() || p.is_dir() => {}
                Ok(_) => {
                    return Err(AppError::Conflict(format!(
                        "原位置的上级路径已被文件占用: {acc}"
                    )))
                }
                Err(_) => fs::create_dir(&p)
                    .await
                    .map_err(|e| AppError::internal(format!("重建上级目录失败: {e}")))?,
            }
        }
        let target = self.svc.sanitize_write_target(&item.original_path)?;
        if fs::symlink_metadata(&target).await.is_ok() {
            return Err(AppError::Conflict(format!(
                "原位置已存在同名条目: {}",
                item.original_path
            )));
        }
        fs::rename(&data, &target)
            .await
            .map_err(|e| AppError::internal(format!("还原失败: {e}")))?;
        let _ = fs::remove_dir_all(&dir).await;
        Ok(item)
    }

    /// 彻底删除单个条目。
    pub async fn purge(&self, id: &str) -> Result<(), AppError> {
        let dir = self.item_dir(id)?;
        fs::remove_dir_all(&dir)
            .await
            .map_err(|e| AppError::internal(format!("清除回收站条目失败: {e}")))
    }

    /// 清空回收站，返回清除的条目数。
    pub async fn purge_all(&self) -> Result<usize, AppError> {
        let mut n = 0;
        let Ok(mut rd) = fs::read_dir(&self.dir).await else {
            return Ok(0);
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            let res = if p.is_dir() {
                fs::remove_dir_all(&p).await
            } else {
                fs::remove_file(&p).await
            };
            res.map_err(|e| AppError::internal(format!("清空回收站失败: {e}")))?;
            n += 1;
        }
        Ok(n)
    }

    /// 清理过期条目与没有记录的残缺目录（尽力而为，失败忽略）。
    async fn purge_expired(&self) {
        let now = chrono::Utc::now();
        let Ok(mut rd) = fs::read_dir(&self.dir).await else {
            return;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            match Self::read_meta(&p).await {
                Some(item) if item.expires_at > now => {}
                Some(_) => {
                    let _ = fs::remove_dir_all(&p).await;
                }
                None => {
                    // 刚创建、尚未写入记录的条目可能正在移入：只清理 1 小时前的残缺目录
                    let stale = entry
                        .metadata()
                        .await
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|d| d.as_secs() > 3600);
                    if stale {
                        let _ = fs::remove_dir_all(&p).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::tests::svc;

    #[tokio::test]
    async fn delete_moves_to_recycle_and_restore_puts_it_back() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("site/css")).unwrap();
        std::fs::write(dir.path().join("site/css/a.css"), "body{}").unwrap();
        std::fs::write(dir.path().join("site/index.html"), "<h1>").unwrap();

        svc.delete("/site", true, false, "alice").await.unwrap();
        assert!(!dir.path().join("site").exists());
        let bin = Recycle::new(&svc);
        let items = bin.list().await.unwrap();
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.original_path, "/site");
        assert!(it.is_dir);
        assert_eq!(it.size, 10);
        assert_eq!(it.deleted_by, "alice");
        assert!(it.expires_at > it.deleted_at);

        bin.restore(&it.id).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("site/css/a.css")).unwrap(),
            "body{}"
        );
        assert!(bin.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn restore_rebuilds_missing_parents_and_refuses_to_overwrite() {
        let (dir, svc) = svc();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("a/b/f.txt"), "1").unwrap();
        svc.delete("/a/b/f.txt", false, false, "admin")
            .await
            .unwrap();
        svc.delete("/a", true, true, "admin").await.unwrap();

        let bin = Recycle::new(&svc);
        let id = bin.list().await.unwrap()[0].id.clone();
        // 同名条目占位 → 409
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("a/b/f.txt"), "2").unwrap();
        assert!(matches!(bin.restore(&id).await, Err(AppError::Conflict(_))));
        std::fs::remove_dir_all(dir.path().join("a")).unwrap();

        bin.restore(&id).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a/b/f.txt")).unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn purge_and_purge_all() {
        let (dir, svc) = svc();
        for n in ["x", "y", "z"] {
            std::fs::write(dir.path().join(n), n).unwrap();
            svc.delete(&format!("/{n}"), false, false, "admin")
                .await
                .unwrap();
        }
        let bin = Recycle::new(&svc);
        let items = bin.list().await.unwrap();
        assert_eq!(items.len(), 3);
        bin.purge(&items[0].id).await.unwrap();
        assert_eq!(bin.list().await.unwrap().len(), 2);
        assert_eq!(bin.purge_all().await.unwrap(), 2);
        assert!(bin.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn expired_items_are_purged_and_ids_are_validated() {
        let (dir, svc) = svc();
        std::fs::write(dir.path().join("old"), "o").unwrap();
        svc.delete("/old", false, false, "admin").await.unwrap();
        let bin = Recycle::new(&svc);
        let mut item = bin.list().await.unwrap().remove(0);
        item.expires_at = chrono::Utc::now() - chrono::Duration::seconds(1);
        std::fs::write(
            dir.path().join(RECYCLE_DIR).join(&item.id).join(META_FILE),
            serde_json::to_vec(&item).unwrap(),
        )
        .unwrap();
        assert!(bin.list().await.unwrap().is_empty());
        assert!(!dir.path().join(RECYCLE_DIR).join(&item.id).exists());

        for bad in ["../x", "a/b", "", "."] {
            assert!(bin.restore(bad).await.is_err(), "{bad:?}");
            assert!(bin.purge(bad).await.is_err(), "{bad:?}");
        }
    }
}
