use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::entity::DockerContainer;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ContainerQuery {
    node_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    tail: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ComposeDeployRequest {
    pub project_name: String,
    pub compose_yaml: String,
}

#[derive(Debug, Serialize)]
pub struct ComposeDeployResponse {
    pub project_name: String,
    pub status: String,
    pub message: String,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ContainerQuery>,
) -> Result<Json<Vec<DockerContainer>>, AppError> {
    let node_id = query.node_id.unwrap_or(0);
    let containers = state.docker_service.list_containers(node_id).await?;
    Ok(Json(containers))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DockerContainer>, AppError> {
    let containers = state.docker_service.list_containers(0).await?;
    let container = containers
        .into_iter()
        .find(|c| c.id == id || c.name == id)
        .ok_or_else(|| AppError::NotFound(format!("Container {} not found", id)))?;
    Ok(Json(container))
}

pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.start_container(&id).await?;
    Ok(Json(()))
}

pub async fn stop(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.stop_container(&id, 10).await?;
    Ok(Json(()))
}

pub async fn restart(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.restart_container(&id, 10).await?;
    Ok(Json(()))
}

pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.remove_container(&id, false).await?;
    Ok(Json(()))
}

pub async fn logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<String>, AppError> {
    let tail = query.tail.unwrap_or(100);
    let logs = state.docker_service.get_container_logs(&id, tail).await?;
    Ok(Json(logs))
}

pub async fn stats(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = state.docker_service.get_container_stats(&id).await?;
    Ok(Json(stats))
}

pub async fn list_images(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let images = state.docker_service.list_images().await?;
    Ok(Json(images))
}

pub async fn remove_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.remove_image(&id).await?;
    Ok(Json(()))
}

pub async fn compose_deploy(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ComposeDeployRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .docker_service
        .compose_deploy(&req.project_name, &req.compose_yaml)
        .await?;
    Ok(Json(result))
}

pub async fn compose_up(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.compose_up(&project_name).await?;
    Ok(Json(()))
}

pub async fn compose_down(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.compose_down(&project_name).await?;
    Ok(Json(()))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/docker/containers", axum::routing::get(list))
        .route("/api/docker/containers/:id", axum::routing::get(get))
        .route(
            "/api/docker/containers/:id/start",
            axum::routing::post(start),
        )
        .route("/api/docker/containers/:id/stop", axum::routing::post(stop))
        .route(
            "/api/docker/containers/:id/restart",
            axum::routing::post(restart),
        )
        .route(
            "/api/docker/containers/:id/remove",
            axum::routing::post(remove),
        )
        .route("/api/docker/containers/:id/logs", axum::routing::get(logs))
        .route(
            "/api/docker/containers/:id/stats",
            axum::routing::get(stats),
        )
        .route("/api/docker/images", axum::routing::get(list_images))
        .route(
            "/api/docker/images/:id/remove",
            axum::routing::post(remove_image),
        )
        .route(
            "/api/docker/compose/deploy",
            axum::routing::post(compose_deploy),
        )
        .route(
            "/api/docker/compose/:project_name/up",
            axum::routing::post(compose_up),
        )
        .route(
            "/api/docker/compose/:project_name/down",
            axum::routing::post(compose_down),
        )
}
