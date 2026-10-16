use super::engine::WebServerEngine;
use super::host::{EngineHost, SharedEngineHost};
use super::layout::{apply_hook, ols_register, ols_unregister, Hook, SiteLayout};
use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::domain::entity::WebServerInstance;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

#[derive(Clone)]
pub struct WebServerManager {
    runner: SharedCommandRunner,
    host: SharedEngineHost,
}

impl WebServerManager {
    /// 便捷构造：使用本地嵌入式执行（行为与重构前一致，测试/默认路径零破坏）。
    pub fn new() -> Self {
        Self::new_with_runner(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 注入统一特权命令执行端口（`execution_mode=embedded|agent` 分离模式）。
    ///
    /// 文件宿主与引擎命令按进程形态自动选择（Doc/32 W3-b）：非 root 且本机特权助手在线时，
    /// 配置读写与白名单内的引擎命令经 `flamepanel helper` 执行，其余命令仍走 `runner`；
    /// root / Docker / `OP_PRIV_HELPER=off` 时行为与此前一致（本机直写）。
    pub fn new_with_runner(runner: SharedCommandRunner) -> Self {
        let (runner, host) = super::helper::engine_access(runner);
        Self { runner, host }
    }

    /// 替换文件宿主（测试用临时 root；特权助手模式下接入 helper 实现）。
    pub fn with_host(mut self, host: SharedEngineHost) -> Self {
        self.host = host;
        self
    }

    pub async fn check_status(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        // Apache 的服务进程名随发行版不同（apache2 / httpd），pgrep 前先识别发行版
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;

        // Caddy 服务进程名可能是 `caddy` 或 `caddy run`（官方包进程树不同），
        // `pgrep -x caddy` 探测不可靠；用 `caddy version` 判断安装存在性，
        // 运行状态经 `caddy validate --config` 只能验证配置，无法判断运行，
        // 因此 Caddy 的运行探测走 systemd（由 PackageManager 安装后 enable/start 管理）。
        if matches!(engine, WebServerEngine::Caddy) {
            let output = self
                .runner
                .run(&PrivilegedCommand::new(
                    "systemctl",
                    vec!["is-active".into(), engine.service_name().into()],
                ))
                .await?;
            return if output.success() {
                Ok("running".into())
            } else {
                Ok("stopped".into())
            };
        }

        let output = self
            .runner
            .run(&PrivilegedCommand::new(
                "pgrep",
                vec!["-x".into(), engine.process_name_for(&distro).into()],
            ))
            .await?;

        if output.success() {
            Ok("running".into())
        } else {
            Ok("stopped".into())
        }
    }

    pub async fn start(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        // config_path 会作为启动参数交给特权执行器：存量行同样须位于引擎配置根下
        crate::webserver::helper::policy::check_path(&instance.config_path)?;
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;

        // OpenLiteSpeed 走官方 lswsctrl 控制脚本（服务树由 lswsctrl 管理，
        // 直接 `lshttpd -c conf` 启动只拉起单进程，行为不等价）。
        if let Some(script) = engine.control_script() {
            let output = self
                .runner
                .run(&PrivilegedCommand::new(script, vec!["start".into()]).prefer_root())
                .await?;
            return if output.success() {
                Ok(format!("{} started successfully", engine.as_str()))
            } else {
                Err(AppError::internal(format!(
                    "Failed to start {}: {}",
                    engine.as_str(),
                    output.stderr
                )))
            };
        }

        // Caddy 用 `caddy start --config` 后台启动（`caddy -c` 参数不存在，
        // 直接 `caddy run` 会前台阻塞）。启动后进程脱离当前命令，立即返回。
        if matches!(engine, WebServerEngine::Caddy) {
            let output = self
                .runner
                .run(&PrivilegedCommand::new(
                    "caddy",
                    vec![
                        "start".into(),
                        "--config".into(),
                        instance.config_path.clone(),
                    ],
                ))
                .await?;
            return if output.success() {
                Ok(format!("{} started successfully", engine.as_str()))
            } else {
                Err(AppError::internal(format!(
                    "Failed to start {}: {}",
                    engine.as_str(),
                    output.stderr
                )))
            };
        }

        // 实例未登记二进制路径时按发行版取默认值（Apache: /usr/sbin/apache2 vs httpd）。
        // 登记值不是该引擎的已知二进制（早于校验写入的存量行）时同样回落默认值，绝不执行。
        let binary = match instance.binary_path.as_deref() {
            Some(p) if !p.is_empty() && engine.binary_allowed(p) => p.to_string(),
            Some(p) if !p.is_empty() => {
                tracing::warn!(
                    binary = p,
                    engine = engine.as_str(),
                    "refusing unknown binary_path; using engine default"
                );
                engine.default_binary_path_for(&distro).to_string()
            }
            _ => engine.default_binary_path_for(&distro).to_string(),
        };
        let output = self
            .runner
            .run(&PrivilegedCommand::new(
                binary,
                vec!["-c".into(), instance.config_path.clone()],
            ))
            .await?;

        if output.success() {
            Ok(format!("{} started successfully", engine.as_str()))
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to start {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn stop(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        // config_path 会作为启动参数交给特权执行器：存量行同样须位于引擎配置根下
        crate::webserver::helper::policy::check_path(&instance.config_path)?;
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;

        // Caddy 优先 `caddy stop --config`（优雅停止并释放管理 API），
        // 失败时回退 killall（进程名 caddy）。
        if matches!(engine, WebServerEngine::Caddy) {
            let output = self
                .runner
                .run(&PrivilegedCommand::new(
                    "caddy",
                    vec![
                        "stop".into(),
                        "--config".into(),
                        instance.config_path.clone(),
                    ],
                ))
                .await?;
            if output.success() {
                return Ok(format!("{} stopped successfully", engine.as_str()));
            }
            tracing::warn!(
                "caddy stop failed, falling back to killall: {}",
                output.stderr
            );
            let output = self
                .runner
                .run(&PrivilegedCommand::new("killall", vec!["caddy".into()]))
                .await?;
            return if output.success() {
                Ok(format!("{} stopped successfully", engine.as_str()))
            } else {
                Err(AppError::internal(format!(
                    "Failed to stop {}: {}",
                    engine.as_str(),
                    output.stderr
                )))
            };
        }

        // OpenLiteSpeed 优先 lswsctrl stop（优雅停止全部服务进程），
        // 失败时回退 killall。
        if let Some(script) = engine.control_script() {
            let output = self
                .runner
                .run(&PrivilegedCommand::new(script, vec!["stop".into()]).prefer_root())
                .await?;
            if output.success() {
                return Ok(format!("{} stopped successfully", engine.as_str()));
            }
            tracing::warn!(
                "lswsctrl stop failed, falling back to killall: {}",
                output.stderr
            );
        }

        // killall 需用真实进程名（Apache: apache2 vs httpd，随发行版分叉）
        let output = self
            .runner
            .run(&PrivilegedCommand::new(
                "killall",
                vec![engine.process_name_for(&distro).into()],
            ))
            .await?;

        if output.success() {
            Ok(format!("{} stopped successfully", engine.as_str()))
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to stop {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn restart(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        self.stop(instance).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start(instance).await
    }

    pub async fn reload(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;

        let cmd = Self::parse_command(engine.reload_command_for(&distro));
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1).prefer_root())
            .await?;

        if output.success() {
            Ok(format!("{} reloaded successfully", engine.as_str()))
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to reload {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }

    pub async fn config_test(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;

        let cmd = Self::parse_command(engine.config_test_command_for(&distro));
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1).prefer_root())
            .await?;

        let success = output.success();
        let stdout = output.stdout;
        let stderr = output.stderr;

        if success {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            Err(AppError::internal(format!(
                "Config test failed: {}",
                stderr
            )))
        }
    }

    /// 把引擎返回的 shell 命令行（如 `nginx -s reload`）解析为 program + args，
    /// 供 `PrivilegedCommandRunner` 直接执行（避免任意 shell 拼接注入）。
    fn parse_command(command_line: &str) -> (String, Vec<String>) {
        let mut parts = command_line.split_whitespace();
        let program = parts.next().unwrap_or("").to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        (program, args)
    }

    fn not_installed(engine: &WebServerEngine, main: &str) -> AppError {
        AppError::BadRequest(format!(
            "{} is not installed on this host (main config {} not found)",
            engine.as_str(),
            main
        ))
    }

    /// 当前发行版下的站点布局。
    pub async fn layout(&self, engine: &WebServerEngine) -> SiteLayout {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        SiteLayout::for_engine(engine.clone(), &distro)
    }

    /// 引擎是否已安装（以主配置文件存在为准）。
    pub async fn engine_installed(&self, engine: &WebServerEngine) -> bool {
        let layout = self.layout(engine).await;
        self.host.exists(&layout.main_config).await
    }

    /// 能否部署：引擎已安装且站点目录可写（「尽力部署」路径据此决定是否跳过）。
    pub async fn can_deploy(&self, engine: &WebServerEngine) -> bool {
        let layout = self.layout(engine).await;
        self.host.exists(&layout.main_config).await && self.host.writable(&layout.sites_dir).await
    }

    /// Phase A2：原子写配置 + 校验 + reload + 失败回滚（单文件，如全局配置）。
    ///
    /// 与站点流水线共用 [`ConfigTxn`] 与引擎级互斥锁：同一引擎的「写 → 校验 → reload」
    /// 串行执行，避免并发请求交错导致回滚到错误快照。
    pub async fn write_config_file_atomic(
        &self,
        engine: &WebServerEngine,
        path: &str,
        content: &str,
        do_reload: bool,
    ) -> Result<(), AppError> {
        let lock = engine_lock(engine);
        let _guard = lock.lock().await;
        let mut txn = ConfigTxn::new(self.host.as_ref());
        txn.write(path, content).await?;
        if let Err(e) = self.atomic_config_test(engine).await {
            txn.rollback().await;
            return Err(e);
        }
        if do_reload {
            if let Err(e) = self.atomic_reload(engine).await {
                txn.rollback().await;
                return Err(e);
            }
        }
        Ok(())
    }

    /// 部署（新建或覆盖）站点配置并使其生效。
    ///
    /// 事务内依次：清理旧版本位置 → 主配置钩子（幂等）→ OLS 注册 → 写站点文件 →
    /// 配置校验 → reload。任一步失败整体回滚。`ols_root` 仅 OpenLiteSpeed 需要（vhRoot）。
    pub async fn deploy_site(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        content: &str,
        ols_root: Option<&str>,
    ) -> Result<SiteDeployment, AppError> {
        let lock = engine_lock(engine);
        let _guard = lock.lock().await;
        let layout = self.layout(engine).await;
        let main = self
            .host
            .read(&layout.main_config)
            .await?
            .ok_or_else(|| Self::not_installed(engine, &layout.main_config))?;

        let mut new_main = match apply_hook(&main, &layout.hook) {
            Ok(Some(m)) => m,
            Ok(None) => main.clone(),
            Err(e) => return Err(AppError::BadRequest(format!("{}: {e}", layout.main_config))),
        };
        if matches!(layout.hook, Hook::OlsRegistry) {
            let root = ols_root.filter(|r| !r.is_empty()).ok_or_else(|| {
                AppError::BadRequest("OpenLiteSpeed deployment requires the site root path".into())
            })?;
            new_main = ols_register(&new_main, domain, root)
                .map_err(|e| AppError::Conflict(format!("{}: {e}", layout.main_config)))?;
        }

        let mut txn = ConfigTxn::new(self.host.as_ref());
        // 旧位置先删（OLS 旧文件 vhosts/<domain> 与新目录同名）
        for legacy in layout.legacy_paths(domain) {
            txn.remove(&legacy).await?;
        }
        if new_main != main {
            txn.write(&layout.main_config, &new_main).await?;
        }
        txn.remove(&layout.disabled_path(domain)).await?;
        let site_path = layout.site_path(domain);
        if matches!(layout.hook, Hook::OlsRegistry) {
            // 重新渲染的 vhconf 不含 WAF 段：按已下发的规则库补回（否则改站点会静默丢掉 WAF）
            let content = self.ols_with_waf(&layout, domain, content).await;
            txn.write(&site_path, &content).await?;
        } else {
            txn.write(&site_path, content).await?;
        }

        let reloaded = self.commit(engine, txn).await?;
        Ok(SiteDeployment {
            engine: engine.as_str().to_string(),
            path: site_path,
            enabled: true,
            reloaded,
        })
    }

    /// 下发 WAF 编译产物（五引擎，见 [`super::waf_engines`]）：主配置钩子（Nginx `http {`
    /// include / Apache `IncludeOptional`，幂等）→ 写主配置级 + 站点级文件 → OLS 逐站点把规则
    /// 拼接进 `vhconf.conf` → 校验 → reload；任一步失败整体回滚。
    ///
    /// 文件总是写出（即使没有规则也写出空汇总），保证站点里的 include 引用始终有定义。
    pub async fn deploy_waf(
        &self,
        engine: &WebServerEngine,
        compiled: &super::waf_nginx::CompiledWaf,
    ) -> Result<bool, AppError> {
        use super::{waf_engines, waf_ols};
        if !waf_engines::supports(engine) {
            return Err(AppError::BadRequest(format!(
                "WAF enforcement is not supported for {}",
                engine.as_str()
            )));
        }
        let lock = engine_lock(engine);
        let _guard = lock.lock().await;
        let layout = self.layout(engine).await;
        let main = self
            .host
            .read(&layout.main_config)
            .await?
            .ok_or_else(|| Self::not_installed(engine, &layout.main_config))?;
        let new_main = match waf_engines::http_hook(&layout) {
            None => None,
            Some(hook) => match apply_hook(&main, &hook) {
                Ok(m) => m,
                Err(e) => return Err(AppError::BadRequest(format!("{}: {e}", layout.main_config))),
            },
        };
        let mut txn = ConfigTxn::new(self.host.as_ref());
        if let Some(path) = waf_engines::http_conf_path(&layout) {
            txn.write(&path, &compiled.http_conf).await?;
        }
        txn.write(
            &waf_engines::server_conf_path(&layout),
            &compiled.server_conf,
        )
        .await?;
        if let Some(m) = new_main {
            txn.write(&layout.main_config, &m).await?;
        }
        if *engine == WebServerEngine::OpenLiteSpeed {
            // OLS vhost 不支持 include：把各站点的规则段拼接进 vhconf（启用 / 停用两种形态）
            for domain in waf_ols::site_domains(&compiled.server_conf) {
                let section = waf_ols::section_for(&compiled.server_conf, &domain);
                for path in [layout.site_path(&domain), layout.disabled_path(&domain)] {
                    if let Some(current) = self.host.read(&path).await? {
                        let next = waf_ols::splice(&current, &section);
                        if next != current {
                            txn.write(&path, &next).await?;
                        }
                    }
                }
            }
        }
        self.commit(engine, txn).await
    }

    /// OLS：把已下发的 WAF 规则段拼接进待部署的 vhconf（规则库缺失时原样返回）。
    async fn ols_with_waf(&self, layout: &SiteLayout, domain: &str, content: &str) -> String {
        use super::{waf_engines, waf_ols};
        match self.host.read(&waf_engines::server_conf_path(layout)).await {
            Ok(Some(library)) => waf_ols::splice(content, &waf_ols::section_for(&library, domain)),
            _ => content.to_string(),
        }
    }

    /// 增量读取引擎的 WAF 命中日志（语义见 [`EngineHost::read_log_chunk`]；非 root 经特权助手）。
    /// 引擎没有命中日志（OLS）时返回 `Ok(None)`。
    pub async fn read_waf_log(
        &self,
        engine: &WebServerEngine,
        offset: u64,
        inode: Option<u64>,
        max: usize,
    ) -> Result<Option<super::host::LogChunk>, AppError> {
        let layout = self.layout(engine).await;
        match super::waf_engines::log_path(&layout) {
            Some(path) => self.host.read_log_chunk(path, offset, inode, max).await,
            None => Ok(None),
        }
    }

    /// 站点文件是否已含 WAF 站点级 include（旧版本渲染的站点需要重新部署才受 WAF 保护）。
    /// OLS 由 [`Self::deploy_waf`] 直接拼接规则，返回 `None`（无需重部署）。
    pub async fn site_has_waf_include(
        &self,
        engine: &WebServerEngine,
        domain: &str,
    ) -> Option<bool> {
        let layout = self.layout(engine).await;
        let directive = super::waf_engines::site_include_directive(&layout)?;
        let content = self.host.read(&layout.site_path(domain)).await.ok()??;
        Some(content.contains(&directive))
    }

    /// 移除站点配置（启用 / 停用 / 旧位置 / OLS 注册全部清理）。未部署时为 no-op。
    pub async fn remove_site(
        &self,
        engine: &WebServerEngine,
        domain: &str,
    ) -> Result<(), AppError> {
        let lock = engine_lock(engine);
        let _guard = lock.lock().await;
        let layout = self.layout(engine).await;
        let main = self.host.read(&layout.main_config).await?;

        let mut txn = ConfigTxn::new(self.host.as_ref());
        txn.remove(&layout.site_path(domain)).await?;
        txn.remove(&layout.disabled_path(domain)).await?;
        for legacy in layout.legacy_paths(domain) {
            txn.remove(&legacy).await?;
        }
        if let (Hook::OlsRegistry, Some(main)) = (&layout.hook, main.as_ref()) {
            let cleaned = ols_unregister(main, domain);
            if &cleaned != main {
                txn.write(&layout.main_config, &cleaned).await?;
            }
        }
        if !txn.changed() {
            return Ok(());
        }
        // 引擎已卸载：文件清理即可，不做校验 / reload
        if main.is_none() {
            return Ok(());
        }
        self.commit(engine, txn).await.map(|_| ())
    }

    /// 启用 / 停用已部署的站点（停用 = 重命名为 `.disabled`，OLS 同时注销映射）。
    ///
    /// 已处于目标状态时为 no-op；启用一个从未部署的站点返回 NotFound。
    pub async fn set_site_enabled(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        enabled: bool,
        ols_root: Option<&str>,
    ) -> Result<(), AppError> {
        let lock = engine_lock(engine);
        let _guard = lock.lock().await;
        let layout = self.layout(engine).await;
        let main = self
            .host
            .read(&layout.main_config)
            .await?
            .ok_or_else(|| Self::not_installed(engine, &layout.main_config))?;
        let site_path = layout.site_path(domain);
        let disabled_path = layout.disabled_path(domain);
        let (from, to) = if enabled {
            (&disabled_path, &site_path)
        } else {
            (&site_path, &disabled_path)
        };

        if self.host.exists(to).await && !self.host.exists(from).await {
            return Ok(());
        }
        let Some(content) = self.host.read(from).await? else {
            return if enabled {
                Err(AppError::NotFound(format!(
                    "site config for {domain} has not been deployed to {}",
                    engine.as_str()
                )))
            } else {
                Ok(())
            };
        };

        let mut txn = ConfigTxn::new(self.host.as_ref());
        txn.write(to, &content).await?;
        txn.remove(from).await?;
        if matches!(layout.hook, Hook::OlsRegistry) {
            let new_main = if enabled {
                let root = ols_root.filter(|r| !r.is_empty()).ok_or_else(|| {
                    AppError::BadRequest(
                        "OpenLiteSpeed deployment requires the site root path".into(),
                    )
                })?;
                ols_register(&main, domain, root)
                    .map_err(|e| AppError::Conflict(format!("{}: {e}", layout.main_config)))?
            } else {
                ols_unregister(&main, domain)
            };
            if new_main != main {
                txn.write(&layout.main_config, &new_main).await?;
            }
        }
        self.commit(engine, txn).await.map(|_| ())
    }

    /// 校验 + reload；失败回滚。返回是否已 reload。
    ///
    /// reload 失败但引擎当前未运行（如刚安装未启动）时保留已通过校验的配置，
    /// 返回 `Ok(false)`——下次启动即生效，不应因「无进程可 reload」回滚合法配置。
    async fn commit(&self, engine: &WebServerEngine, txn: ConfigTxn<'_>) -> Result<bool, AppError> {
        if let Err(e) = self.atomic_config_test(engine).await {
            txn.rollback().await;
            return Err(e);
        }
        match self.atomic_reload(engine).await {
            Ok(()) => Ok(true),
            Err(e) => {
                if self.engine_running(engine).await {
                    txn.rollback().await;
                    Err(e)
                } else {
                    tracing::info!(
                        engine = engine.as_str(),
                        "engine not running; config validated and kept without reload"
                    );
                    Ok(false)
                }
            }
        }
    }

    /// 引擎进程是否在运行（探测失败按「运行中」处理，走保守的回滚路径）。
    async fn engine_running(&self, engine: &WebServerEngine) -> bool {
        let cmd = if matches!(engine, WebServerEngine::Caddy) {
            PrivilegedCommand::new(
                "systemctl",
                vec!["is-active".into(), engine.service_name().into()],
            )
        } else {
            let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
            PrivilegedCommand::new(
                "pgrep",
                vec!["-x".into(), engine.process_name_for(&distro).into()],
            )
        };
        match self.runner.run(&cmd).await {
            Ok(out) => out.success(),
            Err(_) => true,
        }
    }

    /// 基于引擎执行配置校验命令（`nginx -t` 类），供原子写流程使用。
    async fn atomic_config_test(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let cmd = Self::parse_command(engine.config_test_command_for(&distro));
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1))
            .await?;
        if output.success() {
            Ok(())
        } else {
            let detail = if output.stderr.trim().is_empty() {
                output.stdout
            } else {
                output.stderr
            };
            Err(AppError::BadRequest(format!(
                "{} config test failed: {}",
                engine.as_str(),
                detail.trim()
            )))
        }
    }

    /// 基于引擎执行 reload 命令，供原子写流程使用。
    async fn atomic_reload(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let cmd = Self::parse_command(engine.reload_command_for(&distro));
        let output = self
            .runner
            .run(&PrivilegedCommand::new(cmd.0, cmd.1).prefer_root())
            .await?;
        if output.success() {
            Ok(())
        } else {
            let stderr = output.stderr;
            Err(AppError::internal(format!(
                "Failed to reload {}: {}",
                engine.as_str(),
                stderr
            )))
        }
    }
}

/// 站点部署结果（`POST /api/websites/{id}/deploy` 出参）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct SiteDeployment {
    pub engine: String,
    /// 站点配置文件（逻辑路径）。
    pub path: String,
    pub enabled: bool,
    /// 是否已 reload（引擎未运行时为 false，下次启动生效）。
    pub reloaded: bool,
}

type EngineLocks = std::sync::Mutex<HashMap<&'static str, Arc<tokio::sync::Mutex<()>>>>;

/// 进程级引擎互斥锁：同一引擎的配置事务串行。
fn engine_lock(engine: &WebServerEngine) -> Arc<tokio::sync::Mutex<()>> {
    static LOCKS: OnceLock<EngineLocks> = OnceLock::new();
    let map = LOCKS.get_or_init(Default::default);
    let mut guard = map.lock().unwrap_or_else(|p| p.into_inner());
    guard.entry(engine.as_str()).or_default().clone()
}

/// 多文件配置事务：记录每个路径首次改动前的内容，失败时逆序恢复。
struct ConfigTxn<'a> {
    host: &'a dyn EngineHost,
    undo: Vec<(String, Option<String>)>,
}

impl<'a> ConfigTxn<'a> {
    fn new(host: &'a dyn EngineHost) -> Self {
        Self {
            host,
            undo: Vec::new(),
        }
    }

    fn changed(&self) -> bool {
        !self.undo.is_empty()
    }

    async fn snapshot(&mut self, path: &str) -> Result<(), AppError> {
        if !self.undo.iter().any(|(p, _)| p == path) {
            let before = self.host.read(path).await?;
            self.undo.push((path.to_string(), before));
        }
        Ok(())
    }

    async fn write(&mut self, path: &str, content: &str) -> Result<(), AppError> {
        self.snapshot(path).await?;
        self.host.write_atomic(path, content).await
    }

    async fn remove(&mut self, path: &str) -> Result<bool, AppError> {
        if !self.host.exists(path).await {
            return Ok(false);
        }
        self.snapshot(path).await?;
        self.host.remove(path).await
    }

    /// 逆序恢复；单项失败记日志并继续（尽量多地恢复）。
    async fn rollback(self) {
        for (path, before) in self.undo.into_iter().rev() {
            let res = match before {
                Some(content) => self.host.write_atomic(&path, &content).await,
                None => self.host.remove(&path).await.map(|_| ()),
            };
            if let Err(e) = res {
                tracing::error!(path = %path, error = %e, "config rollback failed; manual check required");
            }
        }
    }
}

impl Default for WebServerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Phase A2 原子写 + 回滚单元测试 ─────────────────────────────
// 证书绑定复用既有配置事务；服务只依赖应用端口，可在测试中隔离文件和命令。
#[async_trait::async_trait]
impl crate::application::certificate_service::CertificateSiteConfigWriter for WebServerManager {
    async fn write_site_config(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        content: &str,
    ) -> Result<(), AppError> {
        // OLS / Caddy 的显式证书在证书服务侧已拒绝，这里不需要 vhRoot
        self.deploy_site(engine, domain, content, None)
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::host::LocalEngineHost;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fp-atomic-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_command_splits_program_and_args() {
        let (program, args) = WebServerManager::parse_command("nginx -s reload");
        assert_eq!(program, "nginx");
        assert_eq!(args, vec!["-s", "reload"]);
    }

    #[test]
    fn parse_command_handles_binary_with_path() {
        let (program, args) =
            WebServerManager::parse_command("/usr/local/lsws/bin/lswsctrl reload");
        assert_eq!(program, "/usr/local/lsws/bin/lswsctrl");
        assert_eq!(args, vec!["reload"]);
    }

    #[test]
    fn parse_command_handles_single_token() {
        let (program, args) = WebServerManager::parse_command("nginx");
        assert_eq!(program, "nginx");
        assert!(args.is_empty());
    }

    // ── 站点配置流水线（tmp root + 脚本化 runner）──────────────────────

    /// 按程序 / 参数返回预设结果的 runner。
    #[derive(Default)]
    struct ScriptedRunner {
        fail_test: std::sync::atomic::AtomicBool,
        fail_reload: std::sync::atomic::AtomicBool,
        running: std::sync::atomic::AtomicBool,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl ScriptedRunner {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::application::execution_mode::PrivilegedCommandRunner for ScriptedRunner {
        async fn run(
            &self,
            cmd: &crate::application::execution_mode::PrivilegedCommand,
        ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
            use std::sync::atomic::Ordering::SeqCst;
            let line = format!("{} {}", cmd.program, cmd.args.join(" "));
            self.calls.lock().unwrap().push(line.clone());
            let ok = if line.contains("-t")
                || line.contains("configtest")
                || line.contains("validate")
            {
                !self.fail_test.load(SeqCst)
            } else if line.contains("reload") {
                !self.fail_reload.load(SeqCst)
            } else if cmd.program == "pgrep" || cmd.program == "systemctl" {
                self.running.load(SeqCst)
            } else {
                true
            };
            Ok(crate::application::execution_mode::CommandOutput {
                stdout: String::new(),
                stderr: if ok {
                    String::new()
                } else {
                    "boom: line 3".into()
                },
                exit_code: if ok { 0 } else { 1 },
            })
        }
    }

    fn setup(tag: &str) -> (std::path::PathBuf, Arc<ScriptedRunner>, WebServerManager) {
        let root = tmp_dir(tag);
        let runner = Arc::new(ScriptedRunner::default());
        runner
            .running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mgr = WebServerManager::new_with_runner(runner.clone())
            .with_host(Arc::new(LocalEngineHost::new(&root)));
        (root, runner, mgr)
    }

    fn put(root: &std::path::Path, logical: &str, content: &str) {
        let p = root.join(logical.trim_start_matches('/'));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    fn get(root: &std::path::Path, logical: &str) -> Option<String> {
        std::fs::read_to_string(root.join(logical.trim_start_matches('/'))).ok()
    }

    const NGINX_MAIN: &str = "events {}\nhttp {\n    include /etc/nginx/conf.d/*.conf;\n}\n";
    const SITE_PATH: &str = "/etc/nginx/flamepanel/sites/a.test.conf";

    #[tokio::test]
    async fn deploy_writes_site_hooks_main_and_cleans_legacy() {
        let (root, runner, mgr) = setup("deploy");
        put(&root, "/etc/nginx/nginx.conf", NGINX_MAIN);
        put(&root, "/etc/nginx/sites-available/a.test", "old");
        let d = mgr
            .deploy_site(&WebServerEngine::Nginx, "a.test", "server { }", None)
            .await
            .unwrap();
        assert_eq!(d.path, SITE_PATH);
        assert!(d.reloaded && d.enabled);
        assert_eq!(get(&root, SITE_PATH).as_deref(), Some("server { }"));
        let main = get(&root, "/etc/nginx/nginx.conf").unwrap();
        assert!(main.contains("include /etc/nginx/flamepanel/sites/*.conf;"));
        assert!(get(&root, "/etc/nginx/sites-available/a.test").is_none());
        let calls = runner.calls();
        assert!(calls.iter().any(|c| c == "nginx -t"));
        assert!(calls.iter().any(|c| c == "nginx -s reload"));

        // 二次部署：主配置不再改动（钩子幂等）
        mgr.deploy_site(&WebServerEngine::Nginx, "a.test", "server { v2 }", None)
            .await
            .unwrap();
        let main2 = get(&root, "/etc/nginx/nginx.conf").unwrap();
        assert_eq!(main, main2);
        assert_eq!(main2.matches("flamepanel/sites").count(), 1);
    }

    #[tokio::test]
    async fn deploy_rolls_back_every_file_when_config_test_fails() {
        let (root, runner, mgr) = setup("rollback");
        put(&root, "/etc/nginx/nginx.conf", NGINX_MAIN);
        put(&root, "/etc/nginx/sites-enabled/a.test", "legacy-live");
        runner
            .fail_test
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let err = mgr
            .deploy_site(&WebServerEngine::Nginx, "a.test", "bad", None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::BadRequest(ref m) if m.contains("boom")),
            "{err:?}"
        );
        assert_eq!(
            get(&root, "/etc/nginx/nginx.conf").as_deref(),
            Some(NGINX_MAIN)
        );
        assert!(get(&root, SITE_PATH).is_none());
        assert_eq!(
            get(&root, "/etc/nginx/sites-enabled/a.test").as_deref(),
            Some("legacy-live")
        );
        assert!(!runner.calls().iter().any(|c| c.contains("reload")));
    }

    #[tokio::test]
    async fn reload_failure_keeps_config_when_engine_stopped_but_rolls_back_when_running() {
        use std::sync::atomic::Ordering::SeqCst;
        let (root, runner, mgr) = setup("reload");
        put(&root, "/etc/nginx/nginx.conf", NGINX_MAIN);
        runner.fail_reload.store(true, SeqCst);
        runner.running.store(false, SeqCst);
        let d = mgr
            .deploy_site(&WebServerEngine::Nginx, "a.test", "s1", None)
            .await
            .unwrap();
        assert!(!d.reloaded);
        assert_eq!(get(&root, SITE_PATH).as_deref(), Some("s1"));

        runner.running.store(true, SeqCst);
        assert!(mgr
            .deploy_site(&WebServerEngine::Nginx, "a.test", "s2", None)
            .await
            .is_err());
        assert_eq!(get(&root, SITE_PATH).as_deref(), Some("s1"), "回滚到上一版");
    }

    #[tokio::test]
    async fn deploy_waf_hooks_main_writes_both_files_and_rolls_back() {
        use crate::webserver::waf_nginx::{self, CompiledWaf};
        let (root, runner, mgr) = setup("waf");
        put(&root, "/etc/nginx/nginx.conf", NGINX_MAIN);
        let ngx = WebServerEngine::Nginx;
        let c1 = CompiledWaf {
            http_conf: "h1".into(),
            server_conf: "s1".into(),
            enforced: vec![],
            skipped: vec![],
        };
        // 站点钩子在先，WAF 钩子按指令独立判定（不会因已有 flamepanel 标记而漏插）
        mgr.deploy_site(&ngx, "a.test", "server { }", None)
            .await
            .unwrap();
        assert!(mgr.deploy_waf(&ngx, &c1).await.unwrap());
        let main = get(&root, "/etc/nginx/nginx.conf").unwrap();
        assert!(main.contains(&waf_nginx::http_include_directive(&ngx)));
        assert!(main.contains("include /etc/nginx/flamepanel/sites/*.conf;"));
        assert_eq!(
            get(&root, &waf_nginx::http_conf_path(&ngx)).as_deref(),
            Some("h1")
        );
        assert_eq!(
            get(&root, &waf_nginx::server_conf_path(&ngx)).as_deref(),
            Some("s1")
        );
        assert!(runner.calls().iter().any(|c| c == "nginx -s reload"));

        // 幂等：再次下发主配置不变
        mgr.deploy_waf(&ngx, &c1).await.unwrap();
        assert_eq!(get(&root, "/etc/nginx/nginx.conf").unwrap(), main);

        // 校验失败：两个文件都回滚到上一版
        runner
            .fail_test
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let c2 = CompiledWaf {
            http_conf: "h2".into(),
            server_conf: "s2".into(),
            enforced: vec![],
            skipped: vec![],
        };
        assert!(mgr.deploy_waf(&ngx, &c2).await.is_err());
        assert_eq!(
            get(&root, &waf_nginx::http_conf_path(&ngx)).as_deref(),
            Some("h1")
        );
        assert_eq!(
            get(&root, &waf_nginx::server_conf_path(&ngx)).as_deref(),
            Some("s1")
        );

        // Caddy 已支持，但未安装（无主配置）→ 报错且不落盘
        assert!(mgr.deploy_waf(&WebServerEngine::Caddy, &c1).await.is_err());
        assert!(get(&root, "/etc/caddy/flamepanel/waf/site.caddy").is_none());
        // 引擎未安装：不落盘
        let (root2, _r, mgr2) = setup("waf-missing");
        assert!(mgr2.deploy_waf(&ngx, &c1).await.is_err());
        assert!(get(&root2, &waf_nginx::http_conf_path(&ngx)).is_none());
    }

    fn waf_compiled(http: &str, server: &str) -> crate::webserver::waf_nginx::CompiledWaf {
        crate::webserver::waf_nginx::CompiledWaf {
            http_conf: http.into(),
            server_conf: server.into(),
            enforced: vec![],
            skipped: vec![],
        }
    }

    /// Apache：主配置末尾追加 `IncludeOptional …/waf/http*.conf`（幂等），写 http + vhost 两个
    /// 文件，校验失败回滚；站点 include 检测按发行版布局。
    #[tokio::test]
    async fn deploy_waf_apache_appends_hook_and_rolls_back() {
        use crate::webserver::waf_engines;
        let (root, runner, mgr) = setup("waf-apache");
        let e = WebServerEngine::Apache;
        let layout = mgr.layout(&e).await;
        put(&root, &layout.main_config, "ServerRoot /etc/x\n");
        let http = waf_engines::http_conf_path(&layout).unwrap();
        let vhost = waf_engines::server_conf_path(&layout);

        assert!(mgr.deploy_waf(&e, &waf_compiled("h1", "v1")).await.unwrap());
        let main = get(&root, &layout.main_config).unwrap();
        let directive = format!("IncludeOptional {}/http*.conf", layout.waf_dir());
        assert_eq!(main.matches(&directive).count(), 1);
        assert_eq!(get(&root, &http).as_deref(), Some("h1"));
        assert_eq!(get(&root, &vhost).as_deref(), Some("v1"));
        mgr.deploy_waf(&e, &waf_compiled("h1", "v1")).await.unwrap();
        assert_eq!(get(&root, &layout.main_config).unwrap(), main);

        runner
            .fail_test
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(mgr.deploy_waf(&e, &waf_compiled("h2", "v2")).await.is_err());
        assert_eq!(get(&root, &http).as_deref(), Some("h1"));
        assert_eq!(get(&root, &vhost).as_deref(), Some("v1"));
        runner
            .fail_test
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // 站点 include 检测：旧站点缺 include → Some(false)；新渲染 → Some(true)
        put(
            &root,
            &layout.site_path("old.test"),
            "<VirtualHost *:80>\n</VirtualHost>\n",
        );
        assert_eq!(mgr.site_has_waf_include(&e, "old.test").await, Some(false));
        let inc = waf_engines::site_include_directive(&layout).unwrap();
        put(
            &root,
            &layout.site_path("new.test"),
            &format!("    {inc}\n"),
        );
        assert_eq!(mgr.site_has_waf_include(&e, "new.test").await, Some(true));
        assert_eq!(mgr.site_has_waf_include(&e, "none.test").await, None);
    }

    /// Caddy：无主配置钩子，只写站点级片段；命中日志路径可读。
    #[tokio::test]
    async fn deploy_waf_caddy_writes_site_snippet_only() {
        let (root, runner, mgr) = setup("waf-caddy");
        let e = WebServerEngine::Caddy;
        let main = "{\n}\nimport /etc/caddy/flamepanel/sites/*.caddy\n";
        put(&root, "/etc/caddy/Caddyfile", main);
        assert!(mgr
            .deploy_waf(&e, &waf_compiled("", "snippet"))
            .await
            .unwrap());
        assert_eq!(get(&root, "/etc/caddy/Caddyfile").as_deref(), Some(main));
        assert_eq!(
            get(&root, "/etc/caddy/flamepanel/waf/site.caddy").as_deref(),
            Some("snippet")
        );
        assert!(runner.calls().iter().any(|c| c.contains("caddy")));
        // 日志尚未产生 → None；写入后按块读取
        assert!(mgr.read_waf_log(&e, 0, None, 1024).await.unwrap().is_none());
        put(&root, "/var/log/caddy/flamepanel-waf.log", "{}\n");
        let chunk = mgr.read_waf_log(&e, 0, None, 1024).await.unwrap().unwrap();
        assert_eq!(chunk.data, "{}\n");
    }

    /// OLS：规则库落盘 + 逐站点拼接进 vhconf（启用 / 停用）；重新部署站点时从规则库补回；
    /// 校验失败整体回滚；无命中日志。
    #[tokio::test]
    async fn deploy_waf_ols_splices_vhconf_and_survives_redeploy() {
        use crate::webserver::{waf_engines, waf_ols};
        let (root, runner, mgr) = setup("waf-ols");
        let e = WebServerEngine::OpenLiteSpeed;
        put(
            &root,
            "/usr/local/lsws/conf/httpd_config.conf",
            "listener Default {\n  address *:8088\n  secure 0\n}\n",
        );
        mgr.deploy_site(&e, "a.test", "docRoot $VH_ROOT/\n", Some("/www/a"))
            .await
            .unwrap();
        mgr.deploy_site(&e, "b.test", "docRoot $VH_ROOT/\n", Some("/www/b"))
            .await
            .unwrap();
        mgr.set_site_enabled(&e, "b.test", false, Some("/www/b"))
            .await
            .unwrap();

        let rules = vec![
            crate::webserver::waf_engines::tests::rule(1, "path_block", "/admin", "block", 0),
            crate::webserver::waf_engines::tests::rule(2, "path_block", "/only-a", "block", 11),
        ];
        let sites = vec![waf_ols_site(11, "a.test"), waf_ols_site(12, "b.test")];
        let layout = mgr.layout(&e).await;
        let compiled = waf_engines::compile(&rules, &sites, &layout);
        mgr.deploy_waf(&e, &compiled).await.unwrap();
        assert!(get(&root, "/usr/local/lsws/conf/flamepanel/waf/vhost.rules").is_some());
        let a = get(&root, &layout.site_path("a.test")).unwrap();
        assert!(a.contains(waf_ols::BEGIN) && a.contains("\\x2fonly-a"));
        // 停用站点也拼接（重新启用即受保护），且只含全局规则
        let b = get(&root, &layout.disabled_path("b.test")).unwrap();
        assert!(b.contains("\\x2fadmin") && !b.contains("only-a"));

        // 站点重新渲染（改站点 / 套伪静态）：从规则库补回 WAF 段
        mgr.deploy_site(&e, "a.test", "docRoot $VH_ROOT/\n", Some("/www/a"))
            .await
            .unwrap();
        let a2 = get(&root, &layout.site_path("a.test")).unwrap();
        assert_eq!(a2, a);

        // 校验失败：规则库与 vhconf 一起回滚
        runner
            .fail_test
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let empty = waf_engines::compile(&[], &sites, &layout);
        assert!(mgr.deploy_waf(&e, &empty).await.is_err());
        assert_eq!(get(&root, &layout.site_path("a.test")).unwrap(), a);
        runner
            .fail_test
            .store(false, std::sync::atomic::Ordering::SeqCst);
        // 清空规则：WAF 段移除，vhconf 恢复原样
        mgr.deploy_waf(&e, &empty).await.unwrap();
        assert_eq!(
            get(&root, &layout.site_path("a.test")).as_deref(),
            Some("docRoot $VH_ROOT/\n")
        );
        assert_eq!(mgr.site_has_waf_include(&e, "a.test").await, None);
        assert!(mgr.read_waf_log(&e, 0, None, 1024).await.unwrap().is_none());
    }

    fn waf_ols_site(id: i64, d: &str) -> crate::webserver::waf_nginx::WafSite {
        crate::webserver::waf_nginx::WafSite {
            website_id: id,
            domains: vec![d.into()],
        }
    }

    #[tokio::test]
    async fn deploy_reports_not_installed_without_touching_disk() {
        let (root, _runner, mgr) = setup("missing");
        assert!(!mgr.engine_installed(&WebServerEngine::Nginx).await);
        let err = mgr
            .deploy_site(&WebServerEngine::Nginx, "a.test", "x", None)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(ref m) if m.contains("not installed")));
        assert!(get(&root, SITE_PATH).is_none());
    }

    #[tokio::test]
    async fn disable_enable_and_remove_cycle() {
        let (root, _runner, mgr) = setup("cycle");
        put(&root, "/etc/nginx/nginx.conf", NGINX_MAIN);
        let e = WebServerEngine::Nginx;
        // 未部署即启用 → NotFound；停用 → no-op
        assert!(matches!(
            mgr.set_site_enabled(&e, "a.test", true, None).await,
            Err(AppError::NotFound(_))
        ));
        mgr.set_site_enabled(&e, "a.test", false, None)
            .await
            .unwrap();

        mgr.deploy_site(&e, "a.test", "cfg", None).await.unwrap();
        mgr.set_site_enabled(&e, "a.test", false, None)
            .await
            .unwrap();
        assert!(get(&root, SITE_PATH).is_none());
        assert_eq!(
            get(&root, &format!("{SITE_PATH}.disabled")).as_deref(),
            Some("cfg")
        );
        // 幂等
        mgr.set_site_enabled(&e, "a.test", false, None)
            .await
            .unwrap();
        mgr.set_site_enabled(&e, "a.test", true, None)
            .await
            .unwrap();
        assert_eq!(get(&root, SITE_PATH).as_deref(), Some("cfg"));
        assert!(get(&root, &format!("{SITE_PATH}.disabled")).is_none());

        mgr.remove_site(&e, "a.test").await.unwrap();
        assert!(get(&root, SITE_PATH).is_none());
        // 未部署时删除为 no-op
        mgr.remove_site(&e, "a.test").await.unwrap();
    }

    #[tokio::test]
    async fn ols_deploy_registers_vhost_and_remove_unregisters() {
        let (root, _runner, mgr) = setup("ols");
        let main = "listener Default {\n  address *:8088\n  secure 0\n}\n";
        put(&root, "/usr/local/lsws/conf/httpd_config.conf", main);
        // 旧实现遗留的同名文件（与新目录冲突）
        put(
            &root,
            "/usr/local/lsws/conf/vhosts/a.test",
            "virtualHost a.test {}",
        );
        let e = WebServerEngine::OpenLiteSpeed;
        assert!(mgr
            .deploy_site(&e, "a.test", "docRoot $VH_ROOT/", None)
            .await
            .is_err());
        let d = mgr
            .deploy_site(&e, "a.test", "docRoot $VH_ROOT/", Some("/www/a"))
            .await
            .unwrap();
        assert_eq!(d.path, "/usr/local/lsws/conf/vhosts/a.test/vhconf.conf");
        let reg = get(&root, "/usr/local/lsws/conf/httpd_config.conf").unwrap();
        assert!(reg.contains("virtualHost a.test {"));
        assert!(reg.contains("map                     a.test a.test"));

        mgr.set_site_enabled(&e, "a.test", false, Some("/www/a"))
            .await
            .unwrap();
        let off = get(&root, "/usr/local/lsws/conf/httpd_config.conf").unwrap();
        assert!(!off.contains("a.test"));
        mgr.set_site_enabled(&e, "a.test", true, Some("/www/a"))
            .await
            .unwrap();

        mgr.remove_site(&e, "a.test").await.unwrap();
        assert_eq!(
            get(&root, "/usr/local/lsws/conf/httpd_config.conf").as_deref(),
            Some(main)
        );
    }

    #[tokio::test]
    async fn write_config_file_atomic_rolls_back_on_failed_test() {
        let (root, runner, mgr) = setup("global");
        put(&root, "/etc/nginx/nginx.conf", "good");
        runner
            .fail_test
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(mgr
            .write_config_file_atomic(
                &WebServerEngine::Nginx,
                "/etc/nginx/nginx.conf",
                "bad",
                true
            )
            .await
            .is_err());
        assert_eq!(get(&root, "/etc/nginx/nginx.conf").as_deref(), Some("good"));
    }

    // ── Caddy 生命周期路由：start/stop/check_status 走 caddy CLI ──────────

    /// 记录型 Runner：记录调用但不真正执行。
    struct CaddyRecordingRunner {
        calls: std::sync::Mutex<Vec<(String, Vec<String>)>>,
    }

    impl CaddyRecordingRunner {
        fn new() -> Self {
            Self {
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }
        fn all(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::application::execution_mode::PrivilegedCommandRunner for CaddyRecordingRunner {
        async fn run(
            &self,
            cmd: &crate::application::execution_mode::PrivilegedCommand,
        ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push((cmd.program.clone(), cmd.args.clone()));
            Ok(crate::application::execution_mode::CommandOutput {
                stdout: "ok".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    fn caddy_instance() -> WebServerInstance {
        WebServerInstance {
            id: 1,
            engine: "caddy".into(),
            version: Some("v2.9.1".into()),
            status: "running".into(),
            config_path: "/etc/caddy/Caddyfile".into(),
            binary_path: Some("/usr/bin/caddy".into()),
            port: 80,
            created_at: chrono::Utc::now(),
            resource_version: 0,
        }
    }

    #[tokio::test]
    async fn caddy_start_uses_start_with_config() {
        let runner = std::sync::Arc::new(CaddyRecordingRunner::new());
        let mgr = WebServerManager::new_with_runner(runner.clone());
        let inst = caddy_instance();
        let _ = mgr.start(&inst).await;

        let calls = runner.all();
        let start = calls
            .iter()
            .find(|(p, _)| p == "caddy")
            .expect("should invoke caddy");
        assert_eq!(start.1, vec!["start", "--config", "/etc/caddy/Caddyfile"]);
    }

    #[tokio::test]
    async fn caddy_stop_uses_stop_with_config() {
        let runner = std::sync::Arc::new(CaddyRecordingRunner::new());
        let mgr = WebServerManager::new_with_runner(runner.clone());
        let inst = caddy_instance();
        let _ = mgr.stop(&inst).await;

        let calls = runner.all();
        let stop = calls
            .iter()
            .find(|(p, _)| p == "caddy")
            .expect("should invoke caddy");
        assert_eq!(stop.1, vec!["stop", "--config", "/etc/caddy/Caddyfile"]);
    }

    #[tokio::test]
    async fn caddy_status_uses_systemctl() {
        let runner = std::sync::Arc::new(CaddyRecordingRunner::new());
        let mgr = WebServerManager::new_with_runner(runner.clone());
        let inst = caddy_instance();
        let status = mgr.check_status(&inst).await.unwrap();
        assert_eq!(status, "running");

        let calls = runner.all();
        let sys = calls
            .iter()
            .find(|(p, _)| p == "systemctl")
            .expect("should invoke systemctl for caddy");
        assert_eq!(sys.1, vec!["is-active", "caddy"]);
    }

    #[tokio::test]
    async fn caddy_reload_routes_with_config() {
        let runner = std::sync::Arc::new(CaddyRecordingRunner::new());
        let mgr = WebServerManager::new_with_runner(runner.clone());
        let inst = caddy_instance();
        let _ = mgr.reload(&inst).await;

        let calls = runner.all();
        let reload = calls
            .iter()
            .find(|(p, _)| p == "caddy")
            .expect("should invoke caddy for reload");
        assert_eq!(reload.1, vec!["reload", "--config", "/etc/caddy/Caddyfile"]);
    }

    #[tokio::test]
    async fn caddy_config_test_routes_with_config() {
        let runner = std::sync::Arc::new(CaddyRecordingRunner::new());
        let mgr = WebServerManager::new_with_runner(runner.clone());
        let inst = caddy_instance();
        let _ = mgr.config_test(&inst).await;

        let calls = runner.all();
        let validate = calls
            .iter()
            .find(|(p, _)| p == "caddy")
            .expect("should invoke caddy for config test");
        assert_eq!(
            validate.1,
            vec!["validate", "--config", "/etc/caddy/Caddyfile"]
        );
    }
}
