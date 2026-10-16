//! Docker 引擎端点与仓库
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

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
) -> StatusCode {
    let (h, v) = auth_header();
    let mut req = Request::builder().method(method).uri(uri).header(h, v);
    let body = match body {
        Some(b) => {
            req = req.header("Content-Type", "application/json");
            Body::from(serde_json::to_vec(&b).unwrap())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(req.body(body).unwrap())
        .await
        .unwrap()
        .status()
}

/// Doc/33 P-12：axum 解码 `%2F` 后 ID 会带 `/`，bollard 经 `Url::join` 折叠 `..`——
/// 「停止容器」可被导向 `POST /images/prune`。所有引用在服务层白名单校验 → 400。
#[tokio::test]
async fn test_docker_rejects_path_injection_in_refs() {
    let app = setup_full_router().await;
    for (method, uri) in [
        (
            Method::POST,
            "/api/docker/containers/x%2F..%2F..%2Fimages%2Fprune%3F/stop",
        ),
        (Method::POST, "/api/docker/containers/..%2Fbuild/start"),
        (Method::POST, "/api/docker/containers/a%3Fb/kill"),
        (Method::GET, "/api/docker/containers/x%2F..%2Finfo/logs"),
        (Method::POST, "/api/docker/images/..%2F..%2Fbuild/remove"),
        (Method::DELETE, "/api/docker/networks/..%2Fplugins"),
        (Method::DELETE, "/api/docker/volumes/x%2F..%2F..%2Fswarm"),
        (Method::POST, "/api/docker/compose/-rf/up"),
        (Method::POST, "/api/docker/compose/a..b/down"),
    ] {
        assert_eq!(
            send(&app, method.clone(), uri, None).await,
            StatusCode::BAD_REQUEST,
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn test_docker_rejects_invalid_bodies() {
    let app = setup_full_router().await;
    let cases = [
        (
            "/api/docker/networks",
            json!({ "name": "n1", "subnet": "10.0.0.0/33" }),
        ),
        ("/api/docker/networks", json!({ "name": "n 1" })),
        (
            "/api/docker/networks",
            json!({ "name": "n1", "driver": "Bridge?" }),
        ),
        ("/api/docker/volumes", json!({ "name": "../v" })),
        ("/api/docker/images/pull", json!({ "image": "nginx?x=1" })),
        (
            "/api/docker/images/sha256:abc/tag",
            json!({ "repo": "app:1", "tag": "2" }),
        ),
        (
            "/api/docker/images/sha256:abc/tag",
            json!({ "repo": "app", "tag": "a/b" }),
        ),
        (
            "/api/docker/containers/c1/rename",
            json!({ "new_name": "x/../y" }),
        ),
        (
            "/api/docker/networks/n1/connect",
            json!({ "container_id": "../c" }),
        ),
        (
            "/api/docker/compose/deploy",
            json!({ "project_name": "p1", "compose_yaml": " " }),
        ),
        (
            "/api/docker/compose/deploy",
            json!({ "project_name": "p1", "compose_yaml": "x".repeat(256 * 1024 + 1) }),
        ),
        (
            "/api/docker/compose/deploy",
            json!({ "project_name": "../p", "compose_yaml": "services: {}" }),
        ),
    ];
    for (uri, body) in cases {
        assert_eq!(
            send(&app, Method::POST, uri, Some(body.clone())).await,
            StatusCode::BAD_REQUEST,
            "{uri} {body}"
        );
    }
    // 合法值仍可用
    assert_eq!(
        send(
            &app,
            Method::POST,
            "/api/docker/networks",
            Some(json!({ "name": "n1", "subnet": "172.30.0.0/16" }))
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            Method::POST,
            "/api/docker/images/sha256:abc/tag",
            Some(json!({ "repo": "registry.local:5000/team/app", "tag": "v1.2" }))
        )
        .await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_docker_endpoints_forbidden_without_permissions() {
    let (_r, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(
        flame_kernel::api::routes::create_router(state.clone()),
        state.clone(),
    );
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    let token = flame_kernel::utils::jwt::JwtUtils::new("test-secret", 24)
        .sign(2)
        .unwrap();
    for (method, uri) in [
        (Method::GET, "/api/docker/containers"),
        (Method::POST, "/api/docker/containers/c1/stop"),
        (Method::POST, "/api/docker/compose/deploy"),
        (Method::POST, "/api/docker/images/prune"),
        (Method::POST, "/api/docker/compose/convert-run"),
        (Method::GET, "/api/docker/containers/c1/compose"),
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri(uri)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "{method} {uri}");
    }
}

async fn send_json(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let (h, v) = auth_header();
    let mut req = Request::builder().method(method).uri(uri).header(h, v);
    let body = match body {
        Some(b) => {
            req = req.header("Content-Type", "application/json");
            Body::from(serde_json::to_vec(&b).unwrap())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

/// M-D：`docker run` 粘贴导入——纯转换，结果交给用户审阅后再部署。
#[tokio::test]
async fn test_docker_convert_run_command() {
    let app = setup_full_router().await;
    let (status, body) = send_json(
        &app,
        Method::POST,
        "/api/docker/compose/convert-run",
        Some(json!({
            "command": "docker run -d --name web -p 8080:80 -v webdata:/data --rm nginx:alpine"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["project_name"], "web");
    assert_eq!(body["service_name"], "web");
    let yaml = body["compose_yaml"].as_str().unwrap();
    let doc: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(doc["services"]["web"]["image"], "nginx:alpine");
    assert_eq!(doc["services"]["web"]["ports"][0], "8080:80");
    assert_eq!(doc["volumes"]["webdata"]["name"], "webdata");
    assert!(body["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w.as_str().unwrap().contains("--rm")));

    // 生成的 YAML 可直接走部署
    let (status, _) = send_json(
        &app,
        Method::POST,
        "/api/docker/compose/deploy",
        Some(json!({ "project_name": body["project_name"], "compose_yaml": yaml })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for bad in [
        "",
        "ls -la",
        "docker run nginx | sh",
        "docker run -e X=$(id) nginx",
        "docker run --no-such-flag nginx",
    ] {
        let (status, _) = send_json(
            &app,
            Method::POST,
            "/api/docker/compose/convert-run",
            Some(json!({ "command": bad })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad:?}");
    }
    let (status, _) = send_json(
        &app,
        Method::POST,
        "/api/docker/compose/convert-run",
        Some(json!({ "command": format!("docker run -e X={} nginx", "a".repeat(17 * 1024)) })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// M-D：存量容器收编——导出为 Compose（不改动原容器）。
#[tokio::test]
async fn test_docker_container_to_compose() {
    let app = setup_full_router().await;
    let (status, body) = send_json(
        &app,
        Method::GET,
        "/api/docker/containers/legacy-app/compose",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["project_name"], "legacy-app");
    let doc: serde_yaml::Value =
        serde_yaml::from_str(body["compose_yaml"].as_str().unwrap()).unwrap();
    assert_eq!(
        doc["services"]["legacy-app"]["image"],
        "memory/placeholder:latest"
    );
    assert_eq!(doc["services"]["legacy-app"]["restart"], "unless-stopped");

    let (status, _) = send_json(
        &app,
        Method::GET,
        "/api/docker/containers/x%2F..%2F..%2Finfo/compose",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
