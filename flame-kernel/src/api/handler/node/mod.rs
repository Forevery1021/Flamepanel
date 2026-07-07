use axum::{Json, extract::State};
use crate::domain::entity::ServerNode;
use crate::api::types::{AppState, CreateNodeRequest};

pub async fn list(State(state): State<AppState>) -> Json<Vec<ServerNode>> {
    let nodes = state.node_service.list_nodes().await.unwrap();
    Json(nodes)
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateNodeRequest>,
) -> Json<i64> {
    let id = state.node_service.register_node(&payload.node).await.unwrap();
    Json(id)
}