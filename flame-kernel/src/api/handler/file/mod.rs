use crate::api::extract::ApiJson;
use crate::api::types::{AppState, Username};
use crate::core::error::AppError;
use crate::file::{FileInfo, FileService};
use axum::Router;
use axum::{
    body::Bytes,
    extract::{Extension, Query, State},
    Json,
};
use serde::Deserialize;

/// 从 AppState 构造沙箱文件服务（白名单根目录取自 OP_FILE_ROOT）
fn file_service(state: &AppState) -> FileService {
    FileService::new(state.file_root.clone())
}

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

async fn log_file_op(state: &AppState, username: &str, action: &str, target: &str) {
    // T10/A2：审计用户名从 JWT 认证上下文取，不再硬编码 "system"。
    state
        .operation_log_service
        .log(username, action, Some(target), None)
        .await
        .ok();
}

pub async fn list(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<FileInfo>>, AppError> {
    let path = params.path.unwrap_or_else(|| "/".into());
    let entries = file_service(&state).list(&path).await?;
    log_file_op(&state, &username.0, "file_list", &path).await;
    Ok(Json(entries))
}

pub async fn read(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<ReadParams>,
) -> Result<Json<String>, AppError> {
    let content = file_service(&state).read(&params.path).await?;
    log_file_op(&state, &username.0, "file_read", &params.path).await;
    Ok(Json(content))
}

pub async fn write(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<WriteRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state).write(&req.path, &req.content).await?;
    log_file_op(&state, &username.0, "file_write", &req.path).await;
    Ok(Json("written"))
}

pub async fn create_file(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state).create_file(&req.path).await?;
    log_file_op(&state, &username.0, "file_create_file", &req.path).await;
    Ok(Json("created"))
}

pub async fn create_dir(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state).create_dir(&req.path).await?;
    log_file_op(&state, &username.0, "file_create_dir", &req.path).await;
    Ok(Json("created"))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<DeleteParams>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state)
        .delete(&params.path, params.recursive.unwrap_or(false))
        .await?;
    log_file_op(&state, &username.0, "file_delete", &params.path).await;
    Ok(Json("deleted"))
}

pub async fn rename(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state)
        .rename(&req.old_path, &req.new_path)
        .await?;
    log_file_op(
        &state,
        &username.0,
        "file_rename",
        &format!("{} -> {}", req.old_path, req.new_path),
    )
    .await;
    Ok(Json("renamed"))
}

pub async fn chmod(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<ChmodRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state).chmod(&req.path, &req.mode).await?;
    log_file_op(
        &state,
        &username.0,
        "file_chmod",
        &format!("{} {}", req.path, req.mode),
    )
    .await;
    Ok(Json("ok"))
}

pub async fn upload(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<UploadQuery>,
    body: Bytes,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state)
        .upload(&params.path, &params.name, &body)
        .await?;
    log_file_op(
        &state,
        &username.0,
        "file_upload",
        &format!("{}/{}", params.path, params.name),
    )
    .await;
    Ok(Json("uploaded"))
}

pub async fn download(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<ReadParams>,
) -> Result<(axum::http::StatusCode, [(String, String); 3], Vec<u8>), AppError> {
    let (name, content, mime) = file_service(&state).download(&params.path).await?;
    log_file_op(&state, &username.0, "file_download", &params.path).await;
    Ok((
        axum::http::StatusCode::OK,
        [
            ("Content-Type".into(), mime),
            (
                "Content-Disposition".into(),
                format!("attachment; filename=\"{}\"", name),
            ),
            ("Content-Length".into(), content.len().to_string()),
        ],
        content,
    ))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/files", axum::routing::get(list))
        .route("/api/files/read", axum::routing::get(read))
        .route("/api/files/write", axum::routing::post(write))
        .route("/api/files/create-file", axum::routing::post(create_file))
        .route("/api/files/create-dir", axum::routing::post(create_dir))
        .route("/api/files/delete", axum::routing::delete(delete))
        .route("/api/files/rename", axum::routing::post(rename))
        .route("/api/files/chmod", axum::routing::post(chmod))
        .route("/api/files/upload", axum::routing::post(upload))
        .route("/api/files/download", axum::routing::get(download))
}
