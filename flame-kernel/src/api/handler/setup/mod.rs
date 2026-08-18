//! 首次部署 Setup 向导 API（B1/B2）。
//!
//! - `GET  /api/setup/status`      —— 初始化状态（公开，Health 档限流）
//! - `POST /api/setup/initialize`  —— 两阶段初始化（公开，Login 档限流）
//!
//! 两路由均不进 RBAC 权限表（初始化完成前不存在可认证用户）。

use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::setup_service::{InitializeResponse, SetupRequest, SetupStatusResponse};
use crate::core::error::AppError;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};

/// `GET /api/setup/status`
pub async fn status(State(state): State<AppState>) -> Result<Json<SetupStatusResponse>, AppError> {
    let resp = state.setup_service.status().await?;
    Ok(Json(resp))
}

/// `POST /api/setup/initialize`
pub async fn initialize(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ApiJson(req): ApiJson<SetupRequest>,
) -> Result<Json<InitializeResponse>, AppError> {
    // 以访问域名（Host 头，去端口/括号）作为自签证书 SAN 的默认域名
    let access_domain = headers
        .get("Host")
        .and_then(|v| v.to_str().ok())
        .map(|h| {
            let host = h
                .trim()
                .trim_start_matches('[')
                .split(':')
                .next()
                .unwrap_or(h);
            host.to_string()
        })
        .unwrap_or_else(|| "localhost".into());
    let jwt = state.shared_jwt();
    let resp = state
        .setup_service
        .initialize(&req, &jwt, &access_domain)
        .await?;
    Ok(Json(resp))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/setup/status", get(status))
        .route("/api/setup/initialize", post(initialize))
}
