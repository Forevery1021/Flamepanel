use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::LogEntry;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// 系统事件日志列表
#[utoipa::path(
    get,
    path = "/api/logs",
    tag = "logs",
    params(PaginationParams),
    responses(
        (status = 200, description = "系统事件日志（分页，新在前）", body = PaginatedResponse<LogEntry>),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<LogEntry>>, AppError> {
    let result = state.log_service.list_paginated(&params).await?;
    Ok(Json(result))
}

/// 删除一条系统事件日志
#[utoipa::path(
    delete,
    path = "/api/logs/{id}",
    tag = "logs",
    params(("id" = i64, Path, description = "ID")),
    responses(
        (status = 200, description = "删除成功", body = String),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 404, description = "日志不存在"),
    ),
    security(("BearerAuth" = []))
)]
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
