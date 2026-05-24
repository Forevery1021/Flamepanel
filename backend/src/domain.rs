use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

// ─── User 领域实体 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: NaiveDateTime,
    pub last_login: Option<NaiveDateTime>,
}

// ─── Server 领域实体（系统资源监控）────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerInfo {
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_free_mb: u64,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub disk_free_gb: f64,
    pub uptime_seconds: u64,
    pub load_average: LoadAverage,
    pub network: NetworkInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkInfo {
    pub hostname: String,
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkInterface {
    pub name: String,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub mac: String,
}

// ─── GPU 信息 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GpuInfo {
    pub name: String,
    pub temperature_celsius: f32,
    pub utilization_percent: f32,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_free_mb: u64,
    pub fan_speed_percent: f32,
}

// ─── Website 领域实体（Nginx 站点）─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Website {
    pub id: i64,
    pub domain: String,
    pub root_path: String,
    pub proxy_port: Option<i32>,
    pub ssl_enabled: bool,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
    pub config_path: String,
    pub enabled: bool,
    pub engine: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebsiteRequest {
    pub domain: String,
    pub root_path: String,
    pub proxy_port: Option<i32>,
    pub enable_ssl: bool,
    pub engine: Option<String>,
}

// ─── File 领域实体 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
    pub permissions: String,
}

// ─── Docker 领域实体 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: Vec<String>,
    pub created: String,
    pub memory_usage: Option<String>,
    pub cpu_usage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainerLogs {
    pub container_id: String,
    pub logs: String,
}

// ─── WAF 规则实体 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WafRule {
    pub id: i64,
    pub name: String,
    pub pattern: String,
    pub target: String,
    pub action: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWafRuleRequest {
    pub name: String,
    pub pattern: String,
    pub target: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWafRuleRequest {
    pub name: Option<String>,
    pub pattern: Option<String>,
    pub target: Option<String>,
    pub action: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

// ─── WAF IP 规则 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WafIpRule {
    pub id: i64,
    pub ip: String,
    pub action: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWafIpRuleRequest {
    pub ip: String,
    pub action: String,
    pub description: Option<String>,
}

// ─── Dashboard 聚合数据 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardInfo {
    pub server_info: ServerInfo,
    pub docker_containers_running: i64,
    pub docker_containers_total: i64,
    pub websites_running: i64,
    pub websites_total: i64,
    pub recent_logs: Vec<OperationLogEntry>,
    pub waf_rules_count: i64,
    pub waf_rules_enabled: i64,
    pub gpu_info: Vec<GpuInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OperationLogEntry {
    pub username: String,
    pub action: String,
    pub target: String,
    pub ip: String,
    pub created_at: String,
}

// ─── 系统清理 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupItem {
    pub category: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_display: String,
    pub can_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupScanResult {
    pub items: Vec<CleanupItem>,
    pub total_bytes: u64,
    pub total_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRequest {
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub cleaned_items: Vec<String>,
    pub freed_bytes: u64,
    pub freed_display: String,
    pub errors: Vec<String>,
}

// ─── 面板设置 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettingsRequest {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub theme_color: Option<String>,
    pub background_image: Option<String>,
    pub background_opacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelSettings {
    pub theme: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_opacity: Option<f64>,
}

// ─── 计划任务 (Cron) ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CronJob {
    pub id: i64,
    pub name: String,
    pub schedule: String,
    pub command: Option<String>,
    pub url: Option<String>,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCronJobRequest {
    pub name: String,
    pub schedule: String,
    pub command: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCronJobRequest {
    pub name: Option<String>,
    pub schedule: Option<String>,
    pub command: Option<String>,
    pub url: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CronJobLog {
    pub id: i64,
    pub job_id: i64,
    pub status: String,
    pub output: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

// ─── 数据库管理 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DatabaseInstance {
    pub id: i64,
    pub name: String,
    pub db_type: String,
    pub version: String,
    pub port: i32,
    pub container_id: Option<String>,
    pub username: String,
    pub password: String,
    pub status: String,
    pub data_dir: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub db_type: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DatabaseBackup {
    pub id: i64,
    pub instance_id: i64,
    pub filename: String,
    pub size_bytes: i64,
    pub created_at: String,
}

// ─── 应用商店 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstalledApp {
    pub id: i64,
    pub app_key: String,
    pub name: String,
    pub category: String,
    pub port: i32,
    pub status: String,
    pub compose_file: Option<String>,
    pub data_dir: Option<String>,
    pub version: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallAppRequest {
    pub app_key: String,
    pub name: String,
    pub port: Option<i32>,
    pub extra_env: Option<std::collections::HashMap<String, String>>,
}

// ─── 备份系统 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupConfig {
    pub id: i64,
    pub name: String,
    pub backup_type: String,
    pub target_path: String,
    pub storage_type: String,
    pub storage_path: String,
    pub cron_expr: Option<String>,
    pub retention_days: i64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBackupConfigRequest {
    pub name: String,
    pub backup_type: String,
    pub target_path: String,
    pub storage_type: Option<String>,
    pub storage_path: Option<String>,
    pub cron_expr: Option<String>,
    pub retention_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBackupConfigRequest {
    pub name: Option<String>,
    pub backup_type: Option<String>,
    pub target_path: Option<String>,
    pub storage_type: Option<String>,
    pub storage_path: Option<String>,
    pub cron_expr: Option<String>,
    pub retention_days: Option<i64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupRecord {
    pub id: i64,
    pub config_id: i64,
    pub file_name: String,
    pub file_size: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

// ─── AI 助手 ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiConversation {
    pub id: i64,
    pub title: String,
    pub model: String,
    pub messages: String, // JSON array of {role, content}
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatRequest {
    pub conversation_id: Option<i64>,
    pub model: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatResponse {
    pub conversation_id: i64,
    pub title: String,
    pub reply: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelInfo {
    pub name: String,
    pub size: String,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalyzeRequest {
    pub log_content: String,
    pub model: Option<String>,
}

// ─── 多节点管理 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NodeInfo {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub agent_port: i64,
    pub auth_token: String,
    pub status: String,
    pub cpu_usage: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub load_one: f32,
    pub last_heartbeat: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegisterRequest {
    pub name: String,
    pub host: String,
    pub agent_port: Option<i64>,
    pub auth_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeartbeatRequest {
    pub cpu_usage: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub load_one: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterDashboard {
    pub total_nodes: i64,
    pub online_nodes: i64,
    pub offline_nodes: i64,
    pub avg_cpu: f32,
    pub avg_memory: f32,
    pub avg_disk: f32,
    pub avg_load: f32,
    pub nodes: Vec<NodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecRequest {
    pub command: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecResponse {
    pub node_id: i64,
    pub node_name: String,
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecRequest {
    pub node_ids: Vec<i64>,
    pub command: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

// ─── 告警通知 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NotificationChannel {
    pub id: i64,
    pub name: String,
    pub channel_type: String,
    pub config: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub channel_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateNotificationChannelRequest {
    pub name: Option<String>,
    pub channel_type: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AlertRule {
    pub id: i64,
    pub name: String,
    pub metric_type: String,
    pub condition: String,
    pub threshold: f64,
    pub duration_seconds: i64,
    pub channel_ids: String,
    pub enabled: bool,
    pub cooldown_minutes: i64,
    pub last_triggered: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub metric_type: String,
    pub condition: String,
    pub threshold: f64,
    pub duration_seconds: Option<i64>,
    pub channel_ids: Vec<i64>,
    pub cooldown_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAlertRuleRequest {
    pub name: Option<String>,
    pub metric_type: Option<String>,
    pub condition: Option<String>,
    pub threshold: Option<f64>,
    pub duration_seconds: Option<i64>,
    pub channel_ids: Option<Vec<i64>>,
    pub enabled: Option<bool>,
    pub cooldown_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AlertHistory {
    pub id: i64,
    pub rule_id: i64,
    pub rule_name: String,
    pub metric_type: String,
    pub metric_value: f64,
    pub threshold: f64,
    pub status: String,
    pub message: String,
    pub created_at: String,
}

// ─── RBAC 角色权限 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Permission {
    pub id: i64,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleWithPermissions {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub is_system: bool,
    pub permissions: Vec<Permission>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permission_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permission_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRoleRequest {
    pub user_id: i64,
    pub role: String,
}

// ─── 分页 / 通用 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
