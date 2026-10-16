//! SSL 证书 REST 端点（M-B 主线三 §6.1 · Stage 2 签发链路）。
//!
//! 权限挂 `certificate` 资源组（read/update/delete），路由集中注册于
//! `routes.rs` 组合根。

use crate::api::app_state::AppState;
use crate::api::extract::ApiJson;
use crate::core::error::AppError;
use crate::domain::entity::Certificate;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, ToSchema)]
pub struct IssueAcmeRequest {
    /// SAN 域名列表（主域名 + 备用/泛域名）
    pub domains: Vec<String>,
    /// ACME 目录 URL（空 = Let's Encrypt 生产；staging 联调见常量）
    #[serde(default)]
    pub directory_url: Option<String>,
    /// 是否 DNS-01（泛域名必须）
    #[serde(default)]
    pub dns01: bool,
    /// HTTP-01 webroot（网站 root_path；DNS-01 时忽略）
    #[serde(default)]
    pub webroot: Option<String>,
    /// 签发后绑定网站 ID（可选）
    #[serde(default)]
    pub website_id: Option<i64>,
    /// DNS-01 厂商账号 ID（Stage 5：按账号解析真实厂商凭据；泛域名必填）
    #[serde(default)]
    pub dns_provider_account_id: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct IssueSelfSignedRequest {
    pub domains: Vec<String>,
    #[serde(default)]
    pub website_id: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct UploadCertificateRequest {
    pub domains: Vec<String>,
    /// 证书 PEM（全链，含中间证书）
    pub cert_pem: String,
    /// 私钥 PEM
    pub key_pem: String,
    #[serde(default)]
    pub website_id: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCertificateRequest {
    #[serde(default)]
    pub auto_renew: Option<bool>,
    #[serde(default)]
    pub website_id: Option<Option<i64>>,
}

#[derive(Deserialize, ToSchema)]
pub struct BindCertificateRequest {
    pub website_id: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CertificateListResponse {
    pub certificates: Vec<Certificate>,
    pub total: i64,
}

/// 续签历史查询参数（S-10）
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct RenewalHistoryQuery {
    /// 返回条数：缺省 50，范围 1–200
    pub limit: Option<usize>,
}

/// 证书续签概况 + 时间线（S-10）
#[derive(Debug, Serialize, ToSchema)]
pub struct CertificateRenewalHistory {
    pub certificate_id: i64,
    /// 当前自动续签开关
    pub auto_renew: bool,
    /// 来源是否支持自动续签（仅 ACME；自签 / 上传为 false，开关应禁用）
    pub supports_auto_renew: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 续签窗口起点（到期前 30 天）：此后每次后台扫描都会尝试续签
    pub renewal_window_starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_renewed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 后台续签扫描周期（小时）
    pub scan_interval_hours: u64,
    /// 时间线（最新在前）
    pub entries: Vec<crate::domain::cert_renewal::RenewalEntry>,
    /// 是否还有更早的记录未返回
    pub truncated: bool,
}

/// 证书列表
#[utoipa::path(
    get,
    path = "/api/certificates",
    tag = "certificates",
    responses(
        (status = 200, description = "证书列表", body = CertificateListResponse),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_certificates(
    State(state): State<AppState>,
) -> Result<Json<CertificateListResponse>, AppError> {
    let certificates = state.certificate_service.list_certificates().await?;
    Ok(Json(CertificateListResponse {
        total: certificates.len() as i64,
        certificates,
    }))
}

/// 证书详情
#[utoipa::path(
    get,
    path = "/api/certificates/{id}",
    tag = "certificates",
    params(("id" = i64, Path, description = "证书 ID")),
    responses(
        (status = 200, description = "证书详情", body = Certificate),
        (status = 404, description = "证书记录不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_certificate(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Certificate>, AppError> {
    Ok(Json(state.certificate_service.get_certificate(id).await?))
}

/// 更新证书（auto_renew / website_id）
#[utoipa::path(
    put,
    path = "/api/certificates/{id}",
    tag = "certificates",
    params(("id" = i64, Path, description = "证书 ID")),
    request_body = UpdateCertificateRequest,
    responses(
        (status = 200, description = "更新后的证书", body = Certificate),
        (status = 400, description = "自签 / 上传证书不支持开启自动续签"),
        (status = 404, description = "证书记录不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_certificate(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateCertificateRequest>,
) -> Result<Json<Certificate>, AppError> {
    let mut cert = state.certificate_service.get_certificate(id).await?;
    if let Some(auto) = req.auto_renew {
        // S-10：自签 / 上传证书没有签发方可续，开启自动续签只会每轮扫描徒劳失败
        if auto && !cert.supports_auto_renew() {
            return Err(AppError::ValidationError(format!(
                "证书来源 {} 不支持自动续签（仅 ACME 签发的证书可开启）",
                cert.source
            )));
        }
        cert.auto_renew = auto;
    }
    if let Some(wid) = req.website_id {
        cert.website_id = wid;
    }
    state.certificate_service.update_certificate(&cert).await?;
    Ok(Json(state.certificate_service.get_certificate(id).await?))
}

/// 删除证书记录（不删除落盘文件——运维可手动留存）
#[utoipa::path(
    delete,
    path = "/api/certificates/{id}",
    tag = "certificates",
    params(("id" = i64, Path, description = "证书 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "证书记录不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_certificate(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.certificate_service.delete_certificate(id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// ACME 签发（Let's Encrypt 等；HTTP-01 / DNS-01）
#[utoipa::path(
    post,
    path = "/api/certificates/issue/acme",
    tag = "certificates",
    request_body = IssueAcmeRequest,
    responses(
        (status = 200, description = "签发成功，返回证书记录", body = Certificate),
        (status = 400, description = "参数非法（泛域名未用 DNS-01 / 未配 DNS 厂商）"),
        (status = 502, description = "ACME 服务器/验证失败"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn issue_acme(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<IssueAcmeRequest>,
) -> Result<Json<Certificate>, AppError> {
    let account_key = state.acme_account_key_store.ensure_account_key()?;
    let directory = req
        .directory_url
        .unwrap_or_else(|| crate::infrastructure::acme::client::LETSENCRYPT_DIRECTORY.to_string());
    // Stage 5：指定了厂商账号 → 解析厂商类型名传入签发编排（凭据在编排层解密）
    let provider_name = match req.dns_provider_account_id {
        Some(id) => {
            let account = state.dns_provider_account_service.get_account(id).await?;
            Some(account.provider_type)
        }
        None => None,
    };
    let id = state
        .certificate_service
        .issue_acme_with_provider(
            req.domains,
            &directory,
            &account_key,
            req.dns01,
            req.webroot.as_deref().map(std::path::Path::new),
            provider_name.as_deref(),
        )
        .await?;
    if let Some(wid) = req.website_id {
        state
            .certificate_service
            .bind_website(id, Some(wid))
            .await?;
    }
    Ok(Json(state.certificate_service.get_certificate(id).await?))
}

/// 自签证书（内网/无公网场景）
#[utoipa::path(
    post,
    path = "/api/certificates/issue/self-signed",
    tag = "certificates",
    request_body = IssueSelfSignedRequest,
    responses(
        (status = 200, description = "签发成功", body = Certificate),
        (status = 400, description = "域名列表非法"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn issue_self_signed(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<IssueSelfSignedRequest>,
) -> Result<Json<Certificate>, AppError> {
    let id = state
        .certificate_service
        .issue_self_signed(req.domains)
        .await?;
    if let Some(wid) = req.website_id {
        state
            .certificate_service
            .bind_website(id, Some(wid))
            .await?;
    }
    Ok(Json(state.certificate_service.get_certificate(id).await?))
}

/// 上传第三方证书
#[utoipa::path(
    post,
    path = "/api/certificates",
    tag = "certificates",
    request_body = UploadCertificateRequest,
    responses(
        (status = 200, description = "上传成功", body = Certificate),
        (status = 400, description = "PEM 非法 / 域名列表非法"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn upload_certificate(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UploadCertificateRequest>,
) -> Result<Json<Certificate>, AppError> {
    let id = state
        .certificate_service
        .upload_certificate(req.domains, &req.cert_pem, &req.key_pem, req.website_id)
        .await?;
    Ok(Json(state.certificate_service.get_certificate(id).await?))
}

/// 证书绑定/解绑网站（website_id = None 解绑）
#[utoipa::path(
    post,
    path = "/api/certificates/{id}/bind",
    tag = "certificates",
    params(("id" = i64, Path, description = "证书 ID")),
    request_body = BindCertificateRequest,
    responses(
        (status = 200, description = "绑定后的证书", body = Certificate),
        (status = 400, description = "证书 SAN 不覆盖站点域名"),
        (status = 404, description = "证书/网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn bind_website(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<BindCertificateRequest>,
) -> Result<Json<Certificate>, AppError> {
    let cert = state
        .certificate_service
        .bind_website(id, req.website_id)
        .await?;
    Ok(Json(cert))
}

/// 续签历史（S-10）：签发 / 自动续签成功与失败 / 到期预警时间线，数据来自事件落库
#[utoipa::path(
    get,
    path = "/api/certificates/{id}/renewals",
    tag = "certificates",
    params(("id" = i64, Path, description = "证书 ID"), RenewalHistoryQuery),
    responses(
        (status = 200, description = "续签概况与时间线（最新在前）", body = CertificateRenewalHistory),
        (status = 401, description = "未认证"),
        (status = 403, description = "无 certificate:read 权限"),
        (status = 404, description = "证书记录不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn renewal_history(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(q): axum::extract::Query<RenewalHistoryQuery>,
) -> Result<Json<CertificateRenewalHistory>, AppError> {
    use crate::domain::cert_renewal::DEFAULT_HISTORY_LIMIT;
    let cert = state.certificate_service.get_certificate(id).await?;
    let (entries, truncated) = state
        .outbox_service
        .certificate_timeline(cert.id, q.limit.unwrap_or(DEFAULT_HISTORY_LIMIT))
        .await?;
    Ok(Json(CertificateRenewalHistory {
        certificate_id: cert.id,
        auto_renew: cert.auto_renew,
        supports_auto_renew: cert.supports_auto_renew(),
        expires_at: cert.expires_at,
        renewal_window_starts_at: cert.renewal_window_starts_at(),
        last_renewed_at: cert.last_renewed_at,
        scan_interval_hours: crate::application::certificate_service::RENEW_SCAN_INTERVAL_HOURS,
        entries,
        truncated,
    }))
}

/// 手动触发续签（ACME 来源；自签/上传返回 400）
#[utoipa::path(
    post,
    path = "/api/certificates/{id}/renew",
    tag = "certificates",
    params(("id" = i64, Path, description = "证书 ID")),
    responses(
        (status = 200, description = "续签成功，返回刷新后的证书记录", body = Certificate),
        (status = 400, description = "来源不支持自动续签（自签/上传）或验证配置缺失"),
        (status = 404, description = "证书记录不存在"),
        (status = 502, description = "ACME 签发失败"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn renew_certificate(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Certificate>, AppError> {
    let cert = state.certificate_service.renew_certificate(id).await?;
    Ok(Json(cert))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/certificates",
            axum::routing::get(list_certificates).post(upload_certificate),
        )
        .route(
            "/api/certificates/{id}",
            axum::routing::get(get_certificate)
                .put(update_certificate)
                .delete(delete_certificate),
        )
        .route("/api/certificates/issue/acme", post(issue_acme))
        .route(
            "/api/certificates/issue/self-signed",
            post(issue_self_signed),
        )
        .route("/api/certificates/{id}/bind", post(bind_website))
        .route("/api/certificates/{id}/renew", post(renew_certificate))
        .route(
            "/api/certificates/{id}/renewals",
            axum::routing::get(renewal_history),
        )
}
