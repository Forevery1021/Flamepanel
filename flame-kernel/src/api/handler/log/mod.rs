use axum::{Json, extract::{State, Path, Query}};
use crate::api::types::{AppState, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;
use crate::domain::entity::LogEntry;

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<LogEntry>>, AppError> {
    let result = state.log_service.list_paginated(&params).await?;
    Ok(Json(result))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.log_service.delete_log(id).await?;
    Ok(Json("deleted"))
}
