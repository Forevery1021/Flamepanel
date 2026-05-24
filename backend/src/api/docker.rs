use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use bollard::container::{
    ListContainersOptions, LogsOptions, StartContainerOptions,
    StopContainerOptions, RestartContainerOptions,
};
use bollard::image::ListImagesOptions;
use futures_util::StreamExt;

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

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn format_size(bytes: i64) -> String {
    if bytes > 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1e9)
    } else if bytes > 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1e6)
    } else if bytes > 1_000 {
        format!("{:.2} KB", bytes as f64 / 1e3)
    } else {
        format!("{} B", bytes)
    }
}

fn unix_ts_to_string(ts: i64) -> String {
    if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        String::new()
    }
}

fn split_image_tag(repo_tags: &[String]) -> (String, String) {
    let full = repo_tags.first().map(|s| s.as_str()).unwrap_or("<none>");
    match full.split_once(':') {
        Some((repo, tag)) => (repo.to_string(), tag.to_string()),
        None => (full.to_string(), "latest".to_string()),
    }
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/docker/containers
async fn list_containers(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<DockerContainer>>, AppError> {
    let options = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };

    let containers = state
        .docker
        .list_containers(Some(options))
        .await
        .map_err(|e| AppError::Internal(format!("Docker 容器列表获取失败: {e}")))?;

    let result: Vec<DockerContainer> = containers
        .into_iter()
        .map(|c| {
            let name = c.names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_default();
            let ports: Vec<String> = c.ports
                .unwrap_or_default()
                .into_iter()
                .filter_map(|p| {
                    p.public_port.map(|pub_port| {
                        format!("{}:{}", pub_port, p.private_port)
                    })
                })
                .collect();
            let created_ts = c.created.unwrap_or(0);
            DockerContainer {
                id: c.id.unwrap_or_default(),
                name,
                image: c.image.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                ports,
                created: unix_ts_to_string(created_ts),
            }
        })
        .collect();

    Ok(Json(result))
}

/// POST /api/docker/containers/action
async fn container_action(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<ContainerActionRequest>,
) -> Result<Json<ActionResponse>, AppError> {
    let docker = &state.docker;

    match req.action.as_str() {
        "start" => {
            docker
                .start_container(&req.id, None::<StartContainerOptions<String>>)
                .await
                .map_err(|e| AppError::Internal(format!("启动容器失败: {e}")))?;
        }
        "stop" => {
            docker
                .stop_container(&req.id, None::<StopContainerOptions>)
                .await
                .map_err(|e| AppError::Internal(format!("停止容器失败: {e}")))?;
        }
        "restart" => {
            docker
                .restart_container(&req.id, None::<RestartContainerOptions>)
                .await
                .map_err(|e| AppError::Internal(format!("重启容器失败: {e}")))?;
        }
        _ => return Err(AppError::BadRequest("不支持的操作，可用: start/stop/restart".into())),
    }

    tracing::info!("用户 '{}' 对容器 {} 执行了 {}", _user.0.sub, req.id, req.action);

    Ok(Json(ActionResponse {
        success: true,
        message: format!("容器 {} 操作成功", req.action),
    }))
}

/// GET /api/docker/containers/logs?id=xxx&tail=200
async fn container_logs(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(query): Query<ContainerLogsQuery>,
) -> Result<Json<ContainerLogsResponse>, AppError> {
    if query.id.is_empty() {
        return Err(AppError::BadRequest("容器ID不能为空".into()));
    }

    let tail = query.tail.unwrap_or(200).to_string();
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: tail,
        ..Default::default()
    };

    let mut stream = state.docker.logs(&query.id, Some(options));

    let mut logs = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bollard::container::LogOutput::StdOut { message })
            | Ok(bollard::container::LogOutput::StdErr { message }) => {
                logs.push(message);
            }
            Ok(_) => {}
            Err(e) => {
                return Err(AppError::Internal(format!("读取日志流失败: {e}")));
            }
        }
    }

    let logs_str = String::from_utf8_lossy(&logs.concat()).to_string();

    Ok(Json(ContainerLogsResponse {
        container_id: query.id,
        logs: logs_str,
    }))
}

/// GET /api/docker/images
async fn list_images(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<ImageInfo>>, AppError> {
    let options = ListImagesOptions::<String> {
        all: true,
        ..Default::default()
    };

    let images = state
        .docker
        .list_images(Some(options))
        .await
        .map_err(|e| AppError::Internal(format!("Docker 镜像列表获取失败: {e}")))?;

    let result: Vec<ImageInfo> = images
        .into_iter()
        .map(|img| {
            let (repository, tag) = split_image_tag(&img.repo_tags);
            ImageInfo {
                repository,
                tag,
                id: img.id.chars().take(12).collect(),
                size: format_size(img.size),
                created: unix_ts_to_string(img.created),
            }
        })
        .collect();

    Ok(Json(result))
}
