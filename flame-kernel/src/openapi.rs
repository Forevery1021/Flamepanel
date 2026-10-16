//! OpenAPI 文档（Stage3.3）。
//!
//! 聚合关键路由的 `#[utoipa::path]` 注解与核心 DTO（`ToSchema`），编译期生成
//! OpenAPI 3.x 文档。组合根挂载 `/api/openapi.json` 与（可选 feature）`/api/swagger-ui`。

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// OpenAPI 文档（核心路由 + DTO）。
#[derive(OpenApi)]
#[openapi(
    info(
        title = "FlamePanel API",
        description = "FlamePanel 面板后端 REST API（OpenAPI 3.x，由 utoipa 编译期生成）",
        version = crate::VERSION
    ),
    servers(
        (url = "/", description = "默认")
    ),
    paths(
        // auth
        crate::api::handler::auth::login,
        crate::api::handler::auth::refresh,
        crate::api::handler::auth::logout,
        crate::api::handler::auth::me,
        crate::api::handler::auth::change_password,
        crate::api::handler::auth::rotate_secret,
        crate::api::handler::auth::login_two_factor,
        crate::api::handler::auth::two_factor::status,
        crate::api::handler::auth::two_factor::setup,
        crate::api::handler::auth::two_factor::enable,
        crate::api::handler::auth::two_factor::disable,
        crate::api::handler::auth::two_factor::regenerate_recovery_codes,
        crate::api::handler::auth::two_factor::admin_reset,
        // setup（首装向导，公开）
        crate::api::handler::setup::status,
        crate::api::handler::setup::initialize,
        // health
        crate::api::handler::health::detail,
        crate::api::handler::health::liveness,
        // users
        crate::api::handler::user::list,
        crate::api::handler::user::create,
        crate::api::handler::user::update,
        crate::api::handler::user::delete,
        // nodes
        crate::api::handler::node::list,
        crate::api::handler::node::update,
        crate::api::handler::node::delete,
        crate::api::handler::node::metrics,
        crate::api::handler::node::metrics_history,
        crate::api::handler::node::create,
        crate::api::handler::node::register_agent,
        crate::api::handler::node::heartbeat,
        crate::api::handler::node::status,
        crate::api::handler::node::remote_execute,
        crate::api::handler::node::remote_action,
        crate::api::handler::node::batch_execute,
        crate::api::handler::node::remote_list_files,
        crate::api::handler::node::remote_download_file,
        crate::api::handler::node::remote_upload_file,
        // websites
        crate::api::handler::website::list,
        crate::api::handler::website::create,
        crate::api::handler::website::get,
        crate::api::handler::website::update,
        crate::api::handler::website::delete,
        // websites 绑定闭环（M-B 主线二 Stage A-6 · Doc/20 §5.5）
        crate::api::handler::website::bind_runtime,
        crate::api::handler::website::switch_engine,
        crate::api::handler::website::deploy,
        crate::api::handler::website::apply_rewrite,
        crate::api::handler::website::get_rewrite,
        crate::api::handler::website::clear_rewrite,
        // settings
        crate::api::handler::settings::list_settings,
        crate::api::handler::settings::update_setting,
        crate::api::handler::settings::get_setting,
        crate::api::handler::settings::update_settings_batch,
        crate::api::handler::settings::access_info,
        // backups
        crate::api::handler::backup::download_backup,
        crate::api::handler::backup::restore_backup,
        // memos / logs / operation-logs
        crate::api::handler::memo::list,
        crate::api::handler::memo::create,
        crate::api::handler::memo::update,
        crate::api::handler::memo::delete,
        crate::api::handler::log::list,
        crate::api::handler::log::delete,
        crate::api::handler::operation_log::list,
        crate::api::handler::operation_log::delete,
        crate::api::handler::backup::list_backups,
        crate::api::handler::backup::create_backup,
        crate::api::handler::backup::delete_backup,
        // DNS 厂商账号（主线三 §6.1 · Stage 4）
        crate::api::handler::dns_provider::list_accounts,
        crate::api::handler::dns_provider::create_account,
        crate::api::handler::dns_provider::update_account,
        crate::api::handler::dns_provider::delete_account,
        crate::api::handler::dns_provider::test_account,
        // WAF 基础防护（主线三 §6.2）
        crate::api::handler::waf::list_rules,
        crate::api::handler::waf::create_rule,
        crate::api::handler::waf::get_rule,
        crate::api::handler::waf::update_rule,
        crate::api::handler::waf::delete_rule,
        crate::api::handler::waf::list_logs,
        crate::api::handler::waf::clear_logs,
        crate::api::handler::waf::status,
        crate::api::handler::waf::apply_rules,
        // backup accounts（M-A 收尾 Stage 1）
        crate::api::handler::backup_account::list_accounts,
        crate::api::handler::backup_account::create_account,
        crate::api::handler::backup_account::update_account,
        crate::api::handler::backup_account::delete_account,
        crate::api::handler::backup_account::test_account,
        crate::api::handler::backup_account::list_remote_objects,
        crate::api::handler::backup_account::upload_remote_object,
        crate::api::handler::backup_account::delete_remote_object,
        // backup jobs（M-A 收尾 Stage 3）
        crate::api::handler::backup_job::list_jobs,
        crate::api::handler::backup_job::create_job,
        crate::api::handler::backup_job::update_job,
        crate::api::handler::backup_job::delete_job,
        crate::api::handler::backup_job::toggle_job,
        crate::api::handler::backup_job::run_job,
        crate::api::handler::backup_job::restore_from_cloud,
        // scheduled tasks
        crate::api::handler::scheduled_task::list_tasks,
        crate::api::handler::scheduled_task::create_task,
        crate::api::handler::scheduled_task::update_task,
        crate::api::handler::scheduled_task::delete_task,
        crate::api::handler::scheduled_task::run_task,
        crate::api::handler::scheduled_task::get_task,
        crate::api::handler::scheduled_task::toggle_task,
        // app-store
        crate::api::handler::app_store::list_packages,
        crate::api::handler::app_store::get_package,
        crate::api::handler::app_store::list_versions,
        crate::api::handler::app_store::install,
        crate::api::handler::app_store::import_package,
        crate::api::handler::app_store::get_installed,
        crate::api::handler::app_store::launch,
        crate::api::handler::app_store::uninstall,
        crate::api::handler::app_store::upgrade,
        crate::api::handler::app_store::upgrade_preview,
        crate::api::handler::app_store::get_logs,
        crate::api::handler::app_store::native_status,
        crate::api::handler::app_store::native_start,
        crate::api::handler::app_store::native_stop,
        crate::api::handler::app_store::native_restart,
        crate::api::handler::app_store::list_wasm_builtins,
        crate::api::handler::app_store::list_sources,
        crate::api::handler::app_store::create_source,
        crate::api::handler::app_store::update_source,
        crate::api::handler::app_store::delete_source,
        crate::api::handler::app_store::sync_source,
        crate::api::handler::app_store::list_source_packages,
        crate::api::handler::app_store::list_installed,
        crate::api::handler::app_store::batch_import_packages,
        // 商店安全配置（Doc/20 §4.2 收口）
        crate::api::handler::app_store::get_store_security,
        crate::api::handler::app_store::update_store_security,
        // projects（M-B 主线二 Stage A-3）
        crate::api::handler::project::list_projects,
        crate::api::handler::project::create_project,
        crate::api::handler::project::get_project,
        crate::api::handler::project::update_project,
        crate::api::handler::project::delete_project,
        crate::api::handler::project::deploy_project,
        crate::api::handler::project::start_project,
        crate::api::handler::project::stop_project,
        crate::api::handler::project::restart_project,
        crate::api::handler::project::project_process_status,
        crate::api::handler::project::get_project_procfile,
        crate::api::handler::project::project_health_check,
        crate::api::handler::project::get_project_builder,
        // webhook 部署通道（M-B 主线二 Stage A-7 · Doc/20 §5.8）
        crate::api::handler::project::enable_project_webhook,
        crate::api::handler::project::disable_project_webhook,
        crate::api::handler::project::project_webhook_deploy,
        crate::api::handler::project::project_logs,
        // runtimes（M-B 主线二 Stage A-3）
        crate::api::handler::project::list_runtimes,
        crate::api::handler::project::list_runtimes_by_kind,
        crate::api::handler::project::available_runtimes,
        crate::api::handler::project::install_runtime,
        crate::api::handler::project::get_runtime,
        crate::api::handler::project::uninstall_runtime,
        crate::api::handler::project::set_default_runtime,
        crate::api::handler::project::list_runtime_sites,
        // 服务联链（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）
        crate::api::handler::project::list_project_links,
        crate::api::handler::project::list_linkable_databases,
        crate::api::handler::project::link_project_database,
        crate::api::handler::project::unlink_project_database,
        // tasks（统一 Task 进度查询/取消，Phase B1）
        crate::api::handler::task::list_tasks,
        crate::api::handler::task::get_task,
        crate::api::handler::task::cancel_task,
        crate::api::handler::task::prune_tasks,
        // metrics
        crate::api::handler::metrics::processes,
        crate::api::handler::metrics::history,
        crate::api::handler::metrics::prometheus_metrics,
        // overview（S-06 顶栏聚合徽标）
        crate::api::handler::overview::badges,
        // iac（M-E 首片，只读 YAML 配置清单）
        crate::api::handler::iac::export,
        // inspection（S-17 系统体检）
        crate::api::handler::inspection::report,
        crate::api::handler::inspection::run,
        // kernel-logs（W0 内核日志）
        crate::api::handler::kernel_logs::overview,
        crate::api::handler::kernel_logs::entries,
        crate::api::handler::kernel_logs::stats,
        crate::api::handler::kernel_logs::download,
        crate::api::handler::kernel_logs::delete_file,
        crate::api::handler::kernel_logs::set_level,
        crate::api::handler::mcp::protocol::post,
        crate::api::handler::mcp::protocol::get,
        crate::api::handler::mcp::protocol::delete,
        crate::api::handler::mcp::list_tools,
        crate::api::handler::mcp::list_prompts,
        crate::api::handler::mcp::render_prompt,
        crate::api::handler::mcp::list_principals,
        crate::api::handler::mcp::create_principal,
        crate::api::handler::mcp::revoke_principal,
        crate::api::handler::mcp::list_sessions,
        crate::api::handler::mcp::list_calls,
        crate::api::handler::mcp::approvals::list_approvals,
        crate::api::handler::mcp::approvals::get_approval,
        crate::api::handler::mcp::approvals::approve,
        crate::api::handler::mcp::approvals::reject,
        // M-AI-3 面板内置助手
        crate::api::handler::assistant::get_config,
        crate::api::handler::assistant::update_config,
        crate::api::handler::assistant::test_connection,
        crate::api::handler::assistant::chat,
        // toolbox（S-12 进程管理）
        crate::api::handler::toolbox::list_processes,
        crate::api::handler::toolbox::kill_process,
        crate::api::handler::toolbox::host_details,
        // files（M-C 文件管理：编辑 / 上传下载 / 压缩解压 / 回收站）
        crate::api::handler::firewall::list,
        crate::api::handler::firewall::get,
        crate::api::handler::firewall::create,
        crate::api::handler::firewall::update,
        crate::api::handler::firewall::delete,
        crate::api::handler::firewall::toggle,
        crate::api::handler::firewall::apply_all,
        crate::api::handler::firewall::get_status,
        crate::api::handler::firewall::enable,
        crate::api::handler::firewall::disable,
        crate::api::handler::firewall::reorder,
        crate::api::handler::web_server::list_engines,
        crate::api::handler::web_server::list,
        crate::api::handler::web_server::get,
        crate::api::handler::web_server::create,
        crate::api::handler::web_server::update,
        crate::api::handler::web_server::delete,
        crate::api::handler::web_server::start,
        crate::api::handler::web_server::stop,
        crate::api::handler::web_server::restart,
        crate::api::handler::web_server::reload,
        crate::api::handler::web_server::config_test,
        crate::api::handler::web_server::get_config,
        crate::api::handler::web_server::native_detect,
        crate::api::handler::web_server::native_install,
        crate::api::handler::web_server::native_uninstall,
        crate::api::handler::web_server::set_autostart,
        crate::api::handler::web_server::native_autostart,
        crate::api::handler::web_server::native_status,
        crate::api::handler::web_server::switch_engine,
        crate::api::handler::web_server::apply_preset,
        crate::api::handler::web_server::list_presets,
        crate::api::handler::web_server::engine_capabilities,
        crate::api::handler::web_server::install_capabilities,
        crate::api::handler::database::list,
        crate::api::handler::database::get,
        crate::api::handler::database::delete,
        crate::api::handler::database::install_mysql,
        crate::api::handler::database::install_redis,
        crate::api::handler::database::install_postgres,
        crate::api::handler::database::install_mongodb,
        crate::api::handler::database::start,
        crate::api::handler::database::stop,
        crate::api::handler::database::restart,
        crate::api::handler::database::check_status,
        crate::api::handler::database::create_database,
        crate::api::handler::database::drop_database,
        crate::api::handler::database::list_databases,
        crate::api::handler::database::create_user,
        crate::api::handler::database::drop_user,
        crate::api::handler::database::uninstall,
        crate::api::handler::database::update_batch_status,
        crate::api::handler::plugin::list_plugins,
        crate::api::handler::plugin::get_plugin,
        crate::api::handler::plugin::load_plugin,
        crate::api::handler::plugin::reload_plugin,
        crate::api::handler::plugin::execute_plugin,
        crate::api::handler::plugin::enable_plugin,
        crate::api::handler::plugin::disable_plugin,
        crate::api::handler::plugin::unload_plugin,
        crate::api::handler::plugin::get_plugin_metrics,
        crate::api::handler::plugin::reset_plugin_metrics,
        crate::api::handler::plugin::list_plugin_settings,
        crate::api::handler::plugin::set_plugin_setting,
        crate::api::handler::plugin::get_plugin_setting,
        crate::api::handler::plugin::update_plugin_capabilities,
        crate::api::handler::plugin::market::list_market,
        crate::api::handler::plugin::market::install_market_plugin,
        crate::api::handler::docker::list,
        crate::api::handler::docker::get,
        crate::api::handler::docker::start,
        crate::api::handler::docker::stop,
        crate::api::handler::docker::restart,
        crate::api::handler::docker::remove,
        crate::api::handler::docker::logs,
        crate::api::handler::docker::stats,
        crate::api::handler::docker::list_images,
        crate::api::handler::docker::remove_image,
        crate::api::handler::docker::compose_deploy,
        crate::api::handler::docker::compose_convert_run,
        crate::api::handler::docker::compose_up,
        crate::api::handler::docker::compose_down,
        crate::api::handler::docker::inspect,
        crate::api::handler::docker::container_compose,
        crate::api::handler::docker::rename,
        crate::api::handler::docker::pause,
        crate::api::handler::docker::unpause,
        crate::api::handler::docker::kill,
        crate::api::handler::docker::prune_containers,
        crate::api::handler::docker::list_networks,
        crate::api::handler::docker::create_network,
        crate::api::handler::docker::remove_network,
        crate::api::handler::docker::connect_network,
        crate::api::handler::docker::disconnect_network,
        crate::api::handler::docker::prune_networks,
        crate::api::handler::docker::list_volumes,
        crate::api::handler::docker::create_volume,
        crate::api::handler::docker::remove_volume,
        crate::api::handler::docker::prune_volumes,
        crate::api::handler::docker::pull_image,
        crate::api::handler::docker::tag_image,
        crate::api::handler::docker::prune_images,
        crate::api::handler::docker::compose_ls,
        crate::api::handler::file::list,
        crate::api::handler::file::read,
        crate::api::handler::file::write,
        crate::api::handler::file::create_file,
        crate::api::handler::file::create_dir,
        crate::api::handler::file::delete,
        crate::api::handler::file::rename,
        crate::api::handler::file::chmod,
        crate::api::handler::file::upload,
        crate::api::handler::file::download,
        crate::api::handler::file::compress,
        crate::api::handler::file::copy_entries,
        crate::api::handler::file::move_entries,
        crate::api::handler::file::extract,
        crate::api::handler::file::recycle_list,
        crate::api::handler::file::recycle_restore,
        crate::api::handler::file::recycle_purge,
        crate::api::handler::file::recycle_empty,
        // operation logs
        crate::api::handler::operation_log::export,
        // outbox（事件落库）
        crate::api::handler::outbox::list,
        // certificates（主线三 §6.1 · Stage 2）
        crate::api::handler::certificate::list_certificates,
        crate::api::handler::certificate::get_certificate,
        crate::api::handler::certificate::update_certificate,
        crate::api::handler::certificate::delete_certificate,
        crate::api::handler::certificate::issue_acme,
        crate::api::handler::certificate::issue_self_signed,
        crate::api::handler::certificate::upload_certificate,
        crate::api::handler::certificate::bind_website,
        crate::api::handler::certificate::renew_certificate,
        crate::api::handler::certificate::renewal_history,
    ),
    components(schemas(
            crate::domain::entity::MetricsSnapshot,
        crate::domain::entity::User,
        crate::domain::entity::ServerNode,
        crate::domain::entity::Website,
        crate::domain::entity::ScheduledTask,
        crate::domain::entity::AppMetadata,
        crate::domain::entity::InstalledApp,
        crate::domain::compose_merge::UpgradePreview,
        crate::domain::compose_merge::ComposeMerge,
        crate::domain::compose_merge::MergeConflict,
        crate::domain::entity::AppFormat,
        crate::domain::entity::InstallMode,
        crate::api::types::CreateUserRequest,
        crate::api::types::UpdateUserRequest,
        crate::api::types::CreateNodeRequest,
        crate::api::types::CreateWebsiteRequest,
        crate::api::types::PaginationParams,
        crate::api::types::PaginatedResponse<crate::domain::entity::User>,
        crate::api::types::PaginatedResponse<crate::domain::entity::ServerNode>,
        crate::api::types::PaginatedResponse<crate::domain::entity::Website>,
        crate::api::types::PaginatedResponse<crate::domain::entity::OutboxEvent>,
        crate::domain::entity::OutboxEvent,
        crate::api::handler::outbox::OutboxListQuery,
        crate::api::handler::auth::LoginRequest,
        crate::api::handler::auth::LoginResponse,
        crate::api::handler::auth::LoginTwoFactorRequest,
        crate::api::handler::auth::two_factor::TwoFactorStatusResponse,
        crate::api::handler::auth::two_factor::TwoFactorSetupRequest,
        crate::api::handler::auth::two_factor::TwoFactorEnableRequest,
        crate::api::handler::auth::two_factor::TwoFactorConfirmRequest,
        crate::api::handler::auth::two_factor::RecoveryCodesResponse,
        crate::application::two_factor_service::TwoFactorStatus,
        crate::application::two_factor_service::TwoFactorSetup,
        crate::api::handler::node::HeartbeatRequest,
        crate::api::handler::node::RemoteExecRequest,
        crate::api::handler::node::RemoteBatchExecRequest,
        crate::api::handler::node::RemoteActionRequest,
        crate::api::handler::node::RemoteListQuery,
        crate::api::handler::node::RemoteUploadRequest,
        crate::api::handler::health::HealthDetail,
        crate::api::handler::health::HealthChecks,
        crate::api::handler::health::HealthCheckItem,
        crate::api::handler::settings::SettingEntry,
        crate::api::handler::settings::UpdateSettingRequest,
        crate::api::handler::settings::AccessInfo,
        crate::api::handler::backup::BackupEntryDto,
        crate::api::handler::dns_provider::DnsProviderAccountDto,
        crate::api::handler::dns_provider::CreateDnsProviderRequest,
        crate::api::handler::dns_provider::UpdateDnsProviderRequest,
        crate::domain::entity::WafRule,
        crate::domain::entity::WafLog,
        crate::api::handler::waf::CreateWafRuleRequest,
        crate::api::handler::waf::UpdateWafRuleRequest,
        crate::api::handler::waf::ClearLogsRequest,
        crate::api::handler::waf::WafEngineStatus,
        crate::application::waf_ports::WafApplyReport,
        crate::application::waf_ports::WafEngineApply,
        crate::application::waf_ports::WafSkippedRule,
        crate::application::waf_ports::WafIngestStatus,
        crate::api::handler::backup_account::AccountDto,
        crate::api::handler::backup_account::CreateAccountRequest,
        crate::api::handler::backup_account::UpdateAccountRequest,
        crate::api::handler::backup_account::RemoteObjectDto,
        crate::api::handler::backup_account::UploadObjectRequest,
        crate::api::handler::backup_account::DeleteObjectQuery,
        crate::domain::entity::BackupAccount,
        crate::api::handler::backup_job::JobDto,
        crate::api::handler::backup_job::CreateJobRequest,
        crate::api::handler::backup_job::UpdateJobRequest,
        crate::api::handler::backup_job::ToggleJobRequest,
        crate::api::handler::backup_job::RestoreFromCloudRequest,
        crate::api::handler::backup_job::RestoreFromCloudResponse,
        crate::domain::entity::BackupJob,
        crate::api::handler::scheduled_task::CreateTaskRequest,
        crate::api::handler::scheduled_task::UpdateTaskRequest,
        crate::api::handler::app_store::AppStoreListResponse,
        crate::api::handler::app_store::InstallAppRequest,
        crate::api::handler::app_store::ImportPackageRequest,
        crate::api::handler::app_store::InstalledAppResponse,
        crate::api::handler::app_store::BatchImportPackagesRequest,
        crate::api::handler::app_store::BatchImportPackagesResponse,
        crate::domain::entity::AppHooks,
        crate::domain::entity::AppSource,
        crate::api::handler::app_store::SourcesResponse,
        crate::api::handler::app_store::CreateSourceRequest,
        crate::api::handler::app_store::UpdateSourceRequest,
        crate::api::handler::app_store::SyncSourceResponse,
        crate::application::app_store_service::StoreSecurityConfig,
        crate::application::website_service::SiteRewriteStatus,
        crate::webserver::SiteDeployment,
        crate::api::handler::app_store::UpdateStoreSecurityRequest,
        crate::api::handler::task::TaskListResponse,
        crate::runtime::task_state::TaskRecord,
        crate::runtime::task_state::TaskState,
        crate::runtime::task_state::TaskKind,
        crate::api::handler::metrics::ProcessEntry,
        crate::application::metrics_history_service::MetricsHistoryResponse,
        crate::domain::entity::MetricsHistoryPoint,
        crate::application::iac_service::IacManifest,
        crate::application::iac_service::IacWebsite,
        crate::application::iac_service::IacDatabase,
        crate::application::iac_service::IacApplication,
        crate::application::overview_service::OverviewBadges,
        crate::application::overview_service::NodeBadges,
        crate::application::overview_service::ContainerBadges,
        crate::application::overview_service::TaskBadges,
        crate::application::overview_service::AlertBadges,
        crate::domain::inspection::InspectionReport,
        crate::domain::inspection::InspectionSummary,
        crate::domain::inspection::KernelInfo,
        crate::domain::inspection::Finding,
        crate::domain::inspection::Severity,
        crate::domain::inspection::CheckStatus,
        crate::domain::inspection::Category,
        crate::domain::toolbox::ProcessInfo,
        crate::domain::toolbox::ProtectedReason,
        crate::domain::toolbox::ProcessSignal,
        crate::application::toolbox_service::ProcessRow,
        crate::application::toolbox_service::ProcessListResponse,
        crate::application::toolbox_service::KillProcessResponse,
        crate::api::handler::toolbox::KillProcessRequest,
        crate::domain::host_info::HostDetails,
        crate::domain::host_info::HostSystem,
        crate::domain::host_info::HostCpu,
        crate::domain::host_info::HostMemory,
        crate::domain::host_info::HostTime,
        crate::domain::host_info::DnsConfig,
        crate::domain::host_info::HostsEntry,
        crate::domain::host_info::HostInterface,
        crate::domain::host_info::HostDisk,
        crate::domain::host_info::PanelRuntime,
        crate::file::FileInfo,
        crate::file::WriteResult,
        crate::file::archive::CompressResult,
        crate::file::archive::ExtractResult,
        crate::file::recycle::RecycleItem,
        crate::api::handler::firewall::FirewallRuleResponse,
        crate::api::handler::firewall::CreateFirewallRuleRequest,
        crate::api::handler::firewall::UpdateFirewallRuleRequest,
        crate::api::handler::firewall::ToggleRequest,
        crate::api::handler::firewall::ReorderRequest,
        crate::api::handler::web_server::EngineInfo,
        crate::api::dto::WebServerResponse,
        crate::api::dto::CreateWebServerInstanceRequest,
        crate::webserver::native::NativeWebServerInfo,
        crate::webserver::capability::CapabilityInfo,
        crate::domain::entity::WebServerInstance,
        crate::api::handler::web_server::SwitchEngineRequest,
        crate::api::handler::web_server::ApplyPresetRequest,
        crate::api::handler::web_server::NativeInstallRequest,
        crate::api::handler::web_server::NativeUninstallRequest,
        crate::api::handler::web_server::AutostartRequest,
        crate::api::handler::web_server::NativeAutostartRequest,
        crate::api::handler::web_server::InstallCapabilitiesRequest,
        crate::api::handler::database::DatabaseInstanceResponse,
        crate::api::handler::database::InstallMysqlRequest,
        crate::api::handler::database::InstallRedisRequest,
        crate::api::handler::database::InstallPostgresRequest,
        crate::api::handler::database::InstallMongodbRequest,
        crate::api::handler::database::CreateDatabaseRequest,
        crate::api::handler::database::CreateUserRequest,
        crate::api::handler::database::BatchStatusRequest,
        crate::api::handler::plugin::LoadPluginRequest,
        crate::api::handler::plugin::ExecutePluginRequest,
        crate::api::handler::plugin::PluginResponse,
        crate::api::handler::plugin::ExecutionResponse,
        crate::api::handler::plugin::UpdateCapabilitiesRequest,
        crate::api::handler::plugin::market::MarketPluginView,
        crate::api::handler::plugin::market::MarketListResponse,
        crate::api::handler::plugin::market::MarketInstallRequest,
        crate::plugin::market::MarketPlugin,
        crate::plugin::market::MarketSource,
        crate::api::dto::PluginSettingRequest,
        crate::api::dto::PluginMetricsResponse,
        crate::api::dto::PluginReloadRequest,
        crate::api::handler::docker::ComposeDeployRequest,
        crate::api::handler::docker::ComposeDeployResponse,
        crate::api::handler::docker::ConvertRunRequest,
        crate::domain::compose_convert::ComposeConversion,
        crate::api::handler::docker::RenameRequest,
        crate::api::handler::docker::CreateNetworkRequest,
        crate::api::handler::docker::ConnectNetworkRequest,
        crate::api::handler::docker::CreateVolumeRequest,
        crate::api::handler::docker::PullImageRequest,
        crate::api::handler::docker::TagImageRequest,
        crate::domain::entity::DockerContainer,
        crate::api::handler::file::WriteRequest,
        crate::api::handler::file::CreateRequest,
        crate::api::handler::file::RenameRequest,
        crate::api::handler::file::ChmodRequest,
        crate::api::handler::file::CompressRequest,
        crate::api::handler::file::ExtractRequest,
        crate::api::handler::file::PurgeResult,
        crate::api::handler::file::TransferRequest,
        crate::file::transfer::TransferMode,
        crate::file::transfer::TransferItem,
        crate::file::transfer::TransferResult,
        crate::domain::kernel_log::LogLevel,
        crate::domain::kernel_log::KernelLogEntry,
        crate::domain::kernel_log::LevelCounts,
        crate::infrastructure::logging::reader::LogFileInfo,
        crate::infrastructure::logging::reader::LogPage,
        crate::application::kernel_log_service::KernelLogOverview,
        crate::application::kernel_log_service::KernelLogStats,
        crate::api::handler::kernel_logs::SetKernelLogLevelRequest,
        crate::api::handler::kernel_logs::KernelLogLevelResponse,
        crate::domain::mcp::AgentPrincipal,
        crate::domain::mcp::AgentSession,
        crate::domain::mcp::ToolCallRecord,
        crate::domain::mcp::ToolApproval,
        crate::domain::mcp::RiskLevel,
        crate::domain::mcp::ApprovalStatus,
        crate::api::handler::mcp::approvals::RejectRequest,
        crate::api::handler::mcp::PrincipalView,
        crate::api::handler::mcp::CreatePrincipalRequest,
        crate::api::handler::mcp::CreatePrincipalResponse,
        crate::api::handler::mcp::ToolInfo,
        crate::api::handler::mcp::PromptArgInfo,
        crate::api::handler::mcp::PromptInfo,
        crate::api::handler::mcp::PromptRendered,
        crate::domain::assistant::AssistantConfigView,
        crate::domain::assistant::AssistantConfigPatch,
        crate::domain::assistant::LlmProviderKind,
        crate::api::handler::assistant::ChatTurn,
        crate::api::handler::assistant::ChatRequest,
        crate::api::handler::assistant::ChatResponse,
        crate::api::handler::assistant::TestResult,
        crate::api::handler::assistant::agent::ToolStep,
        crate::api::handler::project::ProjectDto,
        crate::api::handler::project::LinkableDatabaseDto,
        crate::api::handler::project::ProjectDbLinkDto,
        crate::api::handler::project::LinkDatabaseRequest,
        crate::api::handler::project::HealthCheckDto,
        crate::api::handler::project::CreateProjectRequest,
        crate::api::handler::project::WebsiteDto,
        crate::api::handler::website::BindRuntimeRequest,
        crate::api::handler::website::ApplyRewriteRequest,
        crate::domain::entity::Certificate,
        crate::api::handler::certificate::CertificateListResponse,
        crate::api::handler::certificate::IssueAcmeRequest,
        crate::api::handler::certificate::IssueSelfSignedRequest,
        crate::api::handler::certificate::UploadCertificateRequest,
        crate::api::handler::certificate::UpdateCertificateRequest,
        crate::api::handler::certificate::BindCertificateRequest,
        crate::api::handler::certificate::CertificateRenewalHistory,
        crate::domain::cert_renewal::RenewalEntry,
        crate::domain::cert_renewal::RenewalEventKind,
        crate::api::handler::project::UpdateProjectRequest,
        crate::api::handler::project::RuntimeDto,
        crate::api::handler::project::InstallRuntimeRequest,
        crate::api::handler::project::WebhookDeployDto,
        crate::api::handler::project::ProcfileDto,
        crate::api::handler::project::ProcEntry,
        crate::application::project_ports::ProcessStatus,
        crate::application::project_ports::LogLine,
        crate::application::runtime_service::RuntimeVersionInfo,
    ))
)]
pub struct ApiDoc;

/// JWT Bearer 认证（OpenAPI security scheme）。
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "BearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

/// 生成 OpenAPI 文档 JSON（进程级缓存）。
pub fn openapi_json() -> serde_json::Value {
    static CACHE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut doc = ApiDoc::openapi();
            SecurityAddon.modify(&mut doc);
            let mut v = serde_json::to_value(&doc).unwrap_or_default();
            annotate_permissions(&mut v);
            v
        })
        .clone()
}

/// 为每个操作补充 `x-permission: "<resource>:<action>"`（Doc/32 §6）。
///
/// 取自运行时鉴权使用的同一张 [`ROUTE_PERMISSIONS`](crate::api::permissions::ROUTE_PERMISSIONS)
/// 表，文档与实际授权不会漂移；未登记的端点不加（只校验登录或公开）。
fn annotate_permissions(doc: &mut serde_json::Value) {
    let Some(paths) = doc.get_mut("paths").and_then(|p| p.as_object_mut()) else {
        return;
    };
    for (path, item) in paths.iter_mut() {
        let concrete = sample_path(path);
        let Some(ops) = item.as_object_mut() else {
            continue;
        };
        for (method, op) in ops.iter_mut() {
            let Ok(m) = axum::http::Method::from_bytes(method.to_ascii_uppercase().as_bytes())
            else {
                continue;
            };
            if let Some((resource, action)) =
                crate::api::permissions::route_permission(&m, &concrete)
            {
                if let Some(o) = op.as_object_mut() {
                    o.insert(
                        "x-permission".into(),
                        serde_json::Value::String(format!("{resource}:{action}")),
                    );
                }
            }
        }
    }
}

/// `/api/users/{id}/roles` → `/api/users/1/roles`（权限表按具体路径匹配）。
fn sample_path(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if seg.starts_with('{') && seg.ends_with('}') {
                "1"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// OpenAPI 路由：`GET /api/openapi.json`（始终可用）。
pub fn openapi_router() -> axum::Router<crate::api::types::AppState> {
    use axum::routing::get;
    axum::Router::new().route(
        "/api/openapi.json",
        get(|| async { axum::Json(openapi_json()) }),
    )
}

/// 无 Swagger UI 时返回空 Router（保持组合根签名一致）。
/// 说明：为避免额外依赖与 CDN 路径，交互式 UI 由前端调用 /api/openapi.json 自行渲染。
pub fn swagger_ui_router() -> axum::Router<crate::api::types::AppState> {
    axum::Router::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在 `module` 对应源文件里找 `ty` 的声明；找不到时顺着 `pub use crate::…` 再导出递归。
    /// 返回（源码, 声明所在行号）。
    fn locate(
        root: &std::path::Path,
        module: &[&str],
        ty: &str,
        depth: u8,
    ) -> Option<(String, usize)> {
        if depth > 4 {
            return None;
        }
        let base = module.iter().collect::<std::path::PathBuf>();
        let file = [
            root.join(&base).with_extension("rs"),
            root.join(&base).join("mod.rs"),
        ]
        .into_iter()
        .find(|f| f.is_file())?;
        let code = std::fs::read_to_string(&file).ok()?;
        let is_decl = |l: &str| {
            let l = l.trim_start();
            ["pub struct ", "pub enum "].iter().any(|kw| {
                l.strip_prefix(kw).is_some_and(|rest| {
                    rest.strip_prefix(ty)
                        .is_some_and(|after| after.starts_with([' ', '(', '<', ';', '{']))
                })
            })
        };
        if let Some(i) = code.lines().position(is_decl) {
            return Some((code, i));
        }
        // 相对 glob 再导出：`pub use manager::*;`（子模块）
        for l in code.lines().map(str::trim) {
            if let Some(sub) = l
                .strip_prefix("pub use ")
                .and_then(|r| r.strip_suffix("::*;"))
                .filter(|r| !r.starts_with("crate::") && !r.contains('{'))
            {
                let mut segs = module.to_vec();
                segs.extend(sub.split("::"));
                if let Some(found) = locate(root, &segs, ty, depth + 1) {
                    return Some(found);
                }
            }
        }
        // `pub use crate::a::b::{X, Y};` / `pub use crate::a::b::X;` / `pub use crate::a::*;`（可跨行）
        let mut rest = code.as_str();
        while let Some(i) = rest.find("pub use crate::") {
            let stmt_all = &rest[i + "pub use crate::".len()..];
            let stmt = &stmt_all[..stmt_all.find(';')?];
            rest = &stmt_all[stmt.len()..];
            if let Some(glob) = stmt.trim().strip_suffix("::*") {
                let segs: Vec<&str> = glob.split("::").map(str::trim).collect();
                if let Some(found) = locate(root, &segs, ty, depth + 1) {
                    return Some(found);
                }
                continue;
            }
            let names_word = stmt
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == ty);
            if !names_word {
                continue;
            }
            let module_path = match stmt.find("::{") {
                Some(j) => &stmt[..j],
                None => stmt.rsplit_once("::").map(|(m, _)| m)?,
            };
            let segs: Vec<&str> = module_path.split("::").map(str::trim).collect();
            if let Some(found) = locate(root, &segs, ty, depth + 1) {
                return Some(found);
            }
        }
        None
    }

    /// utoipa 按类型名（或 `#[schema(as = …)]`）注册组件，同名类型**静默覆盖**：
    /// 数据库 `CreateUserRequest` 曾顶掉面板用户的、docker `RenameRequest` 曾顶掉文件管理的，
    /// 文档字段全错却无任何报错。这里扫描 `components(schemas(...))` 清单与各类型源码，
    /// 按「别名优先、否则类型名」计算有效组件名并断言唯一。
    #[test]
    fn component_schema_names_are_unique() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let src = include_str!("openapi.rs");
        let start = src.find("components(schemas(").expect("components block");
        let block = &src[start..];
        let block = &block[..block.find("))").expect("end of schemas")];
        let mut seen: std::collections::HashMap<String, String> = Default::default();
        for path in block
            .lines()
            .map(|l| l.trim().trim_end_matches(','))
            // 泛型实例（`PaginatedResponse<User>`）由 utoipa 生成组合名，不参与
            .filter(|l| l.starts_with("crate::") && !l.contains('<'))
        {
            let segs: Vec<&str> = path.trim_start_matches("crate::").split("::").collect();
            let (ty, module) = segs.split_last().expect("path");
            let (code, decl) =
                locate(&root, module, ty, 0).unwrap_or_else(|| panic!("declaration of {path}"));
            let lines: Vec<&str> = code.lines().collect();
            let alias = lines[decl.saturating_sub(6)..decl]
                .iter()
                .rev()
                .take_while(|l| {
                    let l = l.trim_start();
                    l.starts_with("#[") || l.starts_with("///")
                })
                .find_map(|l| {
                    let i = l.find("schema(as = ")?;
                    let rest = &l[i + "schema(as = ".len()..];
                    Some(rest[..rest.find(')')?].trim().to_string())
                });
            let name = alias.unwrap_or_else(|| ty.to_string());
            if let Some(prev) = seen.insert(name.clone(), path.to_string()) {
                panic!(
                    "OpenAPI 组件名 `{name}` 冲突：{prev} 与 {path}（给其一加 #[schema(as = …)]）"
                );
            }
        }
        assert!(seen.len() > 50, "解析到的组件过少：{}", seen.len());
    }

    fn op<'a>(doc: &'a serde_json::Value, path: &str, method: &str) -> &'a serde_json::Value {
        &doc["paths"][path][method]
    }

    #[test]
    fn operations_carry_permission_from_route_table() {
        let doc = openapi_json();
        assert_eq!(op(&doc, "/api/users", "get")["x-permission"], "user:read");
        assert_eq!(
            op(&doc, "/api/users/{id}", "delete")["x-permission"],
            "user:delete"
        );
        // 公开端点不带权限
        assert!(op(&doc, "/api/auth/login", "post")
            .get("x-permission")
            .is_none());
        assert_eq!(sample_path("/api/a/{id}/b/{name}"), "/api/a/1/b/1");
    }

    /// 快照漂移门禁（原 CI 独立 stage `cargo run --bin export-openapi | diff`）：
    /// 并入 `cargo test`，省掉 CI 上一次完整的 dev 构建。字节级比对与 export-openapi 输出同源。
    /// 仅默认特性组合下校验——快照即以默认特性导出。
    #[test]
    #[cfg(all(
        feature = "sqlite",
        feature = "docker",
        feature = "wasm",
        feature = "email"
    ))]
    fn frontend_snapshot_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/openapi.json");
        let committed = std::fs::read_to_string(path).expect("read frontend/openapi.json");
        let current = format!(
            "{}\n",
            serde_json::to_string_pretty(&openapi_json()).expect("serialize openapi")
        );
        assert!(
            committed == current,
            "frontend/openapi.json 过期：请运行 just openapi（或 cargo run -p flame-kernel --bin export-openapi > frontend/openapi.json）并提交"
        );
    }

    #[test]
    fn every_documented_operation_has_a_tag() {
        let doc = openapi_json();
        for (path, item) in doc["paths"].as_object().unwrap() {
            for (method, op) in item.as_object().unwrap() {
                assert!(
                    op["tags"].as_array().is_some_and(|t| !t.is_empty()),
                    "{method} {path} has no tag"
                );
            }
        }
    }
}
