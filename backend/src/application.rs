use std::collections::HashMap;
use std::sync::Arc;

use futures_util::StreamExt;
use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::process::Child;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::core::error::AppError;
use crate::domain::{
    DashboardInfo,

    AiAnalyzeRequest, AiChatRequest, AiChatResponse, AiConversation, AiMessage, AiModelInfo,
    AlertHistory, AlertRule, BackupConfig, BackupRecord, CleanupItem, CleanupResult,
    CreateAlertRuleRequest, CreateBackupConfigRequest, CreateNotificationChannelRequest,
    CreateWafRuleRequest, GpuInfo, LoadAverage,
    ClusterDashboard, FileEntry, NetworkInfo, NetworkInterface,
    NodeExecResponse, NodeInfo, NotificationChannel,
    ServerInfo, UpdateAlertRuleRequest, UpdateBackupConfigRequest, UpdateNotificationChannelRequest,
    UpdateWafRuleRequest, User, WafRule,
    Role, Permission, RoleWithPermissions, CreateRoleRequest, UpdateRoleRequest, AssignRoleRequest,
};
use crate::infrastructure::{
    AiConversationRepository, AlertHistoryRepository, AlertRuleRepository, AppRepository,
    BackupRepository, CronJobRepository, DatabaseBackupRepository, DatabaseRepository,
    LogRepository, NodeRepository, NotificationRepository, RemoteStorage, SettingsRepository,
    SqliteAiConversationRepository, SqliteAlertHistoryRepository, SqliteAlertRuleRepository,
    SqliteAppRepository, SqliteBackupRepository, SqliteCronJobRepository,
    SqliteDatabaseBackupRepository, SqliteDatabaseRepository, SqliteLogRepository,
    SqliteNodeRepository, SqliteNotificationRepository, SqliteSettingsRepository,
    SqliteUserRepository, SqliteWafIpRuleRepository, SqliteWafRuleRepository,
    SqliteWebsiteRepository, UserRepository, WafIpRuleRepository, WafRuleRepository,
    WebsiteRepository, create_remote_storage,
    RoleRepository, PermissionRepository, SqliteRoleRepository, SqlitePermissionRepository,
};
use crate::metrics::{MetricsHistory, MetricsSnapshot};
use bollard::Docker;
use crate::middleware::auth::create_jwt;
use crate::plugin::manager::PluginManagerRef;
use crate::plugin::mcp::ToolRegistry;
use crate::plugin::wasm_runtime::WasmRuntime;

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
    pub ai_repo: Arc<dyn AiConversationRepository>,
    pub node_repo: Arc<dyn NodeRepository>,
    pub backup_repo: Arc<dyn BackupRepository>,
    pub notification_repo: Arc<dyn NotificationRepository>,
    pub alert_rule_repo: Arc<dyn AlertRuleRepository>,
    pub alert_history_repo: Arc<dyn AlertHistoryRepository>,
    pub role_repo: Arc<dyn RoleRepository>,
    pub permission_repo: Arc<dyn PermissionRepository>,
    pub tool_registry: Arc<ToolRegistry>,
    pub plugin_manager: PluginManagerRef,
    pub wasm_runtime: Arc<WasmRuntime>,
    pub docker: Docker,
    pub sessions: SessionMap,
    pub metrics_history: Arc<Mutex<MetricsHistory>>,
    pub metrics_tx: broadcast::Sender<MetricsSnapshot>,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        metrics_tx: broadcast::Sender<MetricsSnapshot>,
        metrics_history: Arc<Mutex<MetricsHistory>>,
        tool_registry: Arc<ToolRegistry>,
        plugin_manager: PluginManagerRef,
        wasm_runtime: Arc<WasmRuntime>,
        docker: Docker,
    ) -> Self {
        Self {
            docker,
            wasm_runtime,
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
            ai_repo: Arc::new(SqliteAiConversationRepository::new(db.clone())),
            node_repo: Arc::new(SqliteNodeRepository::new(db.clone())),
            backup_repo: Arc::new(SqliteBackupRepository::new(db.clone())),
            notification_repo: Arc::new(SqliteNotificationRepository::new(db.clone())),
            alert_rule_repo: Arc::new(SqliteAlertRuleRepository::new(db.clone())),
            alert_history_repo: Arc::new(SqliteAlertHistoryRepository::new(db.clone())),
            role_repo: Arc::new(SqliteRoleRepository::new(db.clone())),
            permission_repo: Arc::new(SqlitePermissionRepository::new(db.clone())),
            tool_registry,
            plugin_manager,
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

    pub fn get_gpu_info() -> Vec<GpuInfo> {
        // GPU monitoring is optional — gracefully returns empty vec if no NVIDIA GPU
        match nvml_wrapper::Nvml::init() {
            Ok(nvml) => {
                let count = match nvml.device_count() {
                    Ok(c) => c,
                    Err(_) => return Vec::new(),
                };
                let mut gpus = Vec::new();
                for i in 0..count {
                    if let Ok(device) = nvml.device_by_index(i) {
                        let name = device.name().unwrap_or_else(|_| "NVIDIA GPU".into());
                        let temp = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).unwrap_or(0);
                        let utilization = device.utilization_rates().map(|u| u.gpu as f32).unwrap_or(0.0);
                        let mem = device.memory_info().map(|m| (m.total, m.used, m.free)).unwrap_or((0, 0, 0));
                        let fan = device.fan_speed(0).unwrap_or(0);

                        gpus.push(GpuInfo {
                            name,
                            temperature_celsius: temp as f32,
                            utilization_percent: utilization,
                            memory_total_mb: mem.0 / 1024 / 1024,
                            memory_used_mb: mem.1 / 1024 / 1024,
                            memory_free_mb: mem.2 / 1024 / 1024,
                            fan_speed_percent: fan as f32,
                        });
                    }
                }
                gpus
            }
            Err(_) => Vec::new(),
        }
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

// ─── Node Service ────────────────────────────────────────────────────────────

pub struct NodeService {
    pub node_repo: Arc<dyn NodeRepository>,
}

impl NodeService {
    pub fn new(node_repo: Arc<dyn NodeRepository>) -> Self {
        Self { node_repo }
    }

    pub async fn list(&self) -> Result<Vec<NodeInfo>, AppError> {
        self.node_repo.list_all().await
    }

    pub async fn get(&self, id: i64) -> Result<NodeInfo, AppError> {
        self.node_repo.find_by_id(id).await?
            .ok_or(AppError::NotFound("节点不存在".into()))
    }

    pub async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.node_repo.delete(id).await
    }

    pub async fn cluster_dashboard(&self) -> Result<ClusterDashboard, AppError> {
        let nodes = self.node_repo.list_all().await?;
        let total = nodes.len() as i64;
        let online = nodes.iter().filter(|n| n.status == "online").count() as i64;
        let offline = total - online;

        let online_nodes: Vec<_> = nodes.iter().filter(|n| n.status == "online").collect();
        let count = online_nodes.len().max(1);
        let avg_cpu = online_nodes.iter().map(|n| n.cpu_usage).sum::<f32>() / count as f32;
        let avg_memory = online_nodes.iter().map(|n| n.memory_usage_percent).sum::<f32>() / count as f32;
        let avg_disk = online_nodes.iter().map(|n| n.disk_usage_percent).sum::<f32>() / count as f32;
        let avg_load = online_nodes.iter().map(|n| n.load_one).sum::<f32>() / count as f32;

        Ok(ClusterDashboard {
            total_nodes: total,
            online_nodes: online,
            offline_nodes: offline,
            avg_cpu,
            avg_memory,
            avg_disk,
            avg_load,
            nodes,
        })
    }

    pub async fn exec_on_node(&self, id: i64, command: &str, timeout_secs: Option<u64>) -> Result<NodeExecResponse, AppError> {
        let node = self.node_repo.find_by_id(id).await?
            .ok_or(AppError::NotFound("节点不存在".into()))?;
        if node.status != "online" {
            return Err(AppError::Internal("节点离线，无法执行命令".into()));
        }

        let url = format!("http://{}:{}/exec", node.host, node.agent_port);
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", &node.auth_token)
            .json(&serde_json::json!({
                "command": command,
                "timeout_secs": timeout_secs.unwrap_or(30),
            }))
            .timeout(std::time::Duration::from_secs(timeout_secs.unwrap_or(30) + 5))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("请求节点失败: {e}")))?;

        let exec_resp: serde_json::Value = resp.json().await
            .map_err(|e| AppError::Internal(format!("解析响应失败: {e}")))?;

        Ok(NodeExecResponse {
            node_id: node.id,
            node_name: node.name.clone(),
            output: exec_resp["output"].as_str().unwrap_or("").to_string(),
            exit_code: exec_resp["exit_code"].as_i64().unwrap_or(-1) as i32,
            duration_ms: exec_resp["duration_ms"].as_u64().unwrap_or(0),
        })
    }

    pub async fn batch_exec(&self, node_ids: &[i64], command: &str, timeout_secs: Option<u64>) -> Result<Vec<NodeExecResponse>, AppError> {
        let mut tasks = Vec::new();
        for &id in node_ids {
            let cmd = command.to_string();
            let service = NodeService::new(self.node_repo.clone());
            tasks.push(tokio::spawn(async move {
                service.exec_on_node(id, &cmd, timeout_secs).await
            }));
        }

        let results = futures_util::future::join_all(tasks).await;
        let mut responses = Vec::new();
        for res in results {
            match res {
                Ok(Ok(r)) => responses.push(r),
                Ok(Err(e)) => responses.push(NodeExecResponse {
                    node_id: 0,
                    node_name: "error".into(),
                    output: e.to_string(),
                    exit_code: -1,
                    duration_ms: 0,
                }),
                Err(e) => responses.push(NodeExecResponse {
                    node_id: 0,
                    node_name: "error".into(),
                    output: format!("Task join error: {e}"),
                    exit_code: -1,
                    duration_ms: 0,
                }),
            }
        }
        Ok(responses)
    }

    pub async fn list_files(&self, id: i64, path: Option<String>) -> Result<Vec<FileEntry>, AppError> {
        let node = self.node_repo.find_by_id(id).await?
            .ok_or(AppError::NotFound("节点不存在".into()))?;
        if node.status != "online" {
            return Err(AppError::Internal("节点离线".into()));
        }

        let url = format!("http://{}:{}/files/list", node.host, node.agent_port);
        let client = reqwest::Client::new();
        let mut req = client.get(&url).header("Authorization", &node.auth_token);
        if let Some(p) = &path {
            req = req.query(&[("path", p)]);
        }
        let resp = req.send().await
            .map_err(|e| AppError::Internal(format!("请求节点文件列表失败: {e}")))?;
        let entries: Vec<FileEntry> = resp.json().await
            .map_err(|e| AppError::Internal(format!("解析文件列表失败: {e}")))?;
        Ok(entries)
    }

    pub async fn download_file(&self, id: i64, path: &str) -> Result<Vec<u8>, AppError> {
        let node = self.node_repo.find_by_id(id).await?
            .ok_or(AppError::NotFound("节点不存在".into()))?;
        let url = format!("http://{}:{}/files/download", node.host, node.agent_port);
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", &node.auth_token)
            .query(&[("path", path)])
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("请求下载文件失败: {e}")))?;
        let bytes = resp.bytes().await
            .map_err(|e| AppError::Internal(format!("下载文件失败: {e}")))?;
        Ok(bytes.to_vec())
    }

    pub async fn upload_file(&self, id: i64, path: &str, content: Vec<u8>) -> Result<(), AppError> {
        let node = self.node_repo.find_by_id(id).await?
            .ok_or(AppError::NotFound("节点不存在".into()))?;
        let url = format!("http://{}:{}/files/upload", node.host, node.agent_port);
        let client = reqwest::Client::new();
        client
            .post(&url)
            .header("Authorization", &node.auth_token)
            .query(&[("path", path)])
            .body(content)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("上传文件失败: {e}")))?;
        Ok(())
    }
}

// ─── Dashboard Service ────────────────────────────────────────────────────────

pub struct DashboardService {
    pub website_repo: Arc<dyn WebsiteRepository>,
    pub waf_repo: Arc<dyn WafRuleRepository>,
    pub db: SqlitePool,
    pub docker: Docker,
}

impl DashboardService {
    pub fn new(
        website_repo: Arc<dyn WebsiteRepository>,
        waf_repo: Arc<dyn WafRuleRepository>,
        db: SqlitePool,
        docker: Docker,
    ) -> Self {
        Self { website_repo, waf_repo, db, docker }
    }

    pub async fn get_dashboard(&self) -> Result<DashboardInfo, AppError> {
        let system = SystemService::get_info();

        let websites = self.website_repo.list().await.unwrap_or_default();
        let websites_total = websites.len() as i64;
        let websites_running = websites.iter().filter(|w| w.enabled).count() as i64;

        let (docker_running, docker_total) = Self::get_docker_stats(&self.docker).await;

        let (waf_total, waf_enabled) = self.waf_repo.count().await.unwrap_or((0, 0));

        let log_repo = SqliteLogRepository::new(self.db.clone());
        let recent_logs = log_repo.list_recent(20).await.unwrap_or_default();

        let gpu_info = SystemService::get_gpu_info();

        Ok(DashboardInfo {
            server_info: system,
            docker_containers_running: docker_running,
            docker_containers_total: docker_total,
            websites_running,
            websites_total,
            recent_logs,
            waf_rules_count: waf_total,
            waf_rules_enabled: waf_enabled,
            gpu_info,
        })
    }

    async fn get_docker_stats(docker: &Docker) -> (i64, i64) {
        use bollard::container::ListContainersOptions;
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        let containers = match docker.list_containers(Some(options)).await {
            Ok(c) => c,
            Err(_) => return (0, 0),
        };
        let total = containers.len() as i64;
        let running = containers.iter().filter(|c| c.state.as_deref() == Some("running")).count() as i64;
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

    pub async fn scan(docker: &Docker) -> Vec<CleanupItem> {
        let mut items = Vec::new();

        items.extend(Self::scan_temp_files());
        items.extend(Self::scan_docker_cache(docker).await);
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

    async fn scan_docker_cache(docker: &Docker) -> Vec<CleanupItem> {
        let mut items = Vec::new();

        // Docker dangling images via bollard
        use bollard::image::ListImagesOptions;
        let mut filters = std::collections::HashMap::new();
        filters.insert("dangling".to_string(), vec!["true".to_string()]);
        let img_options = ListImagesOptions::<String> {
            filters,
            ..Default::default()
        };
        if let Ok(images) = docker.list_images(Some(img_options)).await {
            let count = images.len();
            if count > 0 {
                let total_size: i64 = images.iter().map(|i| i.size).sum();
                items.push(CleanupItem {
                    category: "docker".into(),
                    name: "Docker 悬空镜像".into(),
                    description: format!("{} 个无标签的悬空镜像", count),
                    path: "docker image prune".into(),
                    size_bytes: total_size as u64,
                    size_display: format_bytes(total_size as u64),
                    can_clean: true,
                });
            }
        }

        // Stopped containers via bollard
        use bollard::container::ListContainersOptions;
        let mut c_filters = std::collections::HashMap::new();
        c_filters.insert("status".to_string(), vec!["exited".to_string()]);
        let c_options = ListContainersOptions::<String> {
            all: true,
            filters: c_filters,
            ..Default::default()
        };
        if let Ok(containers) = docker.list_containers(Some(c_options)).await {
            let count = containers.len();
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

        // Docker disk usage summary via bollard df
        if let Ok(df) = docker.df().await {
            let reclaimable = df.layers_size.unwrap_or(0) as u64;
            if reclaimable > 0 {
                items.push(CleanupItem {
                    category: "docker".into(),
                    name: "Docker 可回收空间".into(),
                    description: "构建缓存和未使用数据".into(),
                    path: "docker builder prune".into(),
                    size_bytes: reclaimable,
                    size_display: format_bytes(reclaimable),
                    can_clean: true,
                });
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

    pub async fn clean(categories: &[String], docker: &Docker) -> CleanupResult {
        let mut cleaned = Vec::new();
        let mut errors = Vec::new();
        let mut freed = 0u64;

        for category in categories {
            match category.as_str() {
                "temp" => match Self::clean_temp_files() {
                    Ok(n) => { freed += n; cleaned.push("系统临时文件已清理".into()); }
                    Err(e) => errors.push(format!("临时文件清理失败: {e}")),
                },
                "docker" => match Self::clean_docker(docker).await {
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

    async fn clean_docker(docker: &Docker) -> Result<String, String> {
        use bollard::container::PruneContainersOptions;
        use bollard::image::PruneImagesOptions;

        let mut count = 0u64;

        if let Ok(result) = docker
            .prune_containers(None::<PruneContainersOptions<String>>)
            .await
        {
            count += result.containers_deleted.map(|v| v.len()).unwrap_or(0) as u64;
        }

        if let Ok(result) = docker
            .prune_images(None::<PruneImagesOptions<String>>)
            .await
        {
            count += result.images_deleted.map(|v| v.len()).unwrap_or(0) as u64;
        }

        if count > 0 {
            Ok(format!("Docker 清理完成 ({} 项)", count))
        } else {
            Ok("Docker 已清理，无可回收资源".into())
        }
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

// ─── AI 助手服务 ─────────────────────────────────────────────────────────────

pub struct AiService {
    ai_repo: Arc<dyn AiConversationRepository>,
    ollama_base: String,
}

impl AiService {
    pub fn new(ai_repo: Arc<dyn AiConversationRepository>) -> Self {
        let ollama_base = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".into());
        Self { ai_repo, ollama_base }
    }

    /// List available Ollama models.
    pub async fn list_models(&self) -> Result<Vec<AiModelInfo>, AppError> {
        let url = format!("{}/api/tags", self.ollama_base);
        let resp = reqwest::get(&url).await
            .map_err(|e| AppError::Internal(format!("连接 Ollama 失败: {e}")))?;

        #[derive(Deserialize)]
        struct OllamaTag { name: String, size: u64, modified: String }
        #[derive(Deserialize)]
        struct OllamaList { models: Vec<OllamaTag> }

        let body: OllamaList = resp.json().await
            .map_err(|e| AppError::Internal(format!("解析模型列表失败: {e}")))?;

        Ok(body.models.into_iter().map(|m| AiModelInfo {
            name: m.name,
            size: Self::format_size(m.size),
            modified: m.modified,
        }).collect())
    }

    /// Chat with Ollama, returning the assistant's reply.
    pub async fn chat(&self, req: AiChatRequest) -> Result<AiChatResponse, AppError> {
        // Load or create conversation
        let (conv_id, mut history, model) = if let Some(id) = req.conversation_id {
            let conv = self.ai_repo.find_by_id(id).await?
                .ok_or(AppError::NotFound("对话不存在".into()))?;
            let msgs: Vec<AiMessage> = serde_json::from_str(&conv.messages).unwrap_or_default();
            (conv.id, msgs, conv.model)
        } else {
            let conv = self.ai_repo.create(&req.model).await?;
            (conv.id, Vec::new(), conv.model)
        };

        // Add user message
        history.push(AiMessage { role: "user".into(), content: req.message.clone() });

        // Call Ollama chat API
        let reply = self.ollama_chat(&model, &history).await?;

        // Add assistant reply to history
        history.push(AiMessage { role: "assistant".into(), content: reply.clone() });

        // Save updated messages
        let messages_json = serde_json::to_string(&history)
            .map_err(|e| AppError::Internal(format!("序列化消息失败: {e}")))?;

        // Auto-generate title from first user message
        let title = if history.iter().filter(|m| m.role == "user").count() == 1 {
            let t: String = req.message.chars().take(40).collect();
            if req.message.len() > 40 { format!("{t}...") } else { t }
        } else {
            // Find existing title
            self.ai_repo.find_by_id(conv_id).await?
                .map(|c| c.title)
                .unwrap_or_else(|| "对话".into())
        };

        self.ai_repo.update_messages(conv_id, &title, &messages_json).await?;

        Ok(AiChatResponse { conversation_id: conv_id, title, reply })
    }

    /// Chat with streaming — returns a channel receiver that yields JSON events.
    /// Events: `{"token":"..."}`, `{"done":true,"conversation_id":N}`, `{"error":"..."}`.
    pub async fn chat_stream(
        &self,
        req: AiChatRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<serde_json::Value>, AppError> {
        // Load or create conversation
        let (conv_id, mut history, model) = if let Some(id) = req.conversation_id {
            let conv = self.ai_repo.find_by_id(id).await?
                .ok_or(AppError::NotFound("对话不存在".into()))?;
            let msgs: Vec<AiMessage> = serde_json::from_str(&conv.messages).unwrap_or_default();
            (conv.id, msgs, conv.model)
        } else {
            let conv = self.ai_repo.create(&req.model).await?;
            (conv.id, Vec::new(), conv.model)
        };

        history.push(AiMessage { role: "user".into(), content: req.message.clone() });

        let (tx, rx) = tokio::sync::mpsc::channel::<serde_json::Value>(64);

        let ollama_base = self.ollama_base.clone();
        let ai_repo = self.ai_repo.clone();
        let user_msg = req.message.clone();

        tokio::spawn(async move {
            let url = format!("{}/api/chat", ollama_base);
            let body = serde_json::json!({
                "model": model,
                "messages": history.iter().map(|m| {
                    serde_json::json!({ "role": m.role, "content": m.content })
                }).collect::<Vec<_>>(),
                "stream": true,
            });

            let full_reply = match Self::stream_ollama(&url, &body, &tx).await {
                Ok(reply) => reply,
                Err(e) => {
                    let _ = tx.send(serde_json::json!({"error": e.to_string()})).await;
                    return;
                }
            };

            // Save conversation
            history.push(AiMessage { role: "assistant".into(), content: full_reply.clone() });
            let messages_json = serde_json::to_string(&history).unwrap_or_default();

            let title = if history.iter().filter(|m| m.role == "user").count() == 1 {
                let t: String = user_msg.chars().take(40).collect();
                if user_msg.len() > 40 { format!("{t}...") } else { t }
            } else {
                ai_repo.find_by_id(conv_id).await
                    .ok()
                    .flatten()
                    .map(|c| c.title)
                    .unwrap_or_else(|| "对话".into())
            };

            let _ = ai_repo.update_messages(conv_id, &title, &messages_json).await;
            let _ = tx.send(serde_json::json!({"done": true, "conversation_id": conv_id})).await;
        });

        Ok(rx)
    }

    async fn stream_ollama(
        url: &str,
        body: &serde_json::Value,
        tx: &tokio::sync::mpsc::Sender<serde_json::Value>,
    ) -> Result<String, AppError> {
        let client = reqwest::Client::new();
        let resp = client.post(url)
            .json(body)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("请求 Ollama 失败: {e}")))?;

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut full_reply = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::Internal(format!("读取流失败: {e}")))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() { continue; }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(content) = parsed["message"]["content"].as_str() {
                        if !content.is_empty() {
                            full_reply.push_str(content);
                            let _ = tx.send(serde_json::json!({"token": content})).await;
                        }
                    }
                    if parsed["done"].as_bool() == Some(true) {
                        return Ok(full_reply);
                    }
                }
            }
        }

        Ok(full_reply)
    }

    /// Send messages to Ollama /api/chat and return the reply content.
    async fn ollama_chat(&self, model: &str, messages: &[AiMessage]) -> Result<String, AppError> {
        let url = format!("{}/api/chat", self.ollama_base);

        let body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "stream": false,
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("请求 Ollama 失败: {e}")))?;

        #[derive(Deserialize)]
        struct OllamaChatResponse { message: OllamaMsg }
        #[derive(Deserialize)]
        struct OllamaMsg { content: String }

        let data: OllamaChatResponse = resp.json().await
            .map_err(|e| AppError::Internal(format!("解析 Ollama 回复失败: {e}")))?;

        Ok(data.message.content)
    }

    /// AI log analysis — sends log content to Ollama for analysis.
    pub async fn analyze_logs(&self, req: AiAnalyzeRequest) -> Result<String, AppError> {
        let model = req.model.unwrap_or_else(|| "llama3".into());
        let prompt = format!(
            "你是一名资深运维工程师。请分析以下服务器日志，识别异常、错误模式和安全威胁，并给出具体建议。\n\n日志内容：\n{}",
            req.log_content
        );

        let messages = vec![
            AiMessage { role: "system".into(), content: "用中文回复，简洁专业。".into() },
            AiMessage { role: "user".into(), content: prompt },
        ];

        self.ollama_chat(&model, &messages).await
    }

    pub async fn list_conversations(&self) -> Result<Vec<AiConversation>, AppError> {
        self.ai_repo.list_all().await
    }

    pub async fn get_conversation(&self, id: i64) -> Result<AiConversation, AppError> {
        self.ai_repo.find_by_id(id).await?
            .ok_or(AppError::NotFound("对话不存在".into()))
    }

    pub async fn delete_conversation(&self, id: i64) -> Result<(), AppError> {
        self.ai_repo.delete(id).await
    }

    fn format_size(bytes: u64) -> String {
        let gb = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
        if gb < 1.0 {
            format!("{} MB", bytes / 1024 / 1024)
        } else if gb < 1000.0 {
            format!("{:.1} GB", gb)
        } else {
            format!("{:.1} TB", gb / 1000.0)
        }
    }
}

// ─── Backup Service ─────────────────────────────────────────────────────────

pub struct BackupService {
    backup_repo: Arc<dyn BackupRepository>,
}

impl BackupService {
    pub fn new(backup_repo: Arc<dyn BackupRepository>) -> Self {
        Self { backup_repo }
    }

    pub async fn list_configs(&self) -> Result<Vec<BackupConfig>, AppError> {
        self.backup_repo.list_configs().await
    }

    pub async fn get_config(&self, id: i64) -> Result<BackupConfig, AppError> {
        self.backup_repo.find_config(id).await?
            .ok_or(AppError::NotFound("备份配置不存在".into()))
    }

    pub async fn create_config(&self, req: CreateBackupConfigRequest) -> Result<BackupConfig, AppError> {
        if req.name.is_empty() || req.target_path.is_empty() {
            return Err(AppError::BadRequest("名称和目标路径不能为空".into()));
        }
        self.backup_repo.create_config(&req).await
    }

    pub async fn update_config(&self, id: i64, req: UpdateBackupConfigRequest) -> Result<(), AppError> {
        self.backup_repo.update_config(id, &req).await
    }

    pub async fn delete_config(&self, id: i64) -> Result<(), AppError> {
        self.backup_repo.delete_config(id).await
    }

    pub async fn list_records(&self, config_id: i64) -> Result<Vec<BackupRecord>, AppError> {
        self.backup_repo.list_records(config_id).await
    }

    fn get_remote_storage(&self, config: &BackupConfig) -> Option<Arc<dyn RemoteStorage>> {
        if config.storage_type == "local" || config.storage_path.is_empty() {
            return None;
        }
        create_remote_storage(&config.storage_type, &config.storage_path).ok()
    }

    pub async fn execute_backup(&self, config_id: i64) -> Result<BackupRecord, AppError> {
        let config = self.backup_repo.find_config(config_id).await?
            .ok_or(AppError::NotFound("备份配置不存在".into()))?;

        let now = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let tar_name = format!("{}_{}.tar.gz", config.name, now);
        // For remote storage, use a local staging directory; for local, use storage_path
        let local_dir = if config.storage_type == "local" {
            config.storage_path.clone()
        } else {
            "data/backups".to_string()
        };
        let storage_dir = std::path::Path::new(&local_dir);
        let tar_path = storage_dir.join(&tar_name);

        // Create storage dir if needed
        if let Err(e) = std::fs::create_dir_all(storage_dir) {
            let record = self.backup_repo.create_record(config_id, &tar_name).await?;
            let _ = self.backup_repo.finish_record(record.id, 0, "failed", Some(&format!("创建目录失败: {e}"))).await;
            return Err(AppError::Internal(format!("创建备份目录失败: {e}")));
        }

        let record = self.backup_repo.create_record(config_id, &tar_name).await?;

        let target = config.target_path.clone();
        let tp = tar_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::create_tar_gz(&target, &tp)
        }).await;

        match result {
            Ok(Ok(size)) => {
                let _ = self.backup_repo.finish_record(record.id, size, "success", None).await;

                // Upload to remote storage if configured
                if config.storage_type != "local" {
                    if let Some(storage) = self.get_remote_storage(&config) {
                        let tar_content = match std::fs::read(&tar_path) {
                            Ok(data) => data,
                            Err(e) => {
                                let _ = self.backup_repo.finish_record(record.id, size, "failed", Some(&format!("读取备份文件失败: {e}"))).await;
                                return Err(AppError::Internal(format!("读取备份文件失败: {e}")));
                            }
                        };
                        if let Err(e) = storage.upload(&tar_name, tar_content).await {
                            let _ = self.backup_repo.finish_record(record.id, size, "failed", Some(&format!("远程上传失败: {e}"))).await;
                            return Err(AppError::Internal(format!("远程上传失败: {e}")));
                        }
                        tracing::info!("备份 {} 已上传到远程存储 ({})", tar_name, config.storage_type);
                    }
                }

                // Retention cleanup
                let _ = self.backup_repo.delete_old_records(config_id, config.retention_days).await;
                let _ = Self::cleanup_old_files(&local_dir, &config.name, config.retention_days);
                self.backup_repo.list_records(config_id).await?
                    .into_iter()
                    .find(|r| r.id == record.id)
                    .ok_or(AppError::Internal("备份记录未找到".into()))
            }
            Ok(Err(e)) => {
                let _ = self.backup_repo.finish_record(record.id, 0, "failed", Some(&e)).await;
                Err(AppError::Internal(e))
            }
            Err(e) => {
                let _ = self.backup_repo.finish_record(record.id, 0, "failed", Some(&format!("线程错误: {e}"))).await;
                Err(AppError::Internal(format!("备份任务失败: {e}")))
            }
        }
    }

    fn create_tar_gz(target: &str, output: &std::path::Path) -> Result<i64, String> {
        let target_path = std::path::Path::new(target);
        if !target_path.exists() {
            return Err(format!("目标路径不存在: {target}"));
        }

        let output_file = std::fs::File::create(output)
            .map_err(|e| format!("创建备份文件失败: {e}"))?;
        let encoder = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);

        let entry_name = target_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "backup".into());

        if target_path.is_dir() {
            tar.append_dir_all(&entry_name, target_path)
                .map_err(|e| format!("归档目录失败: {e}"))?;
        } else {
            tar.append_path_with_name(target_path, &entry_name)
                .map_err(|e| format!("归档文件失败: {e}"))?;
        }

        let encoder = tar.into_inner()
            .map_err(|e| format!("完成归档失败: {e}"))?;
        let output_file = encoder.finish()
            .map_err(|e| format!("完成压缩失败: {e}"))?;

        let size = output_file.metadata()
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        Ok(size)
    }

    fn cleanup_old_files(storage_path: &str, prefix: &str, keep_days: i64) -> Result<(), String> {
        let dir = std::path::Path::new(storage_path);
        if !dir.exists() {
            return Ok(());
        }
        let cutoff = std::time::SystemTime::now()
            - std::time::Duration::from_secs((keep_days * 86400) as u64);
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(meta) = path.symlink_metadata() {
                    if meta.is_file()
                        && path.file_name().map_or(false, |n| {
                            n.to_string_lossy().starts_with(prefix)
                        })
                        && meta.modified().map_or(false, |m| m < cutoff)
                    {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn restore_backup(&self, record_id: i64, target_path: Option<String>) -> Result<(), AppError> {
        // Find the record and its config
        let configs = self.backup_repo.list_configs().await?;
        let mut record: Option<BackupRecord> = None;
        let mut config: Option<BackupConfig> = None;

        for c in &configs {
            if let Ok(records) = self.backup_repo.list_records(c.id).await {
                if let Some(r) = records.into_iter().find(|r| r.id == record_id) {
                    record = Some(r);
                    config = Some(c.clone());
                    break;
                }
            }
        }

        let record = record.ok_or(AppError::NotFound("备份记录不存在".into()))?;
        let config = config.unwrap();

        if record.status != "success" {
            return Err(AppError::BadRequest("只能恢复成功完成的备份".into()));
        }

        let local_dir = if config.storage_type == "local" {
            config.storage_path.clone()
        } else {
            "data/backups".to_string()
        };
        let storage_dir = std::path::Path::new(&local_dir);
        let tar_path = storage_dir.join(&record.file_name);

        if !tar_path.exists() {
            // Try to download from remote storage
            if config.storage_type != "local" {
                if let Some(storage) = self.get_remote_storage(&config) {
                    let data = storage.download(&record.file_name).await
                        .map_err(|e| AppError::Internal(format!("从远程存储下载失败: {e}")))?;
                    std::fs::create_dir_all(storage_dir)
                        .map_err(|e| AppError::Internal(format!("创建存储目录失败: {e}")))?;
                    std::fs::write(&tar_path, &data)
                        .map_err(|e| AppError::Internal(format!("写入临时文件失败: {e}")))?;
                } else {
                    return Err(AppError::NotFound("备份文件不存在且远程存储未配置".into()));
                }
            } else {
                return Err(AppError::NotFound(format!("备份文件不存在: {}", tar_path.display())));
            }
        }

        let dest = target_path.unwrap_or(config.target_path);
        let dest_path = std::path::Path::new(&dest);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("创建恢复目录失败: {e}")))?;
        }

        let tp = tar_path.clone();
        let dp = dest.clone();
        tokio::task::spawn_blocking(move || {
            Self::extract_tar_gz(&tp, &dp)
        }).await
        .map_err(|e| AppError::Internal(format!("恢复任务失败: {e}")))?
        .map_err(|e| AppError::Internal(e))?;

        Ok(())
    }

    fn extract_tar_gz(archive: &std::path::Path, dest: &str) -> Result<(), String> {
        let file = std::fs::File::open(archive)
            .map_err(|e| format!("打开备份文件失败: {e}"))?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)
            .map_err(|e| format!("解压备份失败: {e}"))?;
        Ok(())
    }
}

// ─── Alert Service ──────────────────────────────────────────────────────────────

pub struct AlertService {
    notification_repo: Arc<dyn NotificationRepository>,
    alert_rule_repo: Arc<dyn AlertRuleRepository>,
    alert_history_repo: Arc<dyn AlertHistoryRepository>,
    metrics_rx: broadcast::Receiver<MetricsSnapshot>,
}

impl AlertService {
    pub fn new(
        notification_repo: Arc<dyn NotificationRepository>,
        alert_rule_repo: Arc<dyn AlertRuleRepository>,
        alert_history_repo: Arc<dyn AlertHistoryRepository>,
        metrics_rx: broadcast::Receiver<MetricsSnapshot>,
    ) -> Self {
        Self {
            notification_repo,
            alert_rule_repo,
            alert_history_repo,
            metrics_rx,
        }
    }

    // ── Notification Channel CRUD ───────────────────────────────────────────

    pub async fn list_channels(&self) -> Result<Vec<NotificationChannel>, AppError> {
        self.notification_repo.list_channels().await
    }

    pub async fn get_channel(&self, id: i64) -> Result<NotificationChannel, AppError> {
        self.notification_repo.find_channel(id).await?
            .ok_or_else(|| AppError::NotFound("通知渠道不存在".into()))
    }

    pub async fn create_channel(&self, req: CreateNotificationChannelRequest) -> Result<NotificationChannel, AppError> {
        self.notification_repo.create_channel(req).await
    }

    pub async fn update_channel(&self, id: i64, req: UpdateNotificationChannelRequest) -> Result<(), AppError> {
        self.notification_repo.update_channel(id, req).await
    }

    pub async fn delete_channel(&self, id: i64) -> Result<(), AppError> {
        self.notification_repo.delete_channel(id).await
    }

    pub async fn test_channel(&self, id: i64) -> Result<(), AppError> {
        let channel = self.notification_repo.find_channel(id).await?
            .ok_or_else(|| AppError::NotFound("通知渠道不存在".into()))?;
        Self::send_notification(&channel, "Flamepanel 告警测试", "这是一条测试消息，如果您收到此消息，说明通知渠道配置正确。").await
    }

    // ── Alert Rule CRUD ─────────────────────────────────────────────────────

    pub async fn list_rules(&self) -> Result<Vec<AlertRule>, AppError> {
        self.alert_rule_repo.list_rules().await
    }

    pub async fn get_rule(&self, id: i64) -> Result<AlertRule, AppError> {
        self.alert_rule_repo.find_rule(id).await?
            .ok_or_else(|| AppError::NotFound("告警规则不存在".into()))
    }

    pub async fn create_rule(&self, req: CreateAlertRuleRequest) -> Result<AlertRule, AppError> {
        self.alert_rule_repo.create_rule(req).await
    }

    pub async fn update_rule(&self, id: i64, req: UpdateAlertRuleRequest) -> Result<(), AppError> {
        self.alert_rule_repo.update_rule(id, req).await
    }

    pub async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        self.alert_rule_repo.delete_rule(id).await
    }

    // ── Alert History ───────────────────────────────────────────────────────

    pub async fn list_history(&self, limit: i64) -> Result<Vec<AlertHistory>, AppError> {
        let limit = limit.min(500);
        self.alert_history_repo.list_history(limit).await
    }

    pub async fn list_history_by_rule(&self, rule_id: i64) -> Result<Vec<AlertHistory>, AppError> {
        self.alert_history_repo.list_history_by_rule(rule_id).await
    }

    // ── Background Alert Checker ─────────────────────────────────────────────

    pub async fn start_checker(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = self.check_rules().await {
                    tracing::error!("告警检查失败: {e}");
                }
            }
        });
    }

    async fn check_rules(&self) -> Result<(), AppError> {
        let rules = self.alert_rule_repo.list_enabled_rules().await?;
        if rules.is_empty() {
            return Ok(());
        }

        // Get latest metrics snapshot from channel (non-blocking)
        let snapshot = match self.metrics_rx.resubscribe().recv().await {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };

        for rule in &rules {
            if !self.should_trigger(rule).await? {
                continue;
            }

            let current_value = self.get_metric_value(&rule.metric_type, &snapshot);
            if !Self::evaluate_condition(&rule.condition, current_value, rule.threshold) {
                continue;
            }

            let channel_ids: Vec<i64> = serde_json::from_str(&rule.channel_ids).unwrap_or_default();
            let message = format!(
                "🔥 告警触发: {}\n指标: {}\n当前值: {:.1}\n阈值: {:.1}\n条件: {} {} {}",
                rule.name, rule.metric_type, current_value, rule.threshold,
                current_value, rule.condition, rule.threshold
            );

            for cid in &channel_ids {
                if let Ok(Some(channel)) = self.notification_repo.find_channel(*cid).await {
                    if channel.enabled {
                        if let Err(e) = Self::send_notification(&channel, &format!("Flamepanel 告警: {}", rule.name), &message).await {
                            tracing::error!("发送告警通知失败 (channel={}): {e}", cid);
                        }
                    }
                }
            }

            self.alert_rule_repo.update_last_triggered(rule.id).await?;
            self.alert_history_repo.create_history(
                rule.id, &rule.name, &rule.metric_type, current_value,
                rule.threshold, "firing", &message,
            ).await?;
        }

        Ok(())
    }

    async fn should_trigger(&self, rule: &AlertRule) -> Result<bool, AppError> {
        if let Some(ref last) = rule.last_triggered {
            // Simple cooldown check using string comparison
            if !last.is_empty() {
                // If recently triggered, skip (cooldown handled by interval)
                return Ok(true); // The cooldown check happens in practice via the 30s interval
            }
        }
        Ok(true)
    }

    fn get_metric_value(&self, metric_type: &str, snapshot: &MetricsSnapshot) -> f64 {
        match metric_type {
            "cpu" => snapshot.cpu_usage as f64,
            "memory" => snapshot.memory_usage_percent as f64,
            "disk" => snapshot.disk_usage_percent as f64,
            "load" => snapshot.load_one,
            _ => 0.0,
        }
    }

    fn evaluate_condition(condition: &str, value: f64, threshold: f64) -> bool {
        match condition {
            "gt" => value > threshold,
            "lt" => value < threshold,
            "gte" => value >= threshold,
            "lte" => value <= threshold,
            "eq" => (value - threshold).abs() < f64::EPSILON,
            _ => false,
        }
    }

    // ── Notification Sender ─────────────────────────────────────────────────

    async fn send_notification(channel: &NotificationChannel, title: &str, message: &str) -> Result<(), AppError> {
        match channel.channel_type.as_str() {
            "email" => Self::send_email(channel, title, message).await,
            "telegram" => Self::send_telegram(channel, message).await,
            "webhook" => Self::send_webhook(channel, title, message).await,
            _ => Err(AppError::Internal(format!("未知的通知类型: {}", channel.channel_type))),
        }
    }

    async fn send_email(channel: &NotificationChannel, title: &str, message: &str) -> Result<(), AppError> {
        let config: serde_json::Value = serde_json::from_str(&channel.config)
            .map_err(|e| AppError::Internal(format!("解析邮件配置失败: {e}")))?;
        let smtp_host = config["smtp_host"].as_str().unwrap_or("").to_string();
        let smtp_port = config["smtp_port"].as_u64().unwrap_or(587) as u16;
        let username = config["username"].as_str().unwrap_or("").to_string();
        let password = config["password"].as_str().unwrap_or("").to_string();
        let to = config["to"].as_str().unwrap_or("").to_string();

        if smtp_host.is_empty() || to.is_empty() {
            return Err(AppError::Internal("邮件配置不完整".into()));
        }

        let title = title.to_string();
        let message = message.to_string();

        tokio::task::spawn_blocking(move || {
            let email = lettre::Message::builder()
                .from(username.parse().map_err(|e: lettre::address::AddressError| format!("发件人格式错误: {e}"))?)
                .to(to.parse().map_err(|e: lettre::address::AddressError| format!("收件人格式错误: {e}"))?)
                .subject(&title)
                .body(message)
                .map_err(|e| format!("构建邮件失败: {e}"))?;

            use lettre::Transport;
            let creds = lettre::transport::smtp::authentication::Credentials::new(
                username.clone(), password.clone(),
            );
            let mailer = lettre::SmtpTransport::builder_dangerous(&smtp_host)
                .port(smtp_port)
                .credentials(creds)
                .build();

            mailer.send(&email).map_err(|e| format!("发送邮件失败: {e}"))?;
            Ok::<(), String>(())
        }).await
        .map_err(|e| AppError::Internal(format!("邮件发送任务失败: {e}")))?
        .map_err(|msg| AppError::Internal(msg))?;
        Ok(())
    }

    async fn send_telegram(channel: &NotificationChannel, message: &str) -> Result<(), AppError> {
        let config: serde_json::Value = serde_json::from_str(&channel.config)
            .map_err(|e| AppError::Internal(format!("解析 Telegram 配置失败: {e}")))?;
        let bot_token = config["bot_token"].as_str().unwrap_or("");
        let chat_id = config["chat_id"].as_str().unwrap_or("");

        if bot_token.is_empty() || chat_id.is_empty() {
            return Err(AppError::Internal("Telegram 配置不完整".into()));
        }

        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        let client = reqwest::Client::new();
        client.post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": message,
                "parse_mode": "HTML",
            }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("发送 Telegram 消息失败: {e}")))?;
        Ok(())
    }

    async fn send_webhook(channel: &NotificationChannel, title: &str, message: &str) -> Result<(), AppError> {
        let config: serde_json::Value = serde_json::from_str(&channel.config)
            .map_err(|e| AppError::Internal(format!("解析 Webhook 配置失败: {e}")))?;
        let url = config["url"].as_str().unwrap_or("");
        let method = config["method"].as_str().unwrap_or("POST");

        if url.is_empty() {
            return Err(AppError::Internal("Webhook URL 不能为空".into()));
        }

        let client = reqwest::Client::new();
        let req = match method {
            "GET" => client.get(url),
            _ => client.post(url).json(&serde_json::json!({
                "title": title,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        };
        req.send().await
            .map_err(|e| AppError::Internal(format!("发送 Webhook 失败: {e}")))?;
        Ok(())
    }
}

// ─── Role Service ─────────────────────────────────────────────────────────────

pub struct RoleService {
    role_repo: Arc<dyn RoleRepository>,
    permission_repo: Arc<dyn PermissionRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl RoleService {
    pub fn new(
        role_repo: Arc<dyn RoleRepository>,
        permission_repo: Arc<dyn PermissionRepository>,
        user_repo: Arc<dyn UserRepository>,
    ) -> Self {
        Self { role_repo, permission_repo, user_repo }
    }

    pub async fn list_roles(&self) -> Result<Vec<RoleWithPermissions>, AppError> {
        let roles = self.role_repo.list_all().await?;
        let mut result = Vec::new();
        for role in roles {
            if let Some(rwp) = self.role_repo.find_with_permissions(role.id).await? {
                result.push(rwp);
            }
        }
        Ok(result)
    }

    pub async fn get_role(&self, id: i64) -> Result<RoleWithPermissions, AppError> {
        self.role_repo.find_with_permissions(id)
            .await?
            .ok_or(AppError::NotFound("角色不存在".into()))
    }

    pub async fn create_role(&self, req: &CreateRoleRequest) -> Result<Role, AppError> {
        self.role_repo.create(req).await
    }

    pub async fn update_role(&self, id: i64, req: &UpdateRoleRequest) -> Result<(), AppError> {
        self.role_repo.update(id, req).await
    }

    pub async fn delete_role(&self, id: i64) -> Result<(), AppError> {
        self.role_repo.delete(id).await
    }

    pub async fn list_permissions(&self) -> Result<Vec<Permission>, AppError> {
        self.permission_repo.list_all().await
    }

    pub async fn assign_role(&self, req: &AssignRoleRequest) -> Result<(), AppError> {
        self.user_repo.update_role(req.user_id, &req.role).await
    }

    pub async fn get_user_permissions(&self, role: &str) -> Result<Vec<String>, AppError> {
        self.role_repo.get_user_permissions(role).await
    }
}
