//! 插件注册表 / 沙箱 / HTTP 端点
//!
//! 从巨型 `integration_test.rs` 按领域拆分而来（S12），复用 `common` 共享工具。

#![allow(unused_imports)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
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
        capabilities: vec![],
        settings: Default::default(),
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
        capabilities: vec![],
        settings: Default::default(),
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
        capabilities: vec![],
        settings: Default::default(),
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
        capabilities: vec![],
        settings: Default::default(),
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
        capabilities: vec![],
        settings: Default::default(),
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
        capabilities: vec![],
        settings: Default::default(),
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
        capabilities: vec![],
        settings: Default::default(),
        wasm_base64: String::new(),
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Plugin = serde_json::from_str(&json).unwrap();
    assert_eq!(p.id, back.id);
    assert_eq!(p.enabled, back.enabled);
    assert_eq!(p.author, back.author);
    assert_eq!(p.wasm_hash, back.wasm_hash);
}

async fn post_json(app: &axum::Router, uri: &str, body: serde_json::Value) -> StatusCode {
    let (h, v) = auth_header();
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// 插件加载参数越界：非法 ID、内存上限绕过（usize::MAX）、超时为 0 → 400；设置键非法 → 400。
#[tokio::test]
async fn test_plugin_load_rejects_out_of_range_params() {
    use base64::{engine::general_purpose, Engine as _};
    let app = setup_full_router().await;
    let b64 = general_purpose::STANDARD.encode(wat::parse_str("(module)").unwrap());
    for body in [
        json!({ "id": "../evil", "wasm_base64": b64 }),
        json!({ "id": "", "wasm_base64": b64 }),
        json!({ "id": ".hidden", "wasm_base64": b64 }),
        json!({ "id": "a".repeat(65), "wasm_base64": b64 }),
        json!({ "id": "mem", "wasm_base64": b64, "memory_limit_bytes": u64::MAX }),
        json!({ "id": "mem", "wasm_base64": b64, "memory_limit_bytes": 1024 }),
        json!({ "id": "tmo", "wasm_base64": b64, "timeout_ms": 0 }),
    ] {
        assert_eq!(
            post_json(&app, "/api/plugins", body.clone()).await,
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
    // 合法边界可加载
    assert_eq!(
        post_json(
            &app,
            "/api/plugins",
            json!({ "id": "ok.plugin-1", "wasm_base64": b64, "memory_limit_bytes": 1048576, "timeout_ms": 300000 })
        )
        .await,
        StatusCode::OK
    );
    for body in [
        json!({ "key": "", "value": "x" }),
        json!({ "key": "a b", "value": "x" }),
        json!({ "key": "k", "value": "x".repeat(4097) }),
    ] {
        assert_eq!(
            post_json(&app, "/api/plugins/ok.plugin-1/settings", body.clone()).await,
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
    assert_eq!(
        post_json(
            &app,
            "/api/plugins/ok.plugin-1/settings",
            json!({ "key": "theme.mode", "value": "dark" })
        )
        .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_plugin_endpoints_require_auth() {
    let app = setup_full_router().await;
    let (h, v) = bad_auth_header();
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
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
