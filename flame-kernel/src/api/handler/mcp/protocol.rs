//! MCP Streamable HTTP 传输（JSON-RPC 2.0）——手写实现，不引入新依赖。
//!
//! | 请求 | 行为 |
//! |---|---|
//! | `POST /api/mcp` 请求（含 `id`） | 200 `application/json`（单条或批量数组） |
//! | `POST /api/mcp` 仅通知 / 响应 | 202，无响应体 |
//! | `GET /api/mcp` | 405（本服务不提供服务端推送 SSE 流） |
//! | `DELETE /api/mcp` + `Mcp-Session-Id` | 204，终止会话 |
//!
//! 会话：`initialize` 响应头下发 `Mcp-Session-Id`；后续请求携带时必须有效（否则 404，
//! 客户端应重新初始化），不携带时按无状态模式处理（审计中 session 为空）。
//!
//! 安全：认证在 `auth_middleware` 完成（`fpk_` Bearer）；此处额外校验 `Origin`
//! 防 DNS rebinding（浏览器跨站携带凭证的请求直接 403）。
use super::McpCaller;
use super::{gate, tools};
use crate::api::app_state::{UserRole, Username};
use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use serde_json::{json, Value};

pub const SESSION_HEADER: &str = "mcp-session-id";
/// 支持的协议版本（新 → 旧）；客户端请求其一则回显，否则回退到最新。
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
/// 批量请求条数上限。
const MAX_BATCH: usize = 32;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn json_response(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// `Origin` 存在时其 host[:port] 必须与请求 `Host` 一致（非浏览器客户端通常不带 Origin）。
pub fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    if origin == "null" {
        return false;
    }
    let origin_host = origin
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(origin)
        .trim_end_matches('/');
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let strip_default = |h: &str| {
        h.strip_suffix(":443")
            .or_else(|| h.strip_suffix(":80"))
            .unwrap_or(h)
            .to_ascii_lowercase()
    };
    !host.is_empty() && strip_default(origin_host) == strip_default(host)
}

fn negotiate_version(requested: Option<&str>) -> &'static str {
    requested
        .and_then(|r| SUPPORTED_VERSIONS.iter().find(|v| **v == r).copied())
        .unwrap_or(SUPPORTED_VERSIONS[0])
}

struct Ctx<'a> {
    state: &'a AppState,
    caller: &'a McpCaller,
    username: &'a str,
    role: &'a str,
    session_id: Option<String>,
}

/// `POST /api/mcp` — MCP JSON-RPC 消息（单条或批量）
#[utoipa::path(
    post,
    path = "/api/mcp",
    tag = "mcp",
    request_body(content = Object, description = "JSON-RPC 2.0 请求 / 通知（或其数组，≤ 32 条）", content_type = "application/json"),
    params(("Mcp-Session-Id" = Option<String>, Header, description = "initialize 下发的会话 ID；缺省为无状态模式")),
    responses(
        (status = 200, description = "JSON-RPC 响应（initialize 时响应头含 Mcp-Session-Id）", body = Object),
        (status = 202, description = "仅通知 / 响应，无响应体"),
        (status = 400, description = "JSON 解析失败或非法批量"),
        (status = 401, description = "fpk_ 凭证缺失 / 无效 / 吊销 / 过期"),
        (status = 403, description = "Origin 与 Host 不一致"),
        (status = 404, description = "会话不存在或已终止"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn post(
    State(state): State<AppState>,
    Extension(caller): Extension<McpCaller>,
    Extension(Username(username)): Extension<Username>,
    Extension(UserRole(role)): Extension<UserRole>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !origin_allowed(&headers) {
        return json_response(
            StatusCode::FORBIDDEN,
            &rpc_error(Value::Null, INVALID_REQUEST, "Origin not allowed"),
        );
    }
    let msg: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &rpc_error(Value::Null, PARSE_ERROR, format!("Parse error: {e}")),
            )
        }
    };
    let (batch, messages) = match msg {
        Value::Array(items) if items.is_empty() => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &rpc_error(Value::Null, INVALID_REQUEST, "Empty batch"),
            )
        }
        Value::Array(items) if items.len() > MAX_BATCH => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &rpc_error(
                    Value::Null,
                    INVALID_REQUEST,
                    format!("Batch too large (max {MAX_BATCH})"),
                ),
            )
        }
        Value::Array(items) => (true, items),
        other => (false, vec![other]),
    };

    // 会话：initialize 不需要；其余请求若携带会话头则必须有效
    let is_initialize = messages
        .iter()
        .any(|m| m.get("method").and_then(Value::as_str) == Some("initialize"));
    let header_session = headers
        .get(SESSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let mut session_id = None;
    if let (false, Some(sid)) = (is_initialize, header_session) {
        match state
            .mcp_service
            .resolve_session(&sid, caller.principal.id)
            .await
        {
            Ok(s) => session_id = Some(s.id),
            Err(AppError::NotFound(m)) => {
                return json_response(
                    StatusCode::NOT_FOUND,
                    &rpc_error(Value::Null, INVALID_REQUEST, m),
                )
            }
            Err(e) => return e.into_response(),
        }
    }

    let mut ctx = Ctx {
        state: &state,
        caller: &caller,
        username: &username,
        role: &role,
        session_id,
    };
    let mut responses = Vec::new();
    let mut new_session: Option<String> = None;
    for m in messages {
        if let Some(r) = handle_message(&mut ctx, m, &mut new_session).await {
            responses.push(r);
        }
    }

    let mut resp = if responses.is_empty() {
        StatusCode::ACCEPTED.into_response()
    } else if batch {
        json_response(StatusCode::OK, &Value::Array(responses))
    } else {
        json_response(StatusCode::OK, &responses.remove(0))
    };
    if let Some(sid) = new_session {
        if let Ok(v) = HeaderValue::from_str(&sid) {
            resp.headers_mut().insert(SESSION_HEADER, v);
        }
    }
    resp
}

/// 处理单条消息；通知 / 客户端响应返回 `None`。
async fn handle_message(
    ctx: &mut Ctx<'_>,
    msg: Value,
    new_session: &mut Option<String>,
) -> Option<Value> {
    let Value::Object(obj) = msg else {
        return Some(rpc_error(Value::Null, INVALID_REQUEST, "Invalid Request"));
    };
    let id = obj.get("id").cloned();
    let method = obj.get("method").and_then(Value::as_str);
    let Some(method) = method else {
        // 客户端对服务端请求的响应（本服务不发起请求）——忽略
        return None;
    };
    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return id.map(|id| rpc_error(id, INVALID_REQUEST, "jsonrpc must be \"2.0\""));
    }
    // 通知：无 id，不回响应
    let id = id?;
    if !(id.is_string() || id.is_number()) {
        return Some(rpc_error(Value::Null, INVALID_REQUEST, "Invalid id"));
    }
    let params = obj.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => initialize(ctx, id, &params, new_session).await,
        "ping" => rpc_result(id, json!({})),
        "tools/list" => rpc_result(id, json!({"tools": visible_tools(ctx).await})),
        "tools/call" => tools_call(ctx, id, &params).await,
        "prompts/list" => rpc_result(id, json!({"prompts": prompts_list()})),
        "prompts/get" => prompts_get(id, &params),
        _ => rpc_error(id, METHOD_NOT_FOUND, format!("Method not found: {method}")),
    })
}

async fn initialize(
    ctx: &mut Ctx<'_>,
    id: Value,
    params: &Value,
    new_session: &mut Option<String>,
) -> Value {
    let version = negotiate_version(params.get("protocolVersion").and_then(Value::as_str));
    let client = params.get("clientInfo");
    let s = |k: &str| {
        client
            .and_then(|c| c.get(k))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    match ctx
        .state
        .mcp_service
        .open_session(ctx.caller.principal.id, &s("name"), &s("version"), version)
        .await
    {
        Ok(session) => {
            ctx.session_id = Some(session.id.clone());
            *new_session = Some(session.id);
        }
        Err(e) => return rpc_error(id, INTERNAL_ERROR, e.to_string()),
    }
    rpc_result(
        id,
        json!({
            "protocolVersion": version,
            "capabilities": {
                "tools": {"listChanged": false},
                "prompts": {"listChanged": false},
            },
            "serverInfo": {
                "name": "flamepanel",
                "title": "FlamePanel",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "instructions": "FlamePanel 服务器管理面板工具集：主机状态、容器与日志、网站、数据库、应用、防火墙、备份与定时任务、面板日志、文件读取；写操作只生成审批单，由面板用户批准后执行。prompts 提供常用运维技能（排障、巡检、安全检查、变更检查点）。所有调用以凭证绑定用户的权限执行并记录审计。",
        }),
    )
}

impl Ctx<'_> {
    fn gate(&self) -> gate::GateCtx<'_> {
        gate::GateCtx {
            state: self.state,
            principal: &self.caller.principal,
            username: self.username,
            role: self.role,
            session_id: self.session_id.clone(),
        }
    }
}

/// `tools/list` 只列出当前用户角色可用、且凭证允许的工具。
async fn visible_tools(ctx: &Ctx<'_>) -> Vec<Value> {
    gate::visible(&ctx.gate())
        .await
        .into_iter()
        .map(tools::ToolDef::to_json)
        .collect()
}

async fn tools_call(ctx: &Ctx<'_>, id: Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return rpc_error(id, INVALID_PARAMS, "Missing tool name");
    };
    let Some(tool) = tools::find(name) else {
        return rpc_error(id, INVALID_PARAMS, format!("Unknown tool: {name}"));
    };
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let (text, is_error) = match gate::call(&ctx.gate(), tool, &args).await.result {
        Ok(t) => (t, false),
        Err(e) => (e, true),
    };
    rpc_result(
        id,
        json!({"content": [{"type": "text", "text": text}], "isError": is_error}),
    )
}

/// `prompts/list`：运维技能库（纯文本模板，不涉及权限——执行时调用的工具仍逐个鉴权）。
fn prompts_list() -> Vec<Value> {
    crate::domain::runbook::RUNBOOKS
        .iter()
        .map(|rb| {
            json!({
                "name": rb.name,
                "title": rb.title,
                "description": rb.description,
                "arguments": rb.arguments,
            })
        })
        .collect()
}

/// `prompts/get`：渲染指定技能。参数值必须是字符串（MCP 规范）。
fn prompts_get(id: Value, params: &Value) -> Value {
    use crate::domain::runbook;
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return rpc_error(id, INVALID_PARAMS, "Missing prompt name");
    };
    let Some(rb) = runbook::find(name) else {
        return rpc_error(id, INVALID_PARAMS, format!("Unknown prompt: {name}"));
    };
    let mut args = std::collections::HashMap::new();
    match params.get("arguments") {
        None | Some(Value::Null) => {}
        Some(Value::Object(m)) => {
            for (k, v) in m {
                let Some(s) = v.as_str() else {
                    return rpc_error(id, INVALID_PARAMS, format!("参数 {k} 必须是字符串"));
                };
                args.insert(k.clone(), s.to_string());
            }
        }
        Some(_) => return rpc_error(id, INVALID_PARAMS, "arguments 必须是对象"),
    }
    match runbook::render(rb, &args) {
        Ok(text) => rpc_result(
            id,
            json!({
                "description": rb.description,
                "messages": [{"role": "user", "content": {"type": "text", "text": text}}],
            }),
        ),
        Err(e) => rpc_error(id, INVALID_PARAMS, e),
    }
}

/// `GET /api/mcp` — 不提供 SSE 推送流。
#[utoipa::path(
    get,
    path = "/api/mcp",
    tag = "mcp",
    responses((status = 405, description = "不提供服务端推送流")),
    security(("BearerAuth" = []))
)]
pub async fn get() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "POST, DELETE")],
    )
        .into_response()
}

/// `DELETE /api/mcp` — 终止会话。
#[utoipa::path(
    delete,
    path = "/api/mcp",
    tag = "mcp",
    params(("Mcp-Session-Id" = String, Header, description = "要终止的会话 ID")),
    responses(
        (status = 204, description = "会话已终止"),
        (status = 400, description = "缺少 Mcp-Session-Id"),
        (status = 401, description = "fpk_ 凭证无效"),
        (status = 404, description = "会话不存在或已终止"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Extension(caller): Extension<McpCaller>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    if !origin_allowed(&headers) {
        return Err(AppError::Forbidden("Origin not allowed".into()));
    }
    let sid = headers
        .get(SESSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing Mcp-Session-Id header".into()))?;
    state
        .mcp_service
        .close_session(sid, caller.principal.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut m = HeaderMap::new();
        for (k, v) in pairs {
            m.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        m
    }

    #[test]
    fn origin_check() {
        assert!(origin_allowed(&h(&[("host", "panel.example")])));
        assert!(origin_allowed(&h(&[
            ("host", "panel.example"),
            ("origin", "https://panel.example")
        ])));
        assert!(origin_allowed(&h(&[
            ("host", "panel.example:443"),
            ("origin", "https://panel.example")
        ])));
        assert!(origin_allowed(&h(&[
            ("host", "127.0.0.1:9999"),
            ("x-forwarded-host", "panel.example"),
            ("origin", "https://panel.example")
        ])));
        assert!(!origin_allowed(&h(&[
            ("host", "panel.example"),
            ("origin", "https://evil.example")
        ])));
        assert!(!origin_allowed(&h(&[
            ("host", "panel.example"),
            ("origin", "null")
        ])));
    }

    #[test]
    fn version_negotiation() {
        assert_eq!(negotiate_version(Some("2025-03-26")), "2025-03-26");
        assert_eq!(negotiate_version(Some("1999-01-01")), SUPPORTED_VERSIONS[0]);
        assert_eq!(negotiate_version(None), SUPPORTED_VERSIONS[0]);
    }
}
