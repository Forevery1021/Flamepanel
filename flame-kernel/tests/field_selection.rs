//! `?fields=` 响应裁剪中间件（`api::field_selection`）。
//!
//! 覆盖：分页信封只裁列表、元数据保留 / 单对象裁顶层 / 非法字段 400 / 鉴权先于裁剪（401、403
//! 不受影响）/ 未指定 fields 时响应不变 / 非 GET 不处理。

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::domain::host_info::{HostFacts, StaticHostFacts};
use flame_kernel::domain::toolbox::{ProcessInfo, StaticProcessSource};
use flame_kernel::utils::jwt::JwtUtils;

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
) -> (StatusCode, Option<String>, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = req
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let applied = res
        .headers()
        .get("x-fields-applied")
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, applied, body)
}

fn proc(pid: u32, name: &str) -> ProcessInfo {
    ProcessInfo {
        pid,
        ppid: Some(1),
        name: name.into(),
        user: Some("root".into()),
        uid: Some(0),
        command: format!("/usr/bin/{name}"),
        cpu: 1.0,
        memory_bytes: 1 << 20,
        status: "Sleep".into(),
        started_at: 1_700_000_000,
        kernel_thread: false,
    }
}

async fn app() -> axum::Router {
    let (router, state) = setup_router().await;
    state
        .toolbox_service
        .set_source(Arc::new(StaticProcessSource(vec![
            proc(10, "nginx"),
            proc(20, "node"),
        ])));
    state
        .toolbox_service
        .set_host_source(Arc::new(StaticHostFacts(HostFacts {
            hostname: Some("web-01".into()),
            arch: "x86_64".into(),
            ..Default::default()
        })));
    state
        .user_service
        .create_user("norole", "hash", "norole")
        .await
        .unwrap();
    middleware::add_middleware_with_mock_peer(router, state)
}

#[tokio::test]
async fn envelope_lists_are_trimmed_and_metadata_kept() {
    let app = app().await;
    let token = admin_token();

    let (status, applied, full) =
        call(&app, Method::GET, "/api/toolbox/processes", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(applied, None, "未指定 fields 不裁剪");
    assert!(full["items"][0].get("command").is_some());
    assert!(full["items"][0].get("protected").is_some());

    let (status, applied, body) = call(
        &app,
        Method::GET,
        "/api/toolbox/processes?sort=pid&order=asc&fields=pid,name",
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(applied.as_deref(), Some("1"));
    assert_eq!(
        body["items"],
        serde_json::json!([{"pid": 10, "name": "nginx"}, {"pid": 20, "name": "node"}])
    );
    assert_eq!(body["total"], 2, "分页元数据保留");
    assert_eq!(body["matched"], 2);
}

#[tokio::test]
async fn single_object_top_level_is_trimmed() {
    let app = app().await;
    let (status, _, body) = call(
        &app,
        Method::GET,
        "/api/toolbox/host?fields=system,panel",
        Some(&admin_token()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let keys: Vec<&String> = body.as_object().unwrap().keys().collect();
    assert_eq!(keys.len(), 2, "{keys:?}");
    assert_eq!(body["system"]["hostname"], "web-01");
}

#[tokio::test]
async fn invalid_fields_are_rejected_with_400() {
    let app = app().await;
    let (status, applied, _) = call(
        &app,
        Method::GET,
        "/api/toolbox/processes?fields=pid,na.me",
        Some(&admin_token()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(applied, None);
}

#[tokio::test]
async fn auth_runs_before_field_selection() {
    let app = app().await;
    let (status, _, _) = call(&app, Method::GET, "/api/toolbox/processes?fields=a.b", None).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "未认证先 401，不暴露字段校验"
    );

    let norole = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let (status, applied, _) = call(
        &app,
        Method::GET,
        "/api/toolbox/processes?fields=pid",
        Some(&norole),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(applied, None, "错误响应不裁剪");
}

#[tokio::test]
async fn non_get_requests_are_untouched() {
    let app = app().await;
    // POST 带非法 fields 也不会被本中间件拦截（交给处理器：进程 4242 不存在 → 404）
    let (status, applied, _) = call(
        &app,
        Method::POST,
        "/api/toolbox/processes/4242/kill?fields=a.b",
        Some(&admin_token()),
    )
    .await;
    assert_ne!(status, StatusCode::BAD_REQUEST);
    assert_eq!(applied, None);
}
