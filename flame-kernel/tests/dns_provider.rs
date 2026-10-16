//! DNS 厂商账号 REST 集成测试（主线三 §6.1 · Stage 4）
//!
//! 覆盖：CRUD 流 / 凭据不回显 / 连通性测试（成功与缺字段失败）/ 权限拦截。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_full_router};

#[tokio::test]
async fn test_dns_provider_api_flow() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // create（非法厂商类型拒绝）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dns-providers")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"bad","provider_type":"godaddy"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // create 成功（cloudflare）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dns-providers")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"cf-main","provider_type":"cloudflare","credentials_json":"{\"api_token\":\"tok-123\"}"}"#,
                ))
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
    let id = body["id"].as_i64().unwrap();
    assert_eq!(body["provider_type"], "cloudflare");
    // credentials_json 不回显（防厂商 API 凭据泄漏）
    assert!(body.get("credentials_json").is_none());

    // list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/dns-providers")
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
    assert_eq!(body.as_array().unwrap().len(), 1);

    // test connectivity（凭据完整 → ok）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/dns-providers/{id}/test"))
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
    assert_eq!(body["last_test_status"], "ok");

    // update（补齐 aliyun 凭据）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/dns-providers/{id}"))
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "ali-main",
                        "provider_type": "aliyun",
                        "credentials_json": json!({
                            "access_key_id": "ak",
                            "access_key_secret": "sk"
                        }).to_string()
                    })
                    .to_string(),
                ))
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
    assert_eq!(body["name"], "ali-main");
    assert_eq!(body["provider_type"], "aliyun");

    // test connectivity（aliyun 凭据完整 → ok）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/dns-providers/{id}/test"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // create 缺必需字段账号 → test 返回 failed
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dns-providers")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"cf-empty","provider_type":"cloudflare"}"#,
                ))
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
    let id2 = body["id"].as_i64().unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/dns-providers/{id2}/test"))
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
    assert_eq!(body["last_test_status"], "failed");
    assert!(body["test_message"].as_str().unwrap().contains("api_token"));

    // delete
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/dns-providers/{id2}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/dns-providers/{id2}"))
                .header(h.clone(), v.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_dns_provider_requires_auth() {
    let app = setup_full_router().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/dns-providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Stage 5：ACME 签发指定厂商账号——
/// - 账号不存在 → 404；
/// - 账号存在但该厂商无真实 API 交付（tencent/huawei，Plan Step 3）→ 400 明确报错。
/// 两个分支都不会发起真实 DNS/ACME 网络请求。
#[tokio::test]
async fn issue_acme_with_dns_provider_account_validation() {
    let app = setup_full_router().await;
    let (h, v) = auth_header();

    // 指定不存在的账号 → 404
    let body = serde_json::to_vec(&json!({
        "domains": ["a.example.com"],
        "dns01": true,
        "dns_provider_account_id": 99999
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/certificates/issue/acme")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 创建 tencent 账号（真实 API 未交付，build_provider 明确拒绝）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dns-providers")
                .header(h.clone(), v.clone())
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"tx","provider_type":"tencent","credentials_json":"{\"secret_id\":\"a\",\"secret_key\":\"b\"}"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let created: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let tx_id = created["id"].as_i64().unwrap();

    let body = serde_json::to_vec(&json!({
        "domains": ["b.example.com"],
        "dns01": true,
        "dns_provider_account_id": tx_id
    }))
    .unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/certificates/issue/acme")
                .header(h, v)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let msg: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msg_str = msg.to_string();
    assert!(
        msg_str.contains("尚未交付") || msg_str.contains("无可用"),
        "应明确说明厂商不可用: {msg_str}"
    );
}
