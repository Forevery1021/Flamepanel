//! M-AI-1 内置 MCP Server（Doc/30 §M-AI-1）。
//!
//! | 端点 | 鉴权 | 说明 |
//! |---|---|---|
//! | `POST/GET/DELETE /api/mcp` | `fpk_` 凭证（Bearer） | MCP Streamable HTTP 协议端点，见 [`protocol`] |
//! | `GET /api/mcp/tools` | `mcp:read` | 工具目录（含由 REST 权限表推导的所需权限） |
//! | `GET /api/mcp/prompts[/{name}]` | `mcp:read` | 运维技能库（Runbook 提示词）列表 / 渲染 |
//! | `GET /api/mcp/principals` | `mcp:read` | 凭证列表（不含令牌） |
//! | `POST /api/mcp/principals` | `mcp:create` | 为当前用户签发凭证，明文令牌仅此一次返回 |
//! | `POST /api/mcp/principals/{id}/revoke` | `mcp:create` | 吊销；非本人凭证另需 `mcp:delete` |
//! | `GET /api/mcp/sessions` | `mcp:read` | 会话列表（最近活跃在前） |
//! | `GET /api/mcp/calls` | `mcp:read` | 工具调用审计分页 |
//! | `/api/mcp/approvals…` | `mcp:read` / `mcp:update` | M-AI-2 写操作审批，见 [`approvals`] |
//!
//! 凭证绑定签发者，工具以该用户**当前**角色鉴权；因此签发不存在越权问题——
//! 任何人只能签发不超过自己权限的凭证。
pub mod approvals;
pub mod gate;
pub mod protocol;
pub mod tools;

use crate::api::app_state::{UserId, UserRole};
use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::mcp::{AgentPrincipal, AgentSession, ToolCallFilter, ToolCallRecord};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};

/// 认证中间件注入的 MCP 调用方。
#[derive(Clone)]
pub struct McpCaller {
    pub principal: AgentPrincipal,
}

/// 凭证视图（附签发者用户名与有效状态）。
#[derive(Debug, Serialize, ToSchema)]
pub struct PrincipalView {
    #[serde(flatten)]
    pub principal: AgentPrincipal,
    /// 签发者用户名（用户已删除时为空）
    pub owner: String,
    /// 未吊销且未过期
    pub active: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePrincipalRequest {
    /// 凭证名称（1–64 字符），如 `claude-desktop@laptop`
    pub name: String,
    /// 有效天数（1–3650）；缺省 / null 表示永不过期
    #[serde(default)]
    pub expires_in_days: Option<i64>,
    /// 允许发起写操作（M-AI-2）；写操作仍须面板人工批准。缺省 false（只读）
    #[serde(default)]
    pub allow_write: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatePrincipalResponse {
    pub principal: PrincipalView,
    /// 明文令牌——**仅此一次返回**，请立即保存
    pub token: String,
    /// MCP 端点路径（客户端配置 `url` 时拼上面板地址）
    pub endpoint: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ToolInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    /// 所需权限 `resource:action`（由等价 REST 路由推导）
    pub permission: String,
    /// 当前用户是否可用
    pub allowed: bool,
    /// 风险分级：read / write / destructive（非 read 需人工审批）
    pub risk: String,
    #[schema(value_type = Object)]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SessionQuery {
    /// 按凭证过滤
    pub principal_id: Option<i64>,
    /// 返回条数（1–500，缺省 100）
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CallQuery {
    pub principal_id: Option<i64>,
    pub session_id: Option<String>,
    pub tool: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

async fn owner_names(state: &AppState) -> HashMap<i64, String> {
    state
        .user_service
        .list_users()
        .await
        .map(|us| us.into_iter().map(|u| (u.id, u.username)).collect())
        .unwrap_or_default()
}

fn view(p: AgentPrincipal, owners: &HashMap<i64, String>) -> PrincipalView {
    PrincipalView {
        owner: owners.get(&p.user_id).cloned().unwrap_or_default(),
        active: p.is_active(chrono::Utc::now()),
        principal: p,
    }
}

/// `GET /api/mcp/tools` — 工具目录
#[utoipa::path(
    get,
    path = "/api/mcp/tools",
    tag = "mcp",
    responses(
        (status = 200, description = "工具目录", body = Vec<ToolInfo>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_tools(
    State(state): State<AppState>,
    Extension(UserRole(role)): Extension<UserRole>,
) -> Result<Json<Vec<ToolInfo>>, AppError> {
    let mut out = Vec::with_capacity(tools::TOOLS.len());
    for t in tools::TOOLS {
        let (res, act) = t.permission().unwrap_or(("", ""));
        let allowed =
            !res.is_empty() && state.role_service.check_permission(&role, res, act).await?;
        out.push(ToolInfo {
            name: t.name.into(),
            title: t.title.into(),
            description: t.description.into(),
            permission: format!("{res}:{act}"),
            allowed,
            risk: t.risk.as_str().into(),
            input_schema: (t.input_schema)(),
        });
    }
    Ok(Json(out))
}

/// 运维技能参数。
#[derive(Serialize, ToSchema)]
#[schema(as = McpPromptArgument)]
pub struct PromptArgInfo {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// 运维技能（Runbook 提示词）。
#[derive(Serialize, ToSchema)]
#[schema(as = McpPromptInfo)]
pub struct PromptInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub arguments: Vec<PromptArgInfo>,
}

/// 渲染后的技能提示词。
#[derive(Serialize, ToSchema)]
#[schema(as = McpPromptRendered)]
pub struct PromptRendered {
    pub name: String,
    pub title: String,
    pub text: String,
}

/// `GET /api/mcp/prompts` — 运维技能库
#[utoipa::path(
    get,
    path = "/api/mcp/prompts",
    tag = "mcp",
    responses(
        (status = 200, description = "技能列表（与 MCP prompts/list 一致）", body = Vec<PromptInfo>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_prompts() -> Json<Vec<PromptInfo>> {
    Json(
        crate::domain::runbook::RUNBOOKS
            .iter()
            .map(|rb| PromptInfo {
                name: rb.name.into(),
                title: rb.title.into(),
                description: rb.description.into(),
                arguments: rb
                    .arguments
                    .iter()
                    .map(|a| PromptArgInfo {
                        name: a.name.into(),
                        description: a.description.into(),
                        required: a.required,
                    })
                    .collect(),
            })
            .collect(),
    )
}

/// `GET /api/mcp/prompts/{name}` — 渲染技能（查询参数即技能参数）
#[utoipa::path(
    get,
    path = "/api/mcp/prompts/{name}",
    tag = "mcp",
    params(
        ("name" = String, Path, description = "技能名称"),
    ),
    responses(
        (status = 200, description = "渲染后的提示词", body = PromptRendered),
        (status = 400, description = "缺少必填参数 / 未声明的参数"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
        (status = 404, description = "技能不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn render_prompt(
    Path(name): Path<String>,
    Query(args): Query<HashMap<String, String>>,
) -> Result<Json<PromptRendered>, AppError> {
    let rb = crate::domain::runbook::find(&name)
        .ok_or_else(|| AppError::NotFound(format!("技能 {name} 不存在")))?;
    let text = crate::domain::runbook::render(rb, &args).map_err(AppError::ValidationError)?;
    Ok(Json(PromptRendered {
        name: rb.name.into(),
        title: rb.title.into(),
        text,
    }))
}

/// `GET /api/mcp/principals` — 凭证列表
#[utoipa::path(
    get,
    path = "/api/mcp/principals",
    tag = "mcp",
    responses(
        (status = 200, description = "凭证列表（不含令牌）", body = Vec<PrincipalView>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_principals(
    State(state): State<AppState>,
) -> Result<Json<Vec<PrincipalView>>, AppError> {
    let owners = owner_names(&state).await;
    let list = state.mcp_service.list_principals().await?;
    Ok(Json(list.into_iter().map(|p| view(p, &owners)).collect()))
}

/// `POST /api/mcp/principals` — 为当前用户签发凭证
#[utoipa::path(
    post,
    path = "/api/mcp/principals",
    tag = "mcp",
    request_body = CreatePrincipalRequest,
    responses(
        (status = 201, description = "已签发；token 仅此一次返回", body = CreatePrincipalResponse),
        (status = 400, description = "名称或有效期不合法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_principal(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    ApiJson(req): ApiJson<CreatePrincipalRequest>,
) -> Result<(StatusCode, Json<CreatePrincipalResponse>), AppError> {
    let issued = state
        .mcp_service
        .issue_with(user_id, &req.name, req.expires_in_days, req.allow_write)
        .await?;
    let owners = owner_names(&state).await;
    Ok((
        StatusCode::CREATED,
        Json(CreatePrincipalResponse {
            principal: view(issued.principal, &owners),
            token: issued.token,
            endpoint: "/api/mcp".into(),
        }),
    ))
}

/// `POST /api/mcp/principals/{id}/revoke` — 吊销凭证
#[utoipa::path(
    post,
    path = "/api/mcp/principals/{id}/revoke",
    tag = "mcp",
    params(("id" = i64, Path, description = "凭证 ID")),
    responses(
        (status = 200, description = "已吊销（幂等）", body = PrincipalView),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:create 权限，或吊销他人凭证缺少 mcp:delete"),
        (status = 404, description = "凭证不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn revoke_principal(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    Extension(UserRole(role)): Extension<UserRole>,
    Path(id): Path<i64>,
) -> Result<Json<PrincipalView>, AppError> {
    let p = state.mcp_service.get_principal(id).await?;
    // 字段级授权：本人凭证凭 mcp:create 即可自助吊销；他人凭证需 mcp:delete
    if p.user_id != user_id
        && !state
            .role_service
            .check_permission(&role, "mcp", "delete")
            .await?
    {
        return Err(AppError::Forbidden(
            "Missing permission: mcp:delete (revoking another user's credential)".into(),
        ));
    }
    let p = state.mcp_service.revoke(id).await?;
    let owners = owner_names(&state).await;
    Ok(Json(view(p, &owners)))
}

/// `GET /api/mcp/sessions` — 会话列表
#[utoipa::path(
    get,
    path = "/api/mcp/sessions",
    tag = "mcp",
    params(SessionQuery),
    responses(
        (status = 200, description = "会话列表（最近活跃在前）", body = Vec<AgentSession>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    Query(q): Query<SessionQuery>,
) -> Result<Json<Vec<AgentSession>>, AppError> {
    Ok(Json(
        state
            .mcp_service
            .list_sessions(q.principal_id, q.limit.unwrap_or(100))
            .await?,
    ))
}

/// `GET /api/mcp/calls` — 工具调用审计
#[utoipa::path(
    get,
    path = "/api/mcp/calls",
    tag = "mcp",
    params(CallQuery),
    responses(
        (status = 200, description = "调用记录分页（最新在前）", body = PaginatedResponse<ToolCallRecord>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_calls(
    State(state): State<AppState>,
    Query(q): Query<CallQuery>,
) -> Result<Json<PaginatedResponse<ToolCallRecord>>, AppError> {
    let filter = ToolCallFilter {
        principal_id: q.principal_id,
        session_id: q.session_id.filter(|s| !s.is_empty()),
        tool: q.tool.filter(|s| !s.is_empty()),
    };
    let pagination = PaginationParams {
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = state
        .mcp_service
        .list_calls(&filter, pagination.page(), pagination.page_size())
        .await?;
    Ok(Json(PaginatedResponse::new(items, total, &pagination)))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/mcp",
            post(protocol::post)
                .get(protocol::get)
                .delete(protocol::delete),
        )
        .route("/api/mcp/tools", get(list_tools))
        .route("/api/mcp/prompts", get(list_prompts))
        .route("/api/mcp/prompts/{name}", get(render_prompt))
        .route(
            "/api/mcp/principals",
            get(list_principals).post(create_principal),
        )
        .route("/api/mcp/principals/{id}/revoke", post(revoke_principal))
        .route("/api/mcp/sessions", get(list_sessions))
        .route("/api/mcp/calls", get(list_calls))
        .route("/api/mcp/approvals", get(approvals::list_approvals))
        .route("/api/mcp/approvals/{id}", get(approvals::get_approval))
        .route("/api/mcp/approvals/{id}/approve", post(approvals::approve))
        .route("/api/mcp/approvals/{id}/reject", post(approvals::reject))
}
