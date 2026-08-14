use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::LogEntry;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};

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

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/logs", axum::routing::get(list))
        .route("/api/logs/{id}", axum::routing::delete(delete))
}
