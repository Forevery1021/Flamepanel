//! TOTP 两步验证（Doc/33 排除项「TOTP 2FA」）：登录两步流程、自助管理、管理员重置与强制策略。

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;
use common::{auth_header, bcrypt_hash, setup_router};

use flame_kernel::api::middleware;
use flame_kernel::api::types::AppState;
use flame_kernel::utils::jwt::JwtUtils;
use flame_kernel::utils::totp;

const PW: &str = "Str0ng-Passw0rd!";

struct Resp {
    status: StatusCode,
    json: Value,
    set_cookie: bool,
}

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Resp {
    let mut b = Request::builder().method(method).uri(uri);
    // Login 档限流固定 5 次/分钟/IP，且全局限流器被并行测试共享（每个 app() 还会重置它）：
    // 每次密码登录换一个来源地址（对端为回环，XFF 受信），避免跨测试偶发 429。
    if uri == "/api/auth/login" {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        b = b.header(
            "x-forwarded-for",
            format!("10.77.{}.{}", n / 250, n % 250 + 1),
        );
    }
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => b
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let set_cookie = res.headers().contains_key(header::SET_COOKIE);
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    Resp {
        status,
        json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        set_cookie,
    }
}

fn admin() -> String {
    auth_header().1.strip_prefix("Bearer ").unwrap().to_string()
}

/// 带完整中间件的路由 + 一个真实 bcrypt 密码的普通运维用户。
async fn app_with_user(role: &str) -> (axum::Router, AppState, i64) {
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = setup_router().await;
    let u = state
        .user_service
        .create_user("alice", &bcrypt_hash(PW), role)
        .await
        .unwrap();
    let app = middleware::add_middleware_with_mock_peer(router, state.clone());
    (app, state, u.id)
}

fn code_at(secret: &str, offset_steps: i64) -> String {
    let key = totp::base32_decode(secret).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!(
        "{:06}",
        totp::hotp(&key, (totp::step_at(now) as i64 + offset_steps) as u64)
    )
}

async fn login(app: &axum::Router) -> Resp {
    send(
        app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"username": "alice", "password": PW})),
    )
    .await
}

/// 为 alice 启用 2FA，返回 (密钥, 恢复码)
async fn enable_for(app: &axum::Router, token: &str) -> (String, Vec<String>) {
    let r = send(
        app,
        Method::POST,
        "/api/auth/2fa/setup",
        Some(token),
        Some(json!({"password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    let secret = r.json["secret"].as_str().unwrap().to_string();
    assert!(r.json["otpauth_uri"]
        .as_str()
        .unwrap()
        .starts_with("otpauth://totp/FlamePanel:alice?"));
    let r = send(
        app,
        Method::POST,
        "/api/auth/2fa/enable",
        Some(token),
        Some(json!({"code": code_at(&secret, 0)})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    let codes: Vec<String> = serde_json::from_value(r.json["recovery_codes"].clone()).unwrap();
    assert_eq!(codes.len(), 10);
    (secret, codes)
}

#[tokio::test]
async fn two_factor_endpoints_require_auth() {
    let (app, _, _) = app_with_user("operator").await;
    for (m, p) in [
        (Method::GET, "/api/auth/2fa"),
        (Method::POST, "/api/auth/2fa/setup"),
        (Method::POST, "/api/auth/2fa/enable"),
        (Method::POST, "/api/auth/2fa/disable"),
        (Method::POST, "/api/auth/2fa/recovery-codes"),
        (Method::PUT, "/api/users/1/2fa/reset"),
    ] {
        let r = send(&app, m.clone(), p, None, Some(json!({}))).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{m} {p}");
    }
}

#[tokio::test]
async fn login_requires_second_factor_once_enabled() {
    let (app, _, _) = app_with_user("operator").await;
    let r = login(&app).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["two_factor_required"], false);
    let token = r.json["token"].as_str().unwrap().to_string();

    let st = send(&app, Method::GET, "/api/auth/2fa", Some(&token), None).await;
    assert_eq!(st.status, StatusCode::OK);
    assert_eq!(st.json["enabled"], false);
    assert_eq!(st.json["required"], false);

    // 设置需要当前密码；错误密码 400（非 401，前端不会误判会话失效）
    let bad = send(
        &app,
        Method::POST,
        "/api/auth/2fa/setup",
        Some(&token),
        Some(json!({"password": "nope"})),
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);

    let (secret, codes) = enable_for(&app, &token).await;
    let st = send(&app, Method::GET, "/api/auth/2fa", Some(&token), None).await;
    assert_eq!(st.json["enabled"], true);
    assert_eq!(st.json["recovery_codes_remaining"], 10);

    // 第一步只返回挑战令牌：无 access token、不下发 refresh Cookie
    let r = login(&app).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["two_factor_required"], true);
    assert_eq!(r.json["token"], "");
    assert!(!r.set_cookie);
    let challenge = r.json["challenge"].as_str().unwrap().to_string();

    // 挑战令牌不能当 access token 用
    let r = send(&app, Method::GET, "/api/auth/me", Some(&challenge), None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // 错误验证码 401；启用时用过的码不可重放
    let r = send(
        &app,
        Method::POST,
        "/api/auth/login/2fa",
        None,
        Some(json!({"challenge": challenge, "code": "000000"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = send(
        &app,
        Method::POST,
        "/api/auth/login/2fa",
        None,
        Some(json!({"challenge": challenge, "code": code_at(&secret, 0)})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // 下一个步号的验证码通过，拿到完整会话
    let r = send(
        &app,
        Method::POST,
        "/api/auth/login/2fa",
        None,
        Some(json!({"challenge": challenge, "code": code_at(&secret, 1)})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    assert!(r.set_cookie);
    let token2 = r.json["token"].as_str().unwrap().to_string();
    assert!(!token2.is_empty());
    assert_eq!(
        send(&app, Method::GET, "/api/auth/me", Some(&token2), None)
            .await
            .status,
        StatusCode::OK
    );

    // 恢复码登录（一次性）
    let challenge = login(&app).await.json["challenge"]
        .as_str()
        .unwrap()
        .to_string();
    let r = send(
        &app,
        Method::POST,
        "/api/auth/login/2fa",
        None,
        Some(json!({"challenge": challenge, "code": codes[0]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = send(
        &app,
        Method::POST,
        "/api/auth/login/2fa",
        None,
        Some(json!({"challenge": challenge, "code": codes[0]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // 伪造 / 其他类型的挑战令牌被拒
    let access_as_challenge = JwtUtils::new("test-secret", 24).sign(1).unwrap();
    let r = send(
        &app,
        Method::POST,
        "/api/auth/login/2fa",
        None,
        Some(json!({"challenge": access_as_challenge, "code": code_at(&secret, 1)})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // 停用：错误验证码 400；密码 + 恢复码可停用
    let r = send(
        &app,
        Method::POST,
        "/api/auth/2fa/disable",
        Some(&token2),
        Some(json!({"password": PW, "code": "123456"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = send(
        &app,
        Method::POST,
        "/api/auth/2fa/disable",
        Some(&token2),
        Some(json!({"password": PW, "code": codes[1]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    let r = login(&app).await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);
    assert_eq!(r.json["two_factor_required"], false);
}

#[tokio::test]
async fn admin_reset_requires_user_update_permission() {
    let (app, state, alice) = app_with_user("operator").await;
    let token = login(&app).await.json["token"]
        .as_str()
        .unwrap()
        .to_string();
    enable_for(&app, &token).await;

    // operator 虽有 user:update，但重置第二因子仅限 admin → 403；viewer 无 user:update → 403
    let r = send(
        &app,
        Method::PUT,
        &format!("/api/users/{alice}/2fa/reset"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let viewer = state
        .user_service
        .create_user("viewer", "hash", "viewer")
        .await
        .unwrap();
    let vt = JwtUtils::new("test-secret", 24).sign(viewer.id).unwrap();
    let r = send(
        &app,
        Method::PUT,
        &format!("/api/users/{alice}/2fa/reset"),
        Some(&vt),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // admin 可重置；不存在的用户 404
    let r = send(
        &app,
        Method::PUT,
        &format!("/api/users/{alice}/2fa/reset"),
        Some(&admin()),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["reset"], true);
    assert_eq!(login(&app).await.json["two_factor_required"], false);
    let r = send(
        &app,
        Method::PUT,
        "/api/users/9999/2fa/reset",
        Some(&admin()),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn panel_policy_forces_setup_before_other_apis() {
    let (app, _, _) = app_with_user("operator").await;
    let r = send(
        &app,
        Method::PUT,
        "/api/settings",
        Some(&admin()),
        Some(json!({"key": "two_factor_enabled", "value": "true"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json);

    let r = login(&app).await;
    assert_eq!(r.json["must_setup_two_factor"], true);
    let token = r.json["token"].as_str().unwrap().to_string();

    // 未绑定：业务接口 403 TWO_FACTOR_SETUP_REQUIRED；绑定相关端点放行
    let r = send(&app, Method::GET, "/api/websites", Some(&token), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["error"], "TWO_FACTOR_SETUP_REQUIRED");
    let st = send(&app, Method::GET, "/api/auth/2fa", Some(&token), None).await;
    assert_eq!(st.json["required"], true);
    assert_eq!(
        send(&app, Method::GET, "/api/auth/me", Some(&token), None)
            .await
            .status,
        StatusCode::OK
    );

    let (secret, _) = enable_for(&app, &token).await;
    let r = send(&app, Method::GET, "/api/websites", Some(&token), None).await;
    assert_eq!(r.status, StatusCode::OK);

    // 强制期间不可自行停用
    let r = send(
        &app,
        Method::POST,
        "/api/auth/2fa/disable",
        Some(&token),
        Some(json!({"password": PW, "code": code_at(&secret, 1)})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
}
