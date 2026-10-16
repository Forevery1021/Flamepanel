//! SFTP 备份后端（ssh2 crate）
//!
//! ssh2 为同步库，全部网络与文件 IO 统一移入 `spawn_blocking`（F-32 纪律）。

use crate::core::error::AppError;
use crate::infrastructure::backup_backend_shared::spawn_blocking;
use crate::infrastructure::backup_backend_traits::BackupBackend;
use crate::infrastructure::backup_backend_traits::RemoteObject;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// SFTP 后端连接配置（对应账号 `config_json`）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SftpConfig {
    pub host: String,
    /// 默认 22
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    /// 密码认证（与 private_key_path 二选一）
    #[serde(default)]
    pub password: Option<String>,
    /// 私钥路径认证（可选 passphrase）
    #[serde(default)]
    pub private_key_path: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
    /// 远端备份根目录（不存在时自动逐级创建），默认 `flamepanel-backups`
    #[serde(default = "default_root")]
    pub remote_root: String,
}

fn default_port() -> u16 {
    22
}

fn default_root() -> String {
    "flamepanel-backups".to_string()
}

pub struct SftpBackend {
    cfg: SftpConfig,
}

impl SftpBackend {
    pub fn from_config(cfg: &SftpConfig) -> Self {
        Self { cfg: cfg.clone() }
    }

    /// 解析远端 key，防 `..` 穿越
    fn remote_path(&self, remote_key: &str) -> Result<PathBuf, AppError> {
        if remote_key.starts_with('/') || remote_key.split('/').any(|seg| seg == "..") {
            return Err(AppError::BadRequest(format!(
                "非法备份对象 key: {}",
                remote_key
            )));
        }
        Ok(Path::new(&self.cfg.remote_root).join(remote_key))
    }

    fn map_err(stage: &str, e: ssh2::Error) -> AppError {
        AppError::internal(format!("SFTP {} 失败: {}", stage, e))
    }

    /// 建连 + 认证 + 返回 Sftp 句柄（在阻塞线程内执行）
    fn connect_blocking(cfg: &SftpConfig) -> Result<ssh2::Sftp, AppError> {
        let addr = format!("{}:{}", cfg.host, cfg.port);
        let tcp = std::net::TcpStream::connect(&addr)
            .map_err(|e| AppError::internal(format!("SFTP 连接 {} 失败: {}", addr, e)))?;
        tcp.set_nodelay(true).ok();
        let mut session = ssh2::Session::new().map_err(|e| Self::map_err("会话初始化", e))?;
        session.set_tcp_stream(tcp);
        session.set_blocking(true);
        session.handshake().map_err(|e| Self::map_err("握手", e))?;
        if let Some(key) = &cfg.private_key_path {
            session
                .userauth_pubkey_file(
                    &cfg.username,
                    None,
                    Path::new(key),
                    cfg.passphrase.as_deref(),
                )
                .map_err(|e| Self::map_err("私钥认证", e))?;
        } else {
            let password = cfg.password.as_deref().ok_or_else(|| {
                AppError::BadRequest("SFTP 配置缺少凭据：需 password 或 private_key_path".into())
            })?;
            session
                .userauth_password(&cfg.username, password)
                .map_err(|e| Self::map_err("密码认证", e))?;
        }
        if !session.authenticated() {
            return Err(AppError::internal("SFTP 认证失败"));
        }
        session
            .sftp()
            .map_err(|e| Self::map_err("打开 SFTP 通道", e))
    }

    /// 逐级创建远端目录（对 `/a/b/c` 依次 mkdir，已存在则忽略）
    fn mkdir_p(sftp: &ssh2::Sftp, dir: &Path) -> Result<(), AppError> {
        let mut cur = PathBuf::new();
        for seg in dir.components() {
            cur.push(seg);
            if sftp.stat(&cur).is_ok() {
                continue;
            }
            // 已存在或创建成功都算通过；stat 与 mkdir 之间的竞态可忽略（备份目录场景）
            let _ = sftp.mkdir(&cur, 0o755);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl BackupBackend for SftpBackend {
    async fn upload(&self, local: &Path, remote_key: &str) -> Result<RemoteObject, AppError> {
        let cfg = self.cfg.clone();
        let remote = self.remote_path(remote_key)?;
        let local = local.to_path_buf();
        let key = remote_key.to_string();
        spawn_blocking(move || -> Result<RemoteObject, AppError> {
            let data = std::fs::read(&local)?;
            let sftp = Self::connect_blocking(&cfg)?;
            if let Some(parent) = remote.parent() {
                Self::mkdir_p(&sftp, parent)?;
            }
            let mut fh = sftp
                .create(&remote)
                .map_err(|e| Self::map_err("创建远端文件", e))?;
            fh.write_all(&data)?;
            Ok(RemoteObject {
                key,
                size: data.len() as i64,
            })
        })
        .await
    }

    async fn list(&self) -> Result<Vec<RemoteObject>, AppError> {
        let cfg = self.cfg.clone();
        let root = PathBuf::from(&self.cfg.remote_root);
        spawn_blocking(move || -> Result<Vec<RemoteObject>, AppError> {
            let sftp = Self::connect_blocking(&cfg)?;
            let mut out = Vec::new();
            let mut stack = vec![root.clone()];
            while let Some(dir) = stack.pop() {
                let entries = match sftp.readdir(&dir) {
                    Ok(v) => v,
                    Err(_) => continue, // 根目录不存在 → 空列表
                };
                for (path, stat) in entries {
                    if stat.is_dir() {
                        stack.push(path);
                    } else if let Ok(rel) = path.strip_prefix(&root) {
                        out.push(RemoteObject {
                            key: rel.to_string_lossy().replace('\\', "/"),
                            size: stat.size.unwrap_or(0) as i64,
                        });
                    }
                }
            }
            Ok(out)
        })
        .await
    }

    async fn download(&self, remote_key: &str, local: &Path) -> Result<(), AppError> {
        let cfg = self.cfg.clone();
        let remote = self.remote_path(remote_key)?;
        let target = local.to_path_buf();
        spawn_blocking(move || -> Result<(), AppError> {
            let sftp = Self::connect_blocking(&cfg)?;
            let mut fh = sftp
                .open(&remote)
                .map_err(|e| Self::map_err("打开远端文件", e))?;
            let mut data = Vec::new();
            fh.read_to_end(&mut data)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &data)?;
            Ok(())
        })
        .await
    }

    async fn delete(&self, remote_key: &str) -> Result<(), AppError> {
        let cfg = self.cfg.clone();
        let remote = self.remote_path(remote_key)?;
        spawn_blocking(move || -> Result<(), AppError> {
            let sftp = Self::connect_blocking(&cfg)?;
            // key 不存在时幂等成功
            if sftp.stat(&remote).is_err() {
                return Ok(());
            }
            sftp.unlink(&remote)
                .map_err(|e| Self::map_err("删除远端文件", e))?;
            Ok(())
        })
        .await
    }

    async fn test_connection(&self) -> Result<(), AppError> {
        let cfg = self.cfg.clone();
        let root = PathBuf::from(&self.cfg.remote_root);
        spawn_blocking(move || -> Result<(), AppError> {
            let sftp = Self::connect_blocking(&cfg)?;
            Self::mkdir_p(&sftp, &root)?;
            // 写探针确认可写
            let probe = root.join(".flamepanel-probe");
            let mut fh = sftp
                .create(&probe)
                .map_err(|e| Self::map_err("写探针", e))?;
            fh.write_all(b"ok")?;
            drop(fh);
            let _ = sftp.unlink(&probe);
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let cfg: SftpConfig =
            serde_json::from_str(r#"{"host":"192.168.1.10","username":"root","password":"x"}"#)
                .unwrap();
        assert_eq!(cfg.port, 22);
        assert_eq!(cfg.remote_root, "flamepanel-backups");
    }

    #[test]
    fn config_private_key() {
        let cfg: SftpConfig = serde_json::from_str(
            r#"{"host":"h","port":2222,"username":"u","private_key_path":"/root/.ssh/id_ed25519"}"#,
        )
        .unwrap();
        assert_eq!(cfg.port, 2222);
        assert!(cfg.password.is_none());
        assert_eq!(
            cfg.private_key_path.as_deref(),
            Some("/root/.ssh/id_ed25519")
        );
    }

    #[tokio::test]
    async fn connect_refused_maps_to_error() {
        // 未监听端口 → 连接失败应返回 AppError 而非 panic
        let cfg = SftpConfig {
            host: "127.0.0.1".into(),
            port: 1, // 保留端口，本机几乎必然拒绝
            username: "u".into(),
            password: Some("p".into()),
            private_key_path: None,
            passphrase: None,
            remote_root: "rp".into(),
        };
        let backend = SftpBackend::from_config(&cfg);
        assert!(backend.test_connection().await.is_err());
    }

    #[test]
    fn traversal_key_rejected() {
        let cfg: SftpConfig =
            serde_json::from_str(r#"{"host":"h","username":"u","password":"p"}"#).unwrap();
        let backend = SftpBackend::from_config(&cfg);
        assert!(backend.remote_path("../evil").is_err());
        assert!(backend.remote_path("/abs").is_err());
    }
}
