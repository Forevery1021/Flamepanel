// Do not unwrap in production: recover locks with utils::sync::lock_or_recover;
// use ?, ok_or_else, or explicit fallbacks elsewhere. Tests are exempt.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod api;
pub mod application;
/// `fpctl` CLI; offline operations require SQLite.
#[cfg(feature = "sqlite")]
pub mod cli;
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

use api::types::{AppState, Services};
use application::app_store_service::AppStoreService;
use application::backup_service::{db_path_from_url, BackupService, BackupServiceRef};
use application::scheduled_task_service::ScheduledTaskService;
use application::service::*;
use config::AppConfig;
use domain::entity::DomainEvent;
use domain::entity::LogEntry;
use event::{handler::EventHandler, EventBus};
use infrastructure::factory::RepoFactory;
use infrastructure::metrics::{metrics_collector_loop, MetricsHub};
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
    /// Own all long-lived tasks and cancel them together on shutdown.
    pub supervisor: TaskSupervisor,
}

impl FlameKernel {
    pub fn new(config: AppConfig) -> Self {
        Self::new_with_backend(config, RepoFactory::new_in_memory())
    }

    /// Assemble business services from the repository factory at the composition root.
    fn build_services(
        factory: &RepoFactory,
        command_runner: crate::application::execution_mode::SharedCommandRunner,
        mysql_config: &str,
        redis_config: &str,
    ) -> Services {
        let event_bus = EventBus::new(1024);
        // Share the short-lived auth cache across User/Role services; invalidate on writes.
        let auth_cache = crate::utils::auth_cache::AuthCache::new();
        let user_repo = factory.create_user_repo();
        let node_repo = factory.create_node_repo();
        let website_repo = factory.create_website_repo();
        // Compose responsibility-specific Docker ports through a facade adapter.
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
        let database_repo_for_links = database_repo.clone();
        let firewall_repo = factory.create_firewall_repo();
        let app_package_repo = factory.create_app_package_repo();
        let installed_app_repo = factory.create_installed_app_repo();
        let app_source_repo = factory.create_app_source_repo();
        let plugin_repo = factory.create_plugin_repo();
        let scheduled_task_repo = factory.create_scheduled_task_repo();
        let backup_account_repo = factory.create_backup_account_repo();

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

        // Share one task tracker across long-running services for /api/tasks queries/cancellation.
        let shared_task_store = factory.create_task_store();
        let shared_task_tracker =
            crate::runtime::task_state::TaskTracker::with_store(shared_task_store);

        // Build settings before the store so signature and registry policies can be injected.
        let settings_service = Arc::new(SettingsService::new(settings_repo));

        let web_server_service = Arc::new(WebServerService::with_task_tracker(
            web_server_repo,
            command_runner.clone(),
            shared_task_tracker.clone(),
        ));

        let app_store_service = Arc::new(
            AppStoreService::with_ports_and_shared_tracker(
                app_package_repo,
                installed_app_repo.clone(),
                docker_service.clone(),
                web_server_service.clone(),
                database_service.clone(),
                plugin_sandbox.clone(),
                plugin_registry.clone(),
                plugin_repo.clone(),
                AppStoreService::default_apps_dir(),
                event_bus.clone(),
                // Inject concrete app-store ports here; application code depends only on interfaces.
                Arc::new(infrastructure::app_store::adapter::DefaultAdapterProvider),
                Arc::new(
                    infrastructure::app_store::security_scanner::DefaultComposeSecurityScanner::new(
                    ),
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
                app_source_repo,
            )
            // Inject settings so signature policy and registry allowlists are enforced.
            .with_settings_service(settings_service.clone()),
        );

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
        let backup_service: BackupServiceRef =
            Arc::new(BackupService::new("data/app.db", "data/backups"));
        // Persist the master in data/secret_encryption.key (0600) and reuse it on restart.
        // Losing it makes historical ciphertext unreadable; generating a replacement cannot recover it.
        let secret_cipher = match crate::utils::secret_box::load_or_create_master_secret(
            std::path::Path::new("./data"),
        ) {
            Ok((master, created)) => {
                if created {
                    tracing::info!(
                        "generated new secret encryption key at data/secret_encryption.key"
                    );
                }
                Some(crate::utils::SecretCipher::new(master.as_bytes()))
            }
            Err(e) => {
                tracing::warn!("secret encryption key unavailable, backup account credentials will be stored in plaintext: {e}");
                None
            }
        };
        let backup_account_service = Arc::new(
            crate::application::backup_account_service::BackupAccountService::new(
                backup_account_repo,
            )
            .with_cipher_opt(secret_cipher.clone()),
        );
        // Use the same at-rest credential policy as backup accounts.
        let dns_provider_account_service = Arc::new(
            crate::application::dns_provider_service::DnsProviderAccountService::new(
                factory.create_dns_provider_account_repo(),
            )
            .with_cipher_opt(secret_cipher.clone()),
        );
        // Manage WAF rules, engine lifecycle, and hit logs. Publish WafBlocked events
        // for outbox/notifications; aggregate alerts per rule over 300 seconds to avoid floods.
        let backup_job_service = Arc::new(
            crate::application::backup_job_service::BackupJobService::new(
                factory.create_backup_job_repo(),
                backup_account_service.clone(),
                backup_service.clone(),
                installed_app_repo.clone(),
                event_bus.clone(),
            ),
        );
        // Runtime installation orchestration and port registry.
        let runtime_service = Arc::new(crate::application::runtime_service::RuntimeService::new(
            factory.create_runtime_repo(),
            // Build/extract in panel-owned directories; only prefer_root steps use privileged ports.
            crate::infrastructure::runtime::default_runtime_registry(
                crate::infrastructure::execution::LocalUnlessRootRunner::shared(
                    command_runner.clone(),
                ),
            ),
        ));

        // Compose project lifecycle, health probes, events, and database bindings.
        // Inject connection strings at startup without logging credentials.
        let project_service = Arc::new(
            crate::application::project_service::ProjectService::new(
                factory.create_project_repo(),
                factory.create_runtime_repo(),
                Arc::new(infrastructure::supervisor::SystemdSupervisor::new(
                    command_runner.clone(),
                )),
                // Run project builds/deployments locally as the panel user, even in agent mode.
                crate::infrastructure::execution::LocalUnlessRootRunner::shared(
                    command_runner.clone(),
                ),
            )
            .with_health_probe(Arc::new(
                infrastructure::health_probe::HttpHealthProbe::new(),
            ))
            .with_event_bus(std::sync::Arc::new(event_bus.clone()))
            .with_db_link(
                factory.create_project_db_link_repo(),
                database_repo_for_links,
                Arc::new(infrastructure::project_link::NativeDbLinkProvider::new(
                    database_service.mysql_manager.clone(),
                    database_service.redis_manager.clone(),
                    database_service.postgres_manager.clone(),
                )),
                Arc::new(infrastructure::project_link::DatabaseServiceOps {
                    database_service: database_service.clone(),
                }),
            ),
        );

        // new_with_backend overrides setup data_dir and unattended mode from configuration.
        let setup_service = Arc::new(crate::application::setup_service::SetupService::new(
            user_service.clone(),
            settings_service.clone(),
            event_bus.clone(),
            std::path::PathBuf::from("data"),
            command_runner.clone(),
            false,
        ));

        // Compose issuance and ACME key storage; tests/memory use data by default,
        // while new_with_backend overrides the directory from configuration.
        let certificate_repo_for_certs = factory.create_certificate_repo();
        let website_repo_for_certs = factory.create_website_repo();
        let certificate_service = Arc::new(
            crate::application::certificate_service::CertificateService::new(
                certificate_repo_for_certs,
                event_bus.clone(),
                std::path::PathBuf::from("data"),
            )
            .with_website_repo(website_repo_for_certs)
            // Resolve issuance/renewal providers from certificate metadata and real stored credentials.
            .with_dns_accounts(dns_provider_account_service.clone()),
        );
        // Manage WAF rules, engine lifecycle, and hit logs. Publish WafBlocked events
        // for outbox/notifications; aggregate alerts per rule over 300 seconds to avoid floods.
        // Compile native Nginx/OpenResty rules and redeploy sites missing the include;
        // share certificate storage so redeployment preserves HTTPS.
        let waf_service = Arc::new(
            crate::application::waf_service::WafService::new(
                factory.create_waf_rule_repo(),
                factory.create_waf_log_repo(),
            )
            .with_event_bus(event_bus.clone())
            .with_enforcer(Arc::new(
                infrastructure::waf_enforcement::NativeWafEnforcer::new(
                    web_server_service.manager.clone(),
                    website_repo.clone(),
                )
                .with_site_writer(Arc::new(
                    infrastructure::site_config::DefaultSiteConfigWriter::new(
                        web_server_service.manager.clone(),
                    )
                    .with_certificate_repo(certificate_service.cert_repo.clone()),
                )),
            )),
        );
        let acme_account_key_store = crate::infrastructure::acme::account::AcmeAccountKeyStore::new(
            std::path::Path::new("data"),
        );

        Services {
            user_service,
            node_service,
            website_service: Arc::new(
                WebsiteService::new(website_repo, event_bus.clone())
                    .with_binding_repos(
                        factory.create_runtime_repo(),
                        factory.create_project_repo(),
                    )
                    // Render engine-specific vhosts, write atomically, validate, and reload.
                    // Include bound certificates in the pipeline so rewrite updates preserve HTTPS.
                    .with_site_config_writer(Arc::new(
                        infrastructure::site_config::DefaultSiteConfigWriter::new(
                            web_server_service.manager.clone(),
                        )
                        .with_certificate_repo(certificate_service.cert_repo.clone()),
                    )),
            ),
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
            assistant_service: Arc::new(application::assistant_service::AssistantService::new(
                settings_service.clone(),
                secret_cipher.clone(),
                crate::infrastructure::llm::factory(),
            )),
            settings_service,
            database_service,
            firewall_service: Arc::new(FirewallService::new(firewall_repo, command_runner.clone())),
            scheduled_task_service: Arc::new(ScheduledTaskService::new(scheduled_task_repo)),
            task_service,
            backup_service,
            backup_account_service,
            dns_provider_account_service,
            waf_service,
            backup_job_service,
            runtime_service,
            project_service,
            setup_service,
            event_bus,
            revoked_token_repo: factory.create_revoked_token_repo(),
            mcp_service: Arc::new(application::mcp_service::McpService::new(
                factory.create_mcp_repo(),
            )),
            two_factor_service: Arc::new(
                application::two_factor_service::TwoFactorService::new(
                    factory.create_two_factor_repo(),
                )
                .with_cipher_opt(secret_cipher.clone()),
            ),
            certificate_service,
            acme_account_key_store,
            metrics_history_service: Arc::new(
                application::metrics_history_service::MetricsHistoryService::new(
                    factory.create_metrics_history_repo(),
                ),
            ),
            command_runner,
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
        // Override the backup service with the actual database path.
        let db_path = db_path_from_url(&config.database.url);
        services.backup_service = Arc::new(BackupService::new(db_path.clone(), "data/backups"));

        // Override setup data_dir and unattended mode; a configured admin_password enables unattended setup.
        let data_dir = db_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("data"));
        services.setup_service = Arc::new(crate::application::setup_service::SetupService::new(
            services.user_service.clone(),
            services.settings_service.clone(),
            services.event_bus.clone(),
            data_dir.clone(),
            command_runner,
            !config.admin_password.is_empty(),
        ));

        // Use the actual data directory for certificate services and ACME keys.
        services.certificate_service = Arc::new(
            crate::application::certificate_service::CertificateService::new(
                services.certificate_service.cert_repo.clone(),
                services.event_bus.clone(),
                data_dir.clone(),
            )
            .with_website_repo(services.website_service.website_repo.clone())
            .with_dns_accounts(services.dns_provider_account_service.clone()),
        );
        services.acme_account_key_store =
            crate::infrastructure::acme::account::AcmeAccountKeyStore::new(&data_dir);

        // Register background tasks under one cancellation/join lifecycle.
        let mut supervisor = TaskSupervisor::new();

        // Seed built-in apps and restore WASM plugins asynchronously; support cancellation.
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

        // Fill missing SQLite defaults asynchronously; upsert must preserve user settings.
        let factory_for_seed = factory;
        supervisor.spawn("settings-seed", |token| async move {
            tokio::select! {
                _ = token.cancelled() => {}
                _ = factory_for_seed.seed_default_settings() => {}
            }
        });

        // Load enabled rules into the WAF engine at startup.
        {
            let waf_service_for_engine = services.waf_service.clone();
            supervisor.spawn("waf-engine-reload", move |token| async move {
                tokio::select! {
                    _ = token.cancelled() => {}
                    _ = async {
                        match waf_service_for_engine.reload_engine().await {
                            Ok(n) if n > 0 => tracing::info!("WAF engine loaded {n} enabled rules"),
                            Ok(_) => {}
                            Err(e) => tracing::warn!("WAF engine reload failed: {e}"),
                        }
                    } => {}
                }
            });
        }

        // Ingest WAF hit logs and redeploy when the site set changes.
        // OP_WAF_SYNC_SECS sets the interval (default 10 seconds; zero disables sync).
        let waf_sync_secs = std::env::var("OP_WAF_SYNC_SECS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(10);
        if waf_sync_secs > 0 {
            let task = crate::infrastructure::waf_sync::WafSyncTask::new(
                services.web_server_service.manager.clone(),
                services.website_service.website_repo.clone(),
                services.settings_service.repo.clone(),
                services.waf_service.clone(),
            );
            supervisor.spawn("waf-sync", move |token| async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(waf_sync_secs));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tokio::select! {
                        _ = token.cancelled() => break,
                        _ = interval.tick() => {}
                    }
                    let n = task.tick().await;
                    if n > 0 {
                        tracing::debug!("WAF sync imported {n} hit log entries");
                    }
                }
            });
        }

        // Check due scheduled tasks every 30 seconds; honor cancellation.
        let scheduled_task_service = services.scheduled_task_service.clone();
        supervisor.spawn("scheduled-task-tick", |token| async move {
            // Use Delay rather than Burst to avoid catch-up ticks after a stall.
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
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

        // Schedule cron backups to configured accounts; honor cancellation.
        let backup_job_service = services.backup_job_service.clone();
        supervisor.spawn("backup-job-tick", |token| async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {}
                }
                if let Err(e) = backup_job_service.tick().await {
                    tracing::warn!("Backup job tick failed: {}", e);
                }
            }
        });

        // Every six hours, scan the 30-day renewal window; renew ACME certificates
        // and emit 30/7-day expiry warnings. Honor cancellation.
        let certificate_service_for_renew = services.certificate_service.clone();
        supervisor.spawn("cert-renew-tick", |token| async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                crate::application::certificate_service::RENEW_SCAN_INTERVAL_HOURS * 3600,
            ));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {}
                }
                match certificate_service_for_renew.scan_and_renew().await {
                    Ok((renewed, failed)) if renewed > 0 || failed > 0 => {
                        tracing::info!(
                            "Certificate renew scan: {renewed} renewed, {failed} failed"
                        );
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Certificate renew scan failed: {}", e),
                }
            }
        });

        // Automatic backups follow enabled/interval_hours/retention settings and cancellation.
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

        // Keep 120 samples at three-second intervals (six minutes), matching HISTORY_LIMIT.
        // Collection and process endpoints own separate System instances to avoid lock contention.
        let metrics_hub = Arc::new(MetricsHub::new(120));
        // flame.metrics exposes the latest local snapshot only with metrics:read capability.
        services
            .plugin_sandbox
            .set_host_context(metrics_hub.clone() as Arc<dyn crate::plugin::host::HostContext>);
        let (log_tx, _) = tokio::sync::broadcast::channel::<LogEntry>(256);
        {
            let hub = metrics_hub.clone();
            supervisor.spawn("metrics-collector", move |token| async move {
                metrics_collector_loop(hub, token).await;
            });
        }
        // Aggregate three-second Hub snapshots into minute points and hourly rollups.
        {
            let history = services.metrics_history_service.clone();
            let rx = metrics_hub.subscribe();
            supervisor.spawn("metrics-history-writer", move |token| async move {
                history.run_writer(rx, token).await;
            });
        }

        // Clean expired audit/system logs and outbox events daily to bound SQLite growth.
        {
            let op_log_svc = services.operation_log_service.clone();
            let log_svc = services.log_service.clone();
            let outbox_svc = services.outbox_service.clone();
            let waf_service_for_retention = services.waf_service.clone();
            let settings_for_retention = services.settings_service.clone();
            supervisor.spawn("data-retention-cleanup", |token| async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400)); // 每日
                loop {
                    tokio::select! {
                        _ = token.cancelled() => break,
                        _ = interval.tick() => {}
                    }
                    // Read retention settings; defaults are 30 days for logs and 90 for audit.
                    let days = |key: &'static str, default: i64, min: i64| {
                        let svc = settings_for_retention.clone();
                        async move {
                            svc.get(key)
                                .await
                                .ok()
                                .flatten()
                                .and_then(|v| v.trim().parse::<i64>().ok())
                                .filter(|n| *n >= min)
                                .unwrap_or(default)
                        }
                    };
                    let audit_days = days("audit_log_retention_days", 90, 7).await;
                    let log_days = days("log_retention_days", 30, 1).await;
                    // Operation audit records.
                    match op_log_svc.cleanup_older_than(audit_days).await {
                        Ok(n) if n > 0 => tracing::info!("Cleanup: removed {n} operation_logs"),
                        _ => {}
                    }
                    // System logs.
                    match log_svc.cleanup_older_than(log_days).await {
                        Ok(n) if n > 0 => tracing::info!("Cleanup: removed {n} system logs"),
                        _ => {}
                    }
                    // Remove only published outbox events older than 30 days.
                    match outbox_svc.cleanup_older_than(30).await {
                        Ok(n) if n > 0 => tracing::info!("Cleanup: removed {n} outbox events"),
                        _ => {}
                    }
                    // WAF logs follow system-log retention.
                    match waf_service_for_retention
                        .cleanup_logs_older_than(log_days)
                        .await
                    {
                        Ok(n) if n > 0 => tracing::info!("Cleanup: removed {n} waf logs"),
                        _ => {}
                    }
                }
            });
        }

        // Prune expired revoked refresh tokens hourly; expiry filtering alone does not reclaim rows.
        {
            let revoked_repo = services.revoked_token_repo.clone();
            supervisor.spawn("revoked-token-cleanup", move |token| async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                loop {
                    tokio::select! {
                        _ = token.cancelled() => break,
                        _ = interval.tick() => {}
                    }
                    match revoked_repo.cleanup_expired().await {
                        Ok(()) => {}
                        Err(e) => tracing::warn!("revoked token cleanup failed: {}", e),
                    }
                }
            });
        }

        // Detect heartbeats older than 30 seconds and emit one deduplicated offline alert.
        // Honor cancellation.
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
                // Query by heartbeat threshold rather than filtering all nodes in memory.
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
        // Persist domain events through a separate outbox subscriber.
        // This is not an atomic business-transaction/outbox commit.
        let outbox_rx = services.event_bus.subscribe();
        let outbox_service = services.outbox_service.clone();
        let event_bus_for_outbox = services.event_bus.clone();
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
                                // Record lagged message counts and a compensation event for audit traceability.
                                event_bus_for_outbox.record_dropped(n);
                                tracing::warn!("outbox recorder lagged by {n} messages; writing compensation event");
                                // Record the number of lost events for audit traceability.
                                let compensation = crate::domain::entity::DomainEvent::AuditEvent {
                                    message: format!("OUTBOX_LAGGED:{}", n),
                                };
                                if let Err(e) = outbox_service.record_event(&compensation).await {
                                    tracing::error!("Failed to write outbox compensation event: {}", e);
                                }
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
        // Prune terminal tasks older than seven days daily to bound memory/table growth.
        let task_service_for_cleanup = services.task_service.clone();
        supervisor.spawn("task-cleanup", |token| async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
            interval.tick().await; // 首次立即执行一次清理
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = interval.tick() => {}
                }
                match task_service_for_cleanup
                    .tracker
                    .prune_terminal_older_than(7)
                    .await
                {
                    Ok(pruned) if pruned > 0 => tracing::info!(
                        "Task cleanup: removed {pruned} terminal task records older than 7 days"
                    ),
                    Ok(_) => {}
                    Err(error) => tracing::error!(%error, "Task cleanup persistence failed"),
                }
            }
        });

        let event_bus = services.event_bus.clone();
        let plugin_registry = services.plugin_registry.clone();
        let app_state = AppState::new_with_roots(
            config.jwt_secret.clone(),
            services,
            metrics_hub,
            log_tx,
            terminal_manager,
            std::path::PathBuf::from(&config.file_root),
            std::path::PathBuf::from(&config.terminal_cwd),
            config.rate_limit_max,
            config.rate_limit_window_secs,
            config.bootstrap_token.clone(),
            config.metrics_token.clone(),
        );

        // Register subscribers with the Supervisor so shutdown cancels their loops.
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
            // Without email support, log events without attempting SMTP delivery.
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

    /// Initialize according to deployment mode:
    /// - Configured admin_password: seed admin, set setup_completed_at, require password change.
    /// - Empty password and users: wait for setup; do not seed or print a generated password.
    /// - Existing users without setup_completed_at: backfill the marker for compatibility.
    ///
    /// Return the outcome for caller-side logging.
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
                // Seed admin from configuration/environment; never print the password again.
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
            // Backfill setup_completed_at when existing users prove prior initialization.
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

    /// Restore shared task state before listening; do not start with an empty tracker on failure.
    pub async fn restore_task_state(&self) -> Result<usize, crate::core::error::AppError> {
        self.app_state
            .task_service
            .tracker
            .load_from_store()
            .await
            .map(|records| records.len())
            .map_err(|e| {
                crate::core::error::AppError::internal(format!("Task startup recovery failed: {e}"))
            })
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        if let Err(error) = self.restore_task_state().await {
            let mut supervisor = self.supervisor;
            supervisor.shutdown().await;
            return Err(Box::new(error));
        }

        tracing::info!("FlamePanel Kernel v{} starting...", VERSION);

        // Reject JWT signing keys shorter than 32 bytes.
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

        // Create missing file/terminal roots, then canonicalize and verify existing directories.
        if !std::path::Path::new(&self.config.file_root).exists() {
            std::fs::create_dir_all(&self.config.file_root).map_err(|e| {
                format!(
                    "Failed to create OP_FILE_ROOT '{}': {}",
                    self.config.file_root, e
                )
            })?;
        }
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

        // Handle unattended setup, fresh wizard mode, and legacy-marker backfill.
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

        // Start bounded audit batching: at most 64 rows or 200 ms per batch.
        let audit_log_svc = self.app_state.operation_log_service.clone();
        api::middleware::init_audit_consumer(audit_log_svc);

        // Load access/session/lockout policies before listening to avoid an IP-policy gap after restart.
        match self.app_state.settings_service.reload_policy().await {
            Ok(p) if !p.allowed_ips.is_empty() || !p.bind_domains.is_empty() => tracing::info!(
                allowed_ips = p.allowed_ips.len(),
                bind_domains = p.bind_domains.len(),
                "panel access policy active"
            ),
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "failed to load panel access policy; using defaults")
            }
        }
        // Restore the saved log level only when OP_LOG_LEVEL/RUST_LOG are not explicitly set.
        if std::env::var_os("OP_LOG_LEVEL").is_none() && std::env::var_os("RUST_LOG").is_none() {
            if let Ok(Some(level)) = self.app_state.settings_service.get("log_level").await {
                if level.trim() == "info" {
                    // Avoid a redundant level change and its WARN log.
                } else if let Some(level) = crate::domain::kernel_log::LogLevel::parse(&level) {
                    let _ = self.app_state.kernel_log_service.set_level(level);
                }
            }
        }

        let task_tracker = self.app_state.task_service.tracker.clone();
        let app = api::routes::create_router(self.app_state.clone());
        let app = api::middleware::add_middleware(app, self.app_state);

        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        tracing::info!("Listening on {}", addr);

        // Serve through Tokio TcpListener and pass ConnectInfo so middleware can use
        // the TCP peer address when forwarded headers are not trusted.
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await?;

        // Cancel all background tasks and join them with a timeout.
        let mut supervisor = self.supervisor;
        let forced = supervisor.shutdown().await;
        // After stopping producers, drain accepted TaskStore writes, including disconnected requests.
        tokio::time::timeout(
            crate::runtime::supervisor::SHUTDOWN_TIMEOUT,
            task_tracker.flush(),
        )
        .await
        .map_err(|_| "timed out draining task persistence; startup recovery required")?;
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

/// Initialization outcome from FlameKernel::bootstrap_initialization_state.
#[derive(Debug)]
pub enum InitializationOutcome {
    /// Admin fully seeded in unattended mode.
    SeededUnattended { username: String },
    /// Fresh installation awaiting setup; no seeds created.
    PendingWizard,
    /// Already initialized.
    Completed,
    /// Legacy setup_completed_at marker backfilled.
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
