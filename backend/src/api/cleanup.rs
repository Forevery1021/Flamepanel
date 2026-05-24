use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};

use crate::application::{AppState, CleanupService};
use crate::core::error::AppError;
use crate::domain::{CleanupItem, CleanupRequest, CleanupResult, CleanupScanResult};
use crate::middleware::auth::CurrentUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/scan", get(scan))
        .route("/run", post(run_cleanup))
}

async fn scan(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<CleanupScanResult>, AppError> {
    let items: Vec<CleanupItem> = CleanupService::scan(&state.docker).await;

    let total_bytes: u64 = items.iter().map(|i| i.size_bytes).sum();
    let total_display = crate::application::CleanupService::format_size(total_bytes);

    tracing::info!("用户 '{}' 扫描了系统垃圾", _claims.sub);

    Ok(Json(CleanupScanResult {
        items,
        total_bytes,
        total_display,
    }))
}

async fn run_cleanup(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<CleanupRequest>,
) -> Result<Json<CleanupResult>, AppError> {
    if payload.categories.is_empty() {
        return Err(AppError::BadRequest("请选择至少一个清理类别".into()));
    }

    tracing::info!("用户 '{}' 执行清理: {:?}", _claims.sub, payload.categories);

    let result = CleanupService::clean(&payload.categories, &state.docker).await;
    Ok(Json(result))
}
