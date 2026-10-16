//! 审计日志 / 操作日志 / outbox / 导出
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
use common::{auth_header, bad_auth_header, bcrypt_hash, setup_full_router, setup_router};

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
async fn test_operation_log_list() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/operation-logs")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_operation_log_create() {
    let repo = Arc::new(InMemoryOperationLogRepository::new());
    let log = repo
        .create(
            "testuser",
            "test.action",
            Some("target-1"),
            Some("127.0.0.1"),
        )
        .await
        .unwrap();
    assert_eq!(log.username, "testuser");
    assert_eq!(log.action, "test.action");
    assert_eq!(log.target.as_deref(), Some("target-1"));
    assert_eq!(log.ip.as_deref(), Some("127.0.0.1"));
    assert!(log.id > 0);

    let list = repo.list().await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn test_operation_log_find_by_username() {
    let repo = Arc::new(InMemoryOperationLogRepository::new());
    repo.create("alice", "create", None, None).await.unwrap();
    repo.create("bob", "delete", Some("x"), None).await.unwrap();
    repo.create("alice", "update", Some("y"), None)
        .await
        .unwrap();

    let alice_logs = repo.list_by_username("alice").await.unwrap();
    assert_eq!(alice_logs.len(), 2);
    assert!(alice_logs.iter().all(|l| l.username == "alice"));
}

#[tokio::test]
async fn test_audit_write_operations_logged() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 执行一个写操作（创建用户）
    let body = serde_json::json!({
        "username": "audit-user",
        "password_hash": "hash123",
        "role": "viewer"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 稍等异步审计落库
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 查询审计日志
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/operation-logs")
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
    let logs: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let items = logs["data"].as_array().unwrap();
    assert!(!items.is_empty(), "audit log should have entries");
    let first = &items[0];
    let action = first["action"].as_str().unwrap();
    assert!(action.starts_with("POST /api/users"), "action={}", action);
    assert_eq!(first["username"], "admin");
}

#[tokio::test]
async fn test_audit_read_operations_not_logged() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // GET 请求不应产生审计
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/operation-logs")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let logs: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let items = logs["data"].as_array().unwrap();
    assert!(
        items
            .iter()
            .all(|l| !l["action"].as_str().unwrap_or("").starts_with("GET")),
        "GET requests must not be audited"
    );
}

#[tokio::test]
async fn test_audit_login_success_and_filter() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 造一个可登录用户（bcrypt hash）
    let body = serde_json::json!({
        "username": "audit-login",
        "password_hash": bcrypt_hash("Passw0rd!"),
        "role": "admin"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 登录（成功）
    let login_body = serde_json::json!({"username": "audit-login", "password": "Passw0rd!"});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .header("X-Real-IP", "192.168.1.100")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 登录（失败）
    let bad_body = serde_json::json!({"username": "audit-login", "password": "wrong"});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bad_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // 按 action=LOGIN 过滤查询
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/operation-logs?action=LOGIN")
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
    let logs: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let items = logs["data"].as_array().unwrap();
    let actions: Vec<&str> = items
        .iter()
        .map(|l| l["action"].as_str().unwrap_or(""))
        .collect();
    assert!(
        actions.contains(&"LOGIN_SUCCESS"),
        "should contain LOGIN_SUCCESS: {:?}",
        actions
    );
    assert!(
        actions.contains(&"LOGIN_FAILED"),
        "should contain LOGIN_FAILED: {:?}",
        actions
    );
    // 过滤后不应出现其他 action
    assert!(
        actions.iter().all(|a| a.starts_with("LOGIN")),
        "filtered actions: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_operation_log_repo_page_and_count() {
    let repo = Arc::new(InMemoryOperationLogRepository::new());
    for i in 0..25 {
        let action = if i % 2 == 0 {
            format!("LOGIN_SUCCESS_{}", i)
        } else {
            format!("DELETE_{}", i)
        };
        repo.create("admin", &action, Some("t"), None)
            .await
            .unwrap();
    }

    // 全量计数
    assert_eq!(repo.count(None).await.unwrap(), 25);
    // action 前缀过滤计数
    assert_eq!(repo.count(Some("LOGIN")).await.unwrap(), 13);

    // 分页：第 1 页 10 条（无过滤）
    let page1 = repo.list_page(10, 0, None).await.unwrap();
    assert_eq!(page1.len(), 10);
    // 第 3 页 10 条
    let page3 = repo.list_page(10, 20, None).await.unwrap();
    assert_eq!(page3.len(), 5);

    // 带过滤分页
    let login_page = repo.list_page(5, 0, Some("LOGIN")).await.unwrap();
    assert_eq!(login_page.len(), 5);
    assert!(login_page.iter().all(|l| l.action.starts_with("LOGIN")));
}

#[tokio::test]
async fn test_operation_log_service_paginated_pushes_down() {
    use flame_kernel::application::service::OperationLogService;
    let repo = Arc::new(InMemoryOperationLogRepository::new()) as Arc<dyn OperationLogRepository>;
    for i in 0..30 {
        let action = if i < 20 {
            "CREATE".to_string()
        } else {
            "DELETE".to_string()
        };
        repo.create("admin", &action, None, None).await.unwrap();
    }
    let svc = OperationLogService::new(repo);
    let params = PaginationParams {
        page: Some(2),
        page_size: Some(8),
    };
    let res = svc.list_paginated(&params, None).await.unwrap();
    assert_eq!(res.total, 30);
    assert_eq!(res.data.len(), 8);
    assert_eq!(res.total_pages, 4);

    let filtered = svc.list_paginated(&params, Some("CREATE")).await.unwrap();
    assert_eq!(filtered.total, 20);
    assert!(filtered.data.iter().all(|l| l.action.starts_with("CREATE")));
}

#[tokio::test]
async fn test_outbox_service_records_and_paginates() {
    use flame_kernel::application::service::OutboxService;
    let repo = Arc::new(InMemoryOutboxRepository::new()) as Arc<dyn OutboxRepository>;
    let svc = OutboxService::new(repo);

    let e1 = svc
        .record_event(&DomainEvent::AppInstalled {
            app_key: "nginx".into(),
            app_name: "Nginx".into(),
            version: "1.25".into(),
        })
        .await
        .unwrap();
    let e2 = svc
        .record_event(&DomainEvent::UserLoggedIn {
            username: "alice".into(),
        })
        .await
        .unwrap();
    assert_eq!(e1.event_type, "AppInstalled");
    assert_eq!(e2.event_type, "UserLoggedIn");
    assert!(e1.payload.contains("nginx"));
    assert!(e1.published);

    let params = PaginationParams {
        page: Some(1),
        page_size: Some(10),
    };
    let res = svc.list_paginated(&params, None).await.unwrap();
    assert_eq!(res.total, 2);
    assert_eq!(res.data.len(), 2);
    // id 倒序：新事件在前
    assert_eq!(res.data[0].event_type, "UserLoggedIn");

    let filtered = svc
        .list_paginated(&params, Some("AppInstalled"))
        .await
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.data[0].event_type, "AppInstalled");
}

#[tokio::test]
async fn test_outbox_api_endpoint_requires_permission() {
    let app = setup_full_router().await;
    let (_, token) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/outbox-events")
                .header(header::AUTHORIZATION, &token)
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
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn test_operation_logs_export_csv() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/operation-logs/export?format=csv")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("text/csv"), "content-type: {}", ct);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    // BOM + 表头
    assert!(body.starts_with('\u{feff}'));
    assert!(body.contains("id,username,action,target,ip,created_at"));
}

#[tokio::test]
async fn test_operation_logs_export_json() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/operation-logs/export?format=json")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("application/json"), "content-type: {}", ct);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_operation_logs_export_bad_format() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/operation-logs/export?format=xml")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
