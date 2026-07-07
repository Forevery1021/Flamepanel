use regex::Regex;
use crate::core::error::AppError;

/// 验证工具
pub struct ValidationUtils;

impl ValidationUtils {
    /// 验证电子邮件格式
    pub fn validate_email(email: &str) -> Result<(), AppError> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .map_err(|e| AppError::Internal(format!("Failed to compile regex: {}", e)))?;
        
        if !email_regex.is_match(email) {
            return Err(AppError::ValidationError("Invalid email format".to_string()));
        }
        
        Ok(())
    }
    
    /// 验证用户名
    pub fn validate_username(username: &str) -> Result<(), AppError> {
        if username.len() < 3 {
            return Err(AppError::ValidationError("Username must be at least 3 characters".to_string()));
        }
        
        if username.len() > 30 {
            return Err(AppError::ValidationError("Username must not exceed 30 characters".to_string()));
        }
        
        let username_regex = Regex::new(r"^[a-zA-Z0-9_-]+$")
            .map_err(|e| AppError::Internal(format!("Failed to compile regex: {}", e)))?;
        
        if !username_regex.is_match(username) {
            return Err(AppError::ValidationError("Username can only contain letters, numbers, underscores, and hyphens".to_string()));
        }
        
        Ok(())
    }
    
    /// 验证密码强度
    pub fn validate_password_strength(password: &str) -> Result<(), AppError> {
        if password.len() < 8 {
            return Err(AppError::ValidationError("Password must be at least 8 characters".to_string()));
        }
        
        // 至少包含一个大写字母，一个小写字母，一个数字
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        
        if !(has_upper && has_lower && has_digit) {
            return Err(AppError::ValidationError("Password must contain at least one uppercase letter, one lowercase letter, and one digit".to_string()));
        }
        
        Ok(())
    }
    
    /// 验证域名
    pub fn validate_domain(domain: &str) -> Result<(), AppError> {
        if domain.is_empty() {
            return Err(AppError::ValidationError("Domain cannot be empty".to_string()));
        }
        
        if domain.len() > 253 {
            return Err(AppError::ValidationError("Domain is too long".to_string()));
        }
        
        let domain_regex = Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9\-]*[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$")
            .map_err(|e| AppError::Internal(format!("Failed to compile regex: {}", e)))?;
        
        if !domain_regex.is_match(domain) {
            return Err(AppError::ValidationError("Invalid domain format".to_string()));
        }
        
        Ok(())
    }
}
