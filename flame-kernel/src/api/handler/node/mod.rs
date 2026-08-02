use axum::{Json, extract::{State, Path, Query}};
use axum::Router;
use crate::api::extract::ApiJson;
use crate::domain::entity::ServerNode;
use crate::api::types::{AppState, CreateNodeRequest, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;

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
    let id = state.node_service.register_node(&payload.node).await?;
    Ok(Json(id))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<CreateNodeRequest>,
) -> Result<Json<ServerNode>, AppError> {
    let mut node = payload.node;
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
}
