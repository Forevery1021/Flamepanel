//! 站点引擎切换与乐观并发
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
        site_type: "static".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
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

// ── 站点绑定闭环（M-B 主线二 Stage A-6 · Doc/20 §5.5）────────────────────

#[tokio::test]
async fn test_website_bind_runtime_validates_php_installed() {
    let runtime_repo = Arc::new(InMemoryRuntimeRepository::new());
    let project_repo = Arc::new(InMemoryProjectRepository::new());
    let website_repo = Arc::new(InMemoryWebsiteRepository::new());
    let svc = WebsiteService::new(website_repo, EventBus::new(10))
        .with_binding_repos(runtime_repo.clone(), project_repo);

    let ws = Website {
        id: 0,
        name: "blog".into(),
        domain: "blog.bind.test.com".into(),
        root_path: "/var/www/blog".into(),
        status: "active".into(),
        node_id: 1,
        engine: "nginx".into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "static".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = svc.create_website(&ws).await.unwrap();

    // 不存在的运行时 → NotFound
    let err = svc
        .bind_runtime(id, "runtime_php", Some(99), None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)), "got {:?}", err);

    // 未安装 php → BadRequest
    let rid = runtime_repo
        .create(&Runtime {
            id: 0,
            kind: "php".into(),
            version: "8.2".into(),
            install_path: "/opt/flamepanel/runtimes/php/8.2".into(),
            status: "not_installed".into(),
            is_default: false,
            metadata_json: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();
    let err = svc
        .bind_runtime(id, "runtime_php", Some(rid), None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)), "got {:?}", err);

    // 非法 site_type → BadRequest
    let err = svc
        .bind_runtime(id, "hacker", Some(rid), None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));

    // 标记已安装 → 绑定成功
    let mut rt = runtime_repo.find_by_id(rid).await.unwrap().unwrap();
    rt.status = "installed".into();
    runtime_repo.update(&rt).await.unwrap();
    let bound = svc
        .bind_runtime(id, "runtime_php", Some(rid), None)
        .await
        .unwrap();
    assert_eq!(bound.site_type, "runtime_php");
    assert_eq!(bound.runtime_id, Some(rid));

    // runtime/project 互斥 → BadRequest
    let err = svc
        .bind_runtime(id, "reverse_proxy", Some(rid), Some(1))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));

    // 按运行时列站点（绑定中 → 1 条）
    assert_eq!(svc.list_sites_by_runtime(rid).await.unwrap().len(), 1);

    // 解绑回落 static
    let unbound = svc.unbind_runtime(id).await.unwrap();
    assert_eq!(unbound.site_type, "static");
    assert_eq!(unbound.runtime_id, None);
    assert!(svc.list_sites_by_runtime(rid).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_website_rewrite_preset() {
    let svc = WebsiteService::new(
        Arc::new(InMemoryWebsiteRepository::new()),
        EventBus::new(10),
    );
    let ws = Website {
        id: 0,
        name: "wp".into(),
        domain: "wp.rewrite.test.com".into(),
        root_path: "/var/www/wp".into(),
        status: "active".into(),
        node_id: 1,
        engine: "nginx".into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "runtime_php".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = svc.create_website(&ws).await.unwrap();

    let applied = svc.apply_rewrite_preset(id, "wordpress").await.unwrap();
    assert_eq!(applied.preset.as_deref(), Some("wordpress"));
    assert!(applied
        .rules
        .contains("try_files $uri $uri/ /index.php?$args;"));
    // 未注入写盘端口（裁剪装配）→ applied=false，仅落库
    assert!(!applied.applied);

    let err = svc.apply_rewrite_preset(id, "evil").await.unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));
}

/// 伪静态「套用即生效」写穿链路：落库 + 写盘 + 按引擎语法出参 + 清除 + 失败回滚。
#[tokio::test]
async fn test_rewrite_preset_persists_and_rolls_back() {
    use flame_kernel::application::site_config_ports::SiteConfigPort;
    use std::sync::Mutex as StdMutex;

    /// 记录型写盘端口：可切换为失败态，验证回滚语义。
    struct RecordingWriter {
        calls: StdMutex<Vec<String>>,
        fail: StdMutex<bool>,
    }
    impl RecordingWriter {
        fn new() -> Self {
            Self {
                calls: StdMutex::new(Vec::new()),
                fail: StdMutex::new(false),
            }
        }
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
        fn set_fail(&self, v: bool) {
            *self.fail.lock().unwrap() = v;
        }
    }
    #[async_trait::async_trait]
    impl SiteConfigPort for RecordingWriter {
        async fn write_site_config(&self, site: &Website) -> Result<(), AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{}:{}", site.engine, site.domain));
            if *self.fail.lock().unwrap() {
                return Err(AppError::Internal {
                    message: "nginx -t failed".into(),
                    source: None,
                });
            }
            Ok(())
        }
    }

    let repo = Arc::new(InMemoryWebsiteRepository::new());
    let writer = Arc::new(RecordingWriter::new());
    let svc = WebsiteService::new(repo.clone(), EventBus::new(10))
        .with_site_config_writer(writer.clone());

    let ws = Website {
        id: 0,
        name: "wp".into(),
        domain: "wp.persist.test.com".into(),
        root_path: "/var/www/wp".into(),
        status: "active".into(),
        node_id: 1,
        engine: "nginx".into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "runtime_php".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&ws).await.unwrap();

    // 套用：落库 + 写盘（applied=true）
    let applied = svc.apply_rewrite_preset(id, "wordpress").await.unwrap();
    assert_eq!(applied.preset.as_deref(), Some("wordpress"));
    assert!(applied.applied);
    assert_eq!(
        writer.calls(),
        vec!["nginx:wp.persist.test.com".to_string()]
    );
    let stored = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(stored.rewrite_preset.as_deref(), Some("wordpress"));

    // 查询状态：模板名 + 规则文本 + 可选清单
    let status = svc.get_rewrite_preset(id).await.unwrap();
    assert_eq!(status.preset.as_deref(), Some("wordpress"));
    assert!(status.rules.contains("try_files"));
    assert_eq!(status.supported.len(), 6);
    assert!(status.supported.contains(&"thinkphp".to_string()));

    // 写盘失败：错误上抛 + 库中模板名回滚（不出现「库说有、宿主没有」）
    writer.set_fail(true);
    let err = svc.apply_rewrite_preset(id, "laravel").await.unwrap_err();
    assert!(matches!(err, AppError::Internal { .. }));
    let rolled_back = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(
        rolled_back.rewrite_preset.as_deref(),
        Some("wordpress"),
        "写盘失败必须回滚为上次成功套用值"
    );

    // 清除套用：回落默认路由并再次写盘
    writer.set_fail(false);
    let cleared = svc.clear_rewrite_preset(id).await.unwrap();
    assert_eq!(cleared.preset, None);
    assert!(cleared.rules.is_empty());
    let after = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(after.rewrite_preset, None);
    assert_eq!(writer.calls().len(), 3);

    // 未知站点 → 404
    let err = svc.get_rewrite_preset(99999).await.unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
}

/// 规则文本按站点引擎适配（同一模板在不同引擎下语法不同）。
#[tokio::test]
async fn test_rewrite_rules_follow_engine_syntax() {
    let repo = Arc::new(InMemoryWebsiteRepository::new());
    let svc = WebsiteService::new(repo.clone(), EventBus::new(10));
    let base = Website {
        id: 0,
        name: "s".into(),
        domain: "engine.rewrite.test.com".into(),
        root_path: "/var/www/s".into(),
        status: "active".into(),
        node_id: 1,
        engine: "nginx".into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "runtime_php".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = repo.create(&base).await.unwrap();

    let nginx = svc.apply_rewrite_preset(id, "wordpress").await.unwrap();
    assert!(nginx.rules.contains("location / {"));

    // 切引擎（保留套用状态）→ 规则文本随引擎语法变化
    let mut apache = repo.find_by_id(id).await.unwrap().unwrap();
    apache.engine = "apache".into();
    repo.update(&apache).await.unwrap();
    let apache_rules = svc.get_rewrite_preset(id).await.unwrap();
    assert_eq!(apache_rules.preset.as_deref(), Some("wordpress"));
    assert!(apache_rules.rules.contains("RewriteEngine On"));
    assert!(!apache_rules.rules.contains("location / {"));

    // 清除套用后：preset=None、rules 空串
    svc.clear_rewrite_preset(id).await.unwrap();
    let st = svc.get_rewrite_preset(id).await.unwrap();
    assert_eq!(st.preset, None);
    assert!(st.rules.is_empty());
}

/// 伪静态套用端到端（Doc/20 §5.5 收口）：POST 套用 → GET 状态 → DELETE 清除 + 未授权拒绝。
#[tokio::test]
async fn test_website_rewrite_endpoints_end_to_end() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 建站
    let create_body = serde_json::to_vec(&json!({
        "website": {
            "id": 0, "name": "rewrite-e2e", "domain": "rewrite-e2e.example.com",
            "root_path": "/var/www/rewrite-e2e", "engine": "nginx", "node_id": 1,
            "status": "running", "ssl_enabled": false, "proxy_enabled": false,
            "created_at": "2026-01-01T00:00:00Z"
        }
    }))
    .unwrap();
    let res = app
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
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let id: i64 = serde_json::from_slice(&bytes).unwrap();

    // 初始：未套用
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/websites/{}/rewrite", id))
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
    let st: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(st["preset"].is_null());
    assert_eq!(st["rules"].as_str().unwrap(), "");
    let supported = st["supported"].as_array().unwrap();
    assert_eq!(supported.len(), 6);
    assert!(supported.iter().any(|s| s == "wordpress"));

    // 套用（响应保持既有契约：纯规则文本）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/websites/{}/rewrite", id))
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "preset": "wordpress" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let rules: String = serde_json::from_slice(&bytes).unwrap();
    assert!(rules.contains("try_files $uri $uri/ /index.php?$args;"));

    // 状态可查（落库生效）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/websites/{}/rewrite", id))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let st: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(st["preset"].as_str().unwrap(), "wordpress");
    assert!(st["rules"].as_str().unwrap().contains("try_files"));

    // 未知模板 → 400
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/websites/{}/rewrite", id))
                .header(h.clone(), v.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "preset": "evil" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 清除套用
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/websites/{}/rewrite", id))
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
    let st: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(st["preset"].is_null());

    // 不存在的站点 → 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/websites/99999/rewrite")
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 匿名访问 → 401（三个方法全覆盖）
    for method in [Method::GET, Method::POST, Method::DELETE] {
        let mut builder = Request::builder()
            .method(method.clone())
            .uri(format!("/api/websites/{}/rewrite", id));
        if method == Method::POST {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        let body = if method == Method::POST {
            Body::from(serde_json::to_vec(&json!({ "preset": "wordpress" })).unwrap())
        } else {
            Body::empty()
        };
        let res = app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{method:?} 应 401");
    }
}

// ── 站点配置流水线（Doc/20 §5.5）：部署端点三件套 + 生命周期联动 ──────────

#[tokio::test]
async fn deploy_endpoint_requires_auth_permission_and_writer() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let id = state
        .website_service
        .create_website(&lifecycle_site("deploy.example.invalid", "nginx"))
        .await
        .unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let post = |uri: String, auth: Option<(axum::http::HeaderName, String)>| {
        let mut b = Request::builder().method(Method::POST).uri(uri);
        if let Some((h, v)) = auth {
            b = b.header(h, v);
        }
        b.body(Body::empty()).unwrap()
    };

    // 401：未认证 / 坏 token
    let res = app
        .clone()
        .oneshot(post(format!("/api/websites/{id}/deploy"), None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = app
        .clone()
        .oneshot(post(
            format!("/api/websites/{id}/deploy"),
            Some(bad_auth_header()),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // 403：viewer 无 website:update
    let res = app
        .clone()
        .oneshot(post(
            format!("/api/websites/{id}/deploy"),
            Some((header::AUTHORIZATION, format!("Bearer {viewer}"))),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // 404：站点不存在
    let res = app
        .clone()
        .oneshot(post(
            "/api/websites/999999/deploy".into(),
            Some(auth_header()),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // 503：测试装配未注入写盘端口（严格模式不假装成功）
    let res = app
        .oneshot(post(
            format!("/api/websites/{id}/deploy"),
            Some(auth_header()),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

fn lifecycle_site(domain: &str, engine: &str) -> Website {
    Website {
        id: 0,
        name: domain.into(),
        domain: domain.into(),
        root_path: "/var/www/lifecycle".into(),
        status: "running".into(),
        node_id: 1,
        engine: engine.into(),
        ssl_enabled: false,
        proxy_enabled: false,
        proxy_pass: None,
        site_type: "static".into(),
        runtime_id: None,
        project_id: None,
        rewrite_preset: None,
        created_at: Utc::now(),
        resource_version: 0,
    }
}

/// 记录型端口：记录 deploy / remove 调用，可按引擎声明是否已安装、可置失败。
#[derive(Default)]
struct LifecyclePort {
    calls: std::sync::Mutex<Vec<String>>,
    fail_deploy: std::sync::atomic::AtomicBool,
    installed: std::sync::Mutex<Vec<String>>,
}

impl LifecyclePort {
    fn with_installed(engines: &[&'static str]) -> Self {
        let p = Self::default();
        *p.installed.lock().unwrap() = engines.iter().map(|e| e.to_string()).collect();
        p
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.calls.lock().unwrap())
    }
}

#[async_trait::async_trait]
impl flame_kernel::application::site_config_ports::SiteConfigPort for LifecyclePort {
    async fn write_site_config(&self, site: &Website) -> Result<(), AppError> {
        self.calls.lock().unwrap().push(format!(
            "deploy {}@{} {}",
            site.domain, site.engine, site.status
        ));
        if self.fail_deploy.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AppError::BadRequest("nginx config test failed".into()));
        }
        Ok(())
    }
    async fn remove_site_config(&self, site: &Website) -> Result<(), AppError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("remove {}@{}", site.domain, site.engine));
        Ok(())
    }
    async fn can_deploy(&self, engine: &str) -> bool {
        self.installed.lock().unwrap().contains(&engine.to_string())
    }
}

#[tokio::test]
async fn website_lifecycle_drives_site_config_pipeline() {
    use std::sync::atomic::Ordering::SeqCst;
    let repo = Arc::new(InMemoryWebsiteRepository::new());
    let port = Arc::new(LifecyclePort::with_installed(&["nginx", "caddy"]));
    let svc =
        WebsiteService::new(repo.clone(), EventBus::new(10)).with_site_config_writer(port.clone());

    // 创建 → 部署
    let id = svc
        .create_website(&lifecycle_site("life.example.invalid", "nginx"))
        .await
        .unwrap();
    let calls = port.take();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].starts_with("deploy life.example.invalid@nginx"),
        "{calls:?}"
    );

    // 引擎未安装 → 创建成功但跳过部署
    svc.create_website(&lifecycle_site("skip.example.invalid", "apache"))
        .await
        .unwrap();
    assert!(port.take().is_empty());

    // 部署失败 → 撤销创建
    port.fail_deploy.store(true, SeqCst);
    let before = repo.list_all().await.unwrap().len();
    assert!(svc
        .create_website(&lifecycle_site("fail.example.invalid", "nginx"))
        .await
        .is_err());
    assert_eq!(repo.list_all().await.unwrap().len(), before);
    port.take();

    // 更新失败 → 原行恢复
    let mut site = svc.get_website(id).await.unwrap();
    site.root_path = "/var/www/changed".into();
    assert!(svc.update_website(&site).await.is_err());
    assert_eq!(
        svc.get_website(id).await.unwrap().root_path,
        "/var/www/lifecycle"
    );
    port.fail_deploy.store(false, SeqCst);
    port.take();

    // 改域名 + 停用 → 部署新域名（停用态），清理旧域名
    let mut site = svc.get_website(id).await.unwrap();
    site.domain = "life2.example.invalid".into();
    site.status = "stopped".into();
    svc.update_website(&site).await.unwrap();
    assert_eq!(
        port.take(),
        vec![
            "deploy life2.example.invalid@nginx stopped",
            "remove life.example.invalid@nginx"
        ]
    );

    // 切到未安装引擎 → 拒绝，库不变
    let err = svc
        .switch_engine(id, &flame_kernel::webserver::WebServerEngine::Apache)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));
    assert_eq!(svc.get_website(id).await.unwrap().engine, "nginx");
    assert!(port.take().is_empty());

    // 切到已安装引擎 → 新引擎部署 + 旧引擎清理
    let switched = svc
        .switch_engine(id, &flame_kernel::webserver::WebServerEngine::Caddy)
        .await
        .unwrap();
    assert_eq!(switched.engine, "caddy");
    assert_eq!(
        port.take(),
        vec![
            "deploy life2.example.invalid@caddy stopped",
            "remove life2.example.invalid@nginx"
        ]
    );

    // 手动部署（严格）
    let d = svc.deploy_website(id).await.unwrap();
    assert_eq!(d.engine, "caddy");
    port.take();

    // 删除 → 先移除 vhost
    svc.delete_website(id).await.unwrap();
    assert_eq!(port.take(), vec!["remove life2.example.invalid@caddy"]);
    assert!(svc.get_website(id).await.is_err());
}
