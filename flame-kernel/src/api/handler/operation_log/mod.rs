use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::OperationLog;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct LogListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// 按 action 前缀过滤（如 `action=LOGIN` 匹配 LOGIN_SUCCESS/LOGIN_FAILED）
    pub action: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<LogListQuery>,
) -> Result<Json<PaginatedResponse<OperationLog>>, AppError> {
    let params = PaginationParams {
        page: query.page,
        page_size: query.page_size,
    };
    let result = state
        .operation_log_service
        .list_paginated(&params, query.action.as_deref())
        .await?;
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
