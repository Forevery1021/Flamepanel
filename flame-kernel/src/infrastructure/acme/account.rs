//! ACME 账户密钥持久化（M-B 主线三 §6.1 · Stage 1）。
//!
//! ACME 协议要求同一账户（Let's Encrypt / ZeroSSL 等目录）使用稳定账户密钥，
//! 账户私钥丢失 = 无法续签已签发证书。本模块负责账户密钥的生成与持久化：
//! - 首次调用生成 ECDSA P-256 密钥并落盘 `data/acme/account.key`（PEM，0600）
//! - 后续启动复用已有密钥（幂等，不覆盖）
//!
//! 与 ACME 服务器的账户注册（newAccount 请求）属 Stage 2 签发链路（依赖
//! `instant-acme`），本模块只负责密钥材料，保持 Stage 1 零外部协议依赖。

use crate::core::error::AppError;
use rcgen::KeyPair;
use std::fs;
use std::path::{Path, PathBuf};

/// 账户密钥相对路径（面板 data 目录下）。
pub const ACCOUNT_KEY_FILE: &str = "acme/account.key";

/// ACME 账户密钥存储。克隆廉价。
#[derive(Debug, Clone)]
pub struct AcmeAccountKeyStore {
    key_path: PathBuf,
}

impl AcmeAccountKeyStore {
    /// 以面板 data 目录构造（key 落 `data/acme/account.key`）。
    pub fn new(data_dir: &Path) -> Self {
        Self {
            key_path: data_dir.join(ACCOUNT_KEY_FILE),
        }
    }

    /// 密钥文件路径。
    pub fn key_path(&self) -> &Path {
        self.key_path.as_path()
    }

    /// 确保账户密钥存在并返回 PEM 内容（幂等：已存在直接复用）。
    ///
    /// 目录随文件创建（0700），私钥 0600——对齐 `cert.rs` 自签证书的权限纪律。
    pub fn ensure_account_key(&self) -> Result<String, AppError> {
        if let Ok(existing) = fs::read_to_string(&self.key_path) {
            if !existing.trim().is_empty() {
                return Ok(existing);
            }
        }
        let key_pair = KeyPair::generate()
            .map_err(|e| AppError::internal(format!("生成 ACME 账户密钥失败: {}", e)))?;
        let pem = key_pair.serialize_pem();
        if let Some(parent) = self.key_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("创建 acme 目录失败: {}", e)))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
            }
        }
        fs::write(&self.key_path, &pem)
            .map_err(|e| AppError::internal(format!("写入 ACME 账户密钥失败: {}", e)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.key_path, fs::Permissions::from_mode(0o600));
        }
        Ok(pem)
    }

    /// 账户密钥是否已存在（诊断用，不触发生成）。
    pub fn exists(&self) -> bool {
        self.key_path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fp-acme-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generates_and_reuses_account_key() {
        let dir = temp_dir();
        let store = AcmeAccountKeyStore::new(&dir);
        assert!(!store.exists());

        let pem1 = store.ensure_account_key().unwrap();
        assert!(pem1.contains("BEGIN PRIVATE KEY"));
        assert!(store.exists());
        // 私钥 0600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(store.key_path()).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        // 幂等：第二次调用复用同一密钥
        let pem2 = store.ensure_account_key().unwrap();
        assert_eq!(pem1, pem2);

        // 同目录新 store 实例也能复用（重启语义）
        let store2 = AcmeAccountKeyStore::new(&dir);
        assert_eq!(store2.ensure_account_key().unwrap(), pem1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn regenerates_on_empty_file() {
        let dir = temp_dir();
        let store = AcmeAccountKeyStore::new(&dir);
        std::fs::create_dir_all(dir.join("acme")).unwrap();
        std::fs::write(dir.join("acme/account.key"), "").unwrap();
        let pem = store.ensure_account_key().unwrap();
        assert!(pem.contains("BEGIN PRIVATE KEY"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
