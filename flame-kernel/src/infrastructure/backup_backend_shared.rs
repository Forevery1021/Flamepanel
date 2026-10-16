//! 备份后端共享工具：阻塞 IO 统一移入 spawn_blocking（对齐仓库 F-32 纪律）

use crate::core::error::AppError;

/// 包装 spawn_blocking，把 panic 转为 AppError（对齐 backup_service 现有模式）
pub async fn spawn_blocking<F, R>(f: F) -> Result<R, AppError>
where
    F: FnOnce() -> Result<R, AppError> + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(format!("blocking task panicked: {}", e)))?
}
