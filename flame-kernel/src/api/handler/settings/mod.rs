use crate::api::access_guard::request_origin;
use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::kernel_log::LogLevel;
use crate::domain::settings_schema::{self, AccessDecision, PanelPolicy};
use axum::http::HeaderMap;
use axum::Router;
use axum::{
    extract::{ConnectInfo, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSettingRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSettingsBatchRequest {
    /// 批量设置项（key -> value），在一次事务内原子写入。
    pub settings: Vec<(String, String)>,
}

/// 设置列表（分页）
#[utoipa::path(
    get,
    path = "/api/settings",
    tag = "settings",
    params(PaginationParams),
    responses(
        (status = 200, description = "设置列表", body = PaginatedResponse<SettingEntry>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_settings(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<SettingEntry>>, AppError> {
    let result = state.settings_service.list_all_paginated(&params).await?;
    // 保留键（签名密钥 / 内部游标）不经通用接口暴露
    let data = result
        .data
        .into_iter()
        .filter(|s| !settings_schema::is_reserved(&s.key))
        .map(|s| SettingEntry {
            key: s.key,
            value: s.value,
            description: s.description,
        })
        .collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

/// 读取单个设置项
#[utoipa::path(
    get,
    path = "/api/settings/{key}",
    tag = "settings",
    params(("key" = String, Path, description = "设置键")),
    responses(
        (status = 200, description = "设置项", body = SettingEntry),
        (status = 401, description = "未认证"),
        (status = 404, description = "设置项不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_setting(
    State(state): State<AppState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<Json<SettingEntry>, AppError> {
    if settings_schema::is_reserved(&key) {
        return Err(AppError::NotFound(format!("Setting '{}' not found", key)));
    }
    let all = state.settings_service.list_all().await?;
    let setting = all
        .into_iter()
        .find(|s| s.key == key)
        .ok_or_else(|| AppError::NotFound(format!("Setting '{}' not found", key)))?;
    Ok(Json(SettingEntry {
        key: setting.key,
        value: setting.value,
        description: setting.description,
    }))
}

/// 更新设置
#[utoipa::path(
    put,
    path = "/api/settings",
    tag = "settings",
    request_body = UpdateSettingRequest,
    responses(
        (status = 200, description = "更新成功"),
        (status = 400, description = "值不合法 / 保留键 / 保存后会锁定当前访问来源"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_setting(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<UpdateSettingRequest>,
) -> Result<Json<()>, AppError> {
    let entries = vec![(req.key, req.value)];
    let entries = prepare_write(&state, &peer, &headers, &entries).await?;
    state.settings_service.set_many(&entries).await?;
    apply_side_effects(&state, &entries);
    Ok(Json(()))
}

/// 批量更新设置（一次事务原子写入多键）
#[utoipa::path(
    patch,
    path = "/api/settings/batch",
    tag = "settings",
    request_body = UpdateSettingsBatchRequest,
    responses(
        (status = 200, description = "批量更新成功"),
        (status = 400, description = "请求体为空 / 值不合法（一次列出全部错误）/ 保存后会锁定当前访问来源"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_settings_batch(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<UpdateSettingsBatchRequest>,
) -> Result<Json<()>, AppError> {
    if req.settings.is_empty() {
        return Err(AppError::BadRequest("settings must not be empty".into()));
    }
    let entries = prepare_write(&state, &peer, &headers, &req.settings).await?;
    state.settings_service.set_many(&entries).await?;
    apply_side_effects(&state, &entries);
    Ok(Json(()))
}

/// 写入前：类型校验 + 规范化（一次列出全部错误）；涉及访问控制时防止把自己锁在门外。
async fn prepare_write(
    state: &AppState,
    peer: &SocketAddr,
    headers: &HeaderMap,
    entries: &[(String, String)],
) -> Result<Vec<(String, String)>, AppError> {
    let entries = settings_schema::validate_all(entries).map_err(AppError::BadRequest)?;
    if entries
        .iter()
        .any(|(k, _)| k == "allowed_ips" || k == "bind_domains")
    {
        let mut map = state.settings_service.get_all_map().await?;
        for (k, v) in &entries {
            map.insert(k.clone(), v.clone());
        }
        let next = PanelPolicy::from_map(&map);
        let origin = request_origin(peer, headers);
        match next.check(origin.ip, origin.host.as_deref(), origin.local) {
            AccessDecision::Allow => {}
            AccessDecision::DenyIp => {
                return Err(AppError::BadRequest(format!(
                    "allowed_ips: saving would lock out your current IP {}; add it to the list first",
                    origin
                        .ip
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| "unknown".into())
                )))
            }
            AccessDecision::DenyHost => {
                return Err(AppError::BadRequest(format!(
                    "bind_domains: saving would lock out the domain you are using ('{}'); add it to the list first",
                    origin.host.as_deref().unwrap_or("")
                )))
            }
        }
    }
    Ok(entries)
}

/// 写入后立即生效的设置（其余设置由各消费方按需读取）。
fn apply_side_effects(state: &AppState, entries: &[(String, String)]) {
    for (k, v) in entries {
        if k == "log_level" {
            if let Some(level) = LogLevel::parse(v) {
                if let Err(e) = state.kernel_log_service.set_level(level) {
                    tracing::debug!(error = %e, "kernel log level not applied (logging not initialized)");
                }
            }
        }
    }
}

/// 当前请求的访问来源与生效中的访问控制策略（设置页据此提示「你的 IP」，避免误锁）
#[derive(Debug, Serialize, ToSchema)]
pub struct AccessInfo {
    /// 面板看到的客户端 IP（经本机反代时取转发头中的真实客户端）
    pub client_ip: Option<String>,
    /// 访问所用域名 / 主机（含端口时已去除）
    pub host: Option<String>,
    /// 是否本机直连（回环且未经反代）；本机直连不受授权 IP / 绑定域名限制
    pub local: bool,
    pub allowed_ips: Vec<String>,
    pub bind_domains: Vec<String>,
    pub session_timeout_minutes: u64,
    pub login_max_failures: u32,
    pub login_lockout_minutes: u64,
}

/// 访问来源与访问控制策略
#[utoipa::path(
    get,
    path = "/api/settings/access",
    tag = "settings",
    responses(
        (status = 200, description = "当前访问来源与生效策略", body = AccessInfo),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn access_info(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<AccessInfo>, AppError> {
    let origin = request_origin(&peer, &headers);
    let policy = state.settings_service.policy();
    Ok(Json(AccessInfo {
        client_ip: origin.ip.map(|i| i.to_string()),
        host: origin.host.as_deref().map(settings_schema::strip_port),
        local: origin.local,
        allowed_ips: policy.allowed_ips.iter().map(|r| r.to_string()).collect(),
        bind_domains: policy.bind_domains.clone(),
        session_timeout_minutes: policy.session_timeout_minutes,
        login_max_failures: policy.login_max_failures,
        login_lockout_minutes: policy.login_lockout_minutes,
    }))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings", axum::routing::get(list_settings))
        .route("/api/settings/access", axum::routing::get(access_info))
        .route("/api/settings/{key}", axum::routing::get(get_setting))
        .route("/api/settings", axum::routing::put(update_setting))
        .route(
            "/api/settings/batch",
            axum::routing::patch(update_settings_batch),
        )
}
