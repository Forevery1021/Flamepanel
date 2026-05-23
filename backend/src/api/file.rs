use axum::{
    body::Bytes,
    extract::{Multipart, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::middleware::auth::CurrentUser;

const ALLOWED_BASE_PATHS: &[&str] = &["/www", "/data", "/home", "/root"];

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
    pub permissions: String,
}

#[derive(Debug, Deserialize)]
pub struct PathQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub path: String,
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub filename: String,
    pub path: String,
}

// ─── Routes ───────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/list", get(list_files))
        .route("/read", get(read_file))
        .route("/write", post(write_file))
        .route("/upload", post(upload_file))
        .route("/download", get(download_file))
        .route("/mkdir", post(create_dir))
        .route("/rename", post(rename_file))
        .route("/delete", delete(delete_file))
}

// ─── 路径校验 ─────────────────────────────────────────────────────────────────

fn validate_path(p: &str) -> Result<PathBuf, AppError> {
    let path = std::path::Path::new(p);
    if !path.is_absolute() {
        return Err(AppError::BadRequest("必须使用绝对路径".into()));
    }

    // 规范化路径，防止目录穿越
    let canonical = path
        .canonicalize()
        .map_err(|_| AppError::BadRequest("路径不存在或无法访问".into()))?;

    let allowed = ALLOWED_BASE_PATHS.iter().any(|base| {
        canonical.starts_with(base) || canonical.to_string_lossy().starts_with(*base)
    });

    if !allowed {
        return Err(AppError::Forbidden);
    }

    Ok(canonical)
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

async fn list_files(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<PathQuery>,
) -> Result<Json<Vec<FileInfo>>, AppError> {
    let path = validate_path(&query.path)?;

    let mut entries = vec![];
    let dir = std::fs::read_dir(&path)
        .map_err(|e| AppError::Internal(format!("读取目录失败: {e}")))?;

    for entry in dir {
        let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
        let meta = entry.metadata().map_err(|e| AppError::Internal(e.to_string()))?;

        #[cfg(unix)]
        let perms = {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", meta.permissions().mode() & 0o777)
        };
        #[cfg(not(unix))]
        let perms = String::from("rwx");

        entries.push(FileInfo {
            name: entry.file_name().to_string_lossy().into(),
            path: entry.path().to_string_lossy().into(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified: meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| {
                    let secs = d.as_secs();
                    chrono::DateTime::from_timestamp(secs as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "N/A".into())
                })
                .unwrap_or_else(|| "N/A".into()),
            permissions: perms,
        });
    }

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(Json(entries))
}

async fn read_file(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<PathQuery>,
) -> Result<String, AppError> {
    let path = validate_path(&query.path)?;
    if path.is_dir() {
        return Err(AppError::BadRequest("不能读取目录".into()));
    }
    // 限制文本文件读取大小为 10MB
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| AppError::Internal(format!("读取文件失败: {e}")))?;
    Ok(content)
}

async fn write_file(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let path = payload["path"].as_str().ok_or(AppError::BadRequest("path 参数必填".into()))?;
    let content = payload["content"].as_str().ok_or(AppError::BadRequest("content 参数必填".into()))?;

    let path = validate_path(path)?;
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| AppError::Internal(format!("写入文件失败: {e}")))?;

    Ok(Json(serde_json::json!({"success": true, "message": "文件保存成功"})))
}

/// POST /api/file/upload
/// multipart/form-data: file + dir 字段
async fn upload_file(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let mut target_dir = PathBuf::from("/tmp");
    let mut uploaded_file: Option<(String, Bytes)> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::BadRequest(format!("解析上传失败: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "dir" => {
                let dir = field.text().await.unwrap_or_else(|_| "/tmp".into());
                target_dir = validate_path(&dir)?;
            }
            "file" => {
                let filename = field.file_name().unwrap_or("unnamed").to_string();
                let data = field.bytes().await
                    .map_err(|e| AppError::BadRequest(format!("读取文件数据失败: {e}")))?;
                uploaded_file = Some((filename, data));
            }
            _ => {}
        }
    }

    let (filename, data) = uploaded_file.ok_or(AppError::BadRequest("未选择文件".into()))?;
    let dest = target_dir.join(&filename);

    tokio::fs::write(&dest, &data)
        .await
        .map_err(|e| AppError::Internal(format!("保存文件失败: {e}")))?;

    Ok(Json(UploadResponse {
        success: true,
        filename,
        path: dest.to_string_lossy().into(),
    }))
}

/// GET /api/file/download?path=xxx
/// 文件下载（流式响应）
async fn download_file(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<PathQuery>,
) -> Result<impl IntoResponse, AppError> {
    let path = validate_path(&query.path)?;
    if path.is_dir() {
        return Err(AppError::BadRequest("不能下载目录".into()));
    }

    let filename = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".into());

    let content = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::Internal(format!("读取文件失败: {e}")))?;

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
    ];

    Ok((StatusCode::OK, headers, content))
}

async fn create_dir(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<PathQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let path = validate_path(&payload.path)?;
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(|e| AppError::Internal(format!("创建目录失败: {e}")))?;

    Ok(Json(serde_json::json!({"success": true, "message": "目录创建成功"})))
}

async fn rename_file(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(req): Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let src = validate_path(&req.path)?;
    let dest = src.parent()
        .ok_or(AppError::BadRequest("无效路径".into()))?
        .join(&req.new_name);

    tokio::fs::rename(&src, &dest)
        .await
        .map_err(|e| AppError::Internal(format!("重命名失败: {e}")))?;

    Ok(Json(serde_json::json!({"success": true, "message": "重命名成功", "new_path": dest.to_string_lossy()})))
}

async fn delete_file(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let path = validate_path(&query.path)?;

    if path.is_dir() {
        tokio::fs::remove_dir_all(&path)
            .await
            .map_err(|e| AppError::Internal(format!("删除目录失败: {e}")))?;
    } else {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| AppError::Internal(format!("删除文件失败: {e}")))?;
    }

    Ok(Json(serde_json::json!({"success": true, "message": "删除成功"})))
}
