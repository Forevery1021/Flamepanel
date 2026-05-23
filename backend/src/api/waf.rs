use axum::{
    extract::{Query, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::{AppState, WafService};
use crate::core::error::AppError;
use crate::domain::{CreateWafRuleRequest, UpdateWafRuleRequest, WafRule};
use crate::middleware::auth::CurrentUser;

#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    pub id: i64,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/rules", get(list_rules))
        .route("/rules/get", get(get_rule))
        .route("/rules/create", post(create_rule))
        .route("/rules/update", put(update_rule))
        .route("/rules/delete", delete(delete_rule))
        .route("/rules/toggle", post(toggle_rule))
}

async fn list_rules(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<Vec<WafRule>>, AppError> {
    let service = WafService::new(state.waf_repo.clone());
    let rules = service.list_rules().await?;
    Ok(Json(rules))
}

async fn get_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<IdQuery>,
) -> Result<Json<WafRule>, AppError> {
    let service = WafService::new(state.waf_repo.clone());
    let rule = service.get_rule(query.id).await?;
    Ok(Json(rule))
}

async fn create_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<CreateWafRuleRequest>,
) -> Result<Json<WafRule>, AppError> {
    let service = WafService::new(state.waf_repo.clone());
    let rule = service.create_rule(payload).await?;

    tracing::info!("用户 '{}' 创建了 WAF 规则 '{}'", _claims.sub, rule.name);

    Ok(Json(rule))
}

async fn update_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<MessageResponse>, AppError> {
    let id = payload["id"].as_i64().ok_or(AppError::BadRequest("id 必填".into()))?;

    let req = UpdateWafRuleRequest {
        name: payload["name"].as_str().map(|s| s.to_string()),
        pattern: payload["pattern"].as_str().map(|s| s.to_string()),
        target: payload["target"].as_str().map(|s| s.to_string()),
        action: payload["action"].as_str().map(|s| s.to_string()),
        description: payload["description"].as_str().map(|s| s.to_string()),
        enabled: payload["enabled"].as_bool(),
    };

    let service = WafService::new(state.waf_repo.clone());
    service.update_rule(id, req).await?;

    tracing::info!("用户 '{}' 更新了 WAF 规则 id={}", _claims.sub, id);

    Ok(Json(MessageResponse {
        success: true,
        message: "规则更新成功".into(),
    }))
}

async fn delete_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<IdQuery>,
) -> Result<Json<MessageResponse>, AppError> {
    let service = WafService::new(state.waf_repo.clone());
    service.delete_rule(query.id).await?;

    tracing::info!("用户 '{}' 删除了 WAF 规则 id={}", _claims.sub, query.id);

    Ok(Json(MessageResponse {
        success: true,
        message: "规则删除成功".into(),
    }))
}

async fn toggle_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<ToggleRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let service = WafService::new(state.waf_repo.clone());
    service.toggle_rule(payload.id, payload.enabled).await?;

    tracing::info!(
        "用户 '{}' 将 WAF 规则 id={} enabled={}",
        _claims.sub, payload.id, payload.enabled
    );

    Ok(Json(MessageResponse {
        success: true,
        message: format!("规则已{}", if payload.enabled { "启用" } else { "禁用" }),
    }))
}
