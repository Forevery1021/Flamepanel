use axum::{Json, extract::{Query, State}, body::Bytes};
use serde::Deserialize;
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::file::{FileService, FileInfo};

#[derive(Deserialize)]
pub struct ListParams {
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct ReadParams {
    pub path: String,
}

#[derive(Deserialize)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct CreateRequest {
    pub path: String,
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub path: String,
    pub recursive: Option<bool>,
}

#[derive(Deserialize)]
pub struct RenameRequest {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Deserialize)]
pub struct ChmodRequest {
    pub path: String,
    pub mode: String,
}

#[derive(Deserialize)]
pub struct UploadQuery {
    pub path: String,
    pub name: String,
}

pub async fn list(
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<FileInfo>>, AppError> {
    let path = params.path.unwrap_or_else(|| "/".into());
    let entries = FileService::list(&path).await?;
    Ok(Json(entries))
}

pub async fn read(
    Query(params): Query<ReadParams>,
) -> Result<Json<String>, AppError> {
    let content = FileService::read(&params.path).await?;
    Ok(Json(content))
}

pub async fn write(
    Json(req): Json<WriteRequest>,
) -> Result<Json<&'static str>, AppError> {
    FileService::write(&req.path, &req.content).await?;
    Ok(Json("written"))
}

pub async fn create_file(
    Json(req): Json<CreateRequest>,
) -> Result<Json<&'static str>, AppError> {
    FileService::create_file(&req.path).await?;
    Ok(Json("created"))
}

pub async fn create_dir(
    Json(req): Json<CreateRequest>,
) -> Result<Json<&'static str>, AppError> {
    FileService::create_dir(&req.path).await?;
    Ok(Json("created"))
}

pub async fn delete(
    Query(params): Query<DeleteParams>,
) -> Result<Json<&'static str>, AppError> {
    FileService::delete(&params.path, params.recursive.unwrap_or(false)).await?;
    Ok(Json("deleted"))
}

pub async fn rename(
    Json(req): Json<RenameRequest>,
) -> Result<Json<&'static str>, AppError> {
    FileService::rename(&req.old_path, &req.new_path).await?;
    Ok(Json("renamed"))
}

pub async fn chmod(
    Json(req): Json<ChmodRequest>,
) -> Result<Json<&'static str>, AppError> {
    FileService::chmod(&req.path, &req.mode).await?;
    Ok(Json("ok"))
}

pub async fn upload(
    State(state): State<AppState>,
    Query(params): Query<UploadQuery>,
    body: Bytes,
) -> Result<Json<&'static str>, AppError> {
    FileService::upload(&params.path, &params.name, &body).await?;
    // Log the operation
    state.operation_log_service.log("admin", "file_upload", Some(&format!("{}/{}", params.path, params.name)), None).await.ok();
    Ok(Json("uploaded"))
}

pub async fn download(
    Query(params): Query<ReadParams>,
) -> Result<(axum::http::StatusCode, [(String, String); 3], Vec<u8>), AppError> {
    let (name, content, mime) = FileService::download(&params.path).await?;
    Ok((
        axum::http::StatusCode::OK,
        [
            ("Content-Type".into(), mime),
            ("Content-Disposition".into(), format!("attachment; filename=\"{}\"", name)),
            ("Content-Length".into(), content.len().to_string()),
        ],
        content,
    ))
}
