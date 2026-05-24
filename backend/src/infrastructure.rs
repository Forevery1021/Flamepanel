use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::core::error::AppError;
use crate::domain::{User, Website, CreateWebsiteRequest, WafRule, CreateWafRuleRequest, UpdateWafRuleRequest, WafIpRule, CreateWafIpRuleRequest, OperationLogEntry, PageParams, PagedResult, Setting, PanelSettings, CronJob, CreateCronJobRequest, UpdateCronJobRequest, CronJobLog, DatabaseInstance, DatabaseBackup, InstalledApp, AiConversation, NodeInfo, NodeRegisterRequest, NodeHeartbeatRequest, BackupConfig, CreateBackupConfigRequest, UpdateBackupConfigRequest, BackupRecord, NotificationChannel, CreateNotificationChannelRequest, UpdateNotificationChannelRequest, AlertRule, CreateAlertRuleRequest, UpdateAlertRuleRequest, AlertHistory, Role, Permission, RoleWithPermissions, CreateRoleRequest, UpdateRoleRequest};

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
        let mut theme_color: Option<String> = None;
        let mut background_image: Option<String> = None;
        let mut background_opacity: Option<f64> = None;
        for row in rows {
            match row.key.as_str() {
                "theme" => theme = row.value,
                "language" => language = row.value,
                "theme_color" => theme_color = Some(row.value),
                "background_image" => {
                    let v = row.value;
                    if v.is_empty() { background_image = None; }
                    else { background_image = Some(v); }
                }
                "background_opacity" => {
                    background_opacity = row.value.parse::<f64>().ok();
                }
                _ => {}
            }
        }
        Ok(PanelSettings { theme, language, theme_color, background_image, background_opacity })
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

// ─── AI Conversation Repository ──────────────────────────────────────────────

#[async_trait]
pub trait AiConversationRepository: Send + Sync {
    async fn create(&self, model: &str) -> Result<AiConversation, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<AiConversation>, AppError>;
    async fn list_all(&self) -> Result<Vec<AiConversation>, AppError>;
    async fn update_messages(&self, id: i64, title: &str, messages: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteAiConversationRepository {
    db: SqlitePool,
}

impl SqliteAiConversationRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AiConversationRepository for SqliteAiConversationRepository {
    async fn create(&self, model: &str) -> Result<AiConversation, AppError> {
        sqlx::query_as::<_, AiConversation>(
            "INSERT INTO ai_conversations (model, messages) VALUES (?, '[]') RETURNING id, title, model, messages, created_at, updated_at"
        )
        .bind(model)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建对话失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<AiConversation>, AppError> {
        sqlx::query_as::<_, AiConversation>(
            "SELECT id, title, model, messages, created_at, updated_at FROM ai_conversations WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询对话失败: {e}")))
    }

    async fn list_all(&self) -> Result<Vec<AiConversation>, AppError> {
        sqlx::query_as::<_, AiConversation>(
            "SELECT id, title, model, messages, created_at, updated_at FROM ai_conversations ORDER BY updated_at DESC"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询对话列表失败: {e}")))
    }

    async fn update_messages(&self, id: i64, title: &str, messages: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE ai_conversations SET title = ?, messages = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(title)
            .bind(messages)
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新对话失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM ai_conversations WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除对话失败: {e}")))?;
        Ok(())
    }
}

// ─── Node Repository ────────────────────────────────────────────────────────

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn register(&self, req: &NodeRegisterRequest) -> Result<NodeInfo, AppError>;
    async fn heartbeat(&self, id: i64, req: &NodeHeartbeatRequest) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<NodeInfo>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<NodeInfo>, AppError>;
    async fn find_by_host(&self, host: &str) -> Result<Option<NodeInfo>, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteNodeRepository {
    db: SqlitePool,
}

impl SqliteNodeRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NodeRepository for SqliteNodeRepository {
    async fn register(&self, req: &NodeRegisterRequest) -> Result<NodeInfo, AppError> {
        let port = req.agent_port.unwrap_or(9527);
        // Check if node with same host already exists — update if so
        if let Some(existing) = self.find_by_host(&req.host).await? {
            sqlx::query(
                "UPDATE nodes SET name = ?, auth_token = ?, agent_port = ?, status = 'online', last_heartbeat = datetime('now'), updated_at = datetime('now') WHERE id = ?"
            )
            .bind(&req.name)
            .bind(&req.auth_token)
            .bind(port)
            .bind(existing.id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新节点失败: {e}")))?;
            return self.find_by_id(existing.id).await?.ok_or(AppError::Internal("节点不存在".into()));
        }

        sqlx::query(
            "INSERT INTO nodes (name, host, agent_port, auth_token) VALUES (?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.host)
        .bind(port)
        .bind(&req.auth_token)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("注册节点失败: {e}")))?;

        let id = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("获取节点 ID 失败: {e}")))?;

        self.find_by_id(id).await?.ok_or(AppError::Internal("节点创建失败".into()))
    }

    async fn heartbeat(&self, id: i64, req: &NodeHeartbeatRequest) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE nodes SET cpu_usage = ?, memory_usage_percent = ?, disk_usage_percent = ?, load_one = ?, status = 'online', last_heartbeat = datetime('now'), updated_at = datetime('now') WHERE id = ?"
        )
        .bind(req.cpu_usage)
        .bind(req.memory_usage_percent)
        .bind(req.disk_usage_percent)
        .bind(req.load_one)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("心跳更新失败: {e}")))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<NodeInfo>, AppError> {
        sqlx::query_as::<_, NodeInfo>("SELECT * FROM nodes ORDER BY name")
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询节点列表失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<NodeInfo>, AppError> {
        sqlx::query_as::<_, NodeInfo>("SELECT * FROM nodes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询节点失败: {e}")))
    }

    async fn find_by_host(&self, host: &str) -> Result<Option<NodeInfo>, AppError> {
        sqlx::query_as::<_, NodeInfo>("SELECT * FROM nodes WHERE host = ?")
            .bind(host)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询节点失败: {e}")))
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM nodes WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除节点失败: {e}")))?;
        Ok(())
    }
}

// ─── Backup Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait BackupRepository: Send + Sync {
    async fn create_config(&self, req: &CreateBackupConfigRequest) -> Result<BackupConfig, AppError>;
    async fn update_config(&self, id: i64, req: &UpdateBackupConfigRequest) -> Result<(), AppError>;
    async fn list_configs(&self) -> Result<Vec<BackupConfig>, AppError>;
    async fn find_config(&self, id: i64) -> Result<Option<BackupConfig>, AppError>;
    async fn delete_config(&self, id: i64) -> Result<(), AppError>;

    async fn create_record(&self, config_id: i64, file_name: &str) -> Result<BackupRecord, AppError>;
    async fn finish_record(&self, id: i64, file_size: i64, status: &str, error_msg: Option<&str>) -> Result<(), AppError>;
    async fn list_records(&self, config_id: i64) -> Result<Vec<BackupRecord>, AppError>;
    async fn delete_old_records(&self, config_id: i64, keep_days: i64) -> Result<(), AppError>;
}

pub struct SqliteBackupRepository {
    db: SqlitePool,
}

impl SqliteBackupRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BackupRepository for SqliteBackupRepository {
    async fn create_config(&self, req: &CreateBackupConfigRequest) -> Result<BackupConfig, AppError> {
        let storage_type = req.storage_type.as_deref().unwrap_or("local");
        let storage_path = req.storage_path.as_deref().unwrap_or("data/backups");
        let retention = req.retention_days.unwrap_or(30);

        sqlx::query(
            "INSERT INTO backup_configs (name, backup_type, target_path, storage_type, storage_path, cron_expr, retention_days) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.backup_type)
        .bind(&req.target_path)
        .bind(storage_type)
        .bind(storage_path)
        .bind(&req.cron_expr)
        .bind(retention)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建备份配置失败: {e}")))?;

        let id = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("获取备份配置 ID 失败: {e}")))?;

        self.find_config(id).await?.ok_or(AppError::Internal("备份配置创建失败".into()))
    }

    async fn update_config(&self, id: i64, req: &UpdateBackupConfigRequest) -> Result<(), AppError> {
        if let Some(name) = &req.name {
            sqlx::query("UPDATE backup_configs SET name = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(name).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(t) = &req.backup_type {
            sqlx::query("UPDATE backup_configs SET backup_type = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(t).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(p) = &req.target_path {
            sqlx::query("UPDATE backup_configs SET target_path = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(p).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(s) = &req.storage_type {
            sqlx::query("UPDATE backup_configs SET storage_type = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(s).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(p) = &req.storage_path {
            sqlx::query("UPDATE backup_configs SET storage_path = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(p).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(c) = &req.cron_expr {
            sqlx::query("UPDATE backup_configs SET cron_expr = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(c).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(r) = req.retention_days {
            sqlx::query("UPDATE backup_configs SET retention_days = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(r).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        if let Some(e) = req.enabled {
            let v = if e { 1 } else { 0 };
            sqlx::query("UPDATE backup_configs SET enabled = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(v).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新失败: {e}")))?;
        }
        Ok(())
    }

    async fn list_configs(&self) -> Result<Vec<BackupConfig>, AppError> {
        sqlx::query_as::<_, BackupConfig>("SELECT * FROM backup_configs ORDER BY created_at DESC")
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询备份配置失败: {e}")))
    }

    async fn find_config(&self, id: i64) -> Result<Option<BackupConfig>, AppError> {
        sqlx::query_as::<_, BackupConfig>("SELECT * FROM backup_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询备份配置失败: {e}")))
    }

    async fn delete_config(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM backup_configs WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除备份配置失败: {e}")))?;
        Ok(())
    }

    async fn create_record(&self, config_id: i64, file_name: &str) -> Result<BackupRecord, AppError> {
        sqlx::query(
            "INSERT INTO backup_records (config_id, file_name, status) VALUES (?, ?, 'running')"
        )
        .bind(config_id)
        .bind(file_name)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建备份记录失败: {e}")))?;

        let id = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("获取记录 ID 失败: {e}")))?;

        sqlx::query_as::<_, BackupRecord>("SELECT * FROM backup_records WHERE id = ?")
            .bind(id)
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询备份记录失败: {e}")))
    }

    async fn finish_record(&self, id: i64, file_size: i64, status: &str, error_msg: Option<&str>) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE backup_records SET file_size = ?, status = ?, error_message = ?, finished_at = datetime('now') WHERE id = ?"
        )
        .bind(file_size)
        .bind(status)
        .bind(error_msg)
        .bind(id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("更新备份记录失败: {e}")))?;
        Ok(())
    }

    async fn list_records(&self, config_id: i64) -> Result<Vec<BackupRecord>, AppError> {
        sqlx::query_as::<_, BackupRecord>(
            "SELECT * FROM backup_records WHERE config_id = ? ORDER BY started_at DESC LIMIT 50"
        )
        .bind(config_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询备份记录失败: {e}")))
    }

    async fn delete_old_records(&self, config_id: i64, keep_days: i64) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM backup_records WHERE config_id = ? AND started_at < datetime('now', ?)"
        )
        .bind(config_id)
        .bind(format!("-{keep_days} days"))
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("清理旧备份失败: {e}")))?;
        Ok(())
    }
}

// ─── Notification Repository ────────────────────────────────────────────────────

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn list_channels(&self) -> Result<Vec<NotificationChannel>, AppError>;
    async fn find_channel(&self, id: i64) -> Result<Option<NotificationChannel>, AppError>;
    async fn create_channel(&self, req: CreateNotificationChannelRequest) -> Result<NotificationChannel, AppError>;
    async fn update_channel(&self, id: i64, req: UpdateNotificationChannelRequest) -> Result<(), AppError>;
    async fn delete_channel(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteNotificationRepository {
    db: SqlitePool,
}

impl SqliteNotificationRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NotificationRepository for SqliteNotificationRepository {
    async fn list_channels(&self) -> Result<Vec<NotificationChannel>, AppError> {
        sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels ORDER BY created_at DESC"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询通知渠道失败: {e}")))
    }

    async fn find_channel(&self, id: i64) -> Result<Option<NotificationChannel>, AppError> {
        sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询通知渠道失败: {e}")))
    }

    async fn create_channel(&self, req: CreateNotificationChannelRequest) -> Result<NotificationChannel, AppError> {
        let config = serde_json::to_string(&req.config).unwrap_or_default();
        let id = sqlx::query(
            "INSERT INTO notification_channels (name, channel_type, config) VALUES (?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.channel_type)
        .bind(&config)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建通知渠道失败: {e}")))?
        .last_insert_rowid();
        self.find_channel(id).await?.ok_or_else(|| AppError::Internal("创建后查询失败".into()))
    }

    async fn update_channel(&self, id: i64, req: UpdateNotificationChannelRequest) -> Result<(), AppError> {
        if let Some(name) = &req.name {
            sqlx::query("UPDATE notification_channels SET name = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(name).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新通知渠道失败: {e}")))?;
        }
        if let Some(channel_type) = &req.channel_type {
            sqlx::query("UPDATE notification_channels SET channel_type = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(channel_type).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新通知渠道失败: {e}")))?;
        }
        if let Some(config) = &req.config {
            let c = serde_json::to_string(config).unwrap_or_default();
            sqlx::query("UPDATE notification_channels SET config = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&c).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新通知渠道失败: {e}")))?;
        }
        if let Some(enabled) = req.enabled {
            sqlx::query("UPDATE notification_channels SET enabled = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(enabled).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新通知渠道失败: {e}")))?;
        }
        Ok(())
    }

    async fn delete_channel(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM notification_channels WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除通知渠道失败: {e}")))?;
        Ok(())
    }
}

// ─── Alert Rule Repository ──────────────────────────────────────────────────────

#[async_trait]
pub trait AlertRuleRepository: Send + Sync {
    async fn list_rules(&self) -> Result<Vec<AlertRule>, AppError>;
    async fn find_rule(&self, id: i64) -> Result<Option<AlertRule>, AppError>;
    async fn list_enabled_rules(&self) -> Result<Vec<AlertRule>, AppError>;
    async fn create_rule(&self, req: CreateAlertRuleRequest) -> Result<AlertRule, AppError>;
    async fn update_rule(&self, id: i64, req: UpdateAlertRuleRequest) -> Result<(), AppError>;
    async fn delete_rule(&self, id: i64) -> Result<(), AppError>;
    async fn update_last_triggered(&self, id: i64) -> Result<(), AppError>;
}

pub struct SqliteAlertRuleRepository {
    db: SqlitePool,
}

impl SqliteAlertRuleRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AlertRuleRepository for SqliteAlertRuleRepository {
    async fn list_rules(&self) -> Result<Vec<AlertRule>, AppError> {
        sqlx::query_as::<_, AlertRule>(
            "SELECT * FROM alert_rules ORDER BY created_at DESC"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询告警规则失败: {e}")))
    }

    async fn find_rule(&self, id: i64) -> Result<Option<AlertRule>, AppError> {
        sqlx::query_as::<_, AlertRule>("SELECT * FROM alert_rules WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("查询告警规则失败: {e}")))
    }

    async fn list_enabled_rules(&self) -> Result<Vec<AlertRule>, AppError> {
        sqlx::query_as::<_, AlertRule>(
            "SELECT * FROM alert_rules WHERE enabled = 1"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询启用的告警规则失败: {e}")))
    }

    async fn create_rule(&self, req: CreateAlertRuleRequest) -> Result<AlertRule, AppError> {
        let channel_ids = serde_json::to_string(&req.channel_ids).unwrap_or_default();
        let duration = req.duration_seconds.unwrap_or(60);
        let cooldown = req.cooldown_minutes.unwrap_or(5);
        let id = sqlx::query(
            "INSERT INTO alert_rules (name, metric_type, condition, threshold, duration_seconds, channel_ids, cooldown_minutes) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&req.name)
        .bind(&req.metric_type)
        .bind(&req.condition)
        .bind(req.threshold)
        .bind(duration)
        .bind(&channel_ids)
        .bind(cooldown)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建告警规则失败: {e}")))?
        .last_insert_rowid();
        self.find_rule(id).await?.ok_or_else(|| AppError::Internal("创建后查询失败".into()))
    }

    async fn update_rule(&self, id: i64, req: UpdateAlertRuleRequest) -> Result<(), AppError> {
        if let Some(name) = &req.name {
            sqlx::query("UPDATE alert_rules SET name = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(name).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(metric_type) = &req.metric_type {
            sqlx::query("UPDATE alert_rules SET metric_type = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(metric_type).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(condition) = &req.condition {
            sqlx::query("UPDATE alert_rules SET condition = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(condition).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(threshold) = req.threshold {
            sqlx::query("UPDATE alert_rules SET threshold = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(threshold).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(duration_seconds) = req.duration_seconds {
            sqlx::query("UPDATE alert_rules SET duration_seconds = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(duration_seconds).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(channel_ids) = &req.channel_ids {
            let c = serde_json::to_string(channel_ids).unwrap_or_default();
            sqlx::query("UPDATE alert_rules SET channel_ids = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&c).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(enabled) = req.enabled {
            sqlx::query("UPDATE alert_rules SET enabled = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(enabled).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        if let Some(cooldown_minutes) = req.cooldown_minutes {
            sqlx::query("UPDATE alert_rules SET cooldown_minutes = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(cooldown_minutes).bind(id).execute(&self.db).await
                .map_err(|e| AppError::Internal(format!("更新告警规则失败: {e}")))?;
        }
        Ok(())
    }

    async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM alert_rules WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除告警规则失败: {e}")))?;
        Ok(())
    }

    async fn update_last_triggered(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE alert_rules SET last_triggered = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新触发时间失败: {e}")))?;
        Ok(())
    }
}

// ─── Alert History Repository ────────────────────────────────────────────────────

#[async_trait]
pub trait AlertHistoryRepository: Send + Sync {
    async fn create_history(&self, rule_id: i64, rule_name: &str, metric_type: &str, metric_value: f64, threshold: f64, status: &str, message: &str) -> Result<AlertHistory, AppError>;
    async fn list_history(&self, limit: i64) -> Result<Vec<AlertHistory>, AppError>;
    async fn list_history_by_rule(&self, rule_id: i64) -> Result<Vec<AlertHistory>, AppError>;
}

pub struct SqliteAlertHistoryRepository {
    db: SqlitePool,
}

impl SqliteAlertHistoryRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AlertHistoryRepository for SqliteAlertHistoryRepository {
    async fn create_history(&self, rule_id: i64, rule_name: &str, metric_type: &str, metric_value: f64, threshold: f64, status: &str, message: &str) -> Result<AlertHistory, AppError> {
        let id = sqlx::query(
            "INSERT INTO alert_histories (rule_id, rule_name, metric_type, metric_value, threshold, status, message) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(rule_id)
        .bind(rule_name)
        .bind(metric_type)
        .bind(metric_value)
        .bind(threshold)
        .bind(status)
        .bind(message)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建告警历史失败: {e}")))?
        .last_insert_rowid();
        Ok(AlertHistory {
            id,
            rule_id,
            rule_name: rule_name.to_string(),
            metric_type: metric_type.to_string(),
            metric_value,
            threshold,
            status: status.to_string(),
            message: message.to_string(),
            created_at: String::new(),
        })
    }

    async fn list_history(&self, limit: i64) -> Result<Vec<AlertHistory>, AppError> {
        sqlx::query_as::<_, AlertHistory>(
            "SELECT * FROM alert_histories ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询告警历史失败: {e}")))
    }

    async fn list_history_by_rule(&self, rule_id: i64) -> Result<Vec<AlertHistory>, AppError> {
        sqlx::query_as::<_, AlertHistory>(
            "SELECT * FROM alert_histories WHERE rule_id = ? ORDER BY created_at DESC LIMIT 100"
        )
        .bind(rule_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询告警历史失败: {e}")))
    }
}

// ─── Remote Storage ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait RemoteStorage: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError>;
    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}

pub struct S3Storage {
    endpoint: String,
    bucket: String,
    #[allow(dead_code)]
    access_key: String,
    #[allow(dead_code)]
    secret_key: String,
    client: reqwest::Client,
}

impl S3Storage {
    pub fn new(config: &serde_json::Value) -> Result<Self, AppError> {
        let bucket = config["bucket"].as_str().unwrap_or("").to_string();
        let endpoint = config["endpoint"].as_str().unwrap_or("").to_string();
        let access_key = config["access_key"].as_str().unwrap_or("").to_string();
        let secret_key = config["secret_key"].as_str().unwrap_or("").to_string();

        if bucket.is_empty() || endpoint.is_empty() || access_key.is_empty() || secret_key.is_empty() {
            return Err(AppError::Internal("S3 配置不完整：需要 endpoint, bucket, access_key, secret_key".into()));
        }

        Ok(Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            bucket,
            access_key,
            secret_key,
            client: reqwest::Client::new(),
        })
    }

    fn build_url(&self, key: &str) -> String {
        format!("{}/{}/{}", self.endpoint, self.bucket, key)
    }
}

#[async_trait]
impl RemoteStorage for S3Storage {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError> {
        let url = self.build_url(key);
        let response = self.client
            .put(&url)
            .header("x-amz-access-key", &self.access_key)
            .body(data)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 上传失败: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("S3 上传失败: HTTP {status} — {body}")));
        }
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let url = self.build_url(key);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 下载失败: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("S3 下载失败: HTTP {}", response.status())));
        }
        Ok(response.bytes().await.map_err(|e| AppError::Internal(format!("S3 读取响应失败: {e}")))?.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let url = self.build_url(key);
        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 删除失败: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("S3 删除失败: HTTP {}", response.status())));
        }
        Ok(())
    }
}

// ─── Aliyun OSS Storage ─────────────────────────────────────────────────────────

pub struct OssStorage {
    bucket: String,
    endpoint: String,
    access_key_id: String,
    access_key_secret: String,
    client: reqwest::Client,
}

impl OssStorage {
    pub fn new(config: &serde_json::Value) -> Result<Self, AppError> {
        let bucket = config["bucket"].as_str().unwrap_or("").to_string();
        let endpoint = config["endpoint"].as_str().unwrap_or("").to_string();
        let access_key_id = config["access_key_id"].as_str().unwrap_or("").to_string();
        let access_key_secret = config["access_key_secret"].as_str().unwrap_or("").to_string();

        if bucket.is_empty() || endpoint.is_empty() || access_key_id.is_empty() || access_key_secret.is_empty() {
            return Err(AppError::Internal("OSS 配置不完整：需要 bucket, endpoint, access_key_id, access_key_secret".into()));
        }

        Ok(Self {
            bucket, endpoint, access_key_id, access_key_secret,
            client: reqwest::Client::new(),
        })
    }

    fn build_url(&self, key: &str) -> String {
        format!("http://{}.{}/{}", self.bucket, self.endpoint.trim_start_matches("http://").trim_start_matches("https://"), key)
    }

    fn sign(&self, verb: &str, _key: &str, headers: &BTreeMap<String, String>, resource: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        use base64::Engine;

        let content_md5 = headers.get("Content-MD5").map(|s| s.as_str()).unwrap_or("");
        let content_type = headers.get("Content-Type").map(|s| s.as_str()).unwrap_or("");
        let date = headers.get("Date").map(|s| s.as_str()).unwrap_or("");

        let string_to_sign = format!("{verb}\n{content_md5}\n{content_type}\n{date}\n{resource}");
        let mut mac = Hmac::<Sha1>::new_from_slice(self.access_key_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature = mac.finalize().into_bytes();
        base64::engine::general_purpose::STANDARD.encode(signature)
    }
}

#[async_trait]
impl RemoteStorage for OssStorage {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError> {
        let url = self.build_url(key);
        let date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let resource = format!("/{}/{}", self.bucket, key);

        let mut headers = BTreeMap::new();
        headers.insert("Date".to_string(), date.clone());
        headers.insert("Content-Type".to_string(), "application/octet-stream".to_string());

        let signature = self.sign("PUT", key, &headers, &resource);
        let auth = format!("OSS {}:{}", self.access_key_id, signature);

        let response = self.client
            .put(&url)
            .header("Date", &date)
            .header("Content-Type", "application/octet-stream")
            .header("Authorization", &auth)
            .body(data)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OSS 上传失败: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("OSS 上传失败: HTTP {status} — {body}")));
        }
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let url = self.build_url(key);
        let date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let resource = format!("/{}/{}", self.bucket, key);

        let mut headers = BTreeMap::new();
        headers.insert("Date".to_string(), date.clone());

        let signature = self.sign("GET", key, &headers, &resource);
        let auth = format!("OSS {}:{}", self.access_key_id, signature);

        let response = self.client
            .get(&url)
            .header("Date", &date)
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OSS 下载失败: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("OSS 下载失败: HTTP {}", response.status())));
        }
        Ok(response.bytes().await.map_err(|e| AppError::Internal(format!("OSS 读取响应失败: {e}")))?.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let url = self.build_url(key);
        let date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let resource = format!("/{}/{}", self.bucket, key);

        let mut headers = BTreeMap::new();
        headers.insert("Date".to_string(), date.clone());

        let signature = self.sign("DELETE", key, &headers, &resource);
        let auth = format!("OSS {}:{}", self.access_key_id, signature);

        let response = self.client
            .delete(&url)
            .header("Date", &date)
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OSS 删除失败: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("OSS 删除失败: HTTP {}", response.status())));
        }
        Ok(())
    }
}

// ─── Role Repository ──────────────────────────────────────────────────────────

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Role>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError>;
    async fn find_with_permissions(&self, id: i64) -> Result<Option<RoleWithPermissions>, AppError>;
    async fn create(&self, req: &CreateRoleRequest) -> Result<Role, AppError>;
    async fn update(&self, id: i64, req: &UpdateRoleRequest) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn get_user_permissions(&self, role: &str) -> Result<Vec<String>, AppError>;
}

pub struct SqliteRoleRepository {
    db: SqlitePool,
}

impl SqliteRoleRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RoleRepository for SqliteRoleRepository {
    async fn list_all(&self) -> Result<Vec<Role>, AppError> {
        sqlx::query_as::<_, Role>(
            "SELECT id, name, description, is_system, created_at, updated_at FROM roles ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询角色列表失败: {e}")))
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        sqlx::query_as::<_, Role>(
            "SELECT id, name, description, is_system, created_at, updated_at FROM roles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询角色失败: {e}")))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError> {
        sqlx::query_as::<_, Role>(
            "SELECT id, name, description, is_system, created_at, updated_at FROM roles WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询角色失败: {e}")))
    }

    async fn find_with_permissions(&self, id: i64) -> Result<Option<RoleWithPermissions>, AppError> {
        let role = self.find_by_id(id).await?;
        match role {
            None => Ok(None),
            Some(r) => {
                let perms = sqlx::query_as::<_, Permission>(
                    "SELECT p.id, p.name, p.resource, p.action, p.description FROM permissions p INNER JOIN role_permissions rp ON p.id = rp.permission_id WHERE rp.role_id = ? ORDER BY p.id"
                )
                .bind(id)
                .fetch_all(&self.db)
                .await
                .map_err(|e| AppError::Internal(format!("查询角色权限失败: {e}")))?;

                Ok(Some(RoleWithPermissions {
                    id: r.id,
                    name: r.name,
                    description: r.description,
                    is_system: r.is_system,
                    permissions: perms,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                }))
            }
        }
    }

    async fn create(&self, req: &CreateRoleRequest) -> Result<Role, AppError> {
        let role = sqlx::query_as::<_, Role>(
            "INSERT INTO roles (name, description) VALUES (?, ?) RETURNING id, name, description, is_system, created_at, updated_at"
        )
        .bind(&req.name)
        .bind(req.description.as_deref().unwrap_or(""))
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("创建角色失败: {e}")))?;

        for perm_id in &req.permission_ids {
            sqlx::query("INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                .bind(role.id)
                .bind(perm_id)
                .execute(&self.db)
                .await
                .map_err(|e| AppError::Internal(format!("分配权限失败: {e}")))?;
        }

        Ok(role)
    }

    async fn update(&self, id: i64, req: &UpdateRoleRequest) -> Result<(), AppError> {
        let existing = self.find_by_id(id).await?
            .ok_or(AppError::NotFound("角色不存在".into()))?;

        if existing.is_system && req.name.is_some() {
            return Err(AppError::Forbidden);
        }

        if let Some(ref name) = req.name {
            sqlx::query("UPDATE roles SET name = ? WHERE id = ?")
                .bind(name)
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| AppError::Internal(format!("更新角色失败: {e}")))?;
        }
        if let Some(ref desc) = req.description {
            sqlx::query("UPDATE roles SET description = ? WHERE id = ?")
                .bind(desc)
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| AppError::Internal(format!("更新角色失败: {e}")))?;
        }
        if let Some(ref perm_ids) = req.permission_ids {
            sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
                .bind(id)
                .execute(&self.db)
                .await
                .map_err(|e| AppError::Internal(format!("清除旧权限失败: {e}")))?;

            for perm_id in perm_ids {
                sqlx::query("INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                    .bind(id)
                    .bind(perm_id)
                    .execute(&self.db)
                    .await
                    .map_err(|e| AppError::Internal(format!("分配权限失败: {e}")))?;
            }
        }

        sqlx::query("UPDATE roles SET updated_at = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("更新角色时间戳失败: {e}")))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let existing = self.find_by_id(id).await?
            .ok_or(AppError::NotFound("角色不存在".into()))?;
        if existing.is_system {
            return Err(AppError::Forbidden);
        }
        sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("删除角色失败: {e}")))?;
        Ok(())
    }

    async fn get_user_permissions(&self, role: &str) -> Result<Vec<String>, AppError> {
        let perms = sqlx::query_as::<_, (String,)>(
            "SELECT p.name FROM permissions p
             INNER JOIN role_permissions rp ON p.id = rp.permission_id
             INNER JOIN roles r ON r.id = rp.role_id
             WHERE r.name = ?"
        )
        .bind(role)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询用户权限失败: {e}")))?;

        Ok(perms.into_iter().map(|(name,)| name).collect())
    }
}

// ─── Permission Repository ────────────────────────────────────────────────────

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError>;
}

pub struct SqlitePermissionRepository {
    db: SqlitePool,
}

impl SqlitePermissionRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PermissionRepository for SqlitePermissionRepository {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError> {
        sqlx::query_as::<_, Permission>(
            "SELECT id, name, resource, action, description FROM permissions ORDER BY id"
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("查询权限列表失败: {e}")))
    }
}

pub fn create_remote_storage(storage_type: &str, config: &str) -> Result<Arc<dyn RemoteStorage>, AppError> {
    let config: serde_json::Value = serde_json::from_str(config)
        .map_err(|e| AppError::Internal(format!("解析存储配置失败: {e}")))?;
    match storage_type {
        "s3" => Ok(Arc::new(S3Storage::new(&config)?)),
        "oss" => Ok(Arc::new(OssStorage::new(&config)?)),
        _ => Err(AppError::Internal(format!("不支持的远程存储类型: {}", storage_type))),
    }
}
