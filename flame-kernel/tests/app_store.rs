//! 应用商店
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
use common::{auth_header, bad_auth_header, crate_install_req, setup_full_router, setup_router};

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
        Arc::new(InMemoryAppSourceRepository::new()),
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

/// 商店安全配置端点（Doc/20 §4.2 收口）：GET 默认值 / PUT 更新回读 /
/// 未签名源创建门禁（HTTP 层）。
#[tokio::test]
async fn test_store_security_config_endpoints() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // ① 默认配置：optional 策略 + 内置白名单 4 项 + 空追加项
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/app-store/security")
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
    assert_eq!(body["signature_policy"], "optional");
    assert_eq!(body["builtin_registries"].as_array().unwrap().len(), 4);
    assert!(body["registry_whitelist"].as_array().unwrap().is_empty());
    assert_eq!(body["unsigned_source_count"], 0);

    // ② PUT 更新：策略切 required + 追加镜像仓库（大小写/空白归一）
    let update = serde_json::to_vec(&json!({
        "signature_policy": "required",
        "registry_whitelist": ["  Registry.Example.COM ", "harbor.internal:5000"]
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/app-store/security")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(update))
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
    assert_eq!(body["signature_policy"], "required");
    assert_eq!(
        body["registry_whitelist"],
        json!(["registry.example.com", "harbor.internal:5000"])
    );
    // 生效清单 = 内置 4 + 追加 2
    assert_eq!(body["effective_registries"].as_array().unwrap().len(), 6);

    // ③ 回读持久化值（GET 与 PUT 响应一致）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/app-store/security")
                .header(h.clone(), v.clone())
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
    assert_eq!(body["signature_policy"], "required");

    // ④ required 策略下：未签名的源创建被拒（即使显式确认）
    let create = serde_json::to_vec(&json!({
        "name": "unsigned",
        "source_type": "git",
        "url": "https://u.example.com/appstore",
        "confirm_unsigned": true
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/app-store/sources")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(create))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // ⑤ 切回 optional：未确认被拒（二次确认门禁）
    let back = serde_json::to_vec(&json!({ "signature_policy": "optional" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/app-store/security")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(back))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let no_confirm = serde_json::to_vec(&json!({
        "name": "unsigned",
        "source_type": "git",
        "url": "https://u.example.com/appstore"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/app-store/sources")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(no_confirm))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // ⑥ 显式确认后放行，且 unsigned_source_count 随源登记同步
    let confirm = serde_json::to_vec(&json!({
        "name": "unsigned",
        "source_type": "git",
        "url": "https://u.example.com/appstore",
        "confirm_unsigned": true
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/app-store/sources")
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(confirm))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/app-store/security")
                .header(h.clone(), v.clone())
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
    assert_eq!(body["unsigned_source_count"], 1);

    // ⑦ 非法输入：非法策略 / 空更新 / 非法仓库形态
    for payload in [
        json!({ "signature_policy": "mandatory" }),
        json!({}),
        json!({ "registry_whitelist": ["https://a.example.com"] }),
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/app-store/security")
                    .header(h.clone(), v.clone())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "payload={}", payload);
    }
}

async fn post_status(app: &axum::Router, uri: &str, body: serde_json::Value) -> StatusCode {
    let (h, v) = auth_header();
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// 路径 key 与请求体 package_key 不一致 → 400（旧实现静默安装请求体里的包）。
#[tokio::test]
async fn test_app_store_install_rejects_key_mismatch() {
    let app = setup_full_router().await;
    let status = post_status(
        &app,
        "/api/app-store/packages/nginx/install",
        json!({ "package_key": "wasm-hello", "mode": "wasm", "values": {} }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Doc/33 P-9：导入包的 key 含路径穿越 → 400，且不在商店目录外落盘。
#[tokio::test]
async fn test_app_store_import_rejects_traversal_key() {
    let app = setup_full_router().await;
    let tmp = tempfile::tempdir().unwrap();
    let pkg = tmp.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("app.json"),
        r#"{ "key": "../../fp-escaped-p9", "name": "X", "mode": "container", "versions": ["1.0"] }"#,
    )
    .unwrap();
    let status = post_status(
        &app,
        "/api/app-store/packages/import",
        json!({ "path": pkg.to_string_lossy() }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let status = post_status(
        &app,
        "/api/app-store/packages/batch-import",
        json!({ "paths": [pkg.to_string_lossy()] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// 无任何权限的角色：商店 / 插件 / 数据库读端点一律 403。
#[tokio::test]
async fn test_catalog_reads_forbidden_without_permissions() {
    let (_r, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(
        routes::create_router(state.clone()),
        state.clone(),
    );
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    for uri in [
        "/api/app-store/packages",
        "/api/app-store/packages/nginx",
        "/api/app-store/installed/1",
        "/api/app-store/installed/1/logs",
        "/api/app-store/wasm-builtins",
        "/api/plugins",
        "/api/plugins/p1",
        "/api/plugins/p1/settings",
        "/api/databases/1",
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "{uri}");
    }
}

/// Doc/30 M-C 升级三方合并：预览端点 401 / 403 / 404 / 200（同版本 up_to_date），
/// 非法冲突策略 400。
#[tokio::test]
async fn test_app_store_upgrade_preview_endpoint() {
    let (_r, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(
        routes::create_router(state.clone()),
        state.clone(),
    );
    let get = |token: Option<String>, uri: String| {
        let app = app.clone();
        async move {
            let mut req = Request::builder().uri(uri);
            if let Some(t) = token {
                req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
            }
            let res = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
            let status = res.status();
            let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                .await
                .unwrap();
            (
                status,
                serde_json::from_slice::<serde_json::Value>(&bytes).ok(),
            )
        }
    };
    let admin = auth_header().1.strip_prefix("Bearer ").unwrap().to_string();
    let preview =
        |id: i64| format!("/api/app-store/installed/{id}/upgrade-preview?target_version=1.0.0");

    assert_eq!(get(None, preview(1)).await.0, StatusCode::UNAUTHORIZED);
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    let norole = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    assert_eq!(get(Some(norole), preview(1)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(
        get(Some(admin.clone()), preview(999)).await.0,
        StatusCode::NOT_FOUND
    );

    // 装一个 wasm 应用：同版本预览 → up_to_date，无合并结果
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/app-store/packages/wasm-hello/install")
                .header(header::AUTHORIZATION, format!("Bearer {admin}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_key": "wasm-hello",
                        "version": "1.0.0",
                        "mode": "wasm",
                        "name": "preview-hello",
                        "values": { "name": "preview-hello" }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let id = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["id"]
        .as_i64()
        .unwrap();
    let (status, body) = get(Some(admin.clone()), preview(id)).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["up_to_date"], true);
    assert_eq!(body["from_version"], "1.0.0");
    assert!(body["merge"].is_null());

    // 冲突策略只接受 abort / local / upstream（及 ours / theirs 别名）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/app-store/installed/{id}/upgrade?target_version=1.0.0&conflict_strategy=yolo"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {admin}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
