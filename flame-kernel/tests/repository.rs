//! 内存仓库与 RepoFactory
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
async fn test_repo_factory_no_docker_fallback() {
    let factory = RepoFactory::new_in_memory();
    let repo = factory.create_docker_repo();
    // Falls back to in-memory when no Docker connection is configured
    let containers = repo.list_containers(0).await.unwrap();
    assert_eq!(containers.len(), 0);
}

#[tokio::test]
async fn test_in_memory_user_repository() {
    let repo = InMemoryUserRepository::new();
    let user = repo.create("alice", "hash123", "user").await.unwrap();
    assert_eq!(user.username, "alice");
    assert_eq!(user.id, 1);

    let found = repo.find_by_id(1).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().username, "alice");

    let found = repo.find_by_username("alice").await.unwrap();
    assert!(found.is_some());

    let users = repo.list().await.unwrap();
    assert_eq!(users.len(), 1);
}

#[tokio::test]
async fn test_in_memory_node_repository() {
    let repo = InMemoryNodeRepository::new();
    let node = ServerNode {
        id: 0,
        name: "test".into(),
        hostname: "host.test".into(),
        ip_address: "10.0.0.1".into(),
        status: "online".into(),
        created_at: Utc::now(),
        last_heartbeat_at: None,
        metrics_json: None,
        auth_token: None,
        agent_port: 9527,
    };
    let id = repo.create(&node).await.unwrap();
    assert_eq!(id, 1);

    let found = repo.find_by_id(1).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "test");

    let found = repo.find_by_hostname("host.test").await.unwrap();
    assert!(found.is_some());

    let nodes = repo.list_all().await.unwrap();
    assert_eq!(nodes.len(), 1);
}

#[tokio::test]
async fn test_in_memory_website_repository() {
    let repo = InMemoryWebsiteRepository::new();
    let ws = Website {
        id: 0,
        name: "blog".into(),
        domain: "blog.example.com".into(),
        root_path: "/var/www/blog".into(),
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
    let id = repo.create(&ws).await.unwrap();
    assert_eq!(id, 1);

    let found = repo.find_by_domain("blog.example.com").await.unwrap();
    assert!(found.is_some());

    let all = repo.list_all().await.unwrap();
    assert_eq!(all.len(), 1);
}
