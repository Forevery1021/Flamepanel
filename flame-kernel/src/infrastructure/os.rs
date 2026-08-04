use crate::core::error::AppError;

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
    pub async fn detect_distro() -> DistroType {
        let output = tokio::process::Command::new("sh")
            .args([
                "-c",
                "cat /etc/os-release 2>/dev/null || cat /etc/*release 2>/dev/null",
            ])
            .output()
            .await;
        match output {
            Ok(out) => {
                let c = String::from_utf8_lossy(&out.stdout);
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
            Err(_) => DistroType::Unknown("unknown".into()),
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

pub struct ServiceManager;

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

impl ServiceManager {
    /// 构造 systemctl 命令（免密码）：
    /// - root 直接执行
    /// - 非 root 使用 `sudo -n`（non-interactive，绝不触发密码框/polkit 认证）
    /// - sudo 不存在时回退直接调用（会失败并返回清晰错误）
    fn systemctl_cmd(action: &str, name: &str) -> tokio::process::Command {
        let mut cmd = if is_root_process() {
            tokio::process::Command::new("systemctl")
        } else {
            let mut c = tokio::process::Command::new("sudo");
            c.arg("-n").arg("systemctl");
            c
        };
        cmd.arg(action).arg(name);
        cmd
    }

    /// 服务状态变更（start/stop/restart/enable/disable）：免密码 systemctl
    async fn control(action: &str, name: &str) -> Result<(), AppError> {
        let output = Self::systemctl_cmd(action, name)
            .output()
            .await
            .map_err(|e| AppError::internal(format!("systemctl {} failed: {}", action, e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
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

    pub async fn start(name: &str) -> Result<(), AppError> {
        Self::control("start", name).await
    }

    pub async fn stop(name: &str) -> Result<(), AppError> {
        Self::control("stop", name).await
    }

    pub async fn restart(name: &str) -> Result<(), AppError> {
        Self::control("restart", name).await
    }

    pub async fn enable(name: &str) -> Result<(), AppError> {
        Self::control("enable", name).await
    }

    pub async fn disable(name: &str) -> Result<(), AppError> {
        Self::control("disable", name).await
    }

    pub async fn is_running(name: &str) -> Result<bool, AppError> {
        let output = Self::systemctl_cmd("is-active", name).output().await;
        match output {
            Ok(o) => {
                if o.status.success() {
                    Ok(true)
                } else {
                    // systemctl 失败（无权限/未安装）时回退 pgrep 探测
                    let out = tokio::process::Command::new("pgrep")
                        .arg("-x")
                        .arg(name)
                        .output()
                        .await;
                    Ok(out.map(|o| o.status.success()).unwrap_or(false))
                }
            }
            Err(_) => {
                let out = tokio::process::Command::new("pgrep")
                    .arg("-x")
                    .arg(name)
                    .output()
                    .await;
                Ok(out.map(|o| o.status.success()).unwrap_or(false))
            }
        }
    }
}

pub struct PackageManager;

impl PackageManager {
    pub async fn install(pkg: &str) -> Result<String, AppError> {
        let distro = OsInfo::detect_distro().await;
        let cmd = OsInfo::package_install_cmd(&distro, pkg);
        let out = tokio::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Package install failed: {}", e)))?;
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        let e = String::from_utf8_lossy(&out.stderr).to_string();
        if !out.status.success() {
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

    pub async fn is_installed(pkg: &str) -> Result<bool, AppError> {
        let distro = OsInfo::detect_distro().await;
        let (bin, args) = match distro {
            DistroType::Ubuntu | DistroType::Debian => ("dpkg", vec!["-l", pkg]),
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => ("rpm", vec!["-q", pkg]),
            DistroType::Alpine => ("apk", vec!["info", "-e", pkg]),
            DistroType::Unknown(_) => ("dpkg", vec!["-l", pkg]),
        };
        let out = tokio::process::Command::new(bin).args(&args).output().await;
        Ok(out.map(|o| o.status.success()).unwrap_or(false))
    }

    pub async fn uninstall(pkg: &str) -> Result<(), AppError> {
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
        let out = tokio::process::Command::new(cmd)
            .args(&args)
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Package uninstall failed: {}", e)))?;
        if !out.status.success() {
            let e = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::internal(format!(
                "Failed to uninstall {}: {}",
                pkg, e
            )));
        }
        Ok(())
    }

    pub async fn get_version(pkg: &str) -> Result<String, AppError> {
        let distro = OsInfo::detect_distro().await;
        let args: Vec<&str> = match distro {
            DistroType::Ubuntu | DistroType::Debian => vec!["-l", pkg],
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                vec!["-q", "--queryformat", "%{VERSION}", pkg]
            }
            DistroType::Alpine => vec!["info", pkg],
            DistroType::Unknown(_) => vec!["-l", pkg],
        };
        let out = tokio::process::Command::new(
            if distro == DistroType::CentOS
                || distro == DistroType::RHEL
                || distro == DistroType::Fedora
            {
                "rpm"
            } else if distro == DistroType::Alpine {
                "apk"
            } else {
                "dpkg"
            },
        )
        .args(&args)
        .output()
        .await
        .map_err(|e| AppError::internal(format!("Version check failed: {}", e)))?;
        let s = String::from_utf8_lossy(&out.stdout).to_string();
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
