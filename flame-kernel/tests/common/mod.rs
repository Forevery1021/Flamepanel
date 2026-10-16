//! 集成测试共享工具与路由组装（S12 拆分后的公共模块）。
//!
//! 各领域测试文件通过 `mod common;` 引入本模块，复用 `setup_router` /
//! `setup_full_router` / 鉴权头 / 临时目录 等工具，避免在每个文件重复复制。

#![allow(dead_code)]

use axum::{
    http::{header, HeaderName},
    Router,
};
use std::sync::Arc;

use flame_kernel::api::middleware;
use flame_kernel::api::{
    routes,
    types::{AppState, Services},
};
use flame_kernel::application::app_store_service::AppStoreService;
use flame_kernel::application::backup_service::BackupService;
use flame_kernel::application::scheduled_task_service::ScheduledTaskService;
use flame_kernel::application::service::*;
use flame_kernel::application::setup_service::SetupService;
use flame_kernel::application::task_service::TaskService;
use flame_kernel::domain::entity::*;
use flame_kernel::domain::repository::*;
use flame_kernel::event::EventBus;
use flame_kernel::infrastructure::db::*;
use flame_kernel::infrastructure::metrics::MetricsHub;
use flame_kernel::plugin::{PluginRegistry, PluginSandbox};
use flame_kernel::runtime::task_state::TaskTracker;
use flame_kernel::terminal::TerminalManager;
use flame_kernel::utils::auth_cache::AuthCache;
use flame_kernel::utils::jwt::JwtUtils;

/// 构建带管理员种子 + 全部服务的路由与状态。
pub fn auth_header() -> (HeaderName, String) {
    let jwt = JwtUtils::new("test-secret", 24);
    let token = jwt.sign(1).unwrap();
    (header::AUTHORIZATION, format!("Bearer {}", token))
}

/// 非法鉴权头（用于 401 测试）。
pub fn bad_auth_header() -> (HeaderName, String) {
    (header::AUTHORIZATION, "Bearer invalid_token".to_string())
}

/// 无操作进程守护（集成测试用：不触达真实 systemctl，全部操作成功）。
pub struct NoopSupervisor;

#[async_trait::async_trait]
impl flame_kernel::application::project_ports::ProcessSupervisor for NoopSupervisor {
    async fn ensure_unit(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<(), flame_kernel::core::error::AppError> {
        Ok(())
    }
    async fn remove_unit(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<(), flame_kernel::core::error::AppError> {
        Ok(())
    }
    async fn start(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<(), flame_kernel::core::error::AppError> {
        Ok(())
    }
    async fn stop(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<(), flame_kernel::core::error::AppError> {
        Ok(())
    }
    async fn restart(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<(), flame_kernel::core::error::AppError> {
        Ok(())
    }
    async fn status(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<
        flame_kernel::application::project_ports::ProcessStatus,
        flame_kernel::core::error::AppError,
    > {
        Ok(flame_kernel::application::project_ports::ProcessStatus {
            state: "stopped".into(),
            exit_code: None,
            restarts: 0,
        })
    }
    async fn logs(
        &self,
        _project: &flame_kernel::domain::entity::Project,
        _tail: usize,
    ) -> Result<
        Vec<flame_kernel::application::project_ports::LogLine>,
        flame_kernel::core::error::AppError,
    > {
        Ok(Vec::new())
    }
}

/// 集成测试健康探测 mock：恒定健康（端点正路径用；不健康分支由服务层单测覆盖）。
pub struct IntegrationProbe;

#[async_trait::async_trait]
impl flame_kernel::application::project_ports::HealthProbePort for IntegrationProbe {
    async fn wait_healthy(
        &self,
        _project: &flame_kernel::domain::entity::Project,
    ) -> Result<bool, flame_kernel::core::error::AppError> {
        Ok(true)
    }
}

/// 备份测试使用的临时目录（按 tag 区分）。
pub fn backup_temp_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("fp_backup_test_{}_{}", std::process::id(), tag))
}

/// bcrypt 哈希（登录/改密测试用）。
pub fn bcrypt_hash(pw: &str) -> String {
    flame_kernel::utils::password::PasswordUtils::hash(pw).unwrap()
}

/// AppStore 安装请求样例（容器模式）。
pub fn crate_install_req() -> flame_kernel::application::app_store_service::InstallRequest {
    serde_json::from_value(serde_json::json!({
        "package_key": "nginx",
        "version": "1.27",
        "mode": "container",
        "name": "event-nginx",
        "port": 18083,
        "values": { "PORT": "18083", "NAME": "event-nginx" },
        "confirm_risky": true
    }))
    .unwrap()
}

/// 组装最小路由 + 完整服务状态（未加中间件）。
pub async fn setup_router() -> (Router, AppState) {
    let user_repo = Arc::new(InMemoryUserRepository::new());
    let node_repo = Arc::new(InMemoryNodeRepository::new());
    let website_repo = Arc::new(InMemoryWebsiteRepository::new());
    let docker_repo = Arc::new(InMemoryDockerRepository::new());
    let perm_repo = Arc::new(InMemoryPermissionRepository::new());
    let role_repo = Arc::new(InMemoryRoleRepository::new());
    let log_repo = Arc::new(InMemoryOperationLogRepository::new());
    let sys_log_repo = Arc::new(InMemoryLogRepository::new());
    let web_server_repo = Arc::new(InMemoryWebServerRepository::new());
    let settings_repo = Arc::new(InMemorySettingsRepository::new());
    let database_repo = Arc::new(InMemoryDatabaseRepository::new());
    let firewall_repo = Arc::new(InMemoryFirewallRepository::new());
    let metrics_hub = Arc::new(MetricsHub::new(10));
    let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(16);
    let terminal_manager = TerminalManager::new();
    let plugin_sandbox = Arc::new(PluginSandbox::new());
    let plugin_registry = Arc::new(PluginRegistry::new());
    // Seed admin user for RBAC
    user_repo.create("admin", "hash", "admin").await.unwrap();
    let docker_service = Arc::new(DockerService::new(docker_repo));
    let runner: flame_kernel::application::execution_mode::SharedCommandRunner =
        std::sync::Arc::new(flame_kernel::infrastructure::execution::EmbeddedCommandRunner);
    let web_server_service = Arc::new(WebServerService::new(web_server_repo, runner.clone()));
    let database_service = Arc::new(DatabaseService::new(database_repo.clone(), runner.clone()));
    let database_repo_for_links = database_repo;
    let app_package_repo = Arc::new(InMemoryAppPackageRepository::new());
    let installed_app_repo = Arc::new(InMemoryInstalledAppRepository::new());
    let plugin_repo = Arc::new(InMemoryPluginRepository::new());
    // 设置服务单例：与 Services.settings_service 共享同一仓储实例
    //（对齐组合根 wire-up——否则商店安全配置端点写入的值服务层读不到）
    let settings_service = Arc::new(SettingsService::new(settings_repo));
    let app_store_service = Arc::new(
        AppStoreService::new(
            app_package_repo,
            installed_app_repo.clone(),
            docker_service.clone(),
            web_server_service.clone(),
            database_service.clone(),
            plugin_sandbox.clone(),
            plugin_registry.clone(),
            plugin_repo.clone(),
            AppStoreService::default_apps_dir(),
            EventBus::new(100),
            flame_kernel::infrastructure::app_store::default_ports(runner.clone()),
            Arc::new(InMemoryAppSourceRepository::new()),
        )
        // 商店安全配置（Doc/20 §4.2 收口）：验签策略 / 镜像白名单真实生效
        .with_settings_service(settings_service.clone()),
    );
    let backup_dir = backup_temp_dir("setup");
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::write(backup_dir.join("app.db"), b"backup-seed-db").unwrap();
    // 备份账号服务单例：备份账号 CRUD 端点和备份任务服务共享同一仓储实例
    // （对齐组合根 wire-up——否则任务/还原看不到通过 API 创建的账号）
    let backup_account_service = Arc::new(
        flame_kernel::application::backup_account_service::BackupAccountService::new(Arc::new(
            flame_kernel::infrastructure::db::InMemoryBackupAccountRepository::new(),
        )),
    );
    // 事件总线单例：WAF 服务与 Services.event_bus 共享同一实例（对齐组合根
    // wire-up——否则从测试侧订阅不到 WAF 拦截通知事件）
    let event_bus = EventBus::new(100);
    let services = Services {
        user_service: Arc::new(UserService::new(
            user_repo,
            EventBus::new(100),
            AuthCache::new(),
        )),
        node_service: Arc::new(NodeService::new(node_repo, EventBus::new(100))),
        website_service: Arc::new(WebsiteService::new(website_repo, EventBus::new(100))),
        docker_service,
        role_service: Arc::new(RoleService::new(
            role_repo,
            perm_repo.clone(),
            AuthCache::new(),
        )),
        permission_service: Arc::new(PermissionService::new(perm_repo)),
        operation_log_service: Arc::new(OperationLogService::new(log_repo)),
        outbox_service: Arc::new(OutboxService::new(
            Arc::new(InMemoryOutboxRepository::new()),
        )),
        memo_service: Arc::new(MemoService::new(Arc::new(InMemoryMemoRepository::new()))),
        log_service: Arc::new(LogService::new(sys_log_repo)),
        plugin_sandbox,
        plugin_registry,
        plugin_repo,
        app_store_service,
        web_server_service,
        settings_service: settings_service.clone(),
        database_service: database_service.clone(),
        firewall_service: Arc::new(FirewallService::new_embedded(firewall_repo)),
        backup_service: Arc::new(BackupService::new(
            backup_dir.join("app.db"),
            backup_dir.join("backups"),
        )),
        scheduled_task_service: Arc::new(ScheduledTaskService::new(Arc::new(
            InMemoryScheduledTaskRepository::new(),
        ))),
        task_service: Arc::new(TaskService::new(TaskTracker::new())),
        backup_account_service: backup_account_service.clone(),
        dns_provider_account_service: Arc::new(
            flame_kernel::application::dns_provider_service::DnsProviderAccountService::new(
                Arc::new(
                    flame_kernel::infrastructure::db::InMemoryDnsProviderAccountRepository::new(),
                ),
            )
            .with_probe_mode(false),
        ),
        // WAF 拦截通知：注入共享事件总线（对齐组合根 wire-up）
        waf_service: Arc::new(
            flame_kernel::application::waf_service::WafService::new(
                Arc::new(flame_kernel::infrastructure::db::InMemoryWafRuleRepository::new()),
                Arc::new(flame_kernel::infrastructure::db::InMemoryWafLogRepository::new()),
            )
            .with_event_bus(event_bus.clone()),
        ),
        backup_job_service: Arc::new(
            flame_kernel::application::backup_job_service::BackupJobService::new(
                Arc::new(flame_kernel::infrastructure::db::InMemoryBackupJobRepository::new()),
                backup_account_service.clone(),
                Arc::new(BackupService::new(
                    backup_dir.join("app.db"),
                    backup_dir.join("backups"),
                )),
                // 与 AppStoreService 共享同一仓储（对齐组合根）：应用备份还原
                // 依赖「已安装应用」记录定位安装目录
                installed_app_repo.clone(),
                EventBus::new(100),
            ),
        ),
        runtime_service: Arc::new(
            flame_kernel::application::runtime_service::RuntimeService::new(
                Arc::new(flame_kernel::infrastructure::db::InMemoryRuntimeRepository::new()),
                flame_kernel::application::runtime_ports::RuntimeInstallerRegistry::new(),
            ),
        ),
        project_service: Arc::new(
            flame_kernel::application::project_service::ProjectService::new(
                Arc::new(flame_kernel::infrastructure::db::InMemoryProjectRepository::new()),
                Arc::new(flame_kernel::infrastructure::db::InMemoryRuntimeRepository::new()),
                Arc::new(NoopSupervisor),
                runner.clone(),
            )
            // Stage A-9：集成测试注入恒定健康的探测 mock（含真实 HTTP 探测的
            // 单测在 HttpHealthProbe 内部，端到端回滚链路由服务层单测覆盖）
            .with_health_probe(std::sync::Arc::new(IntegrationProbe))
            // Stage A-10：注入数据库联链组件（内存仓储 + 嵌入式管理器 provider）
            .with_db_link(
                Arc::new(flame_kernel::infrastructure::db::InMemoryProjectDbLinkRepository::new()),
                database_repo_for_links.clone(),
                Arc::new(
                    flame_kernel::infrastructure::project_link::NativeDbLinkProvider::new(
                        database_service.mysql_manager.clone(),
                        database_service.redis_manager.clone(),
                        database_service.postgres_manager.clone(),
                    ),
                ),
                Arc::new(
                    flame_kernel::infrastructure::project_link::DatabaseServiceOps {
                        database_service: database_service.clone(),
                    },
                ),
            ),
        ),

        setup_service: Arc::new(SetupService::new(
            Arc::new(UserService::new(
                Arc::new(InMemoryUserRepository::new()),
                EventBus::new(100),
                AuthCache::new(),
            )),
            Arc::new(SettingsService::new(Arc::new(
                InMemorySettingsRepository::new(),
            ))),
            EventBus::new(100),
            std::path::PathBuf::from("."),
            runner.clone(),
            false,
        )),
        certificate_service: Arc::new(
            flame_kernel::application::certificate_service::CertificateService::new(
                Arc::new(InMemoryCertificateRepository::new()),
                EventBus::new(100),
                std::env::temp_dir().join(format!("fp-cert-svc-{}", uuid::Uuid::new_v4())),
            ),
        ),
        acme_account_key_store:
            flame_kernel::infrastructure::acme::account::AcmeAccountKeyStore::new(
                &std::env::temp_dir().join(format!("fp-acme-svc-{}", uuid::Uuid::new_v4())),
            ),
        event_bus,
        revoked_token_repo: Arc::new(InMemoryRevokedTokenRepository::new()),
        assistant_service: Arc::new(
            flame_kernel::application::assistant_service::AssistantService::new(
                settings_service.clone(),
                Some(flame_kernel::utils::SecretCipher::new(b"test-master-key")),
                flame_kernel::infrastructure::llm::factory(),
            ),
        ),
        two_factor_service: Arc::new(
            flame_kernel::application::two_factor_service::TwoFactorService::new(Arc::new(
                flame_kernel::infrastructure::db::InMemoryTwoFactorRepository::new(),
            )),
        ),
        mcp_service: Arc::new(flame_kernel::application::mcp_service::McpService::new(
            Arc::new(flame_kernel::infrastructure::db::InMemoryMcpRepository::new()),
        )),
        metrics_history_service: Arc::new(
            flame_kernel::application::metrics_history_service::MetricsHistoryService::new(
                Arc::new(flame_kernel::infrastructure::db::InMemoryMetricsHistoryRepository::new()),
            ),
        ),
        command_runner: Arc::new(flame_kernel::infrastructure::execution::EmbeddedCommandRunner),
    };
    let mut state = AppState::new(
        "test-secret".to_string(),
        services,
        metrics_hub,
        log_tx,
        terminal_manager,
    );
    // A3.2：测试状态显式配置 bootstrap token（节点注册端点鉴权用）
    state.bootstrap_token = "test-bootstrap-token".into();
    (routes::create_router(state.clone()), state)
}

/// 组装带完整中间件栈的路由（供绝大多数 HTTP 集成测试使用）。
pub async fn setup_full_router() -> Router {
    // 测试进程内所有 router 共享进程级限流器（OnceLock），调高阈值避免全量并行 429
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = setup_router().await;
    middleware::add_middleware_with_mock_peer(router, state)
}
