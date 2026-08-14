use crate::api::extract::ApiJson;
use crate::api::types::{AppState, UserId};
use crate::core::error::AppError;
use crate::utils::password::PasswordUtils;
use axum::Router;
use axum::{
    extract::{Extension, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// Access Token（短过期，默认 15 分钟）
    pub token: String,
    /// Refresh Token（长过期，默认 24 小时；用于换取新 Access Token）
    pub refresh_token: String,
    pub username: String,
    pub role: String,
    /// 是否需强制修改密码（新装面板首次登录）
    pub must_change_password: bool,
}

/// 登录
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "凭据错误"),
        (status = 429, description = "尝试过于频繁"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ApiJson(req): ApiJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // 登录失败锁定检查
    state.login_attempts.check_locked(&req.username).await?;

    let ip = client_ip(&headers);

    let user = state
        .user_service
        .find_by_username(&req.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    if !PasswordUtils::verify(&req.password, &user.password_hash)? {
        state.login_attempts.record_failure(&req.username).await;
        // 登录失败审计
        let _ = state
            .operation_log_service
            .log(&req.username, "LOGIN_FAILED", None, ip.as_deref())
            .await;
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }
    state.login_attempts.reset(&req.username).await;

    // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
    let jwt = state.shared_jwt();
    let token = jwt.sign_access(user.id)?;
    let refresh_token = jwt.sign_refresh(user.id)?;

    // 登录成功审计
    let _ = state
        .operation_log_service
        .log(&user.username, "LOGIN_SUCCESS", None, ip.as_deref())
        .await;

    // 登录成功事件
    let _ = state
        .event_bus
        .publish(crate::domain::entity::DomainEvent::UserLoggedIn {
            username: user.username.clone(),
        })
        .await;

    Ok(Json(LoginResponse {
        token,
        refresh_token,
        username: user.username,
        role: user.role,
        must_change_password: user.must_change_password,
    }))
}

/// 从请求头提取客户端 IP（X-Real-IP 优先，回退 X-Forwarded-For 首值）
fn client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("X-Real-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
}

/// 刷新令牌：必须使用 Refresh Token（Bearer），换取新的 Access Token + 轮换 Refresh Token
/// 剩余寿命不足一半时重新签发 Refresh Token（滑动过期）；否则复用原 Refresh Token
/// 刷新令牌（Refresh Token 换新 Access Token + 轮换 Refresh Token）
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "刷新成功", body = LoginResponse),
        (status = 401, description = "令牌无效/过期"),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<LoginResponse>, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("Missing token".to_string()))?;

    // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
    let jwt = state.shared_jwt();
    // 仅接受 Refresh Token（Access Token 不可用于刷新）
    let claims = jwt.verify_refresh(token)?;
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    // 剩余寿命判定：<一半则轮换 Refresh Token（滑动过期）
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
        .as_secs() as usize;
    let remaining_hours = claims.exp.saturating_sub(now) / 3600;
    let new_refresh =
        if remaining_hours < (crate::utils::jwt::DEFAULT_REFRESH_TTL_HOURS / 2) as usize {
            jwt.sign_refresh(user.id)?
        } else {
            token.to_string()
        };
    let new_access = jwt.sign_access(user.id)?;

    Ok(Json(LoginResponse {
        token: new_access,
        refresh_token: new_refresh,
        username: user.username,
        role: user.role,
        must_change_password: user.must_change_password,
    }))
}

pub async fn change_password(
    State(state): State<AppState>,
    Extension(uid): Extension<UserId>,
    ApiJson(req): ApiJson<serde_json::Value>,
) -> Result<Json<()>, AppError> {
    let user = state
        .user_service
        .find_by_id(uid.0)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let old_pw = req
        .get("old_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing old_password".into()))?;
    if !PasswordUtils::verify(old_pw, &user.password_hash)? {
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }

    let new_pw = req
        .get("new_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing new_password".into()))?;
    let new_hash = PasswordUtils::hash(new_pw)?;
    state
        .user_service
        .update_password(user.id, &new_hash)
        .await?;

    // 密码修改事件
    let _ = state
        .event_bus
        .publish(crate::domain::entity::DomainEvent::PasswordChanged {
            username: user.username.clone(),
        })
        .await;

    Ok(Json(()))
}

/// 轮换 JWT 签名密钥（admin only）：更新后旧 access token 立即失效（短宽限期由客户端刷新换取新 token）
pub async fn rotate_secret(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_secret = req
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing new secret".into()))?;
    if new_secret.len() < crate::utils::jwt::MIN_SECRET_BYTES {
        return Err(AppError::BadRequest(format!(
            "new secret must be at least {} bytes",
            crate::utils::jwt::MIN_SECRET_BYTES
        )));
    }
    let mut guard = state
        .jwt_secret_store
        .write()
        .map_err(|_| AppError::internal("jwt secret store poisoned"))?;
    *guard = new_secret.to_string();
    drop(guard);
    // Stage 7（JWT 加固）：整体替换共享 JwtUtils 实例，使轮换立即对热路径生效
    {
        let mut jwt_guard = state
            .jwt_utils
            .write()
            .map_err(|_| AppError::internal("jwt utils store poisoned"))?;
        *jwt_guard = Arc::new(crate::utils::jwt::JwtUtils::new_pair(new_secret));
    }
    tracing::warn!("JWT secret rotated by admin (old tokens invalidated)");
    Ok(Json(serde_json::json!({
        "message": "JWT secret rotated; old access tokens are invalidated",
        "grace_period": "clients should re-login or use refresh token"
    })))
}

/// 当前登录用户信息（前端刷新页面恢复身份）
pub async fn me(
    State(state): State<AppState>,
    Extension(uid): Extension<UserId>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = state
        .user_service
        .find_by_id(uid.0)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;
    Ok(Json(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "role": user.role,
        "must_change_password": user.must_change_password,
    })))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", axum::routing::post(login))
        .route("/api/auth/refresh", axum::routing::post(refresh))
        .route("/api/auth/me", axum::routing::get(me))
        .route(
            "/api/auth/change-password",
            axum::routing::post(change_password),
        )
        .route(
            "/api/auth/rotate-secret",
            axum::routing::post(rotate_secret),
        )
}
