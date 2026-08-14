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
use utoipa::ToSchema;

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct BackupEntryDto {
    pub filename: String,
    pub size: i64,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RestoreBackupRequest {
    pub filename: String,
}

/// 备份列表
#[utoipa::path(
    get,
    path = "/api/backups",
    tag = "backups",
    responses(
        (status = 200, description = "备份列表", body = Vec<BackupEntryDto>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 创建备份
#[utoipa::path(
    post,
    path = "/api/backups",
    tag = "backups",
    responses(
        (status = 200, description = "备份成功", body = BackupEntryDto),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
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
    // Axum 0.7+：`Full`/`boxed` 已移除，Body 直接由 bytes 构造
    let body = axum::body::Body::from(bytes);
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

/// 删除备份
#[utoipa::path(
    delete,
    path = "/api/backups/{filename}",
    tag = "backups",
    params(("filename" = String, Path, description = "备份文件名")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "备份不存在"),
    ),
    security(("BearerAuth" = []))
)]
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
            "/api/backups/{filename}",
            get(download_backup).delete(delete_backup),
        )
        .route("/api/backups/{filename}/restore", post(restore_backup))
}
