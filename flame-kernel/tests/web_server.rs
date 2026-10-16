//! Web 服务器预设/引擎切换/乐观并发
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

#[tokio::test]
async fn test_web_server_capabilities_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 查询 nginx 配套能力：应包含 php/cache/extra_modules/auto_https，不含 lua
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/web-servers/capabilities?engine=nginx")
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
    assert_eq!(value["engine"], "nginx");
    let caps = value["capabilities"].as_array().unwrap();
    let keys: Vec<&str> = caps.iter().map(|c| c["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"php"));
    assert!(keys.contains(&"cache"));
    assert!(keys.contains(&"extra_modules"));
    assert!(keys.contains(&"auto_https"));
    assert!(!keys.contains(&"lua"));
    // 每个能力都有标签与提示
    for cap in caps {
        assert!(!cap["label_zh"].as_str().unwrap().is_empty());
        assert!(cap.get("packages").is_some());
        assert!(cap.get("hint").is_some());
    }

    // OpenResty 支持 lua；Apache 不支持 lua
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/web-servers/capabilities?engine=openresty")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let keys: Vec<&str> = value["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"lua"));

    // 未知引擎 → 400
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/web-servers/capabilities?engine=whatever")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_web_server_capabilities_install_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 未知引擎 → 400
    let body = serde_json::json!({ "engine": "whatever", "capabilities": ["php"] });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers/capabilities/install")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 未知配套能力 → 400
    let body = serde_json::json!({ "engine": "nginx", "capabilities": ["bogus"] });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers/capabilities/install")
                .header("Content-Type", "application/json")
                .header(h.clone(), v.clone())
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 合法请求：InMemory 环境无包管理器，预期返回 installed/failed 结构且非 401/403
    let body = serde_json::json!({ "engine": "nginx", "capabilities": ["php", "cache"] });
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/web-servers/capabilities/install")
                .header("Content-Type", "application/json")
                .header(h, v)
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(res.status(), StatusCode::FORBIDDEN);
}

/// 实例登记字段收紧：`config_path` 会被读取 / 写入 / 作为启动参数，`binary_path` 会被特权执行，
/// 二者此前不设限——拥有 `web_server:create|update` 的角色可读任意文件、执行任意程序。
#[tokio::test]
async fn test_web_server_instance_paths_are_restricted() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let send = |method: Method, uri: String, body: serde_json::Value| {
        let app = app.clone();
        let (h, v) = (h.clone(), v.clone());
        async move {
            let res = app
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .header(h, v)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = res.status();
            let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                .await
                .unwrap();
            let json: serde_json::Value =
                serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
            (status, json)
        }
    };

    // 任意文件 / 任意程序 / 未知引擎 → 400，且不落库
    for body in [
        json!({"engine": "nginx", "config_path": "/etc/shadow"}),
        json!({"engine": "nginx", "config_path": "/etc/nginx/../shadow"}),
        json!({"engine": "nginx", "binary_path": "/usr/bin/python3"}),
        json!({"engine": "caddy", "binary_path": "/bin/sh"}),
    ] {
        let (status, _) = send(Method::POST, "/api/web-servers".into(), body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }

    // 合法登记（默认路径 / 系统 bin 目录下同名二进制）照常成功
    let (status, created) = send(
        Method::POST,
        "/api/web-servers".into(),
        json!({"engine": "nginx", "binary_path": "/usr/local/sbin/nginx"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["config_path"], "/etc/nginx/nginx.conf");
    let id = created["id"].as_i64().unwrap();

    // 更新同样受限；改回合法值可用
    for body in [
        json!({"config_path": "/var/lib/flamepanel/flamepanel.db"}),
        json!({"binary_path": "/usr/bin/python3"}),
        json!({"engine": "iis"}),
    ] {
        let (status, _) = send(Method::PUT, format!("/api/web-servers/{id}"), body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let (status, updated) = send(
        Method::PUT,
        format!("/api/web-servers/{id}"),
        json!({"config_path": "/etc/nginx/conf.d/main.conf"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["config_path"], "/etc/nginx/conf.d/main.conf");

    // 裸写配置接口已移除（此前绕过助手与配置校验直接 fs::write）
    let (status, _) = send(
        Method::POST,
        format!("/api/web-servers/{id}/config"),
        json!({"content": "x"}),
    )
    .await;
    assert!(
        matches!(
            status,
            StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
        ),
        "{status}"
    );
}
