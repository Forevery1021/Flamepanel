//! NL2API：以 OpenAPI 为工具目录（M-AI-3 · Doc/32 §7）。
//!
//! 给模型两个**只读**工具，让它在 MCP 精选工具之外也能查询面板的任意 GET 接口：
//!
//! - `api_search`：按关键字检索 OpenAPI 目录，只列出当前用户有权限、且不在拒绝名单中的 GET 操作；
//! - `api_get`：调用其中一个 GET 接口。
//!
//! 安全边界：
//! - **只读**：只允许 GET，且路径必须匹配 OpenAPI 中声明的 GET 模板——写操作只能走 MCP 写工具 + 审批；
//! - **权限同源**：用 REST 权限表（`route_permission`）+ 调用者角色判定，与中间件同一套规则；
//!   未声明权限的路径一律拒绝（与中间件对未知路由的处理一致）；
//! - **拒绝名单**：认证 / 初始化、AI 自身、文件内容、下载 / 导出 / 流式、含凭据的设置与账号类接口；
//! - **输出**：只接受 JSON，≤ 256 KiB，统一脱敏（敏感键 + PEM 块），再截断到工具输出上限。
use crate::api::app_state::{UserId, UserRole, Username};
use crate::api::handler::mcp::tools;
use crate::api::types::{route_permission, AppState};
use serde_json::{json, Value};
use std::sync::OnceLock;

/// 响应体读取上限。
const MAX_BODY_BYTES: usize = 256 * 1024;
/// `api_search` 默认 / 最大返回条数。
const SEARCH_DEFAULT: usize = 15;
const SEARCH_MAX: usize = 40;

/// 路径前缀拒绝名单（含凭据、会话、AI 自身或文件内容的接口）。
const DENY_PREFIXES: &[&str] = &[
    "/api/auth",
    "/api/setup",
    "/api/mcp",
    "/api/assistant",
    "/api/files",
    "/api/settings",
    "/api/backup-accounts",
    "/api/dns-providers",
    "/api/certificates",
    "/api/acme",
    "/api/kernel-logs/files",
    "/api/ws",
    "/api/terminal",
    "/api/users/me/2fa",
];
/// 只拒绝其下级路径（列表本身可读，单项是文件下载）。
const DENY_CHILDREN_OF: &[&str] = &["/api/backups"];
/// 路径片段拒绝名单（下载、导出、流式、原始内容）。
const DENY_SEGMENTS: &[&str] = &[
    "download", "export", "stream", "raw", "ws", "terminal", "secret", "token", "2fa",
];

/// OpenAPI 目录中的一个 GET 操作。
#[derive(Debug, Clone)]
pub struct ApiOp {
    pub path: String,
    pub summary: String,
    pub tag: String,
    /// `name(in)` 形式的参数列表
    pub params: Vec<String>,
}

/// 进程级缓存的 GET 目录（OpenAPI 文档在编译期固定）。
pub fn catalog() -> &'static [ApiOp] {
    static CATALOG: OnceLock<Vec<ApiOp>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        use utoipa::OpenApi;
        let doc = serde_json::to_value(crate::openapi::ApiDoc::openapi()).unwrap_or(Value::Null);
        let mut out = Vec::new();
        if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
            for (path, item) in paths {
                let Some(op) = item.get("get") else { continue };
                let s = |k: &str| op.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                let tag = op
                    .pointer("/tags/0")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let params = op
                    .get("parameters")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|p| {
                                let n = p.get("name")?.as_str()?;
                                let i = p.get("in").and_then(Value::as_str).unwrap_or("");
                                Some(format!("{n}({i})"))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut summary = s("summary");
                if summary.is_empty() {
                    summary = s("description").lines().next().unwrap_or("").to_string();
                }
                out.push(ApiOp {
                    path: path.clone(),
                    summary,
                    tag,
                    params,
                });
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    })
}

/// 路径是否在拒绝名单中（模板或具体路径均可）。
pub fn is_denied(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    if DENY_PREFIXES
        .iter()
        .any(|d| p == *d || p.starts_with(&format!("{d}/")) || p.starts_with(&format!("{d}?")))
    {
        return true;
    }
    if DENY_CHILDREN_OF
        .iter()
        .any(|d| p.starts_with(&format!("{d}/")))
    {
        return true;
    }
    p.split('/').any(|seg| {
        DENY_SEGMENTS.iter().any(|d| {
            seg == *d || seg.ends_with(&format!("-{d}")) || seg.starts_with(&format!("{d}-"))
        })
    })
}

/// 具体路径是否匹配模板（`{x}` 匹配任意非空片段）。
pub fn matches_template(template: &str, path: &str) -> bool {
    let t: Vec<&str> = template.trim_end_matches('/').split('/').collect();
    let p: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    t.len() == p.len()
        && t.iter().zip(&p).all(|(a, b)| {
            if a.starts_with('{') && a.ends_with('}') {
                !b.is_empty()
            } else {
                a == b
            }
        })
}

/// 路径对应的**只读**权限；GET 但需要非 read 动作的接口（如经 Agent 列远程目录需 `node:execute`）一律不开放。
pub fn read_permission(path: &str) -> Result<(&'static str, &'static str), String> {
    let (res, act) = route_permission(&axum::http::Method::GET, path)
        .ok_or_else(|| format!("{path} 未声明权限，助手不能调用"))?;
    if act != "read" {
        return Err(format!(
            "{path} 需要 {res}:{act}（非只读权限），助手不能调用"
        ));
    }
    Ok((res, act))
}

async fn permitted(state: &AppState, role: &str, path: &str) -> Result<(), String> {
    let (res, act) = read_permission(path)?;
    match state.role_service.check_permission(role, res, act).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("权限不足：缺少 {res}:{act}")),
        Err(e) => Err(e.to_string()),
    }
}

/// `api_search`：关键字检索（空关键字返回按 tag 分组的概览）。
pub async fn search(state: &AppState, role: &str, args: &Value) -> tools::ToolResult {
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if query.chars().count() > 200 {
        return Err("query 过长（≤ 200 字符）".into());
    }
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .map(|n| (n as usize).clamp(1, SEARCH_MAX))
        .unwrap_or(SEARCH_DEFAULT);
    let terms: Vec<&str> = query.split_whitespace().collect();
    let mut hits: Vec<(usize, &ApiOp)> = Vec::new();
    for op in catalog() {
        if is_denied(&op.path) || permitted(state, role, &op.path).await.is_err() {
            continue;
        }
        let hay = format!("{} {} {}", op.path, op.summary, op.tag).to_lowercase();
        let score = if terms.is_empty() {
            1
        } else {
            terms.iter().filter(|t| hay.contains(**t)).count()
        };
        if score > 0 {
            hits.push((score, op));
        }
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
    let total = hits.len();
    let list: Vec<Value> = hits
        .into_iter()
        .take(limit)
        .map(|(_, op)| {
            json!({"method": "GET", "path": op.path, "summary": op.summary, "tag": op.tag, "params": op.params})
        })
        .collect();
    if list.is_empty() {
        return Ok(format!(
            "没有匹配「{query}」且你有权限调用的只读接口。可换个关键字（如 docker、website、database、cron、log），或直接使用专用工具。"
        ));
    }
    Ok(serde_json::to_string_pretty(&json!({
        "total": total,
        "operations": list,
        "hint": "用 api_get 调用；把 {id} 等路径参数替换为实际值，query 参数直接拼在 path 后（如 ?page=1&page_size=20）",
    }))
    .unwrap_or_default())
}

/// 校验并规范化 `api_get` 的路径，返回 (路径, 完整 URI)。
pub fn validate_path(raw: &str) -> Result<(String, String), String> {
    let raw = raw.trim();
    if raw.len() > 512 {
        return Err("path 过长（≤ 512 字符）".into());
    }
    if raw.chars().any(|c| c.is_control() || c.is_whitespace()) || raw.contains('#') {
        return Err("path 不能包含空白、控制字符或 #".into());
    }
    if !raw.starts_with("/api/") {
        return Err("path 必须以 /api/ 开头（例如 /api/docker/containers）".into());
    }
    let (path, _query) = raw.split_once('?').unwrap_or((raw, ""));
    // 解码后再检查穿越，防止 `%2e%2e`
    let lower = path.to_ascii_lowercase();
    if path.contains("..")
        || path.contains("//")
        || lower.contains("%2e")
        || lower.contains("%2f")
        || lower.contains("%5c")
        || path.contains('\\')
    {
        return Err("path 不合法".into());
    }
    Ok((path.to_string(), raw.to_string()))
}

/// 递归脱敏的补充：PEM 块（证书私钥等）整体替换。
fn redact_pem(v: &mut Value) {
    match v {
        Value::String(s) if s.contains("-----BEGIN") => *s = "***".into(),
        Value::Array(a) => a.iter_mut().for_each(redact_pem),
        Value::Object(m) => m.values_mut().for_each(redact_pem),
        _ => {}
    }
}

/// `api_get`：进程内调用 GET 接口（不经网络，复用同一路由表与处理器）。
pub async fn get(state: &AppState, user: (i64, &str, &str), args: &Value) -> tools::ToolResult {
    let (user_id, username, role) = user;
    let raw = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("缺少必填字符串参数 `path`")?;
    let (path, uri) = validate_path(raw)?;
    if is_denied(&path) {
        return Err(format!(
            "{path} 不对 AI 助手开放（含凭据 / 文件内容 / 下载类接口）"
        ));
    }
    if !catalog().iter().any(|op| matches_template(&op.path, &path)) {
        return Err(format!(
            "{path} 不是已声明的只读接口；先用 api_search 查找可用接口"
        ));
    }
    permitted(state, role, &path).await?;

    let mut req = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri(&uri)
        .header("accept", "application/json")
        .body(axum::body::Body::empty())
        .map_err(|_| "path 不合法".to_string())?;
    req.extensions_mut().insert(UserId(user_id));
    req.extensions_mut().insert(Username(username.to_string()));
    req.extensions_mut().insert(UserRole(role.to_string()));
    let mut router = crate::api::routes::create_router(state.clone()).layer(
        axum::extract::connect_info::MockConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            0,
        ))),
    );
    use tower::Service;
    std::future::poll_fn(|cx| {
        Service::<axum::http::Request<axum::body::Body>>::poll_ready(&mut router, cx)
    })
    .await
    .map_err(|_| "内部路由不可用".to_string())?;
    let resp = router
        .call(req)
        .await
        .map_err(|_| "内部路由调用失败".to_string())?;
    let status = resp.status();
    let is_json = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("application/json"))
        .unwrap_or(false);
    let bytes = axum::body::to_bytes(resp.into_body(), MAX_BODY_BYTES)
        .await
        .map_err(|_| format!("响应超过 {} KiB，请加分页或过滤参数", MAX_BODY_BYTES / 1024))?;
    if !is_json {
        return Err(format!(
            "{path} 返回的不是 JSON（HTTP {}），助手不读取",
            status.as_u16()
        ));
    }
    let mut v: Value =
        serde_json::from_slice(&bytes).map_err(|_| "响应 JSON 解析失败".to_string())?;
    tools::redact(&mut v);
    redact_pem(&mut v);
    let text = serde_json::to_string_pretty(&v).unwrap_or_default();
    let text = crate::application::mcp_service::clip_bytes(&text, tools::MAX_OUTPUT_BYTES);
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!(
            "HTTP {}：{}",
            status.as_u16(),
            crate::application::mcp_service::clip_bytes(&text, 2048)
        ))
    }
}

/// 两个 NL2API 工具的 JSON Schema。
pub fn search_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "关键字，空格分隔（英文路径词更准，如 `docker image`、`cron`、`website ssl`）"},
            "limit": {"type": "integer", "minimum": 1, "maximum": SEARCH_MAX, "default": SEARCH_DEFAULT},
        },
        "additionalProperties": false,
    })
}

pub fn get_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "接口路径，路径参数替换为实际值，可带 query，如 `/api/docker/images?page=1`"},
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_known_get_routes_only() {
        let c = catalog();
        assert!(c.len() > 50, "OpenAPI GET 目录应非空: {}", c.len());
        assert!(c.iter().any(|o| o.path == "/api/docker/containers"));
        // 目录只含 GET 操作：compose/deploy 只有 POST
        assert!(!c.iter().any(|o| o.path == "/api/docker/compose/deploy"));
    }

    #[test]
    fn deny_list() {
        for p in [
            "/api/auth/me",
            "/api/settings",
            "/api/settings/smtp_password",
            "/api/files/content",
            "/api/mcp/principals",
            "/api/assistant/config",
            "/api/backup/1/download",
            "/api/kernel-logs/files/x.log",
            "/api/backup-accounts",
            "/api/certificates/1",
            "/api/dns-providers",
            "/api/backups/app-2026.tar.gz",
            "/api/nodes/1/files/download",
        ] {
            assert!(is_denied(p), "{p}");
        }
        for p in [
            "/api/docker/containers",
            "/api/websites",
            "/api/overview",
            "/api/kernel-logs/entries",
            "/api/backups",
            "/api/metrics/history",
        ] {
            assert!(!is_denied(p), "{p}");
        }
    }

    #[test]
    fn only_read_actions_are_callable() {
        assert!(read_permission("/api/docker/containers").is_ok());
        // GET 但需要 node:execute（经 Agent 列远程目录）
        let e = read_permission("/api/nodes/1/files").unwrap_err();
        assert!(e.contains("node:execute"), "{e}");
        // 大部分目录项可用（防止拒绝名单 / 权限映射误伤过多）
        let usable = catalog()
            .iter()
            .filter(|o| !is_denied(&o.path) && read_permission(&o.path).is_ok())
            .count();
        assert!(usable > 60, "可用只读接口过少: {usable}");
    }

    #[test]
    fn template_matching() {
        assert!(matches_template(
            "/api/docker/containers/{id}",
            "/api/docker/containers/abc"
        ));
        assert!(!matches_template(
            "/api/docker/containers/{id}",
            "/api/docker/containers"
        ));
        assert!(!matches_template(
            "/api/docker/containers/{id}",
            "/api/docker/containers/a/b"
        ));
        assert!(matches_template("/api/websites", "/api/websites/"));
    }

    #[test]
    fn path_validation() {
        assert!(validate_path("/api/docker/containers?all=true").is_ok());
        for bad in [
            "api/x",
            "/health",
            "/api/../etc",
            "/api/x/%2e%2e/y",
            "/api/x%2Fy",
            "/api/a b",
            "/api/a#b",
            "/api//x",
        ] {
            assert!(validate_path(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn pem_redaction() {
        let mut v =
            json!({"cert": {"key_pem": "-----BEGIN PRIVATE KEY-----\nabc"}, "list": ["ok"]});
        redact_pem(&mut v);
        assert_eq!(v["cert"]["key_pem"], "***");
        assert_eq!(v["list"][0], "ok");
    }
}
