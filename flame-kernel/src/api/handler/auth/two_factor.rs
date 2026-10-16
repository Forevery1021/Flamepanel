//! 两步验证（TOTP）自助管理与管理员重置（Doc/33 排除项「TOTP 2FA」）。
//!
//! - `GET  /api/auth/2fa`                 状态（含面板是否强制）
//! - `POST /api/auth/2fa/setup`           生成待绑定密钥（需当前密码）
//! - `POST /api/auth/2fa/enable`          用验证码确认启用，返回恢复码（仅此一次）
//! - `POST /api/auth/2fa/disable`         停用（需当前密码 + 验证码或恢复码；面板强制时拒绝）
//! - `POST /api/auth/2fa/recovery-codes`  重新生成恢复码（需当前密码 + TOTP）
//! - `PUT  /api/users/{id}/2fa/reset`     管理员重置他人 2FA（`user:update` + admin 角色）
//!
//! 自助端点只需登录（`is_auth_only_path`）；密码 / 验证码错误返回 400（而非 401，避免前端
//! 误判会话失效），并计入「用户名 + IP」登录锁定计数，防止借已登录会话暴力枚举。
use crate::api::extract::ApiJson;
use crate::api::types::{AppState, UserId};
use crate::application::two_factor_service::{TwoFactorSetup, TwoFactorStatus};
use crate::core::error::AppError;
use crate::utils::password::PasswordUtils;
use axum::extract::{ConnectInfo, Extension, Path, State};
use axum::http::HeaderMap;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use utoipa::ToSchema;

/// 认证器 App 中显示的签发方
const ISSUER: &str = "FlamePanel";

#[derive(Debug, Serialize, ToSchema)]
pub struct TwoFactorStatusResponse {
    #[serde(flatten)]
    pub status: TwoFactorStatus,
    /// 面板是否强制所有用户启用两步验证（设置 `two_factor_enabled`）
    pub required: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TwoFactorSetupRequest {
    /// 当前密码（二次确认身份）
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TwoFactorEnableRequest {
    /// 认证器 App 显示的 6 位验证码
    pub code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TwoFactorConfirmRequest {
    /// 当前密码
    pub password: String,
    /// 6 位验证码（停用时也可填恢复码）
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecoveryCodesResponse {
    /// 一次性恢复码（明文仅此一次返回，请妥善保存）
    pub recovery_codes: Vec<String>,
}

struct Caller {
    user: crate::domain::entity::User,
    ip: String,
}

async fn caller(
    state: &AppState,
    user_id: i64,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> Result<Caller, AppError> {
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists".into()))?;
    let ip = crate::api::extract::extract_client_ip_trusted(Some(peer), headers);
    state
        .login_attempts
        .check_locked(&user.username, &ip)
        .await?;
    Ok(Caller { user, ip })
}

async fn fail(state: &AppState, c: &Caller, msg: &str) -> AppError {
    let policy = state.settings_service.policy();
    state
        .login_attempts
        .record_failure_with(
            &c.user.username,
            &c.ip,
            policy.login_max_failures,
            std::time::Duration::from_secs(policy.login_lockout_minutes * 60),
        )
        .await;
    AppError::BadRequest(msg.to_string())
}

async fn check_password(state: &AppState, c: &Caller, password: &str) -> Result<(), AppError> {
    let hash = c.user.password_hash.clone();
    let pw = password.to_string();
    let ok = tokio::task::spawn_blocking(move || PasswordUtils::verify(&pw, &hash))
        .await
        .map_err(|e| AppError::internal(format!("bcrypt task panicked: {}", e)))??;
    if ok {
        Ok(())
    } else {
        Err(fail(state, c, "invalid password").await)
    }
}

async fn audit(state: &AppState, c: &Caller, action: &str, target: Option<&str>) {
    let ip = (c.ip != "unknown").then_some(c.ip.as_str());
    let _ = state
        .operation_log_service
        .log(&c.user.username, action, target, ip)
        .await;
}

/// 两步验证状态
#[utoipa::path(
    get,
    path = "/api/auth/2fa",
    tag = "auth",
    responses(
        (status = 200, description = "当前用户的两步验证状态", body = TwoFactorStatusResponse),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn status(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<TwoFactorStatusResponse>, AppError> {
    Ok(Json(TwoFactorStatusResponse {
        status: state.two_factor_service.status(user_id).await?,
        required: state.settings_service.policy().require_two_factor,
    }))
}

/// 开始绑定：生成待验证密钥与 otpauth URI（已启用时不影响旧密钥，确认后才替换）
#[utoipa::path(
    post,
    path = "/api/auth/2fa/setup",
    tag = "auth",
    request_body = TwoFactorSetupRequest,
    responses(
        (status = 200, description = "待绑定密钥", body = TwoFactorSetup),
        (status = 400, description = "密码错误"),
        (status = 401, description = "未认证"),
        (status = 429, description = "尝试过于频繁"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn setup(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<TwoFactorSetupRequest>,
) -> Result<Json<TwoFactorSetup>, AppError> {
    let c = caller(&state, user_id, &peer, &headers).await?;
    check_password(&state, &c, &req.password).await?;
    let info = state
        .two_factor_service
        .begin_setup(user_id, &c.user.username, ISSUER)
        .await?;
    Ok(Json(info))
}

/// 确认启用：校验待绑定密钥的验证码，返回一次性恢复码
#[utoipa::path(
    post,
    path = "/api/auth/2fa/enable",
    tag = "auth",
    request_body = TwoFactorEnableRequest,
    responses(
        (status = 200, description = "已启用；恢复码仅此一次返回", body = RecoveryCodesResponse),
        (status = 400, description = "未开始绑定或验证码错误"),
        (status = 401, description = "未认证"),
        (status = 429, description = "尝试过于频繁"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn enable(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<TwoFactorEnableRequest>,
) -> Result<Json<RecoveryCodesResponse>, AppError> {
    let c = caller(&state, user_id, &peer, &headers).await?;
    match state.two_factor_service.enable(user_id, &req.code).await {
        Ok(codes) => {
            state.login_attempts.reset(&c.user.username, &c.ip).await;
            audit(&state, &c, "2FA_ENABLED", None).await;
            Ok(Json(RecoveryCodesResponse {
                recovery_codes: codes,
            }))
        }
        Err(AppError::BadRequest(m)) if m == "invalid verification code" => {
            Err(fail(&state, &c, &m).await)
        }
        Err(e) => Err(e),
    }
}

/// 停用两步验证
#[utoipa::path(
    post,
    path = "/api/auth/2fa/disable",
    tag = "auth",
    request_body = TwoFactorConfirmRequest,
    responses(
        (status = 200, description = "已停用"),
        (status = 400, description = "密码或验证码错误"),
        (status = 401, description = "未认证"),
        (status = 409, description = "面板强制两步验证，不可停用"),
        (status = 429, description = "尝试过于频繁"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn disable(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<TwoFactorConfirmRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if state.settings_service.policy().require_two_factor {
        return Err(AppError::Conflict(
            "two-factor authentication is enforced by the panel policy".into(),
        ));
    }
    let c = caller(&state, user_id, &peer, &headers).await?;
    check_password(&state, &c, &req.password).await?;
    if state
        .two_factor_service
        .verify_second_factor(user_id, &req.code)
        .await?
        .is_none()
    {
        return Err(fail(&state, &c, "invalid verification code").await);
    }
    state.two_factor_service.disable(user_id).await?;
    state.login_attempts.reset(&c.user.username, &c.ip).await;
    audit(&state, &c, "2FA_DISABLED", None).await;
    Ok(Json(
        serde_json::json!({ "message": "two-factor authentication disabled" }),
    ))
}

/// 重新生成恢复码（旧码全部作废）
#[utoipa::path(
    post,
    path = "/api/auth/2fa/recovery-codes",
    tag = "auth",
    request_body = TwoFactorConfirmRequest,
    responses(
        (status = 200, description = "新恢复码（仅此一次返回）", body = RecoveryCodesResponse),
        (status = 400, description = "密码或验证码错误 / 未启用"),
        (status = 401, description = "未认证"),
        (status = 429, description = "尝试过于频繁"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn regenerate_recovery_codes(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<TwoFactorConfirmRequest>,
) -> Result<Json<RecoveryCodesResponse>, AppError> {
    let c = caller(&state, user_id, &peer, &headers).await?;
    check_password(&state, &c, &req.password).await?;
    if !state
        .two_factor_service
        .verify_totp(user_id, &req.code)
        .await?
    {
        return Err(fail(&state, &c, "invalid verification code").await);
    }
    let codes = state
        .two_factor_service
        .regenerate_recovery_codes(user_id)
        .await?;
    state.login_attempts.reset(&c.user.username, &c.ip).await;
    audit(&state, &c, "2FA_RECOVERY_CODES_REGENERATED", None).await;
    Ok(Json(RecoveryCodesResponse {
        recovery_codes: codes,
    }))
}

/// 管理员重置指定用户的两步验证（用户丢失认证器且恢复码用尽时；需 `user:update` + admin 角色）
#[utoipa::path(
    put,
    path = "/api/users/{id}/2fa/reset",
    tag = "users",
    operation_id = "reset_user_two_factor",
    params(("id" = i64, Path, description = "用户 ID")),
    responses(
        (status = 200, description = "已重置"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 user:update 权限或非 admin 角色"),
        (status = 404, description = "用户不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn admin_reset(
    State(state): State<AppState>,
    Extension(UserId(admin_id)): Extension<UserId>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let target = state
        .user_service
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {id} not found")))?;
    let c = caller(&state, admin_id, &peer, &headers).await?;
    // 除 `user:update` 外还要求 admin 角色：operator 在后端种子里也有 `user:update`，
    // 但不应能摘掉（尤其是管理员的）第二因子。
    if c.user.role != "admin" {
        return Err(AppError::Forbidden(
            "only administrators can reset two-factor authentication".into(),
        ));
    }
    let existed = state.two_factor_service.disable(id).await?;
    audit(&state, &c, "2FA_RESET", Some(&target.username)).await;
    Ok(Json(serde_json::json!({ "reset": existed })))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/2fa", axum::routing::get(status))
        .route("/api/auth/2fa/setup", axum::routing::post(setup))
        .route("/api/auth/2fa/enable", axum::routing::post(enable))
        .route("/api/auth/2fa/disable", axum::routing::post(disable))
        .route(
            "/api/auth/2fa/recovery-codes",
            axum::routing::post(regenerate_recovery_codes),
        )
        .route("/api/users/{id}/2fa/reset", axum::routing::put(admin_reset))
}
