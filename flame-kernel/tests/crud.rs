//! 核心资源 CRUD API（用户/节点/站点）与通用边界
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
async fn test_create_and_list_users() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "username": "testuser", "password_hash": "hashed_pw", "role": "admin"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_and_list_nodes() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let node = json!({"node": {
        "id": 0, "name": "node-1", "hostname": "s1.example.com",
        "ip_address": "10.0.0.1", "status": "online",
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&node).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/nodes")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_and_list_websites() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let ws = json!({"website": {
        "id": 0, "name": "blog", "domain": "blog.example.com",
        "root_path": "/var/www/blog", "status": "active", "node_id": 1,
        "engine": "nginx", "ssl_enabled": false, "proxy_enabled": false,
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/websites")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&ws).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/websites")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_user_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/users/1")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "username": "admin", "role": "operator"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let user: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(user["role"], "operator");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/users/999")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "username": "nobody", "role": "viewer"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_node_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let node = json!({"node": {
        "id": 0, "name": "node-1", "hostname": "s1.example.com",
        "ip_address": "10.0.0.1", "status": "online",
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&node).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    let updated = json!({"node": {
        "id": 0, "name": "node-1-renamed", "hostname": "s1.example.com",
        "ip_address": "10.0.0.2", "status": "offline",
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/nodes/{}", id))
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let node_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(node_res["name"], "node-1-renamed");
    assert_eq!(node_res["ip_address"], "10.0.0.2");
    assert_eq!(node_res["status"], "offline");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/nodes/999")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_website_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let ws = json!({"website": {
        "id": 0, "name": "blog", "domain": "blog.example.com",
        "root_path": "/var/www/blog", "status": "active", "node_id": 1,
        "engine": "nginx", "ssl_enabled": false, "proxy_enabled": false,
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/websites")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&ws).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    let updated = json!({"website": {
        "id": 0, "name": "blog-v2", "domain": "blog.example.com",
        "root_path": "/var/www/blog-v2", "status": "active", "node_id": 1,
        "engine": "caddy", "ssl_enabled": true, "proxy_enabled": true,
        "proxy_pass": "http://127.0.0.1:3000",
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/websites/{}", id))
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let ws_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(ws_res["name"], "blog-v2");
    assert_eq!(ws_res["engine"], "caddy");
    assert_eq!(ws_res["ssl_enabled"], true);
    assert_eq!(ws_res["proxy_enabled"], true);
    assert_eq!(ws_res["proxy_pass"], "http://127.0.0.1:3000");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/websites/999")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_user_empty_body() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    // Should fail with 422 or 400 because required fields are missing
    assert!(res.status().is_client_error());
}

#[tokio::test]
async fn test_invalid_json_body() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from("not-json"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_client_error());
}

#[tokio::test]
async fn test_unknown_route() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/nonexistent")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_wrong_method() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // health is GET only, POST should fail
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/health")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}
