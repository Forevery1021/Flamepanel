//! 内核日志端点（W0 · Doc/32 §4）。
//!
//! | 端点 | 权限 | 说明 |
//! |---|---|---|
//! | `GET /api/kernel-logs` | `log:read` | 概览：启用状态、目录、级别、保留期、文件列表 |
//! | `GET /api/kernel-logs/entries` | `log:read` | 倒序分页（最新在前）+ 级别 / 关键字 / target 过滤 |
//! | `GET /api/kernel-logs/stats` | `log:read` | 单文件五级计数 |
//! | `GET /api/kernel-logs/files/{name}/download` | `log:read` | 下载原始文件 |
//! | `DELETE /api/kernel-logs/files/{name}` | `log:delete` | 删除历史文件（当前文件拒绝） |
//! | `PUT /api/kernel-logs/level` | `settings:update` | 运行时调级（立即生效，重启回到 `OP_LOG_LEVEL`） |
use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::kernel_log_service::{KernelLogOverview, KernelLogStats};
use crate::core::error::AppError;
use crate::domain::kernel_log::{LogFilter, LogLevel};
use crate::infrastructure::logging::reader::LogPage;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// 分页查询参数
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct KernelLogEntriesParams {
    /// 文件名；缺省为当前正在写入的文件
    pub file: Option<String>,
    /// 级别（逗号分隔，如 `warn,error`）；缺省不过滤
    pub levels: Option<String>,
    /// 关键字（不区分大小写，匹配整行，含请求编号 / 用户名等 span 字段）
    pub q: Option<String>,
    /// target 前缀，如 `flame_kernel::api`
    pub target: Option<String>,
    /// 返回条数：缺省 200，范围 1–1000
    pub limit: Option<usize>,
    /// 续读游标（上一页的 `next_before`）
    pub before: Option<u64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct KernelLogStatsParams {
    /// 文件名
    pub file: String,
}

/// 运行时调级请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetKernelLogLevelRequest {
    /// `trace` / `debug` / `info` / `warn` / `error`
    pub level: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KernelLogLevelResponse {
    pub level: LogLevel,
}

/// `GET /api/kernel-logs` — 概览
#[utoipa::path(
    get,
    path = "/api/kernel-logs",
    tag = "kernel-logs",
    responses(
        (status = 200, description = "内核日志概览", body = KernelLogOverview),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 log:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn overview(State(state): State<AppState>) -> Result<Json<KernelLogOverview>, AppError> {
    Ok(Json(state.kernel_log_service.overview().await?))
}

/// `GET /api/kernel-logs/entries` — 倒序分页读取
#[utoipa::path(
    get,
    path = "/api/kernel-logs/entries",
    tag = "kernel-logs",
    params(KernelLogEntriesParams),
    responses(
        (status = 200, description = "日志分页（最新在前）", body = LogPage),
        (status = 400, description = "文件名或级别非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 log:read 权限"),
        (status = 404, description = "文件不存在或文件日志未启用"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn entries(
    State(state): State<AppState>,
    Query(p): Query<KernelLogEntriesParams>,
) -> Result<Json<LogPage>, AppError> {
    let filter = LogFilter::from_params(p.levels.as_deref(), p.q.as_deref(), p.target.as_deref())
        .map_err(AppError::BadRequest)?;
    Ok(Json(
        state
            .kernel_log_service
            .entries(p.file, filter, p.limit.unwrap_or(200), p.before)
            .await?,
    ))
}

/// `GET /api/kernel-logs/stats` — 五级计数
#[utoipa::path(
    get,
    path = "/api/kernel-logs/stats",
    tag = "kernel-logs",
    params(KernelLogStatsParams),
    responses(
        (status = 200, description = "级别计数", body = KernelLogStats),
        (status = 400, description = "文件名非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 log:read 权限"),
        (status = 404, description = "文件不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn stats(
    State(state): State<AppState>,
    Query(p): Query<KernelLogStatsParams>,
) -> Result<Json<KernelLogStats>, AppError> {
    Ok(Json(state.kernel_log_service.stats(p.file).await?))
}

/// `GET /api/kernel-logs/files/{name}/download` — 下载原始日志文件
#[utoipa::path(
    get,
    path = "/api/kernel-logs/files/{name}/download",
    tag = "kernel-logs",
    params(("name" = String, Path, description = "日志文件名")),
    responses(
        (status = 200, description = "原始 JSON Lines 文本", content_type = "text/plain"),
        (status = 400, description = "文件名非法或文件过大"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 log:read 权限"),
        (status = 404, description = "文件不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn download(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response, AppError> {
    let body = state.kernel_log_service.download(name.clone()).await?;
    let mut resp = body.into_response();
    let headers = resp.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    // 文件名已过白名单校验（仅 [A-Za-z0-9._-]），可安全放进头部
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(resp)
}

/// `DELETE /api/kernel-logs/files/{name}` — 删除历史日志文件
#[utoipa::path(
    delete,
    path = "/api/kernel-logs/files/{name}",
    tag = "kernel-logs",
    params(("name" = String, Path, description = "日志文件名")),
    responses(
        (status = 200, description = "已删除"),
        (status = 400, description = "文件名非法或为当前正在写入的文件"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 log:delete 权限"),
        (status = 404, description = "文件不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_file(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.kernel_log_service.delete(name).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `PUT /api/kernel-logs/level` — 运行时调整内核日志级别
#[utoipa::path(
    put,
    path = "/api/kernel-logs/level",
    tag = "kernel-logs",
    request_body = SetKernelLogLevelRequest,
    responses(
        (status = 200, description = "新级别已生效", body = KernelLogLevelResponse),
        (status = 400, description = "级别非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 settings:update 权限"),
        (status = 409, description = "日志系统未初始化（嵌入式 / 测试进程）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn set_level(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<SetKernelLogLevelRequest>,
) -> Result<Json<KernelLogLevelResponse>, AppError> {
    let level = LogLevel::parse(&req.level).ok_or_else(|| {
        AppError::BadRequest(format!(
            "invalid level '{}', expected trace/debug/info/warn/error",
            req.level
        ))
    })?;
    Ok(Json(KernelLogLevelResponse {
        level: state.kernel_log_service.set_level(level)?,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/kernel-logs", get(overview))
        .route("/api/kernel-logs/entries", get(entries))
        .route("/api/kernel-logs/stats", get(stats))
        .route("/api/kernel-logs/files/{name}/download", get(download))
        .route(
            "/api/kernel-logs/files/{name}",
            axum::routing::delete(delete_file),
        )
        .route("/api/kernel-logs/level", put(set_level))
}
