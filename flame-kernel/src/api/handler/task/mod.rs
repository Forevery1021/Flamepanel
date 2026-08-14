//! 统一 Task 查询 / 取消 API（Phase B1 扩展：供前端展示统一任务进度）。
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::runtime::task_state::{TaskRecord, TaskState};
use axum::{extract::State, Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskRecord>,
    pub total: usize,
}

/// 列出全部任务（可按状态过滤）。
#[utoipa::path(
    get,
    path = "/api/tasks",
    tag = "task",
    params(("state" = Option<String>, Query, description = "按状态过滤：pending/running/success/failed/cancelled")),
    responses(
        (status = 200, description = "任务列表", body = TaskListResponse),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<TaskListResponse>, AppError> {
    let tasks = match params.get("state").map(|s| s.as_str()) {
        Some("pending") => state.task_service.list_by_state(TaskState::Pending),
        Some("running") => state.task_service.list_by_state(TaskState::Running),
        Some("success") => state.task_service.list_by_state(TaskState::Success),
        Some("failed") => state.task_service.list_by_state(TaskState::Failed),
        Some("cancelled") => state.task_service.list_by_state(TaskState::Cancelled),
        _ => state.task_service.list_tasks(),
    };
    let total = tasks.len();
    Ok(Json(TaskListResponse { tasks, total }))
}

/// 查询单个任务。
#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    tag = "task",
    params(("id" = u64, Path, description = "任务 id")),
    responses(
        (status = 200, description = "任务详情", body = TaskRecord),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_task(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<TaskRecord>, AppError> {
    let task = state.task_service.get_task(id)?;
    Ok(Json(task))
}

/// 取消任务（`Pending → Cancelled` / `Running → Cancelled`）。
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/cancel",
    tag = "task",
    params(("id" = u64, Path, description = "任务 id")),
    responses(
        (status = 200, description = "已取消", body = TaskRecord),
        (status = 404, description = "任务不存在"),
        (status = 409, description = "任务已处于终态，无法取消"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn cancel_task(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<TaskRecord>, AppError> {
    let task = state.task_service.cancel_task(id)?;
    Ok(Json(task))
}

/// 清理全部终态任务。
#[utoipa::path(
    post,
    path = "/api/tasks/prune",
    tag = "task",
    responses(
        (status = 200, description = "清理数量"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn prune_tasks(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = state.task_service.prune_terminal();
    Ok(Json(serde_json::json!({ "pruned": count })))
}

/// 路由表（集中注册于 routes.rs 组合根）。
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/tasks", axum::routing::get(list_tasks))
        .route("/api/tasks/prune", axum::routing::post(prune_tasks))
        .route("/api/tasks/{id}", axum::routing::get(get_task))
        .route("/api/tasks/{id}/cancel", axum::routing::post(cancel_task))
}
