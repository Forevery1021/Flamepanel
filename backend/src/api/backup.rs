use axum::{extract::{Path, State}, Json, Router, routing::{get, post}};
use serde::Deserialize;

use crate::application::{AppState, BackupService};
use crate::core::error::AppError;
use crate::domain::{
    BackupConfig, BackupRecord, CreateBackupConfigRequest, UpdateBackupConfigRequest,
};

#[derive(Debug, Deserialize)]
struct RestoreRequest {
    target_path: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/configs", get(list_configs).post(create_config))
        .route("/configs/{id}", get(get_config).put(update_config).delete(delete_config))
        .route("/execute/{id}", post(execute_backup))
        .route("/records/{config_id}", get(list_records))
        .route("/restore/{record_id}", post(restore_backup))
}

fn backup_service(state: &AppState) -> BackupService {
    BackupService::new(state.backup_repo.clone())
}

async fn list_configs(State(state): State<AppState>) -> Result<Json<Vec<BackupConfig>>, AppError> {
    let svc = backup_service(&state);
    Ok(Json(svc.list_configs().await?))
}

async fn get_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<BackupConfig>, AppError> {
    let svc = backup_service(&state);
    Ok(Json(svc.get_config(id).await?))
}

async fn create_config(
    State(state): State<AppState>,
    Json(req): Json<CreateBackupConfigRequest>,
) -> Result<Json<BackupConfig>, AppError> {
    let svc = backup_service(&state);
    Ok(Json(svc.create_config(req).await?))
}

async fn update_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBackupConfigRequest>,
) -> Result<Json<()>, AppError> {
    let svc = backup_service(&state);
    svc.update_config(id, req).await?;
    Ok(Json(()))
}

async fn delete_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    let svc = backup_service(&state);
    svc.delete_config(id).await?;
    Ok(Json(()))
}

async fn execute_backup(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<BackupRecord>, AppError> {
    let svc = backup_service(&state);
    Ok(Json(svc.execute_backup(id).await?))
}

async fn list_records(
    State(state): State<AppState>,
    Path(config_id): Path<i64>,
) -> Result<Json<Vec<BackupRecord>>, AppError> {
    let svc = backup_service(&state);
    Ok(Json(svc.list_records(config_id).await?))
}

async fn restore_backup(
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<()>, AppError> {
    let svc = backup_service(&state);
    svc.restore_backup(record_id, req.target_path).await?;
    Ok(Json(()))
}
