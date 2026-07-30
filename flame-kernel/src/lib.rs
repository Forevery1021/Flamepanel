pub mod config;
pub mod core;
pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod api;
pub mod event;
pub mod plugin;
pub mod utils;
pub mod notification;
pub mod resilience;
pub mod webserver;
pub mod database;
pub mod file;
pub mod terminal;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

use std::sync::Arc;
use tokio::sync::Mutex;

use config::AppConfig;
use application::service::*;
use api::types::AppState;
use event::{EventBus, handler::EventHandler};
use notification::{EmailNotifier, SmtpConfig};
use plugin::{PluginRegistry, PluginSandbox};
use terminal::TerminalManager;
use infrastructure::factory::RepoFactory;
use infrastructure::metrics::{MetricsHistory, spawn_metrics_collector};
use domain::entity::MetricsSnapshot;
use domain::entity::LogEntry;

pub struct FlameKernel {
    pub config: AppConfig,
    pub event_bus: EventBus,
    pub plugin_registry: PluginRegistry,
    pub app_state: AppState,
}

impl FlameKernel {
    pub fn new(config: AppConfig) -> Self {
        Self::new_with_backend(config, RepoFactory::new_in_memory())
    }

    pub fn new_with_backend(config: AppConfig, factory: RepoFactory) -> Self {
        let event_bus = EventBus::new(100);
        let plugin_registry = PluginRegistry::new();
        let plugin_sandbox = PluginSandbox::new();

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

        let user_service = UserService::new(user_repo);
        let node_service = NodeService::new(node_repo);
        let website_service = WebsiteService::new(website_repo);
        let docker_service = DockerService::new(docker_repo);
        let web_server_service = WebServerService::new(web_server_repo);
        let settings_service = SettingsService::new(settings_repo);
        let database_service = DatabaseService::new(database_repo);
        let firewall_service = FirewallService::new(firewall_repo);
        let role_service = RoleService::new(role_repo, perm_repo.clone());
        let permission_service = PermissionService::new(perm_repo);
        let log_repo = factory.create_operation_log_repo();
        let operation_log_service = OperationLogService::new(log_repo);
        let sys_log_repo = factory.create_log_repo();
        let log_service = LogService::new(sys_log_repo);

        let metrics_history = Arc::new(Mutex::new(MetricsHistory::new(60)));
        let (metrics_tx, _) = tokio::sync::broadcast::channel::<MetricsSnapshot>(16);
        let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(256);
        spawn_metrics_collector(metrics_history.clone(), metrics_tx.clone());

        let terminal_manager = TerminalManager::new();

        let plugin_sandbox_for_state = plugin_sandbox.clone();
        let plugin_registry_for_state = plugin_registry.clone();

        let app_state = AppState::new(
            config.jwt_secret.clone(),
            user_service,
            node_service,
            website_service,
            docker_service,
            role_service,
            permission_service,
            operation_log_service,
            log_service,
            metrics_history,
            metrics_tx,
            log_tx,
            plugin_sandbox_for_state,
            plugin_registry_for_state,
            web_server_service,
            settings_service,
            database_service,
            firewall_service,
            terminal_manager,
        );

        // Wire up event handler with notification
        let rx = event_bus.subscribe();
        let smtp_config = SmtpConfig {
            host: config.notifications.smtp_host.clone(),
            port: config.notifications.smtp_port,
            username: config.notifications.smtp_username.clone(),
            password: config.notifications.smtp_password.clone(),
            from: config.notifications.smtp_from.clone(),
            use_tls: config.notifications.smtp_tls,
        };
        let notifier = Arc::new(EmailNotifier::new(smtp_config));
        EventHandler::new()
            .with_email(notifier)
            .spawn(rx);

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
            self.app_state.user_service.create_user("admin", &hash, "admin").await?;
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