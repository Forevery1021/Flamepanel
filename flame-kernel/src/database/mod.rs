pub mod mysql;
pub mod redis;

pub use mysql::MySqlManager;
pub use redis::RedisManager;

use async_trait::async_trait;
use crate::core::error::AppError;

pub struct DatabaseOperationResult {
    pub success: bool,
    pub message: String,
}

#[async_trait]
pub trait NativeDbManager: Send + Sync {
    async fn install(&self, version: Option<&str>, port: i32, password: &str) -> Result<(), AppError>;
    async fn uninstall(&self) -> Result<(), AppError>;
    async fn start(&self) -> Result<(), AppError>;
    async fn stop(&self) -> Result<(), AppError>;
    async fn restart(&self) -> Result<(), AppError>;
    async fn is_running(&self) -> Result<bool, AppError>;
    async fn get_version(&self) -> Result<String, AppError>;
    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError>;
    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError>;
}
