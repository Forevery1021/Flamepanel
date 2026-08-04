use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::Router;
use axum::Json;
use serde::Serialize;

/// 进程 TOP 条目（sysinfo 采集）
#[derive(Debug, Serialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory_mb: u64,
    pub status: String,
}

/// `GET /api/metrics/processes` — 按 CPU 占用排序的进程 TOP 5（免认证）
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
    entries.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
    entries.truncate(5);
    Ok(Json(entries))
}

/// 路由表
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/metrics/processes", axum::routing::get(processes))
}
