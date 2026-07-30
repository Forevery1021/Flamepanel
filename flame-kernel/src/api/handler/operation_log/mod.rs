use axum::{Json, extract::State};
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::entity::OperationLog;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<OperationLog>>, AppError> {
    let logs = state.operation_log_service.list().await?;
    Ok(Json(logs))
}
