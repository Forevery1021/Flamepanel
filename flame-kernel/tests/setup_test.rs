//! 首次部署 Setup 向导集成测试（B5）。
//!
//! 覆盖：in_progress / 完成链路 / 重复初始化 409 / 已有用户 409 / 数据库步 /
//! 弱密码 / 无人值守种子 / 老库补写。限流档位映射见 `api/rate_limiter.rs` 单测。

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

use flame_kernel::api::middleware;
use flame_kernel::api::routes;
use flame_kernel::api::types::{AppState, Services};
use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::execution_mode::SharedCommandRunner;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
use flame_kernel::application::setup_service::SetupService;
use flame_kernel::application::task_service::TaskService;
use flame_kernel::config::AppConfig;
use flame_kernel::domain::entity::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::runtime::task_state::TaskTracker;
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::FlameKernel;

fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fp-setup-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 全新（无用户）的完整路由 + 中间件栈，用于公开端点测试。
async fn fresh_router(tag: &str) -> axum::Router {
    let state = fresh_state(tag).await;
    let router = routes::create_router(state.clone());
    middleware::add_middleware(router, state)
}

async fn fresh_state(tag: &str) -> AppState {
    let runner: SharedCommandRunner =
        Arc::new(flame_kernel::infrastructure::execution::EmbeddedCommandRunner);
    let data_dir = temp_data_dir(tag);
    let user_repo = Arc::new(InMemoryUserRepository::new());
    let settings_repo = Arc::new(InMemorySettingsRepository::new());
    let user_service = Arc::new(UserService::new(
        user_repo,
        EventBus::new(100),
        AuthCache::new(),
    ));
    let settings_service = Arc::new(SettingsService::new(settings_repo));
    let setup_service = Arc::new(SetupService::new(
        user_service.clone(),
        settings_service.clone(),
        EventBus::new(100),
        data_dir,
        runner.clone(),
        false,
    ));

    let app_store_service = Arc::new(AppStoreService::new(
        Arc::new(InMemoryAppPackageRepository::new()),
        Arc::new(InMemoryInstalledAppRepository::new()),
        Arc::new(DockerService::new(
            Arc::new(InMemoryDockerRepository::new()),
        )),
        Arc::new(WebServerService::new(
            Arc::new(InMemoryWebServerRepository::new()),
            runner.clone(),
        )),
        Arc::new(DatabaseService::new(
            Arc::new(InMemoryDatabaseRepository::new()),
            runner.clone(),
        )),
        Arc::new(PluginSandbox::new()),
        Arc::new(PluginRegistry::new()),
        Arc::new(InMemoryPluginRepository::new()),
        AppStoreService::default_apps_dir(),
        EventBus::new(100),
        flame_kernel::infrastructure::app_store::default_ports(runner.clone()),
    ));

    let metrics_history = Arc::new(Mutex::new(MetricsHistory::new(10)));
    let (metrics_tx, _) = tokio::sync::broadcast::channel::<MetricsSnapshot>(16);
    let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(16);
    let backup_dir = temp_data_dir(&format!("{tag}-backup"));

    let services = Services {
        user_service,
        node_service: Arc::new(NodeService::new(
            Arc::new(InMemoryNodeRepository::new()),
            EventBus::new(100),
        )),
        website_service: Arc::new(WebsiteService::new(
            Arc::new(InMemoryWebsiteRepository::new()),
            EventBus::new(100),
        )),
        docker_service: Arc::new(DockerService::new(
            Arc::new(InMemoryDockerRepository::new()),
        )),
        role_service: Arc::new(RoleService::new(
            Arc::new(InMemoryRoleRepository::new()),
            Arc::new(InMemoryPermissionRepository::new()),
            AuthCache::new(),
        )),
        permission_service: Arc::new(PermissionService::new(Arc::new(
            InMemoryPermissionRepository::new(),
        ))),
        operation_log_service: Arc::new(OperationLogService::new(Arc::new(
            InMemoryOperationLogRepository::new(),
        ))),
        outbox_service: Arc::new(OutboxService::new(
            Arc::new(InMemoryOutboxRepository::new()),
        )),
        memo_service: Arc::new(MemoService::new(Arc::new(InMemoryMemoRepository::new()))),
        log_service: Arc::new(LogService::new(Arc::new(InMemoryLogRepository::new()))),
        plugin_sandbox: Arc::new(PluginSandbox::new()),
        plugin_registry: Arc::new(PluginRegistry::new()),
        plugin_repo: Arc::new(InMemoryPluginRepository::new()),
        app_store_service,
        web_server_service: Arc::new(WebServerService::new(
            Arc::new(InMemoryWebServerRepository::new()),
            runner.clone(),
        )),
        settings_service,
        database_service: Arc::new(DatabaseService::new(
            Arc::new(InMemoryDatabaseRepository::new()),
            runner.clone(),
        )),
        firewall_service: Arc::new(FirewallService::new_embedded(Arc::new(
            InMemoryFirewallRepository::new(),
        ))),
        scheduled_task_service: Arc::new(ScheduledTaskService::new(Arc::new(
            InMemoryScheduledTaskRepository::new(),
        ))),
        task_service: Arc::new(TaskService::new(TaskTracker::new())),
        backup_service: Arc::new(BackupService::new(
            backup_dir.join("app.db"),
            backup_dir.join("backups"),
        )),
        setup_service,
        event_bus: EventBus::new(100),
    };

    AppState::new(
        "test-secret".to_string(),
        services,
        metrics_history,
        metrics_tx,
        log_tx,
        TerminalManager::new(),
    )
}

fn admin_payload() -> serde_json::Value {
    serde_json::json!({
        "step": "admin",
        "admin": { "username": "admin", "password": "Admin12345" },
        "theme": "flame",
        "language": "zh-CN"
    })
}

async fn post_json(
    app: &axum::Router,
    uri: &str,
    body: serde_json::Value,
    ip: &str,
) -> (StatusCode, serde_json::Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("X-Real-IP", ip)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, v)
}

async fn get_json(app: &axum::Router, uri: &str, ip: &str) -> (StatusCode, serde_json::Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("X-Real-IP", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, v)
}

// ── 1. 新装状态 ─────────────────────────────────────────

#[tokio::test]
async fn status_in_progress_on_fresh_install() {
    let app = fresh_router("fresh").await;
    let (status, body) = get_json(&app, "/api/setup/status", "10.0.0.11").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "in_progress");
    assert_eq!(body["theme"], "flame");
    assert_eq!(body["language"], "zh-CN");
}

// ── 2. 完成链路 ─────────────────────────────────────────

#[tokio::test]
async fn initialize_admin_completes_full_chain() {
    let app = fresh_router("chain").await;
    let (status, body) =
        post_json(&app, "/api/setup/initialize", admin_payload(), "10.0.0.12").await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["status"], "completed");
    let token = body["token"].as_str().unwrap().to_string();
    assert!(!body["refresh_token"].as_str().unwrap().is_empty());
    assert_eq!(body["username"], "admin");
    assert_eq!(body["role"], "admin");

    // 状态翻转 + 主题落库
    let (status, body) = get_json(&app, "/api/setup/status", "10.0.0.12").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert_eq!(body["theme"], "flame");

    // 返回的 token 可用（直接登录态访问受保护端点）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/auth/me")
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Real-IP", "10.0.0.12")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "issued token must be usable");
}

// ── 3. 重复初始化 409 ──────────────────────────────────

#[tokio::test]
async fn initialize_again_rejected_with_409() {
    let app = fresh_router("dup").await;
    let (s1, _) = post_json(&app, "/api/setup/initialize", admin_payload(), "10.0.0.13").await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, body) = post_json(&app, "/api/setup/initialize", admin_payload(), "10.0.0.13").await;
    assert_eq!(s2, StatusCode::CONFLICT, "body: {body}");
}

// ── 4. 已有用户 409 ────────────────────────────────────

#[tokio::test]
async fn initialize_with_existing_users_rejected() {
    let state = fresh_state("users").await;
    state
        .user_service
        .create_user("legacy", "hash", "admin")
        .await
        .unwrap();
    let app = routes::create_router(state);
    let (status, body) =
        post_json(&app, "/api/setup/initialize", admin_payload(), "10.0.0.14").await;
    assert_eq!(status, StatusCode::CONFLICT, "body: {body}");
}

// ── 5. 数据库步（sqlite）───────────────────────────────

#[tokio::test]
async fn initialize_database_step_sqlite() {
    let app = fresh_router("db").await;
    let payload = serde_json::json!({
        "step": "database",
        "database": { "db_type": "sqlite" }
    });
    let (status, body) = post_json(&app, "/api/setup/initialize", payload, "10.0.0.15").await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["status"], "in_progress");
    // 设置落库：状态查询读 db_type 应由后续 admin 步前保留（此处经 status 响应间接验证 theme 默认）
    let (_, body) = get_json(&app, "/api/setup/status", "10.0.0.15").await;
    assert_eq!(body["status"], "in_progress");
}

#[tokio::test]
async fn initialize_database_step_unsupported_type() {
    let app = fresh_router("db2").await;
    let payload = serde_json::json!({
        "step": "database",
        "database": { "db_type": "oracle" }
    });
    let (status, body) = post_json(&app, "/api/setup/initialize", payload, "10.0.0.16").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

// ── 6. 校验 ────────────────────────────────────────────

#[tokio::test]
async fn initialize_unknown_step_rejected() {
    let app = fresh_router("step").await;
    let payload = serde_json::json!({ "step": "bogus" });
    let (status, body) = post_json(&app, "/api/setup/initialize", payload, "10.0.0.17").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

#[tokio::test]
async fn initialize_weak_password_rejected() {
    let app = fresh_router("pw").await;
    let payload = serde_json::json!({
        "step": "admin",
        "admin": { "username": "admin", "password": "short" }
    });
    let (status, body) = post_json(&app, "/api/setup/initialize", payload, "10.0.0.18").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

#[tokio::test]
async fn initialize_missing_admin_section_rejected() {
    let app = fresh_router("nosect").await;
    let payload = serde_json::json!({ "step": "admin" });
    let (status, body) = post_json(&app, "/api/setup/initialize", payload, "10.0.0.19").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}

// ── 7-9. 启动种子分叉（B4）─────────────────────────────

#[tokio::test]
async fn unattended_boot_seeds_admin() {
    let config = AppConfig {
        admin_password: "UnattendedPass123".into(),
        ..Default::default()
    };
    let kernel = FlameKernel::new_with_backend(config, RepoFactory::new_in_memory());

    let outcome = kernel.bootstrap_initialization_state().await.unwrap();
    match outcome {
        flame_kernel::InitializationOutcome::SeededUnattended { username } => {
            assert_eq!(username, "admin")
        }
        other => panic!("expected unattended seed, got {other:?}"),
    }
    let users = kernel.app_state.user_service.list_users().await.unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].role, "admin");
    assert!(
        kernel
            .app_state
            .settings_service
            .get("setup_completed_at")
            .await
            .unwrap()
            .is_some(),
        "unattended seed must write setup_completed_at"
    );
}

#[tokio::test]
async fn wizard_boot_leaves_pending() {
    let config = AppConfig::default();
    let kernel = FlameKernel::new_with_backend(config, RepoFactory::new_in_memory());

    let outcome = kernel.bootstrap_initialization_state().await.unwrap();
    assert!(
        matches!(outcome, flame_kernel::InitializationOutcome::PendingWizard),
        "fresh install without admin_password must stay pending"
    );
    let users = kernel.app_state.user_service.list_users().await.unwrap();
    assert!(users.is_empty(), "wizard mode must NOT seed users");
}

#[tokio::test]
async fn legacy_users_backfill_completed_at() {
    let config = AppConfig::default();
    let kernel = FlameKernel::new_with_backend(config, RepoFactory::new_in_memory());
    // 老库：已有用户但缺 setup_completed_at
    kernel
        .app_state
        .user_service
        .create_user("legacy", "hash", "admin")
        .await
        .unwrap();
    assert!(kernel
        .app_state
        .settings_service
        .get("setup_completed_at")
        .await
        .unwrap()
        .is_none());

    let outcome = kernel.bootstrap_initialization_state().await.unwrap();
    assert!(
        matches!(
            outcome,
            flame_kernel::InitializationOutcome::LegacyBackfilled
        ),
        "legacy db must be backfilled"
    );
    assert!(
        kernel
            .app_state
            .settings_service
            .get("setup_completed_at")
            .await
            .unwrap()
            .is_some(),
        "setup_completed_at must be backfilled for legacy db"
    );
}

#[tokio::test]
async fn initialized_boot_reports_completed() {
    let config = AppConfig::default();
    let kernel = FlameKernel::new_with_backend(config, RepoFactory::new_in_memory());
    kernel
        .app_state
        .user_service
        .create_user("admin", "hash", "admin")
        .await
        .unwrap();
    kernel
        .app_state
        .settings_service
        .set("setup_completed_at", "2026-01-01T00:00:00Z")
        .await
        .unwrap();

    let outcome = kernel.bootstrap_initialization_state().await.unwrap();
    assert!(
        matches!(outcome, flame_kernel::InitializationOutcome::Completed),
        "normal state must be Completed"
    );
}
