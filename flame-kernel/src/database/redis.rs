use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{PackageManager, ServiceManager};
use async_trait::async_trait;

pub struct RedisManager {
    service_name: String,
    config_file: String,
    package_manager: PackageManager,
    service_manager: ServiceManager,
    runner: SharedCommandRunner,
}

impl RedisManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            service_name: "redis-server".into(),
            config_file: "/etc/redis/redis.conf".into(),
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
        }
    }

    /// T16：覆盖 Redis 配置文件路径（默认 `/etc/redis/redis.conf`）。
    pub fn with_config_file(mut self, path: impl Into<String>) -> Self {
        self.config_file = path.into();
        self
    }

    /// 便捷：默认嵌入式执行器（行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 经统一端口执行 `sh -c <script>`（Phase A1：收敛到 PrivilegedCommandRunner）。
    async fn sh(&self, script: &str) -> crate::application::execution_mode::CommandOutput {
        let out = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "sh",
                vec!["-c".into(), script.to_string()],
            ))
            .await;
        out.unwrap_or_default()
    }

    /// 经统一端口执行 `redis-cli <args...>`（Phase A1：收敛到 PrivilegedCommandRunner）。
    async fn redis_cli(
        &self,
        args: Vec<String>,
    ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
        let out = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "redis-cli",
                args,
            ))
            .await?;
        Ok(out)
    }
}

#[async_trait]
impl NativeDbManager for RedisManager {
    async fn install(
        &self,
        _version: Option<&str>,
        port: i32,
        password: &str,
    ) -> Result<(), AppError> {
        if self
            .package_manager
            .is_installed("redis-server")
            .await
            .unwrap_or(false)
        {
            return Err(AppError::BadRequest("Redis is already installed".into()));
        }

        self.package_manager.install("redis-server").await?;

        // Configure port
        if port != 6379 {
            let script = format!("sed -i 's/^port .*/port {}/' {}", port, self.config_file);
            self.sh(&script).await;
        }

        // Configure password
        if !password.is_empty() {
            let script = format!("echo 'requirepass {}' >> {}", password, self.config_file);
            self.sh(&script).await;
        }

        self.service_manager.enable("redis-server").await.ok();
        self.service_manager.start("redis-server").await?;

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        self.service_manager.stop("redis-server").await.ok();
        self.service_manager.disable("redis-server").await.ok();
        // 经统一端口卸载（多包管理器回退，best-effort，失败不阻断）
        let _ = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "sh",
                vec![
                    "-c".into(),
                    "apt remove -y redis-server 2>/dev/null || yum remove -y redis 2>/dev/null || apk del redis 2>/dev/null".into(),
                ],
            ))
            .await;
        Ok(())
    }

    async fn start(&self) -> Result<(), AppError> {
        self.service_manager.start(&self.service_name).await
    }

    async fn stop(&self) -> Result<(), AppError> {
        self.service_manager.stop(&self.service_name).await
    }

    async fn restart(&self) -> Result<(), AppError> {
        self.service_manager.restart(&self.service_name).await
    }

    async fn is_running(&self) -> Result<bool, AppError> {
        self.service_manager.is_running(&self.service_name).await
    }

    async fn get_version(&self) -> Result<String, AppError> {
        let out = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "redis-server",
                vec!["--version".into()],
            ))
            .await?;
        let s = out.stdout;
        // redis-server x.y.z
        for word in s.split_whitespace() {
            if word.contains('.') && word.chars().any(|c| c.is_ascii_digit()) {
                return Ok(word.trim_matches(',').to_string());
            }
        }
        Ok("unknown".into())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError> {
        let _ = self
            .redis_cli(vec![
                "CONFIG".into(),
                "SET".into(),
                key.to_string(),
                value.to_string(),
            ])
            .await?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError> {
        let out = self
            .redis_cli(vec!["CONFIG".into(), "GET".into(), key.to_string()])
            .await?;
        let s = out.stdout.trim().to_string();
        if s.is_empty() {
            return Ok(None);
        }
        // redis-cli CONFIG GET returns key\nvalue
        let lines: Vec<&str> = s.lines().collect();
        if lines.len() >= 2 {
            Ok(Some(lines[1].to_string()))
        } else {
            Ok(Some(s))
        }
    }
}

impl RedisManager {
    pub async fn flush_all(&self) -> Result<(), AppError> {
        let _ = self.redis_cli(vec!["FLUSHALL".into()]).await?;
        Ok(())
    }

    pub async fn info(&self) -> Result<String, AppError> {
        let out = self.redis_cli(vec!["INFO".into()]).await?;
        Ok(out.stdout)
    }

    pub async fn set_max_memory(&self, max_mb: usize) -> Result<(), AppError> {
        self.set_config("maxmemory", &format!("{}", max_mb * 1024 * 1024))
            .await
    }
}
impl Default for RedisManager {
    fn default() -> Self {
        Self::embedded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// 记录型 mock runner：记录收到的命令，统一返回成功。
    struct RecordingRunner {
        calls: Mutex<Vec<String>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
        fn programs(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl PrivilegedCommandRunner for RecordingRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            Ok(CommandOutput {
                stdout: "redis-server 7.2.4".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    #[tokio::test]
    async fn test_redis_routes_through_runner() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = RedisManager::new(runner.clone());

        let _ = mgr.get_version().await.unwrap();
        let _ = mgr.get_config("maxmemory").await.unwrap();
        let _ = mgr.info().await.unwrap();

        let programs = runner.programs();
        assert!(programs
            .iter()
            .any(|p| p.starts_with("redis-server --version")));
        assert!(programs
            .iter()
            .any(|p| p.starts_with("redis-cli CONFIG GET")));
        assert!(programs.iter().any(|p| p.starts_with("redis-cli INFO")));
    }
}
