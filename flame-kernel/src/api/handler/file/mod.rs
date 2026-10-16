//! 文件管理端点（Stage0.2 沙箱 · M-C 文件管理收尾）。
//!
//! 所有路径按 chroot 语义解释（`/` 即 `OP_FILE_ROOT`），列表返回的 `path` 可原样回传。
//!
//! - 浏览 / 读写：`GET /api/files`、`GET /api/files/read`、`POST /api/files/write`（原子写入，
//!   `expected_modified` 冲突检测 → 409）；
//! - 上传 / 下载：`POST /api/files/upload`（请求体即文件原始字节，流式落盘，≤ 2 GiB，
//!   `overwrite` 控制同名覆盖）、`GET /api/files/download`（流式，带 Content-Length）；
//! - 压缩 / 解压：`POST /api/files/compress`、`POST /api/files/extract`；
//! - 批量复制 / 移动：`POST /api/files/copy`、`POST /api/files/move`（逐项结果，一项失败不影响其它项）；
//! - 回收站：删除默认移入回收站（`permanent=true` 直接删除），`/api/files/recycle*` 列出 / 还原 / 清除。
//!
//! 权限：浏览 `file:read`；编辑、删除、复制移动、压缩解压、回收站操作 `file:write`；上传下载 `file:upload`。
use crate::api::extract::ApiJson;
use crate::api::types::{AppState, Username};
use crate::core::error::AppError;
use crate::file::archive::{CompressResult, ExtractResult};
use crate::file::recycle::{Recycle, RecycleItem};
use crate::file::transfer::{TransferMode, TransferResult};
use crate::file::{FileInfo, FileService, WriteResult};
use axum::Router;
use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    Json,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use utoipa::{IntoParams, ToSchema};

/// 从 AppState 构造沙箱文件服务（白名单根目录取自 OP_FILE_ROOT）
fn file_service(state: &AppState) -> FileService {
    FileService::new(state.file_root.clone())
}

#[derive(Deserialize, IntoParams)]
pub struct ListParams {
    /// 目录（根内相对路径，缺省 `/`）
    pub path: Option<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct ReadParams {
    /// 文件路径（根内相对路径）
    pub path: String,
}

#[derive(Deserialize, ToSchema)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
    /// 打开文件时看到的 `modified_at`；与当前不一致返回 409（缺省 = 强制覆盖）
    #[serde(default)]
    pub expected_modified: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRequest {
    pub path: String,
}

#[derive(Deserialize, IntoParams)]
pub struct DeleteParams {
    pub path: String,
    /// 非空目录必须为 true
    pub recursive: Option<bool>,
    /// true = 直接删除，不进回收站
    pub permanent: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct RenameRequest {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ChmodRequest {
    pub path: String,
    /// 八进制权限，如 `644` / `0755`
    pub mode: String,
}

#[derive(Deserialize, IntoParams)]
pub struct UploadQuery {
    /// 目标目录
    pub path: String,
    /// 文件名（不含路径）
    pub name: String,
    /// 同名文件存在时是否覆盖（缺省 false → 409）
    pub overwrite: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct CompressRequest {
    /// 同一目录下的若干条目
    pub paths: Vec<String>,
    /// 压缩包路径，扩展名决定格式：`.tar.gz` / `.tgz` / `.zip`
    pub dest: String,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ExtractRequest {
    /// 压缩包路径（zip / tar / tar.gz / tgz / tar.xz / txz / gz）
    pub path: String,
    /// 解压到的目录（缺省为压缩包所在目录）
    #[serde(default)]
    pub dest: Option<String>,
    /// 同名顶层条目存在时替换（旧条目移入回收站）；缺省 false → 409 且不做改动
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct TransferRequest {
    /// 源条目（根内路径，可来自不同目录；单次 ≤ 500 项）
    pub sources: Vec<String>,
    /// 目标目录（必须已存在）
    pub dest_dir: String,
    /// 同名条目存在时覆盖（旧条目移入回收站）；缺省 false → 该项失败（`error_kind = conflict`）
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PurgeResult {
    pub purged: usize,
}

async fn log_file_op(state: &AppState, username: &str, action: &str, target: &str) {
    // T10/A2：审计用户名从 JWT 认证上下文取，不再硬编码 "system"。
    state
        .operation_log_service
        .log(username, action, Some(target), None)
        .await
        .ok();
}

/// `GET /api/files` — 列出目录
#[utoipa::path(
    get,
    path = "/api/files",
    tag = "files",
    params(ListParams),
    responses(
        (status = 200, description = "目录条目（目录在前，按名称排序；回收站目录不出现）", body = [FileInfo]),
        (status = 400, description = "不是目录"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:read 权限或路径越界"),
        (status = 404, description = "路径不存在"),
    ),
    security(("BearerAuth" = []))
)]
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

/// `GET /api/files/read` — 读取文本文件（≤ 10 MiB，UTF-8）
#[utoipa::path(
    get,
    path = "/api/files/read",
    tag = "files",
    params(ReadParams),
    responses(
        (status = 200, description = "文件内容", body = String),
        (status = 400, description = "不是文件 / 超过 10 MiB / 不是 UTF-8 文本"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:read 权限或路径越界"),
        (status = 404, description = "路径不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn read(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<ReadParams>,
) -> Result<Json<String>, AppError> {
    let content = file_service(&state).read(&params.path).await?;
    log_file_op(&state, &username.0, "file_read", &params.path).await;
    Ok(Json(content))
}

/// `POST /api/files/write` — 保存文件（原子替换，保留权限与属主）
#[utoipa::path(
    post,
    path = "/api/files/write",
    tag = "files",
    request_body = WriteRequest,
    responses(
        (status = 200, description = "已保存，返回新的修改时间", body = WriteResult),
        (status = 400, description = "不是文件"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 404, description = "路径不存在"),
        (status = 409, description = "文件在打开后已被修改"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn write(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<WriteRequest>,
) -> Result<Json<WriteResult>, AppError> {
    let result = file_service(&state)
        .write(&req.path, &req.content, req.expected_modified.as_deref())
        .await?;
    log_file_op(&state, &username.0, "file_write", &req.path).await;
    Ok(Json(result))
}

/// `POST /api/files/create-file` — 新建空文件
#[utoipa::path(
    post,
    path = "/api/files/create-file",
    tag = "files",
    request_body = CreateRequest,
    responses(
        (status = 200, description = "已创建", body = String),
        (status = 400, description = "路径非法或父目录不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 409, description = "同名条目已存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_file(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state).create_file(&req.path).await?;
    log_file_op(&state, &username.0, "file_create_file", &req.path).await;
    Ok(Json("created"))
}

/// `POST /api/files/create-dir` — 新建目录
#[utoipa::path(
    post,
    path = "/api/files/create-dir",
    tag = "files",
    request_body = CreateRequest,
    responses(
        (status = 200, description = "已创建", body = String),
        (status = 400, description = "路径非法或父目录不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 409, description = "同名条目已存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_dir(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Json<&'static str>, AppError> {
    file_service(&state).create_dir(&req.path).await?;
    log_file_op(&state, &username.0, "file_create_dir", &req.path).await;
    Ok(Json("created"))
}

/// `DELETE /api/files/delete` — 删除（默认移入回收站）
#[utoipa::path(
    delete,
    path = "/api/files/delete",
    tag = "files",
    params(DeleteParams),
    responses(
        (status = 200, description = "已删除（或已移入回收站）", body = String),
        (status = 400, description = "目录非空且未确认递归 / 位于其他文件系统无法移入回收站"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限、路径越界或试图删除根目录"),
        (status = 404, description = "路径不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<DeleteParams>,
) -> Result<Json<&'static str>, AppError> {
    let permanent = params.permanent.unwrap_or(false);
    file_service(&state)
        .delete(
            &params.path,
            params.recursive.unwrap_or(false),
            permanent,
            &username.0,
        )
        .await?;
    log_file_op(
        &state,
        &username.0,
        if permanent {
            "file_delete"
        } else {
            "file_recycle"
        },
        &params.path,
    )
    .await;
    Ok(Json(if permanent { "deleted" } else { "recycled" }))
}

/// `POST /api/files/rename` — 重命名 / 移动
#[utoipa::path(
    post,
    path = "/api/files/rename",
    tag = "files",
    request_body = RenameRequest,
    responses(
        (status = 200, description = "已重命名", body = String),
        (status = 400, description = "路径非法或把目录移入自身"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 404, description = "源路径不存在"),
        (status = 409, description = "目标已存在"),
    ),
    security(("BearerAuth" = []))
)]
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

/// `POST /api/files/chmod` — 修改权限
#[utoipa::path(
    post,
    path = "/api/files/chmod",
    tag = "files",
    request_body = ChmodRequest,
    responses(
        (status = 200, description = "已修改", body = String),
        (status = 400, description = "权限值非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 404, description = "路径不存在"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 上传请求体大小上限：2 GiB（与 nginx client_max_body_size 对齐）
pub const MAX_UPLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// `POST /api/files/upload` — 上传文件（请求体为文件原始字节）
#[utoipa::path(
    post,
    path = "/api/files/upload",
    tag = "files",
    params(UploadQuery),
    request_body(content = Vec<u8>, content_type = "application/octet-stream", description = "文件原始字节（不是 multipart）"),
    responses(
        (status = 200, description = "已上传", body = String),
        (status = 400, description = "文件名非法 / 目标不是目录 / 超过 2 GiB"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:upload 权限或路径越界"),
        (status = 409, description = "同名文件已存在（未指定 overwrite）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn upload(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<UploadQuery>,
    body: Body,
) -> Result<Json<&'static str>, AppError> {
    let svc = file_service(&state);
    let (tmp, target) = svc.prepare_upload(
        &params.path,
        &params.name,
        params.overwrite.unwrap_or(false),
    )?;
    // 流式写入同目录临时文件：2 GiB 上传不占用等量内存；中断 / 超限时删除临时文件
    let written = async {
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| AppError::internal(format!("创建上传文件失败: {e}")))?;
        let mut stream = body.into_data_stream();
        let mut total: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::BadRequest(format!("上传中断: {e}")))?;
            total += chunk.len() as u64;
            if total > MAX_UPLOAD_BYTES {
                return Err(AppError::BadRequest(format!(
                    "上传文件超过 {} GiB 上限",
                    MAX_UPLOAD_BYTES >> 30
                )));
            }
            file.write_all(&chunk)
                .await
                .map_err(|e| AppError::internal(format!("写入上传文件失败: {e}")))?;
        }
        file.flush()
            .await
            .map_err(|e| AppError::internal(format!("写入上传文件失败: {e}")))?;
        Ok::<u64, AppError>(total)
    }
    .await;
    if let Err(e) = written {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e);
    }
    svc.finish_upload(&tmp, &target).await?;
    log_file_op(
        &state,
        &username.0,
        "file_upload",
        &format!("{}/{}", params.path.trim_end_matches('/'), params.name),
    )
    .await;
    Ok(Json("uploaded"))
}

/// RFC 5987 文件名编码（`filename*=UTF-8''…`），中文文件名下载不乱码。
fn content_disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

/// `GET /api/files/download` — 下载文件（流式）
#[utoipa::path(
    get,
    path = "/api/files/download",
    tag = "files",
    params(ReadParams),
    responses(
        (status = 200, description = "文件内容", content_type = "application/octet-stream"),
        (status = 400, description = "不是文件"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:upload 权限或路径越界"),
        (status = 404, description = "路径不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn download(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Query(params): Query<ReadParams>,
) -> Result<axum::response::Response, AppError> {
    let (name, file, mime, len) = file_service(&state).download(&params.path).await?;
    log_file_op(&state, &username.0, "file_download", &params.path).await;
    // S2：文件下载改为流式响应，避免大文件整体读入内存。
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let response = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_str(&mime).unwrap_or_else(|_| {
                axum::http::HeaderValue::from_static("application/octet-stream")
            }),
        )
        .header(axum::http::header::CONTENT_LENGTH, len)
        .header(
            axum::http::header::CONTENT_DISPOSITION,
            axum::http::HeaderValue::from_str(&content_disposition(&name))
                .map_err(|_| AppError::internal("Invalid content disposition header"))?,
        )
        .body(body)
        .map_err(|e| AppError::internal_with_source("Failed to build download response", e))?;
    Ok(response)
}

/// `POST /api/files/compress` — 压缩
#[utoipa::path(
    post,
    path = "/api/files/compress",
    tag = "files",
    request_body = CompressRequest,
    responses(
        (status = 200, description = "已生成压缩包", body = CompressResult),
        (status = 400, description = "未选择条目 / 条目不在同一目录 / 扩展名不支持 / 超出 zip 限制"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 404, description = "条目不存在"),
        (status = 409, description = "压缩包已存在（未指定 overwrite）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn compress(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<CompressRequest>,
) -> Result<Json<CompressResult>, AppError> {
    let result = file_service(&state)
        .compress(&req.paths, &req.dest, req.overwrite)
        .await?;
    log_file_op(&state, &username.0, "file_compress", &result.path).await;
    Ok(Json(result))
}

/// `POST /api/files/extract` — 解压
#[utoipa::path(
    post,
    path = "/api/files/extract",
    tag = "files",
    request_body = ExtractRequest,
    responses(
        (status = 200, description = "已解压", body = ExtractResult),
        (status = 400, description = "格式不支持 / 压缩包损坏 / 含路径穿越、包外符号链接、硬链接 / 超出体积与条目上限"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或路径越界"),
        (status = 404, description = "压缩包不存在"),
        (status = 409, description = "目标目录已有同名顶层条目（未指定 overwrite，未做任何改动）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn extract(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<ExtractRequest>,
) -> Result<Json<ExtractResult>, AppError> {
    let result = file_service(&state)
        .extract(&req.path, req.dest.as_deref(), req.overwrite, &username.0)
        .await?;
    log_file_op(&state, &username.0, "file_extract", &req.path).await;
    Ok(Json(result))
}

/// `GET /api/files/recycle` — 回收站列表
#[utoipa::path(
    get,
    path = "/api/files/recycle",
    tag = "files",
    responses(
        (status = 200, description = "回收站条目（新删除的在前；过期条目自动清理）", body = [RecycleItem]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn recycle_list(
    State(state): State<AppState>,
) -> Result<Json<Vec<RecycleItem>>, AppError> {
    let svc = file_service(&state);
    Ok(Json(Recycle::new(&svc).list().await?))
}

/// `POST /api/files/recycle/{id}/restore` — 还原到原位置
#[utoipa::path(
    post,
    path = "/api/files/recycle/{id}/restore",
    tag = "files",
    params(("id" = String, Path, description = "回收站条目 ID")),
    responses(
        (status = 200, description = "已还原", body = RecycleItem),
        (status = 400, description = "ID 非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限"),
        (status = 404, description = "条目不存在"),
        (status = 409, description = "原位置已存在同名条目"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn recycle_restore(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Path(id): Path<String>,
) -> Result<Json<RecycleItem>, AppError> {
    let svc = file_service(&state);
    let item = Recycle::new(&svc).restore(&id).await?;
    log_file_op(&state, &username.0, "file_restore", &item.original_path).await;
    Ok(Json(item))
}

/// `DELETE /api/files/recycle/{id}` — 彻底删除单个条目
#[utoipa::path(
    delete,
    path = "/api/files/recycle/{id}",
    tag = "files",
    params(("id" = String, Path, description = "回收站条目 ID")),
    responses(
        (status = 200, description = "已彻底删除", body = String),
        (status = 400, description = "ID 非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限"),
        (status = 404, description = "条目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn recycle_purge(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    Path(id): Path<String>,
) -> Result<Json<&'static str>, AppError> {
    let svc = file_service(&state);
    Recycle::new(&svc).purge(&id).await?;
    log_file_op(&state, &username.0, "file_recycle_purge", &id).await;
    Ok(Json("purged"))
}

/// `DELETE /api/files/recycle` — 清空回收站
#[utoipa::path(
    delete,
    path = "/api/files/recycle",
    tag = "files",
    responses(
        (status = 200, description = "已清空", body = PurgeResult),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn recycle_empty(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
) -> Result<Json<PurgeResult>, AppError> {
    let svc = file_service(&state);
    let purged = Recycle::new(&svc).purge_all().await?;
    log_file_op(
        &state,
        &username.0,
        "file_recycle_empty",
        &purged.to_string(),
    )
    .await;
    Ok(Json(PurgeResult { purged }))
}

async fn transfer(
    state: &AppState,
    username: &str,
    mode: TransferMode,
    req: TransferRequest,
) -> Result<Json<TransferResult>, AppError> {
    let result = file_service(state)
        .transfer(mode, &req.sources, &req.dest_dir, req.overwrite, username)
        .await?;
    let action = match mode {
        TransferMode::Copy => "file_copy",
        TransferMode::Move => "file_move",
    };
    log_file_op(
        state,
        username,
        action,
        &format!(
            "{} -> {}（成功 {} / 失败 {}）",
            req.sources.join(", "),
            req.dest_dir,
            result.succeeded,
            result.failed
        ),
    )
    .await;
    Ok(Json(result))
}

/// `POST /api/files/copy` — 批量复制到目录
#[utoipa::path(
    post,
    path = "/api/files/copy",
    tag = "files",
    request_body = TransferRequest,
    responses(
        (status = 200, description = "逐项结果（部分失败仍为 200，见 items[].ok / error_kind）", body = TransferResult),
        (status = 400, description = "未选择条目 / 超过 500 项 / 目标不是目录"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或目标目录越界"),
        (status = 404, description = "目标目录不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn copy_entries(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<TransferRequest>,
) -> Result<Json<TransferResult>, AppError> {
    transfer(&state, &username.0, TransferMode::Copy, req).await
}

/// `POST /api/files/move` — 批量移动到目录
#[utoipa::path(
    post,
    path = "/api/files/move",
    tag = "files",
    request_body = TransferRequest,
    responses(
        (status = 200, description = "逐项结果（部分失败仍为 200，见 items[].ok / error_kind）", body = TransferResult),
        (status = 400, description = "未选择条目 / 超过 500 项 / 目标不是目录"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 file:write 权限或目标目录越界"),
        (status = 404, description = "目标目录不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn move_entries(
    State(state): State<AppState>,
    Extension(username): Extension<Username>,
    ApiJson(req): ApiJson<TransferRequest>,
) -> Result<Json<TransferResult>, AppError> {
    transfer(&state, &username.0, TransferMode::Move, req).await
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    use axum::routing::{delete, get, post};
    Router::new()
        .route("/api/files", get(list))
        .route("/api/files/read", get(read))
        // 编辑器保存上限随读取上限（10 MiB）放宽：默认 2 MB 会拒绝较大的配置 / 日志文件
        .route(
            "/api/files/write",
            post(write).layer(axum::extract::DefaultBodyLimit::max(
                (crate::file::MAX_EDIT_BYTES as usize) * 2,
            )),
        )
        .route("/api/files/create-file", post(create_file))
        .route("/api/files/create-dir", post(create_dir))
        .route("/api/files/delete", delete(self::delete))
        .route("/api/files/rename", post(rename))
        .route("/api/files/chmod", post(chmod))
        .route("/api/files/copy", post(copy_entries))
        .route("/api/files/move", post(move_entries))
        // 上传为原始字节流，体积上限在 handler 内按流累计校验
        .route("/api/files/upload", post(upload))
        .route("/api/files/download", get(download))
        .route("/api/files/compress", post(compress))
        .route("/api/files/extract", post(extract))
        .route(
            "/api/files/recycle",
            get(recycle_list).delete(recycle_empty),
        )
        .route("/api/files/recycle/{id}", delete(recycle_purge))
        .route("/api/files/recycle/{id}/restore", post(recycle_restore))
}

#[cfg(test)]
mod tests {
    use super::content_disposition;

    #[test]
    fn content_disposition_encodes_non_ascii_names() {
        let h = content_disposition("报告 v1.txt");
        assert!(h.starts_with("attachment; filename=\"__ v1.txt\""), "{h}");
        assert!(
            h.contains("filename*=UTF-8''%E6%8A%A5%E5%91%8A%20v1.txt"),
            "{h}"
        );
        assert!(axum::http::HeaderValue::from_str(&h).is_ok());
        let q = content_disposition("a\"b.txt");
        assert!(q.contains("filename=\"a_b.txt\""), "{q}");
    }
}
