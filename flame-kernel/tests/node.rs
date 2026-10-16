//! 节点心跳
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
async fn test_node_heartbeat_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 注册节点（带 auth_token）
    let body = serde_json::json!({
        "node": {
            "id": 0,
            "name": "hb-01",
            "hostname": "hb-01.example.com",
            "ip_address": "10.0.0.9",
            "status": "online",
            "created_at": "2026-01-01T00:00:00Z",
            "auth_token": "agent-secret-1"
        }
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let node_id: i64 = serde_json::from_slice(&bytes).unwrap();

    // 心跳（白名单免 JWT，携带正确 Agent token）
    let hb = serde_json::json!({
        "cpu_usage": 12.3,
        "memory_usage_percent": 45.6,
        "disk_usage_percent": 67.8,
        "load_one": 0.5
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/nodes/heartbeat/{}", node_id))
                .header("Content-Type", "application/json")
                .header(header::AUTHORIZATION, "Bearer agent-secret-1")
                .body(Body::from(serde_json::to_string(&hb).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 心跳后 status = online
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/nodes/{}/status", node_id))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status["status"], "online");

    // 指标快照可查询
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/nodes/{}/metrics", node_id))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let metrics: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let cpu = metrics["cpu_usage"].as_f64().unwrap();
    assert!((cpu - 12.3).abs() < 0.01, "cpu_usage={} not ~12.3", cpu);
}

#[tokio::test]
async fn test_node_heartbeat_token_mismatch_rejected() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let body = serde_json::json!({
        "node": {
            "id": 0,
            "name": "hb-02",
            "hostname": "hb-02.example.com",
            "ip_address": "10.0.0.10",
            "status": "online",
            "created_at": "2026-01-01T00:00:00Z",
            "auth_token": "real-token"
        }
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let node_id: i64 = serde_json::from_slice(&bytes).unwrap();

    // 错误 token → 401
    let hb = serde_json::json!({"cpu_usage": 1.0, "memory_usage_percent": 1.0, "disk_usage_percent": 1.0, "load_one": 0.1});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/nodes/heartbeat/{}", node_id))
                .header("Content-Type", "application/json")
                .header(header::AUTHORIZATION, "Bearer wrong-token")
                .body(Body::from(serde_json::to_string(&hb).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}

#[tokio::test]
async fn test_node_heartbeat_without_token_rejected() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 旧 Agent：注册不带 auth_token
    let body = serde_json::json!({
        "node": {
            "id": 0,
            "name": "hb-legacy",
            "hostname": "hb-legacy.example.com",
            "ip_address": "10.0.0.11",
            "status": "online",
            "created_at": "2026-01-01T00:00:00Z"
        }
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let node_id: i64 = serde_json::from_slice(&bytes).unwrap();

    // M7：无 token 心跳 → 拒绝（401），防止伪造节点指标/在线抖动
    let hb = serde_json::json!({"cpu_usage": 5.0, "memory_usage_percent": 5.0, "disk_usage_percent": 5.0, "load_one": 0.2});
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/nodes/heartbeat/{}", node_id))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&hb).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}

/// 带可选 Bearer 的简易请求（返回状态码与 JSON 体）。
async fn call(
    app: &axum::Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

/// Doc/30 M-P：心跳并入每节点 MetricsHub，`/api/nodes/{id}/metrics/history` 返回环形历史；
/// 越界值被钳制，节点删除后历史一并清理。
#[tokio::test]
async fn test_node_metrics_history_via_heartbeat() {
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    let admin = JwtUtils::new("test-secret", 24).sign(1).unwrap();
    let guest = {
        let u = state
            .user_service
            .create_user("nm-guest", "hash", "guest")
            .await
            .unwrap();
        JwtUtils::new("test-secret", 24).sign(u.id).unwrap()
    };

    let (s, id) = call(
        &app,
        Method::POST,
        "/api/nodes",
        Some(&admin),
        Some(json!({"node": {
            "id": 0, "name": "nm-01", "hostname": "nm-01.example.com",
            "ip_address": "10.0.0.31", "status": "online",
            "created_at": "2026-01-01T00:00:00Z", "auth_token": "nm-secret"
        }})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let node_id = id.as_i64().unwrap();
    let history = format!("/api/nodes/{node_id}/metrics/history");

    // 新节点尚无心跳：空历史
    let (s, v) = call(&app, Method::GET, &history, Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 0);

    // 旧协议心跳（仅 4 项）+ 完整心跳（含越界值）
    let hb = format!("/api/nodes/heartbeat/{node_id}");
    let legacy = json!({"cpu_usage": 10.0, "memory_usage_percent": 20.0,
        "disk_usage_percent": 30.0, "load_one": 0.4});
    let full = json!({"cpu_usage": 250.0, "memory_usage_percent": 50.0,
        "disk_usage_percent": 60.0, "load_one": 1.5, "load_five": 1.0,
        "load_fifteen": 0.5, "cpu_cores": 8, "memory_total_mb": 16000,
        "memory_used_mb": 99999, "disk_total_gb": 100.0, "disk_used_gb": 60.0,
        "network_rx_mbps": 2.5, "network_tx_mbps": -1.0});
    for body in [legacy, full] {
        let (s, _) = call(&app, Method::POST, &hb, Some("nm-secret"), Some(body)).await;
        assert_eq!(s, StatusCode::OK);
    }

    let (s, v) = call(&app, Method::GET, &history, Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    let pts = v.as_array().unwrap();
    assert_eq!(pts.len(), 2);
    assert_eq!(pts[0]["cpu_usage"].as_f64().unwrap(), 10.0);
    assert_eq!(pts[0]["cpu_cores"], 0, "旧协议缺省字段记 0");
    let last = &pts[1];
    assert_eq!(last["cpu_usage"].as_f64().unwrap(), 100.0, "百分比钳制");
    assert_eq!(last["cpu_cores"], 8);
    assert_eq!(last["memory_used_mb"], 16000, "已用不超过总量");
    assert_eq!(last["network_tx_mbps"].as_f64().unwrap(), 0.0, "负速率归零");
    assert!(
        last["timestamp"].as_i64().unwrap() > 1_600_000_000_000,
        "毫秒时间戳"
    );

    // 最近快照（/metrics）与历史末项一致（完整字段）
    let (_, m) = call(
        &app,
        Method::GET,
        &format!("/api/nodes/{node_id}/metrics"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(m["cpu_cores"], 8);

    // REST 三件套：401 / 403 / 404
    let (s, _) = call(&app, Method::GET, &history, None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(&app, Method::GET, &history, Some(&guest), None).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "guest 无 node:read");
    let (s, _) = call(
        &app,
        Method::GET,
        "/api/nodes/999999/metrics/history",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // 删除节点 → Hub 释放
    assert!(state.metrics.nodes().hub(node_id).is_some());
    let (s, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/nodes/{node_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(state.metrics.nodes().hub(node_id).is_none());
    let (s, _) = call(&app, Method::GET, &history, Some(&admin), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
