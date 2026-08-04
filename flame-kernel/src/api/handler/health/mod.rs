use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::Router;
use axum::{extract::State, Json};
use serde::Serialize;
use std::time::UNIX_EPOCH;

/// 依赖检查结果
#[derive(Debug, Serialize)]
pub struct HealthCheckItem {
    pub status: String,
    pub detail: Option<String>,
}

/// 详细健康检查响应
#[derive(Debug, Serialize)]
pub struct HealthDetail {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize)]
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
    // 用 `df -k <dir>` 获取可用空间（标准 POSIX 输出：Filesystem 1024-blocks Used Available ...）
    let out = std::process::Command::new("df")
        .args(["-k", dir])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // 标准 df：第 4 列为 Available（KB）
        if cols.len() >= 4 {
            if let Ok(kb) = cols[3].parse::<u64>() {
                return Some(kb * 1024);
            }
        }
    }
    None
}

/// 路由表
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/health", axum::routing::get(detail))
}
