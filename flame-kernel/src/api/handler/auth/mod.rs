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
}

pub async fn login(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = state
        .user_service
        .find_by_username(&req.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    if !PasswordUtils::verify(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }

    let jwt = JwtUtils::new(&state.jwt_secret, 24);
    let token = jwt.sign(user.id)?;

    Ok(Json(LoginResponse {
        token,
        username: user.username,
        role: user.role,
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

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", axum::routing::post(login))
        .route(
            "/api/auth/change-password",
            axum::routing::post(change_password),
        )
}
