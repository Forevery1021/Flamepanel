//! 事件总线通信
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
async fn test_event_bus_publish_subscribe() {
    let bus = EventBus::new(100);
    let mut rx = bus.subscribe();

    bus.publish(DomainEvent::UserCreated {
        user_id: 42,
        username: "test_user".into(),
    })
    .await
    .unwrap();

    let received = rx.try_recv().unwrap();
    match received {
        DomainEvent::UserCreated { user_id, username } => {
            assert_eq!(user_id, 42);
            assert_eq!(username, "test_user");
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_event_bus_multiple_events() {
    let bus = EventBus::new(100);
    let mut rx = bus.subscribe();

    bus.publish(DomainEvent::NodeRegistered {
        node_id: 1,
        node_name: "server-1".into(),
    })
    .await
    .unwrap();

    bus.publish(DomainEvent::WebsiteCreated {
        website_id: 10,
        domain: "example.com".into(),
    })
    .await
    .unwrap();

    let ev1 = rx.try_recv().unwrap();
    assert!(matches!(ev1, DomainEvent::NodeRegistered { .. }));

    let ev2 = rx.try_recv().unwrap();
    assert!(matches!(ev2, DomainEvent::WebsiteCreated { .. }));
}

#[tokio::test]
async fn test_event_bus_multiple_subscribers() {
    let bus = EventBus::new(100);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    bus.publish(DomainEvent::UserCreated {
        user_id: 7,
        username: "multi".into(),
    })
    .await
    .unwrap();

    let received1 = rx1.try_recv().unwrap();
    let received2 = rx2.try_recv().unwrap();
    assert_eq!(received1, received2);
}

#[tokio::test]
async fn test_services_emit_domain_events() {
    let bus = EventBus::new(100);
    let mut rx = bus.subscribe();

    let user_svc = UserService::new(
        Arc::new(InMemoryUserRepository::new()) as Arc<dyn UserRepository>,
        bus.clone(),
        AuthCache::new(),
    );
    let node_svc = NodeService::new(
        Arc::new(InMemoryNodeRepository::new()) as Arc<dyn NodeRepository>,
        bus.clone(),
    );
    let web_svc = WebsiteService::new(
        Arc::new(InMemoryWebsiteRepository::new()) as Arc<dyn WebsiteRepository>,
        bus.clone(),
    );

    let user = user_svc.create_user("eve", "hash", "viewer").await.unwrap();
    let node_id = node_svc
        .register_node(&ServerNode {
            id: 0,
            name: "node-1".into(),
            hostname: "h1".into(),
            ip_address: "10.0.0.1".into(),
            status: "online".into(),
            created_at: Utc::now(),
            last_heartbeat_at: None,
            metrics_json: None,
            auth_token: None,
            agent_port: 9527,
        })
        .await
        .unwrap();
    let web_id = web_svc
        .create_website(&Website {
            id: 0,
            name: "site".into(),
            domain: "example.com".into(),
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
        })
        .await
        .unwrap();

    let ev1 = rx.try_recv().unwrap();
    assert!(matches!(ev1, DomainEvent::UserCreated { user_id, username }
        if user_id == user.id && username == "eve"));

    let ev2 = rx.try_recv().unwrap();
    assert!(
        matches!(ev2, DomainEvent::NodeRegistered { node_id: nid, node_name }
        if nid == node_id && node_name == "node-1")
    );

    let ev3 = rx.try_recv().unwrap();
    assert!(
        matches!(ev3, DomainEvent::WebsiteCreated { website_id: wid, domain }
        if wid == web_id && domain == "example.com")
    );
}

#[tokio::test]
async fn test_event_handler_subscribes_and_logs() {
    use flame_kernel::event::{handler::EventHandler, EventBus};
    let bus = EventBus::new(16);
    let rx = bus.subscribe();
    let handler = EventHandler::new();
    handler.spawn(rx);

    bus.publish(DomainEvent::UserCreated {
        user_id: 42,
        username: "test_user".into(),
    })
    .await
    .unwrap();
    bus.publish(DomainEvent::NodeRegistered {
        node_id: 7,
        node_name: "test_node".into(),
    })
    .await
    .unwrap();

    // Give handler a moment to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    // If no panic, handler processed events (notifications are best-effort)
}

#[tokio::test]
async fn test_event_bus_subscriber_dropped_gracefully() {
    use flame_kernel::event::EventBus;
    let bus = EventBus::new(16);
    let rx = bus.subscribe();
    drop(rx); // drop receiver
              // publish should not panic when no receivers
    bus.publish(DomainEvent::UserCreated {
        user_id: 1,
        username: "x".into(),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_event_bus_emits_new_variants() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    // 发布应用安装事件
    let _ = bus
        .publish(DomainEvent::AppInstalled {
            app_key: "nginx".into(),
            app_name: "Nginx".into(),
            version: "1.27".into(),
        })
        .await;

    let received = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    match received {
        DomainEvent::AppInstalled {
            app_key, version, ..
        } => {
            assert_eq!(app_key, "nginx");
            assert_eq!(version, "1.27");
        }
        other => panic!("expected AppInstalled, got {:?}", other),
    }
}

#[tokio::test]
async fn test_event_bus_certificate_renewal_events() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    // 1. 自动续签成功事件
    bus.publish(DomainEvent::CertificateRenewed {
        id: 101,
        domain: "example.com".into(),
    })
    .await
    .unwrap();

    let ev1 = rx.recv().await.unwrap();
    match ev1 {
        DomainEvent::CertificateRenewed { id, domain } => {
            assert_eq!(id, 101);
            assert_eq!(domain, "example.com");
        }
        other => panic!("expected CertificateRenewed, got {:?}", other),
    }

    // 2. 自动续签失败事件
    bus.publish(DomainEvent::CertificateRenewalFailed {
        id: 102,
        domain: "api.example.com".into(),
        error: "DNS-01 challenge verification timed out".into(),
    })
    .await
    .unwrap();

    let ev2 = rx.recv().await.unwrap();
    match ev2 {
        DomainEvent::CertificateRenewalFailed { id, domain, error } => {
            assert_eq!(id, 102);
            assert_eq!(domain, "api.example.com");
            assert!(error.contains("timed out"));
        }
        other => panic!("expected CertificateRenewalFailed, got {:?}", other),
    }
}
