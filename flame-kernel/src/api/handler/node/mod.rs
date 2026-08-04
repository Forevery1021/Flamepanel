use crate::api::extract::ApiJson;
use crate::api::types::{AppState, CreateNodeRequest, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::ServerNode;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

const HEARTBEAT_TIMEOUT_SECS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub cpu_usage: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub load_one: f32,
}

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ServerNode>>, AppError> {
    let result = state.node_service.list_nodes_paginated(&params).await?;
    Ok(Json(result))
}

pub async fn create(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<CreateNodeRequest>,
) -> Result<Json<i64>, AppError> {
    let id = state.node_service.register_node(&payload.to_node()).await?;
    Ok(Json(id))
}

/// Agent 心跳上报（白名单免 JWT，校验 Agent token）
pub async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Agent token 校验（兼容旧 Agent：库中无 token 时放行）
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).to_string());
    let valid = state
        .node_service
        .verify_agent_token(id, provided.as_deref())
        .await?;
    if !valid {
        return Err(AppError::Unauthorized("Invalid agent token".into()));
    }

    let metrics = serde_json::json!({
        "cpu_usage": req.cpu_usage,
        "memory_usage_percent": req.memory_usage_percent,
        "disk_usage_percent": req.disk_usage_percent,
        "load_one": req.load_one,
    });
    let node = state.node_service.record_heartbeat(id, &metrics).await?;
    Ok(Json(serde_json::json!({
        "id": node.id,
        "status": "ok",
        "last_heartbeat_at": node.last_heartbeat_at,
    })))
}

/// 节点在线状态（惰性判定）
pub async fn status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = state
        .node_service
        .node_status(id, HEARTBEAT_TIMEOUT_SECS)
        .await?;
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

/// 节点最近指标快照
pub async fn metrics(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let metrics = state.node_service.node_metrics(id).await?;
    Ok(Json(metrics))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<CreateNodeRequest>,
) -> Result<Json<ServerNode>, AppError> {
    let mut node = payload.to_node();
    node.id = id;
    state.node_service.update_node(&node).await?;
    let updated = state.node_service.get_node(id).await?;
    Ok(Json(updated))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.node_service.delete_node(id).await?;
    Ok(Json("deleted"))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/nodes", axum::routing::get(list))
        .route("/api/nodes", axum::routing::post(create))
        .route("/api/nodes/:id", axum::routing::put(update))
        .route("/api/nodes/:id", axum::routing::delete(delete))
        .route("/api/nodes/heartbeat/:id", axum::routing::post(heartbeat))
        .route("/api/nodes/:id/status", axum::routing::get(status))
        .route("/api/nodes/:id/metrics", axum::routing::get(metrics))
}
