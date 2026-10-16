//! M-AI-1 内置 MCP Server 集成测试（Doc/30 §M-AI-1）。
//!
//! 覆盖：AI 接入中心 REST（401 / viewer 403 / 签发 201 / 吊销他人 403）、
//! MCP 协议端点（凭证鉴权 401、JWT 不可用、initialize 会话头、通知 202、tools/list 按权限裁剪、
//! tools/call 成功 / 工具级错误 / 协议错误、会话 404 / DELETE 终止、GET 405、Origin 403、
//! 吊销后立即失效）、调用审计落库；M-AI-2 写操作审批（allow_write 闸门、预览生成审批单、
//! 批准 / 拒绝 / 重复处理 409、审批 REST 401 / 403、高危工具需 mcp:delete）。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::api::types::AppState;
use flame_kernel::utils::jwt::JwtUtils;

fn admin_jwt() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

struct Resp {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    json: Value,
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    extra: &[(&str, &str)],
    body: Option<Value>,
) -> Resp {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    for (k, v) in extra {
        req = req.header(*k, *v);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    };
    let res = app.clone().oneshot(req.unwrap()).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    Resp {
        status,
        headers,
        json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

async fn app() -> (Router, AppState) {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    (app, state)
}

/// 经 REST 签发凭证，返回 (id, token)。
async fn issue(app: &Router, jwt: &str, name: &str) -> (i64, String) {
    let r = send(
        app,
        Method::POST,
        "/api/mcp/principals",
        Some(jwt),
        &[],
        Some(json!({"name": name})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.json);
    (
        r.json["principal"]["id"].as_i64().unwrap(),
        r.json["token"].as_str().unwrap().to_string(),
    )
}

async fn rpc(app: &Router, token: &str, session: Option<&str>, body: Value) -> Resp {
    let mut extra = vec![("accept", "application/json, text/event-stream")];
    if let Some(s) = session {
        extra.push(("mcp-session-id", s));
    }
    send(
        app,
        Method::POST,
        "/api/mcp",
        Some(token),
        &extra,
        Some(body),
    )
    .await
}

async fn initialize(app: &Router, token: &str) -> String {
    let r = rpc(
        app,
        token,
        None,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "0.1"},
        }}),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    assert_eq!(r.json["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(r.json["result"]["serverInfo"]["name"], "flamepanel");
    assert!(r.json["result"]["capabilities"]["tools"].is_object());
    r.headers
        .get("mcp-session-id")
        .expect("session header")
        .to_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn management_endpoints_require_auth() {
    let (app, _) = app().await;
    for (m, p) in [
        (Method::GET, "/api/mcp/tools"),
        (Method::GET, "/api/mcp/principals"),
        (Method::POST, "/api/mcp/principals"),
        (Method::POST, "/api/mcp/principals/1/revoke"),
        (Method::GET, "/api/mcp/sessions"),
        (Method::GET, "/api/mcp/calls"),
    ] {
        let r = send(&app, m.clone(), p, None, &[], Some(json!({"name": "x"}))).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{m} {p}");
    }
}

#[tokio::test]
async fn viewer_can_read_but_not_issue() {
    let (app, state) = app().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    for p in [
        "/api/mcp/tools",
        "/api/mcp/principals",
        "/api/mcp/sessions",
        "/api/mcp/calls",
    ] {
        let r = send(&app, Method::GET, p, Some(&viewer), &[], None).await;
        assert_eq!(r.status, StatusCode::OK, "{p}");
    }
    let r = send(
        &app,
        Method::POST,
        "/api/mcp/principals",
        Some(&viewer),
        &[],
        Some(json!({"name": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = send(
        &app,
        Method::POST,
        "/api/mcp/principals/1/revoke",
        Some(&viewer),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn issue_lists_without_token_and_validates() {
    let (app, _) = app().await;
    let jwt = admin_jwt();
    let (id, token) = issue(&app, &jwt, "claude-desktop").await;
    assert!(token.starts_with("fpk_"));
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/principals",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    let list = r.json.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], id);
    assert_eq!(list[0]["owner"], "admin");
    assert_eq!(list[0]["active"], true);
    assert!(list[0].get("token_hash").is_none());
    assert!(!r.json.to_string().contains(&token[12..]));

    let r = send(
        &app,
        Method::POST,
        "/api/mcp/principals",
        Some(&jwt),
        &[],
        Some(json!({"name": "", "expires_in_days": 0})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    let r = send(&app, Method::GET, "/api/mcp/tools", Some(&jwt), &[], None).await;
    let tools = r.json.as_array().unwrap();
    assert_eq!(tools.len(), 24);
    assert!(tools.iter().all(|t| t["allowed"] == true));
    assert!(tools
        .iter()
        .any(|t| t["name"] == "docker_container_remove" && t["risk"] == "destructive"));
    assert!(tools
        .iter()
        .any(|t| t["name"] == "docker_logs" && t["permission"] == "docker:read"));
}

#[tokio::test]
async fn protocol_endpoint_rejects_missing_jwt_and_bad_credentials() {
    let (app, _) = app().await;
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});
    let r = send(
        &app,
        Method::POST,
        "/api/mcp",
        None,
        &[],
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    // 面板 JWT 不能用于 MCP
    let r = rpc(&app, &admin_jwt(), None, body.clone()).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = rpc(&app, "fpk_0000000000000000", None, body).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn full_protocol_flow_and_audit() {
    let (app, _) = app().await;
    let jwt = admin_jwt();
    let (pid, token) = issue(&app, &jwt, "cursor").await;
    let sid = initialize(&app, &token).await;

    // 通知 → 202 无响应体
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED);

    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": "p", "method": "ping"}),
    )
    .await;
    assert_eq!(r.json["id"], "p");
    assert_eq!(r.json["result"], json!({}));

    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    )
    .await;
    let tools = r.json["result"]["tools"].as_array().unwrap();
    // 只读凭证：12 个只读工具 + approval_get + Batch 2 两个只读工具，写工具不可见
    assert_eq!(tools.len(), 15);
    assert!(tools
        .iter()
        .all(|t| t["annotations"]["readOnlyHint"] == true && t["inputSchema"]["type"] == "object"));

    // 验收场景：查容器最近 100 行日志 + inspect 判断 OOM
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
            "name": "docker_logs", "arguments": {"container": "web", "tail": 100}
        }}),
    )
    .await;
    assert_eq!(r.json["result"]["isError"], false, "{:?}", r.json);
    assert_eq!(r.json["result"]["content"][0]["type"], "text");
    assert!(r.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("logs not available"));

    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
            "name": "docker_inspect", "arguments": {"container": "web"}
        }}),
    )
    .await;
    let text = r.json["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("summary").is_some());
    assert!(parsed["summary"].get("oom_killed").is_some());

    // 工具级错误：缺参数 → isError（不是协议错误）
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {
            "name": "docker_logs", "arguments": {"api_token": "s3cret"}
        }}),
    )
    .await;
    assert_eq!(r.json["result"]["isError"], true);

    // 协议错误：未知工具 / 未知方法
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "rm_rf"}}),
    )
    .await;
    assert_eq!(r.json["error"]["code"], -32602);
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 7, "method": "resources/list"}),
    )
    .await;
    assert_eq!(r.json["error"]["code"], -32601);

    // 批量：请求 + 通知 → 只返回请求的响应
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!([
            {"jsonrpc": "2.0", "id": 8, "method": "ping"},
            {"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {}},
        ]),
    )
    .await;
    assert_eq!(r.json.as_array().unwrap().len(), 1);

    // files_read：读取并截断
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {
            "name": "files_read", "arguments": {"path": "Cargo.toml", "max_bytes": 20}
        }}),
    )
    .await;
    assert_eq!(r.json["result"]["isError"], false, "{:?}", r.json);
    assert!(r.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("已截断"));

    // 审计：调用记录（最新在前）+ 参数脱敏 + 会话关联
    let r = send(
        &app,
        Method::GET,
        &format!("/api/mcp/calls?principal_id={pid}&page_size=50"),
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let calls = r.json["data"].as_array().unwrap();
    // docker_logs×2、docker_inspect、files_read（未知工具不入审计）
    assert_eq!(r.json["total"], 4, "{:?}", r.json);
    assert_eq!(calls[0]["tool"], "files_read");
    assert!(calls.iter().all(|c| c["session_id"] == sid.as_str()));
    let failed = calls.iter().find(|c| c["ok"] == false).unwrap();
    assert!(failed["arguments"].as_str().unwrap().contains("***"));
    assert!(!failed["arguments"].as_str().unwrap().contains("s3cret"));

    let r = send(
        &app,
        Method::GET,
        "/api/mcp/sessions",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.json[0]["id"], sid.as_str());
    assert_eq!(r.json[0]["client_name"], "test-client");

    // DELETE 终止会话 → 再用该会话 404
    let r = send(
        &app,
        Method::DELETE,
        "/api/mcp",
        Some(&token),
        &[("mcp-session-id", &sid)],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = rpc(
        &app,
        &token,
        Some(&sid),
        json!({"jsonrpc": "2.0", "id": 10, "method": "ping"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn protocol_edge_cases() {
    let (app, _) = app().await;
    let (_, token) = issue(&app, &admin_jwt(), "edge").await;

    let r = send(&app, Method::GET, "/api/mcp", Some(&token), &[], None).await;
    assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED);

    let r = rpc(
        &app,
        &token,
        Some("no-such-session"),
        json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // 无会话头：无状态模式可用
    let r = rpc(
        &app,
        &token,
        None,
        json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    let r = send(
        &app,
        Method::POST,
        "/api/mcp",
        Some(&token),
        &[("content-type", "application/json")],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["error"]["code"], -32700);

    let r = send(
        &app,
        Method::POST,
        "/api/mcp",
        Some(&token),
        &[
            ("origin", "https://evil.example"),
            ("host", "panel.example"),
        ],
        Some(json!({"jsonrpc": "2.0", "id": 1, "method": "ping"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tools_follow_owner_role_and_revocation() {
    let (app, state) = app().await;
    // 无任何权限的自定义角色用户：tools/list 为空，调用返回工具级 403
    let user = state
        .user_service
        .create_user("bot", "hash", "nobody")
        .await
        .unwrap();
    let issued = state.mcp_service.issue(user.id, "bot", None).await.unwrap();
    let r = rpc(
        &app,
        &issued.token,
        None,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
    )
    .await;
    assert_eq!(r.json["result"]["tools"], json!([]));
    let r = rpc(
        &app,
        &issued.token,
        None,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "docker_list", "arguments": {}
        }}),
    )
    .await;
    assert_eq!(r.json["result"]["isError"], true);
    assert!(r.json["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("docker:read"));

    // operator 不能吊销管理员的凭证（缺 mcp:delete），管理员可以
    let (admin_pid, admin_token) = issue(&app, &admin_jwt(), "admin-cred").await;
    state
        .user_service
        .create_user("op", "hash", "operator")
        .await
        .unwrap();
    let op = state
        .user_service
        .list_users()
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.username == "op")
        .unwrap();
    let op_jwt = JwtUtils::new("test-secret", 24).sign(op.id).unwrap();
    let path = format!("/api/mcp/principals/{admin_pid}/revoke");
    let r = send(&app, Method::POST, &path, Some(&op_jwt), &[], None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    // operator 可以吊销自己的凭证
    let (op_pid, _) = issue(&app, &op_jwt, "op-cred").await;
    let r = send(
        &app,
        Method::POST,
        &format!("/api/mcp/principals/{op_pid}/revoke"),
        Some(&op_jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["active"], false);

    let r = send(&app, Method::POST, &path, Some(&admin_jwt()), &[], None).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = rpc(
        &app,
        &admin_token,
        None,
        json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    let r = send(
        &app,
        Method::POST,
        "/api/mcp/principals/999/revoke",
        Some(&admin_jwt()),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

async fn call_tool(app: &Router, token: &str, name: &str, args: Value) -> (bool, String) {
    let r = rpc(
        app,
        token,
        None,
        json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {
            "name": name, "arguments": args
        }}),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    (
        r.json["result"]["isError"] == true,
        r.json["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

#[tokio::test]
async fn approval_endpoints_require_auth_and_permissions() {
    let (app, state) = app().await;
    for (m, p) in [
        (Method::GET, "/api/mcp/approvals"),
        (Method::GET, "/api/mcp/approvals/1"),
        (Method::POST, "/api/mcp/approvals/1/approve"),
        (Method::POST, "/api/mcp/approvals/1/reject"),
    ] {
        let r = send(&app, m, p, None, &[], None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{p}");
    }
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/approvals",
        Some(&viewer),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["total"], 0);
    for p in [
        "/api/mcp/approvals/1/approve",
        "/api/mcp/approvals/1/reject",
    ] {
        let r = send(&app, Method::POST, p, Some(&viewer), &[], None).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{p}");
    }
    let jwt = admin_jwt();
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/approvals/42",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/approvals?status=bogus",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn write_tools_go_through_approval() {
    let (app, state) = app().await;
    let jwt = admin_jwt();

    // 只读凭证：写工具不可见、调用被拒，且不产生审批单
    let (_, ro_token) = issue(&app, &jwt, "ro").await;
    let (err, text) = call_tool(
        &app,
        &ro_token,
        "docker_container_stop",
        json!({"container": "web"}),
    )
    .await;
    assert!(err && text.contains("未开启写操作"), "{text}");

    // 可写凭证
    let r = send(
        &app,
        Method::POST,
        "/api/mcp/principals",
        Some(&jwt),
        &[],
        Some(json!({"name": "rw", "allow_write": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.json);
    assert_eq!(r.json["principal"]["allow_write"], true);
    let rw_pid = r.json["principal"]["id"].as_i64().unwrap();
    let rw = r.json["token"].as_str().unwrap().to_string();
    let r = rpc(
        &app,
        &rw,
        None,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
    )
    .await;
    let tools = r.json["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 24);
    assert!(tools.iter().any(|t| t["name"] == "docker_container_remove"
        && t["annotations"]["destructiveHint"] == true
        && t["annotations"]["readOnlyHint"] == false));

    // 预览失败（参数 / 目标不存在）不生成审批单
    let (err, _) = call_tool(&app, &rw, "website_deploy", json!({"id": 999})).await;
    assert!(err);
    let (err, _) = call_tool(&app, &rw, "docker_container_stop", json!({})).await;
    assert!(err);
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/approvals",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.json["total"], 0, "{:?}", r.json);

    // 写工具调用 → 审批单（未执行）
    let (err, text) = call_tool(
        &app,
        &rw,
        "docker_container_stop",
        json!({"container": "web", "timeout": 5}),
    )
    .await;
    assert!(!err, "{text}");
    assert!(
        text.contains("审批单 #") && text.contains("尚未执行"),
        "{text}"
    );
    let (_, text2) = call_tool(
        &app,
        &rw,
        "docker_container_restart",
        json!({"container": "web"}),
    )
    .await;
    assert!(text2.contains("审批单 #"), "{text2}");

    let r = send(
        &app,
        Method::GET,
        "/api/mcp/approvals?status=pending",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["total"], 2, "{:?}", r.json);
    let items = r.json["data"].as_array().unwrap();
    assert!(items.iter().all(|a| a.get("raw_arguments").is_none()));
    let restart_id = items[0]["id"].as_i64().unwrap();
    let stop_id = items[1]["id"].as_i64().unwrap();
    assert_eq!(items[1]["tool"], "docker_container_stop");
    assert_eq!(items[1]["risk"], "write");
    assert_eq!(items[1]["principal_id"], rw_pid);

    // 批准 → 执行（测试环境无 Docker 时为 failed，均为终态）；重复批准 409
    let path = format!("/api/mcp/approvals/{stop_id}/approve");
    let r = send(&app, Method::POST, &path, Some(&jwt), &[], None).await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    let st = r.json["status"].as_str().unwrap();
    assert!(st == "succeeded" || st == "failed", "{st}");
    assert!(r.json["decided_by"].is_number());
    let r = send(&app, Method::POST, &path, Some(&jwt), &[], None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    // 拒绝（附理由）→ AI 经 approval_get 可见；再拒绝 409
    let rpath = format!("/api/mcp/approvals/{restart_id}/reject");
    let r = send(
        &app,
        Method::POST,
        &rpath,
        Some(&jwt),
        &[],
        Some(json!({"reason": "业务高峰，稍后再说"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    assert_eq!(r.json["status"], "rejected");
    let r = send(&app, Method::POST, &rpath, Some(&jwt), &[], None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    let (err, text) = call_tool(&app, &rw, "approval_get", json!({"id": restart_id})).await;
    assert!(
        !err && text.contains("rejected") && text.contains("业务高峰"),
        "{text}"
    );
    // 其他凭证看不到该审批单
    let (err, _) = call_tool(&app, &ro_token, "approval_get", json!({"id": restart_id})).await;
    assert!(err);

    // 审计：批准执行记入工具调用表
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/calls?tool=docker_container_stop",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    assert!(r.json["total"].as_i64().unwrap() >= 2, "{:?}", r.json);

    // 高危工具：operator（无 mcp:delete / docker:delete）批准被拒 403
    let (err, text) = call_tool(
        &app,
        &rw,
        "docker_container_remove",
        json!({"container": "web", "force": true}),
    )
    .await;
    if !err {
        let id: i64 = text
            .split('#')
            .nth(1)
            .and_then(|t| t.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|n| n.parse().ok())
            .unwrap();
        let op = state
            .user_service
            .create_user("op", "hash", "operator")
            .await
            .unwrap();
        let op_jwt = JwtUtils::new("test-secret", 24).sign(op.id).unwrap();
        let r = send(
            &app,
            Method::POST,
            &format!("/api/mcp/approvals/{id}/approve"),
            Some(&op_jwt),
            &[],
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{:?}", r.json);
        // 吊销凭证后批准 409
        state.mcp_service.revoke(rw_pid).await.unwrap();
        let r = send(
            &app,
            Method::POST,
            &format!("/api/mcp/approvals/{id}/approve"),
            Some(&jwt),
            &[],
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{:?}", r.json);
    }
}

/// M-AI-2 Batch 2：备份 / 定时任务 / 原生应用工具——只读列表脱敏、预览校验目标、
/// 定时任务执行为高危（批准后同步返回输出）、备份经审批执行。
#[tokio::test]
async fn batch2_tools_follow_approval_gate() {
    let (app, _) = app().await;
    let jwt = admin_jwt();
    for (name, command) in [
        ("hello", "echo mcp-batch2-ok"),
        (
            "dump",
            "mysqldump -uroot -ps3cret-value DB_PASSWORD=s3cret-value db",
        ),
    ] {
        let r = send(
            &app,
            Method::POST,
            "/api/scheduled-tasks",
            Some(&jwt),
            &[],
            Some(json!({"name": name, "command": command, "schedule": "0 3 * * *", "enabled": false})),
        )
        .await;
        assert!(r.status.is_success(), "{:?}", r.json);
    }
    let r = send(
        &app,
        Method::POST,
        "/api/mcp/principals",
        Some(&jwt),
        &[],
        Some(json!({"name": "rw2", "allow_write": true})),
    )
    .await;
    let rw = r.json["token"].as_str().unwrap().to_string();

    // 只读：定时任务列表中的密码类参数已脱敏
    let (err, text) = call_tool(&app, &rw, "scheduled_task_list", json!({})).await;
    assert!(!err, "{text}");
    assert!(
        text.contains("mcp-batch2-ok") && text.contains("-p***"),
        "{text}"
    );
    assert!(!text.contains("s3cret-value"), "{text}");
    let list: Value = serde_json::from_str(&text).unwrap();
    let hello_id = list["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "hello")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let (err, text) = call_tool(&app, &rw, "backup_job_list", json!({})).await;
    assert!(!err && text.contains("\"count\": 0"), "{text}");

    // 预览校验目标：不存在的应用 / 备份任务 / 定时任务直接报错，不生成审批单
    for (tool, args) in [
        ("app_native_restart", json!({"id": 999})),
        ("backup_job_run", json!({"id": 999})),
        ("scheduled_task_run", json!({"id": 999})),
    ] {
        let (err, text) = call_tool(&app, &rw, tool, args).await;
        assert!(err, "{tool}: {text}");
    }

    // 定时任务执行：预览展示（脱敏后的）命令；高危；批准后同步返回输出
    let (err, text) = call_tool(&app, &rw, "scheduled_task_run", json!({"id": hello_id})).await;
    assert!(
        !err && text.contains("审批单 #") && text.contains("echo mcp-batch2-ok"),
        "{text}"
    );
    let (err, text) = call_tool(&app, &rw, "backup_create", json!({})).await;
    assert!(!err && text.contains("审批单 #"), "{text}");

    let r = send(
        &app,
        Method::GET,
        "/api/mcp/approvals?status=pending",
        Some(&jwt),
        &[],
        None,
    )
    .await;
    let items = r.json["data"].as_array().unwrap().clone();
    assert_eq!(items.len(), 2, "{:?}", r.json);
    let find = |tool: &str| {
        items
            .iter()
            .find(|a| a["tool"] == tool)
            .unwrap_or_else(|| panic!("{tool}"))
            .clone()
    };
    let run = find("scheduled_task_run");
    assert_eq!(run["risk"], "destructive");
    let backup = find("backup_create");
    assert_eq!(backup["risk"], "write");

    let path = format!("/api/mcp/approvals/{}/approve", run["id"]);
    let r = send(&app, Method::POST, &path, Some(&jwt), &[], None).await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    assert_eq!(r.json["status"], "succeeded", "{:?}", r.json);
    assert!(
        r.json["result"].as_str().unwrap().contains("mcp-batch2-ok"),
        "{:?}",
        r.json
    );

    // 备份：测试环境的数据库路径可能不存在，执行结果为终态即可
    let path = format!("/api/mcp/approvals/{}/approve", backup["id"]);
    let r = send(&app, Method::POST, &path, Some(&jwt), &[], None).await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    let st = r.json["status"].as_str().unwrap();
    assert!(st == "succeeded" || st == "failed", "{st}");
}

/// Prompts 运维技能库：REST 列表 / 渲染（三件套）+ MCP prompts/list、prompts/get。
#[tokio::test]
async fn prompts_library_over_rest_and_protocol() {
    let (app, state) = app().await;
    for p in ["/api/mcp/prompts", "/api/mcp/prompts/container_diagnose"] {
        let r = send(&app, Method::GET, p, None, &[], None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{p}");
    }
    let guest = state
        .user_service
        .create_user("guest", "hash", "guest")
        .await
        .unwrap();
    let guest = JwtUtils::new("test-secret", 24).sign(guest.id).unwrap();
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/prompts",
        Some(&guest),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    let viewer = state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(viewer.id).unwrap();
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/prompts",
        Some(&viewer),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let list = r.json.as_array().unwrap();
    assert!(list.len() >= 7);
    let diag = list
        .iter()
        .find(|p| p["name"] == "container_diagnose")
        .unwrap();
    assert_eq!(diag["arguments"][0]["name"], "container");
    assert_eq!(diag["arguments"][0]["required"], true);

    let r = send(
        &app,
        Method::GET,
        "/api/mcp/prompts/container_diagnose?container=web",
        Some(&viewer),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    let text = r.json["text"].as_str().unwrap();
    assert!(
        text.contains("「web」") && text.contains("docker_inspect"),
        "{text}"
    );
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/prompts/container_diagnose",
        Some(&viewer),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = send(
        &app,
        Method::GET,
        "/api/mcp/prompts/nope",
        Some(&viewer),
        &[],
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // MCP 协议
    let (_, token) = issue(&app, &admin_jwt(), "prompts").await;
    let r = rpc(
        &app,
        &token,
        None,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
    )
    .await;
    assert!(
        r.json["result"]["capabilities"]["prompts"].is_object(),
        "{:?}",
        r.json
    );
    let r = rpc(
        &app,
        &token,
        None,
        json!({"jsonrpc": "2.0", "id": 2, "method": "prompts/list"}),
    )
    .await;
    assert!(r.json["result"]["prompts"].as_array().unwrap().len() >= 7);
    let r = rpc(
        &app,
        &token,
        None,
        json!({"jsonrpc": "2.0", "id": 3, "method": "prompts/get",
               "params": {"name": "website_unreachable", "arguments": {"website": "example.com"}}}),
    )
    .await;
    let msg = &r.json["result"]["messages"][0];
    assert_eq!(msg["role"], "user");
    assert!(msg["content"]["text"]
        .as_str()
        .unwrap()
        .contains("example.com"));
    for params in [
        json!({"name": "website_unreachable"}),
        json!({"name": "website_unreachable", "arguments": {"website": 1}}),
        json!({"name": "nope"}),
    ] {
        let r = rpc(
            &app,
            &token,
            None,
            json!({"jsonrpc": "2.0", "id": 4, "method": "prompts/get", "params": params}),
        )
        .await;
        assert_eq!(r.json["error"]["code"], -32602, "{:?}", r.json);
    }
}
