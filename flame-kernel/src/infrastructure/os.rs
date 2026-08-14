use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub enum DistroType {
    Debian,
    Ubuntu,
    CentOS,
    RHEL,
    Fedora,
    Alpine,
    Unknown(String),
}

pub struct OsInfo;

impl OsInfo {
    /// 检测当前发行版（Phase A1 收尾：不再 spawn 外部命令，直接读取 `/etc/os-release`）。
    ///
    /// 此前用 `sh -c "cat /etc/os-release ..."` 执行外部命令，属于 A1 剩余的直接命令
    /// 路径。改为标准库读取文件后，无命令注入面，Agent/Embedded 两种模式行为一致，
    /// 也不需在 Agent 白名单放行任意 shell。
    pub async fn detect_distro() -> DistroType {
        // 保持 async 以兼容既有调用点；读取为同步 I/O，先让出一次执行权
        tokio::task::yield_now().await;
        let c = read_release_info();
        if c.contains("Ubuntu") {
            DistroType::Ubuntu
        } else if c.contains("Debian") {
            DistroType::Debian
        } else if c.contains("CentOS") {
            DistroType::CentOS
        } else if c.contains("Red Hat") || c.contains("RHEL") {
            DistroType::RHEL
        } else if c.contains("Fedora") {
            DistroType::Fedora
        } else if c.contains("Alpine") {
            DistroType::Alpine
        } else {
            DistroType::Unknown("unknown".into())
        }
    }

    pub fn package_install_cmd(distro: &DistroType, pkg: &str) -> Vec<String> {
        match distro {
            DistroType::Ubuntu | DistroType::Debian => {
                vec!["apt".into(), "install".into(), "-y".into(), pkg.into()]
            }
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                vec!["yum".into(), "install".into(), "-y".into(), pkg.into()]
            }
            DistroType::Alpine => {
                vec!["apk".into(), "add".into(), pkg.into()]
            }
            DistroType::Unknown(_) => {
                vec!["apt".into(), "install".into(), "-y".into(), pkg.into()]
            }
        }
    }
}

/// 读取发行版标识文件内容（供 `detect_distro` 判断）。
///
/// 依次尝试 `/etc/os-release` 与 `/etc/*release`（回退），找不到时返回空串。
fn read_release_info() -> String {
    let os_release = "/etc/os-release";
    if let Ok(c) = std::fs::read_to_string(os_release) {
        if !c.trim().is_empty() {
            return c;
        }
    }
    // 回退：/etc/*release（如 /etc/redhat-release /etc/debian_version 等）
    if let Ok(entries) = std::fs::read_dir("/etc") {
        let mut candidates: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let n = name.to_string_lossy();
                e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && (n.ends_with("-release") || n.ends_with("_version") || n == "debian_version")
            })
            .collect();
        candidates.sort_by_key(|e| e.file_name());
        for entry in candidates {
            if let Ok(c) = std::fs::read_to_string(entry.path()) {
                if !c.trim().is_empty() {
                    return c;
                }
            }
        }
    }
    String::new()
}

/// 当前进程是否以 root 运行（供各模块免密码 systemctl 判断）
pub fn is_root_process() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                let l = l.trim();
                l.strip_prefix("Uid:").and_then(|v| {
                    v.split_whitespace()
                        .next()
                        .and_then(|id| id.parse::<u32>().ok())
                })
            })
        })
        .map(|uid| uid == 0)
        .unwrap_or(false)
}

/// 系统服务管理器（systemctl）：经 `PrivilegedCommandRunner` 执行（Phase A1 扩展）。
///
/// 与 `FirewallManager` 一致，所有 systemctl / pgrep 调用都收敛到统一特权命令执行端口，
/// 由组合根按 `execution_mode=embedded|agent` 注入具体实现。
pub struct ServiceManager {
    runner: SharedCommandRunner,
}

impl ServiceManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }

    /// 便捷：默认嵌入式执行器（本地直接执行，行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 经执行器运行一条 systemctl 命令并校验成功。
    async fn systemctl(&self, action: &str, name: &str) -> Result<(), AppError> {
        let out = self
            .runner
            .run(
                &PrivilegedCommand::new("systemctl", vec![action.into(), name.into()])
                    .prefer_root(),
            )
            .await?;
        if !out.success() {
            let stderr = out.stderr.trim().to_string();
            let hint = if is_root_process() {
                format!("systemctl {} {}: {}", action, name, stderr)
            } else {
                format!(
                    "systemctl {} {} failed (panel runs as non-root; configure passwordless sudo or run panel as root): {}",
                    action, name, stderr
                )
            };
            return Err(AppError::internal(hint));
        }
        Ok(())
    }

    /// 经执行器运行 systemctl is-active，失败时回退 pgrep 探测。
    async fn is_active(&self, name: &str) -> Result<bool, AppError> {
        let out = self
            .runner
            .run(
                &PrivilegedCommand::new("systemctl", vec!["is-active".into(), name.into()])
                    .prefer_root(),
            )
            .await;
        match out {
            Ok(o) => {
                if o.success() {
                    Ok(true)
                } else {
                    // systemctl 失败（无权限/未安装）时回退 pgrep 探测
                    self.pgrep(name).await
                }
            }
            Err(_) => self.pgrep(name).await,
        }
    }

    /// pgrep 探测进程是否存活。
    async fn pgrep(&self, name: &str) -> Result<bool, AppError> {
        let out = self
            .runner
            .run(&PrivilegedCommand::new(
                "pgrep",
                vec!["-x".into(), name.into()],
            ))
            .await;
        Ok(out.map(|o| o.success()).unwrap_or(false))
    }

    pub async fn start(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("start", name).await
    }

    pub async fn stop(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("stop", name).await
    }

    pub async fn restart(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("restart", name).await
    }

    pub async fn enable(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("enable", name).await
    }

    pub async fn disable(&self, name: &str) -> Result<(), AppError> {
        self.systemctl("disable", name).await
    }

    pub async fn is_running(&self, name: &str) -> Result<bool, AppError> {
        self.is_active(name).await
    }

    /// 免密码 systemctl is-enabled 探测（root 直接；非 root 走 runner 的 prefer_root）。
    pub async fn is_enabled(&self, name: &str) -> Result<bool, AppError> {
        let out = self
            .runner
            .run(
                &PrivilegedCommand::new("systemctl", vec!["is-enabled".into(), name.into()])
                    .prefer_root(),
            )
            .await;
        match out {
            Ok(o) => {
                let s = o.stdout.clone();
                Ok(o.success() && (s.contains("enabled") || s.contains("static")))
            }
            Err(_) => Ok(false),
        }
    }
}

/// 系统包管理器（apt/yum/dnf/apk 等）：经 `PrivilegedCommandRunner` 执行（Phase A1 扩展）。
pub struct PackageManager {
    runner: SharedCommandRunner,
}

impl PackageManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self { runner }
    }

    /// 便捷：默认嵌入式执行器（本地直接执行，行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 经执行器运行一条包管理命令，返回合并输出。
    async fn run_cmd(
        &self,
        program: &str,
        args: Vec<String>,
    ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
        self.runner
            .run(&PrivilegedCommand::new(program, args).prefer_root())
            .await
    }

    pub async fn install(&self, pkg: &str) -> Result<String, AppError> {
        let distro = OsInfo::detect_distro().await;
        let cmd = OsInfo::package_install_cmd(&distro, pkg);
        let out = self.run_cmd(&cmd[0], cmd[1..].to_vec()).await?;
        let s = out.stdout.clone();
        let e = out.stderr.clone();
        if !out.success() {
            return Err(AppError::internal(format!(
                "Failed to install {}: {}",
                pkg, e
            )));
        }
        Ok(if s.len() > 200 {
            format!("{}... ({} chars)", &s[..200], s.len())
        } else {
            s
        })
    }

    pub async fn is_installed(&self, pkg: &str) -> Result<bool, AppError> {
        let distro = OsInfo::detect_distro().await;
        let (bin, args) = match distro {
            DistroType::Ubuntu | DistroType::Debian => ("dpkg", vec!["-l".into(), pkg.into()]),
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                ("rpm", vec!["-q".into(), pkg.into()])
            }
            DistroType::Alpine => ("apk", vec!["info".into(), "-e".into(), pkg.into()]),
            DistroType::Unknown(_) => ("dpkg", vec!["-l".into(), pkg.into()]),
        };
        let out = self.run_cmd(bin, args).await;
        Ok(out.map(|o| o.success()).unwrap_or(false))
    }

    pub async fn uninstall(&self, pkg: &str) -> Result<(), AppError> {
        let distro = OsInfo::detect_distro().await;
        let (cmd, args): (&str, Vec<String>) = match distro {
            DistroType::Ubuntu | DistroType::Debian => {
                ("apt-get", vec!["remove".into(), "-y".into(), pkg.into()])
            }
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                ("dnf", vec!["remove".into(), "-y".into(), pkg.into()])
            }
            DistroType::Alpine => ("apk", vec!["del".into(), pkg.into()]),
            DistroType::Unknown(_) => ("apt-get", vec!["remove".into(), "-y".into(), pkg.into()]),
        };
        let out = self.run_cmd(cmd, args).await?;
        if !out.success() {
            let e = out.stderr.clone();
            return Err(AppError::internal(format!(
                "Failed to uninstall {}: {}",
                pkg, e
            )));
        }
        Ok(())
    }

    pub async fn get_version(&self, pkg: &str) -> Result<String, AppError> {
        let distro = OsInfo::detect_distro().await;
        let args: Vec<&str> = match distro {
            DistroType::Ubuntu | DistroType::Debian => vec!["-l", pkg],
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                vec!["-q", "--queryformat", "%{VERSION}", pkg]
            }
            DistroType::Alpine => vec!["info", pkg],
            DistroType::Unknown(_) => vec!["-l", pkg],
        };
        let bin = if distro == DistroType::CentOS
            || distro == DistroType::RHEL
            || distro == DistroType::Fedora
        {
            "rpm"
        } else if distro == DistroType::Alpine {
            "apk"
        } else {
            "dpkg"
        };
        let out = self
            .run_cmd(bin, args.iter().map(|s| s.to_string()).collect())
            .await?;
        let s = out.stdout.clone();
        for line in s.lines() {
            if line.starts_with("ii") && line.contains(pkg) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return Ok(parts[2].to_string());
                }
            }
        }
        Ok("unknown".into())
    }
}

// ─── 六边形端口实现 ─────────────────────────────────────────────────────────

/// `ServiceManagerPort` 端口实现：持有 runner，委托给 `ServiceManager`。
pub struct DefaultServiceManagerPort {
    manager: ServiceManager,
}

impl DefaultServiceManagerPort {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            manager: ServiceManager::new(runner),
        }
    }

    /// 便捷：默认嵌入式执行器。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }
}

#[async_trait]
impl crate::application::app_store_ports::ServiceManagerPort for DefaultServiceManagerPort {
    async fn start(&self, name: &str) -> Result<(), AppError> {
        self.manager.start(name).await
    }
    async fn stop(&self, name: &str) -> Result<(), AppError> {
        self.manager.stop(name).await
    }
    async fn restart(&self, name: &str) -> Result<(), AppError> {
        self.manager.restart(name).await
    }
    async fn enable(&self, name: &str) -> Result<(), AppError> {
        self.manager.enable(name).await
    }
    async fn disable(&self, name: &str) -> Result<(), AppError> {
        self.manager.disable(name).await
    }
    async fn is_running(&self, name: &str) -> Result<bool, AppError> {
        self.manager.is_running(name).await
    }
}

/// `PackageManagerPort` 端口实现：持有 runner，委托给 `PackageManager`。
pub struct DefaultPackageManagerPort {
    manager: PackageManager,
}

impl DefaultPackageManagerPort {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            manager: PackageManager::new(runner),
        }
    }

    /// 便捷：默认嵌入式执行器。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }
}

#[async_trait]
impl crate::application::app_store_ports::PackageManagerPort for DefaultPackageManagerPort {
    async fn install(&self, pkg: &str) -> Result<String, AppError> {
        self.manager.install(pkg).await
    }
    async fn is_installed(&self, pkg: &str) -> Result<bool, AppError> {
        self.manager.is_installed(pkg).await
    }
    async fn uninstall(&self, pkg: &str) -> Result<(), AppError> {
        self.manager.uninstall(pkg).await
    }
    async fn get_version(&self, pkg: &str) -> Result<String, AppError> {
        self.manager.get_version(pkg).await
    }
}
