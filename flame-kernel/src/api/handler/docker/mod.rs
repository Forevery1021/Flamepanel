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

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNetworkRequest {
    pub name: String,
    #[serde(default = "default_network_driver")]
    pub driver: String,
    pub subnet: Option<String>,
}

fn default_network_driver() -> String {
    "bridge".into()
}

#[derive(Debug, Deserialize)]
pub struct ConnectNetworkRequest {
    pub container_id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateVolumeRequest {
    pub name: String,
    #[serde(default = "default_volume_driver")]
    pub driver: String,
}

fn default_volume_driver() -> String {
    "local".into()
}

#[derive(Debug, Deserialize)]
pub struct PullImageRequest {
    pub image: String,
}

#[derive(Debug, Deserialize)]
pub struct TagImageRequest {
    pub repo: String,
    pub tag: String,
}

#[derive(Debug, Deserialize)]
pub struct ForceQuery {
    #[serde(default)]
    pub force: bool,
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

// ── 容器高级操作 ─────────────────────────────────────────────

pub async fn inspect(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let detail = state.docker_service.inspect_container(&id).await?;
    Ok(Json(detail))
}

pub async fn rename(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .rename_container(&id, &req.new_name)
        .await?;
    Ok(Json(()))
}

pub async fn pause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.pause_container(&id).await?;
    Ok(Json(()))
}

pub async fn unpause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.unpause_container(&id).await?;
    Ok(Json(()))
}

pub async fn kill(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.kill_container(&id).await?;
    Ok(Json(()))
}

pub async fn prune_containers(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_containers().await?;
    Ok(Json(result))
}

// ── 网络管理 ────────────────────────────────────────────────

pub async fn list_networks(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let networks = state.docker_service.list_networks().await?;
    Ok(Json(networks))
}

pub async fn create_network(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateNetworkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .docker_service
        .create_network(&req.name, &req.driver, req.subnet.as_deref())
        .await?;
    Ok(Json(result))
}

pub async fn remove_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.remove_network(&id).await?;
    Ok(Json(()))
}

pub async fn connect_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ConnectNetworkRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .connect_network(&id, &req.container_id)
        .await?;
    Ok(Json(()))
}

pub async fn disconnect_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ConnectNetworkRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .disconnect_network(&id, &req.container_id, req.force)
        .await?;
    Ok(Json(()))
}

pub async fn prune_networks(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_networks().await?;
    Ok(Json(result))
}

// ── 卷管理 ──────────────────────────────────────────────────

pub async fn list_volumes(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let volumes = state.docker_service.list_volumes().await?;
    Ok(Json(volumes))
}

pub async fn create_volume(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateVolumeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .docker_service
        .create_volume(&req.name, &req.driver)
        .await?;
    Ok(Json(result))
}

pub async fn remove_volume(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<ForceQuery>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .remove_volume(&name, query.force)
        .await?;
    Ok(Json(()))
}

pub async fn prune_volumes(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_volumes().await?;
    Ok(Json(result))
}

// ── 镜像管理 ────────────────────────────────────────────────

pub async fn pull_image(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<PullImageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let message = state.docker_service.pull_image(&req.image).await?;
    Ok(Json(
        serde_json::json!({ "image": req.image, "message": message }),
    ))
}

pub async fn tag_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<TagImageRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .tag_image(&id, &req.repo, &req.tag)
        .await?;
    Ok(Json(()))
}

pub async fn prune_images(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_images().await?;
    Ok(Json(result))
}

// ── Compose 项目 ────────────────────────────────────────────

pub async fn compose_ls(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let projects = state.docker_service.compose_ls().await?;
    Ok(Json(projects))
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
        .route("/api/docker/images/pull", axum::routing::post(pull_image))
        .route(
            "/api/docker/images/:id/remove",
            axum::routing::post(remove_image),
        )
        .route("/api/docker/images/:id/tag", axum::routing::post(tag_image))
        .route(
            "/api/docker/images/prune",
            axum::routing::post(prune_images),
        )
        .route(
            "/api/docker/containers/:id/inspect",
            axum::routing::get(inspect),
        )
        .route(
            "/api/docker/containers/:id/rename",
            axum::routing::post(rename),
        )
        .route(
            "/api/docker/containers/:id/pause",
            axum::routing::post(pause),
        )
        .route(
            "/api/docker/containers/:id/unpause",
            axum::routing::post(unpause),
        )
        .route("/api/docker/containers/:id/kill", axum::routing::post(kill))
        .route(
            "/api/docker/containers/prune",
            axum::routing::post(prune_containers),
        )
        .route("/api/docker/networks", axum::routing::get(list_networks))
        .route("/api/docker/networks", axum::routing::post(create_network))
        .route(
            "/api/docker/networks/prune",
            axum::routing::post(prune_networks),
        )
        .route(
            "/api/docker/networks/:id",
            axum::routing::delete(remove_network),
        )
        .route(
            "/api/docker/networks/:id/connect",
            axum::routing::post(connect_network),
        )
        .route(
            "/api/docker/networks/:id/disconnect",
            axum::routing::post(disconnect_network),
        )
        .route("/api/docker/volumes", axum::routing::get(list_volumes))
        .route("/api/docker/volumes", axum::routing::post(create_volume))
        .route(
            "/api/docker/volumes/prune",
            axum::routing::post(prune_volumes),
        )
        .route(
            "/api/docker/volumes/:name",
            axum::routing::delete(remove_volume),
        )
        .route("/api/docker/compose", axum::routing::get(compose_ls))
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
