use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::FirewallRule;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 防火墙规则（面板侧定义；`POST /api/firewall/apply` 才下发到系统防火墙）
#[derive(Serialize, ToSchema)]
pub struct FirewallRuleResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub protocol: String,
    pub port: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub action: String,
    pub enabled: bool,
    pub priority: i32,
    pub direction: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateFirewallRuleRequest {
    pub name: String,
    /// 缺省 `tcp`；`tcp` / `udp` / `both` / `icmp`
    pub description: Option<String>,
    pub protocol: Option<String>,
    pub port: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub direction: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateFirewallRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub protocol: Option<String>,
    pub port: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub direction: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ToggleRequest {
    pub enabled: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ReorderRequest {
    /// 规则 ID，按顺序赋优先级 10、20、30…（单事务）；未列出的规则保持原优先级，不存在的 ID 忽略
    pub ids: Vec<i64>,
}

fn to_response(rule: FirewallRule) -> FirewallRuleResponse {
    FirewallRuleResponse {
        id: rule.id,
        name: rule.name,
        description: rule.description,
        protocol: rule.protocol,
        port: rule.port,
        source: rule.source,
        destination: rule.destination,
        action: rule.action,
        enabled: rule.enabled,
        priority: rule.priority,
        direction: rule.direction,
        created_at: rule.created_at.to_rfc3339(),
        updated_at: rule.updated_at.to_rfc3339(),
    }
}

/// 防火墙规则列表（分页，按优先级）
#[utoipa::path(
    get,
    path = "/api/firewall/rules",
    tag = "firewall",
    params(PaginationParams),
    responses(
        (status = 200, description = "规则分页", body = PaginatedResponse<FirewallRuleResponse>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<FirewallRuleResponse>>, AppError> {
    let result = state.firewall_service.list_rules_paginated(&params).await?;
    let data = result.data.into_iter().map(to_response).collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

/// 单条防火墙规则
#[utoipa::path(
    get,
    path = "/api/firewall/rules/{id}",
    tag = "firewall",
    params(("id" = i64, Path, description = "规则 ID")),
    responses(
        (status = 200, description = "规则", body = FirewallRuleResponse),
        (status = 404, description = "规则不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let rule = state.firewall_service.get_rule(id).await?;
    Ok(Json(to_response(rule)))
}

/// 新建防火墙规则（仅落库，需 apply 才生效）
#[utoipa::path(
    post,
    path = "/api/firewall/rules",
    tag = "firewall",
    request_body = CreateFirewallRuleRequest,
    responses(
        (status = 200, description = "已创建", body = FirewallRuleResponse),
        (status = 400, description = "端口 / 地址 / 协议 / 方向校验失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateFirewallRuleRequest>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let rule = FirewallRule {
        id: 0,
        name: req.name,
        description: req.description,
        protocol: req.protocol.unwrap_or_else(|| "tcp".into()),
        port: req.port,
        source: Some(req.source.unwrap_or_else(|| "0.0.0.0/0".into())),
        destination: req.destination,
        action: req.action.unwrap_or_else(|| "allow".into()),
        enabled: req.enabled.unwrap_or(true),
        priority: req.priority.unwrap_or(50),
        direction: req.direction.unwrap_or_else(|| "in".into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let created = state.firewall_service.create_rule(rule).await?;
    Ok(Json(to_response(created)))
}

/// 更新防火墙规则（字段缺省即保持不变）
#[utoipa::path(
    put,
    path = "/api/firewall/rules/{id}",
    tag = "firewall",
    params(("id" = i64, Path, description = "规则 ID")),
    request_body = UpdateFirewallRuleRequest,
    responses(
        (status = 200, description = "已更新", body = FirewallRuleResponse),
        (status = 400, description = "端口 / 地址 / 协议 / 方向校验失败"),
        (status = 404, description = "规则不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateFirewallRuleRequest>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let mut rule = state.firewall_service.get_rule(id).await?;
    if let Some(name) = req.name {
        rule.name = name;
    }
    if let Some(desc) = req.description {
        rule.description = Some(desc);
    }
    if let Some(protocol) = req.protocol {
        rule.protocol = protocol;
    }
    if let Some(port) = req.port {
        rule.port = Some(port);
    }
    if let Some(source) = req.source {
        rule.source = Some(source);
    }
    if let Some(dest) = req.destination {
        rule.destination = Some(dest);
    }
    if let Some(action) = req.action {
        rule.action = action;
    }
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }
    if let Some(direction) = req.direction {
        rule.direction = direction;
    }
    rule.updated_at = Utc::now();
    let updated = state.firewall_service.update_rule(rule).await?;
    Ok(Json(to_response(updated)))
}

/// 删除防火墙规则
#[utoipa::path(
    delete,
    path = "/api/firewall/rules/{id}",
    tag = "firewall",
    params(("id" = i64, Path, description = "规则 ID")),
    responses(
        (status = 200, description = "`\"deleted\"`", body = String),
        (status = 404, description = "规则不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.delete_rule(id).await?;
    Ok(Json("deleted"))
}

/// 启用 / 停用单条规则
#[utoipa::path(
    post,
    path = "/api/firewall/rules/{id}/toggle",
    tag = "firewall",
    params(("id" = i64, Path, description = "规则 ID")),
    request_body = ToggleRequest,
    responses(
        (status = 200, description = "切换后的规则", body = FirewallRuleResponse),
        (status = 404, description = "规则不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:enable 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn toggle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<ToggleRequest>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let rule = state.firewall_service.toggle_rule(id, req.enabled).await?;
    Ok(Json(to_response(rule)))
}

/// 把全部已启用规则下发到系统防火墙（ufw / firewalld / iptables；面板非 root 时经特权助手）
#[utoipa::path(
    post,
    path = "/api/firewall/apply",
    tag = "firewall",
    responses(
        (status = 200, description = "`\"applied\"`", body = String),
        (status = 500, description = "防火墙后端执行失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:apply 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn apply_all(State(state): State<AppState>) -> Result<Json<&'static str>, AppError> {
    let rules = state.firewall_service.list_rules().await?;
    let rule_count = rules.len();
    state.firewall_service.apply_all_rules().await?;
    let _ = state
        .event_bus
        .publish(crate::domain::entity::DomainEvent::FirewallRulesApplied { rule_count })
        .await;
    Ok(Json("applied"))
}

/// 防火墙后端状态（检测到的后端、是否启用等键值）
#[utoipa::path(
    get,
    path = "/api/firewall/status",
    tag = "firewall",
    responses(
        (status = 200, description = "状态键值", body = std::collections::HashMap<String, String>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<std::collections::HashMap<String, String>>, AppError> {
    let status = state.firewall_service.get_backend_status().await?;
    Ok(Json(status))
}

/// 启用系统防火墙
#[utoipa::path(
    post,
    path = "/api/firewall/enable",
    tag = "firewall",
    responses(
        (status = 200, description = "`\"enabled\"`", body = String),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:enable 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn enable(State(state): State<AppState>) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.enable_firewall().await?;
    Ok(Json("enabled"))
}

/// 停用系统防火墙
#[utoipa::path(
    post,
    path = "/api/firewall/disable",
    tag = "firewall",
    responses(
        (status = 200, description = "`\"disabled\"`", body = String),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:enable 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn disable(State(state): State<AppState>) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.disable_firewall().await?;
    Ok(Json("disabled"))
}

/// 按给定顺序重排规则优先级
#[utoipa::path(
    post,
    path = "/api/firewall/reorder",
    tag = "firewall",
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "`\"reordered\"`", body = String),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 firewall:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn reorder(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ReorderRequest>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.reorder_rules(&req.ids).await?;
    Ok(Json("reordered"))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/firewall/rules", axum::routing::get(list))
        .route("/api/firewall/rules", axum::routing::post(create))
        .route("/api/firewall/rules/{id}", axum::routing::get(get))
        .route("/api/firewall/rules/{id}", axum::routing::put(update))
        .route("/api/firewall/rules/{id}", axum::routing::delete(delete))
        .route(
            "/api/firewall/rules/{id}/toggle",
            axum::routing::post(toggle),
        )
        .route("/api/firewall/apply", axum::routing::post(apply_all))
        .route("/api/firewall/status", axum::routing::get(get_status))
        .route("/api/firewall/enable", axum::routing::post(enable))
        .route("/api/firewall/disable", axum::routing::post(disable))
        .route("/api/firewall/reorder", axum::routing::post(reorder))
}
