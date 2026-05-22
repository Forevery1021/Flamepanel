// src/api/auth.rs
//
// 认证相关 API：
//   POST /api/auth/login   — 用户名 + 密码登录，返回 JWT
//   POST /api/auth/logout  — 登出（客户端丢弃 Token 即可，服务端无状态）
//   GET  /api/auth/me      — 获取当前登录用户信息（需 Token）

use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
};
use bcrypt::verify;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    core::error::AppError,
    middleware::middleware_auth::{create_jwt, CurrentUser},
};

// ─── 请求 / 响应结构体 ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub expires_in: u64, // 秒数，方便前端刷新
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub username: String,
    pub issued_at: usize,
    pub expires_at: usize,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ─── 路由注册 ────────────────────────────────────────────────────────────────

pub fn routes() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

// ─── Handler：登录 ───────────────────────────────────────────────────────────

/// POST /api/auth/login
///
/// 流程：
///   1. 从 SQLite 查询 users 表，找到对应 username 的 bcrypt hash
///   2. bcrypt::verify 校验明文密码
///   3. 通过则签发 JWT，返回 token + 过期时间
async fn login(
    Extension(db): Extension<SqlitePool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // 参数基础校验
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(AppError::BadRequest("用户名和密码不能为空".into()));
    }

    // 查询用户（users 表结构见 migrations/）
    let row = sqlx::query!(
        "SELECT username, password_hash FROM users WHERE username = ? LIMIT 1",
        payload.username
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| AppError::Internal(format!("数据库查询失败: {e}")))?;

    let user = row.ok_or(AppError::Unauthorized)?;

    // bcrypt 密码校验（耗时操作，spawn_blocking 避免阻塞 tokio 线程）
    let hash = user.password_hash.clone();
    let plain = payload.password.clone();
    let valid = tokio::task::spawn_blocking(move || verify(&plain, &hash))
        .await
        .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
        .map_err(|_| AppError::Internal("密码校验失败".into()))?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    // 签发 JWT
    let expires_in: u64 = 7 * 24 * 3600; // 7 天
    let token = create_jwt(&user.username, expires_in)?;

    tracing::info!("用户 '{}' 登录成功", user.username);

    Ok(Json(LoginResponse {
        token,
        username: user.username,
        expires_in,
    }))
}

// ─── Handler：登出 ───────────────────────────────────────────────────────────

/// POST /api/auth/logout
///
/// JWT 无状态，服务端不维护黑名单（简化方案）。
/// 客户端收到响应后删除本地 Token 即完成登出。
/// 若后续需要服务端失效，可在此处将 jti 写入 Redis 黑名单。
async fn logout(
    CurrentUser(claims): CurrentUser,
) -> Result<Json<MessageResponse>, AppError> {
    tracing::info!("用户 '{}' 已登出", claims.sub);

    Ok(Json(MessageResponse {
        message: "已成功登出".into(),
    }))
}

// ─── Handler：获取当前用户 ───────────────────────────────────────────────────

/// GET /api/auth/me
///
/// 从请求头 Authorization: Bearer <token> 中解析 Claims，返回用户信息。
/// 依赖 `CurrentUser` 提取器（见 middleware/auth.rs）。
async fn me(
    CurrentUser(claims): CurrentUser,
) -> Result<Json<MeResponse>, AppError> {
    Ok(Json(MeResponse {
        username: claims.sub.clone(),
        issued_at: claims.iat,
        expires_at: claims.exp,
    }))
}