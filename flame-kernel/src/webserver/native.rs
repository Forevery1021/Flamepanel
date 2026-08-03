use crate::core::error::AppError;
use crate::infrastructure::os::{PackageManager, ServiceManager};
use crate::webserver::engine::WebServerEngine;
use serde::Serialize;
use std::collections::HashSet;
use std::process::Stdio;

/// 原生 Web 服务器检测信息（1Panel 风格：安装状态 / 版本 / 服务 / 端口）
#[derive(Debug, Clone, Serialize)]
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

pub struct WebServerNativeManager;

impl WebServerNativeManager {
    pub fn new() -> Self {
        Self
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
        let binary = engine.binary_name();
        let binary_path = Self::which(binary).await;
        let installed = binary_path.is_some();
        let package_installed = PackageManager::is_installed(engine.package_name())
            .await
            .unwrap_or(false);
        let version = if installed {
            match self.get_version(engine).await {
                Ok(v) => Some(v),
                Err(_) => PackageManager::get_version(engine.package_name())
                    .await
                    .ok()
                    .or_else(|| Some("unknown".into())),
            }
        } else {
            None
        };
        let running = if installed {
            ServiceManager::is_running(engine.service_name())
                .await
                .unwrap_or(false)
        } else {
            false
        };
        let enabled = Self::is_service_enabled(engine.service_name()).await;
        let listening_ports = Self::listening_ports()
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
            service_name: Some(engine.service_name().into()),
            running,
            enabled,
            binary_path,
            config_path: engine.default_config_path().into(),
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
        if Self::which(engine.binary_name()).await.is_some() {
            return Err(AppError::BadRequest(format!(
                "{} is already installed",
                engine.as_str()
            )));
        }
        let pkg = if let Some(ver) = version {
            if ver.is_empty() {
                engine.package_name().to_string()
            } else {
                format!("{}={}", engine.package_name(), ver)
            }
        } else {
            engine.package_name().to_string()
        };
        PackageManager::install(&pkg).await?;
        ServiceManager::enable(engine.service_name()).await?;
        ServiceManager::start(engine.service_name()).await?;
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

    /// 原生卸载：停止 + 禁用 + 移除包
    pub async fn uninstall(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        ServiceManager::stop(engine.service_name()).await.ok();
        ServiceManager::disable(engine.service_name()).await.ok();
        PackageManager::uninstall(engine.package_name()).await
    }

    /// systemd 开机自启开关
    pub async fn set_autostart(&self, engine: &WebServerEngine, enabled: bool) -> Result<(), AppError> {
        if enabled {
            ServiceManager::enable(engine.service_name()).await
        } else {
            ServiceManager::disable(engine.service_name()).await
        }
    }

    /// 检测版本（各引擎 CLI 输出格式不同，统一解析）
    async fn get_version(&self, engine: &WebServerEngine) -> Result<String, AppError> {
        let binary = engine.binary_name();
        let args: &[&str] = match engine {
            WebServerEngine::Caddy => &["version"],
            _ => &["-v"],
        };
        let out = tokio::process::Command::new(binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Version check failed: {}", e)))?;
        let combined = format!(
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        // nginx/1.18.0 (Ubuntu)
        // nginx version: nginx/1.27.0
        // Server version: Apache/2.4.41
        // openresty/1.19.3.2
        // v2.7.6 h1:xxxx
        let re = regex::Regex::new(
            r"(nginx|openresty|Apache|httpd|caddy|v)?[/ ]?(\d+\.\d+(\.\d+)?)",
        )
        .map_err(|e| AppError::internal(format!("Regex error: {}", e)))?;
        if let Some(caps) = re.captures(&combined) {
            if let Some(ver) = caps.get(2) {
                return Ok(ver.as_str().to_string());
            }
        }
        Err(AppError::internal("Version not detected"))
    }

    async fn which(binary: &str) -> Option<String> {
        let out = tokio::process::Command::new("sh")
            .args(["-c", &format!("command -v {} 2>/dev/null", binary)])
            .output()
            .await
            .ok()?;
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
        None
    }

    async fn is_service_enabled(service: &str) -> bool {
        let out = tokio::process::Command::new("systemctl")
            .args(["is-enabled", service])
            .output()
            .await;
        match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                o.status.success() && (s.contains("enabled") || s.contains("static"))
            }
            Err(_) => false,
        }
    }
    /// 扫描当前监听端口（优先 ss，回退 netstat）
    async fn listening_ports() -> HashSet<u16> {
        let mut ports = HashSet::new();
        for cmd in [["ss", "-tln"], ["netstat", "-tln"]] {
            let out = tokio::process::Command::new(cmd[0])
                .args(&cmd[1..])
                .output()
                .await;
            if let Ok(o) = out {
                if !o.status.success() {
                    continue;
                }
                let s = String::from_utf8_lossy(&o.stdout);
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
}

impl Default for WebServerNativeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn detect_all_returns_five() {
        let m = WebServerNativeManager::new();
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
}
