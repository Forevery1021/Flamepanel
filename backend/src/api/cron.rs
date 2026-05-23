use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::{CreateCronJobRequest, CronJob, CronJobLog, UpdateCronJobRequest};
use crate::middleware::auth::CurrentUser;

// ─── GET /cron ────────────────────────────────────────────────────────────────

pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<CronJob>>, AppError> {
    state.cron_repo.list_all().await.map(Json)
}

// ─── POST /cron ───────────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<CreateCronJobRequest>,
) -> Result<Json<CronJob>, AppError> {
    if req.command.is_none() && req.url.is_none() {
        return Err(AppError::BadRequest("必须指定 command 或 url".into()));
    }
    let job = state.cron_repo.create(&req).await?;
    // Recalculate next run time
    crate::application::CronService::recalc_next_run(state.cron_repo.clone(), job.id).await?;
    let updated = state.cron_repo.find_by_id(job.id).await?.unwrap();
    Ok(Json(updated))
}

// ─── PUT /cron/:id ────────────────────────────────────────────────────────────

pub async fn update(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCronJobRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.cron_repo.update(id, &req).await?;
    crate::application::CronService::recalc_next_run(state.cron_repo.clone(), id).await?;
    Ok(Json(json!({"message": "计划任务已更新"})))
}

// ─── DELETE /cron/:id ─────────────────────────────────────────────────────────

pub async fn delete(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.cron_repo.delete(id).await?;
    Ok(Json(json!({"message": "计划任务已删除"})))
}

// ─── POST /cron/:id/execute ───────────────────────────────────────────────────

pub async fn execute_now(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let job = state
        .cron_repo
        .find_by_id(id)
        .await?
        .ok_or(AppError::NotFound("计划任务不存在".into()))?;

    let started_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let (status, output) = crate::application::CronService::execute_job(&job).await;
    let finished_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    state
        .cron_repo
        .log(job.id, &status, output.as_deref(), &started_at, &finished_at)
        .await?;

    Ok(Json(json!({
        "message": "任务已执行",
        "status": status,
        "output": output,
    })))
}

// ─── GET /cron/:id/logs ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LogsQuery {
    limit: Option<i64>,
}

pub async fn job_logs(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
    Query(params): Query<LogsQuery>,
) -> Result<Json<Vec<CronJobLog>>, AppError> {
    let limit = params.limit.unwrap_or(50).min(200);
    state.cron_repo.list_logs(id, limit).await.map(Json)
}

// ─── Routes ───────────────────────────────────────────────────────────────────

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route(
            "/{id}",
            axum::routing::put(update).delete(delete),
        )
        .route("/{id}/execute", axum::routing::post(execute_now))
        .route("/{id}/logs", axum::routing::get(job_logs))
}
