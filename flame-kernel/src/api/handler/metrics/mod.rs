use crate::api::types::AppState;
use crate::application::metrics_history_service::{HistoryRange, MetricsHistoryResponse};
use crate::core::error::AppError;
use axum::Json;
use axum::Router;
use axum::{
    extract::{Query, State},
    http::header,
    response::IntoResponse,
};
use serde::Deserialize;
use std::fmt::Write as _;
use utoipa::IntoParams;

pub use crate::infrastructure::metrics::ProcessEntry;

/// 监控历史查询参数（S-07）
#[derive(Debug, Deserialize, IntoParams)]
pub struct MetricsHistoryQuery {
    /// 时间范围：`1h` / `6h` / `24h`（1 min 点）或 `7d` / `30d`（1 h 点）；缺省 `24h`
    pub range: Option<String>,
}

/// `GET /api/metrics/history` — 监控历史降采样曲线（需 `node:read`）
///
/// 1 min 点由 3 s 采样在内存聚合后落库（保留 7 d），1 h 点由整点 rollup 得到（保留 90 d）；
/// 响应带 `step_secs`，前端按桶宽绘制。刚启动或历史不足时点数少于窗口理论值。
#[utoipa::path(
    get,
    path = "/api/metrics/history",
    tag = "metrics",
    params(MetricsHistoryQuery),
    responses(
        (status = 200, description = "监控历史点列", body = MetricsHistoryResponse),
        (status = 400, description = "range 非法"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn history(
    State(state): State<AppState>,
    Query(q): Query<MetricsHistoryQuery>,
) -> Result<Json<MetricsHistoryResponse>, AppError> {
    let raw = q.range.as_deref().unwrap_or("24h");
    let range = HistoryRange::parse(raw).ok_or_else(|| {
        AppError::BadRequest(format!(
            "invalid range '{}', expected one of {}",
            raw,
            HistoryRange::ALL.join("/")
        ))
    })?;
    let now = chrono::Utc::now().timestamp();
    Ok(Json(state.metrics_history_service.query(range, now).await?))
}

// P-01：经 `MetricsHub::top_processes` 读取——只刷新进程 CPU/内存字段 + 2 s TTL 缓存，
// 多个仪表盘并发轮询只触发一次 /proc 扫描，且不再干扰采集循环的 CPU 差分窗口。
// 注意：下面两行 doc 会进入 OpenAPI summary（frontend/openapi.json 漂移门禁），改动需同步重导快照。
/// `GET /api/metrics/processes` — 按 CPU 占用排序的进程 TOP 5（免认证）
#[utoipa::path(
    get,
    path = "/api/metrics/processes",
    tag = "metrics",
    responses(
        (status = 200, description = "进程 TOP 列表", body = Vec<ProcessEntry>),
    )
)]
/// F-17: processes 接口复用共享的 sysinfo::System 实例，避免每请求全量重建。
pub async fn processes(State(state): State<AppState>) -> Result<Json<Vec<ProcessEntry>>, AppError> {
    let top = state
        .metrics
        .top_processes(crate::infrastructure::metrics::DEFAULT_TOP_N)
        .await;
    Ok(Json(top.as_ref().clone()))
}

/// `GET /metrics` — Prometheus 文本格式指标（面板自身系统指标）
///
/// Stage4.3：暴露 Prometheus 兼容的文本格式（`text/plain; version=0.0.4`），
/// 供 Prometheus/Grafana 直接抓取。指标来自最近一次 `MetricsSnapshot`
/// （3s 周期采集）。公开可读（无敏感信息）。
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "metrics",
    responses(
        (status = 200, description = "Prometheus 文本格式指标", content_type = "text/plain"),
    )
)]
pub async fn prometheus_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let latest = state.metrics.latest();
    let mut out = String::with_capacity(2048);
    gauge(
        &mut out,
        "flamepanel_up",
        "Whether the panel is up (1).",
        1u64,
    );
    gauge(
        &mut out,
        "flamepanel_uptime_seconds",
        "Panel uptime in seconds.",
        state.metrics.uptime().as_secs(),
    );
    out.push_str("# HELP flamepanel_info Static build info.\n# TYPE flamepanel_info gauge\n");
    let _ = writeln!(out, "flamepanel_info{{version=\"{}\"}} 1", crate::VERSION);
    let _ = writeln!(
        out,
        "# HELP flamepanel_metrics_subscribers Active /ws/metrics subscribers.\n# TYPE flamepanel_metrics_subscribers gauge\nflamepanel_metrics_subscribers {}",
        state.metrics.subscriber_count()
    );
    // S-08：推送流背压计数（滞后事件 / 丢帧 / 慢消费者断开），按 stream 标签区分
    crate::api::ws_backpressure::ws_stats().write_prometheus(&mut out);

    if let Some(s) = latest {
        const MIB: f64 = 1024.0 * 1024.0;
        const GIB: f64 = MIB * 1024.0;
        gauge(
            &mut out,
            "flamepanel_cpu_usage_percent",
            "CPU usage percent (global).",
            s.cpu_usage,
        );
        gauge(
            &mut out,
            "flamepanel_cpu_cores",
            "Number of CPU cores.",
            s.cpu_cores,
        );
        gauge(
            &mut out,
            "flamepanel_memory_usage_percent",
            "Memory usage percent.",
            s.memory_usage_percent,
        );
        gauge(
            &mut out,
            "flamepanel_memory_total_bytes",
            "Total memory in bytes.",
            s.memory_total_mb * 1024 * 1024,
        );
        gauge(
            &mut out,
            "flamepanel_memory_used_bytes",
            "Used memory in bytes.",
            s.memory_used_mb * 1024 * 1024,
        );
        gauge(
            &mut out,
            "flamepanel_disk_usage_percent",
            "Disk usage percent.",
            s.disk_usage_percent,
        );
        gauge(
            &mut out,
            "flamepanel_disk_total_bytes",
            "Total disk in bytes.",
            (s.disk_total_gb * GIB) as u64,
        );
        gauge(
            &mut out,
            "flamepanel_disk_used_bytes",
            "Used disk in bytes.",
            (s.disk_used_gb * GIB) as u64,
        );
        gauge(
            &mut out,
            "flamepanel_load1",
            "System load average (1 min).",
            s.load_one,
        );
        gauge(
            &mut out,
            "flamepanel_load5",
            "System load average (5 min).",
            s.load_five,
        );
        gauge(
            &mut out,
            "flamepanel_load15",
            "System load average (15 min).",
            s.load_fifteen,
        );
        gauge(
            &mut out,
            "flamepanel_network_rx_bytes_per_sec",
            "Network receive bytes/sec.",
            (s.network_rx_mbps * MIB) as u64,
        );
        gauge(
            &mut out,
            "flamepanel_network_tx_bytes_per_sec",
            "Network transmit bytes/sec.",
            (s.network_tx_mbps * MIB) as u64,
        );
    }

    Ok((
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    ))
}

/// 写一条 Prometheus gauge（HELP/TYPE/样本三行）
fn gauge<V: std::fmt::Display>(out: &mut String, name: &str, help: &str, value: V) {
    let _ = writeln!(
        out,
        "# HELP {name} {help}\n# TYPE {name} gauge\n{name} {value}"
    );
}

/// 路由表
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/metrics/processes", axum::routing::get(processes))
        .route("/api/metrics/history", axum::routing::get(history))
        .route("/metrics", axum::routing::get(prometheus_metrics))
}
