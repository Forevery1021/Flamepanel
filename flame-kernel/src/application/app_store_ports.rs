//! 应用商店端口（Ports）：供 `AppStoreService` 依赖的抽象接口。
//!
//! 六边形架构：application 只依赖这些端口（trait），具体实现放在
//! `infrastructure/app_store/`，由组合根（`FlameKernel::build_services`）创建并注入。
//! 禁止 application 直接 `use crate::infrastructure::app_store::具体类型`。

use crate::core::error::AppError;
use crate::domain::entity::{AppMetadata, AppVersionInfo};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// ─── 安全扫描结果模型（纯数据，不依赖具体实现） ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanFinding {
    pub severity: Severity,
    pub message: String,
    pub item: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Block,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub findings: Vec<ScanFinding>,
}

impl ScanResult {
    pub fn has_blockers(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Block)
    }

    pub fn block_messages(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Block)
            .map(|f| f.message.clone())
            .collect()
    }

    pub fn summary(&self) -> Vec<String> {
        self.findings.iter().map(|f| f.message.clone()).collect()
    }
}

// ─── 应用包适配器端口 ────────────────────────────────────────────────────────

/// 应用包适配器：统一 1Panel / 宝塔 / 内置 Flame 三种格式。
/// 实现位于 `infrastructure/app_store/adapter/`。
pub trait AppPackageAdapter: Send + Sync {
    fn detect(&self, root: &Path) -> bool;

    fn parse_metadata(&self, root: &Path) -> Result<AppMetadata, AppError>;

    fn list_versions(&self, root: &Path) -> Result<Vec<String>, AppError>;

    fn parse_version(&self, root: &Path, version: &str) -> Result<AppVersionInfo, AppError>;

    /// 本格式支持的标准端口变量列表，供前端预填
    fn known_port_vars(&self) -> &'static [&'static str];
}

/// 适配器选择端口：根据目录结构自动选择对应格式的适配器。
pub trait AppAdapterProvider: Send + Sync {
    fn select(&self, root: &Path) -> Result<Arc<dyn AppPackageAdapter>, AppError>;
}

// ─── Compose 安全扫描端口 ───────────────────────────────────────────────────

pub trait ComposeSecurityScanner: Send + Sync {
    /// Compose 安全检查：返回按严重度分类的结果
    fn scan_compose(&self, compose_yaml: &str, confirmed_risky: bool) -> ScanResult;

    /// 自动为没有 restart 策略的 compose 补充 `restart: unless-stopped`
    fn ensure_restart_policy(&self, compose_yaml: &str) -> String;
}

// ─── 变量映射端口 ───────────────────────────────────────────────────────────

/// 变量映射引擎：支持 `${VAR}`、`$VAR` 与遗留 `{var}` 三种占位符。
pub trait VariableMapper: Send + Sync {
    fn insert(&mut self, key: &str, value: String);

    fn get(&self, key: &str) -> Option<&str>;

    /// 返回 (渲染结果, 未识别变量警告)
    fn replace(&self, template: &str) -> (String, Vec<String>);
}

/// 变量映射工厂：以用户表单值构造映射器实例（`AppStoreService` 需多次创建）。
pub trait VariableMapperFactory: Send + Sync {
    fn create(&self, values: HashMap<String, String>) -> Box<dyn VariableMapper>;
}

// ─── 包管理器 / 服务管理器端口 ──────────────────────────────────────────────

/// 系统包管理器端口（apt/yum/apk 安装、卸载、查询）。
#[async_trait]
pub trait PackageManagerPort: Send + Sync {
    async fn install(&self, pkg: &str) -> Result<String, AppError>;
    async fn is_installed(&self, pkg: &str) -> Result<bool, AppError>;
    async fn uninstall(&self, pkg: &str) -> Result<(), AppError>;
    async fn get_version(&self, pkg: &str) -> Result<String, AppError>;
}

/// 系统服务管理器端口（systemctl 启停等）。
#[async_trait]
pub trait ServiceManagerPort: Send + Sync {
    async fn start(&self, name: &str) -> Result<(), AppError>;
    async fn stop(&self, name: &str) -> Result<(), AppError>;
    async fn restart(&self, name: &str) -> Result<(), AppError>;
    async fn enable(&self, name: &str) -> Result<(), AppError>;
    async fn disable(&self, name: &str) -> Result<(), AppError>;
    async fn is_running(&self, name: &str) -> Result<bool, AppError>;
}

// ─── 默认端口聚合（T12：AppStore 构造整顿） ───────────────────────────────
// `AppStoreService` 便捷构造经本聚合获取默认端口实现，避免 application 层直接
// `use crate::infrastructure::app_store::default_ports`。
// 组合根/测试先用 `infrastructure::app_store::default_ports(runner)` 组装本聚合，
// 再传入 `AppStoreService::new`；本类型仅含 application 层端口，不依赖具体实现。

/// 应用商店默认端口聚合（application 层类型）。
pub struct DefaultAppStorePorts {
    pub adapter_provider: Arc<dyn AppAdapterProvider>,
    pub security_scanner: Arc<dyn ComposeSecurityScanner>,
    pub variable_mapper_factory: Arc<dyn VariableMapperFactory>,
    pub package_manager: Arc<dyn PackageManagerPort>,
    pub service_manager: Arc<dyn ServiceManagerPort>,
    /// 特权命令执行端口
    pub runner: crate::application::execution_mode::SharedCommandRunner,
}
