//! WAF 基础防护 REST 端点（主线三 §6.2 · Doc/20）
//!
//! 规则 CRUD + 拦截日志查询/清理 + 引擎诊断。规则评估为面板进程内引擎，
//! 拦截日志只记来源 IP / 请求行摘要，不含请求头与 body（防敏感信息二次泄漏）。
use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::application::waf_service::WafService;
use crate::core::error::AppError;
use crate::domain::entity::{WafLog, WafRule};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateWafRuleRequest {
    pub name: String,
    /// 规则类型：cc | sql_injection | xss | ip_blacklist | ip_whitelist
    /// | ua_blacklist | ua_whitelist | path_block
    pub rule_type: String,
    /// 作用网站 ID（0 = 全局规则）
    #[serde(default)]
    pub website_id: i64,
    #[serde(default)]
    pub pattern: String,
    /// 类型相关参数 JSON（cc：`{"threshold":120,"window_secs":60}`）
    #[serde(default = "default_config_json")]
    pub config_json: String,
    /// 命中动作：log（默认，仅告警）| block
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_config_json() -> String {
    "{}".to_string()
}
fn default_action() -> String {
    "log".to_string()
}
fn default_enabled() -> bool {
    true
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateWafRuleRequest {
    pub name: Option<String>,
    pub rule_type: Option<String>,
    pub website_id: Option<i64>,
    pub pattern: Option<String>,
    pub config_json: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct ClearLogsRequest {
    /// 0 或缺省 = 清空全部
    #[serde(default)]
    pub website_id: i64,
}

/// WAF 拦截日志查询参数（website_id = 0 不过滤）
#[derive(serde::Deserialize, ToSchema, utoipa::IntoParams)]
pub struct WafLogQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    /// 过滤网站 ID（0 = 全部）
    #[serde(default)]
    pub website_id: i64,
}

/// WAF 引擎诊断信息
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct WafEngineStatus {
    /// 引擎内启用规则数（规则变更即时重建生效）
    pub loaded_rules: usize,
    /// 是否已配置 Web 引擎下发（Nginx / OpenResty 原生配置）
    pub enforcement: bool,
    /// Web 引擎命中日志回灌状态（按引擎；未安装的引擎不出现）
    pub ingest: Vec<crate::application::waf_ports::WafIngestStatus>,
}

fn rule_response(r: &WafRule) -> WafRule {
    r.clone()
}

/// WAF 规则列表（website_id 查询参数过滤，0 = 全部）
#[utoipa::path(
    get,
    path = "/api/waf/rules",
    tag = "waf",
    params(("website_id" = Option<i64>, Query, description = "网站 ID 过滤（0 = 全部）")),
    responses(
        (status = 200, description = "规则列表", body = Vec<WafRule>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_rules(
    State(state): State<AppState>,
    Query(q): Query<WafWebsiteQuery>,
) -> Result<Json<Vec<WafRule>>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    let wid = (q.website_id > 0).then_some(q.website_id);
    Ok(Json(svc.list_rules(wid).await?))
}

#[derive(serde::Deserialize, ToSchema)]
pub struct WafWebsiteQuery {
    #[serde(default)]
    pub website_id: i64,
}

/// 创建 WAF 规则
#[utoipa::path(
    post,
    path = "/api/waf/rules",
    tag = "waf",
    request_body = CreateWafRuleRequest,
    responses(
        (status = 200, description = "创建成功", body = WafRule),
        (status = 400, description = "参数错误"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_rule(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateWafRuleRequest>,
) -> Result<Json<WafRule>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    let rule = svc
        .create_rule(WafRule {
            id: 0,
            name: req.name,
            rule_type: req.rule_type,
            website_id: req.website_id,
            pattern: req.pattern,
            config_json: req.config_json,
            action: req.action,
            enabled: req.enabled,
            ..Default::default()
        })
        .await?;
    Ok(Json(rule_response(&rule)))
}

/// 获取 WAF 规则详情
#[utoipa::path(
    get,
    path = "/api/waf/rules/{id}",
    tag = "waf",
    params(("id" = i64, Path, description = "规则 ID")),
    responses(
        (status = 200, description = "规则详情", body = WafRule),
        (status = 404, description = "规则不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<WafRule>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    Ok(Json(svc.get_rule(id).await?))
}

/// 更新 WAF 规则
#[utoipa::path(
    put,
    path = "/api/waf/rules/{id}",
    tag = "waf",
    params(("id" = i64, Path, description = "规则 ID")),
    request_body = UpdateWafRuleRequest,
    responses(
        (status = 200, description = "更新成功", body = WafRule),
        (status = 404, description = "规则不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateWafRuleRequest>,
) -> Result<Json<WafRule>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    let mut rule = svc.get_rule(id).await?;
    if let Some(v) = req.name {
        rule.name = v;
    }
    if let Some(v) = req.rule_type {
        rule.rule_type = v;
    }
    if let Some(v) = req.website_id {
        rule.website_id = v;
    }
    if let Some(v) = req.pattern {
        rule.pattern = v;
    }
    if let Some(v) = req.config_json {
        rule.config_json = v;
    }
    if let Some(v) = req.action {
        rule.action = v;
    }
    if let Some(v) = req.enabled {
        rule.enabled = v;
    }
    let updated = svc.update_rule(&rule).await?;
    Ok(Json(rule_response(&updated)))
}

/// 删除 WAF 规则（拦截日志保留，规则名快照仍可读）
#[utoipa::path(
    delete,
    path = "/api/waf/rules/{id}",
    tag = "waf",
    params(("id" = i64, Path, description = "规则 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "规则不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<()>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    svc.delete_rule(id).await?;
    Ok(Json(()))
}

/// WAF 拦截日志（分页，按时间倒序）
#[utoipa::path(
    get,
    path = "/api/waf/logs",
    tag = "waf",
    params(WafLogQuery),
    responses(
        (status = 200, description = "拦截日志分页", body = PaginatedResponse<WafLog>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_logs(
    State(state): State<AppState>,
    Query(q): Query<WafLogQuery>,
) -> Result<Json<PaginatedResponse<WafLog>>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    let wid = (q.website_id > 0).then_some(q.website_id);
    let (data, total) = svc
        .list_logs_paginated(q.pagination.page(), q.pagination.page_size(), wid)
        .await?;
    Ok(Json(PaginatedResponse::new(data, total, &q.pagination)))
}

/// 清理 WAF 拦截日志（website_id = 0 清空全部）
#[utoipa::path(
    post,
    path = "/api/waf/logs/clear",
    tag = "waf",
    request_body = ClearLogsRequest,
    responses(
        (status = 200, description = "清理条数", body = i64),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn clear_logs(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ClearLogsRequest>,
) -> Result<Json<i64>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    let wid = (req.website_id > 0).then_some(req.website_id);
    let cleared = svc.clear_logs(wid).await?;
    Ok(Json(cleared as i64))
}

/// WAF 引擎诊断（启用规则数 / 是否配置 Web 引擎下发 / 命中日志回灌状态）
#[utoipa::path(
    get,
    path = "/api/waf/status",
    tag = "waf",
    responses(
        (status = 200, description = "引擎状态", body = WafEngineStatus),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn status(State(state): State<AppState>) -> Result<Json<WafEngineStatus>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    Ok(Json(WafEngineStatus {
        loaded_rules: svc.engine_rule_count().await,
        enforcement: svc.enforcement_configured(),
        ingest: svc.ingest_status(),
    }))
}

/// 下发 WAF 规则到本机 Web 引擎（Nginx / OpenResty 原生配置；Doc/32 W3）
///
/// 规则变更时会自动尽力下发；本端点用于手动重试与查看逐引擎结果（校验失败时已回滚）。
/// 未配置下发能力返回 503。
#[utoipa::path(
    post,
    path = "/api/waf/apply",
    tag = "waf",
    responses(
        (status = 200, description = "逐引擎下发报告", body = crate::application::waf_ports::WafApplyReport),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 503, description = "未配置 Web 引擎下发"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn apply_rules(
    State(state): State<AppState>,
) -> Result<Json<crate::application::waf_ports::WafApplyReport>, AppError> {
    let svc: &WafService = &state.services.waf_service;
    Ok(Json(svc.apply().await?))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/waf/rules", get(list_rules).post(create_rule))
        .route(
            "/api/waf/rules/{id}",
            get(get_rule).put(update_rule).delete(delete_rule),
        )
        .route("/api/waf/logs", get(list_logs))
        .route("/api/waf/logs/clear", post(clear_logs))
        .route("/api/waf/status", get(status))
        .route("/api/waf/apply", post(apply_rules))
}
