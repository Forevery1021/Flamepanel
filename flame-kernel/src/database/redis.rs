use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{redis_layout, OsInfo, PackageManager, ServiceManager};
use async_trait::async_trait;

#[derive(Clone)]
pub struct RedisManager {
    service_name: String,
    config_file: String,
    /// 发行版相关包名（Doc/33 D-3：Debian `redis-server`，其余 `redis`）
    package_name: String,
    package_manager: PackageManager,
    service_manager: ServiceManager,
    runner: SharedCommandRunner,
    /// S7：Redis requirepass 密码（可选）。联链连接串生成用，仅内存流转。
    password: Option<String>,
}

impl RedisManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        let layout = redis_layout(&OsInfo::detected());
        Self {
            service_name: layout.service.into(),
            config_file: layout.config_file(),
            package_name: layout.package.into(),
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
            password: std::env::var("REDIS_PASSWORD")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }

    /// T16：覆盖 Redis 配置文件路径（默认按发行版探测，见 `redis_layout`）。
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

    /// Redis 密码（联链连接串生成用；仅服务层内存流转，不落日志）。
    ///
    /// 来源：运行时环境变量 `REDIS_PASSWORD`。
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
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
        // Doc/33 A-3：先校验再装包。redis.conf 按空白分词，含空白/引号的口令
        // 需按 Redis 字符串语法转义；此处收敛为可打印非空白 ASCII（且不含引号/反斜杠），
        // 既杜绝换行注入配置指令，也避免口令被截断。
        validate_redis_password(password)?;
        if self
            .package_manager
            .is_installed(&self.package_name)
            .await
            .unwrap_or(false)
        {
            return Err(AppError::BadRequest("Redis is already installed".into()));
        }

        self.package_manager.install(&self.package_name).await?;

        // Configure port
        if port != 6379 {
            // 逐参数 argv，不经 shell（Doc/33 §5-3：`sh -c` 清零）
            let _ = self
                .runner
                .run(&crate::application::execution_mode::PrivilegedCommand::new(
                    "sed",
                    vec![
                        "-i".into(),
                        format!("s/^port .*/port {port}/"),
                        self.config_file.clone(),
                    ],
                ))
                .await;
        }

        // Configure password
        // Doc/33 A-3：旧实现 `sh -c "echo 'requirepass {password}' >> file"`，
        // 口令含单引号即可 root 命令注入；改为 argv 传参的追加命令。
        if !password.is_empty() {
            let line = format!("requirepass {}", password);
            let _ = self
                .runner
                .run(&crate::database::append_line_command(
                    &line,
                    &self.config_file,
                ))
                .await;
        }

        self.service_manager.enable(&self.service_name).await.ok();
        self.service_manager.start(&self.service_name).await?;

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        self.service_manager.stop(&self.service_name).await.ok();
        self.service_manager.disable(&self.service_name).await.ok();
        // Doc/33 D-3：按发行版包管理器卸载（旧实现 `apt || yum || apk` 盲试且包名不匹配）；
        // best-effort，失败不阻断
        let _ = self.package_manager.uninstall(&self.package_name).await;
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
        // Doc/33 A-4：`CONFIG SET dir/dbfilename` 是经典的 Redis 任意写文件原语，
        // 仅放行运维调优类参数（纵深防御：当前仅 maxmemory 调用，未直接暴露 API）。
        if !REDIS_TUNABLE_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
            return Err(AppError::ValidationError(format!(
                "不允许修改的 Redis 配置项: {key}"
            )));
        }
        crate::database::validate_secret(value, "配置值")?;
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

/// 允许经面板 `CONFIG SET` 调整的 Redis 参数白名单。
const REDIS_TUNABLE_KEYS: &[&str] = &[
    "maxmemory",
    "maxmemory-policy",
    "maxclients",
    "timeout",
    "tcp-keepalive",
    "appendonly",
    "appendfsync",
    "save",
    "slowlog-log-slower-than",
    "slowlog-max-len",
];

/// Redis 口令校验：空（不设口令）或 1–128 位可打印 ASCII，不含空白、引号、反斜杠。
fn validate_redis_password(password: &str) -> Result<(), AppError> {
    if password.is_empty() {
        return Ok(());
    }
    let ok = password.len() <= 128
        && password
            .chars()
            .all(|c| c.is_ascii_graphic() && !matches!(c, '"' | '\'' | '\\'));
    if !ok {
        return Err(AppError::ValidationError(
            "Redis 密码仅支持 1-128 位可打印 ASCII 字符，且不能包含空格、引号或反斜杠".into(),
        ));
    }
    Ok(())
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

    #[test]
    fn redis_password_validation() {
        assert!(validate_redis_password("").is_ok());
        assert!(validate_redis_password("Str0ng!P@ss#2026").is_ok());
        for bad in [
            "a b",
            "x'y",
            "x\"y",
            "x\\y",
            "x\nrename-command CONFIG x",
            "密码",
        ] {
            assert!(validate_redis_password(bad).is_err(), "{bad:?} 应被拒绝");
        }
    }

    #[tokio::test]
    async fn set_config_blocks_file_write_primitives() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = RedisManager::new(runner.clone());
        assert!(mgr.set_config("dir", "/root/.ssh").await.is_err());
        assert!(mgr
            .set_config("dbfilename", "authorized_keys")
            .await
            .is_err());
        assert!(runner.programs().is_empty());
        assert!(mgr.set_config("maxmemory", "1048576").await.is_ok());
    }

    #[tokio::test]
    async fn install_custom_port_uses_argv_not_shell() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = RedisManager::new(runner.clone());
        let _ = mgr.install(None, 6380, "").await;
        let programs = runner.programs();
        assert!(
            programs
                .iter()
                .any(|p| p.starts_with("sed -i s/^port .*/port 6380/ ")),
            "{programs:?}"
        );
        assert!(
            !programs.iter().any(|p| p.starts_with("sh ")),
            "{programs:?}"
        );
    }

    #[tokio::test]
    async fn install_rejects_injection_before_touching_packages() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = RedisManager::new(runner.clone());
        let err = mgr.install(None, 6379, "x'; touch /tmp/p; echo '").await;
        assert!(err.is_err());
        assert!(runner.programs().is_empty(), "校验失败时不得执行任何命令");
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
