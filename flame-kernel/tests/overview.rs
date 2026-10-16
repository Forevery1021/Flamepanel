//! 顶栏概览聚合端点（S-06）：`GET /api/overview/badges`。
//!
//! 覆盖：200 + 字段形状 / 401 / 按角色逐分组裁剪（fail closed）/ 服务端缓存与失效。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use tower::ServiceExt;

mod common;
use common::{auth_header, bad_auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::domain::entity::ServerNode;
use flame_kernel::runtime::task_state::{TaskKind, TaskState};
use flame_kernel::utils::jwt::JwtUtils;

const PATH: &str = "/api/overview/badges";

fn node(name: &str, status: &str) -> ServerNode {
    ServerNode {
        id: 0,
        name: name.into(),
        hostname: format!("{name}.local"),
        ip_address: "10.0.0.1".into(),
        status: status.into(),
        created_at: Utc::now(),
        last_heartbeat_at: None,
        metrics_json: None,
        auth_token: None,
        agent_port: 9527,
    }
}

async fn get_json(app: &axum::Router, token: Option<&str>) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(Method::GET).uri(PATH);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, body)
}

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

#[tokio::test]
async fn badges_require_authentication() {
    let app = setup_full_router().await;
    let (status, _) = get_json(&app, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let bad = bad_auth_header().1;
    let (status, _) = get_json(&app, Some(bad.strip_prefix("Bearer ").unwrap())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_sees_all_groups_with_real_counts() {
    let (router, state) = setup_router().await;
    // 2 在线 + 1 离线节点；1 个运行中 + 1 个失败任务
    for (name, status) in [("a", "online"), ("b", "online"), ("c", "offline")] {
        state
            .node_service
            .register_node(&node(name, status))
            .await
            .unwrap();
    }
    let tracker = &state.task_service.tracker;
    let running = tracker.create(TaskKind::Generic, "deploy").await.unwrap();
    tracker
        .transition(running.id, TaskState::Running)
        .await
        .unwrap();
    let failed = tracker.create(TaskKind::Generic, "backup").await.unwrap();
    tracker
        .transition(failed.id, TaskState::Running)
        .await
        .unwrap();
    tracker
        .transition(failed.id, TaskState::Failed)
        .await
        .unwrap();

    let app = middleware::add_middleware_with_mock_peer(router, state);
    let (status, body) = get_json(&app, Some(&admin_token())).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(body["nodes"]["online"], 2);
    assert_eq!(body["nodes"]["total"], 3);
    // 内存 Docker 仓储：无容器但可用 → 对象存在且为 0
    assert_eq!(body["containers"]["running"], 0);
    assert_eq!(body["containers"]["total"], 0);
    assert_eq!(body["tasks"]["running"], 1);
    assert_eq!(body["tasks"]["pending"], 0);
    assert_eq!(body["tasks"]["failed"], 1);
    assert_eq!(body["alerts"]["nodes_offline"], 1);
    assert_eq!(body["alerts"]["certificates_expiring"], 0);
    assert_eq!(body["alerts"]["tasks_failed"], 1);
    assert_eq!(body["alerts"]["total"], 2);
    assert!(body["generated_at"].is_string());
    assert_eq!(body["cache_age_secs"], 0);
}

#[tokio::test]
async fn role_without_permissions_gets_all_groups_masked() {
    let (router, state) = setup_router().await;
    state
        .node_service
        .register_node(&node("x", "online"))
        .await
        .unwrap();
    // 角色 "guest" 未在种子角色中 → 权限集合为空 → 所有分组裁剪为 null（fail closed）
    state
        .user_service
        .create_user("guest", "hash", "guest")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, body) = get_json(&app, Some(&token)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "auth-only 端点对任何登录用户都是 200"
    );
    assert!(body["nodes"].is_null());
    assert!(body["containers"].is_null());
    assert!(body["tasks"].is_null());
    assert!(body["alerts"].is_null());
    assert!(body["generated_at"].is_string());
}

#[tokio::test]
async fn viewer_role_sees_read_only_groups() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, body) = get_json(&app, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    // viewer 拥有全部 *:read → 与管理员看到同样的分组
    assert!(body["nodes"].is_object());
    assert!(body["containers"].is_object());
    assert!(body["tasks"].is_object());
    assert!(body["alerts"].is_object());
}

#[tokio::test]
async fn snapshot_is_cached_until_invalidated() {
    let (router, state) = setup_router().await;
    let overview = state.overview_service.clone();
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    let token = admin_token();

    let (_, first) = get_json(&app, Some(&token)).await;
    assert_eq!(first["nodes"]["total"], 0);

    // 缓存窗口内新增节点：快照不变（generated_at 相同，total 仍为 0）
    state
        .node_service
        .register_node(&node("late", "online"))
        .await
        .unwrap();
    let (_, second) = get_json(&app, Some(&token)).await;
    assert_eq!(second["generated_at"], first["generated_at"]);
    assert_eq!(second["nodes"]["total"], 0);

    // 失效后重新聚合
    overview.invalidate().await;
    let (_, third) = get_json(&app, Some(&token)).await;
    assert_eq!(third["nodes"]["total"], 1);
    assert_eq!(third["cache_age_secs"], 0);
}

#[tokio::test]
async fn concurrent_requests_share_one_aggregation() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let app = app.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            let (status, body) = get_json(&app, Some(&token)).await;
            assert_eq!(status, StatusCode::OK);
            body["generated_at"].as_str().unwrap().to_string()
        }));
    }
    let mut stamps = Vec::new();
    for h in handles {
        stamps.push(h.await.unwrap());
    }
    stamps.sort();
    stamps.dedup();
    assert_eq!(stamps.len(), 1, "单飞：并发请求应共享同一份快照 {stamps:?}");
}
