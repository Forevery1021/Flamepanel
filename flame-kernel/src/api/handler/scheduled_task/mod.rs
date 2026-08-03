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

pub async fn list_tasks(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ScheduledTask>>, AppError> {
    Ok(Json(
        state.scheduled_task_service.list_tasks(&params).await?,
    ))
}

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

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    state.scheduled_task_service.delete_task(id).await?;
    Ok(Json(()))
}

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

#[derive(serde::Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub command: String,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub command: Option<String>,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize)]
pub struct ToggleTaskRequest {
    pub enabled: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/scheduled-tasks", get(list_tasks).post(create_task))
        .route(
            "/api/scheduled-tasks/:id",
            axum::routing::get(crate::api::handler::scheduled_task::get_task)
                .put(update_task)
                .delete(delete_task),
        )
        .route(
            "/api/scheduled-tasks/:id/run",
            axum::routing::post(run_task),
        )
        .route(
            "/api/scheduled-tasks/:id/toggle",
            axum::routing::post(toggle_task),
        )
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduledTask>, AppError> {
    Ok(Json(state.scheduled_task_service.get_task(id).await?))
}
