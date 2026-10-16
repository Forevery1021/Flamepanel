use crate::api::extract::ApiJson;
use crate::api::types::{AppState, UserId};
use crate::core::error::AppError;
use crate::utils::password::PasswordUtils;
use axum::Router;
use axum::{
    extract::{Extension, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

pub mod two_factor;

/// M1：用户不存在时用于 dummy bcrypt 校验的哈希，抹平时序差异（成本与真实校验一致）。
/// 这是对某个随机密码预计算的 bcrypt hash（cost=12），无实际用户与之对应。
const DUMMY_HASH: &str = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxNrLCnYg4fF4GcRQ1V3n1y3q7S";

/// Y1：refresh token 迁入 httpOnly Cookie（XSS 无法读取，降低凭证泄露面）。
/// Cookie 名；`HttpOnly` 禁止 JS 读取，`SameSite=Lax` 兼顾 CSRF 防护与站内可用。
const REFRESH_COOKIE: &str = "flame_refresh";

/// 请求是否经 HTTPS 到达（Doc/33 E-2）：本机 nginx 反代写入 `X-Forwarded-Proto`。
///
/// 该头可被直连客户端伪造，但伪造的唯一后果是给**自己的** Cookie 加 `Secure`，
/// 不构成安全问题；反之不加则 HTTPS 部署下 refresh token 可能经明文 HTTP 回传。
pub(crate) fn request_is_https(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(',')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("https")
        })
        .unwrap_or(false)
}

fn secure_attr(secure: bool) -> &'static str {
    if secure {
        "; Secure"
    } else {
        ""
    }
}

/// 构造设置 refresh token 的 Set-Cookie 响应头值（Max-Age 与 refresh token 寿命一致）。
/// HTTPS 访问时追加 `Secure`，禁止浏览器经明文 HTTP 回传。
pub(crate) fn set_refresh_cookie(token: &str, ttl_secs: u64, secure: bool) -> String {
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        REFRESH_COOKIE,
        token,
        ttl_secs,
        secure_attr(secure)
    )
}

/// W4-a：会话时长取面板设置 `session_timeout_minutes`（refresh token 寿命；
/// 活跃会话在剩余寿命不足一半时滑动续期，空闲超过该时长需重新登录）。
fn session_ttl_secs(state: &AppState) -> u64 {
    state.settings_service.policy().session_timeout_minutes * 60
}

/// 登录失败计数：阈值与锁定时长取面板设置。
async fn record_login_failure(state: &AppState, username: &str, ip: Option<&str>) {
    let policy = state.settings_service.policy();
    state
        .login_attempts
        .record_failure_with(
            username,
            ip.unwrap_or("unknown"),
            policy.login_max_failures,
            std::time::Duration::from_secs(policy.login_lockout_minutes * 60),
        )
        .await;
}

/// 构造清除 refresh token 的 Set-Cookie 响应头值（登出/失效时）。
fn clear_refresh_cookie(secure: bool) -> String {
    format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        REFRESH_COOKIE,
        secure_attr(secure)
    )
}

/// 从请求 Cookie 头中解析 refresh token（Y1）。
fn extract_refresh_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|part| {
                let part = part.trim();
                let (name, value) = part.split_once('=')?;
                if name.trim() == REFRESH_COOKIE && !value.is_empty() {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// Access Token（短过期，默认 15 分钟）
    pub token: String,
    /// Refresh Token（长过期，默认 24 小时；用于换取新 Access Token）
    pub refresh_token: String,
    pub username: String,
    pub role: String,
    /// 是否需强制修改密码（新装面板首次登录）
    pub must_change_password: bool,
    /// 密码已通过但需第二因子：此时 `token` 为空，凭 `challenge` 调用 `POST /api/auth/login/2fa`
    #[serde(default)]
    pub two_factor_required: bool,
    /// 两步验证挑战令牌（5 分钟有效；仅 `two_factor_required=true` 时存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
    /// 面板强制两步验证而该用户尚未绑定：登录后须先完成绑定
    #[serde(default)]
    pub must_setup_two_factor: bool,
}

/// 登录第二步请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginTwoFactorRequest {
    /// 第一步返回的挑战令牌
    pub challenge: String,
    /// 6 位 TOTP 验证码或一次性恢复码
    pub code: String,
}

/// 登录
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "凭据错误"),
        (status = 429, description = "尝试过于频繁"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    ApiJson(req): ApiJson<LoginRequest>,
) -> Result<(axum::http::HeaderMap, Json<LoginResponse>), AppError> {
    // 登录失败锁定检查（M3：按「用户名+IP」组合维度；IP 优先 TCP 对端）
    let ip = crate::api::extract::extract_client_ip_trusted(Some(&peer), &headers);
    let ip_opt = if ip == "unknown" { None } else { Some(ip) };
    state
        .login_attempts
        .check_locked(&req.username, ip_opt.as_deref().unwrap_or("unknown"))
        .await?;

    // M1：用户不存在与密码错误统一返回 `Invalid credentials`，消除用户枚举；
    // 并执行一次 dummy bcrypt 校验，抹平时序差异。
    let user = state.user_service.find_by_username(&req.username).await?;
    let user = match user {
        Some(u) => u,
        None => {
            // dummy 校验（成本与真实 bcrypt 一致），再统一报错
            let _ = tokio::task::spawn_blocking(move || {
                PasswordUtils::verify(&req.password, DUMMY_HASH)
            })
            .await;
            record_login_failure(&state, &req.username, ip_opt.as_deref()).await;
            let _ = state
                .operation_log_service
                .log(&req.username, "LOGIN_FAILED", None, ip_opt.as_deref())
                .await;
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }
    };

    // M2：bcrypt 校验移入 spawn_blocking，避免 CPU 密集操作阻塞 tokio worker。
    let password_hash = user.password_hash.clone();
    let pw_ok =
        tokio::task::spawn_blocking(move || PasswordUtils::verify(&req.password, &password_hash))
            .await
            .map_err(|e| AppError::internal(format!("bcrypt task panicked: {}", e)))??;
    if !pw_ok {
        record_login_failure(&state, &req.username, ip_opt.as_deref()).await;
        // 登录失败审计
        let _ = state
            .operation_log_service
            .log(&req.username, "LOGIN_FAILED", None, ip_opt.as_deref())
            .await;
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Doc/33 TOTP：已启用两步验证的用户只拿到挑战令牌；登录失败计数在第二步通过后才清零
    if state.two_factor_service.is_enabled(user.id).await? {
        let challenge = state.shared_jwt().sign_mfa_challenge(user.id)?;
        let _ = state
            .operation_log_service
            .log(
                &user.username,
                "LOGIN_2FA_CHALLENGE",
                None,
                ip_opt.as_deref(),
            )
            .await;
        return Ok((
            axum::http::HeaderMap::new(),
            Json(LoginResponse {
                token: String::new(),
                refresh_token: String::new(),
                username: user.username,
                role: user.role,
                must_change_password: user.must_change_password,
                two_factor_required: true,
                challenge: Some(challenge),
                must_setup_two_factor: false,
            }),
        ));
    }
    state
        .login_attempts
        .reset(&req.username, ip_opt.as_deref().unwrap_or("unknown"))
        .await;
    finish_login(&state, user, &headers, ip_opt.as_deref()).await
}

/// 登录成功收尾（签发令牌、审计、事件、refresh Cookie）——密码登录与两步验证共用。
async fn finish_login(
    state: &AppState,
    user: crate::domain::entity::User,
    headers: &axum::http::HeaderMap,
    ip: Option<&str>,
) -> Result<(axum::http::HeaderMap, Json<LoginResponse>), AppError> {
    // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
    let jwt = state.shared_jwt();
    let token = jwt.sign_access(user.id)?;
    let ttl = session_ttl_secs(state);
    let refresh_token = jwt.sign_refresh_with_ttl(user.id, ttl)?;

    // 登录成功审计
    let _ = state
        .operation_log_service
        .log(&user.username, "LOGIN_SUCCESS", None, ip)
        .await;

    // 登录成功事件
    let _ = state
        .event_bus
        .publish(crate::domain::entity::DomainEvent::UserLoggedIn {
            username: user.username.clone(),
        })
        .await;

    // Y1：refresh token 仅通过 httpOnly Cookie 下发，不写入 JSON 响应体，XSS 无法读取。
    let mut resp_headers = axum::http::HeaderMap::new();
    resp_headers.insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&set_refresh_cookie(
            &refresh_token,
            ttl,
            request_is_https(headers),
        ))
        .map_err(|_| AppError::internal("invalid refresh cookie"))?,
    );
    let must_setup_two_factor = state.settings_service.policy().require_two_factor
        && !state.two_factor_service.is_enabled(user.id).await?;
    Ok((
        resp_headers,
        Json(LoginResponse {
            token,
            refresh_token: String::new(),
            username: user.username,
            role: user.role,
            must_change_password: user.must_change_password,
            two_factor_required: false,
            challenge: None,
            must_setup_two_factor,
        }),
    ))
}

/// 登录第二步：提交挑战令牌 + TOTP 验证码（或一次性恢复码）换取会话令牌。
///
/// 失败计入与密码登录相同的「用户名 + IP」锁定计数；挑战令牌签发后若用户改过密码则失效。
#[utoipa::path(
    post,
    path = "/api/auth/login/2fa",
    tag = "auth",
    request_body = LoginTwoFactorRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "挑战令牌无效 / 过期，或验证码错误"),
        (status = 429, description = "尝试过于频繁"),
    )
)]
pub async fn login_two_factor(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    ApiJson(req): ApiJson<LoginTwoFactorRequest>,
) -> Result<(axum::http::HeaderMap, Json<LoginResponse>), AppError> {
    let claims = state
        .shared_jwt()
        .verify_mfa_challenge(req.challenge.trim())
        .map_err(|_| AppError::Unauthorized("Invalid or expired two-factor challenge".into()))?;
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid two-factor challenge".into()))?;
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid two-factor challenge".into()))?;
    if let Some(changed_at) = state
        .user_repo_for_revocation
        .get_password_changed_at(user_id)
        .await?
    {
        if (claims.iat as i64) < changed_at {
            return Err(AppError::Unauthorized(
                "Two-factor challenge invalidated by password change".into(),
            ));
        }
    }

    let ip = crate::api::extract::extract_client_ip_trusted(Some(&peer), &headers);
    let ip_opt = if ip == "unknown" { None } else { Some(ip) };
    let ip_key = ip_opt.as_deref().unwrap_or("unknown");
    state
        .login_attempts
        .check_locked(&user.username, ip_key)
        .await?;

    match state
        .two_factor_service
        .verify_second_factor(user.id, &req.code)
        .await?
    {
        Some(factor) => {
            state.login_attempts.reset(&user.username, ip_key).await;
            if factor == crate::application::two_factor_service::SecondFactor::RecoveryCode {
                let _ = state
                    .operation_log_service
                    .log(
                        &user.username,
                        "LOGIN_2FA_RECOVERY_CODE_USED",
                        None,
                        ip_opt.as_deref(),
                    )
                    .await;
            }
            finish_login(&state, user, &headers, ip_opt.as_deref()).await
        }
        None => {
            record_login_failure(&state, &user.username, ip_opt.as_deref()).await;
            let _ = state
                .operation_log_service
                .log(&user.username, "LOGIN_2FA_FAILED", None, ip_opt.as_deref())
                .await;
            Err(AppError::Unauthorized("Invalid verification code".into()))
        }
    }
}

/// 刷新令牌：必须使用 Refresh Token（Cookie 或 Bearer），换取新的 Access Token + 轮换 Refresh Token
/// 剩余寿命不足一半时重新签发 Refresh Token（滑动过期）；否则复用原 Refresh Token
/// 刷新令牌（Refresh Token 换新 Access Token + 轮换 Refresh Token）
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "刷新成功", body = LoginResponse),
        (status = 401, description = "令牌无效/过期"),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<(axum::http::HeaderMap, Json<LoginResponse>), AppError> {
    // Y1：优先从 httpOnly Cookie 读取 refresh token；兼容历史 Bearer（旧前端）
    let token = extract_refresh_token(&headers)
        .or_else(|| {
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .map(|s| s.to_string())
        })
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token".to_string()))?;

    // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
    let jwt = state.shared_jwt();
    // 仅接受 Refresh Token（Access Token 不可用于刷新）
    let claims = jwt.verify_refresh(&token)?;
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    // M4/F-04: refresh token 吊销校验——logout 后黑名单中的 token 直接失效（持久化到 SQLite）。
    let token_hash = crate::utils::constant_time_eq_hash(&token);
    if state.revoked_token_repo.contains(&token_hash).await? {
        return Err(AppError::Unauthorized("Refresh token revoked".to_string()));
    }
    // M4/F-04: 改密/删除后吊销——token 签发时间若早于该用户最近一次改密，则失效（持久化到 users 表）。
    if let Some(changed_at) = state
        .user_repo_for_revocation
        .get_password_changed_at(user_id)
        .await?
    {
        if (claims.iat as i64) < changed_at {
            return Err(AppError::Unauthorized(
                "Refresh token invalidated by password change".to_string(),
            ));
        }
    }

    // 剩余寿命判定：<一半则轮换 Refresh Token（滑动过期）
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
        .as_secs() as usize;
    // W4-a：会话时长可配置——剩余寿命不足一半（滑动续期）或超过当前设置（管理员调短了会话）
    // 时按新时长重签，调短立即对活跃会话生效。
    let ttl = session_ttl_secs(&state);
    let remaining = claims.exp.saturating_sub(now) as u64;
    let rotate = remaining < ttl / 2 || remaining > ttl;
    let new_refresh = if rotate {
        jwt.sign_refresh_with_ttl(user.id, ttl)?
    } else {
        token.to_string()
    };
    let cookie_ttl = if rotate { ttl } else { remaining };
    // F-18: Refresh token 轮换时拉黑旧 token——旧 token 在自然过期前不再可用，消除 12h 重放窗口。
    let token_was_rotated = new_refresh != token;
    if token_was_rotated {
        let old_hash = crate::utils::constant_time_eq_hash(&token);
        state
            .revoked_token_repo
            .insert(&old_hash, claims.exp as i64)
            .await?;
    }
    let new_access = jwt.sign_access(user.id)?;

    // Y1：刷新成功同样仅经 httpOnly Cookie 下发新 refresh token，不写入 JSON 响应体。
    let mut resp_headers = axum::http::HeaderMap::new();
    resp_headers.insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&set_refresh_cookie(
            &new_refresh,
            cookie_ttl,
            request_is_https(&headers),
        ))
        .map_err(|_| AppError::internal("invalid refresh cookie"))?,
    );
    let must_setup_two_factor = state.settings_service.policy().require_two_factor
        && !state.two_factor_service.is_enabled(user.id).await?;
    Ok((
        resp_headers,
        Json(LoginResponse {
            token: new_access,
            refresh_token: String::new(),
            username: user.username,
            role: user.role,
            must_change_password: user.must_change_password,
            two_factor_required: false,
            challenge: None,
            must_setup_two_factor,
        }),
    ))
}

/// 修改密码请求（`POST /api/auth/change-password`）
#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)] // 仅用于 OpenAPI 文档：handler 以 JSON 值解析，字段语义与此一致
pub struct ChangePasswordBody {
    pub old_password: String,
    /// 需满足密码强度策略，且不得与用户名相同
    pub new_password: String,
}

/// 轮换 JWT 密钥请求（`POST /api/auth/rotate-secret`）
#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)] // 仅用于 OpenAPI 文档
pub struct RotateSecretBody {
    /// 新密钥（不少于 32 字节）
    pub secret: String,
}

/// 当前登录用户
#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub id: i64,
    pub username: String,
    pub role: String,
    /// 是否需强制修改密码
    pub must_change_password: bool,
}

/// 修改当前用户密码
#[utoipa::path(
    post,
    path = "/api/auth/change-password",
    tag = "auth",
    request_body = ChangePasswordBody,
    responses(
        (status = 200, description = "修改成功；该用户其他会话的 refresh token 失效"),
        (status = 400, description = "旧密码错误或新密码不满足强度策略"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn change_password(
    State(state): State<AppState>,
    Extension(uid): Extension<UserId>,
    ApiJson(req): ApiJson<serde_json::Value>,
) -> Result<Json<()>, AppError> {
    let user = state
        .user_service
        .find_by_id(uid.0)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let old_pw = req
        .get("old_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing old_password".into()))?;
    // M2：bcrypt verify 移入 spawn_blocking
    let old_pw = old_pw.to_string();
    let stored_hash = user.password_hash.clone();
    let pw_ok = tokio::task::spawn_blocking(move || PasswordUtils::verify(&old_pw, &stored_hash))
        .await
        .map_err(|e| AppError::internal(format!("bcrypt task panicked: {}", e)))??;
    if !pw_ok {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    let new_pw = req
        .get("new_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing new_password".into()))?;
    // F-22: 密码强度校验（含 bcrypt 72 字节截断保护）
    PasswordUtils::validate_strength(new_pw, Some(&user.username))?;
    // M2：bcrypt hash 移入 spawn_blocking
    let new_pw = new_pw.to_string();
    let new_hash = tokio::task::spawn_blocking(move || PasswordUtils::hash(&new_pw))
        .await
        .map_err(|e| AppError::internal(format!("bcrypt task panicked: {}", e)))??;
    state
        .user_service
        .update_password(user.id, &new_hash)
        .await?;

    // M4/F-04: 记录改密时间（持久化到 users 表），使该用户此前签发的 refresh token 全部失效。
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(format!("Time error: {}", e)))?
        .as_secs() as i64;
    state
        .user_repo_for_revocation
        .update_password_changed_at(user.id, now)
        .await?;

    // 密码修改事件
    let _ = state
        .event_bus
        .publish(crate::domain::entity::DomainEvent::PasswordChanged {
            username: user.username.clone(),
        })
        .await;

    Ok(Json(()))
}

/// 轮换 JWT 签名密钥（admin only）：更新后旧 access token 立即失效（短宽限期由客户端刷新换取新 token）
#[utoipa::path(
    post,
    path = "/api/auth/rotate-secret",
    tag = "auth",
    request_body = RotateSecretBody,
    responses(
        (status = 200, description = "轮换成功，旧 access token 立即失效"),
        (status = 400, description = "密钥过短"),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn rotate_secret(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_secret = req
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing new secret".into()))?;
    if new_secret.len() < crate::utils::jwt::MIN_SECRET_BYTES {
        return Err(AppError::BadRequest(format!(
            "new secret must be at least {} bytes",
            crate::utils::jwt::MIN_SECRET_BYTES
        )));
    }
    let mut guard = state
        .jwt_secret_store
        .write()
        .map_err(|_| AppError::internal("jwt secret store poisoned"))?;
    *guard = new_secret.to_string();
    drop(guard);
    {
        let mut jwt_guard = state
            .jwt_utils
            .write()
            .map_err(|_| AppError::internal("jwt utils store poisoned"))?;
        *jwt_guard = Arc::new(crate::utils::jwt::JwtUtils::new_pair(new_secret));
    }
    // F-03: 将新密钥持久化到 `data/jwt_secret.key` 文件
    let _ = crate::utils::persist_rotated_jwt_secret(new_secret);
    // W4-a：不再把密钥冗余写进 settings 表——`GET /api/settings` 对持有 `settings:read`
    // 的账号可见，冗余副本等于把签名密钥交给只读用户（可伪造任意身份的令牌）。
    // 持久化只落 `data/jwt_secret.key`（0600）；旧库残留由幂等迁移清除。
    tracing::warn!("JWT secret rotated by admin (old tokens invalidated)");
    Ok(Json(serde_json::json!({
        "message": "JWT secret rotated; old access tokens are invalidated",
        "grace_period": "clients should re-login or use refresh token"
    })))
}

/// 退出登录：吊销当前 refresh token（加入黑名单），使其无法再换取新 access token。
/// M4：logout 端点此前被中间件引用但路由缺失（404）；现补全并实现吊销。
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "退出成功"),
        (status = 401, description = "令牌无效/缺失"),
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<(axum::http::HeaderMap, Json<serde_json::Value>), AppError> {
    // Y1：从 httpOnly Cookie 读取 refresh token（优先）；兼容历史 Bearer（旧前端）
    let token = extract_refresh_token(&headers).or_else(|| {
        headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    });

    // 仅接受合法 refresh token（无效/过期也返回 200，避免泄露有效性差异）
    let jwt = state.shared_jwt();
    if let Some(token) = token {
        if let Ok(claims) = jwt.verify_refresh(&token) {
            let token_hash = crate::utils::constant_time_eq_hash(&token);
            // F-04: 将 token 哈希持久化到黑名单表，带过期时间（取 claims.exp）
            state
                .revoked_token_repo
                .insert(&token_hash, claims.exp as i64)
                .await?;
        }
    }
    // Y1：无论是否有效均清除 Cookie，确保登出后客户端不再持有 refresh token。
    let mut resp_headers = axum::http::HeaderMap::new();
    resp_headers.insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&clear_refresh_cookie(request_is_https(&headers)))
            .map_err(|_| AppError::internal("invalid clear cookie"))?,
    );
    Ok((
        resp_headers,
        Json(serde_json::json!({ "message": "logged out" })),
    ))
}

/// 当前登录用户信息（前端刷新页面恢复身份）
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "当前用户", body = MeResponse),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn me(
    State(state): State<AppState>,
    Extension(uid): Extension<UserId>,
) -> Result<Json<MeResponse>, AppError> {
    let user = state
        .user_service
        .find_by_id(uid.0)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;
    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        role: user.role,
        must_change_password: user.must_change_password,
    }))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", axum::routing::post(login))
        .route("/api/auth/login/2fa", axum::routing::post(login_two_factor))
        .merge(two_factor::routes())
        .route("/api/auth/refresh", axum::routing::post(refresh))
        .route("/api/auth/me", axum::routing::get(me))
        .route(
            "/api/auth/change-password",
            axum::routing::post(change_password),
        )
        .route(
            "/api/auth/rotate-secret",
            axum::routing::post(rotate_secret),
        )
        .route("/api/auth/logout", axum::routing::post(logout))
}

#[cfg(test)]
mod cookie_tests {
    use super::*;

    #[test]
    fn refresh_cookie_secure_only_over_https() {
        let mut h = axum::http::HeaderMap::new();
        assert!(!request_is_https(&h));
        assert!(!set_refresh_cookie("t", 60, request_is_https(&h)).contains("Secure"));
        h.insert("x-forwarded-proto", "https".parse().unwrap());
        assert!(request_is_https(&h));
        let c = set_refresh_cookie("t", 60, request_is_https(&h));
        assert!(c.contains("HttpOnly") && c.contains("SameSite=Lax") && c.ends_with("; Secure"));
        assert!(clear_refresh_cookie(true).contains("Max-Age=0; Secure"));
        h.insert("x-forwarded-proto", "HTTPS, http".parse().unwrap());
        assert!(request_is_https(&h));
        h.insert("x-forwarded-proto", "http".parse().unwrap());
        assert!(!request_is_https(&h));
    }
}
