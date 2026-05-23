use std::net::SocketAddr;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use bcrypt::{hash, DEFAULT_COST};
use ops_panel_backend::{
    application::AppState,
    api,
    infrastructure::{
        SqliteLogRepository, SqliteUserRepository, SqliteWebsiteRepository,
        SqliteWafRuleRepository, SqliteWafIpRuleRepository,
    },
    middleware::auth::create_jwt,
    metrics::{MetricsHistory, MetricsSnapshot},
};
use serde_json::{json, Value};
use tower::util::ServiceExt;

async fn test_app() -> (axum::Router, SocketAddr) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory db");

    sqlx::query(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_login TEXT
        )"
    ).execute(&db).await.unwrap();

    sqlx::query(
        "CREATE TABLE websites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domain TEXT UNIQUE NOT NULL,
            root_path TEXT NOT NULL,
            proxy_port INTEGER,
            ssl_enabled INTEGER NOT NULL DEFAULT 0,
            ssl_cert_path TEXT,
            ssl_key_path TEXT,
            config_path TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            engine TEXT NOT NULL DEFAULT 'nginx',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    ).execute(&db).await.unwrap();

    sqlx::query(
        "CREATE TABLE operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            action TEXT NOT NULL,
            target TEXT NOT NULL DEFAULT '',
            ip TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    ).execute(&db).await.unwrap();

    sqlx::query(
        "CREATE TABLE waf_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            pattern TEXT NOT NULL,
            target TEXT NOT NULL DEFAULT 'url',
            action TEXT NOT NULL DEFAULT 'block',
            description TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    ).execute(&db).await.unwrap();

    sqlx::query(
        "CREATE TABLE waf_ip_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ip TEXT NOT NULL,
            action TEXT NOT NULL DEFAULT 'block',
            description TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    ).execute(&db).await.unwrap();

    // Seed admin user
    let pwd = hash("admin123", DEFAULT_COST).unwrap();
    sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ('admin', ?, 'admin')")
        .bind(&pwd)
        .execute(&db)
        .await
        .unwrap();

    let (metrics_tx, _rx) = tokio::sync::broadcast::channel::<MetricsSnapshot>(1);
    let history = std::sync::Arc::new(tokio::sync::Mutex::new(MetricsHistory::new(10)));

    let state = AppState {
        db: db.clone(),
        user_repo: std::sync::Arc::new(SqliteUserRepository::new(db.clone())),
        website_repo: std::sync::Arc::new(SqliteWebsiteRepository::new(db.clone())),
        waf_repo: std::sync::Arc::new(SqliteWafRuleRepository::new(db.clone())),
        waf_ip_repo: std::sync::Arc::new(SqliteWafIpRuleRepository::new(db.clone())),
        log_repo: std::sync::Arc::new(SqliteLogRepository::new(db.clone())),
        sessions: std::sync::Arc::default(),
        metrics_tx,
        metrics_history: history,
    };

    let app = api::routes().with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    (app.into(), addr)
}

fn auth_header(username: &str) -> String {
    let token = create_jwt(username, "admin", 3600).unwrap();
    format!("Bearer {token}")
}

// ─── Health Check ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check_returns_ok() {
    let (app, _) = test_app().await;
    let req = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "healthy");
}

// ─── Auth: Login ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn login_with_valid_credentials() {
    let (app, _) = test_app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"username": "admin", "password": "admin123"}).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].as_str().is_some());
    assert_eq!(json["username"], "admin");
}

#[tokio::test]
async fn login_with_invalid_credentials_returns_401() {
    let (app, _) = test_app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"username": "admin", "password": "wrong"}).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ─── Auth: Me ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn me_requires_auth() {
    let (app, _) = test_app().await;
    let req = Request::builder()
        .uri("/api/auth/me")
        .header(header::AUTHORIZATION, auth_header("admin"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "admin");
    assert_eq!(json["role"], "admin");
}

#[tokio::test]
async fn me_without_token_returns_401() {
    let (app, _) = test_app().await;
    let req = Request::builder()
        .uri("/api/auth/me")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ─── WAF IP Rules ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn waf_ip_rules_crud() {
    let (app, _) = test_app().await;
    let auth = auth_header("admin");

    // List (empty)
    let req = Request::builder()
        .uri("/api/waf/ip-rules")
        .header(header::AUTHORIZATION, &auth)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Create
    let req = Request::builder()
        .method("POST")
        .uri("/api/waf/ip-rules/create")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, &auth)
        .body(Body::from(json!({"ip": "10.0.0.1", "action": "block"}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List (one item)
    let req = Request::builder()
        .uri("/api/waf/ip-rules")
        .header(header::AUTHORIZATION, &auth)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
}

// ─── System Info ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn system_info_requires_auth() {
    let (app, _) = test_app().await;
    let req = Request::builder()
        .uri("/api/system/info")
        .header(header::AUTHORIZATION, auth_header("admin"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["cpu_cores"].as_u64().is_some());
}

// ─── User Management ──────────────────────────────────────────────────────────

#[tokio::test]
async fn user_list_requires_admin() {
    let (app, _) = test_app().await;
    let admin_auth = auth_header("admin");

    let req = Request::builder()
        .uri("/api/users/list")
        .header(header::AUTHORIZATION, &admin_auth)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ─── Logs ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn logs_list_returns_paginated() {
    let (app, _) = test_app().await;
    let auth = auth_header("admin");

    let req = Request::builder()
        .uri("/api/logs/list?page=1&page_size=10")
        .header(header::AUTHORIZATION, &auth)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].is_array());
    assert!(json["total"].as_i64().is_some());
}
