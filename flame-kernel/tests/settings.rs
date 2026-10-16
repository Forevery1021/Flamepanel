//! 设置批量原子写
//!
//! 从巨型 `integration_test.rs` 按领域拆分而来（S12），复用 `common` 共享工具。

#![allow(unused_imports)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

mod common;
use common::{auth_header, bad_auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::api::{
    routes,
    types::{AppState, PaginationParams, Services},
};
use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
use flame_kernel::application::setup_service::SetupService;
use flame_kernel::application::task_service::TaskService;
use flame_kernel::config::AppConfig;
use flame_kernel::core::error::AppError;
use flame_kernel::domain::entity::*;
use flame_kernel::domain::repository::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::runtime::task_state::TaskTracker;
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::utils::jwt::JwtUtils;
use flame_kernel::utils::password::PasswordUtils;
use flame_kernel::FlameKernel;

#[tokio::test]
async fn test_settings_batch_update_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 批量写入多个键（一次事务原子）
    let body = serde_json::to_vec(&json!({
        "settings": [
            ["panel_name", "FlamePanel-Pro"],
            ["theme", "dark"],
            ["menu_accordion", "true"],
        ]
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/settings/batch")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 验证全部键已生效
    for (key, expected) in [
        ("panel_name", "FlamePanel-Pro"),
        ("theme", "dark"),
        ("menu_accordion", "true"),
    ] {
        let g = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/settings/{key}"))
                    .header(h.clone(), v.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(g.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(g.into_body(), usize::MAX)
            .await
            .unwrap();
        let entry: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(entry["value"], expected, "key {key} 应生效");
    }
}

#[tokio::test]
async fn test_settings_batch_update_rejects_empty() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let body = serde_json::to_vec(&json!({ "settings": [] })).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/settings/batch")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ───────────────────────── W4-a：类型校验 / 保留键 / 访问控制 ─────────────────────────

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
    extra: &[(&str, &str)],
) -> (StatusCode, serde_json::Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    for (k, v) in extra {
        b = b.header(*k, *v);
    }
    let req = match body {
        Some(v) => b
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

fn admin() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

#[tokio::test]
async fn settings_values_are_type_checked_and_all_errors_listed() {
    let app = setup_full_router().await;
    let t = admin();
    let (s, body) = send(
        &app,
        Method::PATCH,
        "/api/settings/batch",
        Some(&t),
        Some(json!({"settings": [["panel_port", "abc"], ["log_level", "loud"], ["panel_name", "ok"]]})),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let msg = body["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("panel_port") && msg.contains("log_level"),
        "{msg}"
    );

    // 原子：任何一项不合法则整批不写
    let (_, got) = send(
        &app,
        Method::GET,
        "/api/settings/panel_name",
        Some(&t),
        None,
        &[],
    )
    .await;
    assert_eq!(got["value"], "FlamePanel");

    // 合法值被规范化后写入
    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "session_timeout_minutes", "value": " 120 "})),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, got) = send(
        &app,
        Method::GET,
        "/api/settings/session_timeout_minutes",
        Some(&t),
        None,
        &[],
    )
    .await;
    assert_eq!(got["value"], "120");
}

#[tokio::test]
async fn reserved_keys_cannot_be_written_or_read_via_settings_api() {
    let (router, state) = setup_router().await;
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    // 模拟旧版本残留（内核内部写入不经校验）
    state
        .settings_service
        .set("jwt_secret_persisted", "leaked-secret")
        .await
        .unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let t = admin();

    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "jwt_secret", "value": "x".repeat(64)})),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, _) = send(
        &app,
        Method::GET,
        "/api/settings/jwt_secret_persisted",
        Some(&t),
        None,
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, list) = send(
        &app,
        Method::GET,
        "/api/settings?page_size=200",
        Some(&t),
        None,
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let leaked = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["key"] == "jwt_secret_persisted");
    assert!(!leaked, "reserved keys must not be listed");
}

#[tokio::test]
async fn access_info_requires_auth_and_settings_read() {
    let (router, state) = setup_router().await;
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (s, _) = send(&app, Method::GET, "/api/settings/access", None, None, &[]).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (s, body) = send(
        &app,
        Method::GET,
        "/api/settings/access",
        Some(&admin()),
        None,
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["local"], true, "oneshot 请求的对端是回环且无转发头");
    assert_eq!(body["session_timeout_minutes"], 1440);

    // viewer 角色的 settings:read 与现有 GET /api/settings 同口径
    let viewer = flame_kernel::utils::jwt::JwtUtils::new("test-secret", 24)
        .sign(2)
        .unwrap();
    let (s_list, _) = send(&app, Method::GET, "/api/settings", Some(&viewer), None, &[]).await;
    let (s_access, _) = send(
        &app,
        Method::GET,
        "/api/settings/access",
        Some(&viewer),
        None,
        &[],
    )
    .await;
    assert_eq!(s_access, s_list);

    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&viewer),
        Some(json!({"key": "panel_name", "value": "x"})),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "viewer 无 settings:update");
}

#[tokio::test]
async fn allowed_ips_blocks_other_clients_but_never_locks_out_the_saver_or_localhost() {
    let app = setup_full_router().await;
    let t = admin();
    // 经本机反代进来的真实客户端（回环对端 + XFF）
    let me = [("X-Forwarded-For", "203.0.113.5")];
    let other = [("X-Forwarded-For", "198.51.100.1")];

    // 保存后会把自己锁在门外 → 400，且不写入
    let (s, body) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "allowed_ips", "value": "10.0.0.0/8"})),
        &me,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap().contains("203.0.113.5"));

    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "allowed_ips", "value": "203.0.113.0/24"})),
        &me,
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = send(
        &app,
        Method::GET,
        "/api/settings/access",
        Some(&t),
        None,
        &me,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    // 未授权来源：连登录接口都被拒绝，错误码可区分
    let (s, body) = send(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"username": "admin", "password": "x"})),
        &other,
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "ACCESS_DENIED");
    // 存活探针不受限
    let (s, _) = send(&app, Method::GET, "/health", None, None, &other).await;
    assert_eq!(s, StatusCode::OK);
    // 本机直连（无转发头）始终放行：自救通道
    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "allowed_ips", "value": ""})),
        &[],
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = send(
        &app,
        Method::GET,
        "/api/settings/access",
        Some(&t),
        None,
        &other,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "清空后恢复不限制");
}

#[tokio::test]
async fn bind_domains_rejects_other_hosts() {
    let app = setup_full_router().await;
    let t = admin();
    let via_domain = [
        ("X-Forwarded-For", "203.0.113.5"),
        ("Host", "panel.example.invalid"),
    ];
    let via_ip = [
        ("X-Forwarded-For", "203.0.113.5"),
        ("Host", "192.0.2.10:8080"),
    ];

    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "bind_domains", "value": "other.example.invalid"})),
        &via_domain,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "会锁定当前访问域名");

    let (s, _) = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&t),
        Some(json!({"key": "bind_domains", "value": "panel.example.invalid"})),
        &via_domain,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, body) = send(
        &app,
        Method::GET,
        "/api/settings/access",
        Some(&t),
        None,
        &via_ip,
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "ACCESS_DENIED");
    let (s, _) = send(
        &app,
        Method::GET,
        "/api/settings/access",
        Some(&t),
        None,
        &via_domain,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
}
