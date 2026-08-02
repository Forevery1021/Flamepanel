use axum::{Json, extract::{State, Path, Query}};
use axum::Router;
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



/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/operation-logs", axum::routing::get(list))
        .route("/api/operation-logs/:id", axum::routing::delete(delete))
}
