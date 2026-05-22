use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};
use crate::core::error::AppError;
use crate::middleware::middleware_auth::CurrentUser;

#[derive(Serialize)]
pub struct DockerContainer {
    id: String,
    name: String,
    image: String,
    status: String,
    state: String,
    ports: String,
}

#[derive(Deserialize)]
pub struct ContainerAction {
    id: String,
    action: String, // start | stop | restart | remove
}

pub fn routes() -> Router {
    Router::new()
        .route("/containers", get(list_containers))
        .route("/action", post(container_action))
}

async fn list_containers(
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<Vec<DockerContainer>>, AppError> {
    // TODO: 实现 Docker 容器列表
    Ok(Json(vec![]))
}

async fn container_action(
    CurrentUser(_claims): CurrentUser,
    Json(action): Json<ContainerAction>,
) -> Result<(), AppError> {
    // TODO: 实现 Docker 容器操作
    match action.action.as_str() {
        "start" | "stop" | "restart" => Ok(()),
        _ => Err(AppError::BadRequest("不支持的操作".into())),
    }
}