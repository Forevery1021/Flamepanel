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
    path == "/health" || path.starts_with("/ws/") || path == "/api/auth/login"
}

pub fn add_middleware(router: Router, state: AppState) -> Router {
    rate_limiter::init_global_limiter(120, 60);
    router
        .layer(middleware::from_fn(rate_limiter::rate_limit_middleware))
        .layer(middleware::from_fn_with_state(state, auth_middleware))
        .layer(TraceLayer::new_for_http())
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
    req.extensions_mut().insert(UserId(user_id));

    Ok(next.run(req).await)
}
