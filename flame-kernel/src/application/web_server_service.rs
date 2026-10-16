//! Web 服务器领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::webserver::{get_config_generator, WebServerEngine, WebServerManager};
use std::sync::Arc;

pub struct WebServerService {
    pub server_repo: Arc<dyn WebServerRepository>,
    pub manager: WebServerManager,
    pub native_manager: crate::webserver::native::WebServerNativeManager,
    /// 统一 Task 状态机跟踪器（Phase B1：安装/引擎切换/批量节点共用）
    pub task_tracker: crate::runtime::task_state::TaskTracker,
}

impl WebServerService {
    pub fn new(
        server_repo: Arc<dyn WebServerRepository>,
        runner: crate::application::execution_mode::SharedCommandRunner,
    ) -> Self {
        Self::new_with_task_store(server_repo, runner, None)
    }

    /// 注入统一 Task 状态机持久化存储（Phase B1 扩展：进程重启可恢复）。
    pub fn new_with_task_store(
        server_repo: Arc<dyn WebServerRepository>,
        runner: crate::application::execution_mode::SharedCommandRunner,
        task_store: Option<crate::runtime::task_state::TaskStoreRef>,
    ) -> Self {
        let task_tracker = match task_store {
            Some(store) => crate::runtime::task_state::TaskTracker::with_store(store),
            None => crate::runtime::task_state::TaskTracker::new(),
        };
        Self::with_task_tracker(server_repo, runner, task_tracker)
    }

    /// 注入共享的统一 Task 状态机跟踪器（Phase B1 扩展：多服务共享同一 tracker，供前端统一查询/取消）。
    pub fn with_task_tracker(
        server_repo: Arc<dyn WebServerRepository>,
        runner: crate::application::execution_mode::SharedCommandRunner,
        task_tracker: crate::runtime::task_state::TaskTracker,
    ) -> Self {
        Self {
            server_repo,
            manager: WebServerManager::new_with_runner(runner.clone()),
            native_manager: crate::webserver::native::WebServerNativeManager::new(runner),
            task_tracker,
        }
    }

    pub async fn list_servers(&self) -> Result<Vec<WebServerInstance>, AppError> {
        self.server_repo.list_all().await
    }

    pub async fn list_servers_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<WebServerInstance>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.server_repo.count().await?;
        let data = self
            .server_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_server(&self, id: i64) -> Result<WebServerInstance, AppError> {
        self.server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Web server {} not found", id)))
    }

    pub async fn create_server(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        validate_instance(instance)?;
        self.server_repo.create(instance).await
    }

    pub async fn update_server(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        validate_instance(instance)?;
        self.server_repo.update(instance).await
    }

    pub async fn delete_server(&self, id: i64) -> Result<(), AppError> {
        self.server_repo.delete(id).await
    }

    pub async fn start_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.start(&instance).await
    }

    pub async fn stop_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.stop(&instance).await
    }

    pub async fn restart_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.restart(&instance).await
    }

    pub async fn reload_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.reload(&instance).await
    }

    pub async fn test_server_config(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.config_test(&instance).await
    }

    pub async fn check_server_status(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.check_status(&instance).await
    }

    pub fn generate_site_config(
        &self,
        site: &Website,
        engine: &WebServerEngine,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String {
        let generator = get_config_generator(engine);
        generator.generate_site_config(site, ssl_cert, ssl_key)
    }

    pub fn generate_global_config(
        &self,
        engine: &WebServerEngine,
        port: u16,
        workers: u32,
    ) -> String {
        let generator = get_config_generator(engine);
        generator.generate_global_config(port, workers)
    }

    pub fn generate_reverse_proxy_config(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        proxy_pass: &str,
        port: u16,
    ) -> String {
        let generator = get_config_generator(engine);
        generator.generate_reverse_proxy_config(domain, proxy_pass, port)
    }

    /// 生成并部署站点配置（面板自有站点目录 + 主配置钩子 + 校验 / reload / 回滚）。
    pub async fn deploy_site(
        &self,
        engine: &WebServerEngine,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> Result<crate::webserver::SiteDeployment, AppError> {
        let config = self.generate_site_config(site, engine, ssl_cert, ssl_key);
        self.manager
            .deploy_site(engine, &site.domain, &config, Some(&site.root_path))
            .await
    }

    /// 启用 / 停用已部署站点（停用 = 重命名为 `.disabled` 并 reload）。
    pub async fn set_site_enabled(
        &self,
        engine: &WebServerEngine,
        site: &Website,
        enabled: bool,
    ) -> Result<(), AppError> {
        self.manager
            .set_site_enabled(engine, &site.domain, enabled, Some(&site.root_path))
            .await
    }

    /// 切换 Web 服务器实例引擎：更新实例信息并重新生成全局配置
    pub async fn switch_engine(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<WebServerInstance, AppError> {
        // 统一 Task 状态机（Phase B1：引擎切换跟踪）
        let task = self
            .task_tracker
            .create(
                crate::runtime::task_state::TaskKind::EngineSwitch,
                format!("switch server {id} to {}", new_engine.as_str()),
            )
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let task_id = task.id;
        let _ = self
            .task_tracker
            .transition(task_id, crate::runtime::task_state::TaskState::Running)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        self.task_tracker
            .update_progress(task_id, 50, "switching engine")
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let result = self.switch_engine_inner(id, new_engine).await;
        match &result {
            Ok(_) => {
                self.task_tracker
                    .update_progress(task_id, 100, "engine switched")
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
                let _ = self
                    .task_tracker
                    .transition(task_id, crate::runtime::task_state::TaskState::Success)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
            Err(e) => {
                self.task_tracker
                    .update_progress(task_id, 100, &format!("switch failed: {}", e))
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
                let _ = self
                    .task_tracker
                    .transition(task_id, crate::runtime::task_state::TaskState::Failed)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
        }
        result
    }

    /// 引擎切换本体（被 `switch_engine` 包裹以接入统一 Task 状态机）。
    async fn switch_engine_inner(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<WebServerInstance, AppError> {
        let instance = self.get_server(id).await?;
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let new = WebServerInstance {
            id: instance.id,
            engine: new_engine.as_str().into(),
            version: instance.version,
            status: instance.status,
            config_path: new_engine.default_config_path_for(&distro).into(),
            binary_path: Some(new_engine.default_binary_path_for(&distro).into()),
            port: instance.port,
            created_at: instance.created_at,
            resource_version: instance.resource_version,
        };
        self.server_repo.update(&new).await?;
        Ok(new)
    }

    /// 应用性能预设：根据引擎生成全局配置并写入
    pub async fn apply_preset(
        &self,
        id: i64,
        preset: &crate::webserver::preset::PerformancePreset,
    ) -> Result<WebServerInstance, AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        let config = self.generate_global_config(
            &engine,
            instance.port as u16,
            preset.worker_processes(&instance.engine),
        );
        // 存量实例可能早于路径校验写入：落盘前再确认一次（root 模式下 host 即本机文件系统）
        crate::webserver::helper::policy::check_path(&instance.config_path)?;
        // Phase A2：原子写 + 配置校验 + reload + 失败回滚
        self.manager
            .write_config_file_atomic(&engine, &instance.config_path, &config, true)
            .await?;
        Ok(instance)
    }

    // ── 原生安装 / 卸载 / 服务控制（对接系统包管理器与 systemd） ──

    /// 访问原生管理器的 systemd 服务管理器（原生生命周期 start/stop/restart 使用）。
    pub fn native_service_manager(&self) -> &crate::infrastructure::os::ServiceManager {
        self.native_manager.service_manager()
    }

    /// 检测本机已安装的 Web 服务器（1Panel 风格）
    pub async fn native_detect(&self) -> Vec<crate::webserver::native::NativeWebServerInfo> {
        self.native_manager.detect_all().await
    }

    /// 原生安装指定引擎并注册实例
    pub async fn native_install(
        &self,
        engine: &WebServerEngine,
        version: Option<&str>,
    ) -> Result<WebServerInstance, AppError> {
        let message = self.native_manager.install(engine, version).await?;
        // 自动注册实例，便于统一管理
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let instance = WebServerInstance {
            id: 0,
            engine: engine.as_str().into(),
            version: Some(message),
            status: "running".into(),
            config_path: engine.default_config_path_for(&distro).into(),
            binary_path: Some(engine.default_binary_path_for(&distro).into()),
            port: engine.default_port() as i32,
            created_at: chrono::Utc::now(),
            resource_version: 0,
        };
        let id = self.server_repo.create(&instance).await?;
        let created = self.get_server(id).await?;
        Ok(created)
    }

    /// 原生卸载并删除注册实例
    pub async fn native_uninstall(&self, id: i64) -> Result<(), AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        self.native_manager.uninstall(&engine).await?;
        self.server_repo.delete(id).await
    }

    /// 按引擎名原生卸载（同时清理该引擎的注册实例）
    pub async fn native_uninstall_by_engine(
        &self,
        engine: &WebServerEngine,
    ) -> Result<(), AppError> {
        self.native_manager.uninstall(engine).await?;
        let instances = self.server_repo.find_by_engine(engine.as_str()).await?;
        for inst in instances {
            self.server_repo.delete(inst.id).await?;
        }
        Ok(())
    }

    /// systemd 开机自启开关（systemd 不可用时容错降级）
    pub async fn set_autostart(
        &self,
        id: i64,
        enabled: bool,
    ) -> Result<WebServerInstance, AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        self.set_autostart_by_engine(&engine, enabled).await?;
        Ok(instance)
    }

    /// 按引擎名设置开机自启
    pub async fn set_autostart_by_engine(
        &self,
        engine: &WebServerEngine,
        enabled: bool,
    ) -> Result<(), AppError> {
        let result = self.native_manager.set_autostart(engine, enabled).await;
        if let Err(e) = result {
            tracing::warn!(
                "systemd autostart {} for {} unavailable, ignored: {}",
                if enabled { "enable" } else { "disable" },
                engine.as_str(),
                e
            );
        }
        Ok(())
    }

    /// 实例的原生状态详情（安装/版本/服务/端口）
    pub async fn native_status(
        &self,
        id: i64,
    ) -> Result<crate::webserver::native::NativeWebServerInfo, AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        Ok(self.native_manager.detect(&engine).await)
    }

    // ── 配套能力（安装时"各取所需"，Doc/13 §5.4 阶段 3）─────────────────────

    /// 指定引擎的配套能力清单（PHP-FPM / Lua / Redis / 扩展模块 / 自动 HTTPS）。
    pub async fn engine_capabilities(
        &self,
        engine: &WebServerEngine,
    ) -> Vec<crate::webserver::capability::CapabilityInfo> {
        self.native_manager.engine_capabilities(engine).await
    }

    /// 批量安装指定引擎的配套依赖（不依赖引擎已安装，可先装配套再装引擎）。
    ///
    /// 返回 `{ installed: [...], failed: [...] }`，前端据此展示结果。
    pub async fn install_capabilities(
        &self,
        engine: &WebServerEngine,
        caps: &[crate::webserver::capability::WebServerCapability],
    ) -> Result<serde_json::Value, AppError> {
        let (installed, failed) = self
            .native_manager
            .install_capabilities(engine, caps)
            .await?;
        Ok(serde_json::json!({
            "engine": engine.as_str(),
            "installed": installed,
            "failed": failed,
            "success": failed.is_empty(),
        }))
    }
}

/// 实例登记字段校验（创建 / 更新共用）。
///
/// `config_path` 会被读（`GET /config`）、写（性能预设）并作为启动参数，`binary_path` 会被特权执行；
/// 二者此前不设限，拥有 `web_server:create|update` 的角色即可读任意文件、以面板身份执行任意程序。
/// 现要求：引擎已知；`config_path` 位于 Web 引擎配置根之下（与特权助手同一份
/// [`check_path`](crate::webserver::helper::policy::check_path)）；`binary_path` 为该引擎的已知二进制。
pub fn validate_instance(instance: &WebServerInstance) -> Result<(), AppError> {
    let engine = WebServerEngine::from_name(&instance.engine).ok_or_else(|| {
        AppError::ValidationError(format!("engine: unknown engine {:?}", instance.engine))
    })?;
    if crate::webserver::helper::policy::check_path(&instance.config_path).is_err() {
        return Err(AppError::ValidationError(format!(
            "config_path: must be a canonical absolute path under {}",
            crate::webserver::helper::policy::ENGINE_CONFIG_ROOTS.join(", ")
        )));
    }
    if let Some(bin) = instance.binary_path.as_deref().filter(|b| !b.is_empty()) {
        if !engine.binary_allowed(bin) {
            return Err(AppError::ValidationError(format!(
                "binary_path: {bin:?} is not a known {} binary",
                engine.as_str()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod instance_validation_tests {
    use super::*;

    fn inst(engine: &str, config: &str, bin: Option<&str>) -> WebServerInstance {
        WebServerInstance {
            id: 0,
            engine: engine.into(),
            version: None,
            status: "stopped".into(),
            config_path: config.into(),
            binary_path: bin.map(Into::into),
            port: 80,
            created_at: chrono::Utc::now(),
            resource_version: 0,
        }
    }

    #[test]
    fn engine_defaults_are_accepted() {
        for e in ["nginx", "apache", "openlitespeed", "openresty", "caddy"] {
            let engine = WebServerEngine::from_name(e).unwrap();
            let i = inst(
                e,
                engine.default_config_path(),
                Some(engine.default_binary_path()),
            );
            assert!(validate_instance(&i).is_ok(), "{e}");
            for c in engine.binary_candidates() {
                assert!(
                    validate_instance(&inst(e, engine.default_config_path(), Some(c))).is_ok(),
                    "{e} {c}"
                );
            }
        }
        assert!(validate_instance(&inst(
            "apache",
            "/etc/apache2/apache2.conf",
            Some("/usr/sbin/apache2")
        ))
        .is_ok());
        assert!(validate_instance(&inst(
            "caddy",
            "/etc/caddy/Caddyfile",
            Some("/usr/local/bin/caddy")
        ))
        .is_ok());
        assert!(validate_instance(&inst("nginx", "/etc/nginx/nginx.conf", None)).is_ok());
        assert!(validate_instance(&inst("nginx", "/etc/nginx/nginx.conf", Some(""))).is_ok());
    }

    #[test]
    fn arbitrary_files_and_programs_are_refused() {
        for bad in [
            "/etc/shadow",
            "/var/lib/flamepanel/flamepanel.db",
            "/etc/nginx/../shadow",
            "etc/nginx/nginx.conf",
            "/etc/nginxx/nginx.conf",
            "/etc/nginx//nginx.conf",
        ] {
            let err = validate_instance(&inst("nginx", bad, None)).unwrap_err();
            assert!(
                matches!(err, AppError::ValidationError(ref m) if m.starts_with("config_path:")),
                "{bad}"
            );
        }
        for bad in [
            "/usr/bin/python3",
            "/bin/sh",
            "python3",
            "/tmp/nginx",
            "/usr/sbin/../bin/sh",
            "/usr/sbin/caddy",
        ] {
            let err =
                validate_instance(&inst("nginx", "/etc/nginx/nginx.conf", Some(bad))).unwrap_err();
            assert!(
                matches!(err, AppError::ValidationError(ref m) if m.starts_with("binary_path:")),
                "{bad}"
            );
        }
        assert!(matches!(
            validate_instance(&inst("iis", "/etc/nginx/nginx.conf", None)),
            Err(AppError::ValidationError(ref m)) if m.starts_with("engine:")
        ));
    }
}
