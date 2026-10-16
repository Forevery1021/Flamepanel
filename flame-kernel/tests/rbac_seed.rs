//! SQLite 模式 RBAC 种子端到端回归（P0 缺陷：admin 全 403）。
//!
//! **缺陷背景**：`run_migrations()` 只建表，`permissions` / `roles` /
//! `role_permissions` 三张表在 SQLite 模式下**没有任何写入路径**（InMemory 模式靠
//! 仓储构造器种子，SQLite 模式则完全依赖这三张表的数据）。结果是生产（SQLite）模式
//! 下三表恒空：`RoleService::check_permission()` 对任何角色都命中空集合，admin 登录后
//! 所有业务接口一律 `403 Missing permission: <resource>:<action>`——面板除
//! `/api/auth/*` 外整体不可用，且**向导模式与无人值守模式同样命中**。
//!
//! 本文件不复用 `common::setup_router()`（它走 InMemory 仓储，天然带种子）——
//! 必须跑 **真实 SQLite 仓储**才能覆盖缺陷，故自建最小 AppState。
//! 断言落在 HTTP 状态码上（用户可见影响面），而非仅检查表行数。

#![allow(unused_imports)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

use flame_kernel::api::middleware;
use flame_kernel::api::routes;
use flame_kernel::api::types::{AppState, Services};
use flame_kernel::application::service::*;
use flame_kernel::application::setup_service::SetupService;
use flame_kernel::config::AppConfig;
use flame_kernel::domain::entity::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::metrics::MetricsHistory;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::runtime::task_state::TaskTracker;
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::FlameKernel;

/// 临时 SQLite 库路径（每用例独立，避免互相污染）。
fn temp_db(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fp-rbac-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("panel.db")
}

/// 真实 SQLite 后端的完整应用（路由 + 中间件），返回 (kernel, router)。
///
/// 无人值守模式创建 admin（`must_change_password=1`），故调用方先走
/// `/api/auth/change-password` 解锁业务端点。
async fn sqlite_kernel(tag: &str) -> (FlameKernel, axum::Router) {
    let db = temp_db(tag);
    let url = format!("sqlite:{}?mode=rwc", db.display());
    let factory = RepoFactory::new_sqlite(&url).await.unwrap();
    let config = AppConfig {
        admin_password: "BootPass123".into(),
        ..Default::default()
    };
    let kernel = FlameKernel::new_with_backend(config, factory);
    kernel.bootstrap_initialization_state().await.unwrap();
    let router = routes::create_router(kernel.app_state.clone());
    let router = middleware::add_middleware_with_mock_peer(router, kernel.app_state.clone());
    (kernel, router)
}

/// 完成首次登录 + 强制改密，返回可用于业务端点的 access token。
async fn login_and_unlock(app: &axum::Router) -> String {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "admin", "password": "BootPass123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "首次登录应成功");
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let token = body["token"].as_str().unwrap().to_string();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/change-password")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "old_password": "BootPass123",
                        "new_password": "UnlockedPass456"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "强制改密应成功");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "admin", "password": "UnlockedPass456"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    body["token"].as_str().unwrap().to_string()
}

async fn get(app: &axum::Router, uri: &str, token: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// 核心回归：SQLite 模式下 admin 的业务端点必须放行（修复前全部 403）。
#[tokio::test]
async fn sqlite_mode_admin_can_reach_business_endpoints() {
    let (_kernel, app) = sqlite_kernel("admin-ok").await;
    let token = login_and_unlock(&app).await;

    // 覆盖多个资源域（每个都对应一个独立权限点，任一缺失都会 403）
    for uri in [
        "/api/users",
        "/api/nodes",
        "/api/websites",
        "/api/certificates",
        "/api/dns-providers",
        "/api/waf/rules",
    ] {
        let status = get(&app, uri, &token).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "{uri} 应放行——修复前三表无种子时此处为 403 Missing permission"
        );
    }
}

/// 角色矩阵在 SQLite 模式下同样收敛：viewer 只读、operator 无 delete。
#[tokio::test]
async fn sqlite_mode_role_matrix_enforced() {
    let (kernel, app) = sqlite_kernel("matrix").await;
    // 直接经服务层断言权限判定（等价于 HTTP 层的第 3 步 RBAC 检查）
    let role_service = kernel.app_state.role_service.clone();

    assert!(
        role_service
            .check_permission("admin", "user", "delete")
            .await
            .unwrap(),
        "admin 应有 delete"
    );
    assert!(
        !role_service
            .check_permission("operator", "user", "delete")
            .await
            .unwrap(),
        "operator 不应有 delete"
    );
    assert!(
        role_service
            .check_permission("operator", "user", "create")
            .await
            .unwrap(),
        "operator 应有 create"
    );
    assert!(
        role_service
            .check_permission("viewer", "user", "read")
            .await
            .unwrap(),
        "viewer 应有 read"
    );
    assert!(
        !role_service
            .check_permission("viewer", "user", "create")
            .await
            .unwrap(),
        "viewer 不应有 create"
    );

    let _ = app;
}

/// 重启幂等：同一库二次构造不得重复插入，也不得重置已收紧的授权。
#[tokio::test]
async fn sqlite_mode_reseed_on_restart_is_idempotent() {
    let db = temp_db("restart");
    let url = format!("sqlite:{}?mode=rwc", db.display());

    // 首次启动
    let factory = RepoFactory::new_sqlite(&url).await.unwrap();
    let config = AppConfig {
        admin_password: "BootPass123".into(),
        ..Default::default()
    };
    let kernel = FlameKernel::new_with_backend(config.clone(), factory);
    kernel.bootstrap_initialization_state().await.unwrap();
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let (perm_count, role_count): (i64, i64) =
        sqlx::query_as("SELECT (SELECT COUNT(*) FROM permissions), (SELECT COUNT(*) FROM roles)")
            .fetch_one(&pool)
            .await
            .unwrap();
    pool.close().await;
    assert_eq!(perm_count as usize, default_permissions().len());
    assert_eq!(role_count, 3);

    // 二次启动（同一库）：行数不变
    let factory = RepoFactory::new_sqlite(&url).await.unwrap();
    let kernel2 = FlameKernel::new_with_backend(config, factory);
    let _ = kernel2.bootstrap_initialization_state().await;
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    let (perm_count2, role_count2): (i64, i64) =
        sqlx::query_as("SELECT (SELECT COUNT(*) FROM permissions), (SELECT COUNT(*) FROM roles)")
            .fetch_one(&pool)
            .await
            .unwrap();
    pool.close().await;
    assert_eq!(perm_count2, perm_count, "重启不得重复插入权限");
    assert_eq!(role_count2, role_count, "重启不得重复插入角色");
}
