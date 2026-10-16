//! 内核日志端点（W0 · Doc/32 §4）集成测试。
//!
//! 覆盖：401 / viewer 可读不可删不可调级（403）/ 管理员读取注入目录的倒序分页、级别与关键字过滤、
//! 统计、下载头部 / 非法文件名与当前文件删除 400 / 调级参数校验 / 文件日志未启用。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::utils::jwt::JwtUtils;

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

async fn call_raw(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
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
    let headers = res.headers().clone();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, headers, bytes.to_vec())
}

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let (status, _, bytes) = call_raw(app, method, path, token, body).await;
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

const OLD: &str = "flamepanel.2026-09-20.log";

fn seed_dir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let lines = [
        r#"{"timestamp":"2026-09-20T10:00:00Z","level":"INFO","fields":{"message":"kernel starting"},"target":"flame_kernel"}"#,
        r#"{"timestamp":"2026-09-20T10:00:01Z","level":"WARN","fields":{"message":"retrying docker"},"target":"flame_kernel::docker"}"#,
        r#"{"timestamp":"2026-09-20T10:00:02Z","level":"ERROR","fields":{"message":"Internal server error","error":"db locked"},"target":"flame_kernel::core::error","spans":[{"name":"http_request","request_id":"req-abc"}]}"#,
        r#"{"timestamp":"2026-09-20T10:00:03Z","level":"DEBUG","fields":{"message":"cache hit"},"target":"flame_kernel::api"}"#,
    ];
    std::fs::write(tmp.path().join(OLD), lines.join("\n") + "\n").unwrap();
    std::fs::write(tmp.path().join("secret.txt"), "top secret").unwrap();
    tmp
}

#[tokio::test]
async fn kernel_logs_reject_anonymous() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    for path in [
        "/api/kernel-logs",
        "/api/kernel-logs/entries",
        "/api/kernel-logs/stats?file=a.log",
    ] {
        let (status, _) = call(&app, Method::GET, path, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/kernel-logs/level",
        None,
        Some(serde_json::json!({"level": "debug"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_reads_filters_downloads_and_deletes() {
    let tmp = seed_dir();
    let (router, state) = setup_router().await;
    state
        .kernel_log_service
        .set_dir(Some(tmp.path().to_path_buf()));
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();

    let (status, ov) = call(&app, Method::GET, "/api/kernel-logs", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "{ov}");
    assert_eq!(ov["enabled"], true);
    let names: Vec<&str> = ov["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec![OLD], "非 .log 文件不应出现在列表中");

    // 倒序：最新（DEBUG）在前
    let (status, page) = call(
        &app,
        Method::GET,
        &format!("/api/kernel-logs/entries?file={OLD}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["entries"][0]["level"], "debug");
    assert_eq!(page["entries"].as_array().unwrap().len(), 4);
    assert!(page["next_before"].is_null());

    // 级别过滤 + 关键字命中 span 中的请求编号
    let (_, page) = call(
        &app,
        Method::GET,
        &format!("/api/kernel-logs/entries?file={OLD}&levels=warn,error&q=REQ-ABC"),
        Some(&token),
        None,
    )
    .await;
    let entries = page["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["request_id"], "req-abc");
    assert_eq!(entries[0]["fields"]["error"], "db locked");

    // 统计
    let (status, st) = call(
        &app,
        Method::GET,
        &format!("/api/kernel-logs/stats?file={OLD}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(st["counts"]["total"], 4);
    assert_eq!(st["counts"]["error"], 1);

    // 下载：附件头 + 原文
    let (status, headers, body) = call_raw(
        &app,
        Method::GET,
        &format!("/api/kernel-logs/files/{OLD}/download"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .contains(OLD));
    assert_eq!(String::from_utf8(body).unwrap().lines().count(), 4);

    // 非法参数 / 路径穿越
    for path in [
        "/api/kernel-logs/entries?levels=loud".to_string(),
        "/api/kernel-logs/entries?file=secret.txt".to_string(),
        "/api/kernel-logs/stats?file=..%2Fsecret.txt".to_string(),
    ] {
        let (status, body) = call(&app, Method::GET, &path, Some(&token), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
    }
    let (status, _) = call(
        &app,
        Method::GET,
        "/api/kernel-logs/entries?file=missing.log",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 调级：非法 400；合法但测试进程未初始化全局日志 → 409
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/kernel-logs/level",
        Some(&token),
        Some(serde_json::json!({"level": "loud"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/kernel-logs/level",
        Some(&token),
        Some(serde_json::json!({"level": "debug"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 删除历史文件
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/kernel-logs/files/{OLD}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!tmp.path().join(OLD).exists());
    assert!(tmp.path().join("secret.txt").exists());
}

#[tokio::test]
async fn viewer_can_read_but_not_delete_or_change_level() {
    let tmp = seed_dir();
    let (router, state) = setup_router().await;
    state
        .kernel_log_service
        .set_dir(Some(tmp.path().to_path_buf()));
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, _) = call(&app, Method::GET, "/api/kernel-logs", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "viewer 持有 log:read");
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/kernel-logs/files/{OLD}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/kernel-logs/level",
        Some(&token),
        Some(serde_json::json!({"level": "debug"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(tmp.path().join(OLD).exists());
}

#[tokio::test]
async fn disabled_file_logging_reports_not_enabled() {
    let (router, state) = setup_router().await;
    state.kernel_log_service.set_dir(None);
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();
    let (status, ov) = call(&app, Method::GET, "/api/kernel-logs", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ov["enabled"], false);
    let (status, _) = call(
        &app,
        Method::GET,
        "/api/kernel-logs/entries",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
