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
#[utoipa::path(
    get,
    path = "/api/setup/status",
    tag = "setup",
    responses(
        (status = 200, description = "初始化状态与环境探测", body = SetupStatusResponse),
    )
)]
pub async fn status(State(state): State<AppState>) -> Result<Json<SetupStatusResponse>, AppError> {
    let resp = state.setup_service.status().await?;
    Ok(Json(resp))
}

/// `POST /api/setup/initialize`
#[utoipa::path(
    post,
    path = "/api/setup/initialize",
    tag = "setup",
    request_body = SetupRequest,
    responses(
        (status = 200, description = "步骤完成；`admin` 步骤返回登录令牌", body = InitializeResponse),
        (status = 400, description = "参数错误"),
        (status = 409, description = "面板已初始化"),
    )
)]
pub async fn initialize(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ApiJson(req): ApiJson<SetupRequest>,
) -> Result<(axum::http::HeaderMap, Json<InitializeResponse>), AppError> {
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
    let mut resp = state
        .setup_service
        .initialize(&req, &jwt, &access_domain)
        .await?;
    // Y1：refresh token 经 httpOnly Cookie 下发，不写入 JSON 响应体。
    let mut resp_headers = axum::http::HeaderMap::new();
    if let Some(refresh_token) = resp.refresh_token.take() {
        resp_headers.insert(
            axum::http::header::SET_COOKIE,
            axum::http::HeaderValue::from_str(&crate::api::handler::auth::set_refresh_cookie(
                &refresh_token,
                crate::utils::jwt::DEFAULT_REFRESH_TTL_HOURS * 3600,
                crate::api::handler::auth::request_is_https(&headers),
            ))
            .map_err(|_| AppError::internal("invalid refresh cookie"))?,
        );
    }
    Ok((resp_headers, Json(resp)))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/setup/status", get(status))
        .route("/api/setup/initialize", post(initialize))
}
