use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::Json;
use axum::Router;
use axum::{extract::State, http::header, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

/// 进程 TOP 条目（sysinfo 采集）
#[derive(Debug, Serialize, ToSchema)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory_mb: u64,
    pub status: String,
}

/// `GET /api/metrics/processes` — 按 CPU 占用排序的进程 TOP 5（免认证）
#[utoipa::path(
    get,
    path = "/api/metrics/processes",
    tag = "metrics",
    responses(
        (status = 200, description = "进程 TOP 列表", body = Vec<ProcessEntry>),
    )
)]
pub async fn processes() -> Result<Json<Vec<ProcessEntry>>, AppError> {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut entries: Vec<ProcessEntry> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| ProcessEntry {
            pid: pid.as_u32(),
            name: proc.name().to_string_lossy().into_owned(),
            cpu: proc.cpu_usage(),
            memory_mb: proc.memory() / 1024 / 1024,
            status: format!("{:?}", proc.status()),
        })
        .collect();
    entries.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    entries.truncate(5);
    Ok(Json(entries))
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
    let history = state.metrics_history.lock().await;
    let snapshots = history.get_all();
    drop(history);

    let latest = snapshots.last();
    let mut out = String::new();
    out.push_str("# HELP flamepanel_up Whether the panel is up (1).\n");
    out.push_str("# TYPE flamepanel_up gauge\n");
    out.push_str("flamepanel_up 1\n");
    out.push_str("# HELP flamepanel_uptime_seconds Panel uptime in seconds.\n");
    out.push_str("# TYPE flamepanel_uptime_seconds gauge\n");
    out.push_str(&format!("flamepanel_uptime_seconds {}\n", uptime_secs()));
    out.push_str("# HELP flamepanel_info Static build info.\n");
    out.push_str("# TYPE flamepanel_info gauge\n");
    out.push_str(&format!(
        "flamepanel_info{{version=\"{}\"}} 1\n",
        crate::VERSION
    ));

    if let Some(s) = latest {
        out.push_str("# HELP flamepanel_cpu_usage_percent CPU usage percent (global).\n");
        out.push_str("# TYPE flamepanel_cpu_usage_percent gauge\n");
        out.push_str(&format!("flamepanel_cpu_usage_percent {}\n", s.cpu_usage));
        out.push_str("# HELP flamepanel_cpu_cores Number of CPU cores.\n");
        out.push_str("# TYPE flamepanel_cpu_cores gauge\n");
        out.push_str(&format!("flamepanel_cpu_cores {}\n", s.cpu_cores));
        out.push_str("# HELP flamepanel_memory_usage_percent Memory usage percent.\n");
        out.push_str("# TYPE flamepanel_memory_usage_percent gauge\n");
        out.push_str(&format!(
            "flamepanel_memory_usage_percent {}\n",
            s.memory_usage_percent
        ));
        out.push_str("# HELP flamepanel_memory_total_bytes Total memory in bytes.\n");
        out.push_str("# TYPE flamepanel_memory_total_bytes gauge\n");
        out.push_str(&format!(
            "flamepanel_memory_total_bytes {}\n",
            s.memory_total_mb * 1024 * 1024
        ));
        out.push_str("# HELP flamepanel_memory_used_bytes Used memory in bytes.\n");
        out.push_str("# TYPE flamepanel_memory_used_bytes gauge\n");
        out.push_str(&format!(
            "flamepanel_memory_used_bytes {}\n",
            s.memory_used_mb * 1024 * 1024
        ));
        out.push_str("# HELP flamepanel_disk_usage_percent Disk usage percent.\n");
        out.push_str("# TYPE flamepanel_disk_usage_percent gauge\n");
        out.push_str(&format!(
            "flamepanel_disk_usage_percent {}\n",
            s.disk_usage_percent
        ));
        out.push_str("# HELP flamepanel_disk_total_bytes Total disk in bytes.\n");
        out.push_str("# TYPE flamepanel_disk_total_bytes gauge\n");
        out.push_str(&format!(
            "flamepanel_disk_total_bytes {}\n",
            (s.disk_total_gb * 1024.0 * 1024.0 * 1024.0) as u64
        ));
        out.push_str("# HELP flamepanel_disk_used_bytes Used disk in bytes.\n");
        out.push_str("# TYPE flamepanel_disk_used_bytes gauge\n");
        out.push_str(&format!(
            "flamepanel_disk_used_bytes {}\n",
            (s.disk_used_gb * 1024.0 * 1024.0 * 1024.0) as u64
        ));
        out.push_str("# HELP flamepanel_load1 System load average (1 min).\n");
        out.push_str("# TYPE flamepanel_load1 gauge\n");
        out.push_str(&format!("flamepanel_load1 {}\n", s.load_one));
        out.push_str("# HELP flamepanel_load5 System load average (5 min).\n");
        out.push_str("# TYPE flamepanel_load5 gauge\n");
        out.push_str(&format!("flamepanel_load5 {}\n", s.load_five));
        out.push_str("# HELP flamepanel_load15 System load average (15 min).\n");
        out.push_str("# TYPE flamepanel_load15 gauge\n");
        out.push_str(&format!("flamepanel_load15 {}\n", s.load_fifteen));
        out.push_str("# HELP flamepanel_network_rx_bytes_per_sec Network receive bytes/sec.\n");
        out.push_str("# TYPE flamepanel_network_rx_bytes_per_sec gauge\n");
        out.push_str(&format!(
            "flamepanel_network_rx_bytes_per_sec {}\n",
            (s.network_rx_mbps * 1024.0 * 1024.0) as u64
        ));
        out.push_str("# HELP flamepanel_network_tx_bytes_per_sec Network transmit bytes/sec.\n");
        out.push_str("# TYPE flamepanel_network_tx_bytes_per_sec gauge\n");
        out.push_str(&format!(
            "flamepanel_network_tx_bytes_per_sec {}\n",
            (s.network_tx_mbps * 1024.0 * 1024.0) as u64
        ));
    }

    Ok((
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    ))
}

/// 进程运行时长（秒）——用二进制 mtime 近似启动时刻
fn uptime_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let exe = std::env::current_exe().ok();
    let mtime = exe
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
        .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(now);
    now.saturating_sub(mtime)
}

/// 路由表
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/metrics/processes", axum::routing::get(processes))
        .route("/metrics", axum::routing::get(prometheus_metrics))
}
