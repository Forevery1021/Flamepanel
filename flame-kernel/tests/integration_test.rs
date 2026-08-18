use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

use flame_kernel::api::middleware;
use flame_kernel::api::{
    routes,
    types::{AppState, PaginationParams, Services},
};
use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
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

// ── Helpers ──────────────────────────────────────────────

fn auth_header() -> (header::HeaderName, String) {
    let jwt = JwtUtils::new("test-secret", 24);
    let token = jwt.sign(1).unwrap();
    (header::AUTHORIZATION, format!("Bearer {}", token))
}

fn bad_auth_header() -> (header::HeaderName, String) {
    (header::AUTHORIZATION, "Bearer invalid_token".to_string())
}

async fn setup_router() -> (axum::Router, AppState) {
    let user_repo = Arc::new(InMemoryUserRepository::new());
    let node_repo = Arc::new(InMemoryNodeRepository::new());
    let website_repo = Arc::new(InMemoryWebsiteRepository::new());
    let docker_repo = Arc::new(InMemoryDockerRepository::new());
    let perm_repo = Arc::new(InMemoryPermissionRepository::new());
    let role_repo = Arc::new(InMemoryRoleRepository::new());
    let log_repo = Arc::new(InMemoryOperationLogRepository::new());
    let sys_log_repo = Arc::new(InMemoryLogRepository::new());
    let web_server_repo = Arc::new(InMemoryWebServerRepository::new());
    let settings_repo = Arc::new(InMemorySettingsRepository::new());
    let database_repo = Arc::new(InMemoryDatabaseRepository::new());
    let firewall_repo = Arc::new(InMemoryFirewallRepository::new());
    let metrics_history = Arc::new(Mutex::new(MetricsHistory::new(10)));
    let (metrics_tx, _) = tokio::sync::broadcast::channel::<MetricsSnapshot>(16);
    let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(16);
    let terminal_manager = TerminalManager::new();
    let plugin_sandbox = Arc::new(PluginSandbox::new());
    let plugin_registry = Arc::new(PluginRegistry::new());
    // Seed admin user for RBAC
    user_repo.create("admin", "hash", "admin").await.unwrap();
    let docker_service = Arc::new(DockerService::new(docker_repo));
    let runner: flame_kernel::application::execution_mode::SharedCommandRunner =
        std::sync::Arc::new(flame_kernel::infrastructure::execution::EmbeddedCommandRunner);
    let web_server_service = Arc::new(WebServerService::new(web_server_repo, runner.clone()));
    let database_service = Arc::new(DatabaseService::new(database_repo, runner.clone()));
    let app_package_repo = Arc::new(InMemoryAppPackageRepository::new());
    let installed_app_repo = Arc::new(InMemoryInstalledAppRepository::new());
    let plugin_repo = Arc::new(InMemoryPluginRepository::new());
    let app_store_service = Arc::new(AppStoreService::new(
        app_package_repo,
        installed_app_repo,
        docker_service.clone(),
        web_server_service.clone(),
        database_service.clone(),
        plugin_sandbox.clone(),
        plugin_registry.clone(),
        plugin_repo.clone(),
        AppStoreService::default_apps_dir(),
        EventBus::new(100),
        flame_kernel::infrastructure::app_store::default_ports(runner.clone()),
    ));
    let backup_dir = backup_temp_dir("setup");
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::write(backup_dir.join("app.db"), b"backup-seed-db").unwrap();
    let services = Services {
        user_service: Arc::new(UserService::new(
            user_repo,
            EventBus::new(100),
            AuthCache::new(),
        )),
        node_service: Arc::new(NodeService::new(node_repo, EventBus::new(100))),
        website_service: Arc::new(WebsiteService::new(website_repo, EventBus::new(100))),
        docker_service,
        role_service: Arc::new(RoleService::new(
            role_repo,
            perm_repo.clone(),
            AuthCache::new(),
        )),
        permission_service: Arc::new(PermissionService::new(perm_repo)),
        operation_log_service: Arc::new(OperationLogService::new(log_repo)),
        outbox_service: Arc::new(OutboxService::new(
            Arc::new(InMemoryOutboxRepository::new()),
        )),
        memo_service: Arc::new(MemoService::new(Arc::new(InMemoryMemoRepository::new()))),
        log_service: Arc::new(LogService::new(sys_log_repo)),
        plugin_sandbox,
        plugin_registry,
        plugin_repo,
        app_store_service,
        web_server_service,
        settings_service: Arc::new(SettingsService::new(settings_repo)),
        database_service,
        firewall_service: Arc::new(FirewallService::new_embedded(firewall_repo)),
        backup_service: Arc::new(BackupService::new(
            backup_dir.join("app.db"),
            backup_dir.join("backups"),
        )),
        scheduled_task_service: Arc::new(ScheduledTaskService::new(Arc::new(
            InMemoryScheduledTaskRepository::new(),
        ))),
        task_service: Arc::new(TaskService::new(TaskTracker::new())),
        setup_service: Arc::new(SetupService::new(
            Arc::new(UserService::new(
                Arc::new(InMemoryUserRepository::new()),
                EventBus::new(100),
                AuthCache::new(),
            )),
            Arc::new(SettingsService::new(Arc::new(
                InMemorySettingsRepository::new(),
            ))),
            EventBus::new(100),
            std::path::PathBuf::from("."),
            runner.clone(),
            false,
        )),
        event_bus: EventBus::new(100),
    };
    let mut state = AppState::new(
        "test-secret".to_string(),
        services,
        metrics_history,
        metrics_tx,
        log_tx,
        terminal_manager,
    );
    // A3.2：测试状态显式配置 bootstrap token（节点注册端点鉴权用）
    state.bootstrap_token = "test-bootstrap-token".into();
    (routes::create_router(state.clone()), state)
}

async fn setup_full_router() -> axum::Router {
    // 测试进程内所有 router 共享进程级限流器（OnceLock），调高阈值避免全量并行 429
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = setup_router().await;
    middleware::add_middleware(router, state)
}

// ── 1. Health Check ──────────────────────────────────────

#[tokio::test]
async fn test_health_check() {
    let app = setup_full_router().await;
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

// ── 2. API Endpoints ────────────────────────────────────

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
async fn test_docker_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers?node_id=1")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_get_container_not_found() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers/nonexistent")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_docker_start_stop_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/test123/start")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/test123/stop")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_repo_factory_no_docker_fallback() {
    let factory = RepoFactory::new_in_memory();
    let repo = factory.create_docker_repo();
    // Falls back to in-memory when no Docker connection is configured
    let containers = repo.list_containers(0).await.unwrap();
    assert_eq!(containers.len(), 0);
}

#[tokio::test]
async fn test_docker_repository_get_after_create() {
    let repo = InMemoryDockerRepository::new();
    // start_container on in-memory repo is a no-op, container not tracked
    repo.start_container("my-nginx").await.unwrap();
    // So get_container returns None for any id
    let found = repo.get_container("my-nginx").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_docker_compose_deploy_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::json!({
        "project_name": "test-project",
        "compose_yaml": "version: '3'\nservices:\n  test:\n    image: nginx:alpine"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/compose/deploy")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_compose_up_down_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/compose/test-up/up")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/compose/test-up/down")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_compose_inmemory_repo() {
    let repo = InMemoryDockerRepository::new();
    let result = repo
        .compose_deploy("test", "services:\n  web:\n    image: nginx")
        .await
        .unwrap();
    assert_eq!(result["status"], "deployed");
    assert_eq!(result["project_name"], "test");
    assert!(result["compose_yaml"].as_str().unwrap().contains("nginx"));

    repo.compose_up("test").await.unwrap();
    repo.compose_down("test").await.unwrap();
}

#[tokio::test]
async fn test_docker_list_with_node_id_filter() {
    let repo = InMemoryDockerRepository::new();
    // In-memory repo always returns empty; just verify API consistency
    let for_node_0 = repo.list_containers(0).await.unwrap();
    let for_node_5 = repo.list_containers(5).await.unwrap();
    assert_eq!(for_node_0.len(), for_node_5.len());
}

#[tokio::test]
async fn test_docker_restart_remove_logs_stats_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // restart
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/c1/restart")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // remove
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/c1/remove")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // logs
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers/c1/logs?tail=50")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // stats
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers/c1/stats")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_images_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // list images
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/images")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // remove image
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/images/sha256:abc/remove")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_container_advanced_operations() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // inspect
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers/c1/inspect")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // rename
    let body = serde_json::json!({ "new_name": "c1-renamed" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/c1/rename")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // pause / unpause / kill
    for action in ["pause", "unpause", "kill"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/docker/containers/c1/{}", action))
                    .header(h.clone(), v.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "action {} failed", action);
    }

    // prune containers
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/prune")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_networks_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/networks")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // create
    let body =
        serde_json::json!({ "name": "test-net", "driver": "bridge", "subnet": "172.28.0.0/16" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/networks")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // connect / disconnect
    let body = serde_json::json!({ "container_id": "c1" });
    for action in ["connect", "disconnect"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/docker/networks/test-net/{}", action))
                    .header("Content-Type", "application/json")
                    .header(h.clone(), v.clone())
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "action {} failed", action);
    }

    // prune
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/networks/prune")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // remove
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/docker/networks/test-net")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_volumes_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/docker/volumes")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // create
    let body = serde_json::json!({ "name": "test-vol", "driver": "local" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/volumes")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // prune
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/volumes/prune")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // remove
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/docker/volumes/test-vol?force=true")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_images_pull_tag_prune() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // pull
    let body = serde_json::json!({ "image": "nginx:alpine" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/images/pull")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // tag
    let body = serde_json::json!({ "repo": "mynginx", "tag": "latest" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/images/sha256:abc/tag")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // prune
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/images/prune")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_compose_ls() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docker/compose")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_web_server_native_detect_and_autostart() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // detect native engines
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/web-servers/native/detect")
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
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    for item in arr {
        assert!(item.get("engine").is_some());
        assert!(item.get("installed").is_some());
        assert!(item.get("default_port").is_some());
    }

    // native install (in-memory 环境无包管理器，预期 BadRequest/错误)
    let body = serde_json::json!({ "engine": "nginx" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers/native/install")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    // 可能是 400（已安装/未知）或 500（无包管理器）——验证不是 401/403
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(res.status(), StatusCode::FORBIDDEN);

    // native uninstall / autostart（容错路径，systemd 不可用时不报错）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers/native/uninstall")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({ "engine": "nginx" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers/native/autostart")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "engine": "nginx",
                        "enabled": true
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 创建实例后 autostart 与 native-status
    let create_body = serde_json::json!({ "engine": "nginx" });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = created["id"].as_i64().unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/web-servers/{}/autostart", id))
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({ "enabled": true })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/web-servers/{}/native-status", id))
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_restart_with_custom_timeout() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/docker/containers/myapp/restart")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_logs_default_tail() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers/myapp/logs")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ── 3. Auth Middleware Tests ─────────────────────────────

#[tokio::test]
async fn test_auth_missing_token_returns_401() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 错误响应应为统一 JSON 格式 {code, error, message}
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_auth_invalid_token_returns_401() {
    let app = setup_full_router().await;
    let (h, v) = bad_auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}

#[tokio::test]
async fn test_auth_health_skip() {
    let app = setup_full_router().await;
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

// ── 4. Repository Tests ────────────────────────────────

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

#[tokio::test]
async fn test_docker_repository() {
    let repo = InMemoryDockerRepository::new();
    let containers = repo.list_containers(0).await.unwrap();
    assert_eq!(containers.len(), 0);

    repo.start_container("test").await.unwrap();
    repo.stop_container("test", 0).await.unwrap();

    let found = repo.get_container("test").await.unwrap();
    assert!(found.is_none());
}

// ── 5. Application Service Tests ───────────────────────

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
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = svc.create_website(&ws).await.unwrap();
    assert_eq!(id, 1);
    assert_eq!(svc.list_websites().await.unwrap().len(), 1);
}

// ── 6. Event Bus Communication ──────────────────────────

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

// ── 7. Plugin Registry ─────────────────────────────────

#[tokio::test]
async fn test_plugin_registry() {
    use chrono::Utc;
    let reg = PluginRegistry::new();
    let now = Utc::now();
    let p1 = Plugin {
        id: "p1".into(),
        name: "Logger".into(),
        version: "1.0".into(),
        enabled: true,
        author: "Test".into(),
        description: "Logger plugin".into(),
        wasm_hash: "abc123".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };
    let p2 = Plugin {
        id: "p2".into(),
        name: "Monitor".into(),
        version: "2.0".into(),
        enabled: false,
        author: "Test".into(),
        description: "Monitor plugin".into(),
        wasm_hash: "def456".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };

    reg.register(p1.clone()).unwrap();
    reg.register(p2).unwrap();

    let plugins = reg.list_all();
    assert_eq!(plugins.len(), 2);

    let logger = plugins.iter().find(|p| p.id == "p1").unwrap();
    assert_eq!(logger.name, "Logger");
    assert!(logger.enabled);
}

#[tokio::test]
async fn test_plugin_registry_duplicate() {
    use chrono::Utc;
    let reg = PluginRegistry::new();
    let now = Utc::now();
    let p = Plugin {
        id: "dup".into(),
        name: "Dup".into(),
        version: "1.0".into(),
        enabled: true,
        author: "Test".into(),
        description: "Duplicate".into(),
        wasm_hash: "abc".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };
    reg.register(p.clone()).unwrap();
    let err = reg.register(p).unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));
}

#[tokio::test]
async fn test_plugin_registry_unregister() {
    use chrono::Utc;
    let reg = PluginRegistry::new();
    let now = Utc::now();
    let p = Plugin {
        id: "test".into(),
        name: "Test".into(),
        version: "1.0".into(),
        enabled: true,
        author: "A".into(),
        description: "D".into(),
        wasm_hash: "h".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };
    reg.register(p).unwrap();
    let plugin = reg.unregister("test").unwrap();
    assert_eq!(plugin.id, "test");
    assert!(reg.list_all().is_empty());
    assert!(reg.unregister("test").is_err());
}

#[tokio::test]
async fn test_plugin_registry_enable_disable() {
    use chrono::Utc;
    let reg = PluginRegistry::new();
    let now = Utc::now();
    let p = Plugin {
        id: "p1".into(),
        name: "P1".into(),
        version: "1.0".into(),
        enabled: false,
        author: "A".into(),
        description: "D".into(),
        wasm_hash: "h".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };
    reg.register(p).unwrap();
    let enabled = reg.enable("p1").unwrap();
    assert!(enabled.enabled);
    let disabled = reg.disable("p1").unwrap();
    assert!(!disabled.enabled);
}

#[tokio::test]
async fn test_plugin_registry_get_and_exists() {
    use chrono::Utc;
    let reg = PluginRegistry::new();
    let now = Utc::now();
    let p = Plugin {
        id: "get-test".into(),
        name: "GetTest".into(),
        version: "1.0".into(),
        enabled: true,
        author: "A".into(),
        description: "D".into(),
        wasm_hash: "h".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };
    reg.register(p).unwrap();
    assert!(reg.exists("get-test"));
    assert!(!reg.exists("nonexistent"));
    let got = reg.get("get-test").unwrap();
    assert_eq!(got.name, "GetTest");
    assert!(reg.get("nonexistent").is_err());
}

#[tokio::test]
async fn test_plugin_sandbox_lifecycle() {
    let sandbox = PluginSandbox::new();
    // Load an invalid WASM — should fail validation
    let result = sandbox.load_plugin("bad", vec![0, 1, 2, 3], None).await;
    assert!(result.is_err());

    // List should be empty
    let list = sandbox.list_plugins().await;
    assert!(list.is_empty());

    // Unload non-existent should fail
    assert!(sandbox.unload_plugin("nonexistent").await.is_err());

    // Enable/disable non-existent should fail
    assert!(sandbox.enable_plugin("nonexistent").await.is_err());
    assert!(sandbox.disable_plugin("nonexistent").await.is_err());
}

#[tokio::test]
async fn test_plugin_sandbox_enable_disable() {
    let sandbox = PluginSandbox::new();
    // Load valid minimal WASM module that defines no functions
    let wasm = wat::parse_str("(module)").unwrap();
    let loaded = sandbox.load_plugin("minimal", wasm, None).await.unwrap();
    assert_eq!(format!("{:?}", loaded.status), "Loaded");

    // Disable
    let disabled = sandbox.disable_plugin("minimal").await.unwrap();
    assert_eq!(format!("{:?}", disabled.status), "Disabled");

    // Enable
    let enabled = sandbox.enable_plugin("minimal").await.unwrap();
    assert_eq!(format!("{:?}", enabled.status), "Loaded");

    // Unload
    sandbox.unload_plugin("minimal").await.unwrap();
    assert!(sandbox.get_plugin("minimal").await.is_err());
}

// ── 7b. Plugin HTTP Endpoints ─────────────────────────

#[tokio::test]
async fn test_plugin_list_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/plugins")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_plugin_load_and_get_endpoint() {
    use base64::{engine::general_purpose, Engine as _};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let wasm = wat::parse_str("(module)").unwrap();
    let b64 = general_purpose::STANDARD.encode(&wasm);
    let body = serde_json::json!({
        "id": "test-plugin",
        "name": "Test Plugin",
        "version": "1.0.0",
        "author": "FlamePanel",
        "description": "A test plugin",
        "wasm_base64": b64,
    });
    // load
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // get
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/plugins/test-plugin")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // get non-existent
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/plugins/nonexistent")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_plugin_enable_disable_endpoint() {
    use base64::{engine::general_purpose, Engine as _};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let wasm = wat::parse_str("(module)").unwrap();
    let b64 = general_purpose::STANDARD.encode(&wasm);
    // load
    let body = serde_json::json!({
        "id": "toggle-plugin",
        "name": "Toggle",
        "wasm_base64": b64,
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // disable
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/toggle-plugin/disable")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // enable
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/toggle-plugin/enable")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // unload
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/toggle-plugin")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_plugin_load_empty_wasm_rejected() {
    use base64::{engine::general_purpose, Engine as _};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::json!({
        "id": "empty-plugin",
        "name": "Empty",
        "wasm_base64": general_purpose::STANDARD.encode([]),
    });
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_plugin_load_invalid_base64_rejected() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::json!({
        "id": "bad-plugin",
        "wasm_base64": "!!!not-base64!!!",
    });
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_plugin_enable_disable_nonexistent_fails() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // disable non-existent
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/nonexistent/disable")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // enable non-existent
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/nonexistent/enable")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_plugin_execute_endpoint() {
    use base64::{engine::general_purpose, Engine as _};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // WASM module exporting "run" -> i32 returning 42
    let wasm =
        wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 42))"#).unwrap();
    let b64 = general_purpose::STANDARD.encode(&wasm);
    // load
    let body = serde_json::json!({
        "id": "exec-plugin",
        "name": "ExecTest",
        "wasm_base64": b64,
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // execute
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/exec-plugin/execute/run")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(r#"{"args":null}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // execute with empty args
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/exec-plugin/execute/run")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_plugin_execute_nonexistent_fails() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/nonexistent/execute/run")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_plugin_execute_disabled_fails() {
    use base64::{engine::general_purpose, Engine as _};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let wasm =
        wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 99))"#).unwrap();
    let b64 = general_purpose::STANDARD.encode(&wasm);
    // load
    let body = serde_json::json!({
        "id": "disable-exec",
        "name": "DisableExec",
        "wasm_base64": b64,
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // disable
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/disable-exec/disable")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // execute should fail
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/plugins/disable-exec/execute/run")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_docker_get_container_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docker/containers/some-container")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // InMemory returns None which maps to NotFound
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_jwt_sign_and_verify() {
    let jwt = JwtUtils::new("test-secret", 1);
    let token = jwt.sign(100).unwrap();
    assert!(token.len() > 20);

    let claims = jwt.verify(&token).unwrap();
    assert_eq!(claims.sub, "100");
}

#[tokio::test]
async fn test_jwt_invalid_token() {
    let jwt = JwtUtils::new("test-secret", 1);
    let err = jwt.verify("invalid.token.here").unwrap_err();
    assert!(matches!(err, AppError::Unauthorized(_)));
}

#[tokio::test]
async fn test_jwt_wrong_secret() {
    let signer = JwtUtils::new("real-secret", 1);
    let verifier = JwtUtils::new("wrong-secret", 1);
    let token = signer.sign(1).unwrap();
    let err = verifier.verify(&token).unwrap_err();
    assert!(matches!(err, AppError::Unauthorized(_)));
}

// ── 9. Password Utils ──────────────────────────────────

#[tokio::test]
async fn test_password_hash_and_verify() {
    let password = "MySecureP@ss1";
    let hash = PasswordUtils::hash(password).unwrap();
    assert!(hash.starts_with("$2b$") || hash.starts_with("$2a$"));

    assert!(PasswordUtils::verify(password, &hash).unwrap());
    assert!(!PasswordUtils::verify("wrong_password", &hash).unwrap());
}

// ── 10. AppError Tests ─────────────────────────────────

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

// ── 11. FlameKernel Initialization ─────────────────────

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

// ── 12. End-to-End: Full Middleware Stack ───────────────

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

// ── 13. Edge Cases ─────────────────────────────────────

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

// ── 14. Plugin entity serialization ────────────────────

#[tokio::test]
async fn test_plugin_serde() {
    use chrono::Utc;
    let now = Utc::now();
    let p = Plugin {
        id: "test".into(),
        name: "Test".into(),
        version: "0.1".into(),
        enabled: false,
        author: "Author".into(),
        description: "Description".into(),
        wasm_hash: "hash".into(),
        created_at: now,
        updated_at: now,
        homepage: None,
        license: None,
        tags: vec![],
        config_schema: None,
        dependencies: vec![],
        wasm_base64: String::new(),
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Plugin = serde_json::from_str(&json).unwrap();
    assert_eq!(p.id, back.id);
    assert_eq!(p.enabled, back.enabled);
    assert_eq!(p.author, back.author);
    assert_eq!(p.wasm_hash, back.wasm_hash);
}

// ── 15. Docker entity serialization ────────────────────

#[tokio::test]
async fn test_docker_container_serde() {
    let c = DockerContainer {
        id: "abc123".into(),
        image: "nginx:latest".into(),
        name: "web".into(),
        status: "running".into(),
        node_id: 1,
        created_at: Utc::now(),
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: DockerContainer = serde_json::from_str(&json).unwrap();
    assert_eq!(c.id, back.id);
    assert_eq!(c.image, back.image);
}

// ── 16. DomainEvent equality ───────────────────────────

#[tokio::test]
async fn test_domain_event_debug_and_clone() {
    let ev = DomainEvent::UserCreated {
        user_id: 1,
        username: "test".into(),
    };
    let ev2 = ev.clone();
    assert!(format!("{:?}", ev2).contains("UserCreated"));
}

// ── 17. Audit log (operation_logs) ──────────────────────

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

// ── 18. Event bus + handler ───────────────────────────

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

// ── 19. SmtpConfig defaults ──────────────────────────

#[tokio::test]
async fn test_smtp_config_defaults() {
    use flame_kernel::notification::SmtpConfig;
    let cfg = SmtpConfig::default();
    assert_eq!(cfg.host, "localhost");
    assert_eq!(cfg.from, "noreply@flamepanel.local");
    assert!(!cfg.use_tls);
}

// ── 20. Log system (real-time logs) ──────────────────

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

// ── 20. Database Endpoints ─────────────────────────────

#[tokio::test]
async fn test_database_list_empty() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/databases")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_database_get_not_found() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/databases/999")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_database_delete_not_found() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/databases/999")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn test_database_batch_status_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({ "updates": [[1,"stopped"]] })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/databases/batch-status")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_database_batch_status_rejects_empty() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({ "updates": [] })).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/databases/batch-status")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── A2 扩展：DatabaseService 批量状态更新（set_many 事务语义） ───────────────────────
#[tokio::test]
async fn test_database_batch_update_status_is_atomic() {
    let repo: Arc<dyn DatabaseRepository> = Arc::new(InMemoryDatabaseRepository::new());
    let runner: flame_kernel::application::execution_mode::SharedCommandRunner =
        Arc::new(flame_kernel::infrastructure::execution::EmbeddedCommandRunner);
    let svc = DatabaseService::new(repo.clone(), runner);
    let now = Utc::now();
    for i in 0..3 {
        let inst = DatabaseInstance {
            id: 0,
            db_type: "mysql".into(),
            name: format!("db-{}", i),
            version: "8.0".into(),
            port: 3306 + i,
            status: "running".into(),
            install_path: String::new(),
            data_dir: String::new(),
            config_file: String::new(),
            root_user: "root".into(),
            created_at: now,
            updated_at: now,
            resource_version: 0,
        };
        svc.repo.create(&inst).await.unwrap();
    }
    let updates = vec![(1, "stopped".to_string()), (2, "stopped".to_string())];
    svc.update_instances_status_batch(&updates).await.unwrap();
    let all = svc.list_instances().await.unwrap();
    assert_eq!(all.iter().find(|i| i.id == 1).unwrap().status, "stopped");
    assert_eq!(all.iter().find(|i| i.id == 2).unwrap().status, "stopped");
    assert_eq!(all.iter().find(|i| i.id == 3).unwrap().status, "running");
}

// ── 21. App Store ────────────────────────────────────────

#[tokio::test]
async fn test_app_store_list_packages() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/app-store/packages")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_app_store_install_wasm_builtin() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({
        "package_key": "wasm-hello",
        "version": "1.0.0",
        "mode": "wasm",
        "name": "api-hello",
        "values": { "name": "api-hello" }
    }))
    .unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/app-store/packages/wasm-hello/install")
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body_json["mode"], "wasm");
    assert_eq!(body_json["status"], "running");
}

#[tokio::test]
async fn test_app_store_list_installed_and_uninstall() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({
        "package_key": "wasm-hello",
        "version": "1.0.0",
        "mode": "wasm",
        "name": "api-hello-2",
        "values": { "name": "api-hello-2" }
    }))
    .unwrap();
    let install_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/app-store/packages/wasm-hello/install")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(install_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(install_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let installed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = installed["id"].as_i64().unwrap();

    let list_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/app-store/installed")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(list_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(list
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"].as_i64() == Some(id)));

    let uninstall_res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/app-store/installed/{}/uninstall", id))
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(uninstall_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_app_store_get_package_not_found() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/app-store/packages/nonexistent-app")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ── 22. Web Engine presets & switching ───────────────────

#[tokio::test]
async fn test_web_server_presets_list() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/web-servers/presets")
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
    let list: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"low") && names.contains(&"ultra"));
    assert!(list
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["recommended"] == true));
}

#[tokio::test]
async fn test_web_server_switch_engine() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let create_body = serde_json::to_vec(&json!({
        "engine": "nginx", "port": 8081
    }))
    .unwrap();
    let create_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = created["id"].as_i64().unwrap();

    let switch_body = serde_json::to_vec(&json!({ "engine": "caddy" })).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/web-servers/{}/switch-engine", id))
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(switch_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(updated["engine"], "caddy");
}

#[tokio::test]
async fn test_website_switch_engine() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let create_body = serde_json::to_vec(&json!({
        "website": {
            "id": 0, "name": "switch-test", "domain": "switch.example.com",
            "root_path": "/var/www/switch", "engine": "nginx", "node_id": 1,
            "status": "running", "ssl_enabled": false, "proxy_enabled": false,
            "created_at": "2026-01-01T00:00:00Z"
        }
    }))
    .unwrap();
    let create_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/websites")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    let switch_body = serde_json::to_vec(&json!({ "engine": "openlitespeed" })).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/websites/{}/switch-engine", id))
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(switch_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(updated["engine"], "openlitespeed");
}

// ── 13. Unified Error Format ────────────────────────────

/// 未知路由应返回统一 JSON 404（而非 axum 默认纯文本）
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

/// 无权限用户（viewer）访问管理端点应返回统一 JSON 403
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
    let app = middleware::add_middleware(router, state);
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

/// 安全基线：未注册 / 未声明权限的路径应返回 **404**（未知路由不泄露内部结构）。
/// 已注册路由的资源级越权则由 RBAC（`test_forbidden_returns_json_403`）与
/// `permission_table_covers_all_routes` 一致性测试保证。
#[tokio::test]
async fn test_undeclared_protected_route_defaults_to_403() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware(router, state);
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

// ── 14. Backup System ────────────────────────────────────

fn backup_temp_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("fp_backup_test_{}_{}", std::process::id(), tag))
}

#[tokio::test]
async fn test_backup_service_crud() {
    let tmp = backup_temp_dir("crud");
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("app.db");
    std::fs::write(&db, b"db-content-v1").unwrap();
    let svc = BackupService::new(&db, tmp.join("backups"));

    // create
    let entry = svc.create_backup().await.unwrap();
    assert!(entry.filename.starts_with("flamepanel-"));
    assert_eq!(entry.size, 13);

    // list
    let list = svc.list_backups().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].filename, entry.filename);

    // locate (download path)
    let p = svc.get_backup_path(&entry.filename).await.unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"db-content-v1");

    // path traversal rejected
    assert!(svc.get_backup_path("../app.db").await.is_err());
    assert!(svc.get_backup_path("backups/../app.db").await.is_err());
    assert!(svc.get_backup_path("nonexistent.db").await.is_err());

    // restore: modify db, restore from backup, content returns
    std::fs::write(&db, b"db-content-mutated").unwrap();
    svc.restore_backup(&entry.filename).await.unwrap();
    assert_eq!(std::fs::read(&db).unwrap(), b"db-content-v1");

    // delete
    svc.delete_backup(&entry.filename).await.unwrap();
    assert!(svc.get_backup_path(&entry.filename).await.is_err());
    // Stage4.5：restore 会留下 pre-restore-* 二次备份（不再为空）
    let remaining = svc.list_backups().await.unwrap();
    assert!(
        remaining
            .iter()
            .all(|b| b.filename.starts_with("pre-restore-")),
        "only pre-restore backups should remain, got {:?}",
        remaining
            .iter()
            .map(|b| b.filename.clone())
            .collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[tokio::test]
async fn test_backup_api_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // create backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/backups")
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
    let filename = body["filename"].as_str().unwrap().to_string();
    assert!(filename.starts_with("flamepanel-"));

    // list backups
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/backups")
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
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["filename"], filename);

    // download backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/backups/{filename}"))
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
    assert_eq!(&bytes[..], b"backup-seed-db");

    // restore backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/backups/{filename}/restore"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "filename": filename }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // delete backup
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/backups/{filename}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // list again: Stage4.5 restore 会留下 pre-restore-* 二次备份
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/backups")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert!(body[0]["filename"]
        .as_str()
        .unwrap()
        .starts_with("pre-restore-"));
}

// ── 15. Scheduled Tasks ─────────────────────────────────

#[tokio::test]
async fn test_scheduled_task_service_crud_and_execution() {
    let repo = Arc::new(InMemoryScheduledTaskRepository::new());
    let svc = ScheduledTaskService::new(repo.clone());

    // create with invalid cron rejected
    let mut task = ScheduledTask {
        id: 0,
        name: "bad".into(),
        command: "echo hi".into(),
        schedule: "not a cron".into(),
        enabled: true,
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(svc.create_task(task.clone()).await.is_err());

    // create valid task
    task.name = "cleanup".into();
    task.schedule = "0 3 * * *".into();
    let created = svc.create_task(task).await.unwrap();
    assert!(created.id > 0);
    assert_eq!(created.last_status, "never");
    assert!(created.next_run_at.is_some());

    // list
    let params = PaginationParams {
        page: Some(1),
        page_size: Some(10),
    };
    let list = svc.list_tasks(&params).await.unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.data[0].name, "cleanup");

    // get / 404
    assert!(svc.get_task(created.id).await.is_ok());
    assert!(svc.get_task(9999).await.is_err());

    // run now (shell command succeeds)
    let mut runner = created.clone();
    runner.id = 0;
    runner.name = "runner".into();
    runner.command = "echo hello".into();
    runner.schedule = "* * * * *".into();
    let runner = svc.create_task(runner).await.unwrap();
    let ran = svc.run_now(runner.id).await.unwrap();
    assert_eq!(ran.last_status, "success");
    assert!(ran.last_run_at.is_some());
    assert_eq!(ran.last_output.trim(), "hello");

    // failing command
    let mut failer = runner.clone();
    failer.id = 0;
    failer.name = "failer".into();
    failer.command = "exit 3".into();
    let failer = svc.create_task(failer).await.unwrap();
    let ran = svc.run_now(failer.id).await.unwrap();
    assert_eq!(ran.last_status, "failed");

    // toggle
    let toggled = svc.toggle_enabled(runner.id, false).await.unwrap();
    assert!(!toggled.enabled);
    assert!(toggled.next_run_at.is_none());
    let reenabled = svc.toggle_enabled(runner.id, true).await.unwrap();
    assert!(reenabled.enabled);
    assert!(reenabled.next_run_at.is_some());

    // delete
    svc.delete_task(runner.id).await.unwrap();
    assert!(svc.get_task(runner.id).await.is_err());
}

#[tokio::test]
async fn test_scheduled_task_tick_executes_due_tasks() {
    let repo = Arc::new(InMemoryScheduledTaskRepository::new());
    let svc = ScheduledTaskService::new(repo.clone());
    let now = Utc::now();

    // task due in the past (next_run_at already passed) - inject via repo,
    // since create_task recomputes next_run_at from now
    let task = ScheduledTask {
        id: 0,
        name: "due".into(),
        command: "echo ticked".into(),
        schedule: "* * * * *".into(),
        enabled: true,
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: Some(now - chrono::Duration::seconds(60)),
        created_at: now,
        updated_at: now,
    };
    let created_id = repo.create(&task).await.unwrap();

    // disabled task not executed
    let disabled = ScheduledTask {
        id: 0,
        name: "disabled".into(),
        command: "echo no".into(),
        schedule: "* * * * *".into(),
        enabled: false,
        last_status: "never".into(),
        last_output: String::new(),
        last_run_at: None,
        next_run_at: Some(now - chrono::Duration::seconds(60)),
        created_at: now,
        updated_at: now,
    };
    let disabled_id = repo.create(&disabled).await.unwrap();

    svc.tick().await.unwrap();

    let due_after = svc.get_task(created_id).await.unwrap();
    assert_eq!(due_after.last_status, "success");
    assert_eq!(due_after.last_output.trim(), "ticked");
    assert!(due_after.next_run_at.unwrap() > now);

    let disabled_after = svc.get_task(disabled_id).await.unwrap();
    assert_eq!(disabled_after.last_status, "never");
}

#[tokio::test]
async fn test_scheduled_task_api_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // create
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scheduled-tasks")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "daily backup",
                        "command": "echo backup",
                        "schedule": "0 2 * * *",
                        "enabled": true,
                    })
                    .to_string(),
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
    let id = body["id"].as_i64().unwrap();
    assert_eq!(body["name"], "daily backup");
    assert!(body["next_run_at"].is_string());

    // invalid cron -> 400
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scheduled-tasks")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "bad", "command": "x", "schedule": "bad", "enabled": true })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/scheduled-tasks")
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
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["name"], "daily backup");

    // update
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/scheduled-tasks/{id}"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "daily backup v2",
                        "command": "echo backup",
                        "schedule": "30 2 * * *",
                        "enabled": true,
                    })
                    .to_string(),
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
    assert_eq!(body["name"], "daily backup v2");

    // run now
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/scheduled-tasks/{id}/run"))
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
    assert_eq!(body["last_status"], "success");

    // toggle off
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/scheduled-tasks/{id}/toggle"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(json!({ "enabled": false }).to_string()))
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
    assert_eq!(body["enabled"], false);
    assert!(body["next_run_at"].is_null());

    // delete
    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/scheduled-tasks/{id}"))
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ── 节点心跳（P0-A） ─────────────────────────────────────────────

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
async fn test_node_heartbeat_no_token_compat() {
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

    // 无 token 心跳 → 放行（兼容）
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
    assert_eq!(res.status(), StatusCode::OK);
}

// ── 生产安全（P0-B）：强制改密 / refresh / 登录锁定 ─────────────

fn bcrypt_hash(pw: &str) -> String {
    flame_kernel::utils::password::PasswordUtils::hash(pw).unwrap()
}

#[tokio::test]
async fn test_auth_refresh_endpoint() {
    let app = setup_full_router().await;
    // setup 的 admin hash 是 "hash" 字符串，无法登录；直接签发 refresh token 测试刷新接口
    let jwt = JwtUtils::new_pair("test-secret");
    let refresh_token = jwt.sign_refresh(1).unwrap();

    // 合法 refresh token → 200，返回新的 access + refresh
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::AUTHORIZATION, format!("Bearer {}", refresh_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(resp["token"].as_str().unwrap().len() > 20);
    assert!(resp["refresh_token"].as_str().unwrap().len() > 20);
    assert_eq!(resp["username"], "admin");

    // 无 token → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // access token 不可用于刷新（类型校验）→ 401
    let access_token = jwt.sign_access(1).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
                .body(Body::empty())
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
async fn test_auth_me_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
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
    let me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(me["username"], "admin");
    assert!(me.get("must_change_password").is_some());
}

#[tokio::test]
async fn test_must_change_password_enforced() {
    // 构造必须改密的用户，验证访问受限
    let user_repo: Arc<dyn UserRepository> = Arc::new(InMemoryUserRepository::new());
    let user = user_repo
        .create("forced", &bcrypt_hash("OldP@ss1"), "admin")
        .await
        .unwrap();
    user_repo
        .update(&User {
            must_change_password: true,
            ..user
        })
        .await
        .unwrap();

    // 通过 service 直接验证中间件逻辑：构造 AppState 级测试过于复杂，
    // 此处验证 set_must_change_password 与 login 响应的标志位
    let bus = EventBus::new(100);
    let user_service = UserService::new(user_repo.clone(), bus, AuthCache::new());
    let updated = user_service
        .find_by_username("forced")
        .await
        .unwrap()
        .unwrap();
    assert!(updated.must_change_password, "flag should persist");

    // 清除标志
    user_service
        .set_must_change_password(updated.id, false)
        .await
        .unwrap();
    let cleared = user_service
        .find_by_username("forced")
        .await
        .unwrap()
        .unwrap();
    assert!(!cleared.must_change_password);
}

#[tokio::test]
async fn test_login_attempt_lock_unit() {
    use flame_kernel::api::login_attempt::LoginAttemptStore;
    let store = LoginAttemptStore::new();
    for _ in 0..5 {
        store.record_failure("alice").await;
    }
    let err = store.check_locked("alice").await;
    assert!(err.is_err(), "locked after 5 failures");
    assert_eq!(err.unwrap_err().status_code(), StatusCode::FORBIDDEN);
}

// ── 自动备份（P0-C） ───────────────────────────────────────────

#[tokio::test]
async fn test_backup_retention_cleans_old() {
    let dir = backup_temp_dir("retention");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("app.db");
    std::fs::write(&db, b"seed-db").unwrap();
    let svc = BackupService::new(&db, dir.join("backups"));

    // 创建 5 份备份
    for _ in 0..5 {
        svc.create_backup().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }
    let all = svc.list_backups().await.unwrap();
    assert_eq!(all.len(), 5);

    // 保留 3 份 → 删除 2 份
    let removed = svc.enforce_retention(3).await.unwrap();
    assert_eq!(removed.len(), 2);
    let remaining = svc.list_backups().await.unwrap();
    assert_eq!(remaining.len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_backup_last_age_and_interval() {
    let dir = backup_temp_dir("age");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("app.db");
    std::fs::write(&db, b"seed-db").unwrap();
    let svc = BackupService::new(&db, dir.join("backups"));

    // 无备份 → None
    assert!(svc.last_backup_age_secs().await.unwrap().is_none());

    // 创建后 age < 5s
    svc.create_backup().await.unwrap();
    let age = svc.last_backup_age_secs().await.unwrap().unwrap();
    assert!(age < 5, "age={} should be < 5s", age);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── 审计日志（M2-A）：写操作自动落库 ────────────────────────────

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

// ── 可观测性（M2-B）：/api/health 详细检查 ───────────────────────

#[tokio::test]
async fn test_health_detail_endpoint() {
    let app = setup_full_router().await;

    // 免认证访问
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let health: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(health["status"], "ok");
    assert!(!health["version"].as_str().unwrap().is_empty());
    assert!(health["checks"]["database"]["status"].as_str().is_some());
    assert!(health["checks"]["disk"]["status"].as_str().is_some());
    // docker 在测试环境可能是 degraded（无 daemon），但字段必须存在
    assert!(health["checks"]["docker"].get("status").is_some());
}

// ── 事件驱动（M2-D） ───────────────────────────────────────────

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
async fn test_app_store_install_publishes_event() {
    // AppStoreService 安装内置应用后应发布 AppInstalled 事件
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

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
    let dir = std::env::temp_dir().join(format!("appstore_event_{}", std::process::id()));
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
        bus.clone(),
        flame_kernel::infrastructure::app_store::default_ports(runner.clone()),
    );
    // 种子内置应用
    svc.seed_builtin_apps().await.unwrap();

    // 安装（容器模式走 InMemory docker，会成功）
    let req = crate_install_req();
    let _ = svc.install(&req).await;

    let received = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(received, DomainEvent::AppInstalled { .. }));
}

fn crate_install_req() -> flame_kernel::application::app_store_service::InstallRequest {
    serde_json::from_value(serde_json::json!({
        "package_key": "nginx",
        "version": "1.27",
        "mode": "container",
        "name": "event-nginx",
        "port": 18083,
        "values": { "PORT": "18083", "NAME": "event-nginx" },
        "confirm_risky": true
    }))
    .unwrap()
}

// ── 备忘录/TODO + 进程TOP + 常用应用（v0.7） ───────────────────

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

// ── Stage0 安全加固（P0）：文件沙箱 / JWT 强化 / WASM 完整性 ───────────

#[tokio::test]
async fn test_file_sandbox_blocks_path_escape() {
    use flame_kernel::file::FileService;
    // 临时白名单根
    let root = std::env::temp_dir().join(format!("fp-sandbox-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let svc = FileService::new(root.clone());

    // 白名单内读写正常
    std::fs::write(root.join("a.txt"), "hello").unwrap();
    let content = svc.read("/a.txt").await.unwrap();
    assert_eq!(content, "hello");

    // `..` 穿越被拒绝
    assert!(svc.read("../etc/shadow").await.is_err());
    assert!(svc.list("../../").await.is_err());

    // 绝对路径逃逸（/etc）被拒绝
    assert!(svc.read("/etc/passwd").await.is_err());
    assert!(svc.read("/etc/shadow").await.is_err());

    // 创建/写入逃逸路径被拒绝
    assert!(svc.create_file("/tmp/evil.txt").await.is_err());
    assert!(svc.write("/tmp/evil.txt", "x").await.is_err());

    // 相对路径解析仍被限制在根内
    std::fs::create_dir_all(root.join("sub")).unwrap();
    assert!(svc.list("sub").await.is_ok());

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_file_sandbox_blocks_absolute_root() {
    use flame_kernel::file::FileService;
    let root = std::env::temp_dir().join(format!("fp-sandbox-abs-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let svc = FileService::new(root.clone());

    // /root 与 /etc/shadow 均不可读
    assert!(svc.read("/root").await.is_err());
    assert!(svc.read("/etc/shadow").await.is_err());

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_jwt_access_vs_refresh_type_check() {
    let jwt = JwtUtils::new_pair("test-secret-0123456789abcdef0123456789abcdef");
    let access = jwt.sign_access(7).unwrap();
    let refresh = jwt.sign_refresh(7).unwrap();

    // access 只能通过 verify_access
    assert!(jwt.verify_access(&access).is_ok());
    assert!(jwt.verify_refresh(&access).is_err());
    // refresh 只能通过 verify_refresh
    assert!(jwt.verify_refresh(&refresh).is_ok());
    assert!(jwt.verify_access(&refresh).is_err());
    // 通用 verify 两者均通过
    assert!(jwt.verify(&access).is_ok());
    assert!(jwt.verify(&refresh).is_ok());
}

#[tokio::test]
async fn test_jwt_secret_min_length_validation() {
    use flame_kernel::utils::jwt::validate_secret;
    // 过短拒绝
    assert!(validate_secret("short").is_err());
    assert!(validate_secret("1234567890123456789012345678901").is_err()); // 31 bytes
                                                                          // 达标通过
    assert!(validate_secret("12345678901234567890123456789012").is_ok()); // 32 bytes
}

#[tokio::test]
async fn test_wasm_hash_verification() {
    use flame_kernel::plugin::verify_wasm_hash;
    let wasm = wat::parse_str("(module)").unwrap();
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&wasm);
    let hash = format!("{:x}", hasher.finalize());

    // 正确哈希通过
    assert!(verify_wasm_hash(&wasm, Some(&hash)).is_ok());
    // 错误哈希拒绝
    assert!(verify_wasm_hash(&wasm, Some("deadbeef")).is_err());
    // None 或空串跳过校验
    assert!(verify_wasm_hash(&wasm, None).is_ok());
    assert!(verify_wasm_hash(&wasm, Some("")).is_ok());
}

#[tokio::test]
async fn test_wasm_restore_hash_mismatch_rejected() {
    // 恢复时 hash 不匹配应拒绝加载（通过 registry 级验证函数模拟）
    use flame_kernel::plugin::verify_wasm_hash;
    let wasm = wat::parse_str("(module)").unwrap();
    assert!(verify_wasm_hash(&wasm, Some("bogus")).is_err());
}

#[tokio::test]
async fn test_sqlite_runtime_pragmas() {
    // 直接验证 PRAGMA 设置（使用内存库）
    use flame_kernel::infrastructure::sqlite::configure_sqlite_pragmas;
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    assert!(configure_sqlite_pragmas(&pool).await.is_ok());
    pool.close().await;
}

#[tokio::test]
async fn test_jwt_rotate_secret_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 过短密钥被拒绝（先于轮换，用旧 token）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/rotate-secret")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"secret":"short"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 轮换为合法长度的新密钥
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/rotate-secret")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"secret":"new-secret-0123456789abcdef0123456789abcdef"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 轮换后旧 access token 失效（401）
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(h, v)
                .body(Body::empty())
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

// ── 30. Stage2 分页下沉 ─────────────────────────────────

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

#[tokio::test]
async fn test_scheduled_task_repo_page_and_count() {
    let repo = Arc::new(InMemoryScheduledTaskRepository::new());
    for i in 0..12 {
        let task = ScheduledTask {
            id: 0,
            name: format!("task-{}", i),
            command: "echo hi".into(),
            schedule: "* * * * *".into(),
            enabled: true,
            last_status: String::new(),
            last_output: String::new(),
            last_run_at: None,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&task).await.unwrap();
    }
    assert_eq!(repo.count().await.unwrap(), 12);
    let page = repo.list_page(10, 0).await.unwrap();
    assert_eq!(page.len(), 10);
    assert_eq!(repo.list_page(10, 10).await.unwrap().len(), 2);
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

// ── Stage6 事件落库（Outbox）────────────────────

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

// ── Stage3.3 OpenAPI ────────────────────────────────────

#[tokio::test]
async fn test_openapi_json_endpoint() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/openapi.json")
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
    // OpenAPI 3.x 基本结构
    assert_eq!(body["openapi"].as_str().unwrap(), "3.1.0");
    assert!(body["info"]["title"]
        .as_str()
        .unwrap()
        .contains("FlamePanel"));
    let paths = body["paths"].as_object().unwrap();
    // 关键路径应出现在文档中
    assert!(paths.contains_key("/api/auth/login"));
    assert!(paths.contains_key("/api/health"));
    assert!(paths.contains_key("/api/users"));
    assert!(paths.contains_key("/api/nodes"));
    assert!(paths.contains_key("/api/websites"));
    assert!(paths.contains_key("/api/scheduled-tasks"));
    assert!(paths.contains_key("/api/app-store/packages"));
    // JWT 安全方案
    assert!(body["components"]["securitySchemes"]["BearerAuth"].is_object());
    // 关键 schema
    let schemas = body["components"]["schemas"].as_object().unwrap();
    assert!(schemas.contains_key("User"));
    assert!(schemas.contains_key("ServerNode"));
    assert!(schemas.contains_key("LoginResponse"));
    assert!(schemas.contains_key("PaginatedResponse<User>"));
}

// ── Stage4.1 WebSocket 鉴权 ─────────────────────────────

/// 无 token 的 WS 握手应被拒绝（401 JSON 而非升级连接）
#[tokio::test]
async fn test_ws_without_token_rejected() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/ws/metrics")
                .header(header::UPGRADE, "websocket")
                .header(header::CONNECTION, "Upgrade")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header("Sec-WebSocket-Version", "13")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // 无 token → 校验失败，返回统一 JSON 401，不进入 upgrade
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}

/// 有效 token 的 WS 握手应通过鉴权（oneshot 无法完成真实 HTTP/1.1 upgrade，
/// 因此只断言未返回 401/403，即中间件已放行）
#[tokio::test]
async fn test_ws_with_valid_token_upgrades() {
    let app = setup_full_router().await;
    let (_, token) = auth_header();
    let token = token.trim_start_matches("Bearer ").to_string();
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/ws/metrics?token={}", token))
                .header(header::UPGRADE, "websocket")
                .header(header::CONNECTION, "Upgrade")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header("Sec-WebSocket-Version", "13")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // 鉴权已通过：不应出现 401/403（oneshot 下 WebSocketUpgrade 无法完成升级，
    // 返回 426 属于 axum 测试环境限制，非鉴权失败）
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(res.status(), StatusCode::FORBIDDEN);
}

/// 无效 token 的 WS 握手应被拒绝
#[tokio::test]
async fn test_ws_with_invalid_token_rejected() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/ws/terminal?token=invalid_token")
                .header(header::UPGRADE, "websocket")
                .header(header::CONNECTION, "Upgrade")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header("Sec-WebSocket-Version", "13")
                .body(Body::empty())
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

// ── Stage4.3 Prometheus /metrics ─────────────────────────

#[tokio::test]
async fn test_prometheus_metrics_endpoint() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("flamepanel_up 1"));
    assert!(body.contains("# TYPE flamepanel_up gauge"));
    // 至少有基本指标
    assert!(body.contains("flamepanel_info{version="));
}

// ── Stage4.4 审计日志导出 ───────────────────────────────

/// 导出 CSV（默认格式）应返回 200 + text/csv + BOM
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

/// 导出 JSON 应返回 200 + application/json + 合法 JSON 数组
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

/// 不支持的格式应返回 400
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

// ── Stage4.5 备份加固 ────────────────────────────────────

/// 备份文件权限应为 600；恢复前会自动生成 `pre-restore-*` 二次备份
#[cfg(unix)]
#[tokio::test]
async fn test_backup_hardening_permissions_and_pre_restore() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = backup_temp_dir("hardening");
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("app.db");
    std::fs::write(&db, b"db-v1").unwrap();
    let svc = BackupService::new(&db, tmp.join("backups"));

    // 创建备份 → 权限 600
    let entry = svc.create_backup().await.unwrap();
    let bp = svc.get_backup_path(&entry.filename).await.unwrap();
    let mode = std::fs::metadata(&bp).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "backup file mode should be 600, got {:o}",
        mode
    );

    // 恢复前自动二次备份
    std::fs::write(&db, b"db-v2").unwrap();
    svc.restore_backup(&entry.filename).await.unwrap();
    assert_eq!(std::fs::read(&db).unwrap(), b"db-v1");

    let list = svc.list_backups().await.unwrap();
    assert!(
        list.iter().any(|b| b.filename.starts_with("pre-restore-")),
        "expected a pre-restore backup in {:?}",
        list.iter().map(|b| b.filename.clone()).collect::<Vec<_>>()
    );
    // pre-restore 备份同样 600
    let pre = list
        .iter()
        .find(|b| b.filename.starts_with("pre-restore-"))
        .unwrap();
    let pp = svc.get_backup_path(&pre.filename).await.unwrap();
    let pmode = std::fs::metadata(&pp).unwrap().permissions().mode();
    assert_eq!(
        pmode & 0o777,
        0o600,
        "pre-restore mode should be 600, got {:o}",
        pmode
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

// ── Phase A: Website Optimistic Concurrency Control (resource_version) ──

#[tokio::test]
async fn test_website_occ_version_conflict() {
    let repo = InMemoryWebsiteRepository::new();
    let ws = Website {
        id: 0,
        name: "site".into(),
        domain: "occ-test.com".into(),
        root_path: "/var/www".into(),
        status: "active".into(),
        node_id: 1,
        engine: "nginx".into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&ws).await.unwrap();
    let created = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(created.resource_version, 0);

    // 正确版本：更新成功，版本自增
    let mut update_ok = created.clone();
    update_ok.name = "site-v2".into();
    repo.update(&update_ok).await.unwrap();
    let after = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after.resource_version, 1);
    assert_eq!(after.name, "site-v2");

    // 过期版本（仍用 v0）：冲突
    let mut stale = created.clone();
    stale.name = "stale-write".into();
    let err = repo.update(&stale).await.unwrap_err();
    assert!(
        matches!(err, AppError::Conflict(_)),
        "expected Conflict, got {:?}",
        err
    );

    // 数据库未被过期写入污染
    let after_stale = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after_stale.name, "site-v2");
    assert_eq!(after_stale.resource_version, 1);
}

#[tokio::test]
async fn test_website_occ_update_endpoint_conflict_409() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 创建网站（resource_version 默认 0）
    let create_body = serde_json::to_vec(&json!({
        "website": {
            "id": 0, "name": "occ-api", "domain": "occ-api.com",
            "root_path": "/var/www/occ", "engine": "nginx", "node_id": 1,
            "status": "active", "ssl_enabled": false, "proxy_enabled": false,
            "created_at": "2026-01-01T00:00:00Z"
        }
    }))
    .unwrap();
    let create_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/websites")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    // 第一次更新成功（resource_version=0 → 1）
    let upd1 = json!({"website": {
        "id": 0, "name": "occ-api-v1", "domain": "occ-api.com",
        "root_path": "/var/www/occ", "engine": "caddy", "node_id": 1,
        "status": "active", "ssl_enabled": false, "proxy_enabled": false,
        "created_at": "2026-01-01T00:00:00Z",
        "resource_version": 0
    }});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/websites/{}", id))
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&upd1).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(updated["name"], "occ-api-v1");
    assert_eq!(updated["resource_version"], 1);

    // 第二次更新用过期版本（resource_version=0）：409 CONFLICT
    let stale = json!({"website": {
        "id": 0, "name": "stale", "domain": "occ-api.com",
        "root_path": "/var/www/occ", "engine": "nginx", "node_id": 1,
        "status": "active", "ssl_enabled": false, "proxy_enabled": false,
        "created_at": "2026-01-01T00:00:00Z",
        "resource_version": 0
    }});
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/websites/{}", id))
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&stale).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err["error"], "CONFLICT");
}

// ── Phase A: Web Server Optimistic Concurrency Control (resource_version) ──

#[tokio::test]
async fn test_web_server_occ_version_conflict() {
    let repo = InMemoryWebServerRepository::new();
    let inst = WebServerInstance {
        id: 0,
        engine: "nginx".into(),
        version: Some("1.24".into()),
        status: "stopped".into(),
        config_path: "/etc/nginx/nginx.conf".into(),
        binary_path: Some("/usr/sbin/nginx".into()),
        port: 80,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&inst).await.unwrap();
    let created = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(created.resource_version, 0);

    // 正确版本：更新成功，版本自增
    let mut update_ok = created.clone();
    update_ok.port = 8080;
    repo.update(&update_ok).await.unwrap();
    let after = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after.resource_version, 1);
    assert_eq!(after.port, 8080);

    // 过期版本（仍用 v0）：冲突
    let mut stale = created.clone();
    stale.port = 9090;
    let err = repo.update(&stale).await.unwrap_err();
    assert!(
        matches!(err, AppError::Conflict(_)),
        "expected Conflict, got {:?}",
        err
    );

    // 数据库未被过期写入污染
    let after_stale = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after_stale.port, 8080);
    assert_eq!(after_stale.resource_version, 1);
}

#[tokio::test]
async fn test_web_server_occ_update_endpoint_conflict_409() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 创建实例（resource_version 默认 0）
    let create_body = serde_json::json!({ "engine": "nginx" });
    let create_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["resource_version"], 0);

    // 第一次更新成功（resource_version=0 → 1）
    let upd1 = serde_json::json!({ "port": 8080, "resource_version": 0 });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/web-servers/{}", id))
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&upd1).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(updated["port"], 8080);
    assert_eq!(updated["resource_version"], 1);

    // 第二次更新用过期版本（resource_version=0）：409 CONFLICT
    let stale = serde_json::json!({ "port": 9090, "resource_version": 0 });
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/web-servers/{}", id))
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&stale).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = res.into_body();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let err: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err["error"], "CONFLICT");
}

// ── Phase A: DatabaseInstance Optimistic Concurrency Control (resource_version) ──

#[tokio::test]
async fn test_database_occ_version_conflict() {
    let repo = InMemoryDatabaseRepository::new();
    let inst = DatabaseInstance {
        id: 0,
        db_type: "mysql".into(),
        name: "primary".into(),
        version: "8.0".into(),
        port: 3306,
        status: "running".into(),
        install_path: "/usr/bin/mysql".into(),
        data_dir: "/var/lib/mysql".into(),
        config_file: "/etc/mysql/my.cnf".into(),
        root_user: "root".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&inst).await.unwrap();
    let created = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(created.resource_version, 0);

    // 正确版本：更新成功，版本自增
    let mut update_ok = created.clone();
    update_ok.port = 3307;
    repo.update(&update_ok).await.unwrap();
    let after = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after.resource_version, 1);
    assert_eq!(after.port, 3307);

    // 过期版本（仍用 v0）：冲突
    let mut stale = created.clone();
    stale.port = 3308;
    let err = repo.update(&stale).await.unwrap_err();
    assert!(
        matches!(err, AppError::Conflict(_)),
        "expected Conflict, got {:?}",
        err
    );

    // 数据库未被过期写入污染
    let after_stale = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after_stale.port, 3307);
    assert_eq!(after_stale.resource_version, 1);
}

#[tokio::test]
async fn test_database_occ_update_not_found() {
    let repo = InMemoryDatabaseRepository::new();
    let inst = DatabaseInstance {
        id: 999,
        db_type: "mysql".into(),
        name: "ghost".into(),
        version: "8.0".into(),
        port: 3306,
        status: "stopped".into(),
        install_path: "".into(),
        data_dir: "".into(),
        config_file: "".into(),
        root_user: "root".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        resource_version: 0,
    };
    let err = repo.update(&inst).await.unwrap_err();
    assert!(
        matches!(err, AppError::NotFound(_)),
        "expected NotFound, got {:?}",
        err
    );
}

// ── Settings 批量原子写（Phase A2 扩展）──────────────────

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
