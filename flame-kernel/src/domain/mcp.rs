//! M-AI-1 内置 MCP Server 领域模型（Doc/30 §M-AI-1 · Doc/32 §2）。
//!
//! - [`AgentPrincipal`]：AI 接入凭证（类 PAT）。明文令牌 `fpk_<prefix>_<secret>` 仅在签发时返回一次，
//!   库内只存 SHA-256 哈希与可展示前缀；凭证**绑定签发者用户**，工具调用时按该用户的
//!   **当前**角色做 RBAC——用户被删除 / 降权立即生效，凭证本身不携带任何权限。
//! - [`AgentSession`]：一次 MCP `initialize` 建立的会话（`Mcp-Session-Id`）。
//! - [`ToolCallRecord`]：每次 `tools/call` 的审计记录（参数已脱敏、截断）。
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 凭证明文前缀（便于密钥扫描器识别）。
pub const TOKEN_PREFIX: &str = "fpk_";

/// AI 接入凭证。`token_hash` 永不序列化到 API 响应。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentPrincipal {
    pub id: i64,
    /// 凭证名称（如 `claude-desktop@laptop`）
    pub name: String,
    /// 绑定用户（签发者）
    pub user_id: i64,
    /// 可展示前缀（`fpk_xxxxxxxx`），用于在列表中辨认凭证
    pub token_prefix: String,
    #[serde(skip_serializing, default)]
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    /// 过期时间（None = 永不过期）
    pub expires_at: Option<DateTime<Utc>>,
    /// 吊销时间（None = 有效）
    pub revoked_at: Option<DateTime<Utc>>,
    /// 是否允许发起写操作（M-AI-2）。缺省 false：凭证只能调用只读工具；
    /// 为 true 时写工具仍需面板人工批准才会执行。
    #[serde(default)]
    pub allow_write: bool,
}

impl AgentPrincipal {
    /// 当前是否可用（未吊销且未过期）。
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at.map(|e| e > now).unwrap_or(true)
    }
}

/// MCP 会话。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSession {
    /// `Mcp-Session-Id`（uuid v4）
    pub id: String,
    pub principal_id: i64,
    /// 客户端自报名称（`clientInfo.name`，如 `claude-ai` / `cursor`）
    pub client_name: String,
    pub client_version: String,
    /// 协商后的协议版本
    pub protocol_version: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// 工具调用审计记录。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolCallRecord {
    pub id: i64,
    pub principal_id: i64,
    pub session_id: Option<String>,
    pub tool: String,
    /// 调用参数（JSON 文本，已脱敏并截断到 2 KiB）
    pub arguments: String,
    pub ok: bool,
    /// 失败原因（成功时为空）
    pub error: String,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

/// 调用记录查询过滤。
#[derive(Debug, Clone, Default)]
pub struct ToolCallFilter {
    pub principal_id: Option<i64>,
    pub session_id: Option<String>,
    pub tool: Option<String>,
}

// ─── M-AI-2：变更闸门（confirm 分级 + 审批）──────────────────────────────

/// 工具风险分级（Doc/30 §M-AI-2）。
///
/// - `read`：只读，调用即执行；
/// - `write`：可逆变更（启停容器、重新下发站点配置），**必须**经面板人工批准后才执行；
/// - `destructive`：不可逆变更（删除容器等），批准者还需 `mcp:delete`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Read,
    Write,
    Destructive,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskLevel::Read => "read",
            RiskLevel::Write => "write",
            RiskLevel::Destructive => "destructive",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "read" => Some(RiskLevel::Read),
            "write" => Some(RiskLevel::Write),
            "destructive" => Some(RiskLevel::Destructive),
            _ => None,
        }
    }
}

/// 审批单状态。`pending` → `executing` → `succeeded` / `failed`；`pending` → `rejected` / `expired`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Executing,
    Succeeded,
    Failed,
    Rejected,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Executing => "executing",
            ApprovalStatus::Succeeded => "succeeded",
            ApprovalStatus::Failed => "failed",
            ApprovalStatus::Rejected => "rejected",
            ApprovalStatus::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => ApprovalStatus::Pending,
            "executing" => ApprovalStatus::Executing,
            "succeeded" => ApprovalStatus::Succeeded,
            "failed" => ApprovalStatus::Failed,
            "rejected" => ApprovalStatus::Rejected,
            "expired" => ApprovalStatus::Expired,
            _ => return None,
        })
    }

    /// 终态（不会再变化）。
    pub fn is_final(self) -> bool {
        !matches!(self, ApprovalStatus::Pending | ApprovalStatus::Executing)
    }
}

/// 写操作工具的审批单。
///
/// `raw_arguments` 是执行时使用的原始参数，**永不序列化**到 API；对外只展示脱敏后的 `arguments`。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolApproval {
    pub id: i64,
    pub principal_id: i64,
    pub session_id: Option<String>,
    pub tool: String,
    pub risk: RiskLevel,
    /// 调用参数（JSON 文本，已脱敏）
    pub arguments: String,
    #[serde(skip_serializing, default)]
    pub raw_arguments: String,
    /// 执行前预览（dry-run）：将要发生的变更与当前状态
    pub preview: String,
    pub status: ApprovalStatus,
    pub requested_at: DateTime<Utc>,
    /// 超过该时间仍未处理即过期
    pub expires_at: DateTime<Utc>,
    /// 批准 / 拒绝者用户 ID
    pub decided_by: Option<i64>,
    pub decided_at: Option<DateTime<Utc>>,
    /// 拒绝理由（可空）
    pub reason: String,
    /// 执行输出或失败原因（截断到 8 KiB）
    pub result: String,
    pub finished_at: Option<DateTime<Utc>>,
}

/// 审批单查询过滤。
#[derive(Debug, Clone, Default)]
pub struct ApprovalFilter {
    pub principal_id: Option<i64>,
    pub status: Option<ApprovalStatus>,
}
