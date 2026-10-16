//! DNS 厂商账号 REST 端点（主线三 §6.1 · Stage 4）
//!
//! 响应不回显 `credentials_json`，防止厂商 API 凭据泄漏（对齐备份账号 Dto 语义）。
use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::dns_provider_service::DnsProviderAccountService;
use crate::core::error::AppError;
use crate::domain::entity::DnsProviderAccount;
use axum::Router;
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json,
};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateDnsProviderRequest {
    pub name: String,
    /// 厂商标识：cloudflare | aliyun | tencent | huawei
    pub provider_type: String,
    /// 厂商凭据（JSON 字符串，落库前统一加密）
    pub credentials_json: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateDnsProviderRequest {
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub credentials_json: Option<String>,
    pub enabled: Option<bool>,
}

fn account_response(a: &DnsProviderAccount) -> DnsProviderAccountDto {
    DnsProviderAccountDto {
        id: a.id,
        name: a.name.clone(),
        provider_type: a.provider_type.clone(),
        enabled: a.enabled,
        last_test_status: a.last_test_status.clone(),
        last_test_at: a.last_test_at,
        created_at: a.created_at,
        updated_at: a.updated_at,
        test_message: None,
    }
}

/// 厂商账号响应（credentials_json 不回显，防止敏感凭据泄漏）
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct DnsProviderAccountDto {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub last_test_status: Option<String>,
    pub last_test_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 最近一次探测失败原因（成功为 null；仅 test 端点返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_message: Option<String>,
}

/// DNS 厂商账号列表
#[utoipa::path(
    get,
    path = "/api/dns-providers",
    tag = "dns_providers",
    responses(
        (status = 200, description = "账号列表", body = Vec<DnsProviderAccountDto>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_accounts(
    State(state): State<AppState>,
) -> Result<Json<Vec<DnsProviderAccountDto>>, AppError> {
    let svc: &DnsProviderAccountService = &state.dns_provider_account_service;
    let accounts = svc.list_accounts().await?;
    Ok(Json(accounts.iter().map(account_response).collect()))
}

/// 创建 DNS 厂商账号
#[utoipa::path(
    post,
    path = "/api/dns-providers",
    tag = "dns_providers",
    request_body = CreateDnsProviderRequest,
    responses(
        (status = 200, description = "创建成功", body = DnsProviderAccountDto),
        (status = 400, description = "参数错误"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_account(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateDnsProviderRequest>,
) -> Result<Json<DnsProviderAccountDto>, AppError> {
    let svc: &DnsProviderAccountService = &state.dns_provider_account_service;
    let account = svc
        .create_account(DnsProviderAccount {
            id: 0,
            name: req.name,
            provider_type: req.provider_type,
            credentials_json: req.credentials_json.unwrap_or_else(|| "{}".into()),
            enabled: req.enabled.unwrap_or(true),
            ..Default::default()
        })
        .await?;
    Ok(Json(account_response(&account)))
}

/// 更新 DNS 厂商账号
#[utoipa::path(
    put,
    path = "/api/dns-providers/{id}",
    tag = "dns_providers",
    params(("id" = i64, Path, description = "账号 ID")),
    request_body = UpdateDnsProviderRequest,
    responses(
        (status = 200, description = "更新成功", body = DnsProviderAccountDto),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateDnsProviderRequest>,
) -> Result<Json<DnsProviderAccountDto>, AppError> {
    let svc: &DnsProviderAccountService = &state.dns_provider_account_service;
    let existing = svc.get_account(id).await?;
    let mut account = existing.clone();
    if let Some(name) = req.name {
        account.name = name;
    }
    if let Some(t) = req.provider_type {
        account.provider_type = t;
    }
    if let Some(cfg) = req.credentials_json {
        account.credentials_json = cfg;
    }
    if let Some(enabled) = req.enabled {
        account.enabled = enabled;
    }
    let updated = svc.update_account(&account).await?;
    Ok(Json(account_response(&updated)))
}

/// 删除 DNS 厂商账号
#[utoipa::path(
    delete,
    path = "/api/dns-providers/{id}",
    tag = "dns_providers",
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
    let svc: &DnsProviderAccountService = &state.dns_provider_account_service;
    svc.delete_account(id).await?;
    Ok(Json(()))
}

/// 连通性测试（Stage 4：凭据结构校验级探测；Stage 5 升级为真实厂商 API 调用）
#[utoipa::path(
    post,
    path = "/api/dns-providers/{id}/test",
    tag = "dns_providers",
    params(("id" = i64, Path, description = "账号 ID")),
    responses(
        (status = 200, description = "测试完成，返回账号（含 last_test_status）", body = DnsProviderAccountDto),
        (status = 404, description = "账号不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn test_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DnsProviderAccountDto>, AppError> {
    let svc: &DnsProviderAccountService = &state.dns_provider_account_service;
    // 探测失败返回 200 + ok=false（业务可预期结果）
    let (ok, message) = svc.test_connection(id).await?;
    let account = svc.get_account(id).await?;
    let mut dto = account_response(&account);
    dto.test_message = message;
    if !ok {
        dto.last_test_status = Some("failed".into());
    }
    Ok(Json(dto))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/dns-providers",
            get(list_accounts).post(create_account),
        )
        .route(
            "/api/dns-providers/{id}",
            put(update_account).delete(delete_account),
        )
        .route("/api/dns-providers/{id}/test", post(test_account))
}
