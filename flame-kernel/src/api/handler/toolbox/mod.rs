//! 系统工具箱 · 进程管理端点（S-12 · Doc/30 M-C）。
//!
//! - `GET /api/toolbox/processes`（`toolbox:read`）：进程列表（关键字 / 用户过滤、排序、截断），
//!   每行带 `protected`（不可结束的原因）；数据来自 `MetricsHub` 进程表（2 s TTL + 单飞）；
//! - `POST /api/toolbox/processes/{pid}/kill`（`toolbox:execute`）：发送 TERM / KILL / HUP / INT。
//!   拒绝 pid ≤ 1、内核线程、面板自身及其祖先进程；可带 `started_at` 防 pid 复用（不一致 → 409）。
//!   写操作由审计中间件自动记入操作日志。
//! - `GET /api/toolbox/host`（`toolbox:read`）：主机详情只读汇总（`domain::host_info`）。
use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::toolbox_service::{KillProcessResponse, ProcessListResponse};
use crate::core::error::AppError;
use crate::domain::host_info::HostDetails;
use crate::domain::toolbox::{ProcessQuery, ProcessSignal, ProcessSort, SortOrder};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

/// 进程列表查询参数
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct ProcessListParams {
    /// 关键字：pid 精确匹配，或进程名 / 用户 / 命令行包含（不区分大小写）
    pub q: Option<String>,
    /// 仅显示该用户的进程
    pub user: Option<String>,
    /// 排序字段：`cpu`（缺省）/ `memory` / `pid` / `name` / `started`
    pub sort: Option<String>,
    /// 排序方向：`desc`（缺省）/ `asc`
    pub order: Option<String>,
    /// 返回条数：缺省 100，范围 1–500
    pub limit: Option<usize>,
    /// 是否包含内核线程（缺省 false）
    pub include_kernel: Option<bool>,
}

/// 结束进程请求体（字段均可省略，`{}` 即发送 TERM）
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct KillProcessRequest {
    /// 信号：`TERM`（缺省）/ `KILL` / `HUP` / `INT`
    #[serde(default)]
    pub signal: ProcessSignal,
    /// 列表中看到的启动时间（Unix 秒）；与当前进程不一致时返回 409，防止误杀复用了 pid 的新进程
    pub started_at: Option<u64>,
}

fn parse_sort(raw: Option<&str>) -> Result<Option<ProcessSort>, AppError> {
    raw.map(|s| match s.trim().to_ascii_lowercase().as_str() {
        "cpu" => Ok(ProcessSort::Cpu),
        "memory" | "mem" => Ok(ProcessSort::Memory),
        "pid" => Ok(ProcessSort::Pid),
        "name" => Ok(ProcessSort::Name),
        "started" => Ok(ProcessSort::Started),
        other => Err(AppError::BadRequest(format!(
            "invalid sort '{other}', expected one of cpu/memory/pid/name/started"
        ))),
    })
    .transpose()
}

fn parse_order(raw: Option<&str>) -> Result<Option<SortOrder>, AppError> {
    raw.map(|s| match s.trim().to_ascii_lowercase().as_str() {
        "asc" => Ok(SortOrder::Asc),
        "desc" => Ok(SortOrder::Desc),
        other => Err(AppError::BadRequest(format!(
            "invalid order '{other}', expected asc/desc"
        ))),
    })
    .transpose()
}

/// `GET /api/toolbox/processes` — 进程列表
#[utoipa::path(
    get,
    path = "/api/toolbox/processes",
    tag = "toolbox",
    params(ProcessListParams),
    responses(
        (status = 200, description = "进程列表", body = ProcessListResponse),
        (status = 400, description = "sort / order 非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 toolbox:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_processes(
    State(state): State<AppState>,
    Query(p): Query<ProcessListParams>,
) -> Result<Json<ProcessListResponse>, AppError> {
    let query = ProcessQuery::normalized(
        p.q.as_deref(),
        p.user.as_deref(),
        parse_sort(p.sort.as_deref())?,
        parse_order(p.order.as_deref())?,
        p.limit,
        p.include_kernel.unwrap_or(false),
    );
    Ok(Json(state.toolbox_service.list(&query).await))
}

/// `POST /api/toolbox/processes/{pid}/kill` — 向进程发送信号
#[utoipa::path(
    post,
    path = "/api/toolbox/processes/{pid}/kill",
    tag = "toolbox",
    params(("pid" = u32, Path, description = "目标进程 pid")),
    request_body = KillProcessRequest,
    responses(
        (status = 200, description = "信号已送达", body = KillProcessResponse),
        (status = 400, description = "受保护进程（init / 内核线程 / 面板自身及祖先）或请求体非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 toolbox:execute 权限；或面板系统权限不足（message 含 insufficient privilege）"),
        (status = 404, description = "进程不存在"),
        (status = 409, description = "started_at 不一致（pid 已被复用）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn kill_process(
    State(state): State<AppState>,
    Path(pid): Path<u32>,
    ApiJson(req): ApiJson<KillProcessRequest>,
) -> Result<Json<KillProcessResponse>, AppError> {
    Ok(Json(
        state
            .toolbox_service
            .kill(pid, req.signal, req.started_at)
            .await?,
    ))
}

/// `GET /api/toolbox/host` — 主机详情
#[utoipa::path(
    get,
    path = "/api/toolbox/host",
    tag = "toolbox",
    responses(
        (status = 200, description = "主机详情：系统 / CPU / 内存 / 时区 / DNS / hosts / 网卡 / 磁盘 / 面板自身", body = HostDetails),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 toolbox:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn host_details(State(state): State<AppState>) -> Json<HostDetails> {
    Json(state.toolbox_service.host_details().await)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/toolbox/host", get(host_details))
        .route("/api/toolbox/processes", get(list_processes))
        .route("/api/toolbox/processes/{pid}/kill", post(kill_process))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_and_order_parsing() {
        assert_eq!(parse_sort(None).unwrap(), None);
        assert_eq!(parse_sort(Some("MEM")).unwrap(), Some(ProcessSort::Memory));
        assert_eq!(
            parse_sort(Some("started")).unwrap(),
            Some(ProcessSort::Started)
        );
        assert!(matches!(
            parse_sort(Some("rss")),
            Err(AppError::BadRequest(_))
        ));
        assert_eq!(parse_order(Some(" Asc ")).unwrap(), Some(SortOrder::Asc));
        assert!(matches!(
            parse_order(Some("up")),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn kill_request_defaults_to_term() {
        let req: KillProcessRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(req.signal, ProcessSignal::Term);
        assert!(req.started_at.is_none());
        assert!(serde_json::from_str::<KillProcessRequest>(r#"{"signal":"STOP"}"#).is_err());
    }
}
