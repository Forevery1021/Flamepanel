use std::collections::HashMap;
use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use sqlx::SqlitePool;
use tokio::process::Child;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::core::error::AppError;
use crate::domain::{
    DashboardInfo, CreateWafRuleRequest, LoadAverage, NetworkInfo, NetworkInterface,
    OperationLogEntry, ServerInfo, UpdateWafRuleRequest, User, WafRule,
};
use crate::infrastructure::{
    LogRepository, SqliteLogRepository, SqliteUserRepository, SqliteWafRuleRepository,
    SqliteWebsiteRepository, UserRepository, WafRuleRepository, WebsiteRepository,
};
use crate::middleware::auth::create_jwt;

// ─── AppState ─────────────────────────────────────────────────────────────────

pub struct SessionHandle {
    pub child: Child,
    pub cols: u16,
    pub rows: u16,
}

pub type SessionMap = Arc<Mutex<HashMap<String, Arc<Mutex<SessionHandle>>>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub user_repo: Arc<dyn UserRepository>,
    pub website_repo: Arc<dyn WebsiteRepository>,
    pub waf_repo: Arc<dyn WafRuleRepository>,
    pub log_repo: Arc<dyn LogRepository>,
    pub sessions: SessionMap,
}

impl AppState {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            user_repo: Arc::new(SqliteUserRepository::new(db.clone())),
            website_repo: Arc::new(SqliteWebsiteRepository::new(db.clone())),
            waf_repo: Arc::new(SqliteWafRuleRepository::new(db.clone())),
            log_repo: Arc::new(SqliteLogRepository::new(db.clone())),
            sessions: SessionMap::default(),
            db,
        }
    }
}

// ─── 初始化管理员账号 ─────────────────────────────────────────────────────────

pub async fn seed_admin(db: &SqlitePool, config: &Config) -> Result<(), AppError> {
    let repo = SqliteUserRepository::new(db.clone());

    if repo.find_by_username(&config.admin_username).await?.is_some() {
        tracing::info!("管理员账号 '{}' 已存在，跳过初始化", config.admin_username);
        return Ok(());
    }

    let password = config.admin_password.clone();
    let password_hash = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
        .await
        .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
        .map_err(|e| AppError::Internal(format!("密码哈希失败: {e}")))?;

    repo.create(&config.admin_username, &password_hash, "admin").await?;

    tracing::info!("管理员账号 '{}' 初始化完成", config.admin_username);
    Ok(())
}

// ─── Auth Service ─────────────────────────────────────────────────────────────

pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
}

impl AuthService {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<(String, User), AppError> {
        let user = self.user_repo.find_by_username(username)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let hash = user.password_hash.clone();
        let plain = password.to_string();
        let valid = tokio::task::spawn_blocking(move || verify(&plain, &hash))
            .await
            .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
            .map_err(|_| AppError::Unauthorized)?;

        if !valid {
            return Err(AppError::Unauthorized);
        }

        self.user_repo.update_last_login(user.id).await.ok();

        let token = create_jwt(&user.username, 7 * 24 * 3600)?;
        Ok((token, user))
    }

    pub async fn register(&self, username: &str, password: &str, role: &str) -> Result<User, AppError> {
        if username.len() < 3 || password.len() < 6 {
            return Err(AppError::BadRequest("用户名至少3位，密码至少6位".into()));
        }

        if self.user_repo.find_by_username(username).await?.is_some() {
            return Err(AppError::BadRequest("用户名已存在".into()));
        }

        let plain = password.to_string();
        let password_hash = tokio::task::spawn_blocking(move || hash(plain, DEFAULT_COST))
            .await
            .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
            .map_err(|e| AppError::Internal(format!("密码哈希失败: {e}")))?;

        let user = self.user_repo.create(username, &password_hash, role).await?;
        Ok(user)
    }

    pub async fn change_password(&self, user_id: i64, old_password: &str, new_password: &str) -> Result<(), AppError> {
        if new_password.len() < 6 {
            return Err(AppError::BadRequest("新密码至少6位".into()));
        }

        let user = self.user_repo.find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("用户不存在".into()))?;

        let pw_hash = user.password_hash.clone();
        let old = old_password.to_string();
        let valid = tokio::task::spawn_blocking(move || verify(&old, &pw_hash))
            .await
            .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
            .map_err(|_| AppError::BadRequest("旧密码不正确".into()))?;

        if !valid {
            return Err(AppError::BadRequest("旧密码不正确".into()));
        }

        let new_plain = new_password.to_string();
        let new_hash = tokio::task::spawn_blocking(move || hash(new_plain, DEFAULT_COST))
            .await
            .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
            .map_err(|e| AppError::Internal(format!("密码哈希失败: {e}")))?;

        self.user_repo.update_password(user_id, &new_hash).await
    }
}

// ─── System Service ───────────────────────────────────────────────────────────

pub struct SystemService;

impl SystemService {
    pub fn get_info() -> ServerInfo {
        use sysinfo::{System, Networks, Disks};

        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let cpu_cores = sys.cpus().len();
        let memory_total = sys.total_memory() / 1024 / 1024;
        let memory_used = sys.used_memory() / 1024 / 1024;
        let uptime = System::uptime();

        let disks = Disks::new_with_refreshed_list();
        let (disk_total, disk_used) = disks.iter().fold((0u64, 0u64), |(total, used), disk| {
            (total + disk.total_space(), used + disk.total_space() - disk.available_space())
        });
        let disk_total_gb = disk_total as f64 / 1024.0 / 1024.0 / 1024.0;
        let disk_used_gb = disk_used as f64 / 1024.0 / 1024.0 / 1024.0;
        let disk_free_gb = disk_total_gb - disk_used_gb;

        let networks = Networks::new_with_refreshed_list();
        let hostname = System::host_name().unwrap_or_else(|| "unknown".into());
        let interfaces: Vec<NetworkInterface> = networks.iter().map(|(name, data)| {
            NetworkInterface {
                name: name.clone(),
                ipv4: data.ip_networks().iter()
                    .filter(|ip| ip.addr.is_ipv4())
                    .map(|ip| ip.addr.to_string())
                    .collect(),
                ipv6: data.ip_networks().iter()
                    .filter(|ip| ip.addr.is_ipv6())
                    .map(|ip| ip.addr.to_string())
                    .collect(),
                mac: data.mac_address().to_string(),
            }
        }).collect();

        let load_avg = System::load_average();

        ServerInfo {
            cpu_usage,
            cpu_cores,
            memory_total_mb: memory_total,
            memory_used_mb: memory_used,
            memory_free_mb: memory_total - memory_used,
            disk_total_gb,
            disk_used_gb,
            disk_free_gb,
            uptime_seconds: uptime,
            load_average: LoadAverage {
                one: load_avg.one,
                five: load_avg.five,
                fifteen: load_avg.fifteen,
            },
            network: NetworkInfo {
                hostname,
                interfaces,
            },
        }
    }

    pub fn get_processes() -> Vec<ProcessInfo> {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut processes: Vec<ProcessInfo> = sys.processes().iter()
            .map(|(pid, proc)| ProcessInfo {
                pid: pid.as_u32(),
                name: proc.name().to_string_lossy().into(),
                cpu_usage: proc.cpu_usage(),
                memory_mb: proc.memory() / 1024 / 1024,
                status: proc.status().to_string(),
            })
            .collect();

        processes.sort_by(|a, b| b.memory_mb.partial_cmp(&a.memory_mb).unwrap_or(std::cmp::Ordering::Equal));
        processes.truncate(50);
        processes
    }
}

#[derive(serde::Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: u64,
    pub status: String,
}

// ─── Dashboard Service ────────────────────────────────────────────────────────

pub struct DashboardService {
    pub website_repo: Arc<dyn WebsiteRepository>,
    pub waf_repo: Arc<dyn WafRuleRepository>,
    pub db: SqlitePool,
}

impl DashboardService {
    pub fn new(
        website_repo: Arc<dyn WebsiteRepository>,
        waf_repo: Arc<dyn WafRuleRepository>,
        db: SqlitePool,
    ) -> Self {
        Self { website_repo, waf_repo, db }
    }

    pub async fn get_dashboard(&self) -> Result<DashboardInfo, AppError> {
        let system = SystemService::get_info();

        let websites = self.website_repo.list().await.unwrap_or_default();
        let websites_total = websites.len() as i64;
        let websites_running = websites.iter().filter(|w| w.enabled).count() as i64;

        let (docker_running, docker_total) = Self::get_docker_stats().await;

        let (waf_total, waf_enabled) = self.waf_repo.count().await.unwrap_or((0, 0));

        let log_repo = SqliteLogRepository::new(self.db.clone());
        let recent_logs = log_repo.list_recent(20).await.unwrap_or_default();

        Ok(DashboardInfo {
            server_info: system,
            docker_containers_running: docker_running,
            docker_containers_total: docker_total,
            websites_running,
            websites_total,
            recent_logs,
            waf_rules_count: waf_total,
            waf_rules_enabled: waf_enabled,
        })
    }

    async fn get_docker_stats() -> (i64, i64) {
        let output = match tokio::process::Command::new("docker")
            .args(["ps", "-a", "--format", "{{.State}}"])
            .output()
            .await
        {
            Ok(o) if o.status.success() => o,
            _ => return (0, 0),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let total = stdout.lines().filter(|l| !l.is_empty()).count() as i64;
        let running = stdout.lines().filter(|l| l.trim() == "running").count() as i64;
        (running, total)
    }
}

// ─── WAF Service ──────────────────────────────────────────────────────────────

pub struct WafService {
    waf_repo: Arc<dyn WafRuleRepository>,
}

impl WafService {
    pub fn new(waf_repo: Arc<dyn WafRuleRepository>) -> Self {
        Self { waf_repo }
    }

    pub async fn list_rules(&self) -> Result<Vec<WafRule>, AppError> {
        self.waf_repo.list_all().await
    }

    pub async fn get_rule(&self, id: i64) -> Result<WafRule, AppError> {
        self.waf_repo.find_by_id(id)
            .await?
            .ok_or(AppError::NotFound("WAF 规则不存在".into()))
    }

    pub async fn create_rule(&self, req: CreateWafRuleRequest) -> Result<WafRule, AppError> {
        if req.name.is_empty() || req.pattern.is_empty() {
            return Err(AppError::BadRequest("规则名称和匹配模式不能为空".into()));
        }

        Self::validate_regex(&req.pattern)?;
        Self::validate_target(&req.target)?;
        Self::validate_action(&req.action)?;

        self.waf_repo.create(&req).await
    }

    pub async fn update_rule(&self, id: i64, req: UpdateWafRuleRequest) -> Result<(), AppError> {
        if let Some(ref pattern) = req.pattern {
            Self::validate_regex(pattern)?;
        }
        if let Some(ref target) = req.target {
            Self::validate_target(target)?;
        }
        if let Some(ref action) = req.action {
            Self::validate_action(action)?;
        }

        self.waf_repo.update(id, &req).await
    }

    pub async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        self.waf_repo.delete(id).await
    }

    pub async fn toggle_rule(&self, id: i64, enabled: bool) -> Result<(), AppError> {
        let req = UpdateWafRuleRequest {
            name: None,
            pattern: None,
            target: None,
            action: None,
            description: None,
            enabled: Some(enabled),
        };
        self.waf_repo.update(id, &req).await
    }

    fn validate_regex(pattern: &str) -> Result<(), AppError> {
        Regex::new(pattern)
            .map_err(|e| AppError::BadRequest(format!("正则表达式无效: {e}")))?;
        Ok(())
    }

    fn validate_target(target: &str) -> Result<(), AppError> {
        match target {
            "url" | "header" | "body" | "cookie" => Ok(()),
            _ => Err(AppError::BadRequest("target 必须为 url/header/body/cookie".into())),
        }
    }

    fn validate_action(action: &str) -> Result<(), AppError> {
        match action {
            "block" | "allow" | "log" => Ok(()),
            _ => Err(AppError::BadRequest("action 必须为 block/allow/log".into())),
        }
    }
}
