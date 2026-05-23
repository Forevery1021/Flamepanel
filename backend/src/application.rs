use std::collections::HashMap;
use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use sqlx::SqlitePool;
use tokio::process::Child;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::core::error::AppError;
use crate::domain::{
    CleanupItem, CleanupResult, DashboardInfo, CreateWafRuleRequest, LoadAverage, NetworkInfo,
    NetworkInterface, ServerInfo, UpdateWafRuleRequest, User, WafRule,
};
use crate::infrastructure::{
    AppRepository, CronJobRepository, DatabaseBackupRepository, DatabaseRepository,
    LogRepository, SettingsRepository, SqliteAppRepository, SqliteCronJobRepository,
    SqliteDatabaseBackupRepository, SqliteDatabaseRepository, SqliteLogRepository,
    SqliteSettingsRepository, SqliteUserRepository, SqliteWafIpRuleRepository,
    SqliteWafRuleRepository, SqliteWebsiteRepository, UserRepository,
    WafIpRuleRepository, WafRuleRepository, WebsiteRepository,
};
use crate::metrics::{MetricsHistory, MetricsSnapshot};
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
    pub waf_ip_repo: Arc<dyn WafIpRuleRepository>,
    pub log_repo: Arc<dyn LogRepository>,
    pub settings_repo: Arc<dyn SettingsRepository>,
    pub cron_repo: Arc<dyn CronJobRepository>,
    pub db_repo: Arc<dyn DatabaseRepository>,
    pub db_backup_repo: Arc<dyn DatabaseBackupRepository>,
    pub app_repo: Arc<dyn AppRepository>,
    pub sessions: SessionMap,
    pub metrics_history: Arc<Mutex<MetricsHistory>>,
    pub metrics_tx: broadcast::Sender<MetricsSnapshot>,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        metrics_tx: broadcast::Sender<MetricsSnapshot>,
        metrics_history: Arc<Mutex<MetricsHistory>>,
    ) -> Self {
        Self {
            user_repo: Arc::new(SqliteUserRepository::new(db.clone())),
            website_repo: Arc::new(SqliteWebsiteRepository::new(db.clone())),
            waf_repo: Arc::new(SqliteWafRuleRepository::new(db.clone())),
            waf_ip_repo: Arc::new(SqliteWafIpRuleRepository::new(db.clone())),
            log_repo: Arc::new(SqliteLogRepository::new(db.clone())),
            settings_repo: Arc::new(SqliteSettingsRepository::new(db.clone())),
            cron_repo: Arc::new(SqliteCronJobRepository::new(db.clone())),
            db_repo: Arc::new(SqliteDatabaseRepository::new(db.clone())),
            db_backup_repo: Arc::new(SqliteDatabaseBackupRepository::new(db.clone())),
            app_repo: Arc::new(SqliteAppRepository::new(db.clone())),
            sessions: SessionMap::default(),
            db,
            metrics_tx,
            metrics_history,
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

        let token = create_jwt(&user.username, &user.role, 7 * 24 * 3600)?;
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

fn clamp_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}
fn clamp_f64(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

impl SystemService {
    pub fn get_info() -> ServerInfo {
        use sysinfo::{System, Networks, Disks};

        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = clamp_f32(sys.global_cpu_usage());
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
            disk_total_gb: clamp_f64(disk_total_gb),
            disk_used_gb: clamp_f64(disk_used_gb),
            disk_free_gb: clamp_f64(disk_free_gb),
            uptime_seconds: uptime,
            load_average: LoadAverage {
                one: clamp_f64(load_avg.one),
                five: clamp_f64(load_avg.five),
                fifteen: clamp_f64(load_avg.fifteen),
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

// ─── Cleanup Service ─────────────────────────────────────────────────────────

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}

fn dir_size(path: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(meta) = path.symlink_metadata() {
                    if meta.is_dir() {
                        total += walk(&path);
                    } else {
                        total += meta.len();
                    }
                }
            }
        }
        total
    }
    walk(path)
}

pub struct CleanupService;

impl CleanupService {
    pub fn format_size(bytes: u64) -> String {
        format_bytes(bytes)
    }

    pub async fn scan() -> Vec<CleanupItem> {
        let mut items = Vec::new();

        items.extend(Self::scan_temp_files());
        items.extend(Self::scan_docker_cache().await);
        items.extend(Self::scan_package_cache());
        items.extend(Self::scan_log_files());
        items.extend(Self::scan_cargo_target());
        items.extend(Self::scan_npm_cache());

        items
    }

    fn scan_temp_files() -> Vec<CleanupItem> {
        let mut items = Vec::new();
        let temp_dirs: &[&str] = if cfg!(unix) {
            &["/tmp"]
        } else {
            &[]
        };

        for dir in temp_dirs {
            let path = std::path::Path::new(dir);
            if path.exists() {
                let size = dir_size(path);
                items.push(CleanupItem {
                    category: "temp".into(),
                    name: "系统临时文件".into(),
                    description: "操作系统临时目录中的文件".into(),
                    path: dir.to_string(),
                    size_bytes: size,
                    size_display: format_bytes(size),
                    can_clean: true,
                });
            }
        }
        items
    }

    async fn scan_docker_cache() -> Vec<CleanupItem> {
        let mut items = Vec::new();

        // Docker dangling images
        if let Ok(output) = tokio::process::Command::new("docker")
            .args(["images", "-f", "dangling=true", "--format", "{{.Size}}"])
            .output()
            .await
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let count = text.lines().filter(|l| !l.is_empty()).count();
                if count > 0 {
                    items.push(CleanupItem {
                        category: "docker".into(),
                        name: "Docker 悬空镜像".into(),
                        description: format!("{} 个无标签的悬空镜像", count),
                        path: "docker image prune".into(),
                        size_bytes: 0,
                        size_display: format!("{} 个镜像", count),
                        can_clean: true,
                    });
                }
            }
        }

        // Docker build cache
        if let Ok(output) = tokio::process::Command::new("docker")
            .args(["builder", "prune", "--force", "--keep-storage", "0"])
            .output()
            .await
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = text.lines().find(|l| l.contains("Total:")) {
                    items.push(CleanupItem {
                        category: "docker".into(),
                        name: "Docker 构建缓存".into(),
                        description: "Docker build 缓存层".into(),
                        path: "docker builder prune".into(),
                        size_bytes: 0,
                        size_display: line.replace("Total:", "").trim().to_string(),
                        can_clean: true,
                    });
                }
            }
        }

        // Stopped containers
        if let Ok(output) = tokio::process::Command::new("docker")
            .args(["ps", "-a", "-f", "status=exited", "--format", "{{.ID}}"])
            .output()
            .await
        {
            if output.status.success() {
                let count = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count();
                if count > 0 {
                    items.push(CleanupItem {
                        category: "docker".into(),
                        name: "已停止容器".into(),
                        description: format!("{} 个已退出的容器", count),
                        path: "docker container prune".into(),
                        size_bytes: 0,
                        size_display: format!("{} 个容器", count),
                        can_clean: true,
                    });
                }
            }
        }

        items
    }

    fn scan_package_cache() -> Vec<CleanupItem> {
        let mut items = Vec::new();

        if cfg!(unix) {
            // apt cache
            let apt_cache = std::path::Path::new("/var/cache/apt/archives");
            if apt_cache.exists() {
                let size = dir_size(apt_cache);
                if size > 0 {
                    items.push(CleanupItem {
                        category: "package".into(),
                        name: "APT 包缓存".into(),
                        description: "apt 下载的 deb 包缓存文件".into(),
                        path: "/var/cache/apt/archives".to_string(),
                        size_bytes: size,
                        size_display: format_bytes(size),
                        can_clean: true,
                    });
                }
            }

            // yum/dnf cache
            for cache_dir in &["/var/cache/yum", "/var/cache/dnf"] {
                let path = std::path::Path::new(cache_dir);
                if path.exists() {
                    let size = dir_size(path);
                    if size > 0 {
                        items.push(CleanupItem {
                            category: "package".into(),
                            name: "YUM/DNF 包缓存".into(),
                            description: "RPM 包管理器缓存".into(),
                            path: cache_dir.to_string(),
                            size_bytes: size,
                            size_display: format_bytes(size),
                            can_clean: true,
                        });
                    }
                }
            }
        }

        // pip cache
        let home = std::env::var("HOME").unwrap_or_else(|_| String::new());
        let pip_cache = std::path::Path::new(&home).join(".cache/pip");
        if pip_cache.exists() {
            let size = dir_size(&pip_cache);
            if size > 0 {
                items.push(CleanupItem {
                    category: "package".into(),
                    name: "pip 缓存".into(),
                    description: "Python pip 下载缓存".into(),
                    path: pip_cache.to_string_lossy().to_string(),
                    size_bytes: size,
                    size_display: format_bytes(size),
                    can_clean: true,
                });
            }
        }

        items
    }

    fn scan_log_files() -> Vec<CleanupItem> {
        let mut items = Vec::new();

        if cfg!(unix) {
            // systemd journal
            let journal = std::path::Path::new("/var/log/journal");
            if journal.exists() {
                let size = dir_size(journal);
                if size > 0 {
                    items.push(CleanupItem {
                        category: "logs".into(),
                        name: "systemd 日志".into(),
                        description: "systemd journal 日志文件".into(),
                        path: "/var/log/journal".to_string(),
                        size_bytes: size,
                        size_display: format_bytes(size),
                        can_clean: true,
                    });
                }
            }

            // Regular log files in /var/log
            let var_log = std::path::Path::new("/var/log");
            if var_log.exists() {
                if let Ok(entries) = std::fs::read_dir(var_log) {
                    let mut total_size = 0u64;
                    let mut count = 0u64;
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Ok(meta) = path.symlink_metadata() {
                            if meta.is_file() &&
                               path.extension().map_or(false, |e| e == "gz" || e == "old") {
                                total_size += meta.len();
                                count += 1;
                            }
                        }
                    }
                    if total_size > 0 {
                        items.push(CleanupItem {
                            category: "logs".into(),
                            name: "旧日志文件".into(),
                            description: format!("{} 个轮转/压缩的旧日志文件", count),
                            path: "/var/log".to_string(),
                            size_bytes: total_size,
                            size_display: format_bytes(total_size),
                            can_clean: true,
                        });
                    }
                }
            }
        }

        items
    }

    fn scan_cargo_target() -> Vec<CleanupItem> {
        let mut items = Vec::new();

        // Look for Rust target directories
        let home = std::env::var("HOME").unwrap_or_else(|_| String::new());
        for check in &[
            std::path::PathBuf::from("target"),
            std::path::PathBuf::from(&home).join(".cargo/registry/cache"),
        ] {
            if check.exists() {
                let size = dir_size(&check);
                if size > 0 {
                    items.push(CleanupItem {
                        category: "dev".into(),
                        name: "Rust 构建产物".into(),
                        description: "cargo build target 目录".into(),
                        path: check.to_string_lossy().to_string(),
                        size_bytes: size,
                        size_display: format_bytes(size),
                        can_clean: true,
                    });
                }
            }
        }

        items
    }

    fn scan_npm_cache() -> Vec<CleanupItem> {
        let mut items = Vec::new();

        let home = std::env::var("HOME").unwrap_or_else(|_| String::new());
        let npm_cache = std::path::Path::new(&home).join(".npm/_cacache");
        if npm_cache.exists() {
            let size = dir_size(&npm_cache);
            if size > 0 {
                items.push(CleanupItem {
                    category: "dev".into(),
                    name: "npm 缓存".into(),
                    description: "Node.js npm 包缓存".into(),
                    path: npm_cache.to_string_lossy().to_string(),
                    size_bytes: size,
                    size_display: format_bytes(size),
                    can_clean: true,
                });
            }
        }

        items
    }

    pub async fn clean(categories: &[String]) -> CleanupResult {
        let mut cleaned = Vec::new();
        let mut errors = Vec::new();
        let mut freed = 0u64;

        for category in categories {
            match category.as_str() {
                "temp" => match Self::clean_temp_files() {
                    Ok(n) => { freed += n; cleaned.push("系统临时文件已清理".into()); }
                    Err(e) => errors.push(format!("临时文件清理失败: {e}")),
                },
                "docker" => match Self::clean_docker().await {
                    Ok(msg) => cleaned.push(msg),
                    Err(e) => errors.push(format!("Docker 清理失败: {e}")),
                },
                "package" => match Self::clean_package_cache() {
                    Ok(n) => { freed += n; cleaned.push("包管理器缓存已清理".into()); }
                    Err(e) => errors.push(format!("包缓存清理失败: {e}")),
                },
                "logs" => match Self::clean_log_files() {
                    Ok(n) => { freed += n; cleaned.push("旧日志文件已清理".into()); }
                    Err(e) => errors.push(format!("日志清理失败: {e}")),
                },
                "dev" => match Self::clean_dev_artifacts() {
                    Ok(n) => { freed += n; cleaned.push("开发构建产物已清理".into()); }
                    Err(e) => errors.push(format!("构建产物清理失败: {e}")),
                },
                _ => errors.push(format!("未知的清理类别: {category}")),
            }
        }

        CleanupResult {
            cleaned_items: cleaned,
            freed_bytes: freed,
            freed_display: format_bytes(freed),
            errors,
        }
    }

    fn clean_temp_files() -> Result<u64, String> {
        let path = std::path::Path::new("/tmp");
        if !path.exists() {
            return Ok(0);
        }
        let original = dir_size(path);
        // Only remove files older than 1 day to be safe
        let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(86400);
        Self::clean_dir_older_than(path, cutoff)?;
        let after = dir_size(path);
        Ok(original.saturating_sub(after))
    }

    async fn clean_docker() -> Result<String, String> {
        let cmds = [
            vec!["container", "prune", "-f"],
            vec!["image", "prune", "-f"],
            vec!["builder", "prune", "-f"],
        ];

        let mut count = 0;
        for args in cmds.iter() {
            let output = tokio::process::Command::new("docker")
                .args(args)
                .output()
                .await
                .map_err(|e| format!("执行 docker prune 失败: {e}"))?;
            if output.status.success() {
                count += 1;
            }
        }
        Ok(format!("Docker 清理完成 ({} 项)", count))
    }

    fn clean_package_cache() -> Result<u64, String> {
        let mut freed = 0u64;
        for dir in &[
            "/var/cache/apt/archives",
            "/var/cache/yum",
            "/var/cache/dnf",
        ] {
            let path = std::path::Path::new(dir);
            if path.exists() {
                freed += dir_size(path);
                let _ = std::fs::remove_dir_all(path);
            }
        }
        // pip cache
        if let Ok(home) = std::env::var("HOME") {
            let pip_cache = std::path::Path::new(&home).join(".cache/pip");
            if pip_cache.exists() {
                freed += dir_size(&pip_cache);
                let _ = std::fs::remove_dir_all(&pip_cache);
            }
        }
        Ok(freed)
    }

    fn clean_log_files() -> Result<u64, String> {
        let mut freed = 0u64;
        if cfg!(unix) {
            let var_log = std::path::Path::new("/var/log");
            if var_log.exists() {
                if let Ok(entries) = std::fs::read_dir(var_log) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Ok(meta) = path.symlink_metadata() {
                            if meta.is_file() &&
                               path.extension().map_or(false, |e| e == "gz" || e == "old") {
                                freed += meta.len();
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
        Ok(freed)
    }

    fn clean_dev_artifacts() -> Result<u64, String> {
        let mut freed = 0u64;
        // Clean local target directory
        let target = std::path::Path::new("target");
        if target.exists() {
            freed += dir_size(target);
            let _ = std::fs::remove_dir_all(target);
        }
        // npm cache
        if let Ok(home) = std::env::var("HOME") {
            let npm_cache = std::path::Path::new(&home).join(".npm/_cacache");
            if npm_cache.exists() {
                freed += dir_size(&npm_cache);
                let _ = std::fs::remove_dir_all(&npm_cache);
            }
        }
        Ok(freed)
    }

    fn clean_dir_older_than(dir: &std::path::Path, cutoff: std::time::SystemTime) -> Result<(), String> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(meta) = path.symlink_metadata() {
                    if let Ok(modified) = meta.modified() {
                        if modified < cutoff {
                            if meta.is_dir() {
                                let _ = std::fs::remove_dir_all(&path);
                            } else {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// ─── Cron 调度服务 ─────────────────────────────────────────────────────────

pub struct CronService;

impl CronService {
    /// Check if a cron expression matches the given time components.
    fn cron_matches(expr: &str, min: u32, hour: u32, dom: u32, month: u32, dow: u32) -> bool {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return false;
        }
        let vals = [min, hour, dom, month, dow];
        for (i, field) in fields.iter().enumerate() {
            if !Self::field_matches(field, vals[i]) {
                return false;
            }
        }
        true
    }

    fn field_matches(field: &str, val: u32) -> bool {
        if field == "*" {
            return true;
        }
        // */N step
        if let Some(step_str) = field.strip_prefix("*/") {
            if let Ok(step) = step_str.parse::<u32>() {
                return step > 0 && val % step == 0;
            }
        }
        // Comma-separated list
        for part in field.split(',') {
            // Range N-M
            if let Some((lo_str, hi_str)) = part.split_once('-') {
                if let (Ok(lo), Ok(hi)) = (lo_str.parse::<u32>(), hi_str.parse::<u32>()) {
                    if val >= lo && val <= hi {
                        return true;
                    }
                }
            } else if let Ok(n) = part.parse::<u32>() {
                if val == n {
                    return true;
                }
            }
        }
        false
    }

    /// Compute the next scheduled run time from a cron expression.
    fn next_run_from(expr: &str, now: chrono::DateTime<chrono::Utc>) -> String {
        // Walk forward minute by minute (up to 366 days) to find next match
        let max_steps = 366 * 24 * 60; // 1 year
        let mut cur = now + chrono::Duration::minutes(1);
        for _ in 0..max_steps {
            let min = cur.format("%M").to_string().parse::<u32>().unwrap_or(0);
            let hr = cur.format("%H").to_string().parse::<u32>().unwrap_or(0);
            let dom = cur.format("%d").to_string().parse::<u32>().unwrap_or(1);
            let mon = cur.format("%m").to_string().parse::<u32>().unwrap_or(1);
            let dow = cur.format("%u").to_string().parse::<u32>().unwrap_or(0);
            if Self::cron_matches(expr, min, hr, dom, mon, dow) {
                return cur.format("%Y-%m-%d %H:%M:%S").to_string();
            }
            cur = cur + chrono::Duration::minutes(1);
        }
        // Fallback
        (now + chrono::Duration::hours(1)).format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Recalculate next_run for a cron job.
    pub async fn recalc_next_run(repo: Arc<dyn CronJobRepository>, job_id: i64) -> Result<(), AppError> {
        if let Some(job) = repo.find_by_id(job_id).await? {
            let now = chrono::Utc::now();
            let next = Self::next_run_from(&job.schedule, now);
            repo.update_run_time(job.id, &job.last_run.unwrap_or_default(), &next).await?;
        }
        Ok(())
    }

    /// Execute a single cron job synchronously, returning (status, output).
    pub async fn execute_job(job: &crate::domain::CronJob) -> (String, Option<String>) {
        if let Some(cmd) = &job.command {
            let output = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                .arg(if cfg!(windows) { "/C" } else { "-c" })
                .arg(cmd)
                .output()
                .await;
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = if stderr.is_empty() { stdout } else { format!("{stdout}\n{stderr}") };
                    if out.status.success() {
                        ("success".into(), Some(combined))
                    } else {
                        ("failed".into(), Some(combined))
                    }
                }
                Err(e) => ("failed".into(), Some(format!("执行失败: {e}"))),
            }
        } else if let Some(url) = &job.url {
            match reqwest::get(url).await {
                Ok(resp) => {
                    let status_code = resp.status().as_u16();
                    match resp.text().await {
                        Ok(body) => {
                            if status_code < 400 {
                                ("success".into(), Some(format!("HTTP {status_code}\n{body}")))
                            } else {
                                ("failed".into(), Some(format!("HTTP {status_code}\n{body}")))
                            }
                        }
                        Err(e) => ("failed".into(), Some(format!("读取响应失败: {e}"))),
                    }
                }
                Err(e) => ("failed".into(), Some(format!("请求失败: {e}"))),
            }
        } else {
            ("failed".into(), Some("无 command 或 url".into()))
        }
    }

    /// Background scheduler: checks every 30s for jobs that need to run.
    pub fn spawn_scheduler(state: AppState) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let jobs = match state.cron_repo.list_enabled().await {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                let now = chrono::Utc::now();
                let current_min = now.format("%M").to_string().parse::<u32>().unwrap_or(0);
                let current_hr = now.format("%H").to_string().parse::<u32>().unwrap_or(0);
                // Check if minute just changed (run within first 30s of a new minute)
                if now.format("%S").to_string().parse::<u32>().unwrap_or(0) > 30 {
                    // We're past the 30s mark; check for jobs that match prev minute too
                }

                for job in &jobs {
                    let dom = now.format("%d").to_string().parse::<u32>().unwrap_or(1);
                    let mon = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
                    let dow = now.format("%u").to_string().parse::<u32>().unwrap_or(0);
                    if !Self::cron_matches(&job.schedule, current_min, current_hr, dom, mon, dow) {
                        continue;
                    }
                    // Avoid running more than once per minute
                    if let Some(ref last) = job.last_run {
                        let last_min = &last[..16]; // "YYYY-MM-DD HH:MM"
                        let now_min = &now.format("%Y-%m-%d %H:%M").to_string();
                        if last_min == now_min {
                            continue;
                        }
                    }

                    let started_at = now.format("%Y-%m-%d %H:%M:%S").to_string();
                    let (status, output) = Self::execute_job(job).await;
                    let finished_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let _ = state.cron_repo.log(job.id, &status, output.as_deref(), &started_at, &finished_at).await;
                    let next = Self::next_run_from(&job.schedule, now);
                    let _ = state.cron_repo.update_run_time(job.id, &started_at, &next).await;
                }
            }
        });
    }
}
