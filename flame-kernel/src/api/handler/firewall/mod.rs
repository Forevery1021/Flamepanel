use axum::{Json, extract::{State, Path, Query}};
use serde::{Serialize, Deserialize};
use crate::api::types::{AppState, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;
use crate::domain::entity::FirewallRule;
use chrono::Utc;

#[derive(Serialize)]
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

#[derive(Deserialize)]
pub struct CreateFirewallRuleRequest {
    pub name: String,
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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct ToggleRequest {
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct ReorderRequest {
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

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<FirewallRuleResponse>>, AppError> {
    let result = state.firewall_service.list_rules_paginated(&params).await?;
    let data = result.data.into_iter().map(to_response).collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let rule = state.firewall_service.get_rule(id).await?;
    Ok(Json(to_response(rule)))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateFirewallRuleRequest>,
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

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateFirewallRuleRequest>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let mut rule = state.firewall_service.get_rule(id).await?;
    if let Some(name) = req.name { rule.name = name; }
    if let Some(desc) = req.description { rule.description = Some(desc); }
    if let Some(protocol) = req.protocol { rule.protocol = protocol; }
    if let Some(port) = req.port { rule.port = Some(port); }
    if let Some(source) = req.source { rule.source = Some(source); }
    if let Some(dest) = req.destination { rule.destination = Some(dest); }
    if let Some(action) = req.action { rule.action = action; }
    if let Some(enabled) = req.enabled { rule.enabled = enabled; }
    if let Some(priority) = req.priority { rule.priority = priority; }
    if let Some(direction) = req.direction { rule.direction = direction; }
    rule.updated_at = Utc::now();
    let updated = state.firewall_service.update_rule(rule).await?;
    Ok(Json(to_response(updated)))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.delete_rule(id).await?;
    Ok(Json("deleted"))
}

pub async fn toggle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ToggleRequest>,
) -> Result<Json<FirewallRuleResponse>, AppError> {
    let rule = state.firewall_service.toggle_rule(id, req.enabled).await?;
    Ok(Json(to_response(rule)))
}

pub async fn apply_all(
    State(state): State<AppState>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.apply_all_rules().await?;
    Ok(Json("applied"))
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<std::collections::HashMap<String, String>>, AppError> {
    let status = state.firewall_service.get_backend_status().await?;
    Ok(Json(status))
}

pub async fn enable(
    State(state): State<AppState>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.enable_firewall().await?;
    Ok(Json("enabled"))
}

pub async fn disable(
    State(state): State<AppState>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.disable_firewall().await?;
    Ok(Json("disabled"))
}

pub async fn reorder(
    State(state): State<AppState>,
    Json(req): Json<ReorderRequest>,
) -> Result<Json<&'static str>, AppError> {
    state.firewall_service.reorder_rules(&req.ids).await?;
    Ok(Json("reordered"))
}
