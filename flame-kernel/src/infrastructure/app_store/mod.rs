pub mod adapter;
pub mod security_scanner;
pub mod variable_mapper;

pub use adapter::{select_adapter, AppPackageAdapter, DefaultAdapterProvider};
pub use security_scanner::{
    ensure_restart_policy, scan_compose, DefaultComposeSecurityScanner, ScanFinding, ScanResult,
    Severity,
};
pub use variable_mapper::{DefaultVariableMapperFactory, VariableMapper};

use crate::application::app_store_ports::DefaultAppStorePorts;
use std::sync::Arc;

/// 创建应用商店默认端口实现（组合根/兼容入口组装）。
/// 返回 application 层 `DefaultAppStorePorts` 聚合（T12 整顿后），避免 application 依赖 infrastructure。
pub fn default_ports(
    runner: crate::application::execution_mode::SharedCommandRunner,
) -> DefaultAppStorePorts {
    DefaultAppStorePorts {
        adapter_provider: Arc::new(DefaultAdapterProvider),
        security_scanner: Arc::new(DefaultComposeSecurityScanner::new()),
        variable_mapper_factory: Arc::new(DefaultVariableMapperFactory::new()),
        package_manager: Arc::new(crate::infrastructure::os::DefaultPackageManagerPort::new(
            runner.clone(),
        )),
        service_manager: Arc::new(crate::infrastructure::os::DefaultServiceManagerPort::new(
            runner.clone(),
        )),
        runner,
    }
}
