pub mod api;
pub mod application;
pub mod config;
pub mod core;
pub mod database;
pub mod domain;
pub mod event;
pub mod file;
pub mod infrastructure;
pub mod notification;
pub mod plugin;
pub mod resilience;
pub mod terminal;
pub mod utils;
pub mod webserver;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

use std::sync::Arc;
use tokio::sync::Mutex;

use api::types::{AppState, Services};
use application::app_store_service::AppStoreService;
use application::backup_service::{db_path_from_url, BackupService};
use application::scheduled_task_service::ScheduledTaskService;
use application::service::*;
use config::AppConfig;
use domain::entity::DomainEvent;
use domain::entity::LogEntry;
use domain::entity::MetricsSnapshot;
use event::{handler::EventHandler, EventBus};
use infrastructure::factory::RepoFactory;
use infrastructure::metrics::{spawn_metrics_collector, MetricsHistory};
use notification::{EmailNotifier, SmtpConfig};
use plugin::{PluginRegistry, PluginSandbox};
use terminal::TerminalManager;

pub struct FlameKernel {
    pub config: AppConfig,
    pub event_bus: EventBus,
    pub plugin_registry: Arc<PluginRegistry>,
    pub app_state: AppState,
}

impl FlameKernel {
    pub fn new(config: AppConfig) -> Self {
        Self::new_with_backend(config, RepoFactory::new_in_memory())
    }

    /// 从仓库工厂组装全部业务服务（应用层组合根）
    fn build_services(factory: &RepoFactory) -> Services {
        let event_bus = EventBus::new(100);
        let user_repo = factory.create_user_repo();
        let node_repo = factory.create_node_repo();
        let website_repo = factory.create_website_repo();
        let docker_repo = factory.create_docker_repo();
        let perm_repo = factory.create_permission_repo();
        let role_repo = factory.create_role_repo();
        let web_server_repo = factory.create_web_server_repo();
        let settings_repo = factory.create_settings_repo();
        let database_repo = factory.create_database_repo();
        let firewall_repo = factory.create_firewall_repo();
        let app_package_repo = factory.create_app_package_repo();
        let installed_app_repo = factory.create_installed_app_repo();
        let plugin_repo = factory.create_plugin_repo();
        let scheduled_task_repo = factory.create_scheduled_task_repo();

        let plugin_sandbox = Arc::new(PluginSandbox::new());
        let plugin_registry = Arc::new(PluginRegistry::new());

        let docker_service = Arc::new(DockerService::new(docker_repo));
        let web_server_service = Arc::new(WebServerService::new(web_server_repo));
        let database_service = Arc::new(DatabaseService::new(database_repo));

        let app_store_service = Arc::new(AppStoreService::new(
            app_package_repo,
            installed_app_repo,
            docker_service.clone(),
            web_server_service.clone(),
            database_service.clone(),
            plugin_sandbox.clone(),
            plugin_registry.clone(),
            plugin_repo.clone(),
            AppStoreService::default_apps_dir(),
            event_bus.clone(),
        ));

        Services {
            user_service: Arc::new(UserService::new(user_repo, event_bus.clone())),
            node_service: Arc::new(NodeService::new(node_repo, event_bus.clone())),
            website_service: Arc::new(WebsiteService::new(website_repo, event_bus.clone())),
            docker_service,
            role_service: Arc::new(RoleService::new(role_repo, perm_repo.clone())),
            permission_service: Arc::new(PermissionService::new(perm_repo)),
            operation_log_service: Arc::new(OperationLogService::new(
                factory.create_operation_log_repo(),
            )),
            memo_service: Arc::new(MemoService::new(factory.create_memo_repo())),
            log_service: Arc::new(LogService::new(factory.create_log_repo())),
            plugin_sandbox,
            plugin_registry,
            plugin_repo,
            app_store_service,
            web_server_service,
            settings_service: Arc::new(SettingsService::new(settings_repo)),
            database_service,
            firewall_service: Arc::new(FirewallService::new(firewall_repo)),
            scheduled_task_service: Arc::new(ScheduledTaskService::new(scheduled_task_repo)),
            backup_service: Arc::new(BackupService::new("data/app.db", "data/backups")),
            event_bus,
        }
    }

    pub fn new_with_backend(config: AppConfig, factory: RepoFactory) -> Self {
        let mut services = Self::build_services(&factory);
        // 用真实数据库路径覆盖备份服务（build_services 使用默认路径）
        let db_path = db_path_from_url(&config.database.url);
        services.backup_service = Arc::new(BackupService::new(db_path, "data/backups"));

        // 内置应用种子 + WASM 插件恢复（后台异步，不阻塞启动）
        let app_store_service_for_restore = services.app_store_service.clone();
        tokio::spawn(async move {
            let _ = app_store_service_for_restore.seed_builtin_apps().await;
            let _ = app_store_service_for_restore.restore_wasm_plugins().await;
        });

        // SQLite 模式补齐默认设置（后台异步，upsert 不覆盖用户已有配置）
        let factory_for_seed = factory;
        tokio::spawn(async move {
            let _ = factory_for_seed.seed_default_settings().await;
        });

        // 定时任务调度器（每 30 秒检查一次到期任务）
        let scheduled_task_service = services.scheduled_task_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = scheduled_task_service.tick().await {
                    tracing::warn!("Scheduled task tick failed: {}", e);
                }
            }
        });

        // 自动备份（settings 驱动：enabled / interval_hours / retention）
        let backup_service = services.backup_service.clone();
        let settings_service = services.settings_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let enabled = settings_service
                    .get("auto_backup_enabled")
                    .await
                    .ok()
                    .flatten()
                    .map(|v| v == "true")
                    .unwrap_or(false);
                if !enabled {
                    continue;
                }
                let interval_hours: u64 = settings_service
                    .get("auto_backup_interval_hours")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(24);
                let retention: usize = settings_service
                    .get("backup_retention")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(7);

                match backup_service.last_backup_age_secs().await {
                    Ok(Some(age)) if age < interval_hours * 3600 => {}
                    _ => {
                        if let Err(e) = backup_service.create_backup().await {
                            tracing::warn!("Auto backup failed: {}", e);
                        } else {
                            tracing::info!("Auto backup created (interval {}h)", interval_hours);
                            if let Err(e) = backup_service.enforce_retention(retention).await {
                                tracing::warn!("Backup retention cleanup failed: {}", e);
                            }
                        }
                    }
                }
            }
        });

        let metrics_history = Arc::new(Mutex::new(MetricsHistory::new(60)));
        let (metrics_tx, _) = tokio::sync::broadcast::channel::<MetricsSnapshot>(16);
        let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(256);
        spawn_metrics_collector(metrics_history.clone(), metrics_tx.clone());

        // 节点下线告警：扫描心跳超时（>30s）的节点，发布 NodeOffline（去重，仅告警一次）
        let node_service_for_offline = services.node_service.clone();
        let event_bus_for_offline = services.event_bus.clone();
        tokio::spawn(async move {
            use std::collections::HashSet;
            let mut alerted: HashSet<i64> = HashSet::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let nodes = match node_service_for_offline.list_nodes().await {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                for node in nodes {
                    let offline = !node.is_online(chrono::Utc::now(), 30);
                    if offline {
                        if alerted.insert(node.id) {
                            tracing::warn!("Node {} offline detected", node.name);
                            let _ = event_bus_for_offline
                                .publish(DomainEvent::NodeOffline {
                                    node_id: node.id,
                                    node_name: node.name.clone(),
                                })
                                .await;
                        }
                    } else {
                        alerted.remove(&node.id);
                    }
                }
            }
        });

        let terminal_manager = TerminalManager::new();

        // Wire up event handler with notification (subscribe before services is moved into AppState)
        let rx = services.event_bus.subscribe();
        let event_bus = services.event_bus.clone();
        let plugin_registry = services.plugin_registry.clone();
        let app_state = AppState::new(
            config.jwt_secret.clone(),
            services,
            metrics_history,
            metrics_tx,
            log_tx,
            terminal_manager,
        );

        let smtp_config = SmtpConfig {
            host: config.notifications.smtp_host.clone(),
            port: config.notifications.smtp_port,
            username: config.notifications.smtp_username.clone(),
            password: config.notifications.smtp_password.clone(),
            from: config.notifications.smtp_from.clone(),
            use_tls: config.notifications.smtp_tls,
        };
        let notifier = Arc::new(EmailNotifier::new(smtp_config));
        EventHandler::new().with_email(notifier).spawn(rx);

        Self {
            config,
            event_bus,
            plugin_registry,
            app_state,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("FlamePanel Kernel v{} starting...", VERSION);

        // Seed admin user if no users exist
        let users = self.app_state.user_service.list_users().await?;
        if users.is_empty() {
            let admin_password = &self.config.admin_password;
            let hash = crate::utils::password::PasswordUtils::hash(admin_password)?;
            let admin = self
                .app_state
                .user_service
                .create_user("admin", &hash, "admin")
                .await?;
            // 新装面板：种子 admin 首次登录强制改密
            self.app_state
                .user_service
                .set_must_change_password(admin.id, true)
                .await?;
            tracing::info!("Seeded admin user (password from config)");
        }

        let app = api::routes::create_router(self.app_state.clone());
        let app = api::middleware::add_middleware(app, self.app_state);

        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        tracing::info!("Listening on {}", addr);

        axum::Server::bind(&addr.parse()?)
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("FlamePanel Kernel shut down gracefully");
        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm => {},
    }
}
