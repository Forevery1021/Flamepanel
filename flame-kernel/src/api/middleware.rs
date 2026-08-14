use crate::api::rate_limiter;
use crate::api::types::{route_permission, AppState, UserId, Username};
use crate::core::error::AppError;
use crate::runtime::{request_id_middleware, RequestId};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    Router,
};
use tower_http::trace::TraceLayer;
use tracing::{info, Instrument};

/// 轻量 percent-decode（仅处理 `%XX`；token 通常为 URL-safe base64，无需其他解码）
fn percent_decode(input: String) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 无需认证的白名单路径
fn is_public_path(path: &str) -> bool {
    path == "/health"
        || path == "/api/health"
        || path == "/api/openapi.json"
        || path == "/metrics"
        || path == "/api/auth/login"
        || path == "/api/auth/refresh"
        || path == "/api/nodes/register"
        || path.starts_with("/api/nodes/heartbeat/")
}

/// WebSocket 握手路径（需 token 校验，见 `auth_middleware`）
fn is_ws_path(path: &str) -> bool {
    path.starts_with("/ws/")
}

/// 已认证但无需资源级 RBAC 的受保护路径（auth-only）。
/// Stage 5（A2）安全基线：未在 `ROUTE_PERMISSIONS` 声明权限、且不在本白名单内的
/// 受保护路径默认 **403**（拒绝比放行更安全），防止新增路由漏鉴权。
/// 本白名单仅收录「需要登录但语义上无资源归属」的端点，与 `ROUTE_PERMISSIONS` 表
/// 保持一一对应（见 `types.rs` 中的 `permission_table_covers_all_routes` 一致性测试）。
fn is_auth_only_path(method: &axum::http::Method, path: &str) -> bool {
    // 认证自服务端点：仅需身份校验
    if path == "/api/auth/me" || path == "/api/auth/change-password" || path == "/api/auth/logout" {
        return true;
    }
    // 历史语义：数据库单实例读取 / 子库列举仅鉴权，不做资源级 RBAC
    if method == axum::http::Method::GET && path.starts_with("/api/databases/") {
        let rest = &path["/api/databases/".len()..];
        // `/api/databases/{id}` 或 `/api/databases/{id}/databases`
        if !rest.contains('/') || rest.ends_with("/databases") {
            return true;
        }
    }
    false
}

pub fn add_middleware(router: Router, state: AppState) -> Router {
    // T6：限流阈值从 AppConfig 注入（经 AppState 传递），默认可调。
    rate_limiter::init_global_limiter(state.rate_limit_max, state.rate_limit_window_secs);
    router
        // T6：rate_limit 置于 auth 之外层，先限流再鉴权，保护昂贵的 JWT 验签与用户查询。
        .layer(middleware::from_fn_with_state(state, auth_middleware))
        .layer(middleware::from_fn(rate_limiter::rate_limit_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(TraceLayer::new_for_http())
}

/// 是否应审计该路径（跳过白名单与审计自身）
fn should_audit(path: &str) -> bool {
    !path.starts_with("/ws/")
        && path != "/health"
        && path != "/api/nodes/register"
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
    request_id: &str,
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
    let request_id = request_id.to_string();
    tokio::spawn(async move {
        let span = tracing::info_span!("audit", request_id = %request_id);
        let _ = state
            .operation_log_service
            .log(&username, &action, Some(&target), ip.as_deref())
            .instrument(span)
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
async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();
    // WS 握手：从 query `token` 校验 access token（与 REST Bearer 同一密钥/类型语义）
    if is_ws_path(path) {
        let token = req
            .uri()
            .query()
            .and_then(|q| {
                q.split('&')
                    .find_map(|kv| kv.strip_prefix("token="))
                    .map(|t| t.to_string())
            })
            .ok_or_else(|| AppError::Unauthorized("Missing token query parameter".to_string()))?;
        let token = percent_decode(token);
        // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
        let jwt = state.shared_jwt();
        let claims = jwt
            .verify_access(&token)
            .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;
        let user_id: i64 = claims
            .sub
            .parse()
            .map_err(|_| AppError::Unauthorized("Invalid token subject".to_string()))?;
        // 用户仍须存在（与 REST 认证一致）
        let user = state
            .user_service
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("User no longer exists".to_string()))?;
        // WS 仅注入用户上下文（RBAC 由具体连接功能决定，此处不做资源级鉴权）
        req.extensions_mut().insert(UserId(user_id));
        req.extensions_mut().insert(Username(user.username.clone()));
        let request_id = req
            .extensions()
            .get::<RequestId>()
            .map(|r| r.0.clone())
            .unwrap_or_else(crate::runtime::request_id::generate_request_id);
        let username_owned = user.username.clone();
        let auth_span = tracing::info_span!(
            "ws-auth",
            request_id = %request_id,
            user_id = user_id,
            username = %username_owned,
        );
        return Ok(next.run(req).instrument(auth_span).await);
    }
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

    // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
    let jwt = state.shared_jwt();
    let claims = jwt.verify_access(token)?;

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
    match route_permission(req.method(), path) {
        Some((resource, action)) => {
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
        None if is_auth_only_path(req.method(), path) => {
            // 仅鉴权路径：需登录但无需资源级 RBAC，放行
        }
        None => {
            // 未在权限表声明的路径：既可能是“已注册但漏声明”的受保护路由，也可能是
            // 根本不存在的未知路由。此处不做硬性 403，而是放行到路由器——由路由器的
            // fallback 对未知路径返回 404；已注册路由若漏声明权限，则由
            // `permission_table_covers_all_routes` 一致性测试在 CI 中拦截，避免请求期误伤
            // 未知路径（未知路由应 404 而非 403）。
        }
    }

    // 4. 注入用户上下文
    let path_owned = path.to_string();
    req.extensions_mut().insert(UserId(user_id));
    req.extensions_mut().insert(Username(user.username.clone()));

    // 4.5 用户上下文字段：与 request_id 同链，日志可按 user 维度过滤
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_else(crate::runtime::request_id::generate_request_id);
    let username_owned = user.username.clone();
    let auth_span = tracing::info_span!(
        "auth",
        request_id = %request_id,
        user_id = user_id,
        username = %username_owned,
    );

    // 5. 审计写操作（异步落库，不阻塞；继承 request_id 以便审计链路串联）
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
        &request_id,
    );

    Ok(next.run(req).instrument(auth_span).await)
}

/// 强制改密状态下仍允许访问的端点
fn is_password_change_allowed(path: &str) -> bool {
    path == "/api/auth/change-password"
        || path == "/api/auth/refresh"
        || path == "/api/auth/me"
        || path == "/api/auth/logout"
}
