//! 应用商店服务：应用包管理 + 三种安装模式（容器 / 原生 / WASM）编排。
//!
//! 按职责拆分（Doc/30 技术债「app_store_service 拆分」）：
//! - `security`  商店安全配置（仓库白名单 / 验签策略）
//! - `catalog`   内置种子、包列表、本地导入
//! - `sources`   远程应用源与同步
//! - `hooks`     生命周期钩子
//! - `install`   安装编排（容器 / 原生 / WASM）
//! - `lifecycle` 已安装应用：卸载 / 升级 / 日志 / 原生启停
//! - `wasm`      内置 WASM 工具与插件恢复
//! - `util`      校验与 RAII 守卫等通用辅助
//!
//! 对外路径保持 `crate::application::app_store_service::*` 不变。

use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Local, Utc};
use uuid::Uuid;

use crate::application::app_store_ports::{
    AppAdapterProvider, ComposeSecurityScanner, PackageManagerPort, ServiceManagerPort,
    VariableMapperFactory,
};
use crate::application::service::{DatabaseService, DockerService, WebServerService};
use crate::core::error::AppError;
use crate::database::{MySqlManager, RedisManager};
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::plugin::{PluginConfig, PluginRegistry, PluginSandbox};

/// 内置 WASM 工具（hello）：导出 `run() -> i32`，返回 42
pub(super) const WASM_HELLO_B64: &str = "AGFzbQEAAAABBQFgAAF/AwIBAAcHAQNydW4AAAoGAQQAQSoL";

/// 应用商店服务：应用包管理 + 三种安装模式（容器 / 原生 / WASM）编排
pub struct AppStoreService {
    pub package_repo: Arc<dyn AppPackageRepository>,
    pub installed_repo: Arc<dyn InstalledAppRepository>,
    pub docker_service: Arc<DockerService>,
    pub web_server_service: Arc<WebServerService>,
    pub database_service: Arc<DatabaseService>,
    pub plugin_sandbox: Arc<PluginSandbox>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub plugin_repo: Arc<dyn PluginRepository>,
    pub apps_dir: PathBuf,
    pub package_manager: Arc<dyn PackageManagerPort>,
    pub service_manager: Arc<dyn ServiceManagerPort>,
    pub mysql_manager: MySqlManager,
    pub redis_manager: RedisManager,
    pub event_bus: crate::event::EventBus,
    /// 适配器选择端口（应用包格式识别）
    pub adapter_provider: Arc<dyn AppAdapterProvider>,
    /// Compose 安全扫描端口
    pub security_scanner: Arc<dyn ComposeSecurityScanner>,
    /// 变量映射工厂端口
    pub variable_mapper_factory: Arc<dyn VariableMapperFactory>,
    /// 统一 Task 状态机跟踪器（Phase B1：安装/引擎切换/批量节点共用）
    pub task_tracker: crate::runtime::task_state::TaskTracker,
    /// 特权命令执行端口（Phase A1 收尾：通用原生脚本等剩余直接命令路径统一收敛）
    pub runner: crate::application::execution_mode::SharedCommandRunner,
    /// F-26: 安装中互斥锁——防止同一应用包被并发重复安装。
    install_locks: std::sync::Mutex<std::collections::HashMap<String, ()>>,
    /// 原生安装器注册表（Doc/16 §6.1：NativeInstaller trait 抽象，key → installer 路由）
    pub native_installers: Arc<crate::application::native_installer::NativeInstallerRegistry>,
    /// 远程应用源仓储（M-A Issue 1.2，Doc/20 §4.1）
    pub source_repo: Arc<dyn AppSourceRepository>,
    /// 同步互斥锁：同一源同时只允许一个同步任务
    sync_locks: std::sync::Mutex<std::collections::HashMap<i64, ()>>,
    /// 设置服务（M-A Issue 1.3：验签策略 / 镜像源白名单等商店级配置读取）。
    /// 可选注入：None 时使用内置默认值（兼容测试与既有构造）。
    pub settings_service: Option<Arc<crate::application::settings_service::SettingsService>>,
}

mod catalog;
mod hooks;
mod install;
mod lifecycle;
mod security;
mod sources;
mod util;
mod wasm;

pub use crate::domain::compose_merge::{ConflictStrategy, UpgradePreview};
pub use hooks::*;
pub use install::*;
pub use security::*;
pub use util::*;

impl AppStoreService {
    /// 完整构造：注入全部端口（组合根负责创建具体适配器并注入）。
    #[allow(clippy::too_many_arguments)]
    pub fn with_ports(
        package_repo: Arc<dyn AppPackageRepository>,
        installed_repo: Arc<dyn InstalledAppRepository>,
        docker_service: Arc<DockerService>,
        web_server_service: Arc<WebServerService>,
        database_service: Arc<DatabaseService>,
        plugin_sandbox: Arc<PluginSandbox>,
        plugin_registry: Arc<PluginRegistry>,
        plugin_repo: Arc<dyn PluginRepository>,
        apps_dir: PathBuf,
        event_bus: crate::event::EventBus,
        adapter_provider: Arc<dyn AppAdapterProvider>,
        security_scanner: Arc<dyn ComposeSecurityScanner>,
        variable_mapper_factory: Arc<dyn VariableMapperFactory>,
        package_manager: Arc<dyn PackageManagerPort>,
        service_manager: Arc<dyn ServiceManagerPort>,
        runner: crate::application::execution_mode::SharedCommandRunner,
        source_repo: Arc<dyn AppSourceRepository>,
    ) -> Self {
        Self::with_ports_and_task_store(
            package_repo,
            installed_repo,
            docker_service,
            web_server_service,
            database_service,
            plugin_sandbox,
            plugin_registry,
            plugin_repo,
            apps_dir,
            event_bus,
            adapter_provider,
            security_scanner,
            variable_mapper_factory,
            package_manager,
            service_manager,
            runner,
            None,
            source_repo,
        )
    }

    /// 完整构造 + 注入统一 Task 状态机持久化存储（Phase B1 扩展：进程重启可恢复）。
    #[allow(clippy::too_many_arguments)]
    pub fn with_ports_and_task_store(
        package_repo: Arc<dyn AppPackageRepository>,
        installed_repo: Arc<dyn InstalledAppRepository>,
        docker_service: Arc<DockerService>,
        web_server_service: Arc<WebServerService>,
        database_service: Arc<DatabaseService>,
        plugin_sandbox: Arc<PluginSandbox>,
        plugin_registry: Arc<PluginRegistry>,
        plugin_repo: Arc<dyn PluginRepository>,
        apps_dir: PathBuf,
        event_bus: crate::event::EventBus,
        adapter_provider: Arc<dyn AppAdapterProvider>,
        security_scanner: Arc<dyn ComposeSecurityScanner>,
        variable_mapper_factory: Arc<dyn VariableMapperFactory>,
        package_manager: Arc<dyn PackageManagerPort>,
        service_manager: Arc<dyn ServiceManagerPort>,
        runner: crate::application::execution_mode::SharedCommandRunner,
        task_store: Option<crate::runtime::task_state::TaskStoreRef>,
        source_repo: Arc<dyn AppSourceRepository>,
    ) -> Self {
        let task_tracker = match task_store {
            Some(store) => crate::runtime::task_state::TaskTracker::with_store(store),
            None => crate::runtime::task_state::TaskTracker::new(),
        };
        Self::with_ports_and_shared_tracker(
            package_repo,
            installed_repo,
            docker_service,
            web_server_service,
            database_service,
            plugin_sandbox,
            plugin_registry,
            plugin_repo,
            apps_dir,
            event_bus,
            adapter_provider,
            security_scanner,
            variable_mapper_factory,
            package_manager,
            service_manager,
            runner,
            task_tracker,
            source_repo,
        )
    }

    /// 注入共享的统一 Task 状态机跟踪器（Phase B1 扩展：多服务共享同一 tracker，供前端统一查询/取消）。
    ///
    /// `TaskTracker` 内部为 `Arc`，多个服务 Clone 同一实例即共享同一任务集合。
    #[allow(clippy::too_many_arguments)]
    pub fn with_ports_and_shared_tracker(
        package_repo: Arc<dyn AppPackageRepository>,
        installed_repo: Arc<dyn InstalledAppRepository>,
        docker_service: Arc<DockerService>,
        web_server_service: Arc<WebServerService>,
        database_service: Arc<DatabaseService>,
        plugin_sandbox: Arc<PluginSandbox>,
        plugin_registry: Arc<PluginRegistry>,
        plugin_repo: Arc<dyn PluginRepository>,
        apps_dir: PathBuf,
        event_bus: crate::event::EventBus,
        adapter_provider: Arc<dyn AppAdapterProvider>,
        security_scanner: Arc<dyn ComposeSecurityScanner>,
        variable_mapper_factory: Arc<dyn VariableMapperFactory>,
        package_manager: Arc<dyn PackageManagerPort>,
        service_manager: Arc<dyn ServiceManagerPort>,
        runner: crate::application::execution_mode::SharedCommandRunner,
        task_tracker: crate::runtime::task_state::TaskTracker,
        source_repo: Arc<dyn AppSourceRepository>,
    ) -> Self {
        let native_installers = Arc::new(
            crate::application::native_builtin_installers::default_native_installers(
                web_server_service.clone(),
                database_service.clone(),
            ),
        );
        Self {
            package_repo,
            installed_repo,
            docker_service,
            web_server_service,
            database_service,
            plugin_sandbox,
            plugin_registry,
            plugin_repo,
            apps_dir,
            package_manager,
            service_manager,
            mysql_manager: MySqlManager::new(runner.clone()),
            redis_manager: RedisManager::new(runner.clone()),
            event_bus,
            adapter_provider,
            security_scanner,
            variable_mapper_factory,
            task_tracker,
            runner,
            install_locks: std::sync::Mutex::new(std::collections::HashMap::new()),
            native_installers,
            source_repo,
            sync_locks: std::sync::Mutex::new(std::collections::HashMap::new()),
            settings_service: None,
        }
    }

    /// 注入设置服务（组合根调用；验签策略与镜像白名单等商店级配置经此读取）。
    pub fn with_settings_service(
        mut self,
        settings_service: Arc<crate::application::settings_service::SettingsService>,
    ) -> Self {
        self.settings_service = Some(settings_service);
        self
    }

    /// 便捷构造：注入默认端口实现（测试、兼容入口使用）。
    ///
    /// 生产组合根建议使用 `with_ports` 显式注入全部端口（六边形落地）；
    /// 默认端口实现由 `crate::infrastructure::app_store::default_ports` 提供。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        package_repo: Arc<dyn AppPackageRepository>,
        installed_repo: Arc<dyn InstalledAppRepository>,
        docker_service: Arc<DockerService>,
        web_server_service: Arc<WebServerService>,
        database_service: Arc<DatabaseService>,
        plugin_sandbox: Arc<PluginSandbox>,
        plugin_registry: Arc<PluginRegistry>,
        plugin_repo: Arc<dyn PluginRepository>,
        apps_dir: PathBuf,
        event_bus: crate::event::EventBus,
        ports: crate::application::app_store_ports::DefaultAppStorePorts,
        source_repo: Arc<dyn AppSourceRepository>,
    ) -> Self {
        Self::with_ports(
            package_repo,
            installed_repo,
            docker_service,
            web_server_service,
            database_service,
            plugin_sandbox,
            plugin_registry,
            plugin_repo,
            apps_dir,
            event_bus,
            ports.adapter_provider,
            ports.security_scanner,
            ports.variable_mapper_factory,
            ports.package_manager,
            ports.service_manager,
            ports.runner,
            source_repo,
        )
    }

    /// 内置应用目录：`data/apps`
    pub fn default_apps_dir() -> PathBuf {
        PathBuf::from("data/apps")
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ports_tests;

#[cfg(test)]
mod persistence_tests;
