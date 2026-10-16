use crate::api::rate_limiter;
use crate::api::types::{route_permission, AppState, UserId, UserRole, Username};
use crate::core::error::AppError;
use crate::runtime::{request_id_middleware, RequestId};
use axum::{
    extract::{connect_info::MockConnectInfo, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    Router,
};
use tower_http::trace::TraceLayer;
use tracing::{info, Instrument};

/// 无需认证的白名单路径
fn is_public_path(path: &str) -> bool {
    path == "/health"
        || path == "/api/health"
        || path == "/api/openapi.json"
        || path == "/api/auth/login"
        || path == "/api/auth/login/2fa"
        || path == "/api/auth/refresh"
        || path == "/api/setup/status"
        || path == "/api/setup/initialize"
}

/// M-AI-1：MCP 协议端点（`fpk_` 凭证鉴权，不接受面板 JWT）
fn is_mcp_endpoint(path: &str) -> bool {
    path == "/api/mcp"
}

/// F-24: /metrics 路径（需要单独的 metrics_token 保护）
fn is_metrics_path(path: &str) -> bool {
    path == "/metrics"
}

/// A3.2：节点注册端点（免 JWT 但要求 bootstrap token）。
fn is_bootstrap_guarded_path(method: &axum::http::Method, path: &str) -> bool {
    method == axum::http::Method::POST && path == "/api/nodes/register"
}

/// S15：节点心跳端点（免 JWT，但要求 Agent token，集中在此守卫，与 register 一致管理）。
/// 命中形如 `POST /api/nodes/heartbeat/{id}` 的路径。
fn is_agent_heartbeat_guarded_path(method: &axum::http::Method, path: &str) -> bool {
    if method != axum::http::Method::POST || !path.starts_with("/api/nodes/heartbeat/") {
        return false;
    }
    // 剩余片段必须是单个数字节点 ID
    let id = &path["/api/nodes/heartbeat/".len()..];
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

/// S15：从心跳路径中提取节点 ID。
fn heartbeat_node_id(path: &str) -> Option<i64> {
    let id = path.strip_prefix("/api/nodes/heartbeat/")?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    id.parse().ok()
}

/// WebSocket 握手路径（需 token 校验，见 `auth_middleware`）
fn is_ws_path(path: &str) -> bool {
    path.starts_with("/ws/")
}

/// M-B 主线二 Stage A-7（Doc/20 §5.8）：项目 webhook 部署端点
/// （形如 `POST /api/projects/{id}/webhook`）。免 JWT，但要求路径携带部署令牌
/// `?token=`（或 `X-Webhook-Token` 头），守卫集中在此（与 heartbeat 同一管理模式）。
/// 令牌形如 hex uuid（不含 `-`），路径段 id 必须是数字。
fn is_project_webhook_guarded_path(method: &axum::http::Method, path: &str) -> bool {
    if method != axum::http::Method::POST || !path.starts_with("/api/projects/") {
        return false;
    }
    let rest = &path["/api/projects/".len()..];
    match rest.strip_suffix("/webhook") {
        Some(id) => !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()),
        None => false,
    }
}

/// 从 webhook 路径提取项目 ID。
fn webhook_project_id(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/api/projects/")?;
    let id = rest.strip_suffix("/webhook")?;
    id.parse().ok()
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
    // Doc/33 TOTP：两步验证自助管理（只作用于调用者本人）
    if is_two_factor_self_path(path) {
        return true;
    }
    // S-06：顶栏聚合徽标对所有登录用户可用；各分组在 handler 内按角色权限逐项裁剪
    // （无 `node:read` 等对应权限的分组返回 null），见 handler/overview。
    if method == axum::http::Method::GET && path == "/api/overview/badges" {
        return true;
    }
    false
}

/// Stage A-7：解析 URL query 中的 `token` 参数（webhook 免 JWT 守卫专用）。
fn parse_query_token(query: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == "token" {
            Some(v.to_string())
        } else {
            None
        }
    })
}

/// M5：自定义 trace span，剔除 URI 中的 query 参数（尤其 `/ws/` 的 `?token=`），
/// 避免 access token 落入访问日志/trace 日志。
///
/// W0：预留 `request_id` 字段——客户端已带 `x-request-id` 时直接填入，否则由内层
/// `request_id_middleware` 生成后回填，保证 tower_http 的请求完成事件（状态码 + 耗时）
/// 也能在内核日志中按请求编号检索。
fn make_span_with<B>(request: &axum::http::Request<B>) -> tracing::Span {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = tracing::field::Empty,
    );
    if let Some(rid) = request
        .headers()
        .get(crate::runtime::request_id::REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        span.record("request_id", rid);
    }
    span
}

pub fn add_middleware(router: Router, state: AppState) -> Router {
    // T6：限流阈值从 AppConfig 注入（经 AppState 传递），默认可调。
    rate_limiter::init_global_limiter(state.rate_limit_max, state.rate_limit_window_secs);
    router
        // `?fields=` 响应裁剪：最内层（鉴权之后、紧贴处理器），401 / 403 不受影响
        .layer(middleware::from_fn(
            crate::api::field_selection::field_selection_middleware,
        ))
        // T6：rate_limit 置于 auth 之外层，先限流再鉴权，保护昂贵的 JWT 验签与用户查询。
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        // W4-a：授权 IP / 绑定域名——在鉴权之前拒绝，未授权来源连登录接口都碰不到
        .layer(middleware::from_fn_with_state(
            state,
            crate::api::access_guard::access_guard,
        ))
        .layer(middleware::from_fn(rate_limiter::rate_limit_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(api_security_headers))
        .layer(TraceLayer::new_for_http().make_span_with(make_span_with))
}

/// 测试用：[`add_middleware`] + 固定对端地址 `127.0.0.1:0`（Doc/33 P-4）。
///
/// `tower::ServiceExt::oneshot` 直驱路由时没有 TCP 连接，`ConnectInfo<SocketAddr>` 提取器
/// 无值可取；`MockConnectInfo` 仅在真实 `ConnectInfo` 缺失时兜底。生产入口
/// （`lib.rs` 的 `into_make_service_with_connect_info`）总有真实对端，因此该层不再挂在
/// 生产中间件栈上——避免未来新增的服务入口（如 Unix socket）忘记注入对端时，
/// 访问控制 / 限流静默把所有请求当成 127.0.0.1。
#[doc(hidden)]
pub fn add_middleware_with_mock_peer(router: Router, state: AppState) -> Router {
    add_middleware(router, state).layer(MockConnectInfo(std::net::SocketAddr::from((
        [127, 0, 0, 1],
        0,
    ))))
}

/// Doc/33 E-1：API 响应的纵深防御头（仅在处理器未显式设置时补齐）。
///
/// - `X-Content-Type-Options: nosniff`：内核端口被直连（绕过 nginx）时同样防 MIME 嗅探；
/// - `Cache-Control: no-store`：`/api/*` 响应含会话/主机敏感数据，禁止浏览器与中间代理缓存。
///
/// 帧/Referrer/CSP 等页面级头仍由 nginx 统一下发，避免与反代重复叠加。
pub async fn api_security_headers(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let is_api = req.uri().path().starts_with("/api/");
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers
        .entry(axum::http::header::X_CONTENT_TYPE_OPTIONS)
        .or_insert(axum::http::HeaderValue::from_static("nosniff"));
    if is_api {
        headers
            .entry(axum::http::header::CACHE_CONTROL)
            .or_insert(axum::http::HeaderValue::from_static("no-store"));
    }
    res
}

/// F-25: 审计写入有界队列——替代每请求 tokio::spawn。
/// 有界（64 条）避免高写入 QPS 下任务无限堆积；后台批量消费，单事务 INSERT。
use std::sync::OnceLock;

/// 审计条目：待批量写入 operation_logs 表的数据。
struct AuditEntry {
    username: String,
    action: String,
    target: Option<String>,
    ip: Option<String>,
    request_id: String,
}

/// 有界审计队列（容量 256，满则丢弃最旧并告警，避免内存无界）。
static AUDIT_TX: OnceLock<tokio::sync::mpsc::Sender<AuditEntry>> = OnceLock::new();

/// 启动审计批量消费者（组合根调用一次）。
/// 每 200ms 或攒满 64 条触发一次批量 INSERT。
pub fn init_audit_consumer(
    operation_log_service: std::sync::Arc<crate::application::misc_service::OperationLogService>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<AuditEntry>(256);
    let _ = AUDIT_TX.set(tx);

    tokio::spawn(async move {
        let mut batch: Vec<AuditEntry> = Vec::with_capacity(64);
        let mut flush_interval = tokio::time::interval(std::time::Duration::from_millis(200));
        flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = flush_interval.tick() => {
                    if !batch.is_empty() {
                        let entries = std::mem::take(&mut batch);
                        flush_audit_batch(&operation_log_service, entries).await;
                    }
                }
                maybe_entry = rx.recv() => {
                    match maybe_entry {
                        Some(entry) => {
                            batch.push(entry);
                            if batch.len() >= 64 {
                                let entries = std::mem::take(&mut batch);
                                flush_audit_batch(&operation_log_service, entries).await;
                            }
                        }
                        None => {
                            // 通道关闭，最后一次 flush 后退出
                            if !batch.is_empty() {
                                let entries = std::mem::take(&mut batch);
                                flush_audit_batch(&operation_log_service, entries).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// 批量写入审计日志。
async fn flush_audit_batch(
    svc: &std::sync::Arc<crate::application::misc_service::OperationLogService>,
    entries: Vec<AuditEntry>,
) {
    for entry in entries {
        let span = tracing::info_span!("audit", request_id = %entry.request_id);
        let _ = svc
            .log(
                &entry.username,
                &entry.action,
                entry.target.as_deref(),
                entry.ip.as_deref(),
            )
            .instrument(span)
            .await;
    }
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
    _state: &AppState,
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
    // F-25: 改用有界队列批量写入，避免每请求 spawn 无限堆积。
    // 若通道未初始化（如测试环境未调用 init_audit_consumer），回退为直接 spawn。
    if let Some(tx) = AUDIT_TX.get() {
        let entry = AuditEntry {
            username: username.to_string(),
            action: format!("{} {}", method, path),
            target: Some(path.to_string()),
            ip: ip.map(|s| s.to_string()),
            request_id: request_id.to_string(),
        };
        // 有界发送：队列满时丢弃最旧（try_send 非阻塞），避免在认证热路径上阻塞等待
        let _ = tx.try_send(entry);
    } else {
        let state = _state.clone();
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
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<std::net::SocketAddr>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();
    // WS 握手（R2）：浏览器 WebSocket 无法携带自定义 Header，原先经 URL query `?token=`
    // 传 access token，会泄露到浏览器历史/代理/Web 日志。现改为「首帧鉴权」：握手阶段
    // 直接放行建立连接，token 在连接建立后的首帧消息中校验（见 `api/ws_auth`），
    // 校验失败由各 WS handler 关闭连接，不产生任何数据。
    if is_ws_path(path) {
        return Ok(next.run(req).await);
    }
    if is_public_path(path) {
        return Ok(next.run(req).await);
    }
    // F-24: /metrics 端点保护——配置了 metrics_token 时要求携带 `Authorization: Bearer <token>`
    if is_metrics_path(path) {
        if !state.metrics_token.is_empty() {
            let provided = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .unwrap_or("");
            let valid =
                crate::utils::constant_time_eq(state.metrics_token.as_bytes(), provided.as_bytes());
            if !valid {
                return Err(AppError::Unauthorized(
                    "Invalid or missing metrics token".to_string(),
                ));
            }
        }
        return Ok(next.run(req).await);
    }
    // A3.2：节点注册端点免 JWT，但必须携带与 OP_BOOTSTRAP_TOKEN 匹配的 X-Bootstrap-Token
    // （常量时间比较防时序侧信道；token 未配置视为拒绝）。
    if is_bootstrap_guarded_path(req.method(), path) {
        let provided = req
            .headers()
            .get("X-Bootstrap-Token")
            .and_then(|v| v.to_str().ok());
        let expected = state.bootstrap_token.as_str();
        let valid = !expected.is_empty()
            && provided
                .map(|p| crate::utils::constant_time_eq(expected.as_bytes(), p.as_bytes()))
                .unwrap_or(false);
        if !valid {
            return Err(AppError::Unauthorized(
                "Invalid or missing X-Bootstrap-Token".to_string(),
            ));
        }
        return Ok(next.run(req).await);
    }

    // Stage A-7：项目 webhook 部署端点免 JWT，但集中在此校验部署令牌
    // （URL query `?token=` 优先，兼容 `X-Webhook-Token` 头；常量时间比较防时序侧信道）。
    if is_project_webhook_guarded_path(req.method(), path) {
        let project_id = webhook_project_id(path)
            .ok_or_else(|| AppError::BadRequest("invalid webhook project id".into()))?;
        let provided = req
            .headers()
            .get("X-Webhook-Token")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .or_else(|| req.uri().query().and_then(parse_query_token));
        let valid = state
            .project_service
            .verify_webhook_token(project_id, provided.as_deref().unwrap_or(""))
            .await;
        match valid {
            Ok(_) => return Ok(next.run(req).await),
            Err(e) => return Err(e),
        }
    }

    // M-AI-1：MCP 协议端点——校验 `fpk_` 凭证并解析绑定用户，注入与 JWT 路径相同的用户上下文；
    // 工具级 RBAC 在协议层按用户**当前**角色逐个校验（凭证本身不携带权限）。
    if is_mcp_endpoint(path) {
        let token = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("Missing MCP credential".to_string()))?;
        let principal = state.mcp_service.authenticate(token.trim()).await?;
        let user = state
            .user_service
            .find_by_id(principal.user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Credential owner no longer exists".into()))?;
        if user.must_change_password {
            return Err(AppError::PasswordChangeRequired(
                "Credential owner must change password first".to_string(),
            ));
        }
        req.extensions_mut().insert(UserId(user.id));
        req.extensions_mut().insert(Username(user.username.clone()));
        req.extensions_mut().insert(UserRole(user.role.clone()));
        req.extensions_mut()
            .insert(crate::api::handler::mcp::McpCaller { principal });
        return Ok(next.run(req).await);
    }

    // S15：节点心跳端点免 JWT，但集中在此守卫 Agent token（与 register 的 bootstrap 守卫一致），
    // 不再把鉴权逻辑散落在 handler 内、白名单与实现分离。
    if is_agent_heartbeat_guarded_path(req.method(), path) {
        let node_id = heartbeat_node_id(path)
            .ok_or_else(|| AppError::BadRequest("invalid heartbeat node id".into()))?;
        let provided = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).to_string());
        let valid = state
            .node_service
            .verify_agent_token(node_id, provided.as_deref())
            .await?;
        if !valid {
            return Err(AppError::Unauthorized("Invalid agent token".into()));
        }
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

    // 2.6 强制两步验证：面板开启 `two_factor_enabled` 且该用户未绑定时，只放行绑定相关端点
    if state.settings_service.policy().require_two_factor
        && !is_two_factor_setup_allowed(path)
        && !state.two_factor_service.is_enabled(user.id).await?
    {
        return Err(AppError::TwoFactorSetupRequired(
            "Two-factor authentication must be set up before accessing this resource".to_string(),
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
            // 未在权限表声明权限、且不在 auth-only 白名单的路径一律拒绝。
            // 对「未知/未注册路由」返回 404（标准 HTTP 语义，不泄露路由存在性），
            // 对「已知但漏声明权限的路由」同样被拦截——与 is_auth_only_path 白名单搭配，
            // 避免新增路由漏鉴权时静默变成零鉴权端点；已知受保护路由漏声明权限则由
            // `permission_table_covers_all_routes` 一致性测试在 CI 中拦截。
            return Err(AppError::NotFound(
                "Route not found or permission not declared".to_string(),
            ));
        }
    }

    // 4. 注入用户上下文
    let path_owned = path.to_string();
    req.extensions_mut().insert(UserId(user_id));
    req.extensions_mut().insert(Username(user.username.clone()));
    req.extensions_mut().insert(UserRole(user.role.clone()));

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
    // M3：客户端 IP 优先 TCP 对端，回环代理才信任转发头。
    let ip = crate::api::extract::extract_client_ip_trusted(Some(&peer), req.headers());
    let ip = if ip == "unknown" { None } else { Some(ip) };
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
/// 两步验证自助端点（`/api/auth/2fa` 及其子路径）。
fn is_two_factor_self_path(path: &str) -> bool {
    path == "/api/auth/2fa" || path.starts_with("/api/auth/2fa/")
}

/// 强制两步验证期间仍放行的端点：绑定流程 + 会话维持 + 改密。
fn is_two_factor_setup_allowed(path: &str) -> bool {
    is_two_factor_self_path(path) || is_password_change_allowed(path)
}

fn is_password_change_allowed(path: &str) -> bool {
    path == "/api/auth/change-password"
        || path == "/api/auth/refresh"
        || path == "/api/auth/me"
        || path == "/api/auth/logout"
}
