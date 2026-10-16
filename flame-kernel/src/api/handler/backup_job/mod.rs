use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::backup_job_service::BackupJobService;
use crate::core::error::AppError;
use crate::domain::entity::BackupJob;
use axum::Router;
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json,
};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateJobRequest {
    pub name: String,
    /// 备份范围：panel（面板库）| app（应用目录）
    pub scope: String,
    /// 范围内具体对象：panel 省略；app 为应用 key
    #[serde(default)]
    pub scope_key: Option<String>,
    /// 标准 5 字段 cron 表达式
    pub schedule: String,
    /// 目标备份账号 ID
    pub target_account_id: i64,
    /// 备份保留份数（0 = 用默认 7）
    #[serde(default)]
    pub retention: Option<i64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateJobRequest {
    pub name: Option<String>,
    pub scope: Option<String>,
    pub scope_key: Option<String>,
    pub schedule: Option<String>,
    pub target_account_id: Option<i64>,
    pub retention: Option<i64>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct ToggleJobRequest {
    pub enabled: bool,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct RestoreFromCloudRequest {
    /// 目标备份账号 ID
    pub account_id: i64,
    /// 远端对象 key（如 panel/flamepanel-20260907.db）
    pub remote_key: String,
}

fn job_response(j: &BackupJob) -> JobDto {
    JobDto {
        id: j.id,
        name: j.name.clone(),
        scope: j.scope.clone(),
        scope_key: j.scope_key.clone(),
        schedule: j.schedule.clone(),
        target_account_id: j.target_account_id,
        retention: j.retention,
        enabled: j.enabled,
        last_status: j.last_status.clone(),
        last_message: j.last_message.clone(),
        last_remote_key: j.last_remote_key.clone(),
        last_run_at: j.last_run_at,
        next_run_at: j.next_run_at,
        created_at: j.created_at,
        updated_at: j.updated_at,
    }
}

/// 备份任务响应
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct JobDto {
    pub id: i64,
    pub name: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_key: Option<String>,
    pub schedule: String,
    pub target_account_id: i64,
    pub retention: i64,
    pub enabled: bool,
    /// never | success | failed
    pub last_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_remote_key: Option<String>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn svc(state: &AppState) -> &BackupJobService {
    &state.services.backup_job_service
}

/// 备份任务列表
#[utoipa::path(
    get,
    path = "/api/backup-jobs",
    tag = "backup_jobs",
    responses(
        (status = 200, description = "任务列表", body = Vec<JobDto>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_jobs(State(state): State<AppState>) -> Result<Json<Vec<JobDto>>, AppError> {
    let jobs = svc(&state).list_jobs().await?;
    Ok(Json(jobs.iter().map(job_response).collect()))
}

/// 创建备份任务
#[utoipa::path(
    post,
    path = "/api/backup-jobs",
    tag = "backup_jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 200, description = "创建成功", body = JobDto),
        (status = 400, description = "参数错误（cron 非法 / 账号不存在）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_job(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateJobRequest>,
) -> Result<Json<JobDto>, AppError> {
    let job = BackupJob {
        retention: req.retention.unwrap_or(7),
        scope_key: req.scope_key.clone(),
        name: req.name.clone(),
        scope: req.scope.clone(),
        schedule: req.schedule.clone(),
        target_account_id: req.target_account_id,
        enabled: req.enabled,
        ..Default::default()
    };
    let created = svc(&state).create_job(job).await?;
    Ok(Json(job_response(&created)))
}

/// 更新备份任务
#[utoipa::path(
    put,
    path = "/api/backup-jobs/{id}",
    tag = "backup_jobs",
    params(("id" = i64, Path, description = "任务 ID")),
    request_body = UpdateJobRequest,
    responses(
        (status = 200, description = "更新成功", body = JobDto),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateJobRequest>,
) -> Result<Json<JobDto>, AppError> {
    let existing = svc(&state).get_job(id).await?;
    let job = BackupJob {
        id,
        name: req.name.unwrap_or(existing.name),
        scope: req.scope.unwrap_or(existing.scope),
        scope_key: req.scope_key.or(existing.scope_key),
        schedule: req.schedule.unwrap_or(existing.schedule),
        target_account_id: req.target_account_id.unwrap_or(existing.target_account_id),
        retention: req.retention.unwrap_or(existing.retention),
        enabled: req.enabled.unwrap_or(existing.enabled),
        ..existing
    };
    let updated = svc(&state).update_job(&job).await?;
    Ok(Json(job_response(&updated)))
}

/// 删除备份任务
#[utoipa::path(
    delete,
    path = "/api/backup-jobs/{id}",
    tag = "backup_jobs",
    params(("id" = i64, Path, description = "任务 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    svc(&state).delete_job(id).await?;
    Ok(Json(()))
}

/// 启停备份任务
#[utoipa::path(
    put,
    path = "/api/backup-jobs/{id}/enabled",
    tag = "backup_jobs",
    params(("id" = i64, Path, description = "任务 ID")),
    request_body = ToggleJobRequest,
    responses(
        (status = 200, description = "已更新", body = JobDto),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn toggle_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<ToggleJobRequest>,
) -> Result<Json<JobDto>, AppError> {
    let job = svc(&state).toggle_enabled(id, req.enabled).await?;
    Ok(Json(job_response(&job)))
}

/// 立即执行一次
#[utoipa::path(
    post,
    path = "/api/backup-jobs/{id}/run",
    tag = "backup_jobs",
    params(("id" = i64, Path, description = "任务 ID")),
    responses(
        (status = 200, description = "执行完成，返回任务（含执行结果）", body = JobDto),
        (status = 404, description = "任务不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn run_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<JobDto>, AppError> {
    let job = svc(&state).run_now(id).await?;
    Ok(Json(job_response(&job)))
}

/// 从云端账号直接拉取并恢复（按 key 前缀分流：面板库 .db / 应用数据 app/<key>/*.tar.gz）
#[utoipa::path(
    post,
    path = "/api/backup-jobs/restore-cloud",
    tag = "backup_jobs",
    request_body = RestoreFromCloudRequest,
    responses(
        (status = 200, description = "恢复完成，返回范围/对象名与生效提示", body = RestoreFromCloudResponse),
        (status = 400, description = "参数错误（key 非法 / 包格式不支持 / 包内容不安全）"),
        (status = 404, description = "账号、远端对象或目标应用不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn restore_from_cloud(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<RestoreFromCloudRequest>,
) -> Result<Json<RestoreFromCloudResponse>, AppError> {
    let report = svc(&state)
        .restore_from_cloud(req.account_id, &req.remote_key)
        .await?;
    Ok(Json(RestoreFromCloudResponse {
        scope: report.scope,
        filename: report.filename,
        message: report.message,
    }))
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct RestoreFromCloudResponse {
    /// panel（面板库）| app（应用数据）
    pub scope: String,
    /// 还原对象名（面板库为 .db 文件名，应用为备份包文件名）
    pub filename: String,
    /// 面向用户的下一步提示（是否需重启）
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/backup-jobs", get(list_jobs).post(create_job))
        .route("/api/backup-jobs/{id}", put(update_job).delete(delete_job))
        .route("/api/backup-jobs/{id}/enabled", put(toggle_job))
        .route("/api/backup-jobs/{id}/run", post(run_job))
        .route("/api/backup-jobs/restore-cloud", post(restore_from_cloud))
}
