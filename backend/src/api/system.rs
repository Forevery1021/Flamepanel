use axum::{Json, Router, routing::get};
use utoipa::ToSchema;
use serde::Serialize;
use sysinfo::System;
use crate::middleware::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct SystemInfo {
    #[schema(example = 45.6)]
    cpu_usage: f32,
    #[schema(example = 16384)]
    memory_total_mb: u64,
    #[schema(example = 8192)]
    memory_used_mb: u64,
    uptime_seconds: u64,
    #[schema(example = "2026-05-21 18:00:00")]
    boot_time: String,
}

#[utoipa::path(
    get,
    path = "/api/system/info",
    responses(
        (status = 200, description = "系统信息", body = SystemInfo),
        (status = 401, description = "未授权")
    )
)]
async fn get_system_info(_: CurrentUser) -> Json<SystemInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    Json(SystemInfo {
        cpu_usage: sys.global_cpu_usage(),
        memory_total_mb: sys.total_memory() / 1024 / 1024,
        memory_used_mb: sys.used_memory() / 1024 / 1024,
        uptime_seconds: System::uptime(),
        boot_time: "N/A".to_string(),
    })
}

pub fn routes() -> Router {
    Router::new().route("/info", get(get_system_info))
}