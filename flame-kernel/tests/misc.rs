//! 内核初始化 / 中间件栈 / 错误格式 / 其他杂项
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
async fn test_app_error_into_response() {
    use axum::response::IntoResponse as _;

    let tests = vec![
        (AppError::NotFound("x".into()), StatusCode::NOT_FOUND),
        (AppError::Unauthorized("x".into()), StatusCode::UNAUTHORIZED),
        (AppError::Forbidden("x".into()), StatusCode::FORBIDDEN),
        (AppError::BadRequest("x".into()), StatusCode::BAD_REQUEST),
        (
            AppError::ValidationError("x".into()),
            StatusCode::BAD_REQUEST,
        ),
        (AppError::internal("x"), StatusCode::INTERNAL_SERVER_ERROR),
    ];
    for (err, expected) in tests {
        let resp = err.into_response();
        assert_eq!(resp.status(), expected);
    }
}

#[tokio::test]
async fn test_kernel_creates_with_default_config() {
    let config = AppConfig::default();
    let kernel = FlameKernel::new(config);

    // Verify all services are wired
    let user = kernel
        .app_state
        .user_service
        .create_user("k1", "hash", "admin")
        .await
        .unwrap();
    assert_eq!(user.username, "k1");

    let node = kernel
        .app_state
        .node_service
        .register_node(&ServerNode {
            id: 0,
            name: "kn1".into(),
            hostname: "kh1".into(),
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
    assert_eq!(node, 1);
}

#[tokio::test]
async fn test_kernel_with_factory() {
    let config = AppConfig::default();
    let factory = RepoFactory::new_in_memory();
    let kernel = FlameKernel::new_with_backend(config, factory);

    let user = kernel
        .app_state
        .user_service
        .create_user("factory_test", "hash", "user")
        .await
        .unwrap();
    assert_eq!(user.username, "factory_test");

    let users = kernel.app_state.user_service.list_users().await.unwrap();
    assert_eq!(users.len(), 1);
}

#[tokio::test]
async fn test_full_middleware_stack() {
    let app = setup_full_router().await;

    // Without auth → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // With valid auth → 200
    let (h, v) = auth_header();
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

    // Health → 200 (no auth)
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_domain_event_debug_and_clone() {
    let ev = DomainEvent::UserCreated {
        user_id: 1,
        username: "test".into(),
    };
    let ev2 = ev.clone();
    assert!(format!("{:?}", ev2).contains("UserCreated"));
}

#[tokio::test]
async fn test_smtp_config_defaults() {
    use flame_kernel::notification::SmtpConfig;
    let cfg = SmtpConfig::default();
    assert_eq!(cfg.host, "localhost");
    assert_eq!(cfg.from, "noreply@flamepanel.local");
    assert!(!cfg.use_tls);
}

#[tokio::test]
async fn test_unknown_route_returns_json_404() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/nonexistent-route")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 404);
    assert_eq!(body["error"], "NOT_FOUND");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_forbidden_returns_json_403() {
    let (router, state) = setup_router().await;
    // 创建 viewer 用户（id=2），以 viewer 身份访问用户管理（仅 admin 可写）
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let jwt = JwtUtils::new("test-secret", 24);
    let token = jwt.sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"x","password":"x","role":"viewer"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 403);
    assert_eq!(body["error"], "AUTH_FORBIDDEN");
}

#[tokio::test]
async fn test_undeclared_protected_route_defaults_to_403() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let (h, v) = auth_header(); // admin（id=1）

    // 1. 未声明权限的受保护路径 → 403（默认拒绝）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/undeclared/route")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "未注册的未知路由应返回 404 而非 403"
    );

    // 2. auth-only 白名单路径（/api/auth/me）→ 需认证但不做资源级 RBAC，不应 403
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::FORBIDDEN);
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_memos_crud_api() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 创建 memo
    let body = serde_json::json!({"content": "备份数据库", "kind": "todo"});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/memos")
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
    let memo: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = memo["id"].as_i64().unwrap();
    assert_eq!(memo["kind"], "todo");
    assert_eq!(memo["done"], false);

    // 列表（kind 过滤）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/memos?kind=todo")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(list.as_array().unwrap().iter().any(|m| m["id"] == id));

    // 标记完成
    let body = serde_json::json!({"done": true});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/memos/{}", id))
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // done 过滤
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/memos?done=true")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(list.as_array().unwrap().iter().any(|m| m["id"] == id));

    // 删除
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/memos/{}", id))
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_processes_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/processes")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let procs: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(procs.as_array().unwrap().len() <= 5);
    if let Some(first) = procs.as_array().unwrap().first() {
        assert!(first.get("pid").is_some());
        assert!(first.get("cpu").is_some());
    }
}

#[tokio::test]
async fn test_app_launch_count() {
    let bus = EventBus::new(16);
    let pkg_repo: Arc<dyn AppPackageRepository> = Arc::new(InMemoryAppPackageRepository::new());
    let installed_repo: Arc<dyn InstalledAppRepository> =
        Arc::new(InMemoryInstalledAppRepository::new());
    let docker_service = Arc::new(DockerService::new(
        Arc::new(InMemoryDockerRepository::new()),
    ));
    let runner: flame_kernel::application::execution_mode::SharedCommandRunner =
        std::sync::Arc::new(flame_kernel::infrastructure::execution::EmbeddedCommandRunner);
    let ws_service = Arc::new(WebServerService::new(
        Arc::new(InMemoryWebServerRepository::new()),
        runner.clone(),
    ));
    let db_service = Arc::new(DatabaseService::new(
        Arc::new(InMemoryDatabaseRepository::new()),
        runner.clone(),
    ));
    let sandbox = Arc::new(PluginSandbox::new());
    let registry = Arc::new(PluginRegistry::new());
    let plugin_repo: Arc<dyn PluginRepository> = Arc::new(InMemoryPluginRepository::new());
    let dir = std::env::temp_dir().join(format!("appstore_launch_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let svc = AppStoreService::new(
        pkg_repo,
        installed_repo,
        docker_service,
        ws_service,
        db_service,
        sandbox,
        registry,
        plugin_repo,
        dir,
        bus,
        flame_kernel::infrastructure::app_store::default_ports(runner.clone()),
        Arc::new(InMemoryAppSourceRepository::new()),
    );
    // 直接插入一个已安装应用
    let now = chrono::Utc::now();
    let installed = InstalledApp {
        id: 0,
        package_key: "nginx".into(),
        name: "nginx-test".into(),
        version: "1.27".into(),
        mode: "container".into(),
        status: "running".into(),
        access_url: Some("http://localhost:18083".into()),
        install_path: "/tmp/fp-launch".into(),
        container_name: None,
        port: Some(18083),
        params_json: "{}".into(),
        created_at: now,
        updated_at: now,
        launch_count: 0,
    };
    let id = svc.installed_repo.create(&installed).await.unwrap();
    let launched = svc.record_launch(id).await.unwrap();
    assert_eq!(launched.launch_count, 1);
}
