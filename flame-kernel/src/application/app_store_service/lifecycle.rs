//! 已安装应用生命周期：查询、卸载、升级、日志与原生服务启停。

use super::*;
use crate::domain::compose_merge::{
    three_way_merge, ComposeMerge, ConflictStrategy, UpgradePreview,
};

impl AppStoreService {
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
        app.record_launch();
        self.installed_repo.update(&app).await?;
        self.get_installed(id).await
    }

    pub async fn uninstall(&self, id: i64) -> Result<(), AppError> {
        let app = self.get_installed(id).await?;
        // Doc/20 §4.3：pre_uninstall 钩子（compose down 之前）。
        // 钩子执行失败不阻断卸载（应用即将整体移除，阻断只会留下僵尸容器），
        // 但错误必须记录。容器模式才取版本 hooks。
        if InstallMode::from_name(&app.mode) == Some(InstallMode::Container) {
            if let Ok(version_info) = self.get_version(&app.package_key, &app.version).await {
                let pkg_root = self
                    .get_package(&app.package_key)
                    .await
                    .ok()
                    .and_then(|p| p.source_path)
                    .map(std::path::PathBuf::from);
                if let Err(e) = self
                    .run_hook(
                        &app.package_key,
                        pkg_root.as_deref(),
                        &version_info.hooks,
                        HookStage::PreUninstall,
                        None,
                    )
                    .await
                {
                    tracing::warn!("pre_uninstall hook failed (non-fatal): {}", e);
                }
            }
        }
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Container) => {
                if let Some(name) = &app.container_name {
                    // `down` 已移除容器与项目网络；旧实现随后再 `up` 一个 `services: {}` 的空项目，
                    // 在持久项目目录语义下只会留下空 compose 文件，已删除。
                    let _ = self.docker_service.compose_down(name).await;
                }
            }
            Some(InstallMode::Native) => {
                // Doc/16 §6.1：优先路由到注册的 NativeInstaller（数据库/Web 引擎精细卸载），
                // 未注册的退回按包名粗粒度卸载。
                match self.native_installers.find(&app.package_key) {
                    Some(installer) => installer.uninstall(&app).await?,
                    None => {
                        let _ = self.package_manager.uninstall(&app.package_key).await;
                    }
                }
            }
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

    /// 升级（冲突即中止）。等价 `upgrade_with(id, target, ConflictStrategy::Abort)`。
    pub async fn upgrade(&self, id: i64, target_version: &str) -> Result<InstalledApp, AppError> {
        self.upgrade_with(id, target_version, ConflictStrategy::Abort)
            .await
    }

    /// 按目标版本渲染容器模式的新 compose（预览与执行共用，保证两者一致）。
    fn render_upgrade_compose(&self, app: &InstalledApp, template: &str) -> String {
        let mut values: HashMap<String, String> =
            serde_json::from_str(&app.params_json).unwrap_or_default();
        values.insert("PORT".into(), app.port.unwrap_or_default().to_string());
        let mut mapper = self.variable_mapper_factory.create(values);
        mapper.insert(
            "CONTAINER_NAME",
            app.container_name
                .as_deref()
                .unwrap_or(&app.package_key)
                .to_string(),
        );
        let (rendered, _) = mapper.replace(template);
        self.security_scanner.ensure_restart_policy(&rendered)
    }

    /// 读取三方合并的三份输入并合并（不落盘）。
    fn merge_for_upgrade(
        &self,
        app: &InstalledApp,
        upstream: &str,
        strategy: ConflictStrategy,
    ) -> Result<(ComposeMerge, bool), AppError> {
        let install_path = Path::new(&app.install_path);
        let base = std::fs::read_to_string(compose_base_path(install_path)).ok();
        let base_available = base.is_some();
        let merge = match std::fs::read_to_string(install_path.join("docker-compose.yml")) {
            Ok(local) => three_way_merge(base.as_deref(), &local, upstream, strategy)?,
            // 本地 compose 丢失：无可保留的修改，直接采用新版本
            Err(_) => ComposeMerge {
                merged: upstream.to_string(),
                conflicts: vec![],
                local_changes: vec![],
                upstream_changes: vec!["$".into()],
                reformatted: false,
            },
        };
        Ok((merge, base_available))
    }

    /// 升级预览：解析目标版本、渲染新 compose 并与本地做三方合并，不改动任何文件。
    pub async fn upgrade_preview(
        &self,
        id: i64,
        target_version: &str,
    ) -> Result<UpgradePreview, AppError> {
        let app = self.get_installed(id).await?;
        let version_info = self.get_version(&app.package_key, target_version).await?;
        app.can_upgrade_to(&version_info.version)?;
        let mut preview = UpgradePreview {
            from_version: app.version.clone(),
            to_version: version_info.version.clone(),
            mode: app.mode.clone(),
            up_to_date: app.version == version_info.version,
            base_available: false,
            merge: None,
        };
        if preview.up_to_date {
            return Ok(preview);
        }
        if let (Some(InstallMode::Container), Some(template)) = (
            InstallMode::from_name(&app.mode),
            version_info.compose_template.as_deref(),
        ) {
            let upstream = self.render_upgrade_compose(&app, template);
            let (merge, base_available) =
                self.merge_for_upgrade(&app, &upstream, ConflictStrategy::Abort)?;
            preview.base_available = base_available;
            preview.merge = Some(merge);
        }
        Ok(preview)
    }

    /// 升级；容器模式对 compose 做三方合并（保留用户修改），冲突按 `strategy` 处理。
    pub async fn upgrade_with(
        &self,
        id: i64,
        target_version: &str,
        strategy: ConflictStrategy,
    ) -> Result<InstalledApp, AppError> {
        let app = self.get_installed(id).await?;
        // 领域规则：降级拒绝（行为增强，逻辑上移到领域层）；同版本保持幂等短路
        app.can_upgrade_to(target_version)?;
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
                let upstream = self.render_upgrade_compose(&app, &compose);
                // 先合并再执行钩子：冲突在任何副作用之前暴露
                let (merge, _) = self.merge_for_upgrade(&app, &upstream, strategy)?;
                if merge.has_conflicts() && strategy == ConflictStrategy::Abort {
                    let paths: Vec<&str> = merge
                        .conflicts
                        .iter()
                        .map(|c| c.path.as_str())
                        .take(10)
                        .collect();
                    return Err(AppError::Conflict(format!(
                        "本地 compose 与新版本有 {} 处冲突（{}）；请先预览，再选择保留本地或采用新版本后重试",
                        merge.conflicts.len(),
                        paths.join(", ")
                    )));
                }

                // Doc/20 §4.3：pre_upgrade 钩子（换镜像前，失败即中止升级）
                let pkg_root = self
                    .get_package(&app.package_key)
                    .await
                    .ok()
                    .and_then(|p| p.source_path)
                    .map(std::path::PathBuf::from);
                self.run_hook(
                    &app.package_key,
                    pkg_root.as_deref(),
                    &version_info.hooks,
                    HookStage::PreUpgrade,
                    None,
                )
                .await?;
                let install_path = Path::new(&app.install_path);
                let compose_path = install_path.join("docker-compose.yml");
                // 升级前快照（回滚用）：.flamepanel/docker-compose.<旧版本>.yml
                if let Ok(current) = std::fs::read_to_string(&compose_path) {
                    let backup = compose_backup_path(install_path, &app.version);
                    if let Err(e) = write_private(&backup, &current) {
                        tracing::warn!("备份升级前 compose 失败（继续升级）: {}", e);
                    }
                }
                let rendered = merge.merged;
                std::fs::write(&compose_path, &rendered)
                    .map_err(|e| AppError::internal(format!("写入 compose 失败: {}", e)))?;
                let _ = self
                    .docker_service
                    .compose_down(app.container_name.as_deref().unwrap_or(&app.package_key))
                    .await;
                self.docker_service
                    .compose_deploy_at(
                        app.container_name.as_deref().unwrap_or(&app.package_key),
                        &rendered,
                        install_path,
                    )
                    .await?;
                // 新基线是**上游渲染结果**（不是合并结果）：下次升级据此区分用户修改
                save_compose_base(install_path, &upstream);
                // Doc/20 §4.3：post_upgrade 钩子（升级成功后；失败仅告警不回滚，
                // 因容器已运行新版本，此时回滚反而扩大故障面）
                if let Err(e) = self
                    .run_hook(
                        &app.package_key,
                        pkg_root.as_deref(),
                        &version_info.hooks,
                        HookStage::PostUpgrade,
                        None,
                    )
                    .await
                {
                    tracing::warn!("post_upgrade hook failed (non-fatal): {}", e);
                }

                let from_version = app.version.clone();
                let mut updated = app;
                updated.mark_upgraded(target_version);
                self.installed_repo.update(&updated).await?;
                self.publish_upgraded(&updated, &from_version).await?;
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
                    acknowledge_scripts: true,
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
                    .reload_plugin(&app.name, bytes, None, None)
                    .await?;
                let from_version = app.version.clone();
                let mut updated = app;
                updated.mark_upgraded(target_version);
                self.installed_repo.update(&updated).await?;
                self.publish_upgraded(&updated, &from_version).await?;
                Ok(updated)
            }
            None => Err(AppError::BadRequest("未知安装模式".into())),
        }
    }

    /// 发布应用升级事件（原生模式走 uninstall+install，已分别发布 Uninstalled/Installed）
    pub(super) async fn publish_upgraded(
        &self,
        app: &InstalledApp,
        from_version: &str,
    ) -> Result<(), AppError> {
        let _ = self
            .event_bus
            .publish(DomainEvent::AppUpgraded {
                app_key: app.package_key.clone(),
                app_name: app.name.clone(),
                from: from_version.to_string(),
                to: app.version.clone(),
            })
            .await;
        Ok(())
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

    // ─── 原生软件生命周期（Doc/16 §6.5：status/start/stop/restart 统一暴露） ──

    /// 原生已安装应用运行状态（running/stopped）。
    /// 容器模式查询 Docker 状态；原生模式路由到 NativeInstaller。
    pub async fn native_app_status(&self, id: i64) -> Result<String, AppError> {
        let app = self.get_installed(id).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Container) => match &app.container_name {
                Some(name) => match self.docker_service.inspect_container(name).await {
                    Ok(info) => {
                        let running = info
                            .get("State")
                            .and_then(|s| s.get("Running"))
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false);
                        Ok(if running { "running" } else { "stopped" }.into())
                    }
                    Err(_) => Ok("stopped".into()),
                },
                None => Ok("stopped".into()),
            },
            Some(InstallMode::Native) => match self.native_installers.find(&app.package_key) {
                Some(installer) => installer.status(&app).await,
                None => Ok(app.status),
            },
            _ => Ok(app.status),
        }
    }

    /// 启动原生已安装应用服务。
    pub async fn native_app_start(&self, id: i64) -> Result<(), AppError> {
        let app = self.get_installed(id).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Native) => {
                let installer = self
                    .native_installers
                    .find(&app.package_key)
                    .ok_or_else(|| {
                        AppError::BadRequest(format!(
                            "应用 {} 无原生安装器，不支持启停",
                            app.package_key
                        ))
                    })?;
                installer.start(&app).await?;
                let mut updated = app;
                updated.status = "running".into();
                updated.updated_at = Utc::now();
                self.installed_repo.update(&updated).await?;
                Ok(())
            }
            _ => Err(AppError::BadRequest("仅原生模式应用支持该操作".into())),
        }
    }

    /// 停止原生已安装应用服务。
    pub async fn native_app_stop(&self, id: i64) -> Result<(), AppError> {
        let app = self.get_installed(id).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Native) => {
                let installer = self
                    .native_installers
                    .find(&app.package_key)
                    .ok_or_else(|| {
                        AppError::BadRequest(format!(
                            "应用 {} 无原生安装器，不支持启停",
                            app.package_key
                        ))
                    })?;
                installer.stop(&app).await?;
                let mut updated = app;
                updated.status = "stopped".into();
                updated.updated_at = Utc::now();
                self.installed_repo.update(&updated).await?;
                Ok(())
            }
            _ => Err(AppError::BadRequest("仅原生模式应用支持该操作".into())),
        }
    }

    /// 重启原生已安装应用服务。
    pub async fn native_app_restart(&self, id: i64) -> Result<(), AppError> {
        let app = self.get_installed(id).await?;
        match InstallMode::from_name(&app.mode) {
            Some(InstallMode::Native) => {
                let installer = self
                    .native_installers
                    .find(&app.package_key)
                    .ok_or_else(|| {
                        AppError::BadRequest(format!(
                            "应用 {} 无原生安装器，不支持启停",
                            app.package_key
                        ))
                    })?;
                installer.restart(&app).await?;
                let mut updated = app;
                updated.status = "running".into();
                updated.updated_at = Utc::now();
                self.installed_repo.update(&updated).await?;
                Ok(())
            }
            _ => Err(AppError::BadRequest("仅原生模式应用支持该操作".into())),
        }
    }

    // ─── WASM 内置工具 ───────────────────────────────────────────────────────
}

/// 面板在安装目录内的私有元数据目录。
pub(crate) const META_DIR: &str = ".flamepanel";

pub(crate) fn compose_base_path(install_path: &Path) -> PathBuf {
    install_path.join(META_DIR).join("compose.base.yml")
}

fn compose_backup_path(install_path: &Path, version: &str) -> PathBuf {
    // 版本号只保留安全字符，防止路径穿越
    let safe: String = version
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    install_path
        .join(META_DIR)
        .join(format!("docker-compose.{safe}.yml"))
}

fn write_private(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)
}

/// 保存三方合并基线（best-effort：失败只告警，下次升级退化为无基线合并）。
pub(crate) fn save_compose_base(install_path: &Path, rendered: &str) {
    if let Err(e) = write_private(&compose_base_path(install_path), rendered) {
        tracing::warn!("保存 compose 基线失败: {}", e);
    }
}

#[cfg(test)]
mod upgrade_file_tests {
    use super::*;

    #[test]
    fn backup_path_sanitises_version() {
        let p = compose_backup_path(Path::new("/opt/app"), "../../etc/passwd");
        assert_eq!(
            p,
            PathBuf::from("/opt/app/.flamepanel/docker-compose..._.._etc_passwd.yml")
        );
        assert!(p.starts_with("/opt/app/.flamepanel"));
    }

    #[test]
    fn base_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        save_compose_base(dir.path(), "services: {}\n");
        assert_eq!(
            std::fs::read_to_string(compose_base_path(dir.path())).unwrap(),
            "services: {}\n"
        );
    }
}
