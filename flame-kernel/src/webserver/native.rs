use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::infrastructure::os::{PackageManager, ServiceManager};
use crate::webserver::engine::WebServerEngine;
use serde::Serialize;
use std::collections::HashSet;

/// 原生 Web 服务器检测信息（1Panel 风格：安装状态 / 版本 / 服务 / 端口）
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct NativeWebServerInfo {
    pub engine: String,
    pub description: String,
    /// 二进制是否存在于 PATH（dpkg/rpm/apk 或 which）
    pub installed: bool,
    /// 发行版包是否已安装（nginx/httpd 等包名）
    pub package_installed: bool,
    pub version: Option<String>,
    pub service_name: Option<String>,
    /// systemd 服务是否运行中
    pub running: bool,
    /// systemd 是否开机自启
    pub enabled: bool,
    pub binary_path: Option<String>,
    pub config_path: String,
    pub default_port: u16,
    /// 当前监听中的端口（ss/netstat 检测）
    pub listening_ports: Vec<u16>,
}

pub struct WebServerNativeManager {
    package_manager: PackageManager,
    service_manager: ServiceManager,
    /// 特权命令执行器：版本 / which / 端口扫描等检测命令统一经此端口执行
    /// （Phase A1 扩展：`execution_mode=embedded|agent` 分离模式）。
    runner: SharedCommandRunner,
}

impl WebServerNativeManager {
    /// 访问内部 systemd 服务管理器（原生生命周期 start/stop/restart 使用）。
    pub fn service_manager(&self) -> &ServiceManager {
        &self.service_manager
    }

    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
        }
    }

    /// 便捷：默认嵌入式执行器（行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 检测全部 5 种引擎的原生状态
    pub async fn detect_all(&self) -> Vec<NativeWebServerInfo> {
        let mut out = Vec::new();
        for engine in [
            WebServerEngine::Nginx,
            WebServerEngine::Apache,
            WebServerEngine::OpenLiteSpeed,
            WebServerEngine::OpenResty,
            WebServerEngine::Caddy,
        ] {
            out.push(self.detect(&engine).await);
        }
        out
    }

    pub async fn detect(&self, engine: &WebServerEngine) -> NativeWebServerInfo {
        // Apache 的包名/服务名在 Debian 系（apache2）与 RHEL 系（httpd）间分叉，
        // 检测前先识别发行版，全链路使用同一份发行版感知元数据。
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let service_name = engine.service_name_for(&distro);
        let binary_path = self.detect_binary_path(engine).await;
        let installed = binary_path.is_some();
        let package_installed = self
            .package_manager
            .is_installed(engine.package_name_for(&distro))
            .await
            .unwrap_or(false);
        let version = if installed {
            match self.get_version(engine).await {
                Ok(v) => Some(v),
                Err(_) => self
                    .package_manager
                    .get_version(engine.package_name_for(&distro))
                    .await
                    .ok()
                    .or_else(|| Some("unknown".into())),
            }
        } else {
            None
        };
        let running = if installed {
            let via_systemd = self
                .service_manager
                .is_running(service_name)
                .await
                .unwrap_or(false);
            if !via_systemd {
                if let Some(script) = engine.control_script() {
                    // OLS systemd 单元不可用时回退 lswsctrl status
                    self.run_ok(script, &["status"]).await
                } else {
                    false
                }
            } else {
                true
            }
        } else {
            false
        };
        let enabled = self
            .service_manager
            .is_enabled(service_name)
            .await
            .unwrap_or(false);
        let listening_ports = self
            .listening_ports()
            .await
            .into_iter()
            .filter(|p| *p == engine.default_port() || *p == engine.default_ssl_port())
            .collect();

        NativeWebServerInfo {
            engine: engine.as_str().into(),
            description: engine.description().into(),
            installed,
            package_installed,
            version,
            service_name: Some(service_name.into()),
            running,
            enabled,
            binary_path,
            config_path: engine.default_config_path_for(&distro).into(),
            default_port: engine.default_port(),
            listening_ports,
        }
    }

    /// 原生安装：包管理器安装 + systemd 启用并启动
    pub async fn install(
        &self,
        engine: &WebServerEngine,
        version: Option<&str>,
    ) -> Result<String, AppError> {
        if self.detect_binary_path(engine).await.is_some() {
            return Err(AppError::BadRequest(format!(
                "{} is already installed",
                engine.as_str()
            )));
        }
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        // OpenLiteSpeed / Caddy 不在发行版默认仓库（或版本陈旧），走官方仓库安装路径；
        // 其余引擎保持系统包管理器安装；Apache 按发行版选包名（apache2/httpd）。
        match engine {
            WebServerEngine::OpenLiteSpeed => self.install_openlitespeed().await?,
            WebServerEngine::Caddy => self.install_caddy().await?,
            _ => {
                let base_pkg = engine.package_name_for(&distro);
                let pkg = if let Some(ver) = version {
                    if ver.is_empty() {
                        base_pkg.to_string()
                    } else {
                        format!("{}={}", base_pkg, ver)
                    }
                } else {
                    base_pkg.to_string()
                };
                self.package_manager.install(&pkg).await?;
                // Debian 系 Apache：安装后启用常用代理/PHP 模块（RHEL 系默认已编译内置）
                if engine.apache_debian_layout(&distro) {
                    self.apache_enable_debian_modules().await;
                }
            }
        }
        let service_name = engine.service_name_for(&distro);
        self.service_manager.enable(service_name).await?;
        self.service_manager.start(service_name).await?;
        let ver = self
            .get_version(engine)
            .await
            .unwrap_or_else(|_| "latest".into());
        Ok(format!(
            "{} v{} installed and started",
            engine.as_str(),
            ver
        ))
    }

    /// 原生卸载：停止 + 禁用 + 移除包。
    /// OpenLiteSpeed 优先 `lswsctrl stop` 优雅停止（systemd 单元可能不存在）。
    pub async fn uninstall(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        if let Some(script) = engine.control_script() {
            let _ = self.run_ok(script, &["stop"]).await;
        }
        let service_name = engine.service_name_for(&distro);
        self.service_manager.stop(service_name).await.ok();
        self.service_manager.disable(service_name).await.ok();
        self.package_manager
            .uninstall(engine.package_name_for(&distro))
            .await
    }

    /// systemd 开机自启开关
    pub async fn set_autostart(
        &self,
        engine: &WebServerEngine,
        enabled: bool,
    ) -> Result<(), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let service_name = engine.service_name_for(&distro);
        if enabled {
            self.service_manager.enable(service_name).await
        } else {
            self.service_manager.disable(service_name).await
        }
    }

    /// 检测版本（各引擎 CLI 输出格式不同，统一解析）
    async fn get_version(&self, engine: &WebServerEngine) -> Result<String, AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let (binary, args) = engine.version_command_for(&distro);
        let cmd = crate::application::execution_mode::PrivilegedCommand::new(
            binary,
            args.iter().map(|s| s.to_string()).collect(),
        )
        .timeout(10);
        let out = self
            .runner
            .run(&cmd)
            .await
            .map_err(|e| AppError::internal(format!("Version check failed: {}", e)))?;
        let combined = out.combined();
        // nginx/1.18.0 (Ubuntu)
        // nginx version: nginx/1.27.0
        // Server version: Apache/2.4.41
        // openresty/1.19.3.2
        // v2.7.6 h1:xxxx
        // OpenLiteSpeed/1.7.19 (build 1)
        let re = regex::Regex::new(
            r"(nginx|openresty|Apache|httpd|caddy|OpenLiteSpeed|v)?[/ ]?(\d+\.\d+(\.\d+)?)",
        )
        .map_err(|e| AppError::internal(format!("Regex error: {}", e)))?;
        if let Some(caps) = re.captures(&combined) {
            if let Some(ver) = caps.get(2) {
                return Ok(ver.as_str().to_string());
            }
        }
        Err(AppError::internal("Version not detected"))
    }

    async fn which(&self, binary: &str) -> Option<String> {
        let cmd = crate::application::execution_mode::PrivilegedCommand::new(
            "which",
            vec![binary.to_string()],
        )
        .timeout(10);
        let out = self.runner.run(&cmd).await.ok()?;
        if out.success() {
            let path = out.stdout.trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
        None
    }

    /// 原生安装检测：PATH `which` 探测 + 引擎候选绝对路径回退。
    ///
    /// OpenLiteSpeed 安装到 `/usr/local/lsws/bin/lshttpd`（不在 PATH），
    /// 仅靠 `which` 会误判为未安装。
    async fn detect_binary_path(&self, engine: &WebServerEngine) -> Option<String> {
        // Apache 二进制名随发行版分叉（apache2/httpd），which 按发行版取名；
        // 无论命中哪个发行版名，候选绝对路径均覆盖双发行版，保证探测不漏。
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        if let Some(p) = self.which(engine.binary_name_for(&distro)).await {
            return Some(p);
        }
        if engine.apache_debian_layout(&distro) {
            // Debian 系再补一次 RHEL 风格探测，双发行版镜像容器可能带另一套
            if let Some(p) = self.which("httpd").await {
                return Some(p);
            }
        }

        for cand in engine.binary_candidates() {
            if self.run_ok(cand, &[]).await {
                return Some((*cand).to_string());
            }
        }
        None
    }

    /// Debian 系 Apache 安装后启用常用模块（proxy/proxy_fcgi/setenvif/rewrite/headers）。
    ///
    /// Debian 的 `apache2` 包将模块拆分为 `/etc/apache2/mods-available`，需经
    /// `a2enmod` 显式启用；RHEL 系 `httpd` 编译内置，无需此步。单个模块启用
    /// 失败仅记录告警不阻断安装（不同包版本可用模块集合略有差异）。
    async fn apache_enable_debian_modules(&self) {
        for module in ["proxy", "proxy_fcgi", "setenvif", "rewrite", "headers"] {
            let out = self
                .runner
                .run(
                    &crate::application::execution_mode::PrivilegedCommand::new(
                        "a2enmod",
                        vec![module.to_string()],
                    )
                    .prefer_root()
                    .timeout(30),
                )
                .await;
            match out {
                Ok(o) if o.success() => {}
                Ok(o) => {
                    tracing::warn!("a2enmod {} failed: {}", module, o.stderr.trim());
                }
                Err(e) => tracing::warn!("a2enmod {} failed: {}", module, e),
            }
        }
    }

    /// OpenLiteSpeed 官方仓库原生安装：
    /// 1. 写入 LiteSpeed 官方 APT/YUM 源配置（Debian 系 / RHEL 系）
    /// 2. `openlitespeed` 包安装
    /// 3. `lswsctrl start` 启动（systemd 单元由包自带，enable 交给上层）
    ///
    /// 若发行版无法识别则返回 BadRequest，提示手动安装。
    async fn install_openlitespeed(&self) -> Result<(), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        if is_suse_or_arch(&distro) {
            return Err(AppError::BadRequest(
                "OpenLiteSpeed 官方仓库不提供 openSUSE / Arch 包，请改用 Docker 模式或手动安装"
                    .into(),
            ));
        }
        // Doc/33 P-5：全链路 HTTPS + 包签名校验；不再经 `sh -c` 拼接，文件内容走 stdin（`tee`）。
        // - Debian 系：签名密钥取官方二进制 keyring `lst_repo.gpg`（旧实现的 `repo.litespeed.sh/gpg.key`
        //   实为安装脚本，`gpg --dearmor` 必失败）
        // - RHEL 系：直接写 `.repo`（旧实现装 EL7 的 `litespeed-repo-1.1-1.el7` rpm，EL8+ 已不适用）
        let repo_setup: Vec<PrivilegedCommand> = match distro {
            crate::infrastructure::os::DistroType::Ubuntu
            | crate::infrastructure::os::DistroType::Debian
            | crate::infrastructure::os::DistroType::Unknown(_) => vec![
                PrivilegedCommand::new(
                    "wget",
                    vec![
                        "-qO".into(),
                        LITESPEED_APT_KEYRING.into(),
                        LITESPEED_APT_KEY_URL.into(),
                    ],
                ),
                PrivilegedCommand::new("tee", vec![LITESPEED_APT_LIST.into()]).stdin(
                    litespeed_apt_source(litespeed_apt_codename(
                        crate::infrastructure::os::os_codename().as_deref(),
                    )),
                ),
                PrivilegedCommand::new("apt-get", vec!["update".into()]),
            ],
            crate::infrastructure::os::DistroType::CentOS
            | crate::infrastructure::os::DistroType::RHEL
            | crate::infrastructure::os::DistroType::Fedora => {
                let el =
                    litespeed_el_version(&distro, crate::infrastructure::os::os_major_version());
                vec![
                    PrivilegedCommand::new("tee", vec![LITESPEED_YUM_REPO.into()])
                        .stdin(litespeed_yum_repo(el)),
                ]
            }
            crate::infrastructure::os::DistroType::Alpine => {
                return Err(AppError::BadRequest(
                    "OpenLiteSpeed native install is not supported on Alpine; use Docker mode or install manually".into(),
                ));
            }
        };
        for cmd in repo_setup {
            let out = self
                .runner
                .run(&cmd.prefer_root().timeout(600))
                .await
                .map_err(|e| {
                    AppError::internal(format!("OpenLiteSpeed repo setup failed: {}", e))
                })?;
            if !out.success() {
                return Err(AppError::internal(format!(
                    "OpenLiteSpeed repo setup failed: {}",
                    out.combined()
                )));
            }
        }
        self.package_manager.install("openlitespeed").await?;
        Ok(())
    }

    /// Caddy 官方仓库原生安装（Cloudsmith stable 源）：
    ///
    /// Caddy 不在多数发行版默认仓库（或版本陈旧），走官方 stable 仓库：
    ///   Debian/Ubuntu: 写入 `caddy-stable.list` apt 源 → `apt install caddy`
    ///   RHEL/CentOS:   写入 `caddy-stable.repo` yum 源 → `yum install caddy`
    ///   Alpine:        社区源自带 caddy 包，直接 `apk add caddy`（跳过仓库配置）
    ///
    /// 每条仓库配置命令拆为单条 PrivilegedCommand（无管道/无 `&&`），
    /// 便于 Agent 白名单前缀匹配；单条失败即中止返回内部错误。
    async fn install_caddy(&self) -> Result<(), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        // openSUSE / Arch 官方仓库自带 caddy，无需第三方源（旧实现落入 Debian 分支写 apt 源）
        if is_suse_or_arch(&distro) {
            self.package_manager.install("caddy").await?;
            return Ok(());
        }
        let repo_setup: Vec<(String, Vec<String>)> = match distro {
            crate::infrastructure::os::DistroType::Ubuntu
            | crate::infrastructure::os::DistroType::Debian
            | crate::infrastructure::os::DistroType::Unknown(_) => vec![
                (
                    "wget".into(),
                    vec![
                        "-qO".into(),
                        "/tmp/caddy-stable.gpg".into(),
                        "https://dl.cloudsmith.io/public/caddy/stable/gpg.key".into(),
                    ],
                ),
                (
                    "gpg".into(),
                    vec![
                        "--dearmor".into(),
                        "-o".into(),
                        "/usr/share/keyrings/caddy-stable-archive-keyring.gpg".into(),
                        "/tmp/caddy-stable.gpg".into(),
                    ],
                ),
                (
                    "wget".into(),
                    vec![
                        "-qO".into(),
                        "/etc/apt/sources.list.d/caddy-stable.list".into(),
                        "https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt".into(),
                    ],
                ),
                ("apt-get".into(), vec!["update".into()]),
            ],
            crate::infrastructure::os::DistroType::CentOS
            | crate::infrastructure::os::DistroType::RHEL
            | crate::infrastructure::os::DistroType::Fedora => vec![
                (
                    "wget".into(),
                    vec![
                        "-qO".into(),
                        "/tmp/caddy-stable.gpg".into(),
                        "https://dl.cloudsmith.io/public/caddy/stable/gpg.key".into(),
                    ],
                ),
                (
                    "gpg".into(),
                    vec![
                        "--dearmor".into(),
                        "-o".into(),
                        "/usr/share/keyrings/caddy-stable-archive-keyring.gpg".into(),
                        "/tmp/caddy-stable.gpg".into(),
                    ],
                ),
                (
                    "wget".into(),
                    vec![
                        "-qO".into(),
                        "/etc/yum.repos.d/caddy-stable.repo".into(),
                        "https://dl.cloudsmith.io/public/caddy/stable/rpm.repo.txt".into(),
                    ],
                ),
            ],
            crate::infrastructure::os::DistroType::Alpine => Vec::new(),
        };
        for (prog, args) in repo_setup {
            let out = self
                .runner
                .run(
                    &crate::application::execution_mode::PrivilegedCommand::new(prog, args)
                        .prefer_root()
                        .timeout(600),
                )
                .await
                .map_err(|e| AppError::internal(format!("Caddy repo setup failed: {}", e)))?;
            if !out.success() {
                return Err(AppError::internal(format!(
                    "Caddy repo setup failed: {}",
                    out.combined()
                )));
            }
        }
        self.package_manager.install("caddy").await?;
        Ok(())
    }

    /// 探测某命令能否成功执行（存在性/可运行性检查）。
    async fn run_ok(&self, program: &str, args: &[&str]) -> bool {
        let cmd = crate::application::execution_mode::PrivilegedCommand::new(
            program,
            args.iter().map(|s| s.to_string()).collect(),
        )
        .timeout(10);
        matches!(self.runner.run(&cmd).await, Ok(o) if o.success())
    }

    /// 扫描当前监听端口（优先 ss，回退 netstat；统一经特权命令端口执行）
    async fn listening_ports(&self) -> HashSet<u16> {
        let mut ports = HashSet::new();
        for (prog, arg) in [("ss", "-tln"), ("netstat", "-tln")] {
            let cmd = crate::application::execution_mode::PrivilegedCommand::new(
                prog,
                vec![arg.to_string()],
            )
            .timeout(10);
            let out = self.runner.run(&cmd).await;
            if let Ok(o) = out {
                if !o.success() {
                    continue;
                }
                let s = o.stdout;
                for line in s.lines().skip(1) {
                    // Local Address:Port 形如 0.0.0.0:80 / [::]:443
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let addr = parts[3];
                        if let Some(port_str) = addr.rsplit(':').next() {
                            if let Ok(p) = port_str.trim_matches(']').parse::<u16>() {
                                if p > 0 {
                                    ports.insert(p);
                                }
                            }
                        }
                    }
                }
                if !ports.is_empty() {
                    break;
                }
            }
        }
        ports
    }

    // ── 配套能力（安装时"各取所需"，Doc/13 §5.4 阶段 3）─────────────────────
    //
    // 查询与批量安装引擎的差异化依赖/配套（PHP-FPM、Lua、Redis 缓存、扩展模块、
    // 自动 HTTPS 等）。元数据声明在 `webserver/capability.rs`，本层只负责经
    // `PackageManager` 端口执行批量安装。

    /// 引擎支持的配套能力清单（含按当前发行版计算的推荐包与配置提示）。
    pub async fn engine_capabilities(
        &self,
        engine: &WebServerEngine,
    ) -> Vec<crate::webserver::capability::CapabilityInfo> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        crate::webserver::capability::capabilities_for(engine)
            .into_iter()
            .map(|cap| crate::webserver::capability::CapabilityInfo {
                key: cap.as_str().into(),
                label_zh: cap.label_zh().into(),
                label_en: cap.label_en().into(),
                packages: crate::webserver::capability::companion_packages(engine, &cap, &distro),
                hint: crate::webserver::capability::post_install_hints(engine, &cap).into(),
            })
            .collect()
    }

    /// 批量安装引擎的配套依赖（PHP-FPM / Lua / Redis 等）。
    ///
    /// 每个能力独立计算推荐包；单包失败仅记录错误并继续（配套缺失不阻断整体）。
    /// 返回 `(成功安装的包, 失败的包列表)`。
    pub async fn install_capabilities(
        &self,
        engine: &WebServerEngine,
        caps: &[crate::webserver::capability::WebServerCapability],
    ) -> Result<(Vec<String>, Vec<String>), AppError> {
        let distro = crate::infrastructure::os::OsInfo::detect_distro().await;
        let mut installed = Vec::new();
        let mut failed = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for cap in caps {
            // 仅接受该引擎支持的能力（防前端夹带不支持项）
            if !crate::webserver::capability::supports_capability(engine, cap) {
                continue;
            }
            for pkg in crate::webserver::capability::companion_packages(engine, cap, &distro) {
                if !seen.insert(pkg.clone()) {
                    continue; // 去重（如 php-fpm 被多个能力重复推荐）
                }
                if self
                    .package_manager
                    .is_installed(&pkg)
                    .await
                    .unwrap_or(false)
                {
                    continue; // 已安装跳过
                }
                match self.package_manager.install(&pkg).await {
                    Ok(_) => installed.push(pkg),
                    Err(e) => {
                        tracing::warn!("companion package {} install failed: {}", pkg, e);
                        failed.push(pkg);
                    }
                }
            }
        }
        Ok((installed, failed))
    }
}

impl Default for WebServerNativeManager {
    fn default() -> Self {
        Self::embedded()
    }
}

/// openSUSE / Arch（`DistroType::Unknown` 承载的族标签）。
fn is_suse_or_arch(distro: &crate::infrastructure::os::DistroType) -> bool {
    use crate::infrastructure::os::{DistroType, FAMILY_ARCH, FAMILY_SUSE};
    matches!(distro, DistroType::Unknown(f) if f == FAMILY_SUSE || f == FAMILY_ARCH)
}

/// LiteSpeed APT 仓库代号：仓库提供的代号原样使用，其余（含未知/新版本）回退 bookworm。
/// LiteSpeed 官方仓库（Doc/33 P-5：HTTPS 已验证可用，签名密钥见 [`LITESPEED_APT_KEY_URL`]）。
pub(crate) const LITESPEED_APT_KEYRING: &str = "/usr/share/keyrings/litespeed.gpg";
pub(crate) const LITESPEED_APT_KEY_URL: &str = "https://rpms.litespeedtech.com/debian/lst_repo.gpg";
pub(crate) const LITESPEED_APT_LIST: &str = "/etc/apt/sources.list.d/litespeed.list";
pub(crate) const LITESPEED_YUM_REPO: &str = "/etc/yum.repos.d/litespeed.repo";

fn litespeed_apt_source(codename: &str) -> String {
    format!(
        "deb [signed-by={LITESPEED_APT_KEYRING}] https://rpms.litespeedtech.com/debian {codename} main\n"
    )
}

/// 仓库目录用的 EL 主版本：RHEL 系取 os-release（官方仓库提供 8/9/10）；
/// Fedora 无对应目录，与官方脚本一致回退 EL10 包。
fn litespeed_el_version(distro: &crate::infrastructure::os::DistroType, major: Option<u32>) -> u32 {
    match (distro, major) {
        (crate::infrastructure::os::DistroType::Fedora, _) => 10,
        (_, Some(v)) if (8..=10).contains(&v) => v,
        (_, Some(v)) if v > 10 => 10,
        _ => 9,
    }
}

/// `.repo` 内容：HTTPS baseurl + gpgcheck。EL10 起官方换用 `RPM-GPG-KEY-litespeed2025`
/// （旧 DSA 密钥不被 EL10 默认加密策略接受）。
fn litespeed_yum_repo(el: u32) -> String {
    let key = if el >= 10 {
        "RPM-GPG-KEY-litespeed2025"
    } else {
        "RPM-GPG-KEY-litespeed"
    };
    let base = "https://rpms.litespeedtech.com/centos";
    format!(
        "[litespeed]\nname=LiteSpeed Tech Repository for EL{el}\nbaseurl={base}/{el}/$basearch/\nenabled=1\ngpgcheck=1\ngpgkey={base}/{key}\n\n[litespeed-update]\nname=LiteSpeed Tech Update Repository for EL{el}\nbaseurl={base}/{el}/update/$basearch/\nenabled=1\ngpgcheck=1\ngpgkey={base}/{key}\n"
    )
}

fn litespeed_apt_codename(detected: Option<&str>) -> &'static str {
    const SUPPORTED: &[&str] = &["bullseye", "bookworm", "trixie", "focal", "jammy", "noble"];
    detected
        .and_then(|c| SUPPORTED.iter().find(|s| **s == c).copied())
        .unwrap_or("bookworm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn litespeed_codename_whitelisted() {
        assert_eq!(litespeed_apt_codename(Some("noble")), "noble");
        assert_eq!(litespeed_apt_codename(Some("jammy")), "jammy");
        assert_eq!(litespeed_apt_codename(Some("trixie")), "trixie");
        assert_eq!(litespeed_apt_codename(Some("sid")), "bookworm");
        assert_eq!(litespeed_apt_codename(None), "bookworm");
    }

    #[test]
    fn litespeed_repos_are_https_and_signed() {
        let apt = litespeed_apt_source("noble");
        assert!(apt.starts_with("deb [signed-by=/usr/share/keyrings/litespeed.gpg] https://"));
        assert!(apt.contains(" noble main"));
        assert!(
            LITESPEED_APT_KEY_URL.starts_with("https://")
                && LITESPEED_APT_KEY_URL.ends_with(".gpg")
        );
        let el9 = litespeed_yum_repo(9);
        assert!(el9.contains("baseurl=https://rpms.litespeedtech.com/centos/9/$basearch/"));
        assert!(el9.contains("gpgcheck=1") && el9.contains("/RPM-GPG-KEY-litespeed\n"));
        assert!(!el9.contains("http://"));
        assert!(litespeed_yum_repo(10).contains("RPM-GPG-KEY-litespeed2025"));
    }

    #[test]
    fn litespeed_el_version_mapping() {
        use crate::infrastructure::os::DistroType;
        assert_eq!(litespeed_el_version(&DistroType::RHEL, Some(9)), 9);
        assert_eq!(litespeed_el_version(&DistroType::RHEL, Some(8)), 8);
        assert_eq!(litespeed_el_version(&DistroType::CentOS, Some(7)), 9);
        assert_eq!(litespeed_el_version(&DistroType::RHEL, None), 9);
        assert_eq!(litespeed_el_version(&DistroType::Fedora, Some(41)), 10);
    }

    #[tokio::test]
    async fn detect_all_returns_five() {
        let m = WebServerNativeManager::embedded();
        let all = m.detect_all().await;
        assert_eq!(all.len(), 5);
        for info in &all {
            assert!(!info.engine.is_empty());
            assert!(!info.config_path.is_empty());
            assert!(info.default_port > 0);
        }
    }

    #[test]
    fn split_image_tag_helpers() {
        // 验证引擎包名映射
        assert_eq!(WebServerEngine::Nginx.package_name(), "nginx");
        assert_eq!(WebServerEngine::Apache.service_name(), "httpd");
        assert_eq!(WebServerEngine::Caddy.default_port(), 80);
    }

    // ── Phase A1 扩展：原生检测命令统一经特权命令端口执行 ────────────────

    /// 记录型 Runner：不真正执行命令，只记录被调用的命令，供断言路由。
    struct RecordingRunner {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }
        fn programs(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::application::execution_mode::PrivilegedCommandRunner for RecordingRunner {
        async fn run(
            &self,
            cmd: &crate::application::execution_mode::PrivilegedCommand,
        ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            Ok(crate::application::execution_mode::CommandOutput {
                stdout: "ok".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    /// 可编程输出 Runner：按 (program, 首个 arg 前缀) 返回预设成功/失败，
    /// 供 OpenLiteSpeed 绝对路径探测/版本探测路由断言。
    struct ScriptedRunner {
        ok_programs: Vec<String>,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl ScriptedRunner {
        fn with_ok(programs: &[&str]) -> Self {
            Self {
                ok_programs: programs.iter().map(|s| s.to_string()).collect(),
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }
        fn programs(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::application::execution_mode::PrivilegedCommandRunner for ScriptedRunner {
        async fn run(
            &self,
            cmd: &crate::application::execution_mode::PrivilegedCommand,
        ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
            let line = format!("{} {}", cmd.program, cmd.args.join(" "));
            self.calls.lock().unwrap().push(line);
            let ok = self.ok_programs.contains(&cmd.program)
                || self
                    .ok_programs
                    .iter()
                    .any(|p| cmd.program.starts_with(p.as_str()));
            Ok(crate::application::execution_mode::CommandOutput {
                stdout: if ok {
                    "OpenLiteSpeed/1.7.19 (build 1)".into()
                } else {
                    String::new()
                },
                stderr: String::new(),
                exit_code: if ok { 0 } else { 1 },
            })
        }
    }

    #[tokio::test]
    async fn ols_detect_uses_absolute_binary_path() {
        // which lshttpd 失败，但绝对路径 /usr/local/lsws/bin/lshttpd 可执行
        let runner = std::sync::Arc::new(ScriptedRunner::with_ok(&["/usr/local/lsws/bin/lshttpd"]));
        let m = WebServerNativeManager::new(runner.clone());
        let info = m.detect(&WebServerEngine::OpenLiteSpeed).await;
        // 探测过绝对路径
        let programs = runner.programs();
        assert!(programs.iter().any(|p| p.starts_with("which lshttpd")));
        assert!(programs
            .iter()
            .any(|p| p.starts_with("/usr/local/lsws/bin/lshttpd")));
        // installed 判定为真（版本输出来自可编程 runner）
        assert!(
            info.installed,
            "OLS absolute-path detection should mark installed"
        );
        assert!(info.version.is_some());
    }

    #[tokio::test]
    async fn ols_version_command_routes_to_lsws_path() {
        let (prog, args) = WebServerEngine::OpenLiteSpeed.version_command();
        assert_eq!(prog, "/usr/local/lsws/bin/lshttpd");
        assert_eq!(args, &["-v"]);
        // 其他引擎保持相对路径
        assert_eq!(WebServerEngine::Nginx.version_command().0, "nginx");
    }

    #[tokio::test]
    async fn ols_binary_candidates_and_control_script() {
        let cands = WebServerEngine::OpenLiteSpeed.binary_candidates();
        assert!(cands.contains(&"/usr/local/lsws/bin/lshttpd"));
        assert_eq!(
            WebServerEngine::OpenLiteSpeed.control_script(),
            Some("/usr/local/lsws/bin/lswsctrl")
        );
        assert_eq!(WebServerEngine::Nginx.control_script(), None);
    }

    // ── Apache 发行版感知专项适配：包名/服务名/控制命令按发行版路由 ──────────

    use crate::infrastructure::os::DistroType;

    #[test]
    fn apache_metadata_routes_by_distro() {
        let debian = DistroType::Debian;
        let ubuntu = DistroType::Ubuntu;
        let rhel = DistroType::RHEL;
        let apache = WebServerEngine::Apache;

        // 包名/服务名：Debian 系 apache2，RHEL 系 httpd
        assert_eq!(apache.package_name_for(&debian), "apache2");
        assert_eq!(apache.package_name_for(&ubuntu), "apache2");
        assert_eq!(apache.package_name_for(&rhel), "httpd");
        assert_eq!(apache.service_name_for(&debian), "apache2");
        assert_eq!(apache.service_name_for(&rhel), "httpd");

        // 配置布局：Debian 系 /etc/apache2，RHEL 系 /etc/httpd
        assert_eq!(
            apache.default_config_path_for(&debian),
            "/etc/apache2/apache2.conf"
        );
        assert_eq!(
            apache.default_config_path_for(&rhel),
            "/etc/httpd/conf/httpd.conf"
        );
        assert_eq!(
            apache.sites_available_dir_for(&debian),
            "/etc/apache2/sites-available"
        );
        assert_eq!(
            apache.sites_enabled_dir_for(&ubuntu),
            "/etc/apache2/sites-enabled"
        );
        assert_eq!(apache.sites_enabled_dir_for(&rhel), "/etc/httpd/conf.d");

        // 控制命令：Debian 系 apache2ctl，RHEL 系 httpd
        assert_eq!(
            apache.config_test_command_for(&debian),
            "apache2ctl configtest"
        );
        assert_eq!(apache.config_test_command_for(&rhel), "httpd -t");
        assert_eq!(apache.reload_command_for(&debian), "apache2ctl graceful");
        assert_eq!(apache.reload_command_for(&rhel), "httpd -k graceful");
        assert_eq!(
            apache.version_command_for(&debian),
            ("apache2", &["-v"] as &[&str])
        );
        assert_eq!(apache.default_binary_path_for(&debian), "/usr/sbin/apache2");
        assert_eq!(apache.process_name_for(&debian), "apache2");
        assert_eq!(apache.process_name_for(&rhel), "httpd");

        // Debian 布局开关：仅 Apache + Debian 系为真
        assert!(apache.apache_debian_layout(&debian));
        assert!(apache.apache_debian_layout(&ubuntu));
        assert!(!apache.apache_debian_layout(&rhel));

        // 非 Apache 引擎不受发行版影响（透传默认元数据）
        assert_eq!(WebServerEngine::Nginx.package_name_for(&debian), "nginx");
        assert_eq!(WebServerEngine::Nginx.service_name_for(&rhel), "nginx");
        assert_eq!(
            WebServerEngine::Caddy.reload_command_for(&debian),
            "caddy reload --config /etc/caddy/Caddyfile"
        );

        // 未知发行版回退 RHEL 风格 httpd（与历史硬编码行为一致）
        assert_eq!(
            apache.package_name_for(&DistroType::Unknown("kali".into())),
            "httpd"
        );
    }

    #[tokio::test]
    async fn apache_binary_candidates_cover_both_distros() {
        let cands = WebServerEngine::Apache.binary_candidates();
        assert!(cands.contains(&"/usr/sbin/apache2"), "Debian 系路径");
        assert!(cands.contains(&"/usr/sbin/httpd"), "RHEL 系路径");
        // which 探测的两个二进制名都应尝试：apache2（Debian）与 httpd（RHEL）
        let runner = std::sync::Arc::new(ScriptedRunner::with_ok(&["/usr/sbin/apache2"]));
        let m = WebServerNativeManager::new(runner.clone());
        let info = m.detect(&WebServerEngine::Apache).await;
        let programs = runner.programs();
        assert!(programs.iter().any(|p| p.starts_with("which apache2")));
        assert!(programs.iter().any(|p| p.starts_with("which httpd")));
        assert!(programs.iter().any(|p| p.starts_with("/usr/sbin/apache2")));
        assert!(info.installed, "Debian 系绝对路径探测应命中");
    }

    #[tokio::test]
    async fn native_detection_routes_through_runner() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let m = WebServerNativeManager::new(runner.clone());

        let _ = m.detect(&WebServerEngine::Nginx).await;
        let programs = runner.programs();
        // which 检测二进制路径
        assert!(programs.iter().any(|p| p.starts_with("which ")));
        // 版本检测走统一端口（nginx -v）
        assert!(programs.iter().any(|p| p.starts_with("nginx -v")));
        // 端口扫描走统一端口（ss / netstat）
        assert!(programs
            .iter()
            .any(|p| p.starts_with("ss ") || p.starts_with("netstat ")));
    }

    // ── Caddy 原生专项适配：官方仓库 / 命令元数据 / 生命周期路由 ──────────

    #[test]
    fn caddy_uses_official_repo_and_config_path() {
        // Caddy 走官方仓库安装（Cloudsmith stable 源）
        assert!(WebServerEngine::Caddy.uses_official_repo());
        assert!(WebServerEngine::OpenLiteSpeed.uses_official_repo());
        assert!(!WebServerEngine::Nginx.uses_official_repo());
        assert!(!WebServerEngine::Apache.uses_official_repo());

        // config_test / reload 显式带 --config 路径（否则默认当前目录找 Caddyfile 失败）
        assert_eq!(
            WebServerEngine::Caddy.config_test_command(),
            "caddy validate --config /etc/caddy/Caddyfile"
        );
        assert_eq!(
            WebServerEngine::Caddy.reload_command(),
            "caddy reload --config /etc/caddy/Caddyfile"
        );
        // 其余引擎不受影响
        assert_eq!(WebServerEngine::Nginx.config_test_command(), "nginx -t");
        assert_eq!(WebServerEngine::Nginx.reload_command(), "nginx -s reload");
    }

    #[test]
    fn caddy_config_format_supported() {
        // Caddy 支持 caddy fmt 格式化辅助；其余引擎无等价工具
        assert!(WebServerEngine::Caddy.supports_config_format());
        assert!(!WebServerEngine::Nginx.supports_config_format());
        assert!(!WebServerEngine::Apache.supports_config_format());
    }

    #[tokio::test]
    async fn caddy_detect_routes_through_runner() {
        // which caddy 成功 → installed；版本命令走 caddy version
        let runner = std::sync::Arc::new(ScriptedRunner::with_ok(&["/usr/bin/caddy"]));
        let m = WebServerNativeManager::new(runner.clone());
        let info = m.detect(&WebServerEngine::Caddy).await;
        let programs = runner.programs();
        assert!(programs.iter().any(|p| p.starts_with("which caddy")));
        assert!(programs.iter().any(|p| p.starts_with("caddy version")));
        assert!(
            info.installed,
            "Caddy which-path detection should mark installed"
        );
        assert!(info.version.is_some());
    }

    #[tokio::test]
    async fn caddy_install_dispatch_uses_official_repo() {
        // install_caddy 应依次执行仓库配置命令 + 包安装（apt-get install -y caddy；Doc/33 D-1 统一用 apt-get）
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let m = WebServerNativeManager::new(runner.clone());
        // 假装未安装（which/candidates 均失败）
        // 这里直接调用 install_caddy 验证仓库命令序列
        let result = m.install_caddy().await;
        // 默认 Distro 在容器内为 Unknown → 走 Debian 分支仓库配置
        assert!(result.is_ok());
        let programs = runner.programs();
        assert!(programs.iter().any(|p| p.starts_with("apt-get update")));
        assert!(programs
            .iter()
            .any(|p| p.starts_with("apt-get install -y caddy")));
        // 官方源配置命令（gpg 密钥 / apt 源列表）也走了 runner
        assert!(programs.iter().any(|p| p
            .starts_with("gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg")));
    }
}
