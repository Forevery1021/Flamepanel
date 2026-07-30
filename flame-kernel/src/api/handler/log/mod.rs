use axum::{Json, extract::State};
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::entity::LogEntry;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<LogEntry>>, AppError> {
    let logs = state.log_service.list().await?;
    Ok(Json(logs))
}
