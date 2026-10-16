//! WAF 基础防护 REST 集成测试（主线三 §6.2）
//!
//! 覆盖：规则 CRUD 流 / 校验拒绝 / 拦截日志查询与清理 / 引擎诊断 / 权限拦截
//! / 拦截通知（`WafBlocked` 事件经共享事件总线发布）。
//! 注：inspect 为进程内评估入口，HTTP 层面暴露的是规则与日志管理端点；
//! 评估逻辑（SQLi/XSS/CC/IP·UA）由 webserver::waf 与 waf_service 单测覆盖。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

mod common;
use common::{auth_header, bad_auth_header, setup_full_router, setup_router};

async fn create_rule(
    app: &axum::Router,
    token: (axum::http::HeaderName, String),
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/waf/rules")
                .header(token.0, token.1)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap_or(serde_json::Value::Null);
    (status, body)
}

#[tokio::test]
async fn test_waf_rule_crud_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 创建：非法规则类型拒绝
    let (status, _) = create_rule(
        &app,
        (h.clone(), v.clone()),
        json!({
            "name": "bad", "rule_type": "rce", "pattern": "x"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 创建：特征类规则 pattern 可空
    let (status, body) = create_rule(
        &app,
        (h.clone(), v.clone()),
        json!({
            "name": "sqli", "rule_type": "sql_injection", "action": "block"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let _id = body["id"].as_i64().unwrap();

    // 创建：cc 规则带参数（config_json 为 JSON 字符串）
    let (status, body) = create_rule(
        &app,
        (h.clone(), v.clone()),
        json!({
            "name": "cc", "rule_type": "cc",
            "config_json": json!({"threshold": 60, "window_secs": 30}).to_string(),
            "action": "block"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cc 创建应成功: {body}");
    let _cc_id = body["id"].as_i64().unwrap();

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/waf/rules")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);

    // 站点维度过滤（website_id=5 无规则）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/waf/rules?website_id=5")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);

    // get / update（切换动作 + 停用）
    let (status, body) = create_rule(
        &app,
        (h.clone(), v.clone()),
        json!({
            "name": "bl", "rule_type": "ip_blacklist", "pattern": "1.2.3.4", "action": "log"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let bl_id = body["id"].as_i64().unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/waf/rules/{bl_id}"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"action": "block", "enabled": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["action"], "block");
    assert_eq!(body["enabled"], false);

    // 引擎诊断：停用 1 条 + 3 条创建中 1 条已删 → 启用规则数
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/waf/status")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let status_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(status_body["loaded_rules"].as_u64().unwrap(), 2);
    // 测试装配未注入下发端口、未运行同步任务
    assert_eq!(status_body["enforcement"], false);
    assert_eq!(status_body["ingest"], serde_json::json!([]));

    // delete
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/waf/rules/{bl_id}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 删除后再取 → 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/waf/rules/{bl_id}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_waf_logs_flow_and_clear() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 日志为空
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/waf/logs")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["total"].as_i64().unwrap(), 0);

    // 清理空日志
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/waf/logs/clear")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body.as_i64().unwrap(), 0);
}

#[tokio::test]
async fn test_waf_endpoints_reject_anonymous() {
    let app = setup_full_router().await;
    for uri in ["/api/waf/rules", "/api/waf/logs", "/api/waf/status"] {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{uri} 应未认证拒绝");
    }
    // 非法 token 同样拒绝
    let (h, v) = bad_auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/waf/rules")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// §6.2 拦截通知：阻断命中经共享事件总线发布 `WafBlocked`（组合根 wire-up 校验）。
///
/// 走完整装配（`setup_router`）而非裸服务实例：验证组合根把事件总线注入
/// WAF 服务——否则「阻断发生但无人收到通知」的接线回归将无法被测试捕获。
#[tokio::test]
async fn test_waf_blocked_publishes_notification_event() {
    let (app, state) = setup_router().await;
    let (h, v) = auth_header();
    let mut rx = state.event_bus.subscribe();

    // 经 API 创建阻断规则（动作必须显式 block：默认 log 仅告警不通知）
    let (status, _body) = create_rule(
        &app,
        (h, v),
        json!({
            "name": "bl", "rule_type": "ip_blacklist",
            "pattern": "9.9.9.9", "action": "block"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let blocked = state
        .waf_service
        .inspect(
            4,
            &flame_kernel::webserver::waf::WafRequest {
                method: "GET".into(),
                path: "/admin".into(),
                client_ip: "9.9.9.9".into(),
                user_agent: "curl".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(!blocked, "黑名单 IP 应被阻断");

    let event = rx.try_recv().expect("阻断命中应发布 WafBlocked 事件");
    match event {
        flame_kernel::domain::entity::DomainEvent::WafBlocked {
            website_id,
            rule_name,
            client_ip,
            hit_count,
            ..
        } => {
            assert_eq!(website_id, 4);
            assert_eq!(rule_name, "bl");
            assert_eq!(client_ip, "9.9.9.9");
            assert_eq!(hit_count, 1);
        }
        other => panic!("非预期事件: {other:?}"),
    }
}

/// `POST /api/waf/apply`（Doc/32 W3）：401 未认证 / 403 viewer 无 waf:update /
/// 503 未注入 Web 引擎下发端口（测试装配不触碰宿主 Nginx）。
#[tokio::test]
async fn test_waf_apply_endpoint_guards() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = flame_kernel::utils::jwt::JwtUtils::new("test-secret", 24)
        .sign(2)
        .unwrap();
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let app = flame_kernel::api::middleware::add_middleware_with_mock_peer(router, state);
    let post = |auth: Option<String>| {
        let mut b = Request::builder().method("POST").uri("/api/waf/apply");
        if let Some(t) = auth {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        b.body(Body::empty()).unwrap()
    };
    let res = app.clone().oneshot(post(None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = app.clone().oneshot(post(Some(viewer))).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "viewer 无 waf:update");
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/waf/apply")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
