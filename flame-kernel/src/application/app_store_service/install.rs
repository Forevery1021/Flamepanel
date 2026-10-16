//! 安装编排：请求校验、互斥锁、三种模式（容器 / 原生 / WASM）。

use super::*;

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
    #[serde(default)]
    pub confirm_risky: bool,
    /// 安装包含原生脚本（native_scripts）时，需用户显式确认执行第三方脚本。
    #[serde(default)]
    pub acknowledge_scripts: bool,
}

impl AppStoreService {
    pub async fn install(&self, req: &InstallRequest) -> Result<InstalledApp, AppError> {
        // F-26: 安装幂等锁——同一应用包同时只能有一个安装任务，防止重复容器与目录竞争。
        {
            let mut locks = self
                .install_locks
                .lock()
                .map_err(|_| AppError::internal("install lock poisoned"))?;
            if locks.contains_key(&req.package_key) {
                return Err(AppError::Conflict(format!(
                    "install for '{}' is already in progress",
                    req.package_key
                )));
            }
            locks.insert(req.package_key.clone(), ());
        }

        // 持久化失败或请求被取消也要释放安装锁，不能把同包后续请求永久卡住。
        let _install_guard = InstallLockGuard {
            service: self,
            key: &req.package_key,
        };

        // 统一 Task 状态机（Phase B1）：为长耗时安装操作提供一致的进度/状态跟踪
        let task = self
            .task_tracker
            .create(
                crate::runtime::task_state::TaskKind::Install,
                format!("install {}", req.package_key),
            )
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let task_id = task.id;
        let _ = self
            .task_tracker
            .transition(task_id, crate::runtime::task_state::TaskState::Running)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let result = self.install_inner(req).await;

        match &result {
            Ok(_) => {
                let _ = self
                    .task_tracker
                    .transition(task_id, crate::runtime::task_state::TaskState::Success)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
            Err(e) => {
                let _ = self
                    .task_tracker
                    .update_progress(task_id, 100, &format!("install failed: {}", e))
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

    /// 安装编排本体（被 `install` 包裹以接入统一 Task 状态机）。
    pub(super) async fn install_inner(
        &self,
        req: &InstallRequest,
    ) -> Result<InstalledApp, AppError> {
        let metadata = self.get_metadata(&req.package_key).await?;
        // M-A Issue 1.3：远程包安装前验签（本地包直接放行）
        let pkg = self.ensure_remote_package_fetched(&req.package_key).await?;
        self.verify_remote_package_at_install(&pkg).await?;
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

    pub(super) async fn install_container(
        &self,
        req: &InstallRequest,
        version: &str,
    ) -> Result<InstalledApp, AppError> {
        let version_info = self.get_version(&req.package_key, version).await?;
        let compose = version_info
            .compose_template
            .clone()
            .ok_or_else(|| AppError::BadRequest("该版本没有 docker-compose.yml".into()))?;

        // Doc/20 §4.3：pre_install 钩子（compose up 之前，失败即中止）
        let pkg_root = self
            .get_package(&req.package_key)
            .await
            .ok()
            .and_then(|p| p.source_path)
            .map(std::path::PathBuf::from);
        self.run_hook(
            &req.package_key,
            pkg_root.as_deref(),
            &version_info.hooks,
            HookStage::PreInstall,
            None,
        )
        .await?;

        // 表单校验
        validate_fields(&version_info.form_fields, &req.values)?;

        // 构建变量映射
        let container_name = req
            .container_name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("{}-{}", req.package_key, short_uuid()));

        // F-05 加固：package_key 与 container_name 均为用户可控输入，且直接拼进
        // 安装根目录路径。在创建目录前做字符白名单校验，杜绝 `../` 穿越任意写
        // （与 docker.rs 的 sanitize_project_name 同一威胁模型：面板常以高权限运行）。
        sanitize_path_component(&req.package_key, "package_key")?;
        sanitize_path_component(&container_name, "container_name")?;

        let name = req.name.clone().unwrap_or_else(|| req.package_key.clone());
        let port = req.port.or(version_info.default_port).unwrap_or(0);
        let install_path = self.apps_dir.join(&req.package_key).join(&container_name);
        std::fs::create_dir_all(&install_path)
            .map_err(|e| AppError::internal(format!("创建安装目录失败: {}", e)))?;

        let mut mapper = self.variable_mapper_factory.create(req.values.clone());
        mapper.insert("CONTAINER_NAME", container_name.clone());
        mapper.insert("NAME", name.clone());
        mapper.insert("PORT", port.to_string());
        mapper.insert("PANEL_APP_PORT_HTTP", port.to_string());
        mapper.insert("PANEL_APP_PORT_HTTPS", (port + 1).to_string());
        mapper.insert("HOST_IP", "0.0.0.0".into());
        mapper.insert("APP_PATH", install_path.to_string_lossy().into_owned());
        // 内置应用模板使用 `{data_dir}`（→ DATA_DIR）。此前从未注入，渲染结果残留
        // `- {data_dir}/html:…`（YAML 流式映射）——compose 无法解析、内置应用无法部署；
        // 结构化扫描（Doc/33 B-1）解析失败即阻断，由此暴露。
        let data_dir = install_path.join("data");
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::internal(format!("创建数据目录失败: {}", e)))?;
        if mapper.get("DATA_DIR").is_none() {
            mapper.insert("DATA_DIR", data_dir.to_string_lossy().into_owned());
        }
        // M-M1（Doc/20 §4.6 ③）：CasaOS 同构系统变量注入——数据统一落盘
        // data/apps/<key>/，权限开箱即对，规避容器 root 运行。用户表单值
        // （req.values）优先级更高，仅在其未显式提供时注入默认值。
        let defaults: [(&str, String); 4] = [
            ("PUID", "1000".into()),
            ("PGID", "1000".into()),
            ("TZ", Local::now().format("%Z").to_string()),
            ("APP_ID", req.package_key.clone()),
        ];
        for (key, value) in defaults {
            if mapper.get(key).is_none() {
                mapper.insert(key, value);
            }
        }
        let (rendered, warnings) = mapper.replace(&compose);
        let rendered = self.security_scanner.ensure_restart_policy(&rendered);
        if !warnings.is_empty() {
            tracing::warn!("app {} compose 变量警告: {:?}", req.package_key, warnings);
        }

        // 安全扫描（M-A Issue 1.3：镜像仓库白名单支持 settings 追加配置）
        let extra_registries = {
            let default = crate::application::app_store_service::DEFAULT_TRUSTED_REGISTRIES;
            let all = self.trusted_registries().await;
            all.into_iter()
                .filter(|r| !default.contains(&r.as_str()))
                .collect::<Vec<String>>()
        };
        let scan = self.security_scanner.scan_compose_with_registries(
            &rendered,
            req.confirm_risky,
            &extra_registries,
        );
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
        // 三方合并基线：升级时据此区分用户对 compose 的手工修改
        super::lifecycle::save_compose_base(&install_path, &rendered);

        // 部署
        let deploy_result = self
            .docker_service
            .compose_deploy_at(&container_name, &rendered, &install_path)
            .await;
        if let Err(e) = deploy_result {
            let _ = std::fs::remove_dir_all(&install_path);
            return Err(e);
        }

        // Doc/20 §4.3：post_install 钩子（compose up 成功后；失败回滚 compose down）
        if let Err(e) = self
            .run_hook(
                &req.package_key,
                pkg_root.as_deref(),
                &version_info.hooks,
                HookStage::PostInstall,
                None,
            )
            .await
        {
            tracing::warn!("post_install hook failed, rolling back compose: {}", e);
            let _ = self.docker_service.compose_down(&container_name).await;
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

    pub(super) async fn install_native(
        &self,
        req: &InstallRequest,
        version: &str,
    ) -> Result<InstalledApp, AppError> {
        let version_info = self.get_version(&req.package_key, version).await?;

        // Doc/16 §6.1：按 package_key 路由到注册的 NativeInstaller（mysql/redis/web 引擎等），
        // 第三方原生软件注册自定义安装器即可接入，无需修改本函数。
        if let Some(installer) = self.native_installers.find(&req.package_key) {
            let mut app = installer.install(req, version).await?;
            let now = Utc::now();
            app.created_at = now;
            app.updated_at = now;
            let id = self.installed_repo.create(&app).await?;
            return self
                .installed_repo
                .find_by_id(id)
                .await?
                .ok_or_else(|| AppError::internal("create installed app"));
        }

        // 未注册安装器的通用原生脚本安装（Flame 格式 install.sh）
        if !version_info.native_scripts.is_empty() {
            // T3：带原生脚本的应用安装，须用户显式 acknowledge_scripts=true 确认执行第三方脚本。
            if !req.acknowledge_scripts {
                return Err(AppError::BadRequest(format!(
                    "应用 {} 包含 {} 行安装脚本，必须显式 acknowledge_scripts=true 才能继续",
                    req.package_key,
                    version_info.native_scripts.len()
                )));
            }
            // 计算脚本 SHA-256 摘要，供审计（前端在确认前展示摘要）。
            let joined = version_info.native_scripts.join("\n");
            let script_sha256 = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(joined.as_bytes());
                format!("{:x}", hasher.finalize())
            };
            for line in &version_info.native_scripts {
                // Phase A1 收尾：经统一特权命令端口执行安装脚本，杜绝面板直接 spawn `sh -c`
                let output = self
                    .runner
                    .run(&crate::application::execution_mode::PrivilegedCommand::new(
                        "sh",
                        vec!["-c".into(), line.clone()],
                    ))
                    .await?;
                if !output.success() {
                    return Err(AppError::internal(format!(
                        "安装脚本执行失败: {} (line: {})",
                        output.stderr, line
                    )));
                }
            }
            // 安装动作 + 脚本哈希写入审计事件（配合中间件写操作审计落库）。
            tracing::info!(
                "native install script executed: app={} version={} scripts={} sha256={}",
                req.package_key,
                version,
                version_info.native_scripts.len(),
                script_sha256
            );
        }

        // 通用原生安装：按包名走 PackageManagerPort
        self.package_manager.install(&req.package_key).await?;
        let app = InstalledApp {
            id: 0,
            package_key: req.package_key.clone(),
            name: req.name.clone().unwrap_or_else(|| req.package_key.clone()),
            version: version.into(),
            mode: InstallMode::Native.as_str().into(),
            status: "running".into(),
            access_url: req.port.map(|p| format!("http://localhost:{}", p)),
            install_path: format!("/usr/{}", req.package_key),
            container_name: None,
            port: req.port,
            params_json: serde_json::to_string(&req.values).unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            launch_count: 0,
        };
        let id = self.installed_repo.create(&app).await?;
        self.installed_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("create installed app"))
    }

    pub(super) async fn install_wasm(
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
            capabilities: vec![],
            settings: Default::default(),
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
}
