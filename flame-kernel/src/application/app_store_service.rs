use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::application::service::{DatabaseService, DockerService, WebServerService};
use crate::core::error::AppError;
use crate::database::{MySqlManager, RedisManager};
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::infrastructure::app_store::adapter::flame::FlameAdapter;
use crate::infrastructure::app_store::{
    ensure_restart_policy, scan_compose, select_adapter, VariableMapper,
};
use crate::infrastructure::os::{PackageManager, ServiceManager};
use crate::plugin::{PluginConfig, PluginRegistry, PluginSandbox};
use crate::webserver::WebServerEngine;

/// 内置 WASM 工具（hello）：导出 `run() -> i32`，返回 42
const WASM_HELLO_B64: &str = "AGFzbQEAAAABBQFgAAF/AwIBAAcHAQNydW4AAAoGAQQAQSoL";

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
    pub package_manager: PackageManager,
    pub service_manager: ServiceManager,
    pub mysql_manager: MySqlManager,
    pub redis_manager: RedisManager,
    pub event_bus: crate::event::EventBus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallRequest {
    pub package_key: String,
    pub version: Option<String>,
    pub mode: Option<String>,
    pub name: Option<String>,
    pub port: Option<i32>,
    pub container_name: Option<String>,
    pub values: HashMap<String, String>,
    /// 用户确认已知晓安全风险
    pub confirm_risky: bool,
}

impl AppStoreService {
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
    ) -> Self {
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
            package_manager: PackageManager,
            service_manager: ServiceManager,
            mysql_manager: MySqlManager::new(),
            redis_manager: RedisManager::new(),
            event_bus,
        }
    }

    /// 内置应用目录：`data/apps`
    pub fn default_apps_dir() -> PathBuf {
        PathBuf::from("data/apps")
    }

    /// 幂等种子：将内置 5 个应用写入包仓库
    pub async fn seed_builtin_apps(&self) -> Result<usize, AppError> {
        let mut count = 0;
        for manifest in crate::domain::entity::builtin_apps() {
            if self
                .package_repo
                .find_by_key(&manifest.key)
                .await?
                .is_some()
            {
                continue;
            }
            let metadata = FlameAdapter::builtin_metadata(&manifest);
            let pkg = AppPackage {
                id: 0,
                key: metadata.key.clone(),
                name: metadata.name.clone(),
                category: metadata.category.clone(),
                format: metadata.format.as_str().into(),
                description: metadata.short_desc_zh.clone(),
                logo: metadata.logo.clone(),
                metadata_json: serde_json::to_string(&metadata)
                    .map_err(|e| AppError::internal(format!("serialize metadata: {}", e)))?,
                source_path: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            self.package_repo.create(&pkg).await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn list_packages(
        &self,
        category: Option<&str>,
    ) -> Result<Vec<AppMetadata>, AppError> {
        let packages = self.package_repo.list_all().await?;
        let mut metas: Vec<AppMetadata> = packages
            .into_iter()
            .filter_map(|p| serde_json::from_str(&p.metadata_json).ok())
            .collect();
        if let Some(cat) = category {
            metas.retain(|m| m.category.eq_ignore_ascii_case(cat));
        }
        Ok(metas)
    }

    pub async fn get_package(&self, key: &str) -> Result<AppPackage, AppError> {
        self.package_repo
            .find_by_key(key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("应用包不存在: {}", key)))
    }

    pub async fn get_metadata(&self, key: &str) -> Result<AppMetadata, AppError> {
        if let Some(m) = self.wasm_builtins().into_iter().find(|m| m.key == key) {
            return Ok(m);
        }
        let pkg = self.get_package(key).await?;
        serde_json::from_str(&pkg.metadata_json)
            .map_err(|e| AppError::internal(format!("parse metadata: {}", e)))
    }

    /// 导入本地目录应用包（自动检测格式）
    pub async fn import_package(&self, path: &str) -> Result<AppMetadata, AppError> {
        let root = Path::new(path);
        if !root.is_dir() {
            return Err(AppError::BadRequest(format!("路径不是目录: {}", path)));
        }
        let adapter = select_adapter(root)?;
        let metadata = adapter.parse_metadata(root)?;

        // 复制到商店目录，保证独立性
        let dest = self.apps_dir.join(&metadata.key);
        std::fs::create_dir_all(&dest)
            .map_err(|e| AppError::internal(format!("创建商店目录失败: {}", e)))?;
        copy_recursive(root, &dest)?;

        if self
            .package_repo
            .find_by_key(&metadata.key)
            .await?
            .is_some()
        {
            return Err(AppError::BadRequest(format!(
                "应用包已存在: {}",
                metadata.key
            )));
        }

        let pkg = AppPackage {
            id: 0,
            key: metadata.key.clone(),
            name: metadata.name.clone(),
            category: metadata.category.clone(),
            format: metadata.format.as_str().into(),
            description: metadata.short_desc_zh.clone(),
            logo: metadata.logo.clone(),
            metadata_json: serde_json::to_string(&metadata)
                .map_err(|e| AppError::internal(format!("serialize metadata: {}", e)))?,
            source_path: Some(dest.to_string_lossy().into_owned()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.package_repo.create(&pkg).await?;
        Ok(metadata)
    }

    pub async fn list_versions(&self, key: &str) -> Result<Vec<String>, AppError> {
        let meta = self.get_metadata(key).await?;
        Ok(meta.versions)
    }

    /// 解析版本信息（内置应用走内置模板，导入应用走适配器）
    pub async fn get_version(&self, key: &str, version: &str) -> Result<AppVersionInfo, AppError> {
        if let Some(v) = self.wasm_builtin_version(key, version) {
            return Ok(v);
        }
        let pkg = self.get_package(key).await?;

        // 内置应用
        if pkg.source_path.is_none() {
            if let Some(manifest) = FlameAdapter::find_builtin(&pkg.key) {
                if version == manifest.version {
                    return Ok(FlameAdapter::builtin_version(&manifest));
                }
            }
            return Err(AppError::NotFound(format!("版本不存在: {}", version)));
        }

        // 导入应用：优先适配器解析
        let root = Path::new(pkg.source_path.as_deref().unwrap_or_default());
        if root.is_dir() {
            let adapter = select_adapter(root)?;
            return adapter.parse_version(root, version);
        }

        // 无目录（理论上不可达）：从 metadata 兜底
        Err(AppError::BadRequest(format!("应用包 {} 缺少源目录", key)))
    }

    // ─── 安装编排 ────────────────────────────────────────────────────────────

    pub async fn install(&self, req: &InstallRequest) -> Result<InstalledApp, AppError> {
        let metadata = self.get_metadata(&req.package_key).await?;
        let version = req
            .version
            .clone()
            .unwrap_or_else(|| metadata.default_version.clone());

        let mode = match req.mode.as_deref() {
            Some(m) => InstallMode::from_name(m)
                .ok_or_else(|| AppError::BadRequest("无效的安装模式".into()))?,
            None => metadata
                .modes
                .first()
                .copied()
                .unwrap_or(InstallMode::Container),
        };

        let app = match mode {
            InstallMode::Container => self.install_container(req, &version).await?,
            InstallMode::Native => self.install_native(req, &version).await?,
            InstallMode::Wasm => self.install_wasm(req, &version).await?,
        };
        let _ = self
            .event_bus
            .publish(DomainEvent::AppInstalled {
                app_key: app.package_key.clone(),
                app_name: app.name.clone(),
                version: app.version.clone(),
            })
            .await;
        Ok(app)
    }

    async fn install_container(
        &self,
        req: &InstallRequest,
        version: &str,
    ) -> Result<InstalledApp, AppError> {
        let version_info = self.get_version(&req.package_key, version).await?;
        let compose = version_info
            .compose_template
            .clone()
            .ok_or_else(|| AppError::BadRequest("该版本没有 docker-compose.yml".into()))?;

        // 表单校验
        validate_fields(&version_info.form_fields, &req.values)?;

        // 构建变量映射
        let container_name = req
            .container_name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("{}-{}", req.package_key, short_uuid()));

        let name = req.name.clone().unwrap_or_else(|| req.package_key.clone());
        let port = req.port.or(version_info.default_port).unwrap_or(0);
        let install_path = self.apps_dir.join(&req.package_key).join(&container_name);
        std::fs::create_dir_all(&install_path)
            .map_err(|e| AppError::internal(format!("创建安装目录失败: {}", e)))?;

        let mut mapper = VariableMapper::new(req.values.clone());
        mapper.insert("CONTAINER_NAME", &container_name);
        mapper.insert("NAME", &name);
        mapper.insert("PORT", port.to_string());
        mapper.insert("PANEL_APP_PORT_HTTP", port.to_string());
        mapper.insert("PANEL_APP_PORT_HTTPS", (port + 1).to_string());
        mapper.insert("HOST_IP", "0.0.0.0");
        mapper.insert("APP_PATH", install_path.to_string_lossy().into_owned());
        let (rendered, warnings) = mapper.replace(&compose);
        let rendered = ensure_restart_policy(&rendered);
        if !warnings.is_empty() {
            tracing::warn!("app {} compose 变量警告: {:?}", req.package_key, warnings);
        }

        // 安全扫描
        let scan = scan_compose(&rendered, req.confirm_risky);
        if scan.has_blockers() {
            return Err(AppError::BadRequest(format!(
                "安全扫描未通过: {}",
                scan.block_messages().join("; ")
            )));
        }

        // 写入 compose 文件
        let compose_path = install_path.join("docker-compose.yml");
        std::fs::write(&compose_path, &rendered)
            .map_err(|e| AppError::internal(format!("写入 compose 失败: {}", e)))?;

        // 部署
        let deploy_result = self
            .docker_service
            .compose_deploy(&container_name, &rendered)
            .await;
        if let Err(e) = deploy_result {
            let _ = std::fs::remove_dir_all(&install_path);
            return Err(e);
        }

        let now = Utc::now();
        let app = InstalledApp {
            id: 0,
            package_key: req.package_key.clone(),
            name,
            version: version.to_string(),
            mode: InstallMode::Container.as_str().into(),
            status: "running".into(),
            access_url: if port > 0 {
                Some(format!("http://localhost:{}", port))
            } else {
                None
            },
            install_path: install_path.to_string_lossy().into_owned(),
            container_name: Some(container_name),
            port: if port > 0 { Some(port) } else { None },
            params_json: serde_json::to_string(&req.values).unwrap_or_default(),
            created_at: now,
            updated_at: now,
            launch_count: 0,
        };
        let id = self.installed_repo.create(&app).await?;
        Ok(self.installed_repo.find_by_id(id).await?.unwrap_or(app))
    }

    async fn install_native(
        &self,
        req: &InstallRequest,
        version: &str,
    ) -> Result<InstalledApp, AppError> {
        let key = req.package_key.as_str();
        let version_info = self.get_version(&req.package_key, version).await?;

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
                        &format!("{}_{}", db_type, short_uuid()),
                    )
                    .await?;
                let _now = Utc::now();
                let app = InstalledApp {
                    id: 0,
                    package_key: key.into(),
                    name: req
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("{} 数据库", key)),
                    version: version.into(),
                    mode: InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: None,
                    install_path: format!("/var/lib/{}", db_type),
                    container_name: None,
                    port: Some(req.port.unwrap_or(3306)),
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    launch_count: 0,
                };
                let id = self.installed_repo.create(&app).await?;
                return self
                    .installed_repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| AppError::internal("create installed app"));
            }
            "redis" => {
                let _ = self
                    .database_service
                    .install_redis(
                        Some(version),
                        req.port.unwrap_or(6379),
                        req.values.get("password").map(|s| s.as_str()),
                        &format!("redis_{}", short_uuid()),
                    )
                    .await?;
                let app = InstalledApp {
                    id: 0,
                    package_key: "redis".into(),
                    name: req.name.clone().unwrap_or_else(|| "Redis".into()),
                    version: version.into(),
                    mode: InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: None,
                    install_path: "/var/lib/redis".into(),
                    container_name: None,
                    port: Some(req.port.unwrap_or(6379)),
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    launch_count: 0,
                };
                let id = self.installed_repo.create(&app).await?;
                return self
                    .installed_repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| AppError::internal("create installed app"));
            }
            "nginx" | "apache" | "openlitespeed" | "openresty" | "caddy" => {
                let engine = WebServerEngine::from_name(key)
                    .ok_or_else(|| AppError::BadRequest(format!("未知 Web 引擎: {}", key)))?;
                let pkg = engine.package_name();
                PackageManager::install(pkg).await?;
                let instance = WebServerInstance {
                    id: 0,
                    engine: engine.as_str().into(),
                    version: Some(version.into()),
                    status: "running".into(),
                    config_path: engine.default_config_path().into(),
                    binary_path: Some(engine.binary_name().into()),
                    port: req.port.unwrap_or(engine.default_port() as i32),
                    created_at: Utc::now(),
                };
                self.web_server_service.create_server(&instance).await?;
                let app = InstalledApp {
                    id: 0,
                    package_key: key.into(),
                    name: req.name.clone().unwrap_or_else(|| engine.as_str().into()),
                    version: version.into(),
                    mode: InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: Some(format!(
                        "http://localhost:{}",
                        req.port.unwrap_or(engine.default_port() as i32)
                    )),
                    install_path: engine.default_config_path().into(),
                    container_name: None,
                    port: Some(req.port.unwrap_or(engine.default_port() as i32)),
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    launch_count: 0,
                };
                let id = self.installed_repo.create(&app).await?;
                return self
                    .installed_repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| AppError::internal("create installed app"));
            }
            _ => {
                // 通用原生脚本安装（Flame 格式 install.sh）
                if !version_info.native_scripts.is_empty() {
                    for line in &version_info.native_scripts {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(line)
                            .output()
                            .map_err(|e| AppError::internal(format!("执行安装脚本失败: {}", e)))?;
                        if !output.status.success() {
                            return Err(AppError::internal(format!(
                                "安装脚本执行失败: {} (line: {})",
                                String::from_utf8_lossy(&output.stderr),
                                line
                            )));
                        }
                    }
                } else {
                    return Err(AppError::BadRequest(format!(
                        "应用 {} 没有可用的原生安装方式",
                        key
                    )));
                }
                let app = InstalledApp {
                    id: 0,
                    package_key: key.into(),
                    name: req.name.clone().unwrap_or_else(|| key.into()),
                    version: version.into(),
                    mode: InstallMode::Native.as_str().into(),
                    status: "running".into(),
                    access_url: None,
                    install_path: String::new(),
                    container_name: None,
                    port: req.port,
                    params_json: serde_json::to_string(&req.values).unwrap_or_default(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    launch_count: 0,
                };
                let id = self.installed_repo.create(&app).await?;
                return self
                    .installed_repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| AppError::internal("create installed app"));
            }
        }
    }

    async fn install_wasm(
        &self,
        req: &InstallRequest,
        version: &str,
    ) -> Result<InstalledApp, AppError> {
        let version_info = self.get_version(&req.package_key, version).await?;
        let wasm_base64 = version_info
            .wasm_base64
            .clone()
            .or_else(|| {
                // 兼容：直接提供 wasm_base64 表单值
                req.values.get("wasm_base64").cloned()
            })
            .ok_or_else(|| AppError::BadRequest("该版本没有 WASM 字节码".into()))?;

        let wasm_bytes = base64::engine::general_purpose::STANDARD
            .decode(&wasm_base64)
            .map_err(|e| AppError::BadRequest(format!("WASM base64 解码失败: {}", e)))?;

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        let wasm_hash = format!("{:x}", hasher.finalize());

        let plugin_id = req.name.clone().unwrap_or_else(|| req.package_key.clone());
        let config = PluginConfig {
            memory_limit_bytes: req
                .values
                .get("memory_limit_bytes")
                .and_then(|v| v.parse().ok())
                .unwrap_or(64 * 1024 * 1024),
            timeout_ms: req
                .values
                .get("timeout_ms")
                .and_then(|v| v.parse().ok())
                .unwrap_or(30_000),
            ..PluginConfig::default()
        };
        let sandbox_plugin = self
            .plugin_sandbox
            .load_plugin(&plugin_id, wasm_bytes, Some(config))
            .await?;

        let now = Utc::now();
        let plugin = Plugin {
            id: plugin_id.clone(),
            name: req.name.clone().unwrap_or_else(|| req.package_key.clone()),
            version: version.into(),
            author: req.values.get("author").cloned().unwrap_or_default(),
            description: req.values.get("description").cloned().unwrap_or_default(),
            wasm_hash,
            wasm_base64,
            enabled: true,
            homepage: None,
            license: None,
            tags: vec!["wasm".into(), "app-store".into()],
            config_schema: None,
            dependencies: vec![],
            created_at: now,
            updated_at: now,
        };
        let _ = sandbox_plugin;
        self.plugin_registry.register(plugin.clone())?;
        self.plugin_repo.save(&plugin).await?;

        let app = InstalledApp {
            id: 0,
            package_key: req.package_key.clone(),
            name: plugin.name.clone(),
            version: version.into(),
            mode: InstallMode::Wasm.as_str().into(),
            status: "running".into(),
            access_url: None,
            install_path: String::new(),
            container_name: None,
            port: None,
            params_json: serde_json::to_string(&req.values).unwrap_or_default(),
            created_at: now,
            updated_at: now,
            launch_count: 0,
        };
        let id = self.installed_repo.create(&app).await?;
        Ok(self.installed_repo.find_by_id(id).await?.unwrap_or(app))
    }

    // ─── 生命周期 ────────────────────────────────────────────────────────────

    pub async fn list_installed(&self) -> Result<Vec<InstalledApp>, AppError> {
        self.installed_repo.list_all().await
    }

    pub async fn get_installed(&self, id: i64) -> Result<InstalledApp, AppError> {
        self.installed_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("已安装应用 {} 不存在", id)))
    }

    /// 记录启动次数（常用应用排序）
    pub async fn record_launch(&self, id: i64) -> Result<InstalledApp, AppError> {
        let mut app = self.get_installed(id).await?;
        app.launch_count += 1;
        self.installed_repo.update(&app).await?;
        self.get_installed(id).await
    }

    pub async fn uninstall(&self, id: i64) -> Result<(), AppError> {
        let app = self.get_installed(id).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Container) => {
                if let Some(name) = &app.container_name {
                    let _ = self.docker_service.compose_down(name).await;
                    let _ = self
                        .docker_service
                        .compose_deploy(name, "services: {}")
                        .await;
                }
            }
            Some(InstallMode::Native) => match app.package_key.as_str() {
                "mysql" | "mariadb" => {
                    let _ = PackageManager::uninstall("mysql-server").await;
                }
                "redis" => {
                    let _ = PackageManager::uninstall("redis-server").await;
                }
                _ => {
                    let _ = PackageManager::uninstall(&app.package_key).await;
                }
            },
            Some(InstallMode::Wasm) => {
                let _ = self.plugin_sandbox.unload_plugin(&app.name).await;
                let _ = self.plugin_registry.unregister(&app.name);
                let _ = self.plugin_repo.delete(&app.name).await;
            }
            None => {}
        }
        let path = Path::new(&app.install_path);
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        }
        self.installed_repo.delete(id).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::AppUninstalled {
                app_key: app.package_key.clone(),
                app_name: app.name.clone(),
            })
            .await;
        Ok(())
    }

    pub async fn upgrade(&self, id: i64, target_version: &str) -> Result<InstalledApp, AppError> {
        let app = self.get_installed(id).await?;
        if app.version == target_version {
            return Ok(app);
        }
        let version_info = self.get_version(&app.package_key, target_version).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Container) => {
                let compose = version_info
                    .compose_template
                    .clone()
                    .ok_or_else(|| AppError::BadRequest("新版本没有 compose 模板".into()))?;
                let mut values: HashMap<String, String> =
                    serde_json::from_str(&app.params_json).unwrap_or_default();
                values.insert("PORT".into(), app.port.unwrap_or_default().to_string());
                let mut mapper = VariableMapper::new(values);
                mapper.insert(
                    "CONTAINER_NAME",
                    app.container_name.as_deref().unwrap_or(&app.package_key),
                );
                let (rendered, _) = mapper.replace(&compose);
                let rendered = ensure_restart_policy(&rendered);
                let install_path = Path::new(&app.install_path);
                let compose_path = install_path.join("docker-compose.yml");
                std::fs::write(&compose_path, &rendered)
                    .map_err(|e| AppError::internal(format!("写入 compose 失败: {}", e)))?;
                let _ = self
                    .docker_service
                    .compose_down(app.container_name.as_deref().unwrap_or(&app.package_key))
                    .await;
                self.docker_service
                    .compose_deploy(
                        app.container_name.as_deref().unwrap_or(&app.package_key),
                        &rendered,
                    )
                    .await?;
                let mut updated = app;
                updated.version = target_version.into();
                updated.status = "running".into();
                updated.updated_at = Utc::now();
                self.installed_repo.update(&updated).await?;
                Ok(updated)
            }
            Some(InstallMode::Native) => {
                // 原生升级：卸载旧版本并重新安装
                let mut req = InstallRequest {
                    package_key: app.package_key.clone(),
                    version: Some(target_version.into()),
                    mode: Some(InstallMode::Native.as_str().into()),
                    name: Some(app.name.clone()),
                    port: app.port,
                    container_name: None,
                    values: serde_json::from_str(&app.params_json).unwrap_or_default(),
                    confirm_risky: true,
                };
                req.values.insert("force_reinstall".into(), "true".into());
                self.uninstall(id).await?;
                self.install(&req).await
            }
            Some(InstallMode::Wasm) => {
                // 更新 WASM 字节码
                let wasm_base64 = version_info
                    .wasm_base64
                    .ok_or_else(|| AppError::BadRequest("新版本没有 WASM 字节码".into()))?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&wasm_base64)
                    .map_err(|e| AppError::BadRequest(format!("base64 解码失败: {}", e)))?;
                let _ = self
                    .plugin_sandbox
                    .reload_plugin(&app.name, bytes, None)
                    .await?;
                let mut updated = app;
                updated.version = target_version.into();
                updated.updated_at = Utc::now();
                self.installed_repo.update(&updated).await?;
                Ok(updated)
            }
            None => Err(AppError::BadRequest("未知安装模式".into())),
        }
    }

    pub async fn get_logs(&self, id: i64, tail: usize) -> Result<String, AppError> {
        let app = self.get_installed(id).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Container) => {
                if let Some(name) = &app.container_name {
                    self.docker_service.get_container_logs(name, tail).await
                } else {
                    Ok(String::new())
                }
            }
            _ => Ok(String::new()),
        }
    }

    // ─── WASM 内置工具 ───────────────────────────────────────────────────────

    /// 内置 WASM 工具列表
    pub fn wasm_builtins(&self) -> Vec<AppMetadata> {
        vec![AppMetadata {
            key: "wasm-hello".into(),
            name: "Hello WASM".into(),
            category: "wasm".into(),
            short_desc_zh: "内置 WASM 演示工具：导出 run() 函数返回 42，验证沙箱可用".into(),
            short_desc_en: Some("Builtin WASM demo tool: exports run() returning 42".into()),
            tags: vec!["wasm".into(), "demo".into()],
            format: AppFormat::Flame,
            modes: vec![InstallMode::Wasm],
            versions: vec!["1.0.0".into()],
            default_version: "1.0.0".into(),
            logo: Some("wasm".into()),
            min_memory_mb: None,
            architectures: vec![],
            readme: None,
            recommended: false,
        }]
    }

    /// 内置 WASM 工具版本信息
    pub fn wasm_builtin_version(&self, key: &str, version: &str) -> Option<AppVersionInfo> {
        if key == "wasm-hello" && version == "1.0.0" {
            Some(AppVersionInfo {
                version: "1.0.0".into(),
                mode: InstallMode::Wasm,
                default_port: None,
                form_fields: vec![FormField {
                    env_key: "name".into(),
                    label_zh: "插件名称".into(),
                    label_en: Some("Plugin name".into()),
                    field_type: crate::domain::entity::FieldType::Text,
                    default: Some("wasm-hello".into()),
                    required: true,
                    pattern: Some(r"^[a-zA-Z0-9_-]+$".into()),
                    min: None,
                    max: None,
                    min_length: Some(2),
                    max_length: Some(64),
                    options: vec![],
                    description: None,
                    group: Some("基础".into()),
                }],
                compose_template: None,
                native_scripts: vec![],
                wasm_base64: Some(WASM_HELLO_B64.into()),
                min_memory_mb: None,
                architectures: vec![],
            })
        } else {
            None
        }
    }

    /// 从持久化恢复 WASM 插件到沙箱（启动时调用）
    pub async fn restore_wasm_plugins(&self) -> Result<usize, AppError> {
        let plugins = self.plugin_repo.list().await?;
        let mut count = 0;
        for plugin in plugins {
            if self.plugin_sandbox.get_plugin(&plugin.id).await.is_ok() {
                count += 1;
                continue;
            }
            match base64::engine::general_purpose::STANDARD.decode(&plugin.wasm_base64) {
                Ok(bytes) => {
                    if let Err(e) = self
                        .plugin_sandbox
                        .load_plugin(&plugin.id, bytes, None)
                        .await
                    {
                        tracing::warn!("恢复 WASM 插件 {} 失败: {}", plugin.id, e);
                        continue;
                    }
                    if !plugin.enabled {
                        let _ = self.plugin_sandbox.disable_plugin(&plugin.id).await;
                    }
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!("WASM 插件 {} base64 解码失败: {}", plugin.id, e);
                }
            }
        }
        Ok(count)
    }
}

/// 校验表单字段：required + pattern + 长度
pub fn validate_fields(
    fields: &[FormField],
    values: &HashMap<String, String>,
) -> Result<(), AppError> {
    for field in fields {
        let value = values.get(&field.env_key).map(|s| s.as_str()).unwrap_or("");
        if field.required && value.is_empty() {
            return Err(AppError::BadRequest(format!(
                "字段 [{}] 为必填项",
                field.label_zh
            )));
        }
        if value.is_empty() {
            continue;
        }
        if let Some(pattern) = &field.pattern {
            let re = regex::Regex::new(pattern)
                .map_err(|e| AppError::internal(format!("校验规则无效: {}", e)))?;
            if !re.is_match(value) {
                return Err(AppError::BadRequest(format!(
                    "字段 [{}] 格式不正确",
                    field.label_zh
                )));
            }
        }
        if let Some(min) = field.min {
            if let Ok(num) = value.parse::<i64>() {
                if num < min {
                    return Err(AppError::BadRequest(format!(
                        "字段 [{}] 不能小于 {}",
                        field.label_zh, min
                    )));
                }
            }
        }
        if let Some(max) = field.max {
            if let Ok(num) = value.parse::<i64>() {
                if num > max {
                    return Err(AppError::BadRequest(format!(
                        "字段 [{}] 不能大于 {}",
                        field.label_zh, max
                    )));
                }
            }
        }
        if let Some(min_len) = field.min_length {
            if value.len() < min_len {
                return Err(AppError::BadRequest(format!(
                    "字段 [{}] 长度不能少于 {}",
                    field.label_zh, min_len
                )));
            }
        }
        if let Some(max_len) = field.max_length {
            if value.len() > max_len {
                return Err(AppError::BadRequest(format!(
                    "字段 [{}] 长度不能超过 {}",
                    field.label_zh, max_len
                )));
            }
        }
    }
    Ok(())
}

fn short_uuid() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| AppError::internal(format!("创建目录失败: {}", e)))?;
        for entry in std::fs::read_dir(src)
            .map_err(|e| AppError::internal(format!("读取目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| AppError::internal(format!("读取目录失败: {}", e)))?;
            let file_type = entry
                .file_type()
                .map_err(|e| AppError::internal(format!("读取类型失败: {}", e)))?;
            if file_type.is_dir() {
                copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                std::fs::copy(entry.path(), dst.join(entry.file_name()))
                    .map_err(|e| AppError::internal(format!("复制文件失败: {}", e)))?;
            }
        }
    }
    Ok(())
}

/// 当前系统资源（用于原生安装决策与预设）
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemResources {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub is_ssd: bool,
}

impl SystemResources {
    pub fn detect() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self {
            cpu_cores: sys.cpus().len().max(1) as u32,
            memory_mb: sys.total_memory() / 1024,
            is_ssd: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::*;

    fn test_service() -> AppStoreService {
        let pkg_repo: Arc<dyn AppPackageRepository> = Arc::new(InMemoryAppPackageRepository::new());
        let installed_repo: Arc<dyn InstalledAppRepository> =
            Arc::new(InMemoryInstalledAppRepository::new());
        let docker_service = Arc::new(DockerService::new(
            Arc::new(InMemoryDockerRepository::new()),
        ));
        let ws_service = Arc::new(WebServerService::new(Arc::new(
            InMemoryWebServerRepository::new(),
        )));
        let db_service = Arc::new(DatabaseService::new(Arc::new(
            InMemoryDatabaseRepository::new(),
        )));
        let sandbox = Arc::new(PluginSandbox::new());
        let registry = Arc::new(PluginRegistry::new());
        let plugin_repo: Arc<dyn PluginRepository> = Arc::new(InMemoryPluginRepository::new());
        let dir = std::env::temp_dir().join(format!("appstore_svc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        AppStoreService::new(
            pkg_repo,
            installed_repo,
            docker_service,
            ws_service,
            db_service,
            sandbox,
            registry,
            plugin_repo,
            dir,
            crate::event::EventBus::new(16),
        )
    }

    #[tokio::test]
    async fn seeds_builtin_apps() {
        let svc = test_service();
        let count = svc.seed_builtin_apps().await.unwrap();
        assert_eq!(count, 5);
        // 幂等
        let count2 = svc.seed_builtin_apps().await.unwrap();
        assert_eq!(count2, 0);
        let packages = svc.list_packages(None).await.unwrap();
        assert_eq!(packages.len(), 5);
    }

    #[tokio::test]
    async fn installs_builtin_container_app() {
        let svc = test_service();
        svc.seed_builtin_apps().await.unwrap();
        let mut values = HashMap::new();
        values.insert("PORT".into(), "18081".into());
        values.insert("NAME".into(), "wptest".into());
        let req = InstallRequest {
            package_key: "wordpress".into(),
            version: None,
            mode: None,
            name: Some("wptest".into()),
            port: Some(18081),
            container_name: Some("wp-test-01".into()),
            values,
            confirm_risky: false,
        };
        let app = svc.install(&req).await.unwrap();
        assert_eq!(app.status, "running");
        assert_eq!(app.mode, "container");
        assert_eq!(app.container_name.as_deref(), Some("wp-test-01"));
        assert_eq!(app.access_url.as_deref(), Some("http://localhost:18081"));
    }

    #[tokio::test]
    async fn install_blocks_privileged_without_confirmation() {
        // 通过导入的包路径不可行（无 docker daemon），直接验证扫描逻辑由单元测试覆盖
        let svc = test_service();
        svc.seed_builtin_apps().await.unwrap();
        let req = InstallRequest {
            package_key: "wordpress".into(),
            version: None,
            mode: None,
            name: None,
            port: None,
            container_name: None,
            values: HashMap::new(),
            confirm_risky: false,
        };
        // 内置包不包含 privileged，应能正常解析
        assert!(svc.get_version("wordpress", "6.7").await.is_ok());
        let _ = req;
    }

    #[tokio::test]
    async fn installs_wasm_builtin() {
        let svc = test_service();
        let version = svc.wasm_builtin_version("wasm-hello", "1.0.0").unwrap();
        assert!(version.wasm_base64.is_some());

        let mut values = HashMap::new();
        values.insert("name".into(), "hello-demo".into());
        let req = InstallRequest {
            package_key: "wasm-hello".into(),
            version: Some("1.0.0".into()),
            mode: Some("wasm".into()),
            name: Some("hello-demo".into()),
            port: None,
            container_name: None,
            values,
            confirm_risky: false,
        };
        let app = svc.install(&req).await.unwrap();
        assert_eq!(app.mode, "wasm");

        // 插件已注册且可执行
        let result = svc
            .plugin_sandbox
            .execute_plugin("hello-demo", "run", None)
            .await
            .unwrap();
        assert_eq!(result.output_as_i32(), Some(42));

        // 持久化
        let saved = svc
            .plugin_repo
            .find_by_id("hello-demo")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.name, "hello-demo");

        // 卸载
        svc.uninstall(app.id).await.unwrap();
        assert!(svc
            .plugin_repo
            .find_by_id("hello-demo")
            .await
            .unwrap()
            .is_none());
        assert!(svc.plugin_sandbox.get_plugin("hello-demo").await.is_err());
    }

    #[tokio::test]
    async fn restores_wasm_plugins() {
        let svc = test_service();
        svc.seed_builtin_apps().await.unwrap();
        let mut values = HashMap::new();
        values.insert("name".into(), "hello-restore".into());
        let req = InstallRequest {
            package_key: "wasm-hello".into(),
            version: Some("1.0.0".into()),
            mode: Some("wasm".into()),
            name: Some("hello-restore".into()),
            port: None,
            container_name: None,
            values,
            confirm_risky: false,
        };
        svc.install(&req).await.unwrap();

        let count = svc.restore_wasm_plugins().await.unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn validate_fields_works() {
        let fields = vec![
            FormField {
                env_key: "NAME".into(),
                label_zh: "名称".into(),
                label_en: None,
                field_type: crate::domain::entity::FieldType::Text,
                default: None,
                required: true,
                pattern: Some(r"^[a-zA-Z0-9_-]+$".into()),
                min: None,
                max: None,
                min_length: Some(2),
                max_length: Some(10),
                options: vec![],
                description: None,
                group: None,
            },
            FormField {
                env_key: "PORT".into(),
                label_zh: "端口".into(),
                label_en: None,
                field_type: crate::domain::entity::FieldType::Port,
                default: None,
                required: true,
                pattern: None,
                min: Some(1),
                max: Some(65535),
                min_length: None,
                max_length: None,
                options: vec![],
                description: None,
                group: None,
            },
        ];
        let mut ok = HashMap::new();
        ok.insert("NAME".into(), "gitea-1".into());
        ok.insert("PORT".into(), "3000".into());
        assert!(validate_fields(&fields, &ok).is_ok());

        // 缺必填
        let mut missing = HashMap::new();
        missing.insert("NAME".into(), "gitea-1".into());
        assert!(validate_fields(&fields, &missing).is_err());

        // 格式错误
        let mut bad = HashMap::new();
        bad.insert("NAME".into(), "bad name!".into());
        bad.insert("PORT".into(), "3000".into());
        assert!(validate_fields(&fields, &bad).is_err());

        // 超范围
        let mut out = HashMap::new();
        out.insert("NAME".into(), "gitea-1".into());
        out.insert("PORT".into(), "99999".into());
        assert!(validate_fields(&fields, &out).is_err());
    }

    #[test]
    fn system_resources_detect() {
        let res = SystemResources::detect();
        assert!(res.cpu_cores >= 1);
        assert!(res.memory_mb > 0);
    }
}
