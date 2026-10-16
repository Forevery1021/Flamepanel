//! 应用服务单测
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
async fn test_user_service() {
    let repo = Arc::new(InMemoryUserRepository::new()) as Arc<dyn UserRepository>;
    let svc = UserService::new(repo, EventBus::new(100), AuthCache::new());
    let user = svc.create_user("bob", "hash", "admin").await.unwrap();
    assert_eq!(user.username, "bob");

    let users = svc.list_users().await.unwrap();
    assert_eq!(users.len(), 1);
}

#[tokio::test]
async fn test_node_service() {
    let repo = Arc::new(InMemoryNodeRepository::new()) as Arc<dyn NodeRepository>;
    let svc = NodeService::new(repo, EventBus::new(100));
    let node = ServerNode {
        id: 0,
        name: "n1".into(),
        hostname: "h1".into(),
        ip_address: "1.2.3.4".into(),
        status: "online".into(),
        created_at: Utc::now(),
        last_heartbeat_at: None,
        metrics_json: None,
        auth_token: None,
        agent_port: 9527,
    };
    let id = svc.register_node(&node).await.unwrap();
    assert_eq!(id, 1);
    assert_eq!(svc.list_nodes().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_website_service() {
    let repo = Arc::new(InMemoryWebsiteRepository::new()) as Arc<dyn WebsiteRepository>;
    let svc = WebsiteService::new(repo, EventBus::new(100));
    let ws = Website {
        id: 0,
        name: "site".into(),
        domain: "site.com".into(),
        root_path: "/var/www".into(),
        status: "active".into(),
        node_id: 1,
        engine: "nginx".into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "static".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = svc.create_website(&ws).await.unwrap();
    assert_eq!(id, 1);
    assert_eq!(svc.list_websites().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_log_service() {
    use flame_kernel::application::service::LogService;
    use std::sync::Arc;
    let repo = Arc::new(InMemoryLogRepository::new());
    let svc = LogService::new(repo);

    svc.log("system", "warn", "Disk usage high", None)
        .await
        .unwrap();
    svc.log("docker", "info", "Container created", None)
        .await
        .unwrap();

    let all = svc.list().await.unwrap();
    assert_eq!(all.len(), 2);

    let by_source = svc.list_by_source("docker").await.unwrap();
    assert_eq!(by_source.len(), 1);
}
