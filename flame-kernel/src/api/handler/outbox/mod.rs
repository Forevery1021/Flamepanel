use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::OutboxEvent;
use axum::Router;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

/// 事件落库（Outbox）查询参数。
#[derive(Debug, Deserialize, Default, IntoParams, ToSchema)]
pub struct OutboxListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// 按事件类型过滤（如 `type=AppInstalled` 精确匹配变体名）
    #[serde(rename = "type")]
    pub event_type: Option<String>,
}

/// `GET /api/outbox-events` — 分页查询事件落库（审计）。
#[utoipa::path(
    get,
    path = "/api/outbox-events",
    tag = "outbox",
    params(PaginationParams, OutboxListQuery),
    responses(
        (status = 200, description = "事件落库分页列表", body = PaginatedResponse<OutboxEvent>),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<OutboxListQuery>,
) -> Result<Json<PaginatedResponse<OutboxEvent>>, AppError> {
    let params = PaginationParams {
        page: query.page,
        page_size: query.page_size,
    };
    let result = state
        .outbox_service
        .list_paginated(&params, query.event_type.as_deref())
        .await?;
    Ok(Json(result))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/outbox-events", axum::routing::get(list))
}
