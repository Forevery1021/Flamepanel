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
