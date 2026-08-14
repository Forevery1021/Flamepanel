use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::ScheduledTask;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json,
};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub name: String,
    pub command: String,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub command: Option<String>,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct ToggleTaskRequest {
    pub enabled: bool,
}

/// 定时任务列表（分页）
#[utoipa::path(
    get,
    path = "/api/scheduled-tasks",
    tag = "scheduled_tasks",
    params(PaginationParams),
    responses(
        (status = 200, description = "任务列表", body = PaginatedResponse<ScheduledTask>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ScheduledTask>>, AppError> {
    Ok(Json(
        state.scheduled_task_service.list_tasks(&params).await?,
    ))
}

/// 创建定时任务
#[utoipa::path(
    post,
    path = "/api/scheduled-tasks",
    tag = "scheduled_tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 200, description = "创建成功", body = ScheduledTask),
        (status = 400, description = "参数错误"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_task(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateTaskRequest>,
) -> Result<Json<ScheduledTask>, AppError> {
    let task = ScheduledTask {
        id: 0,
        name: req.name,
        command: req.command,
        schedule: req.schedule.unwrap_or_else(|| "* * * * *".into()),
        enabled: req.enabled.unwrap_or(true),
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    Ok(Json(state.scheduled_task_service.create_task(task).await?))
}

/// 更新定时任务
#[utoipa::path(
    put,
    path = "/api/scheduled-tasks/{id}",
    tag = "scheduled_tasks",
    params(("id" = i64, Path, description = "任务 ID")),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "更新成功", body = ScheduledTask),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTaskRequest>,
) -> Result<Json<ScheduledTask>, AppError> {
    let mut existing = state.scheduled_task_service.get_task(id).await?;
    if let Some(name) = req.name {
        existing.name = name;
    }
    if let Some(command) = req.command {
        existing.command = command;
    }
    if let Some(schedule) = req.schedule {
        existing.schedule = schedule;
    }
    if let Some(enabled) = req.enabled {
        existing.enabled = enabled;
    }
    Ok(Json(
        state.scheduled_task_service.update_task(&existing).await?,
    ))
}

/// 删除定时任务
#[utoipa::path(
    delete,
    path = "/api/scheduled-tasks/{id}",
    tag = "scheduled_tasks",
    params(("id" = i64, Path, description = "任务 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    state.scheduled_task_service.delete_task(id).await?;
    Ok(Json(()))
}

/// 立即执行定时任务
#[utoipa::path(
    post,
    path = "/api/scheduled-tasks/{id}/run",
    tag = "scheduled_tasks",
    params(("id" = i64, Path, description = "任务 ID")),
    responses(
        (status = 200, description = "执行结果", body = ScheduledTask),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn run_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduledTask>, AppError> {
    Ok(Json(state.scheduled_task_service.run_now(id).await?))
}

pub async fn toggle_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<ToggleTaskRequest>,
) -> Result<Json<ScheduledTask>, AppError> {
    Ok(Json(
        state
            .scheduled_task_service
            .toggle_enabled(id, req.enabled)
            .await?,
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/scheduled-tasks", get(list_tasks).post(create_task))
        .route(
            "/api/scheduled-tasks/{id}",
            axum::routing::get(crate::api::handler::scheduled_task::get_task)
                .put(update_task)
                .delete(delete_task),
        )
        .route(
            "/api/scheduled-tasks/{id}/run",
            axum::routing::post(run_task),
        )
        .route(
            "/api/scheduled-tasks/{id}/toggle",
            axum::routing::post(toggle_task),
        )
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduledTask>, AppError> {
    Ok(Json(state.scheduled_task_service.get_task(id).await?))
}
