use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::backup_account_service::BackupAccountService;
use crate::core::error::AppError;
use crate::domain::entity::BackupAccount;
use axum::Router;
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json,
};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub name: String,
    /// 账号类型：s3 | sftp | local
    pub account_type: String,
    /// 后端连接配置（JSON 字符串，敏感字段按各后端约定脱敏存储）
    pub config_json: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub account_type: Option<String>,
    pub config_json: Option<String>,
    pub enabled: Option<bool>,
}

fn account_response(a: &BackupAccount) -> AccountDto {
    AccountDto {
        id: a.id,
        name: a.name.clone(),
        account_type: a.account_type.clone(),
        enabled: a.enabled,
        last_test_status: a.last_test_status.clone(),
        last_test_at: a.last_test_at,
        created_at: a.created_at,
        updated_at: a.updated_at,
        test_message: None,
    }
}

/// 账号响应（config_json 不回显，防止敏感连接信息泄漏）
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AccountDto {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub enabled: bool,
    pub last_test_status: Option<String>,
    pub last_test_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 最近一次探测失败原因（成功为 null；仅 test 端点返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_message: Option<String>,
}

/// 备份账号列表
#[utoipa::path(
    get,
    path = "/api/backup-accounts",
    tag = "backup_accounts",
    responses(
        (status = 200, description = "账号列表", body = Vec<AccountDto>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_accounts(
    State(state): State<AppState>,
) -> Result<Json<Vec<AccountDto>>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    let accounts = svc.list_accounts().await?;
    Ok(Json(accounts.iter().map(account_response).collect()))
}

/// 创建备份账号
#[utoipa::path(
    post,
    path = "/api/backup-accounts",
    tag = "backup_accounts",
    request_body = CreateAccountRequest,
    responses(
        (status = 200, description = "创建成功", body = AccountDto),
        (status = 400, description = "参数错误"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_account(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateAccountRequest>,
) -> Result<Json<AccountDto>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    let account = svc
        .create_account(BackupAccount {
            id: 0,
            name: req.name,
            account_type: req.account_type,
            config_json: req.config_json.unwrap_or_else(|| "{}".into()),
            enabled: req.enabled.unwrap_or(true),
            ..Default::default()
        })
        .await?;
    Ok(Json(account_response(&account)))
}

/// 更新备份账号
#[utoipa::path(
    put,
    path = "/api/backup-accounts/{id}",
    tag = "backup_accounts",
    params(("id" = i64, Path, description = "账号 ID")),
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "更新成功", body = AccountDto),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateAccountRequest>,
) -> Result<Json<AccountDto>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    let existing = svc.get_account(id).await?;
    let mut account = existing.clone();
    if let Some(name) = req.name {
        account.name = name;
    }
    if let Some(t) = req.account_type {
        account.account_type = t;
    }
    if let Some(cfg) = req.config_json {
        account.config_json = cfg;
    }
    if let Some(enabled) = req.enabled {
        account.enabled = enabled;
    }
    let updated = svc.update_account(&account).await?;
    Ok(Json(account_response(&updated)))
}

/// 删除备份账号
#[utoipa::path(
    delete,
    path = "/api/backup-accounts/{id}",
    tag = "backup_accounts",
    params(("id" = i64, Path, description = "账号 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    svc.delete_account(id).await?;
    Ok(Json(()))
}

/// 连通性测试（Stage 2：按账号类型构造真实后端探测，结果落库）
#[utoipa::path(
    post,
    path = "/api/backup-accounts/{id}/test",
    tag = "backup_accounts",
    params(("id" = i64, Path, description = "账号 ID")),
    responses(
        (status = 200, description = "测试完成，返回账号（含 last_test_status）", body = AccountDto),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn test_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<AccountDto>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    // Stage 2：真实探测（连接 / 认证 / 写探针），探测失败返回 200 + ok=false（业务可预期结果）
    let (ok, message) = svc.test_connection(id).await?;
    let account = svc.get_account(id).await?;
    let mut dto = account_response(&account);
    dto.test_message = message;
    if !ok {
        dto.last_test_status = Some("failed".into());
    }
    Ok(Json(dto))
}

/// 远端备份对象（云端账号内）
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct RemoteObjectDto {
    pub key: String,
    pub size: i64,
}

/// 上传本地备份到云端账号请求
#[derive(serde::Deserialize, ToSchema)]
pub struct UploadObjectRequest {
    /// 本地备份文件名（位于备份目录内，如 `flamepanel-20260907.db`）
    pub local_filename: String,
    /// 远端对象 key（缺省用文件名）
    pub remote_key: Option<String>,
}

/// 列出云端账号的备份对象
#[utoipa::path(
    get,
    path = "/api/backup-accounts/{id}/objects",
    tag = "backup_accounts",
    params(("id" = i64, Path, description = "账号 ID")),
    responses(
        (status = 200, description = "远端对象列表", body = Vec<RemoteObjectDto>),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_remote_objects(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<RemoteObjectDto>>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    let objects = svc.list_remote_backups(id).await?;
    Ok(Json(
        objects
            .into_iter()
            .map(|o| RemoteObjectDto {
                key: o.key,
                size: o.size,
            })
            .collect(),
    ))
}

/// 上传本地备份文件到云端账号
#[utoipa::path(
    post,
    path = "/api/backup-accounts/{id}/objects/upload",
    tag = "backup_accounts",
    params(("id" = i64, Path, description = "账号 ID")),
    request_body = UploadObjectRequest,
    responses(
        (status = 200, description = "上传成功，返回远端对象", body = RemoteObjectDto),
        (status = 400, description = "参数错误（文件名非法 / 账号停用）"),
        (status = 404, description = "账号或本地文件不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn upload_remote_object(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UploadObjectRequest>,
) -> Result<Json<RemoteObjectDto>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    // local_filename 只允许安全文件名（防路径穿越）
    if req.local_filename.contains('/')
        || req.local_filename.contains('\\')
        || req.local_filename.contains("..")
        || req.local_filename.trim().is_empty()
    {
        return Err(AppError::BadRequest("非法备份文件名".into()));
    }
    let local_path = state
        .backup_service
        .get_backup_path(&req.local_filename)
        .await?;
    let remote_key = req.remote_key.unwrap_or_else(|| req.local_filename.clone());
    let obj = svc.upload_backup(id, &local_path, &remote_key).await?;
    Ok(Json(RemoteObjectDto {
        key: obj.key,
        size: obj.size,
    }))
}

/// 删除云端账号的备份对象（key 不存在时幂等成功）
#[utoipa::path(
    delete,
    path = "/api/backup-accounts/{id}/objects",
    tag = "backup_accounts",
    params(
        ("id" = i64, Path, description = "账号 ID"),
        ("key" = String, Query, description = "远端对象 key")
    ),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_remote_object(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(q): axum::extract::Query<DeleteObjectQuery>,
) -> Result<Json<()>, AppError> {
    let svc: &BackupAccountService = &state.services.backup_account_service;
    svc.delete_remote_backup(id, &q.key).await?;
    Ok(Json(()))
}

#[derive(serde::Deserialize, ToSchema)]
pub struct DeleteObjectQuery {
    pub key: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/backup-accounts",
            get(list_accounts).post(create_account),
        )
        .route(
            "/api/backup-accounts/{id}",
            put(update_account).delete(delete_account),
        )
        .route("/api/backup-accounts/{id}/test", post(test_account))
        .route(
            "/api/backup-accounts/{id}/objects",
            get(list_remote_objects).delete(delete_remote_object),
        )
        .route(
            "/api/backup-accounts/{id}/objects/upload",
            post(upload_remote_object),
        )
}
