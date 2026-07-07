use axum::{Json, extract::State};
use axum::http::StatusCode;
use crate::api::types::AppState;
use crate::domain::entity::LogEntry;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    state.log_service.list().await.map(Json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
