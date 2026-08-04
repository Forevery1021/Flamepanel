use crate::api::rate_limiter;
use crate::api::types::{route_permission, AppState, UserId};
use crate::core::error::AppError;
use crate::utils::jwt::JwtUtils;
use axum::{
    extract::State,
    http::{header, Request},
    middleware::{self, Next},
    response::Response,
    Router,
};
use tower_http::trace::TraceLayer;
use tracing::info;

/// 无需认证的白名单路径
fn is_public_path(path: &str) -> bool {
    path == "/health"
        || path == "/api/health"
        || path.starts_with("/ws/")
        || path == "/api/auth/login"
        || path.starts_with("/api/nodes/heartbeat/")
}

pub fn add_middleware(router: Router, state: AppState) -> Router {
    rate_limiter::init_global_limiter(120, 60);
    router
        .layer(middleware::from_fn(rate_limiter::rate_limit_middleware))
        .layer(middleware::from_fn_with_state(state, auth_middleware))
        .layer(TraceLayer::new_for_http())
}

/// 是否应审计该路径（跳过白名单与审计自身）
fn should_audit(path: &str) -> bool {
    !path.starts_with("/ws/")
        && path != "/health"
        && !path.starts_with("/api/nodes/heartbeat/")
        && !path.starts_with("/api/operation-logs")
        && !path.starts_with("/api/auth/refresh")
        && !path.starts_with("/api/auth/me")
        && !path.starts_with("/api/auth/login")
}

/// 审计写操作（POST/PUT/DELETE）：异步写入 operation_logs，不阻塞响应
/// 在认证中间件内调用，此时 username 已确定
fn audit_write(
    state: &AppState,
    method: &axum::http::Method,
    path: &str,
    username: &str,
    ip: Option<&str>,
) {
    let is_write = method == axum::http::Method::POST
        || method == axum::http::Method::PUT
        || method == axum::http::Method::DELETE;
    if !is_write || !should_audit(path) {
        return;
    }
    let state = state.clone();
    let action = format!("{} {}", method, path);
    let target = path.to_string();
    let username = username.to_string();
    let ip = ip.map(|s| s.to_string());
    tokio::spawn(async move {
        let _ = state
            .operation_log_service
            .log(&username, &action, Some(&target), ip.as_deref())
            .await;
    });
}

pub fn log_request(method: &str, uri: &str, status: u16) {
    info!(method = %method, uri = %uri, status = status, "HTTP request");
}

/// 认证 + RBAC 合并中间件：
/// - 一次 JWT 校验 + 一次用户查询完成身份认证
/// - 通过扩展注入 UserId 与用户角色，随后在同一处完成 RBAC 鉴权
/// - 未登录路径直接放行（白名单）
async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, AppError> {
    let path = req.uri().path();
    if is_public_path(path) {
        return Ok(next.run(req).await);
    }

    // 1. 解析并校验 JWT
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid Authorization scheme".to_string()))?;

    let jwt = JwtUtils::new(&state.jwt_secret, 24);
    let claims = jwt.verify(token)?;

    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid token subject".to_string()))?;

    // 2. 一次用户查询（认证 + RBAC 共用）
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists".to_string()))?;

    // 2.5 强制改密拦截：must_change_password=1 时仅放行白名单端点
    if user.must_change_password && !is_password_change_allowed(path) {
        return Err(AppError::PasswordChangeRequired(
            "Password change required before accessing this resource".to_string(),
        ));
    }

    // 3. RBAC 鉴权
    if let Some((resource, action)) = route_permission(req.method(), path) {
        let allowed = state
            .role_service
            .check_permission(&user.role, resource, action)
            .await?;
        if !allowed {
            return Err(AppError::Forbidden(format!(
                "Missing permission: {}:{}",
                resource, action
            )));
        }
    }

    // 4. 注入用户上下文
    let path_owned = path.to_string();
    req.extensions_mut().insert(UserId(user_id));

    // 5. 审计写操作（异步落库，不阻塞）
    let ip = req
        .headers()
        .get("X-Real-IP")
        .or_else(|| req.headers().get("X-Forwarded-For"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    audit_write(
        &state,
        req.method(),
        &path_owned,
        &user.username,
        ip.as_deref(),
    );

    Ok(next.run(req).await)
}

/// 强制改密状态下仍允许访问的端点
fn is_password_change_allowed(path: &str) -> bool {
    path == "/api/auth/change-password"
        || path == "/api/auth/refresh"
        || path == "/api/auth/me"
        || path == "/api/auth/logout"
}
