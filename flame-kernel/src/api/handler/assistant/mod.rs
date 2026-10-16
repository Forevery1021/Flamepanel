//! 面板内置 AI 助手（M-AI-3 · Doc/30 / Doc/32 §7）。
//!
//! | 端点 | 权限 | 说明 |
//! |---|---|---|
//! | `GET /api/assistant/config` | `mcp:read` | 助手配置（不含密钥，只返回 `has_api_key`） |
//! | `PUT /api/assistant/config` | `settings:update` | 更新配置（API Key 加密落库；默认关闭） |
//! | `POST /api/assistant/test` | `settings:update` | 按已保存配置列出模型，验证连通性 |
//! | `POST /api/assistant/chat` | `mcp:create` | 一轮对话（非流式）：模型 ⇄ 工具，返回回复与工具轨迹 |
//!
//! **权限模型**：对话以当前用户身份运行——可见工具按其角色过滤，写工具只生成审批单
//! （挂在该用户的内置凭证 `panel-assistant` 名下），与外部 MCP 客户端走同一闸门。
//! 同一用户同时只允许一轮对话在进行（429）。
pub mod agent;
pub mod nl2api;

use crate::api::app_state::{UserId, UserRole, Username};
use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::assistant::{AssistantConfigPatch, AssistantConfigView};
use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use utoipa::ToSchema;

/// 客户端提交的一条历史消息（只允许 user / assistant）。
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ChatTurn {
    /// `user` 或 `assistant`
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChatRequest {
    /// 对话历史（≤ 40 条，末条为用户消息）
    pub messages: Vec<ChatTurn>,
    /// 会话 ID（首轮留空，由服务端生成；后续轮次回传以便审计归组）
    #[serde(default)]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatResponse {
    pub conversation_id: String,
    /// 助手回复（Markdown 文本）
    pub reply: String,
    /// 本轮工具调用轨迹
    pub steps: Vec<agent::ToolStep>,
    /// 是否因达到步数上限而强制收尾
    pub step_limit_reached: bool,
    /// 本轮生成的审批单 ID（便于前端直达审批页）
    pub approval_ids: Vec<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestResult {
    pub ok: bool,
    /// 服务端返回的模型列表（≤ 200）
    pub models: Vec<String>,
    /// 已配置的模型是否在列表中（列表为空时为 null）
    pub model_found: Option<bool>,
}

/// 每用户单并发守卫（RAII）。
struct InFlight(i64);

fn in_flight() -> &'static Mutex<HashSet<i64>> {
    static SET: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

impl InFlight {
    fn acquire(user_id: i64) -> Result<Self, AppError> {
        let mut g = in_flight().lock().unwrap_or_else(|e| e.into_inner());
        if !g.insert(user_id) {
            return Err(AppError::RateLimited("上一轮对话尚未结束，请稍候".into()));
        }
        Ok(Self(user_id))
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        in_flight()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.0);
    }
}

/// 会话 ID：沿用客户端回传的合法 UUID，否则新生成。
fn conversation_id(raw: Option<&str>) -> String {
    raw.and_then(|s| uuid::Uuid::parse_str(s.trim()).ok())
        .unwrap_or_else(uuid::Uuid::new_v4)
        .to_string()
}

/// `GET /api/assistant/config`
#[utoipa::path(
    get,
    path = "/api/assistant/config",
    tag = "assistant",
    responses(
        (status = 200, description = "助手配置（不含密钥）", body = AssistantConfigView),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<AssistantConfigView>, AppError> {
    Ok(Json(state.assistant_service.view().await?))
}

/// `PUT /api/assistant/config`
#[utoipa::path(
    put,
    path = "/api/assistant/config",
    tag = "assistant",
    request_body = AssistantConfigPatch,
    responses(
        (status = 200, description = "更新后的配置", body = AssistantConfigView),
        (status = 400, description = "字段不合法（如启用但未填模型、base_url 内嵌凭据）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 settings:update 权限"),
        (status = 503, description = "主密钥不可用，拒绝明文保存 API Key"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_config(
    State(state): State<AppState>,
    Extension(Username(username)): Extension<Username>,
    ApiJson(patch): ApiJson<AssistantConfigPatch>,
) -> Result<Json<AssistantConfigView>, AppError> {
    let key_touched = patch.api_key.is_some();
    let v = state.assistant_service.update(patch).await?;
    tracing::info!(
        user = %username,
        enabled = v.enabled,
        provider = ?v.provider,
        model = %v.model,
        api_key_changed = key_touched,
        "assistant config updated"
    );
    Ok(Json(v))
}

/// `POST /api/assistant/test`
#[utoipa::path(
    post,
    path = "/api/assistant/test",
    tag = "assistant",
    responses(
        (status = 200, description = "连通性测试结果", body = TestResult),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 settings:update 权限"),
        (status = 503, description = "无法连接模型服务 / 鉴权失败"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn test_connection(State(state): State<AppState>) -> Result<Json<TestResult>, AppError> {
    let models = state.assistant_service.test_connection().await?;
    let model = state.assistant_service.config().await?.model;
    let model_found = if models.is_empty() || model.is_empty() {
        None
    } else {
        Some(models.iter().any(|m| m == &model))
    };
    Ok(Json(TestResult {
        ok: true,
        models,
        model_found,
    }))
}

/// `POST /api/assistant/chat`
#[utoipa::path(
    post,
    path = "/api/assistant/chat",
    tag = "assistant",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "助手回复与工具轨迹", body = ChatResponse),
        (status = 400, description = "消息不合法（角色、长度、条数）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:create 权限"),
        (status = 429, description = "上一轮对话尚未结束"),
        (status = 503, description = "助手未启用 / 模型服务不可用"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn chat(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    Extension(Username(username)): Extension<Username>,
    Extension(UserRole(role)): Extension<UserRole>,
    ApiJson(req): ApiJson<ChatRequest>,
) -> Result<Json<ChatResponse>, AppError> {
    let history = agent::parse_history(&req.messages)?;
    let (cfg, provider) = state.assistant_service.provider().await?;
    let _guard = InFlight::acquire(user_id)?;
    let principal = state.mcp_service.assistant_principal(user_id).await?;
    let conversation_id = conversation_id(req.conversation_id.as_deref());
    let actor = agent::Actor {
        user_id,
        username: &username,
        role: &role,
        principal: &principal,
        session_id: format!("assistant:{conversation_id}"),
    };
    let out = agent::run(&state, provider.as_ref(), &cfg, &actor, history).await?;
    let approval_ids = out.steps.iter().filter_map(|s| s.approval_id).collect();
    Ok(Json(ChatResponse {
        conversation_id,
        reply: out.reply,
        steps: out.steps,
        step_limit_reached: out.step_limit_reached,
        approval_ids,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/assistant/config", get(get_config).put(update_config))
        .route("/api/assistant/test", post(test_connection))
        .route("/api/assistant/chat", post(chat))
}
