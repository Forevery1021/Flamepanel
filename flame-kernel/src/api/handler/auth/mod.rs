use crate::api::extract::ApiJson;
use crate::api::types::{AppState, UserId};
use crate::core::error::AppError;
use crate::utils::jwt::JwtUtils;
use crate::utils::password::PasswordUtils;
use axum::Router;
use axum::{
    extract::{Extension, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub role: String,
    /// 是否需强制修改密码（新装面板首次登录）
    pub must_change_password: bool,
}

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

    let jwt = JwtUtils::new(&state.jwt_secret, 24);
    let token = jwt.sign(user.id)?;

    // 登录成功审计
    let _ = state
        .operation_log_service
        .log(&user.username, "LOGIN_SUCCESS", None, ip.as_deref())
        .await;

    Ok(Json(LoginResponse {
        token,
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

/// 刷新令牌（滑动过期）：剩余寿命不足 12h 时重置为 24h
pub async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<LoginResponse>, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("Missing token".to_string()))?;

    let jwt = JwtUtils::new(&state.jwt_secret, 24);
    let claims = jwt.verify(token)?;
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    // 剩余寿命判定：<12h 则重置
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
        .as_secs() as usize;
    let remaining_hours = claims.exp.saturating_sub(now) / 3600;
    let new_token = if remaining_hours < 12 {
        jwt.sign(user.id)?
    } else {
        token.to_string()
    };

    Ok(Json(LoginResponse {
        token: new_token,
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

    Ok(Json(()))
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
}
