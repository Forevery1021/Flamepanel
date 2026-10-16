use crate::application::app_store_service::AppStoreService;
use crate::application::backup_account_service::BackupAccountService;
use crate::application::backup_job_service::BackupJobService;
use crate::application::backup_service::BackupServiceRef;
use crate::application::scheduled_task_service::ScheduledTaskService;
use crate::application::service::*;
use crate::application::setup_service::SetupService;
use crate::application::task_service::TaskService;
use crate::domain::entity::LogEntry;
use crate::domain::repository::PluginRepository;
use crate::event::EventBus;
use crate::infrastructure::metrics::MetricsHub;
use crate::plugin::{PluginRegistry, PluginSandbox};
use crate::terminal::TerminalManager;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    /// F-28: 业务服务聚合引用。所有服务字段已收敛为 Arc<Services>，
    /// 每请求克隆时只克隆一个 Arc 指针，而非逐字段复制。
    pub services: Arc<Services>,
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
    pub backup_account_service: Arc<BackupAccountService>,
    /// DNS 厂商账号服务（主线三 §6.1 · Stage 4）
    pub dns_provider_account_service:
        Arc<crate::application::dns_provider_service::DnsProviderAccountService>,
    /// WAF 基础防护服务（主线三 §6.2）
    pub waf_service: Arc<crate::application::waf_service::WafService>,
    pub backup_job_service: Arc<BackupJobService>,
    pub runtime_service: Arc<crate::application::runtime_service::RuntimeService>,
    pub project_service: Arc<crate::application::project_service::ProjectService>,
    /// P-01: 指标中枢（历史环形缓冲 + 预序列化广播 + 进程 TOP 缓存），
    /// 取代此前分散的 `metrics_history` / `metrics_tx` / `metrics_system` 三个字段。
    pub metrics: Arc<MetricsHub>,
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
    /// M-E 首片：复用既有仓储端口的只读配置清单导出
    pub iac_service: Arc<crate::application::iac_service::IacService>,
    /// S-06：顶栏概览聚合（TTL 缓存 + 单飞），由 `new_with_roots` 基于既有服务组装
    pub overview_service: Arc<crate::application::overview_service::OverviewService>,
    /// S-07: 监控历史查询（`GET /api/metrics/history`）
    pub metrics_history_service:
        Arc<crate::application::metrics_history_service::MetricsHistoryService>,
    /// S-17：系统体检（内核信息 + 风险分析，TTL 缓存 + 单飞），由 `new_with_roots` 组装
    pub inspection_service: Arc<crate::application::inspection_service::InspectionService>,
    /// S-12：系统工具箱 · 进程管理（进程表来自 `MetricsHub`，结束进程经命令执行器）
    pub toolbox_service: Arc<crate::application::toolbox_service::ToolboxService>,
    /// W0：内核日志文件查询（目录取全局日志初始化结果，测试可注入）
    pub kernel_log_service: Arc<crate::application::kernel_log_service::KernelLogService>,
    /// SSL 证书自动化（主线三 §6.1 · Stage 2）
    pub certificate_service: Arc<crate::application::certificate_service::CertificateService>,
    /// ACME 账户密钥存储（Stage 1；签发端点按需 ensure）
    pub acme_account_key_store: crate::infrastructure::acme::account::AcmeAccountKeyStore,
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
    /// F-24: /metrics 端点访问令牌（空 = 不校验）
    pub metrics_token: String,
    /// Stage 7（JWT 加固）：共享的 JwtUtils 实例，禁止每次请求 new；
    /// 基于启动时密钥构建；密钥轮换时整体替换实例（见 `rotate_secret`）。
    /// 热路径经 `shared_jwt()` 取共享读锁获取 `Arc<JwtUtils>`，并发不互斥。
    pub jwt_utils: Arc<std::sync::RwLock<Arc<crate::utils::jwt::JwtUtils>>>,
    /// M4/F-04: 已吊销 refresh token 持久化黑名单（SQLite 落库，重启后仍然生效）。
    pub revoked_token_repo: Arc<dyn crate::domain::repository::RevokedTokenRepository>,
    /// M4/F-04: 用户最近一次改密时间查询（持久化到 users 表）。
    pub user_repo_for_revocation: Arc<dyn crate::domain::repository::UserRepository>,
    /// M-AI-1：MCP 凭证 / 会话 / 调用审计（`/api/mcp` 协议端点与 AI 接入中心共用）
    pub mcp_service: Arc<crate::application::mcp_service::McpService>,
    /// Doc/33：TOTP 两步验证
    pub two_factor_service: Arc<crate::application::two_factor_service::TwoFactorService>,
    /// M-AI-3：面板内置 AI 助手（配置 + Provider 工厂）
    pub assistant_service: Arc<crate::application::assistant_service::AssistantService>,
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
    pub backup_account_service: Arc<BackupAccountService>,
    pub dns_provider_account_service:
        Arc<crate::application::dns_provider_service::DnsProviderAccountService>,
    /// WAF 基础防护服务（主线三 §6.2）
    pub waf_service: Arc<crate::application::waf_service::WafService>,
    pub backup_job_service: Arc<BackupJobService>,
    pub runtime_service: Arc<crate::application::runtime_service::RuntimeService>,
    pub project_service: Arc<crate::application::project_service::ProjectService>,
    pub event_bus: EventBus,
    /// 首次部署 Setup 服务（B1/B2）
    pub setup_service: Arc<SetupService>,
    /// F-04: 已吊销 refresh token 持久化黑名单
    pub revoked_token_repo: Arc<dyn crate::domain::repository::RevokedTokenRepository>,
    /// M-AI-1：MCP 凭证 / 会话 / 调用审计
    pub mcp_service: Arc<crate::application::mcp_service::McpService>,
    /// Doc/33：TOTP 两步验证
    pub two_factor_service: Arc<crate::application::two_factor_service::TwoFactorService>,
    /// M-AI-3：面板内置 AI 助手（配置 + Provider 工厂）
    pub assistant_service: Arc<crate::application::assistant_service::AssistantService>,
    /// SSL 证书自动化（主线三 §6.1 · Stage 2）
    pub certificate_service: Arc<crate::application::certificate_service::CertificateService>,
    /// ACME 账户密钥存储
    pub acme_account_key_store: crate::infrastructure::acme::account::AcmeAccountKeyStore,
    /// S-07: 监控历史持久化（1 min / 1 h 降采样点的写入与查询）
    pub metrics_history_service:
        Arc<crate::application::metrics_history_service::MetricsHistoryService>,
    /// 特权命令执行器（S-17 起在聚合中显式持有，供组合根按需组装体检 / 工具箱等服务）
    pub command_runner: crate::application::execution_mode::SharedCommandRunner,
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
        let (log_tx, _) = broadcast::channel::<LogEntry>(256);
        Self::new(
            jwt_secret,
            services,
            Arc::new(MetricsHub::new(120)),
            log_tx,
            TerminalManager::new(),
        )
    }

    pub fn new(
        jwt_secret: String,
        services: Services,
        metrics: Arc<MetricsHub>,
        log_tx: broadcast::Sender<LogEntry>,
        terminal_manager: TerminalManager,
    ) -> Self {
        Self::new_with_roots(
            jwt_secret,
            services,
            metrics,
            log_tx,
            terminal_manager,
            std::path::PathBuf::from("."),
            std::path::PathBuf::from("."),
            120,
            60,
            String::new(),
            String::new(),
        )
    }

    /// 带沙箱根目录的构造（组合根使用）
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_roots(
        jwt_secret: String,
        services: Services,
        metrics: Arc<MetricsHub>,
        log_tx: broadcast::Sender<LogEntry>,
        terminal_manager: TerminalManager,
        file_root: std::path::PathBuf,
        terminal_cwd: std::path::PathBuf,
        rate_limit_max: u64,
        rate_limit_window_secs: u64,
        bootstrap_token: String,
        metrics_token: String,
    ) -> Self {
        // F-04: 在移动 user_service 前先克隆其 repo 引用，用于持久化吊销校验
        let user_repo_for_revocation = services.user_service.user_repo.clone();
        let revoked_token_repo = services.revoked_token_repo.clone();
        // S-06：聚合服务只依赖既有服务句柄，在此组装可让 `Services {}` 的三处构造点零改动
        let overview_service =
            Arc::new(crate::application::overview_service::OverviewService::new(
                services.node_service.clone(),
                services.docker_service.clone(),
                services.task_service.clone(),
                services.certificate_service.clone(),
            ));
        // M-E：聚合只读仓储端口，不增加 Services 构造点或访问基础设施。
        let iac_service = Arc::new(crate::application::iac_service::IacService::new(
            services.website_service.website_repo.clone(),
            services.database_service.repo.clone(),
            services.app_store_service.installed_repo.clone(),
        ));
        // S-17：体检服务只依赖命令执行器 / 设置 / 指标中枢，同样在此组装
        let inspection_service = Arc::new(
            crate::application::inspection_service::InspectionService::new(
                crate::infrastructure::system_probe::SystemProbe::shared(
                    services.command_runner.clone(),
                ),
                services.settings_service.clone(),
                metrics.clone(),
                !metrics_token.is_empty(),
            ),
        );
        // S-12：进程管理复用指标中枢的进程表（与仪表盘 TOP 共用 sysinfo::System）
        let toolbox_service = Arc::new(
            crate::application::toolbox_service::ToolboxService::new(
                metrics.clone(),
                services.command_runner.clone(),
            )
            .with_host_source(crate::infrastructure::host_probe::HostProbe::shared()),
        );
        Self {
            // F-28: 保存 services 聚合引用，供 handler 经 state.services 访问
            services: Arc::new(services.clone()),
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
            metrics,
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
            overview_service,
            iac_service,
            inspection_service,
            toolbox_service,
            kernel_log_service: Arc::new(
                crate::application::kernel_log_service::KernelLogService::new(),
            ),
            metrics_history_service: services.metrics_history_service,
            backup_service: services.backup_service,
            backup_account_service: services.backup_account_service.clone(),
            dns_provider_account_service: services.dns_provider_account_service.clone(),
            waf_service: services.waf_service.clone(),
            backup_job_service: services.backup_job_service.clone(),
            runtime_service: services.runtime_service.clone(),
            project_service: services.project_service.clone(),
            certificate_service: services.certificate_service.clone(),
            acme_account_key_store: services.acme_account_key_store.clone(),
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
            metrics_token,
            revoked_token_repo,
            user_repo_for_revocation,
            mcp_service: services.mcp_service.clone(),
            two_factor_service: services.two_factor_service.clone(),
            assistant_service: services.assistant_service.clone(),
        }
    }
}

#[derive(Clone)]
pub struct UserId(pub i64);

/// 请求上下文中已认证用户的用户名（由认证中间件注入，供审计/日志复用）。
#[derive(Clone)]
pub struct Username(pub String);

/// 请求上下文中已认证用户的角色名（由认证中间件注入）。
/// 供聚合类端点在 handler 内做**字段级**权限裁剪（如 `GET /api/overview/badges`），
/// 路由级 RBAC 仍由中间件统一完成，handler 不得据此放宽访问。
#[derive(Clone)]
pub struct UserRole(pub String);
