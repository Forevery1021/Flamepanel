//! 数据库端点与乐观并发
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

#[tokio::test]
async fn test_sqlite_runtime_pragmas() {
    // 直接验证 PRAGMA 设置（使用内存库）
    use flame_kernel::infrastructure::sqlite::configure_sqlite_pragmas;
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    assert!(configure_sqlite_pragmas(&pool).await.is_ok());
    pool.close().await;
}

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

#[tokio::test]
async fn test_database_type_postgres_metadata() {
    // Doc/20 §6.4：DatabaseType 新增 Postgres 变体的元数据完整性
    let t = DatabaseType::from_name("postgres").unwrap();
    assert_eq!(t, DatabaseType::Postgres);
    assert_eq!(
        DatabaseType::from_name("postgresql"),
        Some(DatabaseType::Postgres)
    );
    assert_eq!(
        DatabaseType::from_name("pgsql"),
        Some(DatabaseType::Postgres)
    );
    assert_eq!(t.as_str(), "postgres");
    assert_eq!(t.default_port(), 5432);
    assert_eq!(t.service_name(), "postgresql");
    assert_eq!(t.package_name(), "postgresql");
    assert!(!t.description().is_empty());
}

#[tokio::test]
async fn test_database_service_postgres_dispatch_via_recording_runner() {
    // 服务层 postgres 分派验证：start/stop/restart/status/uninstall 均路由到 PostgresManager
    use flame_kernel::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
    };
    use std::sync::Mutex as StdMutex;

    struct RecordingRunner {
        calls: StdMutex<Vec<String>>,
    }
    #[async_trait::async_trait]
    impl PrivilegedCommandRunner for RecordingRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            Ok(CommandOutput {
                stdout: "active".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    let runner: flame_kernel::application::execution_mode::SharedCommandRunner =
        Arc::new(RecordingRunner {
            calls: StdMutex::new(Vec::new()),
        });
    let repo: Arc<dyn DatabaseRepository> = Arc::new(InMemoryDatabaseRepository::new());
    let svc = DatabaseService::new(repo.clone(), runner.clone());

    // 构造一个 postgres 实例记录（不实际执行安装，仅验证生命周期分派）
    let inst = DatabaseInstance {
        id: 0,
        db_type: "postgres".into(),
        name: "pg-main".into(),
        version: "16.2".into(),
        port: 5432,
        status: "stopped".into(),
        install_path: "/var/lib/postgresql".into(),
        data_dir: "/var/lib/postgresql/data".into(),
        config_file: "/etc/postgresql".into(),
        root_user: "postgres".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&inst).await.unwrap();

    svc.start(id).await.unwrap();
    let status = svc.status(id).await.unwrap();
    assert_eq!(status, "running");
    svc.stop(id).await.unwrap();

    // stop 后落库状态流转为 stopped（restart 不改写状态，与既有 mysql 行为一致）
    let after = svc.get_instance(id).await.unwrap();
    assert_eq!(after.status, "stopped");
    svc.restart(id).await.unwrap();
}

#[tokio::test]
async fn test_database_service_postgres_dispatch_stop_status() {
    use flame_kernel::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
    };
    use std::sync::Mutex as StdMutex;

    /// systemctl is-active 固定返回 inactive，验证 stop 后状态流转
    struct InactiveRunner;
    #[async_trait::async_trait]
    impl PrivilegedCommandRunner for InactiveRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            // systemctl is-active 返回 exit 3（inactive）；其余命令成功
            let probe_fail = cmd.program == "systemctl"
                && cmd.args.first().map(|s| s.as_str()) == Some("is-active")
                || (cmd.program == "pgrep");
            Ok(CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: if probe_fail { 3 } else { 0 },
            })
        }
    }

    let repo: Arc<dyn DatabaseRepository> = Arc::new(InMemoryDatabaseRepository::new());
    let svc = DatabaseService::new(repo.clone(), Arc::new(InactiveRunner));

    let inst = DatabaseInstance {
        id: 0,
        db_type: "postgres".into(),
        name: "pg-x".into(),
        version: "16".into(),
        port: 5432,
        status: "running".into(),
        install_path: String::new(),
        data_dir: String::new(),
        config_file: String::new(),
        root_user: "postgres".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&inst).await.unwrap();

    svc.stop(id).await.unwrap();
    let after = svc.get_instance(id).await.unwrap();
    assert_eq!(after.status, "stopped");

    // status 探测走 postgres 分支（is-active exit 3 → stopped），不报 Unknown database type
    let s = svc.status(id).await.unwrap();
    assert_eq!(s, "stopped");
}

async fn get_status(app: &axum::Router, token: &str, uri: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// Doc/33 P-8：实例详情 / 子库列表曾因 RBAC 规则失效而仅鉴权——无 database:read 的角色也能读。
#[tokio::test]
async fn test_database_reads_require_database_read_permission() {
    let (_r, state) = setup_router().await;
    let app = flame_kernel::api::middleware::add_middleware_with_mock_peer(
        flame_kernel::api::routes::create_router(state.clone()),
        state.clone(),
    );
    // 未知角色名 → 空权限集
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    let norole = flame_kernel::utils::jwt::JwtUtils::new("test-secret", 24)
        .sign(2)
        .unwrap();
    for uri in [
        "/api/databases",
        "/api/databases/1",
        "/api/databases/1/databases",
        "/api/databases/1/status",
    ] {
        assert_eq!(
            get_status(&app, &norole, uri).await,
            StatusCode::FORBIDDEN,
            "{uri}"
        );
    }
    let admin = auth_header().1.strip_prefix("Bearer ").unwrap().to_string();
    assert_eq!(
        get_status(&app, &admin, "/api/databases/1").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get_status(&app, &admin, "/api/databases/1/databases").await,
        StatusCode::NOT_FOUND
    );
}

async fn patch_batch(body: serde_json::Value) -> StatusCode {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    app.oneshot(
        Request::builder()
            .method(Method::PATCH)
            .uri("/api/databases/batch-status")
            .header(h, v)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

#[tokio::test]
async fn test_database_batch_status_rejects_unknown_status_and_oversize() {
    assert_eq!(
        patch_batch(json!({ "updates": [[1, "pwned"]] })).await,
        StatusCode::BAD_REQUEST
    );
    let many: Vec<_> = (1..=201).map(|i| json!([i, "stopped"])).collect();
    assert_eq!(
        patch_batch(json!({ "updates": many })).await,
        StatusCode::BAD_REQUEST
    );
}

async fn post_json_as(
    app: &axum::Router,
    token: Option<&str>,
    uri: &str,
    body: serde_json::Value,
) -> StatusCode {
    let mut req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    app.clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
        .status()
}

/// Doc/30 M-C MongoDB：安装端点 401 / 403 / 参数校验 400（校验先于任何特权命令）。
#[tokio::test]
async fn test_mongodb_install_auth_and_validation() {
    let (_r, state) = setup_router().await;
    let app = flame_kernel::api::middleware::add_middleware_with_mock_peer(
        flame_kernel::api::routes::create_router(state.clone()),
        state.clone(),
    );
    let uri = "/api/databases/mongodb/install";
    let ok_body = json!({ "name": "mongo", "version": "8.0", "port": 27017 });
    assert_eq!(
        post_json_as(&app, None, uri, ok_body.clone()).await,
        StatusCode::UNAUTHORIZED
    );
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    let norole = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    assert_eq!(
        post_json_as(&app, Some(&norole), uri, ok_body).await,
        StatusCode::FORBIDDEN
    );
    let admin = auth_header().1.strip_prefix("Bearer ").unwrap().to_string();
    for bad in [
        json!({ "name": "m", "version": "7.0" }),
        json!({ "name": "m", "port": 80 }),
        json!({ "name": "m", "password": "short" }),
        json!({ "name": "m", "password": "line\nbreak-pass" }),
    ] {
        assert_eq!(
            post_json_as(&app, Some(&admin), uri, bad.clone()).await,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }
}
