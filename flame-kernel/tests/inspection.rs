//! 系统体检端点（S-17）：`GET /api/inspection` / `POST /api/inspection/run`。
//!
//! 覆盖：401 / 报告形状（内核信息 + 汇总 + 风险项）/ 服务端缓存与强制重跑 /
//! 角色裁剪（viewer 可读不可执行）/ 已知风险事实 → 对应 finding 判定。

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::{auth_header, setup_full_router, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::domain::inspection::{StaticFactsSource, SystemFacts};
use flame_kernel::utils::jwt::JwtUtils;

const GET_PATH: &str = "/api/inspection";
const RUN_PATH: &str = "/api/inspection/run";

fn admin_token() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

fn risky_facts() -> SystemFacts {
    SystemFacts {
        arch: "x86_64".into(),
        proc_version: Some(
            "Linux version 5.4.0-150-generic (buildd@lcy02) (gcc version 9.4.0) #167-Ubuntu SMP"
                .into(),
        ),
        osrelease: Some("5.4.0-150-generic".into()),
        hostname: Some("test-host".into()),
        os_release_file: Some(
            "NAME=\"Ubuntu\"\nID=ubuntu\nVERSION_ID=\"20.04\"\nPRETTY_NAME=\"Ubuntu 20.04.6 LTS\"\n"
                .into(),
        ),
        sshd_config: Some("Port 22\nPermitRootLogin yes\nPasswordAuthentication yes\n".into()),
        shadow_mode: Some(0o644),
        ..Default::default()
    }
}

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(path);
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

#[tokio::test]
async fn inspection_requires_authentication() {
    let app = setup_full_router().await;
    let (status, _) = call(&app, Method::GET, GET_PATH, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&app, Method::POST, RUN_PATH, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_gets_report_with_kernel_info_and_findings() {
    let (router, state) = setup_router().await;
    state
        .inspection_service
        .set_source(Arc::new(StaticFactsSource(risky_facts())));
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();

    let (status, body) = call(&app, Method::GET, GET_PATH, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["kernel"]["release"], "5.4.0-150-generic");
    assert_eq!(body["kernel"]["hostname"], "test-host");
    assert_eq!(body["kernel"]["os_name"], "Ubuntu 20.04.6 LTS");
    assert_eq!(body["kernel"]["os_id"], "ubuntu");
    assert!(body["generated_at"].is_string());
    assert!(body["data_version"].is_string());
    assert_eq!(body["cache_age_secs"], 0);
    assert!(body["summary"]["score"].as_u64().unwrap() <= 100);
    assert!(body["summary"]["total"].as_u64().unwrap() >= 10);

    let findings = body["findings"].as_array().expect("findings 数组");
    let by_code = |code: &str| {
        findings
            .iter()
            .find(|f| f["code"] == code)
            .unwrap_or_else(|| panic!("缺少 finding {code}"))
            .clone()
    };
    // 已知风险事实 → 对应判定
    assert_eq!(by_code("sshd_root_login")["status"], "fail");
    assert_eq!(by_code("sshd_password_auth")["status"], "fail");
    assert_eq!(by_code("shadow_permissions")["status"], "fail");
    assert_eq!(by_code("shadow_permissions")["severity"], "high");
    // 生命周期结论必须标注启发式
    let kernel_eol = by_code("kernel_eol");
    assert_eq!(kernel_eol["heuristic"], true);
    // 每条结论都带稳定编码、类别与语言无关取证
    for f in findings {
        assert!(f["code"].is_string());
        assert!(f["category"].is_string());
        assert!(f["evidence"].is_string());
    }
}

#[tokio::test]
async fn report_is_cached_and_run_forces_refresh() {
    let (router, state) = setup_router().await;
    state
        .inspection_service
        .set_source(Arc::new(StaticFactsSource(risky_facts())));
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let token = admin_token();

    let (_, first) = call(&app, Method::GET, GET_PATH, Some(&token)).await;
    let (_, second) = call(&app, Method::GET, GET_PATH, Some(&token)).await;
    assert_eq!(
        first["generated_at"], second["generated_at"],
        "TTL 内的 GET 命中同一份报告"
    );

    let (status, forced) = call(&app, Method::POST, RUN_PATH, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{forced}");
    assert_eq!(forced["cache_age_secs"], 0);
    assert!(forced["generated_at"].as_str().unwrap() >= first["generated_at"].as_str().unwrap());
    assert_eq!(forced["kernel"]["release"], "5.4.0-150-generic");
}

#[tokio::test]
async fn viewer_can_read_but_cannot_run() {
    let (router, state) = setup_router().await;
    state
        .inspection_service
        .set_source(Arc::new(StaticFactsSource(risky_facts())));
    state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let token = JwtUtils::new("test-secret", 24).sign(2).unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state);

    let (status, body) = call(&app, Method::GET, GET_PATH, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "viewer 持有 toolbox:read → {body}");
    assert!(body["findings"].is_array());

    let (status, _) = call(&app, Method::POST, RUN_PATH, Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "viewer 无 toolbox:execute");
}
