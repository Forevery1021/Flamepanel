//! 防火墙规则写入校验（S-13）：`POST /api/firewall/rules` / `PUT /api/firewall/rules/{id}`。
//!
//! 覆盖：401 / viewer 可读不可建 403 / 非法字段一次性返回全部字段错误（400，且不落库）/
//! 合法输入按规范形态落库（端口段 `:`→`-`、主机位归零、大小写、任意来源）/ 编辑可清空端口 /
//! 编辑时的跨字段校验（改为 icmp 却保留端口）。
//!
//! 测试规则一律 `enabled: false`：不触碰 CI 主机的真实防火墙（写系统命令只发生在启用的规则上）。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::utils::jwt::JwtUtils;

const RULES: &str = "/api/firewall/rules";

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    };
    let res = app.clone().oneshot(req.unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn admin() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

fn disabled(mut rule: Value) -> Value {
    rule["enabled"] = json!(false);
    rule
}

#[tokio::test]
async fn firewall_rules_require_authentication() {
    let app = setup_full_router().await;
    let (status, _) = call(&app, Method::GET, RULES, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &app,
        Method::POST,
        RULES,
        None,
        Some(disabled(json!({ "name": "x" }))),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_rule_returns_every_field_error_and_is_not_stored() {
    let app = setup_full_router().await;
    let token = admin();
    let (_, before) = call(&app, Method::GET, RULES, Some(&token), None).await;
    let seeded = before["total"].as_i64().unwrap();
    let (status, body) = call(
        &app,
        Method::POST,
        RULES,
        Some(&token),
        Some(disabled(json!({
            "name": "  ",
            "protocol": "sctp",
            "port": "9000-8000",
            "source": "1.2.3.4\" accept",
            "action": "drop",
            "direction": "both",
            "priority": 0
        }))),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "VALIDATION_ERROR");
    let msg = body["message"].as_str().unwrap();
    for field in [
        "name:",
        "protocol:",
        "action:",
        "direction:",
        "priority:",
        "port:",
        "source:",
    ] {
        assert!(msg.contains(field), "missing {field} in {msg}");
    }

    let (status, body) = call(&app, Method::GET, RULES, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["total"], seeded,
        "invalid rule must not be stored: {body}"
    );
}

#[tokio::test]
async fn valid_rule_is_stored_in_canonical_form_and_editable() {
    let app = setup_full_router().await;
    let token = admin();
    let (status, created) = call(
        &app,
        Method::POST,
        RULES,
        Some(&token),
        Some(disabled(json!({
            "name": "  web  ",
            "protocol": "TCP",
            "port": "80, 443,8000:9000",
            "source": "192.168.1.10/24",
            "action": "Allow",
            "direction": "IN",
            "priority": 10
        }))),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["name"], "web");
    assert_eq!(created["protocol"], "tcp");
    assert_eq!(created["port"], "80,443,8000-9000");
    assert_eq!(created["source"], "192.168.1.0/24");
    assert_eq!(created["action"], "allow");
    assert_eq!(created["direction"], "in");
    let id = created["id"].as_i64().unwrap();
    let path = format!("{RULES}/{id}");

    // 跨字段：改成 icmp 但保留端口 → 400，原规则不变
    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({ "protocol": "icmp" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["message"].as_str().unwrap().contains("port:"),
        "{body}"
    );

    // 清空端口 + 任意来源 + 改协议 → 规范化落库
    let (status, updated) = call(
        &app,
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({ "protocol": "icmp", "port": "", "source": "any", "description": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["protocol"], "icmp");
    assert!(updated["port"].is_null(), "{updated}");
    assert_eq!(updated["source"], "0.0.0.0/0");
    assert!(updated["description"].is_null());

    let (status, got) = call(&app, Method::GET, &path, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["protocol"], "icmp");
    assert!(got["port"].is_null());
}

#[tokio::test]
async fn viewer_can_read_but_cannot_create_rules() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, body) = call(&app, Method::GET, RULES, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "viewer 持有 firewall:read → {body}");
    let (status, _) = call(
        &app,
        Method::POST,
        RULES,
        Some(&token),
        Some(disabled(json!({ "name": "ssh", "port": "22" }))),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "viewer 无 firewall:create");
}
