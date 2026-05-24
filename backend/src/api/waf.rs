use axum::{
    extract::{Query, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::{AppState, WafService};
use crate::core::error::AppError;
use crate::domain::{CreateWafRuleRequest, UpdateWafRuleRequest, WafRule, WafIpRule, CreateWafIpRuleRequest};
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

#[derive(Debug, Deserialize)]
pub struct TestRuleRequest {
    pub pattern: String,
    pub target: String,
}

#[derive(Debug, Serialize)]
pub struct TestRuleResponse {
    pub matches: bool,
    pub captures: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WafStats {
    pub total_rules: i64,
    pub enabled_rules: i64,
    pub total_ip_rules: i64,
    pub enabled_ip_rules: i64,
    pub block_ip_rules: i64,
    pub allow_ip_rules: i64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/rules", get(list_rules))
        .route("/rules/get", get(get_rule))
        .route("/rules/create", post(create_rule))
        .route("/rules/update", put(update_rule))
        .route("/rules/delete", delete(delete_rule))
        .route("/rules/toggle", post(toggle_rule))
        .route("/rules/test", post(test_rule))
        .route("/ip-rules", get(list_ip_rules))
        .route("/ip-rules/create", post(create_ip_rule))
        .route("/ip-rules/toggle", post(toggle_ip_rule))
        .route("/ip-rules/delete", delete(delete_ip_rule))
        .route("/stats", get(waf_stats))
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

// ─── IP Rule Handlers ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct IpRuleIdQuery {
    pub id: i64,
}

async fn list_ip_rules(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<Vec<WafIpRule>>, AppError> {
    let rules = state.waf_ip_repo.list_all().await?;
    Ok(Json(rules))
}

async fn create_ip_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<CreateWafIpRuleRequest>,
) -> Result<Json<WafIpRule>, AppError> {
    if payload.ip.is_empty() {
        return Err(AppError::BadRequest("IP 不能为空".into()));
    }
    if payload.action != "allow" && payload.action != "block" {
        return Err(AppError::BadRequest("action 必须为 allow 或 block".into()));
    }

    let rule = state.waf_ip_repo.create(&payload).await?;

    tracing::info!("用户 '{}' 创建了 IP 规则: {} -> {}", _claims.sub, rule.ip, rule.action);

    Ok(Json(rule))
}

async fn toggle_ip_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<ToggleRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    state.waf_ip_repo.update(payload.id, payload.enabled, None).await?;

    tracing::info!("用户 '{}' 将 IP 规则 id={} enabled={}", _claims.sub, payload.id, payload.enabled);

    Ok(Json(MessageResponse {
        success: true,
        message: format!("IP 规则已{}", if payload.enabled { "启用" } else { "禁用" }),
    }))
}

async fn delete_ip_rule(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<IpRuleIdQuery>,
) -> Result<Json<MessageResponse>, AppError> {
    state.waf_ip_repo.delete(query.id).await?;

    tracing::info!("用户 '{}' 删除了 IP 规则 id={}", _claims.sub, query.id);

    Ok(Json(MessageResponse {
        success: true,
        message: "IP 规则删除成功".into(),
    }))
}

/// POST /api/waf/rules/test
/// Test a regex pattern against sample input text
async fn test_rule(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<TestRuleRequest>,
) -> Result<Json<TestRuleResponse>, AppError> {
    match regex::Regex::new(&payload.pattern) {
        Ok(re) => {
            let mut captures = Vec::new();
            for cap in re.captures_iter(&payload.target) {
                for (i, m) in cap.iter().enumerate() {
                    if let Some(m) = m {
                        captures.push(format!("${}={}", i, m.as_str()));
                    }
                }
            }
            let matched = re.is_match(&payload.target);
            Ok(Json(TestRuleResponse {
                matches: matched,
                captures,
                error: None,
            }))
        }
        Err(e) => Ok(Json(TestRuleResponse {
            matches: false,
            captures: vec![],
            error: Some(format!("正则表达式错误: {e}")),
        })),
    }
}

/// GET /api/waf/stats
/// Get WAF statistics summary
async fn waf_stats(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<WafStats>, AppError> {
    let (total, enabled) = state.waf_repo.count().await.unwrap_or((0, 0));
    let ip_rules = state.waf_ip_repo.list_all().await.unwrap_or_default();
    let total_ip = ip_rules.len() as i64;
    let enabled_ip = ip_rules.iter().filter(|r| r.enabled).count() as i64;
    let block_count = ip_rules.iter().filter(|r| r.action == "block").count() as i64;
    let allow_count = ip_rules.iter().filter(|r| r.action == "allow").count() as i64;

    Ok(Json(WafStats {
        total_rules: total,
        enabled_rules: enabled,
        total_ip_rules: total_ip,
        enabled_ip_rules: enabled_ip,
        block_ip_rules: block_count,
        allow_ip_rules: allow_count,
    }))
}
