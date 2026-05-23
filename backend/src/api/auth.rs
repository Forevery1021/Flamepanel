use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::{AppState, AuthService};
use crate::core::error::AppError;
use crate::middleware::auth::{CurrentUser, RequireAdmin};

// ─── DTOs ─────────────────────────────────────────────────────────────────────

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
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ─── 公开路由 ─────────────────────────────────────────────────────────────────

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
}

// ─── 需认证路由 ───────────────────────────────────────────────────────────────

pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/change-password", post(change_password))
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(AppError::BadRequest("用户名和密码不能为空".into()));
    }

    let svc = AuthService::new(state.user_repo.clone());
    let (token, user) = svc.login(&payload.username, &payload.password).await?;

    tracing::info!("用户 '{}' 登录成功", user.username);

    Ok(Json(LoginResponse {
        token,
        username: user.username,
        role: user.role,
        expires_in: 7 * 24 * 3600,
    }))
}

async fn register(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let svc = AuthService::new(state.user_repo.clone());
    let role = payload.role.unwrap_or_else(|| "user".into());
    let user = svc.register(&payload.username, &payload.password, &role).await?;

    tracing::info!("管理员 '{}' 创建了用户 '{}'", claims.sub, user.username);

    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        role: user.role,
        created_at: user.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    }))
}

async fn change_password(
    State(state): State<AppState>,
    CurrentUser(claims): CurrentUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let svc = AuthService::new(state.user_repo.clone());

    let user = state.user_repo.find_by_username(&claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;

    svc.change_password(user.id, &payload.old_password, &payload.new_password).await?;

    tracing::info!("用户 '{}' 修改了密码", claims.sub);

    Ok(Json(MessageResponse {
        message: "密码修改成功".into(),
    }))
}

async fn logout(
    CurrentUser(claims): CurrentUser,
) -> Result<Json<MessageResponse>, AppError> {
    tracing::info!("用户 '{}' 已登出", claims.sub);
    Ok(Json(MessageResponse {
        message: "已成功登出".into(),
    }))
}

async fn me(
    State(state): State<AppState>,
    CurrentUser(claims): CurrentUser,
) -> Result<Json<UserResponse>, AppError> {
    let user = state.user_repo.find_by_username(&claims.sub)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        role: user.role,
        created_at: user.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    }))
}
