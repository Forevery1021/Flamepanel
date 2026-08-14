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
        self.server_repo.create(instance).await
    }

    pub async fn update_server(&self, instance: &WebServerInstance) -> Result<(), AppError> {
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

    pub async fn write_site_config(
        &self,
        engine: &WebServerEngine,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> Result<(), AppError> {
        let config = self.generate_site_config(site, engine, ssl_cert, ssl_key);
        let config_path = format!("{}/{}", engine.sites_available_dir(), site.domain);
        // Phase A2：原子写 + 配置校验 + reload + 失败回滚
        self.manager
            .write_config_file_atomic(engine, &config_path, &config, true)
            .await
    }

    pub async fn enable_site(
        &self,
        engine: &WebServerEngine,
        site: &Website,
    ) -> Result<(), AppError> {
        let config = self.generate_site_config(site, engine, None, None);
        self.manager
            .enable_site(engine, &site.domain, &config)
            .await
    }

    pub async fn disable_site(
        &self,
        engine: &WebServerEngine,
        site: &Website,
    ) -> Result<(), AppError> {
        self.manager.disable_site(engine, &site.domain).await
    }

    /// 切换 Web 服务器实例引擎：更新实例信息并重新生成全局配置
    pub async fn switch_engine(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<WebServerInstance, AppError> {
        // 统一 Task 状态机（Phase B1：引擎切换跟踪）
        let task = self.task_tracker.create(
            crate::runtime::task_state::TaskKind::EngineSwitch,
            format!("switch server {id} to {}", new_engine.as_str()),
        );
        let task_id = task.id;
        let _ = self
            .task_tracker
            .transition(task_id, crate::runtime::task_state::TaskState::Running);
        self.task_tracker
            .update_progress(task_id, 50, "switching engine");

        let result = self.switch_engine_inner(id, new_engine).await;
        match &result {
            Ok(_) => {
                self.task_tracker
                    .update_progress(task_id, 100, "engine switched");
                let _ = self
                    .task_tracker
                    .transition(task_id, crate::runtime::task_state::TaskState::Success);
            }
            Err(e) => {
                self.task_tracker
                    .update_progress(task_id, 100, &format!("switch failed: {}", e));
                let _ = self
                    .task_tracker
                    .transition(task_id, crate::runtime::task_state::TaskState::Failed);
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
        let new = WebServerInstance {
            id: instance.id,
            engine: new_engine.as_str().into(),
            version: instance.version,
            status: instance.status,
            config_path: new_engine.default_config_path().into(),
            binary_path: Some(new_engine.binary_name().into()),
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
        // Phase A2：原子写 + 配置校验 + reload + 失败回滚
        self.manager
            .write_config_file_atomic(&engine, &instance.config_path, &config, true)
            .await?;
        Ok(instance)
    }

    // ── 原生安装 / 卸载 / 服务控制（对接系统包管理器与 systemd） ──

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
        let instance = WebServerInstance {
            id: 0,
            engine: engine.as_str().into(),
            version: Some(message),
            status: "running".into(),
            config_path: engine.default_config_path().into(),
            binary_path: Some(engine.binary_name().into()),
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
}
