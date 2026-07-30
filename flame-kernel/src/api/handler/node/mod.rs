use axum::{Json, extract::{State, Path, Query}};
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
    Json(payload): Json<CreateNodeRequest>,
) -> Result<Json<i64>, AppError> {
    let id = state.node_service.register_node(&payload.node).await?;
    Ok(Json(id))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.node_service.delete_node(id).await?;
    Ok(Json("deleted"))
}