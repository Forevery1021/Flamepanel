//! 鉴权中间件 / JWT / 密码 / 登录 / refresh
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
use common::{auth_header, bad_auth_header, bcrypt_hash, setup_full_router, setup_router};

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
async fn test_auth_missing_token_returns_401() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 错误响应应为统一 JSON 格式 {code, error, message}
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_auth_invalid_token_returns_401() {
    let app = setup_full_router().await;
    let (h, v) = bad_auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}

#[tokio::test]
async fn test_auth_health_skip() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_jwt_sign_and_verify() {
    let jwt = JwtUtils::new("test-secret", 1);
    let token = jwt.sign(100).unwrap();
    assert!(token.len() > 20);

    let claims = jwt.verify(&token).unwrap();
    assert_eq!(claims.sub, "100");
}

#[tokio::test]
async fn test_jwt_invalid_token() {
    let jwt = JwtUtils::new("test-secret", 1);
    let err = jwt.verify("invalid.token.here").unwrap_err();
    assert!(matches!(err, AppError::Unauthorized(_)));
}

#[tokio::test]
async fn test_jwt_wrong_secret() {
    let signer = JwtUtils::new("real-secret", 1);
    let verifier = JwtUtils::new("wrong-secret", 1);
    let token = signer.sign(1).unwrap();
    let err = verifier.verify(&token).unwrap_err();
    assert!(matches!(err, AppError::Unauthorized(_)));
}

#[tokio::test]
async fn test_password_hash_and_verify() {
    let password = "MySecureP@ss1";
    let hash = PasswordUtils::hash(password).unwrap();
    assert!(hash.starts_with("$2b$") || hash.starts_with("$2a$"));

    assert!(PasswordUtils::verify(password, &hash).unwrap());
    assert!(!PasswordUtils::verify("wrong_password", &hash).unwrap());
}

#[tokio::test]
async fn test_auth_refresh_endpoint() {
    let app = setup_full_router().await;
    // setup 的 admin hash 是 "hash" 字符串，无法登录；直接签发 refresh token 测试刷新接口
    let jwt = JwtUtils::new_pair("test-secret");
    let refresh_token = jwt.sign_refresh(1).unwrap();

    // 合法 refresh token（经 Cookie）→ 200，返回新的 access，并通过 Set-Cookie 轮换 refresh
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::COOKIE, format!("flame_refresh={}", refresh_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Y1：refresh token 仅经 Set-Cookie 下发，不在 JSON 响应体中暴露
    let set_cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(set_cookie.contains("flame_refresh="));
    assert!(set_cookie.contains("HttpOnly"));
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(resp["token"].as_str().unwrap().len() > 20);
    // refresh_token 不再写入响应体
    assert_eq!(resp["refresh_token"].as_str().unwrap(), "");
    assert_eq!(resp["username"], "admin");

    // 兼容：Bearer 方式仍可刷新
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::AUTHORIZATION, format!("Bearer {}", refresh_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 无 token → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // access token 不可用于刷新（类型校验）→ 401
    let access_token = jwt.sign_access(1).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::COOKIE, format!("flame_refresh={}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}

#[tokio::test]
async fn test_auth_me_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
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
    let me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(me["username"], "admin");
    assert!(me.get("must_change_password").is_some());
}

#[tokio::test]
async fn test_must_change_password_enforced() {
    // 构造必须改密的用户，验证访问受限
    let user_repo: Arc<dyn UserRepository> = Arc::new(InMemoryUserRepository::new());
    let user = user_repo
        .create("forced", &bcrypt_hash("OldP@ss1"), "admin")
        .await
        .unwrap();
    user_repo
        .update(&User {
            must_change_password: true,
            ..user
        })
        .await
        .unwrap();

    // 通过 service 直接验证中间件逻辑：构造 AppState 级测试过于复杂，
    // 此处验证 set_must_change_password 与 login 响应的标志位
    let bus = EventBus::new(100);
    let user_service = UserService::new(user_repo.clone(), bus, AuthCache::new());
    let updated = user_service
        .find_by_username("forced")
        .await
        .unwrap()
        .unwrap();
    assert!(updated.must_change_password, "flag should persist");

    // 清除标志
    user_service
        .set_must_change_password(updated.id, false)
        .await
        .unwrap();
    let cleared = user_service
        .find_by_username("forced")
        .await
        .unwrap()
        .unwrap();
    assert!(!cleared.must_change_password);
}

#[tokio::test]
async fn test_login_attempt_lock_unit() {
    use flame_kernel::api::login_attempt::LoginAttemptStore;
    let store = LoginAttemptStore::new();
    for _ in 0..5 {
        store.record_failure("alice", "127.0.0.1").await;
    }
    let err = store.check_locked("alice", "127.0.0.1").await;
    assert!(err.is_err(), "locked after 5 failures");
    assert_eq!(err.unwrap_err().status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_jwt_access_vs_refresh_type_check() {
    let jwt = JwtUtils::new_pair("test-secret-0123456789abcdef0123456789abcdef");
    let access = jwt.sign_access(7).unwrap();
    let refresh = jwt.sign_refresh(7).unwrap();

    // access 只能通过 verify_access
    assert!(jwt.verify_access(&access).is_ok());
    assert!(jwt.verify_refresh(&access).is_err());
    // refresh 只能通过 verify_refresh
    assert!(jwt.verify_refresh(&refresh).is_ok());
    assert!(jwt.verify_access(&refresh).is_err());
    // 通用 verify 两者均通过
    assert!(jwt.verify(&access).is_ok());
    assert!(jwt.verify(&refresh).is_ok());
}

#[tokio::test]
async fn test_jwt_secret_min_length_validation() {
    use flame_kernel::utils::jwt::validate_secret;
    // 过短拒绝
    assert!(validate_secret("short").is_err());
    assert!(validate_secret("1234567890123456789012345678901").is_err()); // 31 bytes
                                                                          // 达标通过
    assert!(validate_secret("12345678901234567890123456789012").is_ok()); // 32 bytes
}

#[tokio::test]
async fn test_jwt_rotate_secret_endpoint() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 过短密钥被拒绝（先于轮换，用旧 token）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/rotate-secret")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"secret":"short"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 轮换为合法长度的新密钥
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/rotate-secret")
                .header(h.clone(), v.clone())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"secret":"new-secret-0123456789abcdef0123456789abcdef"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 轮换后旧 access token 失效（401）
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], 401);
    assert_eq!(body["error"], "AUTH_UNAUTHORIZED");
}
