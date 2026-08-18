use crate::application::app_store_service::AppStoreService;
use crate::application::backup_service::BackupServiceRef;
use crate::application::scheduled_task_service::ScheduledTaskService;
use crate::application::service::*;
use crate::application::setup_service::SetupService;
use crate::application::task_service::TaskService;
use crate::domain::entity::{LogEntry, MetricsSnapshot};
use crate::domain::repository::PluginRepository;
use crate::event::EventBus;
use crate::infrastructure::metrics::MetricsHistory;
use crate::plugin::{PluginRegistry, PluginSandbox};
use crate::terminal::TerminalManager;
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
    pub outbox_service: Arc<OutboxService>,
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
    pub task_service: Arc<TaskService>,
    pub terminal_manager: Arc<TerminalManager>,
    /// 首次部署 Setup 服务（B1/B2）
    pub setup_service: Arc<SetupService>,
    /// 登录失败锁定存储（进程内）
    pub login_attempts: Arc<crate::api::login_attempt::LoginAttemptStore>,
    /// 事件总线（handler 层发布业务事件）
    pub event_bus: crate::event::EventBus,
    /// 文件/终端沙箱白名单根目录
    pub file_root: std::path::PathBuf,
    /// 终端默认工作目录
    pub terminal_cwd: std::path::PathBuf,
    /// 限流：普通 API 每窗口请求上限
    pub rate_limit_max: u64,
    /// 限流窗口秒数
    pub rate_limit_window_secs: u64,
    /// 当前 JWT 签名密钥（支持运行时轮换，读取时加锁）
    pub jwt_secret_store: Arc<std::sync::RwLock<String>>,
    /// A3.2：节点注册引导令牌（Agent 注册端点 `POST /api/nodes/register` 鉴权用）
    pub bootstrap_token: String,
    /// Stage 7（JWT 加固）：共享的 JwtUtils 实例，禁止每次请求 new；
    /// 基于启动时密钥构建；密钥轮换时整体替换实例（见 `rotate_secret`）。
    /// 热路径经 `shared_jwt()` 取共享读锁获取 `Arc<JwtUtils>`，并发不互斥。
    pub jwt_utils: Arc<std::sync::RwLock<Arc<crate::utils::jwt::JwtUtils>>>,
}

/// 业务服务聚合（统一由 FlameKernel::build_services 组装，测试可自建）
#[derive(Clone)]
pub struct Services {
    pub user_service: Arc<UserService>,
    pub node_service: Arc<NodeService>,
    pub website_service: Arc<WebsiteService>,
    pub docker_service: Arc<DockerService>,
    pub role_service: Arc<RoleService>,
    pub permission_service: Arc<PermissionService>,
    pub operation_log_service: Arc<OperationLogService>,
    pub outbox_service: Arc<OutboxService>,
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
    pub task_service: Arc<TaskService>,
    pub backup_service: BackupServiceRef,
    pub event_bus: EventBus,
    /// 首次部署 Setup 服务（B1/B2）
    pub setup_service: Arc<SetupService>,
}

impl AppState {
    /// 当前生效的 JWT 签名密钥（支持运行时轮换）
    pub fn current_jwt_secret(&self) -> String {
        self.jwt_secret_store
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| self.jwt_secret.clone())
    }

    /// Stage 7（JWT 加固）：取共享 JwtUtils 实例（共享读锁，热路径不互斥），
    /// 禁止每次请求 new。密钥轮换后 `rotate_secret` 会替换底层实例。
    pub fn shared_jwt(&self) -> Arc<crate::utils::jwt::JwtUtils> {
        self.jwt_utils
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| Arc::new(crate::utils::jwt::JwtUtils::new_pair(&self.jwt_secret)))
    }

    /// 从业务服务聚合直接构造（默认沙箱根目录 = 当前目录，方便测试/快速组装）。
    ///
    /// Stage2.4：将 metrics/log channel 的默认创建内聚于此，减少组合根样板。
    pub fn from_services(jwt_secret: String, services: Services) -> Self {
        let metrics_history = Arc::new(Mutex::new(MetricsHistory::new(60)));
        let (metrics_tx, _) = broadcast::channel::<MetricsSnapshot>(16);
        let (log_tx, _) = broadcast::channel::<LogEntry>(256);
        Self::new(
            jwt_secret,
            services,
            metrics_history,
            metrics_tx,
            log_tx,
            TerminalManager::new(),
        )
    }

    pub fn new(
        jwt_secret: String,
        services: Services,
        metrics_history: Arc<Mutex<MetricsHistory>>,
        metrics_tx: broadcast::Sender<MetricsSnapshot>,
        log_tx: broadcast::Sender<LogEntry>,
        terminal_manager: TerminalManager,
    ) -> Self {
        Self::new_with_roots(
            jwt_secret,
            services,
            metrics_history,
            metrics_tx,
            log_tx,
            terminal_manager,
            std::path::PathBuf::from("."),
            std::path::PathBuf::from("."),
            120,
            60,
            String::new(),
        )
    }

    /// 带沙箱根目录的构造（组合根使用）
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_roots(
        jwt_secret: String,
        services: Services,
        metrics_history: Arc<Mutex<MetricsHistory>>,
        metrics_tx: broadcast::Sender<MetricsSnapshot>,
        log_tx: broadcast::Sender<LogEntry>,
        terminal_manager: TerminalManager,
        file_root: std::path::PathBuf,
        terminal_cwd: std::path::PathBuf,
        rate_limit_max: u64,
        rate_limit_window_secs: u64,
        bootstrap_token: String,
    ) -> Self {
        Self {
            jwt_secret: jwt_secret.clone(),
            user_service: services.user_service,
            node_service: services.node_service,
            website_service: services.website_service,
            docker_service: services.docker_service,
            role_service: services.role_service,
            permission_service: services.permission_service,
            operation_log_service: services.operation_log_service,
            outbox_service: services.outbox_service,
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
            task_service: services.task_service,
            backup_service: services.backup_service,
            terminal_manager: Arc::new(terminal_manager),
            setup_service: services.setup_service,
            login_attempts: Arc::new(crate::api::login_attempt::LoginAttemptStore::new()),
            event_bus: services.event_bus.clone(),
            file_root,
            terminal_cwd,
            rate_limit_max,
            rate_limit_window_secs,
            jwt_secret_store: Arc::new(std::sync::RwLock::new(jwt_secret.clone())),
            jwt_utils: Arc::new(std::sync::RwLock::new(Arc::new(
                crate::utils::jwt::JwtUtils::new_pair(&jwt_secret),
            ))),
            bootstrap_token,
        }
    }
}

#[derive(Clone)]
pub struct UserId(pub i64);

/// 请求上下文中已认证用户的用户名（由认证中间件注入，供审计/日志复用）。
#[derive(Clone)]
pub struct Username(pub String);
