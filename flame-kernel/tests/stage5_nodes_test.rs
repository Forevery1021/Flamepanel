//! Stage5 多节点远程调用集成测试
//!
//! 覆盖：Agent 注册（公开白名单）、远程命令权限、远程命令上游不可达、
//! 批量命令空列表校验、远程文件列表/上传/下载权限。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use flame_kernel::api::middleware;
use flame_kernel::api::{routes, types::AppState};

async fn setup_router() -> axum::Router {
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    // 复用主集成测试的组装逻辑：构建最小 AppState
    let state = minimal_state().await;
    let router = routes::create_router(state.clone());
    middleware::add_middleware(router, state)
}

use std::sync::Arc;
use tokio::sync::Mutex;

use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
use flame_kernel::domain::entity::*;
use flame_kernel::domain::repository::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::utils::jwt::JwtUtils;

fn auth_header() -> (header::HeaderName, String) {
    let jwt = JwtUtils::new("test-secret", 24);
    let token = jwt.sign(1).unwrap();
    (header::AUTHORIZATION, format!("Bearer {}", token))
}

async fn minimal_state() -> AppState {
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
    let backup_dir = std::env::temp_dir().join(format!("stage5-backup-{}", std::process::id()));
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::write(backup_dir.join("app.db"), b"backup-seed-db").unwrap();
    let services = flame_kernel::api::types::Services {
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
        task_service: Arc::new(flame_kernel::application::task_service::TaskService::new(
            flame_kernel::runtime::task_state::TaskTracker::new(),
        )),
        setup_service: Arc::new(flame_kernel::application::setup_service::SetupService::new(
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
    state
}

async fn register_test_node(app: &axum::Router, token: &str, ip: &str, port: u16) -> i64 {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes/register")
                .header("X-Bootstrap-Token", "test-bootstrap-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "name": "agent-node",
                        "host": ip,
                        "ip_address": ip,
                        "agent_port": port,
                        "auth_token": token,
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "register should succeed");
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["id"].as_i64().unwrap()
}

#[tokio::test]
async fn stage5_agent_register_public_endpoint() {
    let app = setup_router().await;
    let id = register_test_node(&app, "agent-secret", "10.0.0.9", 9527).await;
    assert!(id > 0);
    // 已注册节点可被列表读取
    let (h, v) = auth_header();
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

/// A3.2：注册端点必须携带匹配的 X-Bootstrap-Token，缺失/错误一律 401。
#[tokio::test]
async fn stage5_agent_register_requires_bootstrap_token() {
    let app = setup_router().await;
    let body = serde_json::to_string(&json!({
        "name": "agent-node",
        "host": "10.0.0.9",
        "ip_address": "10.0.0.9",
        "agent_port": 9527,
        "auth_token": "agent-secret",
    }))
    .unwrap();

    // 无 token → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes/register")
                .header("Content-Type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 错误 token → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes/register")
                .header("X-Bootstrap-Token", "wrong-token")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 正确 token → 200
    let id = register_test_node(&app, "agent-secret", "10.0.0.9", 9527).await;
    assert!(id > 0);
}

#[tokio::test]
async fn stage5_remote_execute_requires_auth() {
    let app = setup_router().await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes/1/execute")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"command":"echo hi"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stage5_remote_execute_agent_unreachable() {
    let app = setup_router().await;
    let (h, v) = auth_header();
    // 端口 1 无服务 → 上游不可达应返回 5xx
    let id = register_test_node(&app, "agent-secret", "127.0.0.1", 1).await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/nodes/{}/execute", id))
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"command":"echo hi"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.status().is_server_error(),
        "expected 5xx for unreachable agent, got {}",
        res.status()
    );
}

#[tokio::test]
async fn stage5_batch_execute_empty_ids_rejected() {
    let app = setup_router().await;
    let (h, v) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/nodes/batch-execute")
                .header(h, v)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"node_ids":[],"command":"ls"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stage5_remote_files_requires_permission() {
    let app = setup_router().await;
    // 未认证访问节点文件 → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/nodes/1/files?path=/tmp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── A1：Agent 心跳全链路联调 ─────────────────────────────────

fn heartbeat_body() -> String {
    serde_json::to_string(&json!({
        "cpu_usage": 12.5,
        "memory_usage_percent": 33.3,
        "disk_usage_percent": 45.0,
        "load_one": 0.8,
    }))
    .unwrap()
}

async fn send_heartbeat_req(
    app: &axum::Router,
    node_id: i64,
    token: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/nodes/heartbeat/{}", node_id))
        .header("Content-Type", "application/json");
    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    app.clone()
        .oneshot(builder.body(Body::from(heartbeat_body())).unwrap())
        .await
        .unwrap()
}

/// A1 回归：注册 → 心跳（带注册时 token）→ 再心跳，全链路断言 200；
/// token 错误断言 401（曾回归为全部 401）。
#[tokio::test]
async fn agent_heartbeat_full_chain_with_token() {
    let app = setup_router().await;
    let id = register_test_node(&app, "agent-secret-abc", "10.0.0.9", 9527).await;

    // 带正确 token 的心跳 → 200
    let res = send_heartbeat_req(&app, id, Some("agent-secret-abc")).await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "heartbeat with token should succeed"
    );
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["status"], "ok");
    assert!(v["last_heartbeat_at"].is_string() || v["last_heartbeat_at"].is_null());

    // 第二次心跳仍成功
    let res = send_heartbeat_req(&app, id, Some("agent-secret-abc")).await;
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "second heartbeat should succeed"
    );

    // 错误 token → 401
    let res = send_heartbeat_req(&app, id, Some("wrong-token")).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 无 token → 401（新 Agent 必须带 token）
    let res = send_heartbeat_req(&app, id, None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 节点不存在 → 404
    let res = send_heartbeat_req(&app, 9999, Some("agent-secret-abc")).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
