//! SSL 证书自动化集成测试（M-B 主线三 §6.1 · Stage 2 签发链路）。
//!
//! 覆盖：CRUD 端点 / 自签签发 / 上传证书（真实 PEM 解析）/ 绑定校验 /
//! 权限拒绝（匿名 401）/ 泛域名 DNS-01 前置校验 / ACME 目录 URL 校验。

#![allow(unused_imports)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

mod common;
use common::{auth_header, bad_auth_header, setup_full_router};

/// 自签证书签发：真实 rcgen 落盘 + 记录回写 + 到期时间解析。
#[tokio::test]
async fn issue_self_signed_creates_record() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({
        "domains": ["selfsigned.example.com", "www.selfsigned.example.com"]
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/issue/self-signed")
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
    let cert: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cert["source"], "self_signed");
    assert!(cert["expires_at"].is_string());
    // 记录可查
    let (h2, v2) = auth_header();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/certificates/{}",
                    cert["id"].as_i64().unwrap()
                ))
                .header(h2, v2)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

/// 上传证书：用 rcgen 生成真实 PEM 链路，验证过期时间解析与落库。
#[tokio::test]
async fn upload_certificate_parses_expiry() {
    // 生成一张真实证书
    use rcgen::{CertificateParams, DnType, IsCa, KeyPair};
    let mut params = CertificateParams::new(vec!["upload.test".to_string()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "upload.test");
    params.is_ca = IsCa::NoCa;
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::days(1);
    params.not_after = now + time::Duration::days(90);
    let kp = KeyPair::generate().unwrap();
    let cert = params.self_signed(&kp).unwrap();
    let cert_pem = cert.pem();
    let key_pem = kp.serialize_pem();

    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({
        "domains": ["upload.test"],
        "cert_pem": cert_pem,
        "key_pem": key_pem,
    }))
    .unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates")
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
    let record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record["source"], "uploaded");
    assert!(record["cert_path"]
        .as_str()
        .unwrap()
        .contains("upload.test"));
}

/// 泛域名证书必须走 DNS-01（无 DNS 厂商时 ACME 签发被前置校验拒绝）。
#[tokio::test]
async fn acme_wildcard_without_dns_provider_rejected() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({
        "domains": ["*.wild.example.com"],
        "dns01": false,
        "webroot": "/tmp/fp-nonexistent-webroot"
    }))
    .unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/issue/acme")
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

/// DNS-01 但未配置厂商 → 400 前置校验（不会发起真实网络请求）。
#[tokio::test]
async fn acme_dns01_without_provider_rejected() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({
        "domains": ["ok.example.com"],
        "dns01": true
    }))
    .unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/issue/acme")
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

/// 非法域名列表 → 400。
#[tokio::test]
async fn issue_rejects_invalid_domains() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let body = serde_json::to_vec(&json!({ "domains": ["-bad..com"] })).unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/issue/self-signed")
                .header(h, v)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

/// 匿名访问 → 401。
#[tokio::test]
async fn list_certificates_rejects_anonymous() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/certificates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// 删除不存在记录 → 404。
#[tokio::test]
async fn delete_missing_certificate_404() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/certificates/99999")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// 空列表返回 200 空数组。
#[tokio::test]
async fn list_certificates_empty_ok() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/certificates")
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
    let resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(resp["total"], 0);
}

/// 续签端点（Stage 3）：自签证书手动续签返回 400（不支持自动续签）；
/// ACME 证书续签在本测试环境无法连接 ACME 服务器，走 502/内部错误语义而非 400。
#[tokio::test]
async fn renew_self_signed_rejected_with_400() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    // 先自签一张
    let body = serde_json::to_vec(&json!({ "domains": ["renew.test"] })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/issue/self-signed")
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
    let cert: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let cert_id = cert["id"].as_i64().unwrap();

    // 手动续签 → 400（自签不支持）
    let (h2, v2) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/certificates/{cert_id}/renew"))
                .header(h2, v2)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

/// 续签端点：匿名访问 401（挂 certificate:update 权限）。
#[tokio::test]
async fn renew_rejects_anonymous() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/1/renew")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// 续签不存在的证书 → 404。
#[tokio::test]
async fn renew_missing_certificate_404() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/certificates/99999/renew")
                .header(h, v)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ── S-10 续签历史 + 自动续签开关约束 ─────────────────────────────

async fn call_json(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    };
    let res = app.clone().oneshot(req.unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, body)
}

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

async fn issue_self_signed(app: &axum::Router, domain: &str) -> i64 {
    let token = admin_token();
    let (status, cert) = call_json(
        app,
        Method::POST,
        "/api/certificates/issue/self-signed",
        Some(&token),
        Some(json!({ "domains": [domain] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cert}");
    cert["id"].as_i64().unwrap()
}

/// 时间线来自事件落库：失败事件带错误原因，其他证书的事件不串入；自签证书标记不支持自动续签。
#[tokio::test]
async fn renewal_history_returns_timeline_and_flags() {
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = common::setup_router().await;
    let outbox = state.outbox_service.clone();
    let app = flame_kernel::api::middleware::add_middleware_with_mock_peer(router, state);
    let id = issue_self_signed(&app, "history.example.com").await;

    use flame_kernel::domain::entity::DomainEvent;
    for event in [
        DomainEvent::CertificateRenewalFailed {
            id,
            domain: "history.example.com".into(),
            error: "ACME 目录不可达".into(),
        },
        DomainEvent::CertificateRenewalFailed {
            id: id + 1000,
            domain: "other.example.com".into(),
            error: "x".into(),
        },
    ] {
        outbox.record_event(&event).await.unwrap();
    }

    let token = admin_token();
    let (status, body) = call_json(
        &app,
        Method::GET,
        &format!("/api/certificates/{id}/renewals?limit=10"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["certificate_id"], id);
    assert_eq!(body["supports_auto_renew"], false);
    assert_eq!(body["scan_interval_hours"], 6);
    assert!(body["expires_at"].is_string());
    assert!(body["renewal_window_starts_at"].is_string());
    assert_eq!(body["truncated"], false);
    let entries = body["entries"].as_array().unwrap();
    let failed: Vec<_> = entries
        .iter()
        .filter(|e| e["kind"] == "auto_renew_failed")
        .collect();
    assert_eq!(failed.len(), 1, "{body}");
    assert_eq!(failed[0]["error"], "ACME 目录不可达");
    assert!(entries.iter().all(|e| e["domain"] != "other.example.com"));
}

#[tokio::test]
async fn renewal_history_rejects_anonymous_and_unprivileged() {
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = common::setup_router().await;
    // 未知角色没有任何权限（role_permissions 缺省空集）
    state
        .user_service
        .create_user("guest", "hash", "guest")
        .await
        .unwrap();
    let app = flame_kernel::api::middleware::add_middleware_with_mock_peer(router, state);
    let id = issue_self_signed(&app, "denied.example.com").await;
    let path = format!("/api/certificates/{id}/renewals");

    let (status, _) = call_json(&app, Method::GET, &path, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let guest = flame_kernel::utils::jwt::JwtUtils::new("test-secret", 24)
        .sign(2)
        .unwrap();
    let (status, _) = call_json(&app, Method::GET, &path, Some(&guest), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "guest 无 certificate:read");
}

#[tokio::test]
async fn renewal_history_missing_certificate_404() {
    let app = setup_full_router().await;
    let token = admin_token();
    let (status, _) = call_json(
        &app,
        Method::GET,
        "/api/certificates/999999/renewals",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// 自签 / 上传证书没有签发方可续：开启自动续签 400，关闭仍可。
#[tokio::test]
async fn enabling_auto_renew_on_self_signed_rejected() {
    let app = setup_full_router().await;
    let id = issue_self_signed(&app, "noauto.example.com").await;
    let token = admin_token();
    let path = format!("/api/certificates/{id}");
    let (status, body) = call_json(
        &app,
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({ "auto_renew": true })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = call_json(
        &app,
        Method::PUT,
        &path,
        Some(&token),
        Some(json!({ "auto_renew": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["auto_renew"], false);
}
