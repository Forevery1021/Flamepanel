//! WebSocket 首帧鉴权（R2）
//!
//! 浏览器 WebSocket API 无法在握手时携带自定义 Header，原先通过 URL query `?token=` 传
//! access token，导致 token 泄露到浏览器历史、代理访问日志与 Web 服务器日志。
//!
//! R2 改为「首帧鉴权」：握手阶段先建立连接，客户端在连接建立后的第一帧发送
//! `{"type":"auth","token":"<access_token>"}`，由各 WS handler 调用本模块完成
//! JWT 校验 + RBAC 鉴权；校验失败则关闭连接，不产生任何数据。这样 token 不再
//! 出现在 URL 中，兼顾安全与浏览器兼容性。
//!
//! **会话控制**（Doc/02 §WS）：鉴权失败与会话失效以关闭码区分——`4001` 未认证 / 会话过期
//! （前端刷新令牌后重连），`4003` 无权限（前端停止重连）。连接期间每 60 s 复核一次，
//! 并接受带内续期帧，见 [`WsSession`]。

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes};

use crate::api::types::{AppState, Username};
use crate::core::error::AppError;

/// WS 路径到 RBAC 权限点映射（R3：WS 终端/日志/指标不再绕过 RBAC）。
/// 返回 (resource, action)；None 表示未知 WS 路径，将被拒绝。
pub fn ws_permission(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/ws/terminal" => Some(("terminal", "execute")),
        "/ws/logs" => Some(("log", "read")),
        "/ws/metrics" => Some(("node", "read")),
        _ => None,
    }
}

/// WS 关闭码（应用私有区 4000–4999）：会话未认证 / 已失效（令牌过期、吊销、改密、用户删除）。
/// 前端收到后先刷新 access token 再重连；刷新失败即回登录页。
pub const CLOSE_UNAUTHENTICATED: u16 = 4001;
/// WS 关闭码：已认证但无权访问该通道（角色被降级 / 权限被收回）。前端收到后停止重连。
pub const CLOSE_FORBIDDEN: u16 = 4003;

/// WS 会话复核结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsVerdict {
    Valid,
    Unauthenticated,
    Forbidden,
}

impl WsVerdict {
    fn of_error(e: &AppError) -> Self {
        match e {
            AppError::Forbidden(_) => WsVerdict::Forbidden,
            _ => WsVerdict::Unauthenticated,
        }
    }

    /// 需要关闭时的关闭码。
    pub fn close_code(self) -> Option<u16> {
        match self {
            WsVerdict::Valid => None,
            WsVerdict::Unauthenticated => Some(CLOSE_UNAUTHENTICATED),
            WsVerdict::Forbidden => Some(CLOSE_FORBIDDEN),
        }
    }
}

/// 带关闭码与原因的 Close 帧。
pub fn close_message(code: u16) -> Message {
    let reason = match code {
        CLOSE_UNAUTHENTICATED => "session expired",
        CLOSE_FORBIDDEN => "forbidden",
        _ => "",
    };
    Message::Close(Some(CloseFrame {
        code,
        reason: Utf8Bytes::from_static(reason),
    }))
}

/// 完整校验 access token：签名 / 过期 / 类型 + 用户存在 + 改密吊销 + 通道 RBAC。
/// 成功返回 `(user_id, Username)`。
pub async fn authenticate_ws_user(
    state: &AppState,
    token: &str,
    path: &str,
) -> Result<(i64, Username), AppError> {
    // Stage 7（JWT 加固）：复用共享 JwtUtils 实例，禁止每次请求 new
    let jwt = state.shared_jwt();
    let claims = jwt
        .verify_access(token)
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
    // 改密后此前签发的令牌失效（iat 早于 password_changed_at）
    if let Some(changed_at) = state
        .user_repo_for_revocation
        .get_password_changed_at(user_id)
        .await?
    {
        if (claims.iat as i64) < changed_at {
            return Err(AppError::Unauthorized(
                "Token invalidated by password change".to_string(),
            ));
        }
    }
    // R3：WS 通道同样执行 RBAC，不再绕过权限体系。
    match ws_permission(path) {
        Some((resource, action)) => {
            let allowed = state
                .role_service
                .check_permission(&user.role, resource, action)
                .await?;
            // admin 为超级管理员，即使 sqlite 未 seed 新权限也放行，避免锁死终端；
            // 其余角色必须显式具备对应权限，否则 403。
            if !allowed && user.role != "admin" {
                return Err(AppError::Forbidden(format!(
                    "Missing permission: {}:{}",
                    resource, action
                )));
            }
        }
        None => {
            return Err(AppError::Forbidden(
                "Unknown websocket endpoint".to_string(),
            ));
        }
    }
    Ok((user_id, Username(user.username)))
}

/// 校验 WS 首帧携带的 access token，完成 JWT 校验 + 用户存在性 + RBAC 鉴权。
/// 成功返回该连接的 Username（供审计使用）。
pub async fn authenticate_ws(
    state: &AppState,
    token: &str,
    path: &str,
) -> Result<Username, AppError> {
    authenticate_ws_user(state, token, path)
        .await
        .map(|(_, u)| u)
}

/// 已认证的 WS 会话：持有当前 access token，支持周期复核与带内续期，并能从任一任务「踢出」。
///
/// - **复核**（每 60 s）：令牌过期 / 吊销 / 改密 / 用户删除 → 4001；通道权限被收回 → 4003。
/// - **带内续期**：客户端随时可再发 `{"type":"auth","token":"<新 access token>"}`，
///   同一用户且校验通过即替换当前令牌——终端等长会话不必因 15 分钟的 access token 轮换而断开。
///   换成其他用户的令牌视为失效（4001）。
/// - **踢出**：接收任务发现续期失败时调用 [`WsSession::kill`]，发送任务在 [`WsSession::kill_rx`]
///   上收到关闭码后发出 Close 帧并退出（发送端只在一个任务里，避免并发写）。
pub struct WsSession {
    path: &'static str,
    user_id: i64,
    token: std::sync::Mutex<String>,
    kill_tx: tokio::sync::watch::Sender<u16>,
}

impl WsSession {
    pub fn new(path: &'static str, user_id: i64, token: String) -> std::sync::Arc<Self> {
        let (kill_tx, _) = tokio::sync::watch::channel(0);
        std::sync::Arc::new(Self {
            path,
            user_id,
            token: std::sync::Mutex::new(token),
            kill_tx,
        })
    }

    pub fn user_id(&self) -> i64 {
        self.user_id
    }

    fn current_token(&self) -> String {
        self.token.lock().map(|t| t.clone()).unwrap_or_default()
    }

    async fn verdict_for(&self, state: &AppState, token: &str) -> WsVerdict {
        match authenticate_ws_user(state, token, self.path).await {
            Ok((uid, _)) if uid == self.user_id => WsVerdict::Valid,
            Ok(_) => WsVerdict::Unauthenticated,
            Err(e) => WsVerdict::of_error(&e),
        }
    }

    /// 周期复核当前令牌。
    pub async fn recheck(&self, state: &AppState) -> WsVerdict {
        let token = self.current_token();
        self.verdict_for(state, &token).await
    }

    /// 带内续期：校验新令牌，通过则替换。
    pub async fn reauth(&self, state: &AppState, token: &str) -> WsVerdict {
        let v = self.verdict_for(state, token).await;
        if v == WsVerdict::Valid {
            if let Ok(mut t) = self.token.lock() {
                *t = token.to_string();
            }
        }
        v
    }

    /// 标记会话需以 `verdict` 对应的关闭码结束（`Valid` 时无操作）。
    pub fn kill(&self, verdict: WsVerdict) {
        if let Some(code) = verdict.close_code() {
            self.kill_tx.send_replace(code);
        }
    }

    /// 订阅踢出信号：值变为非 0 即关闭码。
    pub fn kill_rx(&self) -> tokio::sync::watch::Receiver<u16> {
        self.kill_tx.subscribe()
    }
}

/// 从 WS 首帧（或任意帧）中提取 access token。
/// 首帧约定格式：`{"type":"auth","token":"<access_token>"}`。
pub fn extract_auth_token(frame: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(frame).ok()?;
    if parsed["type"].as_str() != Some("auth") {
        return None;
    }
    parsed["token"].as_str().map(|s| s.to_string())
}
