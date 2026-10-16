//! 备份与 DNS 账号共用的存储兼容策略，不改变密钥不可用时的既有行为。
//! 仅供 application 内部使用；不是新的公有加密 API。
use crate::core::error::AppError;
use crate::utils::SecretCipher;

/// 有密钥时仅空白 / 空对象跳过加密；其他内容逐字节加密，不规范化 JSON。
pub(super) fn seal(cipher: Option<&SecretCipher>, plaintext: &str) -> Result<String, AppError> {
    match cipher {
        Some(cipher) => {
            let trimmed = plaintext.trim();
            if trimmed.is_empty() || trimmed == "{}" {
                Ok(trimmed.to_string())
            } else {
                cipher.encrypt(plaintext)
            }
        }
        None => Ok(plaintext.to_string()),
    }
}

/// 有密钥时校验密文并兼容历史明文；无密钥时保留原始值，不伪称已解密。
pub(super) fn open(cipher: Option<&SecretCipher>, stored: &str) -> Result<String, AppError> {
    match cipher {
        Some(cipher) => cipher.decrypt(stored),
        None => Ok(stored.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_without_key_preserves_bytes_including_empty_objects() {
        for value in ["", " \n", "{}", " {} \n", "{ }", r#" {"token":"test"} "#] {
            assert_eq!(seal(None, value).unwrap(), value);
        }
    }

    #[test]
    fn seal_with_key_only_normalizes_empty_values() {
        let cipher = SecretCipher::new(b"credential-storage-test");
        for (value, expected) in [("", ""), (" \n", ""), ("{}", "{}"), (" {} \n", "{}")] {
            assert_eq!(seal(Some(&cipher), value).unwrap(), expected);
        }
        // 内含空格的对象不是旧策略中的特殊值，必须继续加密。
        for value in ["{ }", r#" {"token":"test"} "#, "null", "[]"] {
            let encrypted = seal(Some(&cipher), value).unwrap();
            assert!(encrypted.starts_with("enc:v1:"));
            assert_eq!(open(Some(&cipher), &encrypted).unwrap(), value);
        }
    }

    #[test]
    fn open_preserves_legacy_plaintext_with_or_without_key() {
        let cipher = SecretCipher::new(b"credential-storage-test");
        for value in ["", " {} \n", r#" {"token":"legacy"} "#] {
            assert_eq!(open(Some(&cipher), value).unwrap(), value);
            assert_eq!(open(None, value).unwrap(), value);
        }
    }

    #[test]
    fn missing_key_preserves_even_encrypted_or_malformed_values() {
        let cipher = SecretCipher::new(b"credential-storage-test");
        let encrypted = seal(Some(&cipher), "credential").unwrap();
        for value in [encrypted.as_str(), "enc:v1:not-base64!"] {
            assert_eq!(open(None, value).unwrap(), value);
        }
    }

    #[test]
    fn wrong_key_and_malformed_ciphertext_fail_closed_when_key_exists() {
        let cipher = SecretCipher::new(b"credential-storage-test");
        let other = SecretCipher::new(b"different-key");
        let encrypted = seal(Some(&cipher), "credential").unwrap();
        assert!(open(Some(&other), &encrypted).is_err());
        for value in ["enc:v1:", "enc:v1:not-base64!", "enc:v1:AA=="] {
            assert!(open(Some(&cipher), value).is_err());
        }
    }

    #[test]
    fn encryption_remains_randomized() {
        let cipher = SecretCipher::new(b"credential-storage-test");
        let first = seal(Some(&cipher), "credential").unwrap();
        let second = seal(Some(&cipher), "credential").unwrap();
        assert_ne!(first, second);
        assert_eq!(open(Some(&cipher), &first).unwrap(), "credential");
        assert_eq!(open(Some(&cipher), &second).unwrap(), "credential");
    }
}
