use crate::application::app_store_service::AppStoreService;
use crate::application::backup_service::BackupServiceRef;
use crate::application::scheduled_task_service::ScheduledTaskService;
use crate::application::service::*;
use crate::domain::entity::{LogEntry, MetricsSnapshot};
use crate::domain::repository::PluginRepository;
use crate::event::EventBus;
use crate::infrastructure::metrics::MetricsHistory;
use crate::plugin::{PluginRegistry, PluginSandbox};
use crate::terminal::TerminalManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub jwt_secret: String,
    pub user_service: Arc<UserService>,
    pub node_service: Arc<NodeService>,
    pub website_service: Arc<WebsiteService>,
    pub docker_service: Arc<DockerService>,
    pub role_service: Arc<RoleService>,
    pub permission_service: Arc<PermissionService>,
    pub operation_log_service: Arc<OperationLogService>,
    pub memo_service: Arc<MemoService>,
    pub log_service: Arc<LogService>,
    pub backup_service: BackupServiceRef,
    pub metrics_history: Arc<Mutex<MetricsHistory>>,
    pub metrics_tx: broadcast::Sender<MetricsSnapshot>,
    pub log_tx: broadcast::Sender<LogEntry>,
    pub plugin_sandbox: Arc<PluginSandbox>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub plugin_repo: Arc<dyn PluginRepository>,
    pub app_store_service: Arc<AppStoreService>,
    pub web_server_service: Arc<WebServerService>,
    pub settings_service: Arc<SettingsService>,
    pub database_service: Arc<DatabaseService>,
    pub firewall_service: Arc<FirewallService>,
    pub scheduled_task_service: Arc<ScheduledTaskService>,
    pub terminal_manager: Arc<TerminalManager>,
    /// 登录失败锁定存储（进程内）
    pub login_attempts: Arc<crate::api::login_attempt::LoginAttemptStore>,
    /// 事件总线（handler 层发布业务事件）
    pub event_bus: crate::event::EventBus,
}

#[derive(serde::Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateUserRequest {
    pub username: String,
    pub password_hash: Option<String>,
    pub role: String,
}

/// 节点注册请求：兼容两种格式
/// - 面板/测试：`{"node": {name, hostname, ip_address, status, ...}}`
/// - Agent 平铺：`{name, host, agent_port, auth_token}`
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    #[serde(default)]
    pub node: Option<crate::domain::entity::ServerNode>,
    // 平铺字段（Agent 格式：{name, host, agent_port, auth_token}）
    #[serde(default)]
    pub name: String,
    #[serde(default, alias = "host")]
    pub hostname: String,
    #[serde(default)]
    pub ip_address: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub auth_token: Option<String>,
}

impl CreateNodeRequest {
    pub fn to_node(&self) -> crate::domain::entity::ServerNode {
        if let Some(node) = &self.node {
            return node.clone();
        }
        crate::domain::entity::ServerNode {
            id: 0,
            name: if self.name.is_empty() { self.hostname.clone() } else { self.name.clone() },
            hostname: self.hostname.clone(),
            ip_address: self.ip_address.clone(),
            status: if self.status.is_empty() { "unknown".into() } else { self.status.clone() },
            created_at: chrono::Utc::now(),
            last_heartbeat_at: None,
            metrics_json: None,
            auth_token: self.auth_token.clone(),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct CreateWebsiteRequest {
    pub website: crate::domain::entity::Website,
}

#[derive(Debug, Serialize)]
pub struct WebServerResponse {
    pub id: i64,
    pub engine: String,
    pub version: Option<String>,
    pub status: String,
    pub config_path: String,
    pub binary_path: Option<String>,
    pub port: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PluginSettingRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct PluginMetricsResponse {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_ms: f64,
    pub max_execution_ms: u64,
    pub min_execution_ms: u64,
    pub last_execution_ms: u64,
    pub peak_memory_bytes: usize,
}

#[derive(Debug, Deserialize)]
pub struct PluginReloadRequest {
    pub wasm_base64: String,
    pub memory_limit_bytes: Option<usize>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebServerInstanceRequest {
    pub engine: String,
    pub version: Option<String>,
    pub config_path: Option<String>,
    pub binary_path: Option<String>,
    pub port: Option<i32>,
}

/// 业务服务聚合（统一由 FlameKernel::build_services 组装，测试可自建）
pub struct Services {
    pub user_service: Arc<UserService>,
    pub node_service: Arc<NodeService>,
    pub website_service: Arc<WebsiteService>,
    pub docker_service: Arc<DockerService>,
    pub role_service: Arc<RoleService>,
    pub permission_service: Arc<PermissionService>,
    pub operation_log_service: Arc<OperationLogService>,
    pub memo_service: Arc<MemoService>,
    pub log_service: Arc<LogService>,
    pub plugin_sandbox: Arc<PluginSandbox>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub plugin_repo: Arc<dyn PluginRepository>,
    pub app_store_service: Arc<AppStoreService>,
    pub web_server_service: Arc<WebServerService>,
    pub settings_service: Arc<SettingsService>,
    pub database_service: Arc<DatabaseService>,
    pub firewall_service: Arc<FirewallService>,
    pub scheduled_task_service: Arc<ScheduledTaskService>,
    pub backup_service: BackupServiceRef,
    pub event_bus: EventBus,
}

impl AppState {
    pub fn new(
        jwt_secret: String,
        services: Services,
        metrics_history: Arc<Mutex<MetricsHistory>>,
        metrics_tx: broadcast::Sender<MetricsSnapshot>,
        log_tx: broadcast::Sender<LogEntry>,
        terminal_manager: TerminalManager,
    ) -> Self {
        Self {
            jwt_secret,
            user_service: services.user_service,
            node_service: services.node_service,
            website_service: services.website_service,
            docker_service: services.docker_service,
            role_service: services.role_service,
            permission_service: services.permission_service,
            operation_log_service: services.operation_log_service,
            memo_service: services.memo_service,
            log_service: services.log_service,
            metrics_history,
            metrics_tx,
            log_tx,
            plugin_sandbox: services.plugin_sandbox,
            plugin_registry: services.plugin_registry,
            plugin_repo: services.plugin_repo,
            app_store_service: services.app_store_service,
            web_server_service: services.web_server_service,
            settings_service: services.settings_service,
            database_service: services.database_service,
            firewall_service: services.firewall_service,
            scheduled_task_service: services.scheduled_task_service,
            backup_service: services.backup_service,
            terminal_manager: Arc::new(terminal_manager),
            login_attempts: Arc::new(crate::api::login_attempt::LoginAttemptStore::new()),
            event_bus: services.event_bus.clone(),
        }
    }
}

#[derive(Clone)]
pub struct UserId(pub i64);

pub fn route_permission(
    method: &axum::http::Method,
    path: &str,
) -> Option<(&'static str, &'static str)> {
    let path = path.trim_end_matches('/');
    match (method.as_str(), path) {
        ("GET", "/api/users") => Some(("user", "read")),
        ("POST", "/api/users") => Some(("user", "create")),
        ("PUT", p) if p.starts_with("/api/users/") => Some(("user", "update")),
        ("DELETE", p) if p.starts_with("/api/users/") => Some(("user", "delete")),
        ("GET", "/api/nodes") => Some(("node", "read")),
        ("POST", "/api/nodes") => Some(("node", "create")),
        ("GET", p) if p.starts_with("/api/nodes/") => Some(("node", "read")),
        ("PUT", p) if p.starts_with("/api/nodes/") => Some(("node", "update")),
        ("DELETE", p) if p.starts_with("/api/nodes/") => Some(("node", "delete")),
        ("GET", "/api/websites") => Some(("website", "read")),
        ("POST", "/api/websites") => Some(("website", "create")),
        ("GET", p) if p.starts_with("/api/websites/") => Some(("website", "read")),
        ("PUT", p) if p.starts_with("/api/websites/") => Some(("website", "update")),
        ("DELETE", p) if p.starts_with("/api/websites/") => Some(("website", "delete")),
        ("GET", "/api/docker/containers") => Some(("docker", "read")),
        ("GET", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/logs") => {
            Some(("docker", "read"))
        }
        ("GET", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/stats") => {
            Some(("docker", "read"))
        }
        ("GET", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/inspect") => {
            Some(("docker", "read"))
        }
        ("POST", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/start") => {
            Some(("docker", "start"))
        }
        ("POST", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/stop") => {
            Some(("docker", "stop"))
        }
        ("POST", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/restart") => {
            Some(("docker", "start"))
        }
        ("POST", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/remove") => {
            Some(("docker", "delete"))
        }
        ("POST", p) if p.starts_with("/api/docker/containers/") && p.ends_with("/rename") => {
            Some(("docker", "update"))
        }
        ("POST", p)
            if p.starts_with("/api/docker/containers/")
                && (p.ends_with("/pause") || p.ends_with("/unpause") || p.ends_with("/kill")) =>
        {
            Some(("docker", "start"))
        }
        ("POST", "/api/docker/containers/prune") => Some(("docker", "delete")),
        ("GET", "/api/docker/images") => Some(("docker", "read")),
        ("POST", "/api/docker/images/pull") => Some(("docker", "start")),
        ("POST", p) if p.starts_with("/api/docker/images/") && p.ends_with("/remove") => {
            Some(("docker", "delete"))
        }
        ("POST", p) if p.starts_with("/api/docker/images/") && p.ends_with("/tag") => {
            Some(("docker", "update"))
        }
        ("POST", "/api/docker/images/prune") => Some(("docker", "delete")),
        ("GET", "/api/docker/networks") => Some(("docker", "read")),
        ("POST", "/api/docker/networks") => Some(("docker", "create")),
        ("DELETE", p) if p.starts_with("/api/docker/networks/") && !p.ends_with("/prune") => {
            Some(("docker", "delete"))
        }
        ("POST", p) if p.starts_with("/api/docker/networks/") && p.ends_with("/connect") => {
            Some(("docker", "update"))
        }
        ("POST", p) if p.starts_with("/api/docker/networks/") && p.ends_with("/disconnect") => {
            Some(("docker", "update"))
        }
        ("POST", "/api/docker/networks/prune") => Some(("docker", "delete")),
        ("GET", "/api/docker/volumes") => Some(("docker", "read")),
        ("POST", "/api/docker/volumes") => Some(("docker", "create")),
        ("DELETE", p) if p.starts_with("/api/docker/volumes/") && !p.ends_with("/prune") => {
            Some(("docker", "delete"))
        }
        ("POST", "/api/docker/volumes/prune") => Some(("docker", "delete")),
        ("GET", "/api/docker/compose") => Some(("docker", "read")),
        ("POST", "/api/docker/compose/deploy") => Some(("docker", "start")),
        ("POST", p) if p.starts_with("/api/docker/compose/") && p.ends_with("/up") => {
            Some(("docker", "start"))
        }
        ("POST", p) if p.starts_with("/api/docker/compose/") && p.ends_with("/down") => {
            Some(("docker", "stop"))
        }
        ("GET", "/api/plugins") => Some(("plugin", "read")),
        ("POST", "/api/plugins") => Some(("plugin", "create")),
        ("GET", p)
            if p == "/api/plugins"
                || (p.starts_with("/api/plugins/")
                    && !p.contains("/execute/")
                    && !p.contains("/enable")
                    && !p.contains("/disable")
                    && !p.contains("/settings")
                    && !p.contains("/metrics")
                    && !p.contains("/reload")) =>
        {
            Some(("plugin", "read"))
        }
        ("POST", p) if p.starts_with("/api/plugins/") && p.ends_with("/enable") => {
            Some(("plugin", "create"))
        }
        ("POST", p) if p.starts_with("/api/plugins/") && p.ends_with("/disable") => {
            Some(("plugin", "create"))
        }
        ("POST", p) if p.starts_with("/api/plugins/") && p.contains("/execute/") => {
            Some(("plugin", "execute"))
        }
        ("POST", p)
            if p.starts_with("/api/plugins/")
                && !p.ends_with("/enable")
                && !p.ends_with("/disable")
                && !p.contains("/execute/")
                && !p.contains("/reload")
                && !p.contains("/settings") =>
        {
            Some(("plugin", "delete"))
        }
        ("POST", p) if p.starts_with("/api/plugins/") && p.contains("/reload") => {
            Some(("plugin", "create"))
        }
        ("GET", p) if p.starts_with("/api/plugins/") && p.contains("/settings") => {
            Some(("plugin", "config"))
        }
        ("POST", p) if p.starts_with("/api/plugins/") && p.contains("/settings") => {
            Some(("plugin", "config"))
        }
        ("GET", p) if p.starts_with("/api/plugins/") && p.contains("/metrics") => {
            Some(("plugin", "read"))
        }
        ("DELETE", p) if p.starts_with("/api/plugins/") && p.contains("/metrics") => {
            Some(("plugin", "config"))
        }
        ("GET", "/api/web-servers/engines") => Some(("web_server", "read")),
        ("GET", "/api/web-servers") => Some(("web_server", "read")),
        ("POST", "/api/web-servers") => Some(("web_server", "create")),
        ("GET", p)
            if p.starts_with("/api/web-servers/")
                && !p.contains("/start")
                && !p.contains("/stop")
                && !p.contains("/restart")
                && !p.contains("/reload")
                && !p.contains("/configtest")
                && !p.contains("/config") =>
        {
            Some(("web_server", "read"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/start") => {
            Some(("web_server", "start"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/stop") => {
            Some(("web_server", "stop"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/restart") => {
            Some(("web_server", "start"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/reload") => {
            Some(("web_server", "reload"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/configtest") => {
            Some(("web_server", "configtest"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/switch-engine") => {
            Some(("web_server", "update"))
        }
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/preset") => {
            Some(("web_server", "update"))
        }
        ("GET", "/api/web-servers/native/detect") => Some(("web_server", "read")),
        ("POST", "/api/web-servers/native/install") => Some(("web_server", "create")),
        ("POST", "/api/web-servers/native/uninstall") => Some(("web_server", "delete")),
        ("POST", "/api/web-servers/native/autostart") => Some(("web_server", "update")),
        ("POST", p) if p.starts_with("/api/web-servers/") && p.ends_with("/autostart") => {
            Some(("web_server", "update"))
        }
        ("GET", p) if p.starts_with("/api/web-servers/") && p.ends_with("/native-status") => {
            Some(("web_server", "read"))
        }
        ("GET", "/api/web-servers/presets") => Some(("web_server", "read")),
        ("POST", p) if p.starts_with("/api/websites/") && p.ends_with("/switch-engine") => {
            Some(("website", "update"))
        }
        ("GET", p) if p.starts_with("/api/web-servers/") && p.contains("/config") => {
            Some(("web_server", "read"))
        }
        ("PUT", p)
            if p.starts_with("/api/web-servers/")
                && !p.contains("/start")
                && !p.contains("/stop")
                && !p.contains("/restart")
                && !p.contains("/reload")
                && !p.contains("/configtest")
                && !p.contains("/config") =>
        {
            Some(("web_server", "update"))
        }
        ("DELETE", p)
            if p.starts_with("/api/web-servers/")
                && !p.contains("/start")
                && !p.contains("/stop")
                && !p.contains("/restart")
                && !p.contains("/reload")
                && !p.contains("/configtest")
                && !p.contains("/config") =>
        {
            Some(("web_server", "delete"))
        }
        ("GET", "/api/settings") => Some(("settings", "read")),
        ("GET", p) if p.starts_with("/api/settings/") => Some(("settings", "read")),
        ("PUT", "/api/settings") => Some(("settings", "update")),
        ("GET", "/api/databases") => Some(("database", "read")),
        ("GET", p)
            if p.starts_with("/api/databases/")
                && !p.contains("/start")
                && !p.contains("/stop")
                && !p.contains("/restart")
                && !p.contains("/status")
                && !p.contains("/uninstall")
                && !p.contains("/databases/")
                && !p.contains("/users") =>
        {
            Some(("database", "read"))
        }
        ("DELETE", p) if p.starts_with("/api/databases/") && !p.contains("/uninstall") => {
            Some(("database", "delete"))
        }
        ("POST", p) if p.ends_with("/install") => Some(("database", "create")),
        ("POST", p) if p.ends_with("/start") => Some(("database", "start")),
        ("POST", p) if p.ends_with("/stop") => Some(("database", "stop")),
        ("POST", p) if p.ends_with("/restart") => Some(("database", "start")),
        ("GET", p) if p.contains("/status") => Some(("database", "read")),
        ("POST", p) if p.contains("/databases") && !p.contains("/delete") => {
            Some(("database", "create"))
        }
        ("DELETE", p) if p.contains("/databases/") => Some(("database", "delete")),
        ("POST", p) if p.contains("/users") => Some(("database", "update")),
        ("DELETE", p) if p.contains("/users/") => Some(("database", "update")),
        ("POST", p) if p.ends_with("/uninstall") => Some(("database", "delete")),
        ("GET", "/api/files") => Some(("file", "read")),
        ("GET", "/api/app-store/packages") => Some(("app_store", "read")),
        ("GET", "/api/app-store/wasm-builtins") => Some(("app_store", "read")),
        ("GET", p) if p.starts_with("/api/app-store/packages/") => Some(("app_store", "read")),
        ("POST", p) if p.starts_with("/api/app-store/packages/") && p.ends_with("/install") => {
            Some(("app_store", "create"))
        }
        ("POST", p) if p.starts_with("/api/app-store/packages/") && p.ends_with("/import") => {
            Some(("app_store", "create"))
        }
        ("GET", "/api/app-store/installed") => Some(("app_store", "read")),
        ("GET", p)
            if p.starts_with("/api/app-store/installed/")
                && !p.ends_with("/upgrade")
                && !p.ends_with("/uninstall")
                && !p.ends_with("/logs") =>
        {
            Some(("app_store", "read"))
        }
        ("GET", p) if p.starts_with("/api/app-store/installed/") && p.ends_with("/logs") => {
            Some(("app_store", "read"))
        }
        ("POST", p) if p.starts_with("/api/app-store/installed/") && p.ends_with("/upgrade") => {
            Some(("app_store", "update"))
        }
        ("POST", p) if p.starts_with("/api/app-store/installed/") && p.ends_with("/launch") => {
            Some(("app_store", "read"))
        }
        ("POST", p) if p.starts_with("/api/app-store/installed/") && p.ends_with("/uninstall") => {
            Some(("app_store", "delete"))
        }
        ("GET", "/api/files/read") => Some(("file", "read")),
        ("GET", "/api/files/download") => Some(("file", "upload")),
        ("POST", "/api/files/write") => Some(("file", "write")),
        ("POST", "/api/files/create-file") => Some(("file", "write")),
        ("POST", "/api/files/create-dir") => Some(("file", "write")),
        ("DELETE", "/api/files/delete") => Some(("file", "write")),
        ("POST", "/api/files/rename") => Some(("file", "write")),
        ("POST", "/api/files/chmod") => Some(("file", "write")),
        ("POST", "/api/files/upload") => Some(("file", "upload")),
        ("GET", "/api/firewall/rules") => Some(("firewall", "read")),
        ("GET", p) if p.starts_with("/api/firewall/rules/") && !p.ends_with("/toggle") => {
            Some(("firewall", "read"))
        }
        ("POST", "/api/firewall/rules") => Some(("firewall", "create")),
        ("PUT", p) if p.starts_with("/api/firewall/rules/") => Some(("firewall", "update")),
        ("DELETE", p) if p.starts_with("/api/firewall/rules/") => Some(("firewall", "delete")),
        ("POST", p) if p.ends_with("/toggle") => Some(("firewall", "enable")),
        ("POST", "/api/firewall/apply") => Some(("firewall", "apply")),
        ("GET", "/api/firewall/status") => Some(("firewall", "read")),
        ("POST", "/api/firewall/enable") => Some(("firewall", "enable")),
        ("POST", "/api/firewall/disable") => Some(("firewall", "enable")),
        ("POST", "/api/firewall/reorder") => Some(("firewall", "update")),
        ("GET", "/api/memos") => Some(("memo", "read")),
        ("POST", "/api/memos") => Some(("memo", "create")),
        ("PUT", p) if p.starts_with("/api/memos/") => Some(("memo", "update")),
        ("DELETE", p) if p.starts_with("/api/memos/") => Some(("memo", "delete")),
        ("GET", "/api/operation-logs") => Some(("operation_log", "read")),
        ("DELETE", p) if p.starts_with("/api/operation-logs/") => Some(("operation_log", "delete")),
        ("GET", "/api/logs") => Some(("log", "read")),
        ("DELETE", p) if p.starts_with("/api/logs/") => Some(("log", "delete")),
        ("GET", "/api/backups") => Some(("backup", "read")),
        ("POST", "/api/backups") => Some(("backup", "create")),
        ("GET", p) if p.starts_with("/api/backups/") => Some(("backup", "read")),
        ("DELETE", p) if p.starts_with("/api/backups/") => Some(("backup", "delete")),
        ("POST", p) if p.starts_with("/api/backups/") => Some(("backup", "create")),
        ("GET", "/api/scheduled-tasks") => Some(("scheduled_task", "read")),
        ("POST", "/api/scheduled-tasks") => Some(("scheduled_task", "create")),
        ("GET", p) if p.starts_with("/api/scheduled-tasks/") && !p.ends_with("/run") => {
            Some(("scheduled_task", "read"))
        }
        ("PUT", p) if p.starts_with("/api/scheduled-tasks/") => Some(("scheduled_task", "update")),
        ("DELETE", p) if p.starts_with("/api/scheduled-tasks/") => {
            Some(("scheduled_task", "delete"))
        }
        ("POST", p) if p.starts_with("/api/scheduled-tasks/") && p.ends_with("/run") => {
            Some(("scheduled_task", "execute"))
        }
        ("POST", p) if p.starts_with("/api/scheduled-tasks/") && p.ends_with("/toggle") => {
            Some(("scheduled_task", "update"))
        }
        _ => None,
    }
}

// ── Pagination ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

impl PaginationParams {
    pub fn page(&self) -> i64 {
        self.page.filter(|p| *p > 0).unwrap_or(1)
    }
    pub fn page_size(&self) -> i64 {
        self.page_size.filter(|s| *s > 0 && *s <= 200).unwrap_or(20)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.page_size()
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, params: &PaginationParams) -> Self {
        let page = params.page();
        let page_size = params.page_size();
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            1
        };
        Self {
            data: items,
            page,
            page_size,
            total,
            total_pages,
        }
    }
}

/// Slice a full Vec into a paginated window.
pub fn paginate_slice<T: Clone>(items: &[T], params: &PaginationParams) -> Vec<T> {
    let start = params.offset() as usize;
    let end = (start + params.page_size() as usize).min(items.len());
    if start < items.len() {
        items[start..end].to_vec()
    } else {
        vec![]
    }
}
