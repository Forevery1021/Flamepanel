use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::middleware::auth::CurrentUser;

// ─── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: Vec<String>,
    pub created: String,
}

#[derive(Debug, Deserialize)]
pub struct ContainerActionRequest {
    pub id: String,
    pub action: String, // start | stop | restart
}

#[derive(Debug, Deserialize)]
pub struct ContainerLogsQuery {
    pub id: String,
    pub tail: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ContainerLogsResponse {
    pub container_id: String,
    pub logs: String,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ImageInfo {
    pub repository: String,
    pub tag: String,
    pub id: String,
    pub size: String,
    pub created: String,
}

// ─── Routes ───────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/containers", get(list_containers))
        .route("/containers/action", post(container_action))
        .route("/containers/logs", get(container_logs))
        .route("/images", get(list_images))
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/docker/containers
/// 获取所有容器列表（包括已停止的）
async fn list_containers(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<Vec<DockerContainer>>, AppError> {
    let output = Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}|{{.State}}|{{.Ports}}|{{.CreatedAt}}"])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker 命令执行失败: {e}")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Docker 错误: {err}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<DockerContainer> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            DockerContainer {
                id: parts.first().map(|s| s.to_string()).unwrap_or_default(),
                name: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                image: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                status: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
                state: parts.get(4).map(|s| s.to_string()).unwrap_or_default(),
                ports: parts.get(5).map(|s| {
                    if s.is_empty() { vec![] } else { s.split(',').map(|p| p.trim().to_string()).collect() }
                }).unwrap_or_default(),
                created: parts.get(6).map(|s| s.to_string()).unwrap_or_default(),
            }
        })
        .collect();

    Ok(Json(containers))
}

/// POST /api/docker/containers/action
/// 对容器执行 start / stop / restart 操作
async fn container_action(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(req): Json<ContainerActionRequest>,
) -> Result<Json<ActionResponse>, AppError> {
    let action = req.action.as_str();
    match action {
        "start" | "stop" | "restart" => {}
        _ => return Err(AppError::BadRequest("不支持的操作，可用: start/stop/restart".into())),
    }

    let output = Command::new("docker")
        .args([action, &req.id])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker 命令执行失败: {e}")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("操作失败: {err}")));
    }

    tracing::info!("用户 '{}' 对容器 {} 执行了 {}", _claims.sub, req.id, action);

    Ok(Json(ActionResponse {
        success: true,
        message: format!("容器 {} 操作成功", action),
    }))
}

/// GET /api/docker/containers/logs?id=xxx&tail=200
/// 获取容器日志
async fn container_logs(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<ContainerLogsQuery>,
) -> Result<Json<ContainerLogsResponse>, AppError> {
    if query.id.is_empty() {
        return Err(AppError::BadRequest("容器ID不能为空".into()));
    }

    let tail = query.tail.unwrap_or(200).to_string();
    let output = Command::new("docker")
        .args(["logs", "--tail", &tail, &query.id])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker 命令执行失败: {e}")))?;

    let logs = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(Json(ContainerLogsResponse {
        container_id: query.id,
        logs,
    }))
}

/// GET /api/docker/images
/// 获取本地镜像列表
async fn list_images(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<Vec<ImageInfo>>, AppError> {
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}|{{.Tag}}|{{.ID}}|{{.Size}}|{{.CreatedAt}}"])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker 命令执行失败: {e}")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Docker 错误: {err}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let images: Vec<ImageInfo> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            ImageInfo {
                repository: parts.first().map(|s| s.to_string()).unwrap_or_default(),
                tag: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                id: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                size: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
                created: parts.get(4).map(|s| s.to_string()).unwrap_or_default(),
            }
        })
        .collect();

    Ok(Json(images))
}
