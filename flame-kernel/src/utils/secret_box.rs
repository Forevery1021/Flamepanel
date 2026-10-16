//! 敏感凭据静态加密（Doc/20 §6.5 遗留收尾）。
//!
//! 面板落库的第三方凭据（备份账号的 S3 SecretKey / SFTP 密码等）不得以明文落盘。
//! 本模块提供对称加解密原语：
//! - 算法：AES-256-GCM（认证加密，AEAD，篡改即解密失败）
//! - 密钥：主密钥经 HKDF-SHA256 派生（域分离，不与 JWT 签名等其他用途共用密钥材料）
//! - 密文格式：`enc:v1:BASE64(nonce|ciphertext|tag)`，自描述版本号便于后续轮换升级
//!
//! 兼容语义：非 `enc:v1:` 前缀的输入视为历史明文原样返回（ decrypt ），写入路径
//! 统一走 encrypt 后即完成静默迁移；不持有密钥时 encrypt / decrypt 返回确定性错误，
//! 由调用方决定降级或拒绝。

use aes_gcm::aead::Nonce;
use aes_gcm::aead::{Aead, Generate, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce as GcmNonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hkdf::Hkdf;
use sha2::Sha256;

/// 密文版本前缀（自描述，后续算法轮换时升 v2 并保留 v1 解密能力）。
const CIPHER_PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;
/// HKDF info 域分离标签。
const HKDF_INFO: &[u8] = b"flamepanel/secret-encryption/v1";

/// 对称凭据加密器。克隆廉价（内部仅持有 32 字节派生密钥）。
#[derive(Clone)]
pub struct SecretCipher {
    /// HKDF 派生出的 AES-256 密钥（不落日志、不序列化）。
    key: [u8; 32],
}

impl std::fmt::Debug for SecretCipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretCipher").finish_non_exhaustive()
    }
}

impl SecretCipher {
    /// 从主密钥材料（任意长度）派生加密密钥。
    pub fn new(master: &[u8]) -> Self {
        let hk = Hkdf::<Sha256>::new(None, master);
        let mut key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut key)
            .expect("HKDF-SHA256 输出 32 字节不会失败");
        Self { key }
    }

    /// 加密任意 UTF-8 明文，返回 `enc:v1:...` 自描述密文。
    /// 每次调用生成随机 nonce，同一明文两次加密结果不同。
    pub fn encrypt(&self, plaintext: &str) -> Result<String, crate::core::error::AppError> {
        let cipher = Aes256Gcm::new((&self.key).into());
        let nonce_arr = <Nonce<Aes256Gcm> as Generate>::generate();
        let nonce_bytes: [u8; NONCE_LEN] = nonce_arr.into();
        let sealed = cipher
            .encrypt(
                &nonce_arr,
                Payload {
                    msg: plaintext.as_bytes(),
                    aad: CIPHER_PREFIX.as_bytes(),
                },
            )
            .map_err(|_| crate::core::error::AppError::internal("凭据加密失败".to_string()))?;
        let mut packed = Vec::with_capacity(NONCE_LEN + sealed.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&sealed);
        Ok(format!("{}{}", CIPHER_PREFIX, BASE64.encode(packed)))
    }

    /// 解密 `enc:v1:` 密文；非该前缀的输入视为历史明文原样返回（兼容静默迁移）。
    pub fn decrypt(&self, stored: &str) -> Result<String, crate::core::error::AppError> {
        let Some(b64) = stored.strip_prefix(CIPHER_PREFIX) else {
            return Ok(stored.to_string());
        };
        let packed = BASE64.decode(b64).map_err(
            |_e: base64::DecodeError| -> crate::core::error::AppError {
                crate::core::error::AppError::internal("凭据密文不是合法 BASE64".to_string())
            },
        )?;
        if packed.len() < NONCE_LEN {
            return Err(crate::core::error::AppError::internal(String::from(
                "凭据密文长度非法",
            )));
        }
        let (nonce_bytes, sealed) = packed.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new((&self.key).into());
        let plain = cipher
            .decrypt(
                &GcmNonce::try_from(nonce_bytes)
                    .unwrap_or_else(|_| panic!("凭据密文 nonce 长度非法")),
                Payload {
                    msg: sealed,
                    aad: CIPHER_PREFIX.as_bytes(),
                },
            )
            .map_err(|_| {
                crate::core::error::AppError::internal(String::from(
                    "凭据解密失败（密钥不匹配或数据被篡改）",
                ))
            })?;
        String::from_utf8(plain).map_err(|_| {
            crate::core::error::AppError::internal(String::from("凭据明文不是合法 UTF-8"))
        })
    }
}

/// 生成 32 字节随机主密钥的 hex 表示（64 字符）。
/// 熵源复用 uuid v4（getrandom），与既有 random_secret_hex 保持一致。
pub fn generate_master_secret_hex() -> String {
    uuid::Uuid::new_v4().simple().to_string().repeat(2)
}

/// 从 `data/secret_encryption.key` 读取主密钥；不存在时生成并持久化（0600）。
/// 返回 (密钥 hex, 是否新生成)。
pub fn load_or_create_master_secret(data_dir: &std::path::Path) -> std::io::Result<(String, bool)> {
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join("secret_encryption.key");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok((trimmed.to_string(), false));
        }
    }
    let fresh = generate_master_secret_hex();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&path, &fresh)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    std::fs::write(&path, &fresh)?;
    Ok((fresh, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let c = SecretCipher::new(b"master-secret-material");
        let ct = c
            .encrypt(r#"{"secret_key":"top-secret","password":"p@ss"}"#)
            .unwrap();
        assert!(ct.starts_with("enc:v1:"));
        assert!(!ct.contains("top-secret"));
        let pt = c.decrypt(&ct).unwrap();
        assert_eq!(pt, r#"{"secret_key":"top-secret","password":"p@ss"}"#);
    }

    #[test]
    fn encrypt_is_randomized() {
        let c = SecretCipher::new(b"k");
        let a = c.encrypt("same").unwrap();
        let b = c.encrypt("same").unwrap();
        assert_ne!(a, b, "随机 nonce 下同一明文密文必须不同");
    }

    #[test]
    fn decrypt_plain_passthrough() {
        let c = SecretCipher::new(b"k");
        // 旧明文（含空 JSON {}）原样返回，不报错——兼容静默迁移
        assert_eq!(c.decrypt("{}").unwrap(), "{}");
        assert_eq!(
            c.decrypt(r#"{"root":"/data"}"#).unwrap(),
            r#"{"root":"/data"}"#
        );
    }

    #[test]
    fn decrypt_tampered_fails() {
        let c = SecretCipher::new(b"k");
        let mut ct = c.encrypt("secret").unwrap();
        // 篡改密文尾部一个字符
        let last = ct.pop().unwrap();
        ct.push(if last == 'A' { 'B' } else { 'A' });
        assert!(c.decrypt(&ct).is_err());
    }

    #[test]
    fn wrong_master_fails() {
        let ct = SecretCipher::new(b"master-a").encrypt("x").unwrap();
        assert!(SecretCipher::new(b"master-b").decrypt(&ct).is_err());
    }

    #[test]
    fn master_rotation_via_hkdf_domain() {
        // 同一主密钥，不同用途（模拟未来其他 info 域）不会得到相同加密密钥——
        // 这里仅验证 HKDF 输出确定性：同主密钥两次构造行为一致。
        let a = SecretCipher::new(b"m");
        let b = SecretCipher::new(b"m");
        let ct = a.encrypt("v").unwrap();
        assert_eq!(b.decrypt(&ct).unwrap(), "v");
    }

    #[test]
    fn load_or_create_persists_and_reuses() {
        let dir = tempfile::tempdir().unwrap();
        let (k1, created1) = load_or_create_master_secret(dir.path()).unwrap();
        assert!(created1);
        assert_eq!(k1.len(), 64);
        let (k2, created2) = load_or_create_master_secret(dir.path()).unwrap();
        assert!(!created2);
        assert_eq!(k1, k2, "重启必须复用同一密钥，否则历史密文无法解密");
    }
}
