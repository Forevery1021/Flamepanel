use crate::core::error::AppError;
use bcrypt::{hash, verify, DEFAULT_COST};

/// bcrypt 最大有效输入字节数（超出部分被静默截断）。
const BCRYPT_MAX_BYTES: usize = 72;
/// 密码最小长度。
pub const MIN_PASSWORD_LEN: usize = 8;
/// 密码最大长度（UTF-8 字符数上限，防止极端长密码资源消耗）。
const MAX_PASSWORD_CHARS: usize = 128;

pub struct PasswordUtils;

impl PasswordUtils {
    pub fn hash(password: &str) -> Result<String, AppError> {
        let hashed = hash(password, DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("Failed to hash password: {}", e)))?;
        Ok(hashed)
    }

    pub fn verify(password: &str, hashed: &str) -> Result<bool, AppError> {
        let valid = verify(password, hashed)
            .map_err(|e| AppError::internal(format!("Failed to verify password: {}", e)))?;
        Ok(valid)
    }

    /// F-22: 校验密码强度与 bcrypt 72 字节截断限制。
    ///
    /// 规则：
    /// - 长度 ≥ 8 字符（MIN_PASSWORD_LEN）
    /// - 长度 ≤ 128 字符（MAX_PASSWORD_CHARS）
    /// - 字节数 ≤ 72（bcrypt 截断保护）
    /// - 至少包含两类字符（大小写/数字/特殊符号）
    /// - 不能与用户名相同或高度相似（相似定义为「用户名是其前缀」且长度差距 ≤ 3）
    pub fn validate_strength(password: &str, username: Option<&str>) -> Result<(), AppError> {
        let char_count = password.chars().count();
        if char_count < MIN_PASSWORD_LEN {
            return Err(AppError::ValidationError(format!(
                "password must be at least {} characters",
                MIN_PASSWORD_LEN
            )));
        }
        if char_count > MAX_PASSWORD_CHARS {
            return Err(AppError::ValidationError(format!(
                "password must be at most {} characters",
                MAX_PASSWORD_CHARS
            )));
        }
        if password.len() > BCRYPT_MAX_BYTES {
            return Err(AppError::ValidationError(
                "password exceeds 72 bytes (bcrypt limit)".into(),
            ));
        }
        // 复杂度：至少包含大小写字母/数字/符号中的两类
        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace());
        let categories = [has_upper, has_lower, has_digit, has_special]
            .iter()
            .filter(|&&b| b)
            .count();
        if categories < 2 {
            return Err(AppError::ValidationError(
                "password must contain at least two of: uppercase, lowercase, digits, special chars"
                    .into(),
            ));
        }
        // 与用户名相似性检查
        if let Some(name) = username {
            let name = name.trim();
            if !name.is_empty() && password.to_lowercase().starts_with(&name.to_lowercase()) {
                return Err(AppError::ValidationError(
                    "password must not be too similar to the username".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_min_length_validation() {
        let err = PasswordUtils::validate_strength("Short1!", Some("user")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[test]
    fn password_complexity_validation() {
        // Only lowercase, no other category - should fail
        let err = PasswordUtils::validate_strength("onlylowercase", Some("user")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));

        // Lowercase + digit - should pass
        assert!(PasswordUtils::validate_strength("password123", Some("user")).is_ok());
    }

    #[test]
    fn password_similar_to_username_rejected() {
        let err = PasswordUtils::validate_strength("admin12345", Some("admin")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[test]
    fn password_bcrypt_72_byte_limit() {
        // 73+ byte password should be rejected
        let long_pw = "a".repeat(80) + "B1!";
        let err = PasswordUtils::validate_strength(&long_pw, Some("user")).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[test]
    fn strong_password_accepted() {
        assert!(PasswordUtils::validate_strength("Str0ng@Passw0rd", Some("admin")).is_ok());
    }
}
