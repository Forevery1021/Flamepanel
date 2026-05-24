use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum::body::Bytes;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;

use crate::application::{AppState, NodeService};
use crate::core::error::AppError;
use crate::domain::{
    BatchExecRequest, ClusterDashboard, FileEntry, NodeExecRequest,
    NodeExecResponse, NodeHeartbeatRequest, NodeInfo, NodeRegisterRequest,
};
use crate::middleware::auth::CurrentUser;

fn node_service(state: &AppState) -> NodeService {
    NodeService::new(state.node_repo.clone())
}

// ─── GET /nodes ─────────────────────────────────────────────────────────────────

pub async fn list_nodes(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<NodeInfo>>, AppError> {
    let service = node_service(&state);
    service.list().await.map(Json)
}

// ─── GET /nodes/cluster ─────────────────────────────────────────────────────────

pub async fn cluster_dashboard(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<ClusterDashboard>, AppError> {
    let service = node_service(&state);
    service.cluster_dashboard().await.map(Json)
}

// ─── GET /nodes/{id} ────────────────────────────────────────────────────────────

pub async fn get_node(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<NodeInfo>, AppError> {
    let service = node_service(&state);
    service.get(id).await.map(Json)
}

// ─── DELETE /nodes/{id} ─────────────────────────────────────────────────────────

pub async fn delete_node(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = node_service(&state);
    service.delete(id).await?;
    Ok(Json(json!({"message": "节点已删除"})))
}

// ─── POST /nodes/{id}/exec ──────────────────────────────────────────────────────

pub async fn exec_on_node(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<NodeExecRequest>,
) -> Result<Json<NodeExecResponse>, AppError> {
    let service = node_service(&state);
    service.exec_on_node(id, &req.command, req.timeout_secs).await.map(Json)
}

// ─── POST /nodes/batch-exec ─────────────────────────────────────────────────────

pub async fn batch_exec(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<BatchExecRequest>,
) -> Result<Json<Vec<NodeExecResponse>>, AppError> {
    let service = node_service(&state);
    service.batch_exec(&req.node_ids, &req.command, req.timeout_secs).await.map(Json)
}

// ─── GET /nodes/{id}/files/list ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct FileListQuery {
    path: Option<String>,
}

async fn list_files(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<FileListQuery>,
) -> Result<Json<Vec<FileEntry>>, AppError> {
    let service = node_service(&state);
    service.list_files(id, q.path).await.map(Json)
}

// ─── GET /nodes/{id}/files/download ─────────────────────────────────────────────

#[derive(Deserialize)]
struct FileDownloadQuery {
    path: String,
}

async fn download_file(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<FileDownloadQuery>,
) -> Result<Response, AppError> {
    let service = node_service(&state);
    let bytes = service.download_file(id, &q.path).await?;
    let filename = std::path::Path::new(&q.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .body(bytes.into())
        .unwrap();
    Ok(response)
}

// ─── POST /nodes/{id}/files/upload ──────────────────────────────────────────────

#[derive(Deserialize)]
struct FileUploadQuery {
    path: String,
}

async fn upload_file(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<FileUploadQuery>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = node_service(&state);
    service.upload_file(id, &q.path, body.to_vec()).await?;
    Ok(Json(json!({"message": "上传成功", "path": q.path})))
}

// ─── POST /nodes/register (agent endpoint, token-based auth) ─────────────────────

pub async fn register_node(
    State(state): State<AppState>,
    Json(req): Json<NodeRegisterRequest>,
) -> Result<Json<NodeInfo>, AppError> {
    if req.name.trim().is_empty() || req.host.trim().is_empty() || req.auth_token.trim().is_empty() {
        return Err(AppError::BadRequest("节点名称、主机和认证令牌不能为空".into()));
    }
    let node = state.node_repo.register(&req).await?;
    Ok(Json(node))
}

// ─── POST /nodes/heartbeat/{id} (agent endpoint, token-based auth) ───────────────

pub async fn node_heartbeat(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<NodeHeartbeatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.node_repo.heartbeat(id, &req).await?;
    Ok(Json(json!({"message": "心跳已更新"})))
}

// ─── Routes ─────────────────────────────────────────────────────────────────────

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/nodes/register", post(register_node))
        .route("/nodes/heartbeat/{id}", post(node_heartbeat))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/cluster", get(cluster_dashboard))
        .route("/nodes/batch-exec", post(batch_exec))
        .route("/nodes/{id}", get(get_node).delete(delete_node))
        .route("/nodes/{id}/exec", post(exec_on_node))
        .route("/nodes/{id}/files/list", get(list_files))
        .route("/nodes/{id}/files/download", get(download_file))
        .route("/nodes/{id}/files/upload", post(upload_file))
}