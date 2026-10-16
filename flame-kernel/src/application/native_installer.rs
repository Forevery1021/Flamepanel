//! 原生安装器抽象（NativeInstaller trait，Doc/16 §6.1）。
//!
//! 将应用商店 `install_native` 中按 `package_key` 的 `match` 分支抽象为 trait 注册表：
//! 第三方原生软件（MySQL/Redis/Nginx/Web 引擎等）实现 `NativeInstaller` 并注册到
//! `NativeInstallerRegistry` 后，即可获得统一的原生生命周期能力：
//! install / uninstall / status / start / stop / restart。
//!
//! 面板内置两种安装器：
//! - [`DatabaseNativeInstaller`]：MySQL / MariaDB / Redis（委托 `DatabaseService`）
//! - [`WebServerNativeInstaller`]：Nginx / Apache / OpenLiteSpeed / OpenResty / Caddy
//!   （委托 `WebServerService` + `PackageManager` + `ServiceManager`）

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::application::app_store_service::InstallRequest;
use crate::core::error::AppError;
use crate::domain::entity::InstalledApp;

/// 原生软件安装器端口：一种（或一族）原生软件的生命周期编排。
///
/// `keys()` 声明本安装器接管的 `package_key` 集合（如 `["mysql", "mariadb"]`）；
/// 注册表按 key 路由到首个匹配的安装器。
#[async_trait::async_trait]
pub trait NativeInstaller: Send + Sync {
    /// 本安装器接管的 package_key 集合。
    fn keys(&self) -> &'static [&'static str];

    /// 安装原生软件并返回已安装应用记录。
    async fn install(&self, req: &InstallRequest, version: &str) -> Result<InstalledApp, AppError>;

    /// 卸载原生软件（不负责删除 `InstalledApp` 记录，由调用方统一清理）。
    async fn uninstall(&self, app: &InstalledApp) -> Result<(), AppError>;

    /// 运行状态：`running` / `stopped`。
    async fn status(&self, app: &InstalledApp) -> Result<String, AppError>;

    /// 启动服务。
    async fn start(&self, app: &InstalledApp) -> Result<(), AppError>;

    /// 停止服务。
    async fn stop(&self, app: &InstalledApp) -> Result<(), AppError>;

    /// 重启服务。
    async fn restart(&self, app: &InstalledApp) -> Result<(), AppError>;
}

/// 安装器注册表：按 `package_key` 路由到注册的 `NativeInstaller`。
///
/// 使用 `BTreeMap` 保证遍历顺序稳定（key → installer）。
pub struct NativeInstallerRegistry {
    installers: BTreeMap<&'static str, Arc<dyn NativeInstaller>>,
}

impl NativeInstallerRegistry {
    pub fn new() -> Self {
        Self {
            installers: BTreeMap::new(),
        }
    }

    /// 注册一个安装器（同 key 重复注册时后注册者覆盖）。
    pub fn register(&mut self, installer: Arc<dyn NativeInstaller>) {
        for key in installer.keys() {
            self.installers.insert(key, installer.clone());
        }
    }

    /// 按 package_key 查找安装器。
    pub fn find(&self, package_key: &str) -> Option<Arc<dyn NativeInstaller>> {
        self.installers.get(package_key).cloned()
    }

    /// 是否有安装器接管该 key。
    pub fn handles(&self, package_key: &str) -> bool {
        self.installers.contains_key(package_key)
    }

    /// 已注册的全部 key（测试 / 诊断用）。
    pub fn registered_keys(&self) -> Vec<&'static str> {
        self.installers.keys().copied().collect()
    }
}

impl Default for NativeInstallerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn dummy_request(key: &str) -> InstallRequest {
        InstallRequest {
            package_key: key.into(),
            version: None,
            mode: Some("native".into()),
            name: None,
            port: None,
            container_name: None,
            values: HashMap::new(),
            confirm_risky: false,
            acknowledge_scripts: false,
        }
    }

    fn dummy_app(key: &str) -> InstalledApp {
        InstalledApp {
            id: 1,
            package_key: key.into(),
            name: key.into(),
            version: "1.0.0".into(),
            mode: "native".into(),
            status: "running".into(),
            access_url: None,
            install_path: "/tmp".into(),
            container_name: None,
            port: None,
            params_json: "{}".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            launch_count: 0,
        }
    }

    /// 记录型 mock 安装器：记录被调用的动作。
    struct MockInstaller {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockInstaller {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                calls: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait::async_trait]
    impl NativeInstaller for MockInstaller {
        fn keys(&self) -> &'static [&'static str] {
            &["mysql", "mariadb"]
        }
        async fn install(
            &self,
            _req: &InstallRequest,
            version: &str,
        ) -> Result<InstalledApp, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("install:{version}"));
            let mut app = dummy_app("mysql");
            app.version = version.into();
            Ok(app)
        }
        async fn uninstall(&self, _app: &InstalledApp) -> Result<(), AppError> {
            self.calls.lock().unwrap().push("uninstall".into());
            Ok(())
        }
        async fn status(&self, _app: &InstalledApp) -> Result<String, AppError> {
            self.calls.lock().unwrap().push("status".into());
            Ok("running".into())
        }
        async fn start(&self, _app: &InstalledApp) -> Result<(), AppError> {
            self.calls.lock().unwrap().push("start".into());
            Ok(())
        }
        async fn stop(&self, _app: &InstalledApp) -> Result<(), AppError> {
            self.calls.lock().unwrap().push("stop".into());
            Ok(())
        }
        async fn restart(&self, _app: &InstalledApp) -> Result<(), AppError> {
            self.calls.lock().unwrap().push("restart".into());
            Ok(())
        }
    }

    #[tokio::test]
    async fn registry_routes_by_key() {
        let mut registry = NativeInstallerRegistry::new();
        let installer = MockInstaller::new();
        registry.register(installer.clone());

        assert!(registry.handles("mysql"));
        assert!(registry.handles("mariadb"));
        assert!(!registry.handles("redis"));
        assert_eq!(registry.registered_keys(), vec!["mariadb", "mysql"]);

        let found = registry.find("mysql").unwrap();
        let app = found
            .install(&dummy_request("mysql"), "8.0.36")
            .await
            .unwrap();
        assert_eq!(app.version, "8.0.36");
        assert_eq!(found.status(&app).await.unwrap(), "running");
        found.start(&app).await.unwrap();
        found.stop(&app).await.unwrap();
        found.restart(&app).await.unwrap();
        found.uninstall(&app).await.unwrap();

        let calls = installer.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![
                "install:8.0.36",
                "status",
                "start",
                "stop",
                "restart",
                "uninstall"
            ]
        );
    }

    #[tokio::test]
    async fn registry_find_missing_key() {
        let registry = NativeInstallerRegistry::new();
        assert!(registry.find("unknown").is_none());
        assert!(!registry.handles("unknown"));
    }
}
