use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::Router;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct BackupEntryDto {
    pub filename: String,
    pub size: i64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RestoreBackupRequest {
    pub filename: String,
}

pub async fn list_backups(
    State(state): State<AppState>,
) -> Result<Json<Vec<BackupEntryDto>>, AppError> {
    let backups = state.backup_service.list_backups().await?;
    Ok(Json(
        backups
            .into_iter()
            .map(|b| BackupEntryDto {
                filename: b.filename,
                size: b.size,
                created_at: b.created_at,
            })
            .collect(),
    ))
}

pub async fn create_backup(
    State(state): State<AppState>,
) -> Result<Json<BackupEntryDto>, AppError> {
    let backup = state.backup_service.create_backup().await?;
    let _ = state
        .event_bus
        .publish(crate::domain::entity::DomainEvent::BackupCreated {
            filename: backup.filename.clone(),
        })
        .await;
    Ok(Json(BackupEntryDto {
        filename: backup.filename,
        size: backup.size,
        created_at: backup.created_at,
    }))
}

pub async fn download_backup(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<axum::response::Response, AppError> {
    let path = state.backup_service.get_backup_path(&filename).await?;
    let bytes = std::fs::read(&path)?;
    let disposition = format!("attachment; filename=\"{filename}\"")
        .parse::<axum::http::HeaderValue>()
        .map_err(|_| AppError::internal("Invalid content disposition header"))?;
    let body = axum::body::boxed(axum::body::Full::from(bytes));
    let response = axum::response::Response::builder()
        .status(200)
        .header(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/octet-stream"),
        )
        .header(axum::http::header::CONTENT_DISPOSITION, disposition)
        .body(body)
        .map_err(|e| AppError::internal_with_source("Failed to build download response", e))?;
    Ok(response)
}

pub async fn delete_backup(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<Json<()>, AppError> {
    state.backup_service.delete_backup(&filename).await?;
    Ok(Json(()))
}

pub async fn restore_backup(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<RestoreBackupRequest>,
) -> Result<Json<()>, AppError> {
    state.backup_service.restore_backup(&req.filename).await?;
    Ok(Json(()))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/backups", get(list_backups).post(create_backup))
        .route(
            "/api/backups/:filename",
            get(download_backup).delete(delete_backup),
        )
        .route("/api/backups/:filename/restore", post(restore_backup))
}
