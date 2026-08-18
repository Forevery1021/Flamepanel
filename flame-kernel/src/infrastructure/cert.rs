//! 自签证书基础设施（B3）：为后续 Web Server（nginx）TLS 提供本地生成的自签证书。
//!
//! 使用 `rcgen` 纯 Rust 实现生成 ECDSA P-256 自签证书，写入 `data/certs/`，
//! 文件权限 0600。已存在证书时复用（幂等），不覆盖用户替换的证书。

use crate::core::error::AppError;
use rcgen::{CertificateParams, DnType, IsCa, KeyPair};
use std::fs;
use std::path::{Path, PathBuf};

/// 证书有效期：约 10 年（自签证书，换机重装即可再生）。
const NOT_BEFORE_DAYS: i64 = -1;
const NOT_AFTER_DAYS: i64 = 3650;

/// 确保 `data/certs/panel.{crt,key}` 存在；不存在则按 `domain` 生成自签证书。
/// 返回 `(cert_path, key_path)`。幂等：证书已存在时直接复用。
pub fn ensure_self_signed_cert(
    data_dir: &Path,
    domain: &str,
) -> Result<(PathBuf, PathBuf), AppError> {
    let cert_dir = data_dir.join("certs");
    let cert_path = cert_dir.join("panel.crt");
    let key_path = cert_dir.join("panel.key");

    if cert_path.exists() && key_path.exists() {
        return Ok((cert_path, key_path));
    }

    let (cert_pem, key_pem) = generate_self_signed(domain)?;
    fs::create_dir_all(&cert_dir)
        .map_err(|e| AppError::internal(format!("Failed to create cert dir: {}", e)))?;
    fs::write(&cert_path, &cert_pem)
        .map_err(|e| AppError::internal(format!("Failed to write cert: {}", e)))?;
    fs::write(&key_path, &key_pem)
        .map_err(|e| AppError::internal(format!("Failed to write cert key: {}", e)))?;

    // 私钥仅本人可读写（防其他系统用户窃取私钥）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
        let _ = fs::set_permissions(&cert_path, fs::Permissions::from_mode(0o644));
    }

    Ok((cert_path, key_path))
}

/// 生成 ECDSA P-256 自签证书（SAN = domain），返回 `(cert_pem, key_pem)`。
fn generate_self_signed(domain: &str) -> Result<(String, String), AppError> {
    let mut params = CertificateParams::new(vec![domain.to_string()])
        .map_err(|e| AppError::internal(format!("Invalid cert params: {}", e)))?;
    params
        .distinguished_name
        .push(DnType::CommonName, domain.to_string());
    params.is_ca = IsCa::NoCa;
    // 有效期：约 10 年
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::days(NOT_BEFORE_DAYS);
    params.not_after = now + time::Duration::days(NOT_AFTER_DAYS);

    let key_pair = KeyPair::generate()
        .map_err(|e| AppError::internal(format!("Failed to generate key pair: {}", e)))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| AppError::internal(format!("Failed to self-sign cert: {}", e)))?;

    Ok((cert.pem(), key_pair.serialize_pem()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_pem_cert() {
        let (cert_pem, key_pem) = generate_self_signed("localhost").unwrap();
        assert!(cert_pem.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(key_pem.starts_with("-----BEGIN PRIVATE KEY-----"));
    }

    #[test]
    fn ensure_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("fp-cert-test-{}", uuid::Uuid::new_v4()));
        let (c1, k1) = ensure_self_signed_cert(&dir, "example.test").unwrap();
        assert!(c1.exists() && k1.exists());
        // 再次调用复用已有证书（不重新生成）
        let (c2, _) = ensure_self_signed_cert(&dir, "example.test").unwrap();
        assert_eq!(c1, c2);
        let _ = fs::remove_dir_all(&dir);
    }
}
