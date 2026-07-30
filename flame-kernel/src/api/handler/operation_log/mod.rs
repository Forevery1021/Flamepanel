use axum::{Json, extract::{State, Path, Query}};
use crate::api::types::{AppState, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;
use crate::domain::entity::OperationLog;

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<OperationLog>>, AppError> {
    let result = state.operation_log_service.list_paginated(&params).await?;
    Ok(Json(result))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.operation_log_service.delete_log(id).await?;
    Ok(Json("deleted"))
}
