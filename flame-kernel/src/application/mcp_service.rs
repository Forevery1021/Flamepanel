//! M-AI-1 内置 MCP Server 的凭证 / 会话 / 审计服务（Doc/30 §M-AI-1）。
//!
//! 协议解析与工具分发在 `api::handler::mcp`（需要 `AppState` 访问各业务服务）；
//! 本服务只负责「谁在调用、调用了什么」：
//! - 签发 / 校验 / 吊销 `fpk_` 凭证（明文只在签发时返回一次，库内存 SHA-256）；
//! - MCP 会话（`Mcp-Session-Id`）的建立、续期与终止；
//! - 工具调用审计落库与保留（最近 [`CALL_RETENTION`] 条）。
use crate::core::error::AppError;
use crate::domain::mcp::{
    AgentPrincipal, AgentSession, ApprovalFilter, ApprovalStatus, RiskLevel, ToolApproval,
    ToolCallFilter, ToolCallRecord, TOKEN_PREFIX,
};
use crate::domain::repository::McpRepository;
use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 调用审计保留条数。
pub const CALL_RETENTION: i64 = 10_000;
/// 每写入多少条调用记录触发一次保留清理。
const PRUNE_EVERY: u64 = 200;
/// `last_used_at` / `last_seen_at` 写入节流（秒），避免每次调用都写库。
const TOUCH_THROTTLE_SECS: i64 = 60;
/// 凭证名称最大长度（字符）。
const MAX_NAME_CHARS: usize = 64;
/// 过期天数上限（10 年）。
const MAX_EXPIRES_DAYS: i64 = 3650;
/// 审批单有效期（分钟）：超时未处理即过期，AI 需重新发起。
pub const APPROVAL_TTL_MINUTES: i64 = 30;
/// 每个凭证同时待审批的上限（防刷屏）。
pub const MAX_PENDING_PER_PRINCIPAL: i64 = 20;
/// 审批执行结果保存上限（字节）。
const APPROVAL_RESULT_MAX: usize = 8 * 1024;

/// 面板内置助手凭证的固定名称。
pub const ASSISTANT_PRINCIPAL_NAME: &str = "panel-assistant";

/// 签发结果：明文令牌仅此一次返回。
#[derive(Debug, Clone)]
pub struct IssuedPrincipal {
    pub principal: AgentPrincipal,
    pub token: String,
}

pub struct McpService {
    repo: Arc<dyn McpRepository>,
    calls_since_prune: AtomicU64,
}

impl McpService {
    pub fn new(repo: Arc<dyn McpRepository>) -> Self {
        Self {
            repo,
            calls_since_prune: AtomicU64::new(0),
        }
    }

    fn hash(token: &str) -> String {
        crate::utils::constant_time_eq_hash(token)
    }

    /// 生成明文令牌：`fpk_` + 64 位十六进制（两段 uuid v4，熵源 getrandom）。
    fn generate_token() -> String {
        format!(
            "{}{}{}",
            TOKEN_PREFIX,
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        )
    }

    /// 可展示前缀：`fpk_` + 8 位。
    fn display_prefix(token: &str) -> String {
        token.chars().take(TOKEN_PREFIX.len() + 8).collect()
    }

    /// 为 `user_id` 签发只读凭证。
    pub async fn issue(
        &self,
        user_id: i64,
        name: &str,
        expires_in_days: Option<i64>,
    ) -> Result<IssuedPrincipal, AppError> {
        self.issue_with(user_id, name, expires_in_days, false).await
    }

    /// 为 `user_id` 签发凭证；`allow_write` 为 true 时可发起写操作（仍需人工批准）。
    pub async fn issue_with(
        &self,
        user_id: i64,
        name: &str,
        expires_in_days: Option<i64>,
        allow_write: bool,
    ) -> Result<IssuedPrincipal, AppError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(AppError::BadRequest(format!(
                "凭证名称长度需为 1–{} 个字符",
                MAX_NAME_CHARS
            )));
        }
        if name.chars().any(|c| c.is_control()) {
            return Err(AppError::BadRequest("凭证名称不能包含控制字符".into()));
        }
        let now = Utc::now();
        let expires_at = match expires_in_days {
            None => None,
            Some(d) if (1..=MAX_EXPIRES_DAYS).contains(&d) => Some(now + Duration::days(d)),
            Some(_) => {
                return Err(AppError::BadRequest(format!(
                    "有效期需为 1–{} 天，或留空表示永不过期",
                    MAX_EXPIRES_DAYS
                )))
            }
        };
        let token = Self::generate_token();
        let mut principal = AgentPrincipal {
            id: 0,
            name: name.to_string(),
            user_id,
            token_prefix: Self::display_prefix(&token),
            token_hash: Self::hash(&token),
            created_at: now,
            last_used_at: None,
            expires_at,
            revoked_at: None,
            allow_write,
        };
        principal.id = self.repo.create_principal(&principal).await?;
        Ok(IssuedPrincipal { principal, token })
    }

    /// 校验 Bearer 令牌 → 有效凭证；任何失败统一返回 401（不区分不存在 / 吊销 / 过期，防枚举）。
    pub async fn authenticate(&self, token: &str) -> Result<AgentPrincipal, AppError> {
        let unauthorized = || AppError::Unauthorized("Invalid or expired MCP credential".into());
        if !token.starts_with(TOKEN_PREFIX) || token.len() > 256 {
            return Err(unauthorized());
        }
        let now = Utc::now();
        let principal = self
            .repo
            .find_principal_by_hash(&Self::hash(token))
            .await?
            .filter(|p| p.is_active(now))
            .ok_or_else(unauthorized)?;
        if Self::stale(principal.last_used_at, now) {
            self.repo.touch_principal(principal.id, now).await?;
        }
        Ok(principal)
    }

    fn stale(last: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
        last.map(|t| (now - t).num_seconds() >= TOUCH_THROTTLE_SECS)
            .unwrap_or(true)
    }

    pub async fn list_principals(&self) -> Result<Vec<AgentPrincipal>, AppError> {
        self.repo.list_principals().await
    }

    /// M-AI-3：用户在面板内使用 AI 助手时所用的**内置凭证**（每用户一个，按需签发）。
    ///
    /// 助手的写操作与外部 MCP 客户端一样只生成审批单，审批单挂在此凭证名下，
    /// 在「AI 接入中心」的凭证 / 审计 / 审批列表中可辨认。明文令牌签发后即丢弃——
    /// 该凭证无法被外部客户端使用。吊销后下次对话会重新签发（是否允许使用助手由
    /// `mcp:create` 权限决定，而不是凭证本身）。
    pub async fn assistant_principal(&self, user_id: i64) -> Result<AgentPrincipal, AppError> {
        let now = Utc::now();
        if let Some(p) = self.repo.list_principals().await?.into_iter().find(|p| {
            p.user_id == user_id
                && p.name == ASSISTANT_PRINCIPAL_NAME
                && p.allow_write
                && p.is_active(now)
        }) {
            return Ok(p);
        }
        Ok(self
            .issue_with(user_id, ASSISTANT_PRINCIPAL_NAME, None, true)
            .await?
            .principal)
    }

    pub async fn get_principal(&self, id: i64) -> Result<AgentPrincipal, AppError> {
        self.repo
            .find_principal(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("MCP credential {} not found", id)))
    }

    /// 吊销凭证（幂等），并关闭其所有未结束会话。
    pub async fn revoke(&self, id: i64) -> Result<AgentPrincipal, AppError> {
        self.get_principal(id).await?;
        let now = Utc::now();
        self.repo.revoke_principal(id, now).await?;
        for s in self.repo.list_sessions(Some(id), 1000).await? {
            if s.closed_at.is_none() {
                self.repo.close_session(&s.id, now).await?;
            }
        }
        self.get_principal(id).await
    }

    /// `initialize` 成功后建立会话。
    pub async fn open_session(
        &self,
        principal_id: i64,
        client_name: &str,
        client_version: &str,
        protocol_version: &str,
    ) -> Result<AgentSession, AppError> {
        let now = Utc::now();
        let clip = |s: &str| s.chars().take(128).collect::<String>();
        let session = AgentSession {
            id: uuid::Uuid::new_v4().to_string(),
            principal_id,
            client_name: clip(client_name),
            client_version: clip(client_version),
            protocol_version: clip(protocol_version),
            created_at: now,
            last_seen_at: now,
            closed_at: None,
        };
        self.repo.create_session(&session).await?;
        Ok(session)
    }

    /// 解析请求携带的 `Mcp-Session-Id`：不存在 / 已关闭 / 属于其他凭证一律 404
    /// （MCP 规范：客户端收到 404 后应重新 `initialize`）。
    pub async fn resolve_session(
        &self,
        id: &str,
        principal_id: i64,
    ) -> Result<AgentSession, AppError> {
        let not_found = || AppError::NotFound("MCP session not found or terminated".into());
        let session = self
            .repo
            .find_session(id)
            .await?
            .filter(|s| s.principal_id == principal_id && s.closed_at.is_none())
            .ok_or_else(not_found)?;
        let now = Utc::now();
        if Self::stale(Some(session.last_seen_at), now) {
            self.repo.touch_session(&session.id, now).await?;
        }
        Ok(session)
    }

    pub async fn close_session(&self, id: &str, principal_id: i64) -> Result<(), AppError> {
        let session = self.resolve_session(id, principal_id).await?;
        self.repo.close_session(&session.id, Utc::now()).await
    }

    pub async fn list_sessions(
        &self,
        principal_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<AgentSession>, AppError> {
        self.repo
            .list_sessions(principal_id, limit.clamp(1, 500))
            .await
    }

    /// 记录一次工具调用（审计失败只告警，不影响调用结果）。
    pub async fn record_call(&self, mut record: ToolCallRecord) {
        record.arguments = clip_bytes(&record.arguments, 2048);
        record.error = clip_bytes(&record.error, 1024);
        if let Err(e) = self.repo.record_call(&record).await {
            tracing::warn!(error = %e, tool = %record.tool, "MCP 调用审计写入失败");
            return;
        }
        let n = self.calls_since_prune.fetch_add(1, Ordering::Relaxed) + 1;
        if n.is_multiple_of(PRUNE_EVERY) {
            if let Err(e) = self.repo.prune_calls(CALL_RETENTION).await {
                tracing::warn!(error = %e, "MCP 调用审计保留清理失败");
            }
        }
    }

    /// 分页查询调用记录，返回 `(items, total)`。
    pub async fn list_calls(
        &self,
        filter: &ToolCallFilter,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<ToolCallRecord>, i64), AppError> {
        let page = page.max(1);
        let size = page_size.clamp(1, 200);
        let items = self
            .repo
            .list_calls(filter, size, (page - 1) * size)
            .await?;
        let total = self.repo.count_calls(filter).await?;
        Ok((items, total))
    }

    // ─── M-AI-2 审批 ─────────────────────────────────────────────────────

    /// AI 发起写操作 → 创建待审批单（参数与预览已由调用方校验 / 生成）。
    #[allow(clippy::too_many_arguments)]
    pub async fn request_approval(
        &self,
        principal_id: i64,
        session_id: Option<String>,
        tool: &str,
        risk: RiskLevel,
        raw_arguments: &str,
        redacted_arguments: &str,
        preview: &str,
    ) -> Result<ToolApproval, AppError> {
        if risk == RiskLevel::Read {
            return Err(AppError::BadRequest("只读工具不需要审批".into()));
        }
        let now = Utc::now();
        self.repo.expire_approvals(now).await?;
        let pending = self
            .repo
            .count_approvals(&ApprovalFilter {
                principal_id: Some(principal_id),
                status: Some(ApprovalStatus::Pending),
            })
            .await?;
        if pending >= MAX_PENDING_PER_PRINCIPAL {
            return Err(AppError::RateLimited(format!(
                "该凭证已有 {pending} 个待审批请求，请等待处理后再发起"
            )));
        }
        let mut a = ToolApproval {
            id: 0,
            principal_id,
            session_id,
            tool: tool.to_string(),
            risk,
            arguments: clip_bytes(redacted_arguments, 2048),
            raw_arguments: raw_arguments.to_string(),
            preview: clip_bytes(preview, 4096),
            status: ApprovalStatus::Pending,
            requested_at: now,
            expires_at: now + Duration::minutes(APPROVAL_TTL_MINUTES),
            decided_by: None,
            decided_at: None,
            reason: String::new(),
            result: String::new(),
            finished_at: None,
        };
        a.id = self.repo.create_approval(&a).await?;
        Ok(a)
    }

    /// 读取审批单（先把超时的待审批单标为过期）。
    pub async fn get_approval(&self, id: i64) -> Result<ToolApproval, AppError> {
        self.repo.expire_approvals(Utc::now()).await?;
        self.repo
            .find_approval(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("approval {} not found", id)))
    }

    pub async fn list_approvals(
        &self,
        filter: &ApprovalFilter,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<ToolApproval>, i64), AppError> {
        self.repo.expire_approvals(Utc::now()).await?;
        let page = page.max(1);
        let size = page_size.clamp(1, 200);
        let items = self
            .repo
            .list_approvals(filter, size, (page - 1) * size)
            .await?;
        let total = self.repo.count_approvals(filter).await?;
        Ok((items, total))
    }

    /// 批准：`pending` → `executing`（比较并设置，并发批准只有一个成功）。
    /// 返回最新审批单；调用方随后执行工具并调用 [`Self::finish_approval`]。
    pub async fn begin_approval(&self, id: i64, approver: i64) -> Result<ToolApproval, AppError> {
        let current = self.get_approval(id).await?;
        if current.status != ApprovalStatus::Pending {
            return Err(AppError::Conflict(format!(
                "审批单当前状态为 {}，不能批准",
                current.status.as_str()
            )));
        }
        let ok = self
            .repo
            .transition_approval(
                id,
                ApprovalStatus::Pending,
                ApprovalStatus::Executing,
                Some(approver),
                Utc::now(),
                "",
            )
            .await?;
        if !ok {
            return Err(AppError::Conflict("审批单已被他人处理".into()));
        }
        self.get_approval(id).await
    }

    /// 拒绝：`pending` → `rejected`。
    pub async fn reject_approval(
        &self,
        id: i64,
        approver: i64,
        reason: &str,
    ) -> Result<ToolApproval, AppError> {
        let current = self.get_approval(id).await?;
        let reason = clip_bytes(reason.trim(), 500);
        let ok = current.status == ApprovalStatus::Pending
            && self
                .repo
                .transition_approval(
                    id,
                    ApprovalStatus::Pending,
                    ApprovalStatus::Rejected,
                    Some(approver),
                    Utc::now(),
                    &reason,
                )
                .await?;
        if !ok {
            return Err(AppError::Conflict(format!(
                "审批单当前状态为 {}，不能拒绝",
                current.status.as_str()
            )));
        }
        self.get_approval(id).await
    }

    /// 执行结束：写入结果（成功输出或失败原因）。
    pub async fn finish_approval(
        &self,
        id: i64,
        outcome: &Result<String, String>,
    ) -> Result<ToolApproval, AppError> {
        let (status, text) = match outcome {
            Ok(t) => (ApprovalStatus::Succeeded, t.as_str()),
            Err(e) => (ApprovalStatus::Failed, e.as_str()),
        };
        self.repo
            .finish_approval(
                id,
                status,
                &clip_bytes(text, APPROVAL_RESULT_MAX),
                Utc::now(),
            )
            .await?;
        self.get_approval(id).await
    }
}

/// 按字节上限截断（落在 UTF-8 边界上），超长时追加省略号。
pub fn clip_bytes(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::InMemoryMcpRepository;

    fn svc() -> McpService {
        McpService::new(Arc::new(InMemoryMcpRepository::new()))
    }

    #[tokio::test]
    async fn issue_then_authenticate_roundtrip() {
        let s = svc();
        let issued = s.issue(7, "claude", None).await.unwrap();
        assert!(issued.token.starts_with("fpk_"));
        assert_eq!(issued.token.len(), 4 + 64);
        assert_eq!(issued.principal.token_prefix, &issued.token[..12]);
        assert_ne!(issued.principal.token_hash, issued.token);
        let p = s.authenticate(&issued.token).await.unwrap();
        assert_eq!(p.user_id, 7);
        // 首次使用写入 last_used_at
        assert!(s.get_principal(p.id).await.unwrap().last_used_at.is_some());
    }

    #[tokio::test]
    async fn authenticate_rejects_wrong_revoked_and_garbage() {
        let s = svc();
        let issued = s.issue(1, "a", Some(30)).await.unwrap();
        assert!(s.authenticate("fpk_nope").await.is_err());
        assert!(s.authenticate("eyJhbGciOi.jwt.token").await.is_err());
        assert!(s.authenticate(&"fpk_x".repeat(100)).await.is_err());
        s.revoke(issued.principal.id).await.unwrap();
        assert!(matches!(
            s.authenticate(&issued.token).await,
            Err(AppError::Unauthorized(_))
        ));
        // 幂等
        let again = s.revoke(issued.principal.id).await.unwrap();
        assert!(again.revoked_at.is_some());
    }

    #[tokio::test]
    async fn expired_principal_is_inactive() {
        let now = Utc::now();
        let mut p = AgentPrincipal {
            id: 1,
            name: "x".into(),
            user_id: 1,
            token_prefix: String::new(),
            token_hash: String::new(),
            created_at: now,
            last_used_at: None,
            expires_at: Some(now - Duration::seconds(1)),
            revoked_at: None,
            allow_write: false,
        };
        assert!(!p.is_active(now));
        p.expires_at = None;
        assert!(p.is_active(now));
    }

    #[tokio::test]
    async fn issue_validates_name_and_expiry() {
        let s = svc();
        assert!(s.issue(1, "  ", None).await.is_err());
        assert!(s.issue(1, &"x".repeat(65), None).await.is_err());
        assert!(s.issue(1, "a\nb", None).await.is_err());
        assert!(s.issue(1, "ok", Some(0)).await.is_err());
        assert!(s.issue(1, "ok", Some(3651)).await.is_err());
        let p = s.issue(1, " ok ", Some(1)).await.unwrap().principal;
        assert_eq!(p.name, "ok");
        assert!(p.expires_at.is_some());
    }

    #[tokio::test]
    async fn sessions_are_scoped_to_principal_and_close_on_revoke() {
        let s = svc();
        let a = s.issue(1, "a", None).await.unwrap().principal;
        let b = s.issue(1, "b", None).await.unwrap().principal;
        let sess = s
            .open_session(a.id, "cursor", "1.0", "2025-06-18")
            .await
            .unwrap();
        assert!(s.resolve_session(&sess.id, a.id).await.is_ok());
        assert!(matches!(
            s.resolve_session(&sess.id, b.id).await,
            Err(AppError::NotFound(_))
        ));
        assert!(s.resolve_session("missing", a.id).await.is_err());
        s.revoke(a.id).await.unwrap();
        assert!(s.resolve_session(&sess.id, a.id).await.is_err());
        let listed = s.list_sessions(Some(a.id), 10).await.unwrap();
        assert!(listed[0].closed_at.is_some());
    }

    #[tokio::test]
    async fn record_call_clips_and_lists_with_filter() {
        let s = svc();
        for i in 0..3 {
            s.record_call(ToolCallRecord {
                id: 0,
                principal_id: 1 + (i % 2),
                session_id: None,
                tool: "docker_list".into(),
                arguments: "x".repeat(5000),
                ok: true,
                error: String::new(),
                duration_ms: 1,
                created_at: Utc::now(),
            })
            .await;
        }
        let (items, total) = s
            .list_calls(
                &ToolCallFilter {
                    principal_id: Some(1),
                    ..Default::default()
                },
                1,
                50,
            )
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
        assert!(items[0].arguments.len() <= 2048 + '…'.len_utf8());
        assert!(items[0].id > items[1].id);
    }

    #[test]
    fn clip_bytes_respects_utf8_boundary() {
        assert_eq!(clip_bytes("abc", 5), "abc");
        let s = "中文字符";
        let c = clip_bytes(s, 4);
        assert_eq!(c, "中…");
    }

    #[tokio::test]
    async fn approval_lifecycle_and_guards() {
        let s = svc();
        let p = s.issue_with(1, "w", None, true).await.unwrap().principal;
        assert!(p.allow_write);
        assert!(!s.issue(1, "r", None).await.unwrap().principal.allow_write);
        // 只读工具不走审批
        assert!(s
            .request_approval(p.id, None, "docker_list", RiskLevel::Read, "{}", "{}", "")
            .await
            .is_err());
        let a = s
            .request_approval(
                p.id,
                Some("sess".into()),
                "docker_container_stop",
                RiskLevel::Write,
                r#"{"container":"web"}"#,
                r#"{"container":"web"}"#,
                "停止容器 web",
            )
            .await
            .unwrap();
        assert_eq!(a.status, ApprovalStatus::Pending);
        assert!(a.expires_at > a.requested_at);
        // 批准 → executing；重复批准 / 拒绝冲突
        let e = s.begin_approval(a.id, 7).await.unwrap();
        assert_eq!(e.status, ApprovalStatus::Executing);
        assert_eq!(e.decided_by, Some(7));
        assert!(matches!(
            s.begin_approval(a.id, 7).await,
            Err(AppError::Conflict(_))
        ));
        assert!(s.reject_approval(a.id, 7, "no").await.is_err());
        let big = "x".repeat(APPROVAL_RESULT_MAX * 2);
        let done = s.finish_approval(a.id, &Ok(big)).await.unwrap();
        assert_eq!(done.status, ApprovalStatus::Succeeded);
        assert!(done.result.len() <= APPROVAL_RESULT_MAX + 64);
        // 拒绝路径
        let b = s
            .request_approval(
                p.id,
                None,
                "website_deploy",
                RiskLevel::Write,
                "{}",
                "{}",
                "",
            )
            .await
            .unwrap();
        let r = s.reject_approval(b.id, 7, "  不需要  ").await.unwrap();
        assert_eq!(r.status, ApprovalStatus::Rejected);
        assert_eq!(r.reason, "不需要");
        assert!(matches!(
            s.get_approval(9999).await,
            Err(AppError::NotFound(_))
        ));
        let (items, total) = s
            .list_approvals(&ApprovalFilter::default(), 1, 10)
            .await
            .unwrap();
        assert_eq!((items.len(), total), (2, 2));
    }

    #[tokio::test]
    async fn pending_approvals_are_capped_per_principal() {
        let s = svc();
        let p = s.issue_with(1, "w", None, true).await.unwrap().principal;
        for _ in 0..MAX_PENDING_PER_PRINCIPAL {
            s.request_approval(p.id, None, "t", RiskLevel::Write, "{}", "{}", "")
                .await
                .unwrap();
        }
        assert!(matches!(
            s.request_approval(p.id, None, "t", RiskLevel::Write, "{}", "{}", "")
                .await,
            Err(AppError::RateLimited(_))
        ));
        // 其他凭证不受影响
        let q = s.issue_with(1, "w2", None, true).await.unwrap().principal;
        assert!(s
            .request_approval(q.id, None, "t", RiskLevel::Destructive, "{}", "{}", "")
            .await
            .is_ok());
    }
}
