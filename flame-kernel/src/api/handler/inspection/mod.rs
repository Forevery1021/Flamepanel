//! 系统体检端点（S-17 · Doc/30 M-C）：内核信息 + 风险 / 漏洞分析。
//!
//! - `GET /api/inspection`（`toolbox:read`）：最近一次报告，服务端 10 min 缓存；无缓存时同步执行
//!   一次——安装后首次进入即得到"安装后检测"结果；
//! - `POST /api/inspection/run`（`toolbox:execute`）：强制重新体检。
//!
//! 判定逻辑全部在 `domain::inspection`（纯函数），采集在 `infrastructure::system_probe`
//! （文件优先、命令经 `PrivilegedCommandRunner` 且带超时）。
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::inspection::InspectionReport;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};

/// `GET /api/inspection` — 系统体检报告（内核信息 + 风险项）
///
/// 命中缓存时 `cache_age_secs > 0`；`findings[].code` 为稳定编码，前端据此展示三语言标题与建议，
/// `evidence` 为语言无关的原始取证。`heuristic = true` 的结论依赖内置生命周期表（`data_version`）。
#[utoipa::path(
    get,
    path = "/api/inspection",
    tag = "inspection",
    responses(
        (status = 200, description = "系统体检报告", body = InspectionReport),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 toolbox:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn report(State(state): State<AppState>) -> Result<Json<InspectionReport>, AppError> {
    Ok(Json(state.inspection_service.report(false).await))
}

/// `POST /api/inspection/run` — 立即重新体检（忽略缓存）
///
/// 采集有 15 s 总预算，单条外部命令 3 s 超时；超时或不可用的项以 `unknown` 呈现而不是让请求失败。
#[utoipa::path(
    post,
    path = "/api/inspection/run",
    tag = "inspection",
    responses(
        (status = 200, description = "新生成的体检报告", body = InspectionReport),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 toolbox:execute 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn run(State(state): State<AppState>) -> Result<Json<InspectionReport>, AppError> {
    Ok(Json(state.inspection_service.report(true).await))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/inspection", get(report))
        .route("/api/inspection/run", post(run))
}
