use axum::{Json, extract::State};
use axum::http::StatusCode;
use crate::api::types::AppState;
use crate::domain::entity::OperationLog;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<OperationLog>>, StatusCode> {
    state.operation_log_service.list().await.map(Json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
