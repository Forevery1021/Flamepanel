use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::core::error::AppError;
use crate::domain::{User, Website, CreateWebsiteRequest, WafRule, CreateWafRuleRequest, UpdateWafRuleRequest, WafIpRule, CreateWafIpRuleRequest, OperationLogEntry, PageParams, PagedResult};

// ─── User Repository ──────────────────────────────────────────────────────────

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError>;
    async fn create(&self, username: &str, password_hash: &str, role: &str) -> Result<User, AppError>;
    async fn update_password(&self, id: i64, password_hash: &str) -> Result<(), AppError>;
    async fn update_last_login(&self, id: i64) -> Result<(), AppError>;
    async fn update_role(&self, id: i64, role: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn list(&self) -> Result<Vec<User>, AppError>;
}

pub struct SqliteUserRepository {
    db: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at, last_login FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("数据库查询失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at, last_login FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("数据库查询失败: {e}")))
    }

    async fn create(&self, username: &str, password_hash: &str, role: &str) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?) RETURNING id, username, password_hash, role, created_at, last_login"
        )
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建用户失败: {e}")))
    }

    async fn update_password(&self, id: i64, password_hash: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(password_hash)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新密码失败: {e}")))?;
        Ok(())
    }

    async fn update_last_login(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET last_login = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新登录时间失败: {e}")))?;
        Ok(())
    }

    async fn update_role(&self, id: i64, role: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(role)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新用户角色失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除用户失败: {e}")))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<User>, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at, last_login FROM users ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询用户列表失败: {e}")))
    }
}

// ─── Website Repository ───────────────────────────────────────────────────────

#[async_trait]
pub trait WebsiteRepository: Send + Sync {
    async fn create(&self, req: &CreateWebsiteRequest, config_path: &str) -> Result<Website, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError>;
    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError>;
    async fn list(&self) -> Result<Vec<Website>, AppError>;
    async fn update_ssl(&self, id: i64, cert_path: &str, key_path: &str) -> Result<(), AppError>;
    async fn toggle_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteWebsiteRepository {
    db: SqlitePool,
}

impl SqliteWebsiteRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl WebsiteRepository for SqliteWebsiteRepository {
    async fn create(&self, req: &CreateWebsiteRequest, config_path: &str) -> Result<Website, AppError> {
        sqlx::query_as::<_, Website>(
            "INSERT INTO websites (domain, root_path, proxy_port, ssl_enabled, config_path, engine, enabled) VALUES (?, ?, ?, ?, ?, ?, true) RETURNING id, domain, root_path, proxy_port, ssl_enabled, ssl_cert_path, ssl_key_path, config_path, engine, enabled, created_at, updated_at"
        )
        .bind(&req.domain)
        .bind(&req.root_path)
        .bind(req.proxy_port)
        .bind(req.enable_ssl)
        .bind(config_path)
        .bind(req.engine.as_deref().unwrap_or("nginx"))
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建站点失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError> {
        sqlx::query_as::<_, Website>(
            "SELECT id, domain, root_path, proxy_port, ssl_enabled, ssl_cert_path, ssl_key_path, config_path, enabled, engine, created_at, updated_at FROM websites WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询站点失败: {e}")))
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError> {
        sqlx::query_as::<_, Website>(
            "SELECT id, domain, root_path, proxy_port, ssl_enabled, ssl_cert_path, ssl_key_path, config_path, enabled, engine, created_at, updated_at FROM websites WHERE domain = ?"
        )
        .bind(domain)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询站点失败: {e}")))
    }

    async fn list(&self) -> Result<Vec<Website>, AppError> {
        sqlx::query_as::<_, Website>(
            "SELECT id, domain, root_path, proxy_port, ssl_enabled, ssl_cert_path, ssl_key_path, config_path, enabled, engine, created_at, updated_at FROM websites ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询站点列表失败: {e}")))
    }

    async fn update_ssl(&self, id: i64, cert_path: &str, key_path: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE websites SET ssl_enabled = true, ssl_cert_path = ?, ssl_key_path = ? WHERE id = ?")
            .bind(cert_path)
            .bind(key_path)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新SSL失败: {e}")))?;
        Ok(())
    }

    async fn toggle_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError> {
        sqlx::query("UPDATE websites SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("切换站点状态失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM websites WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除站点失败: {e}")))?;
        Ok(())
    }
}

// ─── Operation Log Repository ─────────────────────────────────────────────────

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn log(&self, username: &str, action: &str, target: Option<&str>, ip: Option<&str>) -> Result<(), AppError>;
    async fn list_paginated(&self, params: &PageParams) -> Result<PagedResult<OperationLogEntry>, AppError>;
    async fn count(&self) -> Result<i64, AppError>;
}

pub struct SqliteLogRepository {
    db: SqlitePool,
}

impl SqliteLogRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LogRepository for SqliteLogRepository {
    async fn log(&self, username: &str, action: &str, target: Option<&str>, ip: Option<&str>) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO operation_logs (username, action, target, ip) VALUES (?, ?, ?, ?)"
        )
        .bind(username)
        .bind(action)
        .bind(target.unwrap_or(""))
        .bind(ip.unwrap_or(""))
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("写入日志失败: {e}")))?;
        Ok(())
    }

    async fn list_paginated(&self, params: &PageParams) -> Result<PagedResult<OperationLogEntry>, AppError> {
        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).min(100);

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM operation_logs")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("统计日志失败: {e}")))?;

        let offset = (page - 1) * page_size;
        let items = sqlx::query_as::<_, OperationLogEntry>(
            "SELECT username, action, COALESCE(target,'') as target, COALESCE(ip,'') as ip, created_at FROM operation_logs ORDER BY id DESC LIMIT ? OFFSET ?"
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询日志列表失败: {e}")))?;

        Ok(PagedResult { items, total: total.0, page, page_size })
    }

    async fn count(&self) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM operation_logs")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("统计日志失败: {e}")))?;
        Ok(row.0)
    }
}

// ─── WAF Rule Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait WafRuleRepository: Send + Sync {
    async fn create(&self, req: &CreateWafRuleRequest) -> Result<WafRule, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<WafRule>, AppError>;
    async fn list_all(&self) -> Result<Vec<WafRule>, AppError>;
    async fn list_enabled(&self) -> Result<Vec<WafRule>, AppError>;
    async fn update(&self, id: i64, req: &UpdateWafRuleRequest) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn count(&self) -> Result<(i64, i64), AppError>;
}

pub struct SqliteWafRuleRepository {
    db: SqlitePool,
}

impl SqliteWafRuleRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl WafRuleRepository for SqliteWafRuleRepository {
    async fn create(&self, req: &CreateWafRuleRequest) -> Result<WafRule, AppError> {
        sqlx::query_as::<_, WafRule>(
            "INSERT INTO waf_rules (name, pattern, target, action, description) VALUES (?, ?, ?, ?, ?) RETURNING id, name, pattern, target, action, description, enabled, created_at, updated_at"
        )
        .bind(&req.name)
        .bind(&req.pattern)
        .bind(&req.target)
        .bind(&req.action)
        .bind(&req.description)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建 WAF 规则失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WafRule>, AppError> {
        sqlx::query_as::<_, WafRule>(
            "SELECT id, name, pattern, target, action, description, enabled, created_at, updated_at FROM waf_rules WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询 WAF 规则失败: {e}")))
    }

    async fn list_all(&self) -> Result<Vec<WafRule>, AppError> {
        sqlx::query_as::<_, WafRule>(
            "SELECT id, name, pattern, target, action, description, enabled, created_at, updated_at FROM waf_rules ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询 WAF 规则列表失败: {e}")))
    }

    async fn list_enabled(&self) -> Result<Vec<WafRule>, AppError> {
        sqlx::query_as::<_, WafRule>(
            "SELECT id, name, pattern, target, action, description, enabled, created_at, updated_at FROM waf_rules WHERE enabled = 1 ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询启用的 WAF 规则失败: {e}")))
    }

    async fn update(&self, id: i64, req: &UpdateWafRuleRequest) -> Result<(), AppError> {
        let existing = self.find_by_id(id).await?
            .ok_or(AppError::NotFound("WAF 规则不存在".into()))?;

        sqlx::query(
            "UPDATE waf_rules SET name = ?, pattern = ?, target = ?, action = ?, description = ?, enabled = ? WHERE id = ?"
        )
        .bind(req.name.as_deref().unwrap_or(&existing.name))
        .bind(req.pattern.as_deref().unwrap_or(&existing.pattern))
        .bind(req.target.as_deref().unwrap_or(&existing.target))
        .bind(req.action.as_deref().unwrap_or(&existing.action))
        .bind(req.description.as_ref().or(existing.description.as_ref()))
        .bind(req.enabled.unwrap_or(existing.enabled))
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("更新 WAF 规则失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM waf_rules WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除 WAF 规则失败: {e}")))?;
        Ok(())
    }

    async fn count(&self) -> Result<(i64, i64), AppError> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waf_rules")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("统计 WAF 规则失败: {e}")))?;
        let enabled: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waf_rules WHERE enabled = 1")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("统计启用的 WAF 规则失败: {e}")))?;
        Ok((total.0, enabled.0))
    }
}

// ─── Log Query (for dashboard) ────────────────────────────────────────────────

impl SqliteLogRepository {
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<OperationLogEntry>, AppError> {
        sqlx::query_as::<_, OperationLogEntry>(
            "SELECT username, action, COALESCE(target,'') as target, COALESCE(ip,'') as ip, created_at FROM operation_logs ORDER BY id DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询日志失败: {e}")))
    }
}

// ─── WAF IP Rule Repository ───────────────────────────────────────────────────

#[async_trait]
pub trait WafIpRuleRepository: Send + Sync {
    async fn create(&self, req: &CreateWafIpRuleRequest) -> Result<WafIpRule, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<WafIpRule>, AppError>;
    async fn list_all(&self) -> Result<Vec<WafIpRule>, AppError>;
    async fn list_enabled(&self) -> Result<Vec<WafIpRule>, AppError>;
    async fn update(&self, id: i64, enabled: bool, description: Option<&str>) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteWafIpRuleRepository {
    db: SqlitePool,
}

impl SqliteWafIpRuleRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl WafIpRuleRepository for SqliteWafIpRuleRepository {
    async fn create(&self, req: &CreateWafIpRuleRequest) -> Result<WafIpRule, AppError> {
        sqlx::query_as::<_, WafIpRule>(
            "INSERT INTO waf_ip_rules (ip, action, description) VALUES (?, ?, ?) RETURNING id, ip, action, description, enabled, created_at, updated_at"
        )
        .bind(&req.ip)
        .bind(&req.action)
        .bind(&req.description)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建 IP 规则失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WafIpRule>, AppError> {
        sqlx::query_as::<_, WafIpRule>(
            "SELECT id, ip, action, description, enabled, created_at, updated_at FROM waf_ip_rules WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询 IP 规则失败: {e}")))
    }

    async fn list_all(&self) -> Result<Vec<WafIpRule>, AppError> {
        sqlx::query_as::<_, WafIpRule>(
            "SELECT id, ip, action, description, enabled, created_at, updated_at FROM waf_ip_rules ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询 IP 规则列表失败: {e}")))
    }

    async fn list_enabled(&self) -> Result<Vec<WafIpRule>, AppError> {
        sqlx::query_as::<_, WafIpRule>(
            "SELECT id, ip, action, description, enabled, created_at, updated_at FROM waf_ip_rules WHERE enabled = 1 ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询启用的 IP 规则失败: {e}")))
    }

    async fn update(&self, id: i64, enabled: bool, description: Option<&str>) -> Result<(), AppError> {
        sqlx::query("UPDATE waf_ip_rules SET enabled = ?, description = COALESCE(?, description) WHERE id = ?")
            .bind(enabled)
            .bind(description)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新 IP 规则失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM waf_ip_rules WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除 IP 规则失败: {e}")))?;
        Ok(())
    }
}
