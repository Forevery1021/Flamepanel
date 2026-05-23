use axum::{Json, Router, routing::get, extract::State};
use serde::Serialize;

use crate::application::{AppState, SystemService};
use crate::middleware::auth::CurrentUser;

#[derive(Serialize)]
pub struct SystemInfoResponse {
    cpu_usage: f32,
    cpu_cores: usize,
    memory_total_mb: u64,
    memory_used_mb: u64,
    memory_free_mb: u64,
    memory_usage_percent: f32,
    disk_total_gb: f64,
    disk_used_gb: f64,
    disk_free_gb: f64,
    disk_usage_percent: f32,
    uptime_seconds: u64,
    uptime_display: String,
    load_one: f64,
    load_five: f64,
    load_fifteen: f64,
    hostname: String,
    network_interfaces: Vec<NetworkInterfaceResponse>,
}

#[derive(Serialize)]
pub struct NetworkInterfaceResponse {
    name: String,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
    mac: String,
}

#[derive(Serialize)]
pub struct ProcessResponse {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
    status: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/info", get(system_info))
        .route("/processes", get(process_list))
}

async fn system_info(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Json<SystemInfoResponse> {
    let info = SystemService::get_info();

    let memory_usage = if info.memory_total_mb > 0 {
        (info.memory_used_mb as f32 / info.memory_total_mb as f32) * 100.0
    } else {
        0.0
    };

    let disk_usage = if info.disk_total_gb > 0.0 {
        ((info.disk_used_gb / info.disk_total_gb) * 100.0) as f32
    } else {
        0.0
    };

    let uptime_display = {
        let secs = info.uptime_seconds;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        format!("{days}d {hours}h {mins}m")
    };

    Json(SystemInfoResponse {
        cpu_usage: info.cpu_usage,
        cpu_cores: info.cpu_cores,
        memory_total_mb: info.memory_total_mb,
        memory_used_mb: info.memory_used_mb,
        memory_free_mb: info.memory_free_mb,
        memory_usage_percent: memory_usage,
        disk_total_gb: info.disk_total_gb,
        disk_used_gb: info.disk_used_gb,
        disk_free_gb: info.disk_free_gb,
        disk_usage_percent: disk_usage,
        uptime_seconds: info.uptime_seconds,
        uptime_display,
        load_one: info.load_average.one,
        load_five: info.load_average.five,
        load_fifteen: info.load_average.fifteen,
        hostname: info.network.hostname,
        network_interfaces: info.network.interfaces.into_iter().map(|i| NetworkInterfaceResponse {
            name: i.name,
            ipv4: i.ipv4,
            ipv6: i.ipv6,
            mac: i.mac,
        }).collect(),
    })
}

async fn process_list(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Json<Vec<ProcessResponse>> {
    let processes = SystemService::get_processes();
    Json(processes.into_iter().map(|p| ProcessResponse {
        pid: p.pid,
        name: p.name,
        cpu_usage: p.cpu_usage,
        memory_mb: p.memory_mb,
        status: p.status,
    }).collect())
}
