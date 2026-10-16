//! WASM 插件宿主 API + 插件市场（batch-14，Doc/21 §5.7–5.8）。
//!
//! 覆盖：401 / viewer 可浏览市场不可安装、不可改能力（403）/ 能力申请须显式同意（400）/
//! 内置插件安装 → 执行（input/output/logs）/ 重复安装 409 / 能力授予校验（敏感目录 400）/
//! 设置经 API 写入后持久化到库 / 卸载删除库记录。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::utils::jwt::JwtUtils;

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn admin() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

#[tokio::test]
async fn market_requires_authentication() {
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let (s, _) = call(&app, Method::GET, "/api/plugins/market", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins/market/flame-hello/install",
        None,
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(
        &app,
        Method::PUT,
        "/api/plugins/x/capabilities",
        None,
        Some(json!({"capabilities": []})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn viewer_can_browse_but_not_install_or_grant() {
    let (router, state) = setup_router().await;
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let viewer = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (s, body) = call(
        &app,
        Method::GET,
        "/api/plugins/market",
        Some(&viewer),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "viewer 持有 plugin:read → {body}");
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins/market/flame-hello/install",
        Some(&viewer),
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "viewer 无 plugin:create");
    let (s, _) = call(
        &app,
        Method::PUT,
        "/api/plugins/flame-hello/capabilities",
        Some(&viewer),
        Some(json!({"capabilities": ["metrics:read"]})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn market_install_execute_and_grant_flow() {
    let (router, state) = setup_router().await;
    let repo = state.plugin_repo.clone();
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let t = admin();

    // 目录：3 个内置插件，能力清单可见，均未安装
    let (s, body) = call(&app, Method::GET, "/api/plugins/market", Some(&t), None).await;
    assert_eq!(s, StatusCode::OK, "{body}");
    let list = body["plugins"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    let metrics = list
        .iter()
        .find(|p| p["id"] == "flame-metrics-snapshot")
        .unwrap();
    assert_eq!(metrics["capabilities"], json!(["metrics:read"]));
    assert_eq!(metrics["source"], "builtin");
    assert_eq!(metrics["installed"], false);
    assert_eq!(metrics["wasm_sha256"].as_str().unwrap().len(), 64);
    assert!(body["remote_error"].is_null());

    // 申请能力的插件未同意 → 400；同意 → 200 且能力已授予
    let path = "/api/plugins/market/flame-metrics-snapshot/install";
    let (s, _) = call(&app, Method::POST, path, Some(&t), Some(json!({}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, body) = call(
        &app,
        Method::POST,
        path,
        Some(&t),
        Some(json!({"accept_capabilities": true})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["capabilities"], json!(["metrics:read"]));
    let (s, _) = call(
        &app,
        Method::POST,
        path,
        Some(&t),
        Some(json!({"accept_capabilities": true})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "重复安装");
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins/market/nope/install",
        Some(&t),
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // 无能力插件无需同意；执行带 input，返回 output 与日志
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins/market/flame-hello/install",
        Some(&t),
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, body) = call(
        &app,
        Method::POST,
        "/api/plugins/flame-hello/execute/run",
        Some(&t),
        Some(json!({"input": "Flame"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["output_string"], "Hello, Flame!");
    assert_eq!(body["logs"], json!(["[INFO] hello plugin invoked"]));

    let (_, body) = call(&app, Method::GET, "/api/plugins/market", Some(&t), None).await;
    let hello = body["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "flame-hello")
        .unwrap()
        .clone();
    assert_eq!(hello["installed"], true);
    assert_eq!(hello["installed_version"], "1.0.0");

    // 能力授予：敏感目录 / 未知能力 → 400；合法 → 200 并落库
    let cap_path = "/api/plugins/flame-hello/capabilities";
    for bad in [
        json!(["fs:read:/"]),
        json!(["fs:write:/etc"]),
        json!(["exec:*"]),
    ] {
        let (s, _) = call(
            &app,
            Method::PUT,
            cap_path,
            Some(&t),
            Some(json!({"capabilities": bad})),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (s, body) = call(
        &app,
        Method::PUT,
        cap_path,
        Some(&t),
        Some(json!({"capabilities": ["settings:write", "fs:read:/var/log/"]})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(
        body["capabilities"],
        json!(["settings:write", "fs:read:/var/log"])
    );
    let saved = repo.find_by_id("flame-hello").await.unwrap().unwrap();
    assert_eq!(
        saved.capabilities,
        vec!["settings:write", "fs:read:/var/log"]
    );
    let (s, _) = call(
        &app,
        Method::PUT,
        "/api/plugins/nope/capabilities",
        Some(&t),
        Some(json!({"capabilities": []})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // 设置经 API 写入后落库（旧实现只在内存）
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins/flame-hello/settings",
        Some(&t),
        Some(json!({"key": "greeting", "value": "hi"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let saved = repo.find_by_id("flame-hello").await.unwrap().unwrap();
    assert_eq!(
        saved.settings.get("greeting").map(String::as_str),
        Some("hi")
    );

    // 卸载删除库记录（旧实现不删库，重启后插件复活）
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins/flame-hello",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(repo.find_by_id("flame-hello").await.unwrap().is_none());
}

#[tokio::test]
async fn load_endpoint_validates_capabilities_and_conflicts() {
    use base64::{engine::general_purpose, Engine as _};
    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let t = admin();
    let b64 = general_purpose::STANDARD.encode(wat::parse_str("(module)").unwrap());

    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins",
        Some(&t),
        Some(json!({"id": "c1", "wasm_base64": b64, "capabilities": ["fs:read:/proc"]})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins",
        Some(&t),
        Some(json!({"id": "market", "wasm_base64": b64})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "保留 ID");
    let (s, body) = call(
        &app,
        Method::POST,
        "/api/plugins",
        Some(&t),
        Some(json!({"id": "c1", "wasm_base64": b64, "capabilities": ["metrics:read"]})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["capabilities"], json!(["metrics:read"]));
    let (s, _) = call(
        &app,
        Method::POST,
        "/api/plugins",
        Some(&t),
        Some(json!({"id": "c1", "wasm_base64": b64})),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
}
