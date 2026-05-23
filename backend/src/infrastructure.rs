use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::core::error::AppError;
use crate::domain::{User, Website, CreateWebsiteRequest, WafRule, CreateWafRuleRequest, UpdateWafRuleRequest, WafIpRule, CreateWafIpRuleRequest, OperationLogEntry, PageParams, PagedResult, Setting, PanelSettings, CronJob, CreateCronJobRequest, UpdateCronJobRequest, CronJobLog, DatabaseInstance, DatabaseBackup, InstalledApp};

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

// ─── Settings Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_all(&self) -> Result<PanelSettings, AppError>;
    async fn get(&self, key: &str) -> Result<Option<String>, AppError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), AppError>;
}

pub struct SqliteSettingsRepository {
    db: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get_all(&self) -> Result<PanelSettings, AppError> {
        let rows = sqlx::query_as::<_, Setting>(
            "SELECT key, value FROM settings"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询设置失败: {e}")))?;

        let mut theme = "light".to_string();
        let mut language = "zh-CN".to_string();
        for row in rows {
            match row.key.as_str() {
                "theme" => theme = row.value,
                "language" => language = row.value,
                _ => {}
            }
        }
        Ok(PanelSettings { theme, language })
    }

    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let row = sqlx::query_as::<_, Setting>(
            "SELECT key, value FROM settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询设置失败: {e}")))?;
        Ok(row.map(|r| r.value))
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(key)
        .bind(value)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("保存设置失败: {e}")))?;
        Ok(())
    }
}

// ─── CronJob Repository ───────────────────────────────────────────────────────

#[async_trait]
pub trait CronJobRepository: Send + Sync {
    async fn create(&self, req: &CreateCronJobRequest) -> Result<CronJob, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<CronJob>, AppError>;
    async fn list_all(&self) -> Result<Vec<CronJob>, AppError>;
    async fn list_enabled(&self) -> Result<Vec<CronJob>, AppError>;
    async fn update(&self, id: i64, req: &UpdateCronJobRequest) -> Result<(), AppError>;
    async fn update_run_time(&self, id: i64, last_run: &str, next_run: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn log(&self, job_id: i64, status: &str, output: Option<&str>, started_at: &str, finished_at: &str) -> Result<(), AppError>;
    async fn list_logs(&self, job_id: i64, limit: i64) -> Result<Vec<CronJobLog>, AppError>;
}

pub struct SqliteCronJobRepository {
    db: SqlitePool,
}

impl SqliteCronJobRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CronJobRepository for SqliteCronJobRepository {
    async fn create(&self, req: &CreateCronJobRequest) -> Result<CronJob, AppError> {
        sqlx::query_as::<_, CronJob>(
            "INSERT INTO cron_jobs (name, schedule, command, url) VALUES (?, ?, ?, ?) RETURNING id, name, schedule, command, url, enabled, last_run, next_run, created_at, updated_at"
        )
        .bind(&req.name)
        .bind(&req.schedule)
        .bind(&req.command)
        .bind(&req.url)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建计划任务失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<CronJob>, AppError> {
        sqlx::query_as::<_, CronJob>(
            "SELECT id, name, schedule, command, url, enabled, last_run, next_run, created_at, updated_at FROM cron_jobs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询计划任务失败: {e}")))
    }

    async fn list_all(&self) -> Result<Vec<CronJob>, AppError> {
        sqlx::query_as::<_, CronJob>(
            "SELECT id, name, schedule, command, url, enabled, last_run, next_run, created_at, updated_at FROM cron_jobs ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询计划任务列表失败: {e}")))
    }

    async fn list_enabled(&self) -> Result<Vec<CronJob>, AppError> {
        sqlx::query_as::<_, CronJob>(
            "SELECT id, name, schedule, command, url, enabled, last_run, next_run, created_at, updated_at FROM cron_jobs WHERE enabled = 1 ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询启用的计划任务失败: {e}")))
    }

    async fn update(&self, id: i64, req: &UpdateCronJobRequest) -> Result<(), AppError> {
        let existing = self.find_by_id(id).await?
            .ok_or(AppError::NotFound("计划任务不存在".into()))?;

        sqlx::query(
            "UPDATE cron_jobs SET name = ?, schedule = ?, command = ?, url = ?, enabled = ? WHERE id = ?"
        )
        .bind(req.name.as_deref().unwrap_or(&existing.name))
        .bind(req.schedule.as_deref().unwrap_or(&existing.schedule))
        .bind(req.command.as_ref().or(existing.command.as_ref()))
        .bind(req.url.as_ref().or(existing.url.as_ref()))
        .bind(req.enabled.unwrap_or(existing.enabled))
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("更新计划任务失败: {e}")))?;
        Ok(())
    }

    async fn update_run_time(&self, id: i64, last_run: &str, next_run: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE cron_jobs SET last_run = ?, next_run = ? WHERE id = ?")
            .bind(last_run)
            .bind(next_run)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新运行时间失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM cron_jobs WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除计划任务失败: {e}")))?;
        Ok(())
    }

    async fn log(&self, job_id: i64, status: &str, output: Option<&str>, started_at: &str, finished_at: &str) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO cron_job_logs (job_id, status, output, started_at, finished_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(job_id)
        .bind(status)
        .bind(output)
        .bind(started_at)
        .bind(finished_at)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("写入任务日志失败: {e}")))?;
        Ok(())
    }

    async fn list_logs(&self, job_id: i64, limit: i64) -> Result<Vec<CronJobLog>, AppError> {
        sqlx::query_as::<_, CronJobLog>(
            "SELECT id, job_id, status, output, started_at, finished_at FROM cron_job_logs WHERE job_id = ? ORDER BY id DESC LIMIT ?"
        )
        .bind(job_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询任务日志失败: {e}")))
    }
}

// ─── Database Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait DatabaseRepository: Send + Sync {
    async fn create(
        &self, name: &str, db_type: &str, version: &str, port: i32,
        container_id: Option<&str>, username: &str, password: &str,
        data_dir: Option<&str>,
    ) -> Result<DatabaseInstance, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError>;
    async fn list_all(&self) -> Result<Vec<DatabaseInstance>, AppError>;
    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteDatabaseRepository {
    db: SqlitePool,
}

impl SqliteDatabaseRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DatabaseRepository for SqliteDatabaseRepository {
    async fn create(
        &self, name: &str, db_type: &str, version: &str, port: i32,
        container_id: Option<&str>, username: &str, password: &str,
        data_dir: Option<&str>,
    ) -> Result<DatabaseInstance, AppError> {
        sqlx::query_as::<_, DatabaseInstance>(
            "INSERT INTO database_instances (name, db_type, version, port, container_id, username, password, status, data_dir) VALUES (?, ?, ?, ?, ?, ?, ?, 'installing', ?) RETURNING id, name, db_type, version, port, container_id, username, password, status, data_dir, created_at, updated_at"
        )
        .bind(name)
        .bind(db_type)
        .bind(version)
        .bind(port)
        .bind(container_id)
        .bind(username)
        .bind(password)
        .bind(data_dir)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建数据库实例失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError> {
        sqlx::query_as::<_, DatabaseInstance>(
            "SELECT id, name, db_type, version, port, container_id, username, password, status, data_dir, created_at, updated_at FROM database_instances WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询数据库实例失败: {e}")))
    }

    async fn list_all(&self) -> Result<Vec<DatabaseInstance>, AppError> {
        sqlx::query_as::<_, DatabaseInstance>(
            "SELECT id, name, db_type, version, port, container_id, username, password, status, data_dir, created_at, updated_at FROM database_instances ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询数据库列表失败: {e}")))
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE database_instances SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新数据库状态失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM database_instances WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除数据库实例失败: {e}")))?;
        Ok(())
    }
}

// ─── Database Backup Repository ────────────────────────────────────────────────

#[async_trait]
pub trait DatabaseBackupRepository: Send + Sync {
    async fn create(&self, instance_id: i64, filename: &str, size_bytes: i64) -> Result<DatabaseBackup, AppError>;
    async fn list_by_instance(&self, instance_id: i64) -> Result<Vec<DatabaseBackup>, AppError>;
}

pub struct SqliteDatabaseBackupRepository {
    db: SqlitePool,
}

impl SqliteDatabaseBackupRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DatabaseBackupRepository for SqliteDatabaseBackupRepository {
    async fn create(&self, instance_id: i64, filename: &str, size_bytes: i64) -> Result<DatabaseBackup, AppError> {
        sqlx::query_as::<_, DatabaseBackup>(
            "INSERT INTO database_backups (instance_id, filename, size_bytes) VALUES (?, ?, ?) RETURNING id, instance_id, filename, size_bytes, created_at"
        )
        .bind(instance_id)
        .bind(filename)
        .bind(size_bytes)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建备份记录失败: {e}")))
    }

    async fn list_by_instance(&self, instance_id: i64) -> Result<Vec<DatabaseBackup>, AppError> {
        sqlx::query_as::<_, DatabaseBackup>(
            "SELECT id, instance_id, filename, size_bytes, created_at FROM database_backups WHERE instance_id = ? ORDER BY id DESC"
        )
        .bind(instance_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询备份列表失败: {e}")))
    }
}

// ─── App Store Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait AppRepository: Send + Sync {
    async fn create(
        &self, app_key: &str, name: &str, category: &str, port: i32,
        version: &str, description: Option<&str>, compose_file: Option<&str>,
        data_dir: Option<&str>,
    ) -> Result<InstalledApp, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<InstalledApp>, AppError>;
    async fn list_all(&self) -> Result<Vec<InstalledApp>, AppError>;
    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteAppRepository {
    db: SqlitePool,
}

impl SqliteAppRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AppRepository for SqliteAppRepository {
    async fn create(
        &self, app_key: &str, name: &str, category: &str, port: i32,
        version: &str, description: Option<&str>, compose_file: Option<&str>,
        data_dir: Option<&str>,
    ) -> Result<InstalledApp, AppError> {
        sqlx::query_as::<_, InstalledApp>(
            "INSERT INTO installed_apps (app_key, name, category, port, status, version, description, compose_file, data_dir) VALUES (?, ?, ?, ?, 'installing', ?, ?, ?, ?) RETURNING id, app_key, name, category, port, status, compose_file, data_dir, version, description, created_at, updated_at"
        )
        .bind(app_key)
        .bind(name)
        .bind(category)
        .bind(port)
        .bind(version)
        .bind(description)
        .bind(compose_file)
        .bind(data_dir)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建应用失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError> {
        sqlx::query_as::<_, InstalledApp>(
            "SELECT id, app_key, name, category, port, status, compose_file, data_dir, version, description, created_at, updated_at FROM installed_apps WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询应用失败: {e}")))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<InstalledApp>, AppError> {
        sqlx::query_as::<_, InstalledApp>(
            "SELECT id, app_key, name, category, port, status, compose_file, data_dir, version, description, created_at, updated_at FROM installed_apps WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询应用失败: {e}")))
    }

    async fn list_all(&self) -> Result<Vec<InstalledApp>, AppError> {
        sqlx::query_as::<_, InstalledApp>(
            "SELECT id, app_key, name, category, port, status, compose_file, data_dir, version, description, created_at, updated_at FROM installed_apps ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询应用列表失败: {e}")))
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE installed_apps SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新应用状态失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM installed_apps WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除应用失败: {e}")))?;
        Ok(())
    }
}
