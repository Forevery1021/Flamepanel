//! 运行时安装端口（M-B 主线二 Stage A-2 · Doc/20 §5.1/5.3）。
//!
//! 六边形架构：`RuntimeService`（application）只依赖本文件声明的端口 trait，
//! 具体实现（`infrastructure/runtime/`）由组合根创建并注入，禁止 application
//! 直接依赖 infrastructure 执行实现。
//!
//! 与 `NativeInstaller` 注册表模式一致：每种语言一个 [`RuntimeInstaller`]，
//! 组合根注册到 [`RuntimeInstallerRegistry`]，服务按 `Runtime.kind` 路由。

use crate::core::error::AppError;
use crate::domain::entity::Runtime;
use async_trait::async_trait;

/// 运行时安装器端口：一种语言运行时的安装/卸载/版本探测编排。
#[async_trait]
pub trait RuntimeInstaller: Send + Sync {
    /// 本安装器接管的语言种类（如 `["php"]`，与 `Runtime.kind` 对齐）。
    fn kinds(&self) -> &'static [&'static str];

    /// 安装运行时到 `runtime.install_path`（实现自建目录），完成后更新状态字段。
    async fn install(&self, runtime: &Runtime) -> Result<(), AppError>;

    /// 卸载运行时（尽力而为：目录存在则清理，失败不阻断状态修正）。
    async fn uninstall(&self, runtime: &Runtime) -> Result<(), AppError>;

    /// 探测指定系统包是否已安装（发行版感知路由，供 `available()` 过滤）。
    async fn is_package_installed(&self, pkg: &str) -> Result<bool, AppError>;

    /// 安装完成后的自述（S-11）：如 Node 解析后的真实版本 / 是否带 pm2，
    /// 由服务层合并进 `Runtime.metadata_json` 供前端展示。缺省无。
    async fn describe(&self, runtime: &Runtime) -> Option<serde_json::Value> {
        let _ = runtime;
        None
    }
}

/// 运行时安装器注册表：按 `Runtime.kind` 路由到注册的安装器。
///
/// 使用 `BTreeMap` 保证遍历顺序稳定（kind → installer）。
pub struct RuntimeInstallerRegistry {
    installers: std::collections::BTreeMap<&'static str, std::sync::Arc<dyn RuntimeInstaller>>,
}

impl RuntimeInstallerRegistry {
    pub fn new() -> Self {
        Self {
            installers: std::collections::BTreeMap::new(),
        }
    }

    /// 注册一个安装器（同 kind 重复注册时后注册者覆盖）。
    pub fn register(&mut self, installer: std::sync::Arc<dyn RuntimeInstaller>) {
        for kind in installer.kinds() {
            self.installers.insert(kind, installer.clone());
        }
    }

    /// 按 kind 查找安装器。
    pub fn find(&self, kind: &str) -> Option<std::sync::Arc<dyn RuntimeInstaller>> {
        self.installers.get(kind).cloned()
    }

    /// 是否有安装器接管该 kind。
    pub fn handles(&self, kind: &str) -> bool {
        self.installers.contains_key(kind)
    }

    /// 已注册的全部 kind（测试 / 诊断用）。
    pub fn registered_kinds(&self) -> Vec<&'static str> {
        self.installers.keys().copied().collect()
    }
}

impl Default for RuntimeInstallerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn sample(kind: &str, version: &str) -> Runtime {
        Runtime {
            id: 0,
            kind: kind.into(),
            version: version.into(),
            install_path: format!("/opt/flamepanel/runtimes/{kind}/{version}"),
            status: "not_installed".into(),
            is_default: false,
            metadata_json: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    struct MockInstaller {
        calls: Mutex<Vec<String>>,
    }

    impl MockInstaller {
        fn new() -> std::sync::Arc<Self> {
            std::sync::Arc::new(Self {
                calls: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl RuntimeInstaller for MockInstaller {
        fn kinds(&self) -> &'static [&'static str] {
            &["php", "java"]
        }
        async fn install(&self, runtime: &Runtime) -> Result<(), AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("install:{}", runtime.version));
            Ok(())
        }
        async fn uninstall(&self, _runtime: &Runtime) -> Result<(), AppError> {
            self.calls.lock().unwrap().push("uninstall".into());
            Ok(())
        }
        async fn is_package_installed(&self, pkg: &str) -> Result<bool, AppError> {
            Ok(pkg == "php8.2")
        }
    }

    #[tokio::test]
    async fn registry_routes_by_kind() {
        let mut registry = RuntimeInstallerRegistry::new();
        let installer = MockInstaller::new();
        registry.register(installer.clone());

        assert!(registry.handles("php"));
        assert!(registry.handles("java"));
        assert!(!registry.handles("node"));
        assert_eq!(registry.registered_kinds(), vec!["java", "php"]);

        let found = registry.find("php").unwrap();
        let r = sample("php", "8.2");
        found.install(&r).await.unwrap();
        found.uninstall(&r).await.unwrap();
        assert!(found.is_package_installed("php8.2").await.unwrap());
        assert!(!found.is_package_installed("php8.3").await.unwrap());

        assert_eq!(
            *installer.calls.lock().unwrap(),
            vec!["install:8.2".to_string(), "uninstall".to_string()]
        );
    }

    #[tokio::test]
    async fn registry_find_missing_kind() {
        let registry = RuntimeInstallerRegistry::new();
        assert!(registry.find("go").is_none());
        assert!(!registry.handles("go"));
    }
}
