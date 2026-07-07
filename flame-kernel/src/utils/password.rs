use bcrypt::{hash, verify, DEFAULT_COST};
use crate::core::error::AppError;

pub struct PasswordUtils;

impl PasswordUtils {
    pub fn hash(password: &str) -> Result<String, AppError> {
        let hashed = hash(password, DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;
        Ok(hashed)
    }
    
    pub fn verify(password: &str, hashed: &str) -> Result<bool, AppError> {
        let valid = verify(password, hashed)
            .map_err(|e| AppError::Internal(format!("Failed to verify password: {}", e)))?;
        Ok(valid)
    }
}
