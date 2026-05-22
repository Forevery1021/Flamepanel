use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use crate::core::error::AppError;

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
    username: String,
    expires_in: i64,
}

pub fn routes() -> Router {
    Router::new().route("/login", post(login))
}

async fn login(
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // TODO: 查询数据库 + bcrypt 验证
    if req.username != "admin" {
        return Err(AppError::Unauthorized);
    }

    let token = crate::middleware::auth::create_jwt(&req.username)?;
    Ok(Json(LoginResponse {
        token,
        username: req.username,
        expires_in: 86400,
    }))
}
