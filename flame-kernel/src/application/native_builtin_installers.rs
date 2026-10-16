//! 面板内置原生安装器：数据库（MySQL/MariaDB/Redis）与 Web 引擎（Nginx/Apache/
//! OpenLiteSpeed/OpenResty/Caddy）。
//!
//! 将 `AppStoreService::install_native` 中原 `match key` 的分派逻辑迁移为
//! `NativeInstaller` 实现（Doc/16 §6.1），并补齐原生软件生命周期
//! status/start/stop/restart（Doc/16 §6.5 / Doc/13 §5.4）。

use std::sync::Arc;

use crate::application::app_store_service::InstallRequest;
use crate::application::database_service::DatabaseService;
use crate::application::web_server_service::WebServerService;
use crate::application::{NativeInstaller, NativeInstallerRegistry};
use crate::core::error::AppError;
use crate::domain::entity::InstalledApp;
use crate::webserver::WebServerEngine;

/// 数据库原生安装器（mysql / mariadb / redis）。
pub struct DatabaseNativeInstaller {
    pub database_service: Arc<DatabaseService>,
}

impl DatabaseNativeInstaller {
    /// 本安装器可路由的 package_key 集合。
    pub fn new(database_service: Arc<DatabaseService>) -> Self {
        Self { database_service }
    }
}

#[async_trait::async_trait]
impl NativeInstaller for DatabaseNativeInstaller {
    fn keys(&self) -> &'static [&'static str] {
        &["mysql", "mariadb", "redis", "postgres"]
    }

    async fn install(&self, req: &InstallRequest, version: &str) -> Result<InstalledApp, AppError> {
        let key = req.package_key.as_str();
        match key {
            "mysql" | "mariadb" => {
                let db_type = if key == "mariadb" { "mariadb" } else { "mysql" };
                let _ = self
                    .database_service
                    .install_mysql(
                        Some(version),
                        req.port.unwrap_or(3306),
                        req.values
                            .get("root_password")
                            .map(|s| s.as_str())
                            .unwrap_or("flamepanel_root"),
                        &format!(
                            "{}_{}",
                            db_type,
                            crate::application::app_store_service::short_uuid()
                        ),
                    )
                    .await?;
                Ok(InstalledApp {
                    id: 0,
                    package_key: key.into(),
                    name: req
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("{} 数据库", key)),
                    version: version.into(),
                    mode: crate::domain::entity::InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: None,
                    install_path: format!("/var/lib/{}", db_type),
                    container_name: None,
                    port: Some(req.port.unwrap_or(3306)),
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    launch_count: 0,
                })
            }
            "redis" => {
                let _ = self
                    .database_service
                    .install_redis(
                        Some(version),
                        req.port.unwrap_or(6379),
                        req.values.get("password").map(|s| s.as_str()),
                        &format!(
                            "redis_{}",
                            crate::application::app_store_service::short_uuid()
                        ),
                    )
                    .await?;
                Ok(InstalledApp {
                    id: 0,
                    package_key: "redis".into(),
                    name: req.name.clone().unwrap_or_else(|| "Redis".into()),
                    version: version.into(),
                    mode: crate::domain::entity::InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: None,
                    install_path: "/var/lib/redis".into(),
                    container_name: None,
                    port: Some(req.port.unwrap_or(6379)),
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    launch_count: 0,
                })
            }
            "postgres" => {
                let _ = self
                    .database_service
                    .install_postgres(
                        Some(version),
                        req.port.unwrap_or(5432),
                        req.values.get("password").map(|s| s.as_str()),
                        &format!(
                            "postgres_{}",
                            crate::application::app_store_service::short_uuid()
                        ),
                    )
                    .await?;
                Ok(InstalledApp {
                    id: 0,
                    package_key: "postgres".into(),
                    name: req.name.clone().unwrap_or_else(|| "PostgreSQL".into()),
                    version: version.into(),
                    mode: crate::domain::entity::InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: None,
                    install_path: "/var/lib/postgresql".into(),
                    container_name: None,
                    port: Some(req.port.unwrap_or(5432)),
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    launch_count: 0,
                })
            }
            _ => Err(AppError::BadRequest(format!(
                "DatabaseNativeInstaller 不支持的应用: {}",
                key
            ))),
        }
    }

    async fn uninstall(&self, app: &InstalledApp) -> Result<(), AppError> {
        // 找到该 package_key 的数据库实例并卸载（含 systemctl stop/disable + 包移除）
        let instances = self.database_service.list_instances().await?;
        for inst in instances {
            let matches = match app.package_key.as_str() {
                "mysql" | "mariadb" => inst.db_type == "mysql" || inst.db_type == "mariadb",
                "redis" => inst.db_type == "redis",
                "postgres" => inst.db_type == "postgres",
                _ => false,
            };
            if matches {
                self.database_service.uninstall(inst.id).await?;
            }
        }
        Ok(())
    }

    async fn status(&self, _app: &InstalledApp) -> Result<String, AppError> {
        let instances = self.database_service.list_instances().await?;
        for inst in instances {
            let matches = match _app.package_key.as_str() {
                "mysql" | "mariadb" => inst.db_type == "mysql" || inst.db_type == "mariadb",
                "redis" => inst.db_type == "redis",
                "postgres" => inst.db_type == "postgres",
                _ => false,
            };
            if matches {
                return self.database_service.status(inst.id).await;
            }
        }
        Ok("stopped".into())
    }

    async fn start(&self, app: &InstalledApp) -> Result<(), AppError> {
        let instances = self.database_service.list_instances().await?;
        for inst in instances {
            let matches = match app.package_key.as_str() {
                "mysql" | "mariadb" => inst.db_type == "mysql" || inst.db_type == "mariadb",
                "redis" => inst.db_type == "redis",
                "postgres" => inst.db_type == "postgres",
                _ => false,
            };
            if matches {
                return self.database_service.start(inst.id).await;
            }
        }
        Err(AppError::NotFound(format!(
            "未找到 {} 的数据库实例",
            app.package_key
        )))
    }

    async fn stop(&self, app: &InstalledApp) -> Result<(), AppError> {
        let instances = self.database_service.list_instances().await?;
        for inst in instances {
            let matches = match app.package_key.as_str() {
                "mysql" | "mariadb" => inst.db_type == "mysql" || inst.db_type == "mariadb",
                "redis" => inst.db_type == "redis",
                "postgres" => inst.db_type == "postgres",
                _ => false,
            };
            if matches {
                return self.database_service.stop(inst.id).await;
            }
        }
        Err(AppError::NotFound(format!(
            "未找到 {} 的数据库实例",
            app.package_key
        )))
    }

    async fn restart(&self, app: &InstalledApp) -> Result<(), AppError> {
        let instances = self.database_service.list_instances().await?;
        for inst in instances {
            let matches = match app.package_key.as_str() {
                "mysql" | "mariadb" => inst.db_type == "mysql" || inst.db_type == "mariadb",
                "redis" => inst.db_type == "redis",
                "postgres" => inst.db_type == "postgres",
                _ => false,
            };
            if matches {
                return self.database_service.restart(inst.id).await;
            }
        }
        Err(AppError::NotFound(format!(
            "未找到 {} 的数据库实例",
            app.package_key
        )))
    }
}

/// Web 引擎原生安装器（nginx / apache / openlitespeed / openresty / caddy）。
pub struct WebServerNativeInstaller {
    pub web_server_service: Arc<WebServerService>,
}

impl WebServerNativeInstaller {
    pub fn new(web_server_service: Arc<WebServerService>) -> Self {
        Self { web_server_service }
    }

    fn engine_for(&self, package_key: &str) -> Result<WebServerEngine, AppError> {
        WebServerEngine::from_name(package_key)
            .ok_or_else(|| AppError::BadRequest(format!("未知 Web 引擎: {}", package_key)))
    }
}

#[async_trait::async_trait]
impl NativeInstaller for WebServerNativeInstaller {
    fn keys(&self) -> &'static [&'static str] {
        &["nginx", "apache", "openlitespeed", "openresty", "caddy"]
    }

    async fn install(&self, req: &InstallRequest, version: &str) -> Result<InstalledApp, AppError> {
        let engine = self.engine_for(&req.package_key)?;
        // 委托 WebServerService 原生安装：包管理器安装 + systemd enable/start + 注册实例
        let _instance = self
            .web_server_service
            .native_install(&engine, Some(version))
            .await?;
        Ok(InstalledApp {
            id: 0,
            package_key: req.package_key.clone(),
            name: req
                .name
                .clone()
                .unwrap_or_else(|| engine.as_str().to_string()),
            version: version.into(),
            mode: crate::domain::entity::InstallMode::Native.as_str().into(),
            status: "running".into(),
            access_url: Some(format!(
                "http://localhost:{}",
                req.port.unwrap_or(engine.default_port() as i32)
            )),
            install_path: engine.default_config_path().into(),
            container_name: None,
            port: Some(req.port.unwrap_or(engine.default_port() as i32)),
            params_json: serde_json::to_string(&req.values).unwrap_or_default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            launch_count: 0,
        })
    }

    async fn uninstall(&self, app: &InstalledApp) -> Result<(), AppError> {
        let engine = self.engine_for(&app.package_key)?;
        // 同时清理该引擎的注册实例
        self.web_server_service
            .native_uninstall_by_engine(&engine)
            .await
    }

    async fn status(&self, app: &InstalledApp) -> Result<String, AppError> {
        let engine = self.engine_for(&app.package_key)?;
        let info = self.web_server_service.native_manager.detect(&engine).await;
        Ok(if info.running { "running" } else { "stopped" }.into())
    }

    async fn start(&self, app: &InstalledApp) -> Result<(), AppError> {
        let engine = self.engine_for(&app.package_key)?;
        let manager = self.web_server_service.native_service_manager();
        manager.start(engine.service_name()).await
    }

    async fn stop(&self, app: &InstalledApp) -> Result<(), AppError> {
        let engine = self.engine_for(&app.package_key)?;
        let manager = self.web_server_service.native_service_manager();
        manager.stop(engine.service_name()).await
    }

    async fn restart(&self, app: &InstalledApp) -> Result<(), AppError> {
        let engine = self.engine_for(&app.package_key)?;
        let manager = self.web_server_service.native_service_manager();
        manager.restart(engine.service_name()).await
    }
}

/// 组装面板默认的原生安装器注册表（组合根调用）。
pub fn default_native_installers(
    web_server_service: Arc<WebServerService>,
    database_service: Arc<DatabaseService>,
) -> NativeInstallerRegistry {
    let mut registry = NativeInstallerRegistry::new();
    registry.register(Arc::new(WebServerNativeInstaller::new(web_server_service)));
    registry.register(Arc::new(DatabaseNativeInstaller::new(database_service)));
    registry
}
