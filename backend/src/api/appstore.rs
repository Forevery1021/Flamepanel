use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use tokio::process::Command;

use crate::app_catalog::AppManifest;
use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::{InstallAppRequest, InstalledApp};
use crate::middleware::auth::CurrentUser;

// ─── GET /appstore/catalog ─────────────────────────────────────────────────────

pub async fn catalog(
    _user: CurrentUser,
) -> Result<Json<Vec<AppManifest>>, AppError> {
    Ok(Json(crate::app_catalog::builtin_apps()))
}

// ─── GET /appstore/installed ──────────────────────────────────────────────────

pub async fn installed(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<InstalledApp>>, AppError> {
    state.app_repo.list_all().await.map(Json)
}

// ─── POST /appstore/install ──────────────────────────────────────────────────

pub async fn install(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<InstallAppRequest>,
) -> Result<Json<InstalledApp>, AppError> {
    // Find manifest
    let apps = crate::app_catalog::builtin_apps();
    let manifest = apps
        .iter()
        .find(|a| a.key == req.app_key)
        .ok_or(AppError::BadRequest("应用不存在".into()))?
        .clone();

    // Check name not taken
    if state.app_repo.find_by_name(&req.name).await?.is_some() {
        return Err(AppError::BadRequest("应用名称已被使用".into()));
    }

    let port = req.port.unwrap_or(manifest.default_port);
    let version = manifest.version.clone();
    let data_dir = format!("data/app_{}", &req.name);
    let _ = std::fs::create_dir_all(&data_dir);
    let data_dir_abs = format!(
        "{}/{}",
        std::env::current_dir().unwrap_or_default().display(),
        &data_dir
    );

    // Render compose file from template
    let mut compose = manifest.compose.clone();
    compose = compose.replace("{name}", &req.name);
    compose = compose.replace("{port}", &port.to_string());
    compose = compose.replace("{data_dir}", &data_dir_abs);
    // SSH port for Gitea
    let ssh_port = port + 100;
    compose = compose.replace("{ssh_port}", &ssh_port.to_string());
    // phpMyAdmin optional db host/port
    if let Some(ref env) = req.extra_env {
        compose = compose.replace("{db_host}", env.get("db_host").map_or("127.0.0.1", |v| v));
        compose = compose.replace("{db_port}", env.get("db_port").map_or("3306", |v| v));
    } else {
        compose = compose.replace("{db_host}", "127.0.0.1");
        compose = compose.replace("{db_port}", "3306");
    }

    // Save compose file
    let compose_path = format!("{data_dir}/docker-compose.yml");
    std::fs::write(&compose_path, &compose)
        .map_err(|e| AppError::Internal(format!("写入 compose 文件失败: {e}")))?;

    // Create DB record
    let app = state
        .app_repo
        .create(
            &req.app_key, &req.name, &manifest.category, port,
            &version, Some(&manifest.description),
            Some(&compose_path), Some(&data_dir),
        )
        .await?;

    // Run docker compose up
    tracing::info!("Installing app '{}' via docker compose", req.name);
    let out = Command::new("docker")
        .args(["compose", "-f", &compose_path, "-p", &format!("fp-{}", &req.name), "up", "-d"])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker compose up 失败: {e}")))?;

    if out.status.success() {
        state.app_repo.update_status(app.id, "running").await?;
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        state.app_repo.update_status(app.id, "error").await?;
        tracing::error!("App install failed: {err}");
    }

    let updated = state.app_repo.find_by_id(app.id).await?.unwrap();
    Ok(Json(updated))
}

// ─── POST /appstore/:id/start ──────────────────────────────────────────────────

pub async fn start(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let app = state.app_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("应用不存在".into()))?;
    if let Some(ref f) = app.compose_file {
        let out = Command::new("docker")
            .args(["compose", "-f", f, "-p", &format!("fp-{}", &app.name), "start"])
            .output().await
            .map_err(|e| AppError::Internal(format!("启动失败: {e}")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::Internal(format!("启动失败: {err}")));
        }
    }
    state.app_repo.update_status(id, "running").await?;
    Ok(Json(json!({"message": "已启动"})))
}

// ─── POST /appstore/:id/stop ──────────────────────────────────────────────────

pub async fn stop(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let app = state.app_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("应用不存在".into()))?;
    if let Some(ref f) = app.compose_file {
        let out = Command::new("docker")
            .args(["compose", "-f", f, "-p", &format!("fp-{}", &app.name), "stop"])
            .output().await
            .map_err(|e| AppError::Internal(format!("停止失败: {e}")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::Internal(format!("停止失败: {err}")));
        }
    }
    state.app_repo.update_status(id, "stopped").await?;
    Ok(Json(json!({"message": "已停止"})))
}

// ─── POST /appstore/:id/restart ───────────────────────────────────────────────

pub async fn restart(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let app = state.app_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("应用不存在".into()))?;
    if let Some(ref f) = app.compose_file {
        let out = Command::new("docker")
            .args(["compose", "-f", f, "-p", &format!("fp-{}", &app.name), "restart"])
            .output().await
            .map_err(|e| AppError::Internal(format!("重启失败: {e}")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::Internal(format!("重启失败: {err}")));
        }
    }
    state.app_repo.update_status(id, "running").await?;
    Ok(Json(json!({"message": "已重启"})))
}

// ─── DELETE /appstore/:id ────────────────────────────────────────────────────

pub async fn uninstall(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let app = state.app_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("应用不存在".into()))?;

    if let Some(ref f) = app.compose_file {
        let _ = Command::new("docker")
            .args(["compose", "-f", f, "-p", &format!("fp-{}", &app.name), "down", "-v"])
            .output().await;
    }
    if let Some(ref dir) = app.data_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
    state.app_repo.delete(id).await?;
    Ok(Json(json!({"message": "已卸载"})))
}

// ─── GET /appstore/:id/logs ──────────────────────────────────────────────────

pub async fn logs(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let app = state.app_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("应用不存在".into()))?;
    if let Some(ref f) = app.compose_file {
        let out = Command::new("docker")
            .args(["compose", "-f", f, "-p", &format!("fp-{}", &app.name), "logs", "--tail=100"])
            .output().await
            .map_err(|e| AppError::Internal(format!("获取日志失败: {e}")))?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        Ok(Json(json!({"logs": text})))
    } else {
        Ok(Json(json!({"logs": ""})))
    }
}

// ─── Routes ───────────────────────────────────────────────────────────────────

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/catalog", axum::routing::get(catalog))
        .route("/installed", axum::routing::get(installed))
        .route("/install", axum::routing::post(install))
        .route("/{id}/start", axum::routing::post(start))
        .route("/{id}/stop", axum::routing::post(stop))
        .route("/{id}/restart", axum::routing::post(restart))
        .route("/{id}/uninstall", axum::routing::delete(uninstall))
        .route("/{id}/logs", axum::routing::get(logs))
}
