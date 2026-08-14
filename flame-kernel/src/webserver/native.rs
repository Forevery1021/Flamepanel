use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::infrastructure::os::{PackageManager, ServiceManager};
use crate::webserver::engine::WebServerEngine;
use serde::Serialize;
use std::collections::HashSet;

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

pub struct WebServerNativeManager {
    package_manager: PackageManager,
    service_manager: ServiceManager,
    /// 特权命令执行器：版本 / which / 端口扫描等检测命令统一经此端口执行
    /// （Phase A1 扩展：`execution_mode=embedded|agent` 分离模式）。
    runner: SharedCommandRunner,
}

impl WebServerNativeManager {
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
        let binary = engine.binary_name();
        let binary_path = self.which(binary).await;
        let installed = binary_path.is_some();
        let package_installed = self
            .package_manager
            .is_installed(engine.package_name())
            .await
            .unwrap_or(false);
        let version = if installed {
            match self.get_version(engine).await {
                Ok(v) => Some(v),
                Err(_) => self
                    .package_manager
                    .get_version(engine.package_name())
                    .await
                    .ok()
                    .or_else(|| Some("unknown".into())),
            }
        } else {
            None
        };
        let running = if installed {
            self.service_manager
                .is_running(engine.service_name())
                .await
                .unwrap_or(false)
        } else {
            false
        };
        let enabled = self
            .service_manager
            .is_enabled(engine.service_name())
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
        if self.which(engine.binary_name()).await.is_some() {
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
        self.package_manager.install(&pkg).await?;
        self.service_manager.enable(engine.service_name()).await?;
        self.service_manager.start(engine.service_name()).await?;
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
        self.service_manager.stop(engine.service_name()).await.ok();
        self.service_manager
            .disable(engine.service_name())
            .await
            .ok();
        self.package_manager.uninstall(engine.package_name()).await
    }

    /// systemd 开机自启开关
    pub async fn set_autostart(
        &self,
        engine: &WebServerEngine,
        enabled: bool,
    ) -> Result<(), AppError> {
        if enabled {
            self.service_manager.enable(engine.service_name()).await
        } else {
            self.service_manager.disable(engine.service_name()).await
        }
    }

    /// 检测版本（各引擎 CLI 输出格式不同，统一解析）
    async fn get_version(&self, engine: &WebServerEngine) -> Result<String, AppError> {
        let binary = engine.binary_name();
        let args: &[&str] = match engine {
            WebServerEngine::Caddy => &["version"],
            _ => &["-v"],
        };
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
        let re =
            regex::Regex::new(r"(nginx|openresty|Apache|httpd|caddy|v)?[/ ]?(\d+\.\d+(\.\d+)?)")
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
}

impl Default for WebServerNativeManager {
    fn default() -> Self {
        Self::embedded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
