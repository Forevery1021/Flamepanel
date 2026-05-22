use axum::{
    extract::{Query},
    Json, Router, routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::path::{Path as StdPath, PathBuf};
use crate::core::error::AppError;
use crate::middleware::middleware_auth::CurrentUser;

const ALLOWED_BASE_PATHS: &[&str] = &["/www", "/data", "/home", "/root/ops-panel/data"];

#[derive(Serialize)]
pub struct FileInfo {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
}

#[derive(Deserialize)]
pub struct FileListQuery {
    path: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/list", get(list_files))
        .route("/read", get(read_file))
        .route("/write", post(write_file))
        .route("/mkdir", post(create_dir))
}

fn validate_path(p: &str) -> Result<PathBuf, AppError> {
    let path = StdPath::new(p);
    if !path.is_absolute() {
        return Err(AppError::BadRequest("必须使用绝对路径".into()));
    }

    let canonical = path.canonicalize()
        .map_err(|_| AppError::BadRequest("路径访问被拒绝".into()))?;

    for base in ALLOWED_BASE_PATHS {
        if canonical.starts_with(base) {
            return Ok(canonical);
        }
    }
    Err(AppError::BadRequest("路径不在允许范围内".into()))
}

async fn list_files(
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<FileListQuery>,
) -> Result<Json<Vec<FileInfo>>, AppError> {
    let path = validate_path(&query.path)?;

    let mut entries = vec![];
    for entry in std::fs::read_dir(&path).map_err(|e| AppError::Internal(e.to_string()))? {
        let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
        let meta = entry.metadata().map_err(|e| AppError::Internal(e.to_string()))?;

        entries.push(FileInfo {
            name: entry.file_name().to_string_lossy().into(),
            path: entry.path().to_string_lossy().into(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified: format!("{:?}", meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)),
        });
    }

    Ok(Json(entries))
}

async fn read_file(
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<FileListQuery>,
) -> Result<String, AppError> {
    let path = validate_path(&query.path)?;
    if path.is_dir() {
        return Err(AppError::BadRequest("不能读取目录".into()));
    }
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn write_file(
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<(String, String)>, // (path, content)
) -> Result<(), AppError> {
    let path = validate_path(&payload.0)?;
    tokio::fs::write(path, payload.1)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}

async fn create_dir(
    CurrentUser(_claims): CurrentUser,
    Json(path): Json<String>,
) -> Result<(), AppError> {
    let path = validate_path(&path)?;
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}