//! WebSocket 鉴权（R2：首帧鉴权）
//!
//! 从巨型 `integration_test.rs` 按领域拆分而来（S12），复用 `common` 共享工具。
//!
//! R2 起 WS 鉴权由握手期（URL query `?token=`）改为「首帧鉴权」：握手阶段中间件放行，
//! 连接建立后首帧 `{"type":"auth","token":...}` 完成 JWT+RBAC 校验，失败即关闭。
//! 因此 `oneshot` 只能验证握手不再被中间件拦截（返回 426 而非 401/403）；
//! 真正的首帧鉴权逻辑依赖 WS 连接内的消息交互，由前端 `utils/ws.ts` 与后端 handler 协同。
//!
//! 另：握手通过不代表鉴权通过——未认证连接会在首帧校验时被关闭。

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

/// 发起一次 WS 握手请求（不带 token query），断言握手被中间件放行（不再在握手期拒绝）。
/// oneshot 下 WebSocketUpgrade 无法完成实际升级，返回 426（Upgrade Required）属 axum
/// 测试环境限制，而非鉴权失败；断言不等于 401/403 即可证明握手未被中间件拦截。
fn ws_handshake_request(path: &str) -> axum::http::Request<Body> {
    Request::builder()
        .uri(path)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("Sec-WebSocket-Version", "13")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn test_ws_handshake_not_blocked_by_middleware() {
    // R2：握手阶段中间件不再从 query 校验 token（token 移至首帧），因此无 token 握手
    // 不再返回 401，而是进入 upgrade（oneshot 下返回 426）。真正的鉴权在首帧。
    let app = setup_full_router().await;
    let res = app
        .oneshot(ws_handshake_request("/ws/metrics"))
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_ws_handshake_all_paths_forwarded() {
    // 所有 WS 路径握手均不应被中间件以 401/403 拦截（鉴权移交首帧）。
    let app = setup_full_router().await;
    for path in [
        "/ws/metrics",
        "/ws/metrics?node_id=1",
        "/ws/logs",
        "/ws/terminal",
    ] {
        let res = app
            .clone()
            .oneshot(ws_handshake_request(path))
            .await
            .unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
        assert_ne!(res.status(), StatusCode::FORBIDDEN);
    }
}

/// Doc/30 M-P：`/ws/metrics?node_id=` 非法参数在升级前以 400 拒绝。
#[tokio::test]
async fn test_ws_metrics_rejects_malformed_node_id() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(ws_handshake_request("/ws/metrics?node_id=abc"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

/// Doc/33「WS 4001」：会话复核与带内续期的判定（关闭码映射见 `ws_auth`）。
#[tokio::test]
async fn ws_session_recheck_and_reauth_verdicts() {
    use flame_kernel::api::ws_auth::{
        WsSession, WsVerdict, CLOSE_FORBIDDEN, CLOSE_UNAUTHENTICATED,
    };
    let (_router, state) = setup_router().await;
    let jwt = JwtUtils::new("test-secret", 24);
    let op = state
        .user_service
        .create_user("wsop", "hash", "operator")
        .await
        .unwrap();
    let other = state
        .user_service
        .create_user("wsother", "hash", "operator")
        .await
        .unwrap();
    let token = jwt.sign(op.id).unwrap();
    let session = WsSession::new("/ws/logs", op.id, token.clone());
    assert_eq!(session.recheck(&state).await, WsVerdict::Valid);

    // 换成其他用户的令牌 → 4001，且不替换当前令牌
    let v = session.reauth(&state, &jwt.sign(other.id).unwrap()).await;
    assert_eq!(v, WsVerdict::Unauthenticated);
    assert_eq!(v.close_code(), Some(CLOSE_UNAUTHENTICATED));
    assert_eq!(session.recheck(&state).await, WsVerdict::Valid);
    // 垃圾令牌 → 4001
    assert_eq!(
        session.reauth(&state, "garbage").await,
        WsVerdict::Unauthenticated
    );

    // 权限被收回（降为 viewer，/ws/logs 需要 log:read——viewer 有读权限，改测终端）
    let term = WsSession::new("/ws/terminal", op.id, token.clone());
    let mut u = state.user_service.find_by_id(op.id).await.unwrap().unwrap();
    u.role = "viewer".into();
    state.user_service.update_user(&u).await.unwrap();
    let v = term.recheck(&state).await;
    assert_eq!(v, WsVerdict::Forbidden);
    assert_eq!(v.close_code(), Some(CLOSE_FORBIDDEN));

    // 带内续期换新令牌仍有效（改密吊销依赖 SQLite 的 password_changed_at，InMemory 不持久化，
    // 该分支与 REST 共用 authenticate 逻辑，由 auth 集成测试覆盖）
    let fresh = jwt.sign(op.id).unwrap();
    assert_eq!(session.reauth(&state, &fresh).await, WsVerdict::Valid);
    assert_eq!(session.recheck(&state).await, WsVerdict::Valid);

    // 用户被删除 → 4001；kill 把关闭码送到发送任务
    let mut rx = session.kill_rx();
    state.user_service.delete_user(op.id).await.unwrap();
    let v = session.recheck(&state).await;
    assert_eq!(v, WsVerdict::Unauthenticated);
    session.kill(v);
    rx.changed().await.unwrap();
    assert_eq!(*rx.borrow(), CLOSE_UNAUTHENTICATED);
}
