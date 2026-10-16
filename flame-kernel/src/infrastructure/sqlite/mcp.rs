/// M-AI-1：MCP 凭证 / 会话 / 工具调用审计的 SQLite 仓储。
///
/// 时间统一以 RFC 3339 文本存储（与 `tasks` 表一致），读取时解析失败回退为 UNIX 纪元，
/// 不让单行脏数据拖垮整个列表。
use super::*;
use crate::domain::mcp::{
    AgentPrincipal, AgentSession, ApprovalFilter, ApprovalStatus, RiskLevel, ToolApproval,
    ToolCallFilter, ToolCallRecord,
};

pub struct SqliteMcpRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteMcpRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self::new_rw(pool.clone(), pool)
    }

    pub fn new_rw(write_pool: SqlitePool, read_pool: SqlitePool) -> Self {
        Self {
            pool: write_pool,
            read_pool,
        }
    }
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::internal(format!("DB error: {}", e))
}

fn ts(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
}

fn parse_opt_ts(s: Option<String>) -> Option<DateTime<Utc>> {
    s.filter(|v| !v.is_empty()).map(|v| parse_ts(&v))
}

const PRINCIPAL_COLS: &str = "id, name, user_id, token_prefix, token_hash, created_at, last_used_at, expires_at, revoked_at, allow_write";
const APPROVAL_COLS: &str = "id, principal_id, session_id, tool, risk, arguments, raw_arguments, preview, status, requested_at, expires_at, decided_by, decided_at, reason, result, finished_at";
const SESSION_COLS: &str = "id, principal_id, client_name, client_version, protocol_version, created_at, last_seen_at, closed_at";
const CALL_COLS: &str =
    "id, principal_id, session_id, tool, arguments, ok, error, duration_ms, created_at";

fn row_to_principal(r: &sqlx::sqlite::SqliteRow) -> AgentPrincipal {
    AgentPrincipal {
        id: r.get("id"),
        name: r.get("name"),
        user_id: r.get("user_id"),
        token_prefix: r.get("token_prefix"),
        token_hash: r.get("token_hash"),
        created_at: parse_ts(&r.get::<String, _>("created_at")),
        last_used_at: parse_opt_ts(r.get("last_used_at")),
        expires_at: parse_opt_ts(r.get("expires_at")),
        revoked_at: parse_opt_ts(r.get("revoked_at")),
        allow_write: r.get::<i64, _>("allow_write") != 0,
    }
}

fn row_to_approval(r: &sqlx::sqlite::SqliteRow) -> ToolApproval {
    ToolApproval {
        id: r.get("id"),
        principal_id: r.get("principal_id"),
        session_id: r.get("session_id"),
        tool: r.get("tool"),
        // 未知取值按最严格处理，不让脏数据降级风险
        risk: RiskLevel::parse(&r.get::<String, _>("risk")).unwrap_or(RiskLevel::Destructive),
        arguments: r.get("arguments"),
        raw_arguments: r.get("raw_arguments"),
        preview: r.get("preview"),
        status: ApprovalStatus::parse(&r.get::<String, _>("status"))
            .unwrap_or(ApprovalStatus::Failed),
        requested_at: parse_ts(&r.get::<String, _>("requested_at")),
        expires_at: parse_ts(&r.get::<String, _>("expires_at")),
        decided_by: r.get("decided_by"),
        decided_at: parse_opt_ts(r.get("decided_at")),
        reason: r.get("reason"),
        result: r.get("result"),
        finished_at: parse_opt_ts(r.get("finished_at")),
    }
}

/// 审批过滤 → WHERE 子句（列名常量，值参数绑定）。
fn approval_where(f: &ApprovalFilter) -> String {
    let mut conds: Vec<&str> = Vec::new();
    if f.principal_id.is_some() {
        conds.push("principal_id = ?");
    }
    if f.status.is_some() {
        conds.push("status = ?");
    }
    if conds.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conds.join(" AND "))
    }
}

macro_rules! bind_approval_filter {
    ($q:expr, $f:expr) => {{
        let mut q = $q;
        if let Some(p) = $f.principal_id {
            q = q.bind(p);
        }
        if let Some(s) = $f.status {
            q = q.bind(s.as_str());
        }
        q
    }};
}

fn row_to_session(r: &sqlx::sqlite::SqliteRow) -> AgentSession {
    AgentSession {
        id: r.get("id"),
        principal_id: r.get("principal_id"),
        client_name: r.get("client_name"),
        client_version: r.get("client_version"),
        protocol_version: r.get("protocol_version"),
        created_at: parse_ts(&r.get::<String, _>("created_at")),
        last_seen_at: parse_ts(&r.get::<String, _>("last_seen_at")),
        closed_at: parse_opt_ts(r.get("closed_at")),
    }
}

fn row_to_call(r: &sqlx::sqlite::SqliteRow) -> ToolCallRecord {
    ToolCallRecord {
        id: r.get("id"),
        principal_id: r.get("principal_id"),
        session_id: r.get("session_id"),
        tool: r.get("tool"),
        arguments: r.get("arguments"),
        ok: r.get::<i64, _>("ok") != 0,
        error: r.get("error"),
        duration_ms: r.get("duration_ms"),
        created_at: parse_ts(&r.get::<String, _>("created_at")),
    }
}

/// 过滤条件 → WHERE 子句（列名为常量，值全部走参数绑定）。
fn call_where(f: &ToolCallFilter) -> String {
    let mut conds: Vec<&str> = Vec::new();
    if f.principal_id.is_some() {
        conds.push("principal_id = ?");
    }
    if f.session_id.is_some() {
        conds.push("session_id = ?");
    }
    if f.tool.is_some() {
        conds.push("tool = ?");
    }
    if conds.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conds.join(" AND "))
    }
}

macro_rules! bind_call_filter {
    ($q:expr, $f:expr) => {{
        let mut q = $q;
        if let Some(p) = $f.principal_id {
            q = q.bind(p);
        }
        if let Some(s) = &$f.session_id {
            q = q.bind(s.clone());
        }
        if let Some(t) = &$f.tool {
            q = q.bind(t.clone());
        }
        q
    }};
}

#[async_trait]
impl McpRepository for SqliteMcpRepository {
    async fn create_principal(&self, p: &AgentPrincipal) -> Result<i64, AppError> {
        let r = sqlx::query(
            "INSERT INTO agent_principals (name, user_id, token_prefix, token_hash, created_at, expires_at, allow_write) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&p.name)
        .bind(p.user_id)
        .bind(&p.token_prefix)
        .bind(&p.token_hash)
        .bind(ts(&p.created_at))
        .bind(p.expires_at.as_ref().map(ts))
        .bind(p.allow_write as i64)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_rowid())
    }

    async fn list_principals(&self) -> Result<Vec<AgentPrincipal>, AppError> {
        let sql = format!("SELECT {PRINCIPAL_COLS} FROM agent_principals ORDER BY id DESC");
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(rows.iter().map(row_to_principal).collect())
    }

    async fn find_principal(&self, id: i64) -> Result<Option<AgentPrincipal>, AppError> {
        let sql = format!("SELECT {PRINCIPAL_COLS} FROM agent_principals WHERE id = ?");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(row.as_ref().map(row_to_principal))
    }

    async fn find_principal_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AgentPrincipal>, AppError> {
        let sql = format!("SELECT {PRINCIPAL_COLS} FROM agent_principals WHERE token_hash = ?");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(token_hash)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(row.as_ref().map(row_to_principal))
    }

    async fn touch_principal(&self, id: i64, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE agent_principals SET last_used_at = ? WHERE id = ?")
            .bind(ts(&at))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn revoke_principal(&self, id: i64, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE agent_principals SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(ts(&at))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn create_session(&self, s: &AgentSession) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO agent_sessions (id, principal_id, client_name, client_version, protocol_version, created_at, last_seen_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&s.id)
        .bind(s.principal_id)
        .bind(&s.client_name)
        .bind(&s.client_version)
        .bind(&s.protocol_version)
        .bind(ts(&s.created_at))
        .bind(ts(&s.last_seen_at))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn find_session(&self, id: &str) -> Result<Option<AgentSession>, AppError> {
        let sql = format!("SELECT {SESSION_COLS} FROM agent_sessions WHERE id = ?");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(row.as_ref().map(row_to_session))
    }

    async fn touch_session(&self, id: &str, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE agent_sessions SET last_seen_at = ? WHERE id = ?")
            .bind(ts(&at))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn close_session(&self, id: &str, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query("UPDATE agent_sessions SET closed_at = ? WHERE id = ? AND closed_at IS NULL")
            .bind(ts(&at))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn list_sessions(
        &self,
        principal_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<AgentSession>, AppError> {
        let rows = match principal_id {
            Some(pid) => {
                let sql = format!(
                    "SELECT {SESSION_COLS} FROM agent_sessions WHERE principal_id = ? ORDER BY last_seen_at DESC LIMIT ?"
                );
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(pid)
                    .bind(limit)
                    .fetch_all(&self.read_pool)
                    .await
            }
            None => {
                let sql = format!(
                    "SELECT {SESSION_COLS} FROM agent_sessions ORDER BY last_seen_at DESC LIMIT ?"
                );
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(limit)
                    .fetch_all(&self.read_pool)
                    .await
            }
        }
        .map_err(db_err)?;
        Ok(rows.iter().map(row_to_session).collect())
    }

    async fn record_call(&self, c: &ToolCallRecord) -> Result<i64, AppError> {
        let r = sqlx::query(
            "INSERT INTO tool_calls (principal_id, session_id, tool, arguments, ok, error, duration_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(c.principal_id)
        .bind(&c.session_id)
        .bind(&c.tool)
        .bind(&c.arguments)
        .bind(if c.ok { 1i64 } else { 0i64 })
        .bind(&c.error)
        .bind(c.duration_ms)
        .bind(ts(&c.created_at))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_rowid())
    }

    async fn list_calls(
        &self,
        filter: &ToolCallFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ToolCallRecord>, AppError> {
        let sql = format!(
            "SELECT {CALL_COLS} FROM tool_calls{} ORDER BY id DESC LIMIT ? OFFSET ?",
            call_where(filter)
        );
        let q = bind_call_filter!(sqlx::query(sqlx::AssertSqlSafe(sql)), filter);
        let rows = q
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(rows.iter().map(row_to_call).collect())
    }

    async fn count_calls(&self, filter: &ToolCallFilter) -> Result<i64, AppError> {
        let sql = format!("SELECT COUNT(*) AS n FROM tool_calls{}", call_where(filter));
        let q = bind_call_filter!(sqlx::query(sqlx::AssertSqlSafe(sql)), filter);
        let row = q.fetch_one(&self.read_pool).await.map_err(db_err)?;
        Ok(row.get::<i64, _>("n"))
    }

    async fn prune_calls(&self, keep: i64) -> Result<u64, AppError> {
        let r = sqlx::query(
            "DELETE FROM tool_calls WHERE id <= (SELECT id FROM tool_calls ORDER BY id DESC LIMIT 1 OFFSET ?)",
        )
        .bind(keep.max(0))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.rows_affected())
    }

    async fn create_approval(&self, a: &ToolApproval) -> Result<i64, AppError> {
        let r = sqlx::query(
            "INSERT INTO tool_approvals (principal_id, session_id, tool, risk, arguments, raw_arguments, preview, status, requested_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(a.principal_id)
        .bind(&a.session_id)
        .bind(&a.tool)
        .bind(a.risk.as_str())
        .bind(&a.arguments)
        .bind(&a.raw_arguments)
        .bind(&a.preview)
        .bind(a.status.as_str())
        .bind(ts(&a.requested_at))
        .bind(ts(&a.expires_at))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.last_insert_rowid())
    }

    async fn find_approval(&self, id: i64) -> Result<Option<ToolApproval>, AppError> {
        // 走写池：审批状态机需要读到最新提交
        let sql = format!("SELECT {APPROVAL_COLS} FROM tool_approvals WHERE id = ?");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(row.as_ref().map(row_to_approval))
    }

    async fn list_approvals(
        &self,
        filter: &ApprovalFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ToolApproval>, AppError> {
        let sql = format!(
            "SELECT {APPROVAL_COLS} FROM tool_approvals{} ORDER BY id DESC LIMIT ? OFFSET ?",
            approval_where(filter)
        );
        let q = bind_approval_filter!(sqlx::query(sqlx::AssertSqlSafe(sql)), filter);
        let rows = q
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.read_pool)
            .await
            .map_err(db_err)?;
        Ok(rows.iter().map(row_to_approval).collect())
    }

    async fn count_approvals(&self, filter: &ApprovalFilter) -> Result<i64, AppError> {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM tool_approvals{}",
            approval_where(filter)
        );
        let q = bind_approval_filter!(sqlx::query(sqlx::AssertSqlSafe(sql)), filter);
        let row = q.fetch_one(&self.read_pool).await.map_err(db_err)?;
        Ok(row.get::<i64, _>("n"))
    }

    async fn transition_approval(
        &self,
        id: i64,
        from: ApprovalStatus,
        to: ApprovalStatus,
        decided_by: Option<i64>,
        at: DateTime<Utc>,
        reason: &str,
    ) -> Result<bool, AppError> {
        let r = sqlx::query(
            "UPDATE tool_approvals SET status = ?, decided_by = ?, decided_at = ?, reason = ? WHERE id = ? AND status = ?",
        )
        .bind(to.as_str())
        .bind(decided_by)
        .bind(ts(&at))
        .bind(reason)
        .bind(id)
        .bind(from.as_str())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.rows_affected() == 1)
    }

    async fn finish_approval(
        &self,
        id: i64,
        status: ApprovalStatus,
        result: &str,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tool_approvals SET status = ?, result = ?, finished_at = ? WHERE id = ? AND status = 'executing'",
        )
        .bind(status.as_str())
        .bind(result)
        .bind(ts(&at))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn expire_approvals(&self, now: DateTime<Utc>) -> Result<u64, AppError> {
        // RFC 3339（同为 UTC、同格式）按字典序即时间序
        let r = sqlx::query(
            "UPDATE tool_approvals SET status = 'expired', decided_at = ? WHERE status = 'pending' AND expires_at < ?",
        )
        .bind(ts(&now))
        .bind(ts(&now))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.rows_affected())
    }
}
