use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode, Method, header},
};
use tower::ServiceExt;
use serde_json::json;
use chrono::Utc;
use tokio::sync::Mutex;

use flame_kernel::application::service::*;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::api::{routes, types::AppState};
use flame_kernel::api::middleware;
use flame_kernel::domain::entity::*;
use flame_kernel::domain::repository::*;
use flame_kernel::event::EventBus;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::jwt::JwtUtils;
use flame_kernel::utils::password::PasswordUtils;
use flame_kernel::core::error::AppError;
use flame_kernel::config::AppConfig;
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
    let plugin_sandbox = PluginSandbox::new();
    let plugin_registry = PluginRegistry::new();
    // Seed admin user for RBAC
    user_repo.create("admin", "hash", "admin").await.unwrap();
    let state = AppState::new(
        "test-secret".to_string(),
        UserService::new(user_repo),
        NodeService::new(node_repo),
        WebsiteService::new(website_repo),
        DockerService::new(docker_repo),
        RoleService::new(role_repo, perm_repo.clone()),
        PermissionService::new(perm_repo),
        OperationLogService::new(log_repo),
        LogService::new(sys_log_repo),
        metrics_history,
        metrics_tx,
        log_tx,
        plugin_sandbox,
        plugin_registry,
        WebServerService::new(web_server_repo),
        SettingsService::new(settings_repo),
        DatabaseService::new(database_repo),
        FirewallService::new(firewall_repo),
        terminal_manager,
    );
    (routes::create_router(state.clone()), state)
}

async fn setup_full_router() -> axum::Router {
    let (router, state) = setup_router().await;
    middleware::add_middleware(router, state)
}

// ── 1. Health Check ──────────────────────────────────────

#[tokio::test]
async fn test_health_check() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ── 2. API Endpoints ────────────────────────────────────

#[tokio::test]
async fn test_create_and_list_users() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let res = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST).uri("/api/users")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "username": "testuser", "password_hash": "hashed_pw", "role": "admin"
                })).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/users")
                .header(h, v)
                .body(Body::empty()).unwrap()
        ).await.unwrap();
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/nodes")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&node).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app.clone()
        .oneshot(Request::builder().uri("/api/nodes").header(h, v).body(Body::empty()).unwrap())
        .await.unwrap();
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/websites")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&ws).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app.clone()
        .oneshot(Request::builder().uri("/api/websites").header(h, v).body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_user_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::PUT).uri("/api/users/1")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "username": "admin", "role": "operator"
                })).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    let user: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(user["role"], "operator");

    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::PUT).uri("/api/users/999")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "username": "nobody", "role": "viewer"
                })).unwrap())).unwrap()
        ).await.unwrap();
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/nodes")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&node).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    let updated = json!({"node": {
        "id": 0, "name": "node-1-renamed", "hostname": "s1.example.com",
        "ip_address": "10.0.0.2", "status": "offline",
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::PUT).uri(format!("/api/nodes/{}", id))
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    let node_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(node_res["name"], "node-1-renamed");
    assert_eq!(node_res["ip_address"], "10.0.0.2");
    assert_eq!(node_res["status"], "offline");

    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::PUT).uri("/api/nodes/999")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap())).unwrap()
        ).await.unwrap();
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/websites")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&ws).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    let updated = json!({"website": {
        "id": 0, "name": "blog-v2", "domain": "blog.example.com",
        "root_path": "/var/www/blog-v2", "status": "active", "node_id": 1,
        "engine": "caddy", "ssl_enabled": true, "proxy_enabled": true,
        "proxy_pass": "http://127.0.0.1:3000",
        "created_at": "2026-01-01T00:00:00Z"
    }});
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::PUT).uri(format!("/api/websites/{}", id))
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    let ws_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(ws_res["name"], "blog-v2");
    assert_eq!(ws_res["engine"], "caddy");
    assert_eq!(ws_res["ssl_enabled"], true);
    assert_eq!(ws_res["proxy_enabled"], true);
    assert_eq!(ws_res["proxy_pass"], "http://127.0.0.1:3000");

    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::PUT).uri("/api/websites/999")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&updated).unwrap())).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_docker_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/docker/containers")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder().uri("/api/docker/containers?node_id=1")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_get_container_not_found() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder().uri("/api/docker/containers/nonexistent")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_docker_start_stop_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/containers/test123/start")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/containers/test123/stop")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/compose/deploy")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_compose_up_down_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/compose/test-up/up")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/compose/test-up/down")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_compose_inmemory_repo() {
    let repo = InMemoryDockerRepository::new();
    let result = repo.compose_deploy("test", "services:\n  web:\n    image: nginx").await.unwrap();
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/containers/c1/restart")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // remove
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/containers/c1/remove")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // logs
    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/docker/containers/c1/logs?tail=50")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // stats
    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/docker/containers/c1/stats")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_images_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // list images
    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/docker/images")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // remove image
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/images/sha256:abc/remove")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_restart_with_custom_timeout() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/docker/containers/myapp/restart")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docker_logs_default_tail() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder().uri("/api/docker/containers/myapp/logs")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ── 3. Auth Middleware Tests ─────────────────────────────

#[tokio::test]
async fn test_auth_missing_token_returns_401() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(Request::builder().uri("/api/users").body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_invalid_token_returns_401() {
    let app = setup_full_router().await;
    let (h, v) = bad_auth_header();
    let res = app
        .oneshot(
            Request::builder().uri("/api/users")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_health_skip() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await.unwrap();
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
        id: 0, name: "test".into(), hostname: "host.test".into(),
        ip_address: "10.0.0.1".into(), status: "online".into(),
        created_at: Utc::now(),
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
        id: 0, name: "blog".into(), domain: "blog.example.com".into(),
        root_path: "/var/www/blog".into(), status: "active".into(),
        node_id: 1, engine: "nginx".into(),
        ssl_enabled: false, proxy_enabled: false, proxy_pass: None,
        created_at: Utc::now(),
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
    let svc = UserService::new(repo);
    let user = svc.create_user("bob", "hash", "admin").await.unwrap();
    assert_eq!(user.username, "bob");

    let users = svc.list_users().await.unwrap();
    assert_eq!(users.len(), 1);
}

#[tokio::test]
async fn test_node_service() {
    let repo = Arc::new(InMemoryNodeRepository::new()) as Arc<dyn NodeRepository>;
    let svc = NodeService::new(repo);
    let node = ServerNode {
        id: 0, name: "n1".into(), hostname: "h1".into(),
        ip_address: "1.2.3.4".into(), status: "online".into(),
        created_at: Utc::now(),
    };
    let id = svc.register_node(&node).await.unwrap();
    assert_eq!(id, 1);
    assert_eq!(svc.list_nodes().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_website_service() {
    let repo = Arc::new(InMemoryWebsiteRepository::new()) as Arc<dyn WebsiteRepository>;
    let svc = WebsiteService::new(repo);
    let ws = Website {
        id: 0, name: "site".into(), domain: "site.com".into(),
        root_path: "/var/www".into(), status: "active".into(),
        node_id: 1, engine: "nginx".into(),
        ssl_enabled: false, proxy_enabled: false, proxy_pass: None,
        created_at: Utc::now(),
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
        user_id: 42, username: "test_user".into(),
    }).await.unwrap();

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
        node_id: 1, node_name: "server-1".into(),
    }).await.unwrap();

    bus.publish(DomainEvent::WebsiteCreated {
        website_id: 10, domain: "example.com".into(),
    }).await.unwrap();

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
        user_id: 7, username: "multi".into(),
    }).await.unwrap();

    let received1 = rx1.try_recv().unwrap();
    let received2 = rx2.try_recv().unwrap();
    assert_eq!(received1, received2);
}

// ── 7. Plugin Registry ─────────────────────────────────

#[tokio::test]
async fn test_plugin_registry() {
    use chrono::Utc;
    let reg = PluginRegistry::new();
    let now = Utc::now();
    let p1 = Plugin {
        id: "p1".into(), name: "Logger".into(),
        version: "1.0".into(), enabled: true,
        author: "Test".into(), description: "Logger plugin".into(),
        wasm_hash: "abc123".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
    };
    let p2 = Plugin {
        id: "p2".into(), name: "Monitor".into(),
        version: "2.0".into(), enabled: false,
        author: "Test".into(), description: "Monitor plugin".into(),
        wasm_hash: "def456".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
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
        id: "dup".into(), name: "Dup".into(),
        version: "1.0".into(), enabled: true,
        author: "Test".into(), description: "Duplicate".into(),
        wasm_hash: "abc".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
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
        id: "test".into(), name: "Test".into(),
        version: "1.0".into(), enabled: true,
        author: "A".into(), description: "D".into(),
        wasm_hash: "h".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
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
        id: "p1".into(), name: "P1".into(),
        version: "1.0".into(), enabled: false,
        author: "A".into(), description: "D".into(),
        wasm_hash: "h".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
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
        id: "get-test".into(), name: "GetTest".into(),
        version: "1.0".into(), enabled: true,
        author: "A".into(), description: "D".into(),
        wasm_hash: "h".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
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
            Request::builder().uri("/api/plugins")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_plugin_load_and_get_endpoint() {
    use base64::{Engine as _, engine::general_purpose};
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // get
    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/plugins/test-plugin")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // get non-existent
    let res = app.clone()
        .oneshot(
            Request::builder().uri("/api/plugins/nonexistent")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_plugin_enable_disable_endpoint() {
    use base64::{Engine as _, engine::general_purpose};
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
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // disable
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/toggle-plugin/disable")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // enable
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/toggle-plugin/enable")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // unload
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/toggle-plugin")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_plugin_load_empty_wasm_rejected() {
    use base64::{Engine as _, engine::general_purpose};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::json!({
        "id": "empty-plugin",
        "name": "Empty",
        "wasm_base64": general_purpose::STANDARD.encode(&[]),
    });
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
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
            Request::builder().method(Method::POST).uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_plugin_enable_disable_nonexistent_fails() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // disable non-existent
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/nonexistent/disable")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // enable non-existent
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/nonexistent/enable")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_plugin_execute_endpoint() {
    use base64::{Engine as _, engine::general_purpose};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // WASM module exporting "run" -> i32 returning 42
    let wasm = wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 42))"#).unwrap();
    let b64 = general_purpose::STANDARD.encode(&wasm);
    // load
    let body = serde_json::json!({
        "id": "exec-plugin",
        "name": "ExecTest",
        "wasm_base64": b64,
    });
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // execute
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/exec-plugin/execute/run")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(r#"{"args":null}"#))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // execute with empty args
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/exec-plugin/execute/run")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(r#"{}"#))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_plugin_execute_nonexistent_fails() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/nonexistent/execute/run")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(r#"{}"#))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_plugin_execute_disabled_fails() {
    use base64::{Engine as _, engine::general_purpose};
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let wasm = wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 99))"#).unwrap();
    let b64 = general_purpose::STANDARD.encode(&wasm);
    // load
    let body = serde_json::json!({
        "id": "disable-exec",
        "name": "DisableExec",
        "wasm_base64": b64,
    });
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // disable
    let res = app.clone()
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/disable-exec/disable")
                .header(h.clone(), v.clone()).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // execute should fail
    let res = app
        .oneshot(
            Request::builder().method(Method::POST).uri("/api/plugins/disable-exec/execute/run")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(r#"{}"#))
                .unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_docker_get_container_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder().uri("/api/docker/containers/some-container")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
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
    assert!(matches!(err, AppError::Unauthorized));
}

#[tokio::test]
async fn test_jwt_wrong_secret() {
    let signer = JwtUtils::new("real-secret", 1);
    let verifier = JwtUtils::new("wrong-secret", 1);
    let token = signer.sign(1).unwrap();
    let err = verifier.verify(&token).unwrap_err();
    assert!(matches!(err, AppError::Unauthorized));
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
        (AppError::Unauthorized, StatusCode::UNAUTHORIZED),
        (AppError::Forbidden("x".into()), StatusCode::FORBIDDEN),
        (AppError::BadRequest("x".into()), StatusCode::BAD_REQUEST),
        (AppError::ValidationError("x".into()), StatusCode::BAD_REQUEST),
        (AppError::Internal("x".into()), StatusCode::INTERNAL_SERVER_ERROR),
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
    let user = kernel.app_state.user_service
        .create_user("k1", "hash", "admin").await.unwrap();
    assert_eq!(user.username, "k1");

    let node = kernel.app_state.node_service
        .register_node(&ServerNode {
            id: 0, name: "kn1".into(), hostname: "kh1".into(),
            ip_address: "10.0.0.1".into(), status: "online".into(),
            created_at: Utc::now(),
        }).await.unwrap();
    assert_eq!(node, 1);
}

#[tokio::test]
async fn test_kernel_with_factory() {
    let config = AppConfig::default();
    let factory = RepoFactory::new_in_memory();
    let kernel = FlameKernel::new_with_backend(config, factory);

    let user = kernel.app_state.user_service
        .create_user("factory_test", "hash", "user").await.unwrap();
    assert_eq!(user.username, "factory_test");

    let users = kernel.app_state.user_service.list_users().await.unwrap();
    assert_eq!(users.len(), 1);
}

// ── 12. End-to-End: Full Middleware Stack ───────────────

#[tokio::test]
async fn test_full_middleware_stack() {
    let app = setup_full_router().await;

    // Without auth → 401
    let res = app.clone()
        .oneshot(Request::builder().uri("/api/users").body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // With valid auth → 200
    let (h, v) = auth_header();
    let res = app.clone()
        .oneshot(Request::builder().uri("/api/users").header(h, v).body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Health → 200 (no auth)
    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await.unwrap();
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
                .method(Method::POST).uri("/api/users")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from("{}")).unwrap()
        ).await.unwrap();
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
                .method(Method::POST).uri("/api/users")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from("not-json")).unwrap()
        ).await.unwrap();
    assert!(res.status().is_client_error());
}

#[tokio::test]
async fn test_unknown_route() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder().uri("/api/nonexistent")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
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
                .method(Method::POST).uri("/health")
                .header(h, v).body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// ── 14. Plugin entity serialization ────────────────────

#[tokio::test]
async fn test_plugin_serde() {
    use chrono::Utc;
    let now = Utc::now();
    let p = Plugin {
        id: "test".into(), name: "Test".into(),
        version: "0.1".into(), enabled: false,
        author: "Author".into(), description: "Description".into(),
        wasm_hash: "hash".into(), created_at: now, updated_at: now,
        homepage: None, license: None, tags: vec![], config_schema: None, dependencies: vec![],
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
        id: "abc123".into(), image: "nginx:latest".into(),
        name: "web".into(), status: "running".into(),
        node_id: 1, created_at: Utc::now(),
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
        user_id: 1, username: "test".into(),
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
                .body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_operation_log_create() {
    let repo = Arc::new(InMemoryOperationLogRepository::new());
    let log = repo.create("testuser", "test.action", Some("target-1"), Some("127.0.0.1")).await.unwrap();
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
    repo.create("alice", "update", Some("y"), None).await.unwrap();

    let alice_logs = repo.list_by_username("alice").await.unwrap();
    assert_eq!(alice_logs.len(), 2);
    assert!(alice_logs.iter().all(|l| l.username == "alice"));
}

// ── 18. Event bus + handler ───────────────────────────

#[tokio::test]
async fn test_event_handler_subscribes_and_logs() {
    use flame_kernel::event::{EventBus, handler::EventHandler};
    let bus = EventBus::new(16);
    let rx = bus.subscribe();
    let handler = EventHandler::new();
    handler.spawn(rx);

    bus.publish(DomainEvent::UserCreated { user_id: 42, username: "test_user".into() }).await.unwrap();
    bus.publish(DomainEvent::NodeRegistered { node_id: 7, node_name: "test_node".into() }).await.unwrap();

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
    bus.publish(DomainEvent::UserCreated { user_id: 1, username: "x".into() }).await.unwrap();
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
    let entry = repo.create("docker", "info", "Container started", Some(r#"{"container_id":"abc"}"#)).await.unwrap();
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

    svc.log("system", "warn", "Disk usage high", None).await.unwrap();
    svc.log("docker", "info", "Container created", None).await.unwrap();

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
                .body(Body::empty()).unwrap()
        ).await.unwrap();
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
                .body(Body::empty()).unwrap()
        ).await.unwrap();
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
                .body(Body::empty()).unwrap()
        ).await.unwrap();
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
                .body(Body::empty()).unwrap()
        ).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}