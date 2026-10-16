//! 实时日志系统
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
async fn test_log_repository_crud() {
    let repo = InMemoryLogRepository::new();
    let entry = repo
        .create(
            "docker",
            "info",
            "Container started",
            Some(r#"{"container_id":"abc"}"#),
        )
        .await
        .unwrap();
    assert_eq!(entry.source, "docker");
    assert_eq!(entry.level, "info");
    assert_eq!(entry.message, "Container started");
    assert!(entry.id > 0);

    let list = repo.list().await.unwrap();
    assert_eq!(list.len(), 1);

    let found = repo.find_by_id(entry.id).await.unwrap();
    assert!(found.is_some());

    let by_source = repo.list_by_source("docker").await.unwrap();
    assert_eq!(by_source.len(), 1);

    let by_level = repo.list_by_level("info").await.unwrap();
    assert_eq!(by_level.len(), 1);

    let by_level_err = repo.list_by_level("error").await.unwrap();
    assert!(by_level_err.is_empty());
}

#[tokio::test]
async fn test_log_ws_endpoint_registered() {
    // Verify the log list endpoint is registered and accessible
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/logs")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_log_repo_page_and_count() {
    let repo = Arc::new(InMemoryLogRepository::new());
    for i in 0..15 {
        repo.create("source", "info", &format!("msg-{}", i), None)
            .await
            .unwrap();
    }
    assert_eq!(repo.count().await.unwrap(), 15);
    let page = repo.list_page(10, 0).await.unwrap();
    assert_eq!(page.len(), 10);
    let page2 = repo.list_page(10, 10).await.unwrap();
    assert_eq!(page2.len(), 5);
}
