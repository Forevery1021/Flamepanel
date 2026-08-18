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
pub mod openapi;
pub mod plugin;
pub mod resilience;
pub mod runtime;
pub mod terminal;
pub mod utils;
pub mod webserver;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

use std::sync::Arc;
use tokio::sync::Mutex;

use api::types::{AppState, Services};
use application::app_store_service::AppStoreService;
use application::backup_service::{db_path_from_url, BackupService, BackupServiceRef};
use application::scheduled_task_service::ScheduledTaskService;
use application::service::*;
use config::AppConfig;
use domain::entity::DomainEvent;
use domain::entity::LogEntry;
use domain::entity::MetricsSnapshot;
use event::{handler::EventHandler, EventBus};
use infrastructure::factory::RepoFactory;
use infrastructure::metrics::{metrics_collector_loop, MetricsHistory};
#[cfg(feature = "email")]
use notification::EmailNotifier;
#[cfg(feature = "email")]
use notification::SmtpConfig;
use plugin::{PluginRegistry, PluginSandbox};
use runtime::TaskSupervisor;
use terminal::TerminalManager;

pub struct FlameKernel {
    pub config: AppConfig,
    pub event_bus: EventBus,
    pub plugin_registry: Arc<PluginRegistry>,
    pub app_state: AppState,
    /// 后台任务 Supervisor：所有长生命周期任务统一注册，进程关闭时统一取消
    pub supervisor: TaskSupervisor,
}

impl FlameKernel {
    pub fn new(config: AppConfig) -> Self {
        Self::new_with_backend(config, RepoFactory::new_in_memory())
    }

    /// 从仓库工厂组装全部业务服务（应用层组合根）
    fn build_services(
        factory: &RepoFactory,
        command_runner: crate::application::execution_mode::SharedCommandRunner,
        mysql_config: &str,
        redis_config: &str,
    ) -> Services {
        let event_bus = EventBus::new(1024);
        // Stage 2（A4）：鉴权短缓存，User/Role 服务共享同一实例，写路径统一失效
        let auth_cache = crate::utils::auth_cache::AuthCache::new();
        let user_repo = factory.create_user_repo();
        let node_repo = factory.create_node_repo();
        let website_repo = factory.create_website_repo();
        // 六边形：Docker 端口按职责拆分，DockerService 经门面适配器组装各子端口
        let container_repo = factory.create_container_repo();
        let network_repo = factory.create_network_repo();
        let volume_repo = factory.create_volume_repo();
        let image_repo = factory.create_image_repo();
        let compose_repo = factory.create_compose_repo();
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

        let docker_service = Arc::new(DockerService::from_repos(
            container_repo,
            network_repo,
            volume_repo,
            image_repo,
            compose_repo,
        ));
        let database_service = Arc::new(DatabaseService::new_with_config_paths(
            database_repo,
            command_runner.clone(),
            mysql_config,
            redis_config,
        ));

        // Phase B1：多个长耗时服务共享同一个统一 Task 状态机跟踪器，
        // 供前端通过 `/api/tasks` 统一查询 / 取消任务进度。
        let shared_task_store = factory.create_task_store();
        let shared_task_tracker =
            crate::runtime::task_state::TaskTracker::with_store(shared_task_store);

        let web_server_service = Arc::new(WebServerService::with_task_tracker(
            web_server_repo,
            command_runner.clone(),
            shared_task_tracker.clone(),
        ));

        let app_store_service = Arc::new(AppStoreService::with_ports_and_shared_tracker(
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
            // 组合根注入：应用商店端口的具体实现（六边形：application 只依赖端口）
            Arc::new(infrastructure::app_store::adapter::DefaultAdapterProvider),
            Arc::new(
                infrastructure::app_store::security_scanner::DefaultComposeSecurityScanner::new(),
            ),
            Arc::new(
                infrastructure::app_store::variable_mapper::DefaultVariableMapperFactory::new(),
            ),
            Arc::new(infrastructure::os::DefaultPackageManagerPort::new(
                command_runner.clone(),
            )),
            Arc::new(infrastructure::os::DefaultServiceManagerPort::new(
                command_runner.clone(),
            )),
            command_runner.clone(),
            shared_task_tracker.clone(),
        ));

        let node_service = Arc::new(NodeService::with_task_tracker(
            node_repo,
            event_bus.clone(),
            shared_task_tracker.clone(),
        ));

        let task_service = Arc::new(crate::application::task_service::TaskService::new(
            shared_task_tracker,
        ));

        let user_service = Arc::new(UserService::new(
            user_repo,
            event_bus.clone(),
            auth_cache.clone(),
        ));
        let settings_service = Arc::new(SettingsService::new(settings_repo));
        let backup_service: BackupServiceRef =
            Arc::new(BackupService::new("data/app.db", "data/backups"));
        // B1：SetupService（data_dir 与无人值守标志由 new_with_backend 按 config 覆盖）
        let setup_service = Arc::new(crate::application::setup_service::SetupService::new(
            user_service.clone(),
            settings_service.clone(),
            event_bus.clone(),
            std::path::PathBuf::from("data"),
            command_runner.clone(),
            false,
        ));

        Services {
            user_service,
            node_service,
            website_service: Arc::new(WebsiteService::new(website_repo, event_bus.clone())),
            docker_service,
            role_service: Arc::new(RoleService::new(role_repo, perm_repo.clone(), auth_cache)),
            permission_service: Arc::new(PermissionService::new(perm_repo)),
            operation_log_service: Arc::new(OperationLogService::new(
                factory.create_operation_log_repo(),
            )),
            outbox_service: Arc::new(OutboxService::new(factory.create_outbox_repo())),
            memo_service: Arc::new(MemoService::new(factory.create_memo_repo())),
            log_service: Arc::new(LogService::new(factory.create_log_repo())),
            plugin_sandbox,
            plugin_registry,
            plugin_repo,
            app_store_service,
            web_server_service,
            settings_service,
            database_service,
            firewall_service: Arc::new(FirewallService::new(firewall_repo, command_runner)),
            scheduled_task_service: Arc::new(ScheduledTaskService::new(scheduled_task_repo)),
            task_service,
            backup_service,
            setup_service,
            event_bus,
        }
    }

    pub fn new_with_backend(config: AppConfig, factory: RepoFactory) -> Self {
        let mode = crate::application::execution_mode::ExecutionMode::from_str_loose(
            &config.execution_mode,
        );
        let command_runner =
            crate::infrastructure::execution::make_command_runner(mode, None, None);
        let mut services = Self::build_services(
            &factory,
            command_runner.clone(),
            &config.mysql_config_file,
            &config.redis_config_file,
        );
        // 用真实数据库路径覆盖备份服务（build_services 使用默认路径）
        let db_path = db_path_from_url(&config.database.url);
        services.backup_service = Arc::new(BackupService::new(db_path.clone(), "data/backups"));

        // B1：按 config 覆盖 SetupService 的 data_dir 与无人值守标志（admin_password 非空即无人值守）
        let data_dir = db_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("data"));
        services.setup_service = Arc::new(crate::application::setup_service::SetupService::new(
            services.user_service.clone(),
            services.settings_service.clone(),
            services.event_bus.clone(),
            data_dir,
            command_runner,
            !config.admin_password.is_empty(),
        ));

        // 后台任务 Supervisor（CancellationToken + JoinSet，统一生命周期）
        let mut supervisor = TaskSupervisor::new();

        // 内置应用种子 + WASM 插件恢复（后台异步，不阻塞启动；可取消）
        let app_store_service_for_restore = services.app_store_service.clone();
        supervisor.spawn("app-store-seed", |token| async move {
            tokio::select! {
                _ = token.cancelled() => {}
                _ = async {
                    let _ = app_store_service_for_restore.seed_builtin_apps().await;
                    let _ = app_store_service_for_restore.restore_wasm_plugins().await;
                } => {}
            }
        });

        // SQLite 模式补齐默认设置（后台异步，upsert 不覆盖用户已有配置）
        let factory_for_seed = factory;
        supervisor.spawn("settings-seed", |token| async move {
            tokio::select! {
                _ = token.cancelled() => {}
                _ = factory_for_seed.seed_default_settings() => {}
            }
        });

        // 定时任务调度器（每 30 秒检查一次到期任务；响应取消）
        let scheduled_task_service = services.scheduled_task_service.clone();
        supervisor.spawn("scheduled-task-tick", |token| async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {}
                }
                if let Err(e) = scheduled_task_service.tick().await {
                    tracing::warn!("Scheduled task tick failed: {}", e);
                }
            }
        });

        // 自动备份（settings 驱动：enabled / interval_hours / retention；响应取消）
        let backup_service = services.backup_service.clone();
        let settings_service = services.settings_service.clone();
        supervisor.spawn("auto-backup", |token| async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {}
                }
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
        let (metrics_tx, _) = tokio::sync::broadcast::channel::<MetricsSnapshot>(64);
        let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(256);
        {
            let history = metrics_history.clone();
            let tx = metrics_tx.clone();
            supervisor.spawn("metrics-collector", move |token| async move {
                metrics_collector_loop(history, tx, token).await;
            });
        }

        // 节点下线告警：扫描心跳超时（>30s）的节点，发布 NodeOffline（去重，仅告警一次；响应取消）
        let node_service_for_offline = services.node_service.clone();
        let event_bus_for_offline = services.event_bus.clone();
        supervisor.spawn("node-offline-scan", |token| async move {
            use std::collections::HashSet;
            let mut alerted: HashSet<i64> = HashSet::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {}
                }
                // 离线扫描条件化（Stage1）：按心跳阈值条件查询，避免全量 list_all 后过滤
                let stale_before = chrono::Utc::now() - chrono::Duration::seconds(30);
                let nodes = match node_service_for_offline
                    .list_stale_nodes(stale_before)
                    .await
                {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                for node in nodes {
                    if alerted.insert(node.id) {
                        tracing::warn!("Node {} offline detected", node.name);
                        let _ = event_bus_for_offline
                            .publish(DomainEvent::NodeOffline {
                                node_id: node.id,
                                node_name: node.name.clone(),
                            })
                            .await;
                    }
                }
            }
        });

        let terminal_manager =
            TerminalManager::with_cwd(std::path::PathBuf::from(&config.terminal_cwd));

        // Wire up event handler with notification (subscribe before services is moved into AppState)
        let rx = services.event_bus.subscribe();
        // 事件落库（Outbox）：独立订阅器把每条领域事件持久化存档，保证审计不丢（Stage6）
        let outbox_rx = services.event_bus.subscribe();
        let outbox_service = services.outbox_service.clone();
        supervisor.spawn("event-outbox", |token| async move {
            let mut rx = outbox_rx;
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::debug!("event outbox recorder shutting down");
                        break;
                    }
                    received = rx.recv() => {
                        match received {
                            Ok(event) => {
                                if let Err(e) = outbox_service.record_event(&event).await {
                                    tracing::error!("Outbox record failed for event {:?}: {}", event, e);
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                // T4：慢消费者滞后时不中断 outbox 记录，仅告警并继续。
                                tracing::warn!("outbox recorder lagged by {n} messages; continuing");
                            }
                            Err(_closed) => {
                                tracing::debug!("event channel closed; outbox recorder stopping");
                                break;
                            }
                        }
                    }
                }
            }
        });
        let event_bus = services.event_bus.clone();
        let plugin_registry = services.plugin_registry.clone();
        let app_state = AppState::new_with_roots(
            config.jwt_secret.clone(),
            services,
            metrics_history,
            metrics_tx,
            log_tx,
            terminal_manager,
            std::path::PathBuf::from(&config.file_root),
            std::path::PathBuf::from(&config.terminal_cwd),
            config.rate_limit_max,
            config.rate_limit_window_secs,
            config.bootstrap_token.clone(),
        );

        // 事件订阅经 Supervisor 注册（可取消，避免 shutdown 后残留循环）
        #[cfg(feature = "email")]
        {
            let smtp_config = SmtpConfig {
                host: config.notifications.smtp_host.clone(),
                port: config.notifications.smtp_port,
                username: config.notifications.smtp_username.clone(),
                password: config.notifications.smtp_password.clone(),
                from: config.notifications.smtp_from.clone(),
                use_tls: config.notifications.smtp_tls,
            };
            let notifier = Arc::new(EmailNotifier::new(smtp_config));
            supervisor.spawn("event-handler", move |token| async move {
                EventHandler::new()
                    .with_email("admin@flamepanel.local", notifier)
                    .spawn_with_token(rx, token);
            });
        }
        #[cfg(not(feature = "email"))]
        {
            // 关闭 email feature：事件仅记录日志，不尝试发送 SMTP 通知
            supervisor.spawn("event-handler", move |token| async move {
                EventHandler::new().spawn_with_token(rx, token);
            });
        }

        Self {
            config,
            event_bus,
            plugin_registry,
            app_state,
            supervisor,
        }
    }

    /// 初始化状态分叉（B4/A3.4）：
    ///
    /// - 无人值守（config.admin_password 非空，即 OP_ADMIN_PASSWORD 已设置）：
    ///   全量种子 admin + 写 `setup_completed_at` + 首次登录强制改密。
    /// - 向导模式（admin_password 为空）且用户表为空：进入「待初始化」状态，不做任何种子
    ///   （等待 Setup 向导完成；不再打印随机密码——向导模式下初始密码由向导创建）。
    /// - 老库兼容：users 非空但缺 `setup_completed_at` → 启动补写该键。
    ///
    /// 返回初始化结果供调用方记录日志。
    pub async fn bootstrap_initialization_state(
        &self,
    ) -> Result<InitializationOutcome, crate::core::error::AppError> {
        let users = self.app_state.user_service.list_users().await?;
        let setup_completed = self
            .app_state
            .settings_service
            .get("setup_completed_at")
            .await?
            .is_some();
        let unattended = !self.config.admin_password.is_empty();

        if users.is_empty() {
            if unattended {
                // 无人值守全量：种子 admin（密码来自配置/环境变量，不在日志重复打印——A3.4）
                let hash =
                    crate::utils::password::PasswordUtils::hash(&self.config.admin_password)?;
                let admin = self
                    .app_state
                    .user_service
                    .create_user("admin", &hash, "admin")
                    .await?;
                self.app_state
                    .user_service
                    .set_must_change_password(admin.id, true)
                    .await?;
                let now = chrono::Utc::now().to_rfc3339();
                self.app_state
                    .settings_service
                    .set_many(&[
                        ("setup_completed_at".into(), now),
                        ("theme".into(), "flame".into()),
                        ("language".into(), "zh-CN".into()),
                    ])
                    .await?;
                Ok(InitializationOutcome::SeededUnattended {
                    username: admin.username,
                })
            } else {
                Ok(InitializationOutcome::PendingWizard)
            }
        } else if !setup_completed {
            // 老库兼容：补写 setup_completed_at（用户已存在，说明面板此前已初始化过）
            let now = chrono::Utc::now().to_rfc3339();
            self.app_state
                .settings_service
                .set("setup_completed_at", &now)
                .await?;
            Ok(InitializationOutcome::LegacyBackfilled)
        } else {
            Ok(InitializationOutcome::Completed)
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("FlamePanel Kernel v{} starting...", VERSION);

        // JWT 密钥强度校验：短于 32 字节时拒绝启动（防暴力猜测签名密钥）
        if self.config.jwt_secret.len() < 32 {
            return Err(format!(
                "OP_JWT_SECRET must be at least 32 bytes (got {}); refusing to start. ",
                self.config.jwt_secret.len()
            )
            .into());
        }
        tracing::info!(
            "JWT secret strength check passed ({} bytes)",
            self.config.jwt_secret.len()
        );

        // 文件/终端沙箱根目录启动时校验（规范化后必须是已存在目录）
        let file_root = std::fs::canonicalize(&self.config.file_root)
            .map_err(|e| format!("Invalid OP_FILE_ROOT '{}': {}", self.config.file_root, e))?;
        if !file_root.is_dir() {
            return Err(format!(
                "OP_FILE_ROOT '{}' is not a directory",
                self.config.file_root
            )
            .into());
        }
        let terminal_cwd = std::fs::canonicalize(&self.config.terminal_cwd).map_err(|e| {
            format!(
                "Invalid OP_TERMINAL_CWD '{}': {}",
                self.config.terminal_cwd, e
            )
        })?;
        if !terminal_cwd.starts_with(&file_root) {
            return Err(format!(
                "OP_TERMINAL_CWD '{}' must be inside OP_FILE_ROOT '{}'",
                self.config.terminal_cwd, self.config.file_root
            )
            .into());
        }
        tracing::info!(
            "Sandbox roots: file_root={} terminal_cwd={}",
            file_root.display(),
            terminal_cwd.display()
        );

        // 初始化状态分叉（B4）：无人值守全量种子 / 向导模式待初始化 / 老库兼容补写
        match self.bootstrap_initialization_state().await? {
            InitializationOutcome::SeededUnattended { username } => {
                tracing::info!("Seeded admin user '{}' (unattended mode); password change required on first login", username);
            }
            InitializationOutcome::PendingWizard => {
                tracing::info!(
                    "Fresh install: waiting for setup wizard (GET /api/setup/status, POST /api/setup/initialize)"
                );
            }
            InitializationOutcome::LegacyBackfilled => {
                tracing::info!("Legacy database detected: backfilled setup_completed_at");
            }
            InitializationOutcome::Completed => {}
        }

        let app = api::routes::create_router(self.app_state.clone());
        let app = api::middleware::add_middleware(app, self.app_state);

        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        tracing::info!("Listening on {}", addr);

        // Axum 0.7+：使用 tokio TcpListener + axum::serve 替代已移除的 axum::Server
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        // 停止全部后台任务（CancellationToken 广播取消 + JoinSet 带超时 join）
        let mut supervisor = self.supervisor;
        let forced = supervisor.shutdown().await;
        tracing::info!("FlamePanel Kernel shut down gracefully");
        if forced > 0 {
            tracing::warn!(
                "{} background task(s) did not exit in time and were forcibly aborted",
                forced
            );
        }
        Ok(())
    }
}

/// 初始化状态分叉结果（`FlameKernel::bootstrap_initialization_state`）
#[derive(Debug)]
pub enum InitializationOutcome {
    /// 无人值守模式：已全量种子 admin
    SeededUnattended { username: String },
    /// 新装待向导（不做任何种子）
    PendingWizard,
    /// 已初始化（正常状态）
    Completed,
    /// 老库兼容：补写了 setup_completed_at
    LegacyBackfilled,
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
