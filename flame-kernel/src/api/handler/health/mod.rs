use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::Router;
use axum::{extract::State, Json};
use serde::Serialize;
use std::time::UNIX_EPOCH;
use utoipa::ToSchema;

/// 依赖检查结果
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthCheckItem {
    pub status: String,
    pub detail: Option<String>,
}

/// 详细健康检查响应
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthDetail {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthChecks {
    pub database: HealthCheckItem,
    pub docker: HealthCheckItem,
    pub disk: HealthCheckItem,
}

/// 进程启动时间（懒初始化）
fn start_time() -> std::time::SystemTime {
    static START: std::sync::OnceLock<std::time::SystemTime> = std::sync::OnceLock::new();
    *START.get_or_init(std::time::SystemTime::now)
}

/// `GET /api/health` — 详细健康检查（免认证）
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "健康状态", body = HealthDetail),
    )
)]
pub async fn detail(State(state): State<AppState>) -> Result<Json<HealthDetail>, AppError> {
    // database：SQLite/InMemory 查询最小表（users）探活
    let database = match state.user_service.list_users().await {
        Ok(_) => HealthCheckItem {
            status: "ok".into(),
            detail: None,
        },
        Err(e) => HealthCheckItem {
            status: "error".into(),
            detail: Some(e.to_string()),
        },
    };

    // docker：Docker daemon 可用性（容器列表）
    let docker = match state.docker_service.list_containers(0).await {
        Ok(containers) => HealthCheckItem {
            status: "ok".into(),
            detail: Some(format!("{} containers", containers.len())),
        },
        Err(e) => HealthCheckItem {
            status: "degraded".into(),
            detail: Some(format!("{}", e)),
        },
    };

    // disk：数据目录可用空间（目录不存在时 unknown，不判失败）
    let disk = if std::path::Path::new("data").exists() {
        match disk_free_bytes("data") {
            Some(bytes) if bytes > 0 => HealthCheckItem {
                status: "ok".into(),
                detail: Some(format!("{} MB free", bytes / 1024 / 1024)),
            },
            Some(bytes) => HealthCheckItem {
                status: "error".into(),
                detail: Some(format!("{} bytes free", bytes)),
            },
            None => HealthCheckItem {
                status: "unknown".into(),
                detail: Some("unable to stat data dir".into()),
            },
        }
    } else {
        HealthCheckItem {
            status: "unknown".into(),
            detail: Some("data dir not present".into()),
        }
    };

    let all_ok = database.status == "ok" && disk.status != "error";
    let uptime = start_time()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(Json(HealthDetail {
        status: if all_ok { "ok" } else { "degraded" }.into(),
        version: crate::VERSION.into(),
        uptime_secs: uptime,
        checks: HealthChecks {
            database,
            docker,
            disk,
        },
    }))
}

fn disk_free_bytes(dir: &str) -> Option<u64> {
    // 用 sysinfo 获取指定目录所在挂载点的可用空间，替代 `df -k <dir>` 外部命令。
    // （Phase A1 收尾：health 不再 spawn 外部命令，无命令注入面）
    let abs = std::path::absolute(dir).ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best: Option<(&sysinfo::Disk, usize)> = None;
    for disk in disks.iter() {
        let mp = disk.mount_point();
        if abs.starts_with(mp) {
            let depth = mp.components().count();
            if best.map(|(_, d)| depth > d).unwrap_or(true) {
                best = Some((disk, depth));
            }
        }
    }
    best.map(|(disk, _)| disk.available_space())
}

/// 路由表
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/health", axum::routing::get(detail))
}
