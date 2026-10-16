use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{mysql_layout, OsInfo, PackageManager, ServiceManager};
use async_trait::async_trait;

#[derive(Clone)]
pub struct MySqlManager {
    pub service_name: String,
    pub config_file: String,
    /// 发行版相关包名（Doc/33 P-3：Debian/RHEL `mysql-server`，Alpine/SUSE/Arch `mariadb`）
    pub package_name: String,
    pub package_manager: PackageManager,
    pub service_manager: ServiceManager,
    runner: SharedCommandRunner,
    /// S7：MySQL root 密码（可选）。经 `MYSQL_PWD` 环境变量传递，避免落入 argv/`ps` 可见。
    root_password: Option<String>,
}

/// 校验 MySQL 标识符（库名/用户名），防止 SQL 注入。
/// 仅允许 ASCII 字母数字、下划线、美元符，长度 <= 64。
/// MySQL 内置系统库（`SHOW DATABASES` 输出中需隐藏的条目，含表头 `Database`）。
const MYSQL_SYSTEM_DATABASES: &[&str] = &[
    "Database",
    "information_schema",
    "performance_schema",
    "mysql",
    "sys",
];

fn validate_ident<'a>(s: &'a str, what: &str) -> Result<&'a str, AppError> {
    if s.is_empty()
        || s.len() > 64
        || !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        return Err(AppError::BadRequest(format!(
            "invalid MySQL {what}: {:?}",
            s
        )));
    }
    Ok(s)
}

/// 校验 MySQL 字符集，仅允许白名单枚举，防止注入。
fn validate_charset(cs: &str) -> Result<&str, AppError> {
    if matches!(cs, "utf8mb4" | "utf8" | "latin1" | "ascii" | "utf8mb3") {
        Ok(cs)
    } else {
        Err(AppError::BadRequest(format!(
            "invalid MySQL charset: {:?}",
            cs
        )))
    }
}

/// 校验 MySQL host，仅允许 localhost/%/合法 IPv4。
fn validate_host(h: &str) -> Result<&str, AppError> {
    if h == "localhost" || h == "%" {
        return Ok(h);
    }
    // 合法 IPv4
    let parts: Vec<&str> = h.split('.').collect();
    if parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 3
                && p.chars().all(|c| c.is_ascii_digit())
                && p.parse::<u8>().is_ok()
        })
    {
        return Ok(h);
    }
    Err(AppError::BadRequest(format!("invalid MySQL host: {:?}", h)))
}

impl MySqlManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        let layout = mysql_layout(&OsInfo::detected());
        Self {
            service_name: layout.service.into(),
            config_file: layout.config_file(),
            package_name: layout.package.into(),
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
            root_password: std::env::var("MYSQL_ROOT_PASSWORD")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }

    /// T16：覆盖 MySQL 配置文件路径（默认按发行版探测，见 `mysql_layout`）。
    pub fn with_config_file(mut self, path: impl Into<String>) -> Self {
        self.config_file = path.into();
        self
    }

    /// S7：显式设置 root 密码（经 `MYSQL_PWD` 环境变量传递）。
    pub fn with_root_password(mut self, password: impl Into<String>) -> Self {
        self.root_password = Some(password.into());
        self
    }

    /// 便捷：默认嵌入式执行器（行为与重构前一致）。
    pub fn embedded() -> Self {
        Self::new(std::sync::Arc::new(
            crate::infrastructure::execution::EmbeddedCommandRunner,
        ))
    }

    /// 数据库实例的超级用户密码（联链连接串生成用；仅服务层内存流转，不落日志）。
    ///
    /// 来源优先级：运行时环境变量 `MYSQL_ROOT_PASSWORD`（面板自身部署凭据）。
    pub fn root_password(&self) -> Option<&str> {
        self.root_password.as_deref()
    }

    /// 经统一特权命令端口执行 `mysql -u root`，SQL 经 **stdin** 传入（Doc/33 P-2：
    /// 旧实现 `-e <sql>` 让 `CREATE USER … IDENTIFIED BY '<密码>'` 出现在 argv，
    /// 任何本地用户可经 `ps` / `/proc/<pid>/cmdline` 读到）。非 tty stdin 下 mysql
    /// 进入批处理模式，输出格式（制表符分隔 + 表头）与 `-e` 相同。
    /// S7：若配置了 root 密码，经 `MYSQL_PWD` 环境变量传递（避免密码落入 argv/`ps` 可见）。
    async fn exec_mysql(&self, sql: &str) -> Result<String, AppError> {
        let mut cmd = crate::application::execution_mode::PrivilegedCommand::new(
            "mysql",
            vec!["-u".into(), "root".into()],
        )
        .stdin(sql);
        if let Some(pwd) = &self.root_password {
            cmd = cmd.env("MYSQL_PWD", pwd.clone());
        }
        let out = self.runner.run(&cmd).await?;
        let stdout = out.stdout.trim().to_string();
        let stderr = out.stderr.trim().to_string();
        if !out.success() && !stderr.is_empty() {
            return Err(AppError::internal(format!("MySQL error: {}", stderr)));
        }
        Ok(stdout)
    }

    /// 追加一行到 MySQL 配置文件（Doc/33 A-2：内容经 argv 传入，不做 shell 拼接）。
    async fn append_config(&self, line: &str) {
        let _ = self
            .runner
            .run(&crate::database::append_line_command(
                line,
                &self.config_file,
            ))
            .await;
    }
}

#[async_trait]
impl NativeDbManager for MySqlManager {
    async fn install(
        &self,
        version: Option<&str>,
        port: i32,
        root_password: &str,
    ) -> Result<(), AppError> {
        // Doc/33 A-1：先校验再动包管理器，非法输入不留下半装状态
        crate::database::validate_secret(root_password, "root 密码")?;
        if let Some(ver) = version {
            if ver.is_empty()
                || ver.len() > 16
                || !ver.chars().all(|c| c.is_ascii_digit() || c == '.')
            {
                return Err(AppError::ValidationError(format!(
                    "非法 MySQL 版本号: {ver}"
                )));
            }
        }
        // 版本化包名只有 Debian 系 `mysql-server-<ver>` 存在；其他发行版忽略版本走仓库默认
        let pkg = match version {
            Some(ver) if self.package_name == "mysql-server" && self.service_name == "mysql" => {
                format!("mysql-server-{}", ver)
            }
            _ => self.package_name.clone(),
        };

        if self
            .package_manager
            .is_installed(&self.package_name)
            .await
            .unwrap_or(false)
        {
            return Err(AppError::BadRequest("MySQL is already installed".into()));
        }

        self.package_manager.install(&pkg).await?;

        // Start and enable service
        self.service_manager.enable(&self.service_name).await.ok();
        self.service_manager.start(&self.service_name).await?;

        // Wait for MySQL to be ready
        for _ in 0..30 {
            let out = self
                .runner
                .run(&crate::application::execution_mode::PrivilegedCommand::new(
                    "mysqladmin",
                    vec!["ping".into(), "-u".into(), "root".into()],
                ))
                .await;
            if let Ok(o) = out {
                if o.success() {
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Set root password if provided
        if !root_password.is_empty() {
            let sql = format!(
                "ALTER USER 'root'@'localhost' IDENTIFIED BY '{}'; FLUSH PRIVILEGES;",
                crate::database::mysql_escape_literal(root_password)
            );
            self.exec_mysql(&sql).await.ok();
        }

        // Change port if not default
        if port != 3306 {
            let port_line = format!("port = {}", port);
            self.append_config(&port_line).await;
            self.service_manager.restart(&self.service_name).await?;
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        self.service_manager.stop(&self.service_name).await.ok();
        self.service_manager.disable(&self.service_name).await.ok();
        // Doc/33 P-3：按发行版包管理器卸载（旧实现 `apt || yum || apk` 盲试）；best-effort
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
        let s = self.exec_mysql("SELECT VERSION();").await?;
        for line in s.lines() {
            if line.contains('.') && !line.contains("VERSION") {
                return Ok(line.trim().to_string());
            }
        }
        Ok("unknown".into())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError> {
        // Use mysql CLI to set global variable; key 走白名单，防止注入
        validate_ident(key, "variable key")?;
        crate::database::validate_secret(value, "配置值")?;
        let sql = format!(
            "SET GLOBAL {} = '{}';",
            key,
            crate::database::mysql_escape_literal(value)
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError> {
        validate_ident(key, "variable key")?;
        let sql = format!("SHOW VARIABLES LIKE '{}';", key);
        let out = self.exec_mysql(&sql).await?;
        for line in out.lines() {
            if line.contains(key) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Ok(Some(parts[1].trim().to_string()));
                }
            }
        }
        Ok(None)
    }
}

// Additional MySQL management functions
impl MySqlManager {
    pub async fn create_database(&self, db_name: &str, charset: &str) -> Result<(), AppError> {
        validate_ident(db_name, "database name")?;
        let charset = if charset.is_empty() {
            "utf8mb4"
        } else {
            validate_charset(charset)?;
            charset
        };
        let sql = format!(
            "CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET {};",
            db_name, charset
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn drop_database(&self, db_name: &str) -> Result<(), AppError> {
        validate_ident(db_name, "database name")?;
        let sql = format!("DROP DATABASE IF EXISTS `{}`;", db_name);
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn list_databases(&self) -> Result<Vec<String>, AppError> {
        let out = self.exec_mysql("SHOW DATABASES;").await?;
        let dbs: Vec<String> = out
            .lines()
            .filter(|l| {
                // Doc/33 A-5：系统库按**全名**精确排除——旧的子串匹配会把
                // `mysql_app`、`analysis` 这类用户库一并隐藏
                let name = l.trim();
                !name.is_empty() && !MYSQL_SYSTEM_DATABASES.contains(&name)
            })
            .map(|l| l.trim().to_string())
            .collect();
        Ok(dbs)
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        host: &str,
    ) -> Result<(), AppError> {
        validate_ident(username, "username")?;
        crate::database::validate_secret(password, "用户密码")?;
        let host = if host.is_empty() { "localhost" } else { host };
        validate_host(host)?;
        let sql = format!(
            "CREATE USER IF NOT EXISTS '{}'@'{}' IDENTIFIED BY '{}';",
            username,
            host,
            crate::database::mysql_escape_literal(password)
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn drop_user(&self, username: &str, host: &str) -> Result<(), AppError> {
        validate_ident(username, "username")?;
        let host = if host.is_empty() { "localhost" } else { host };
        validate_host(host)?;
        let sql = format!("DROP USER IF EXISTS '{}'@'{}';", username, host);
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn grant_privileges(
        &self,
        username: &str,
        db_name: &str,
        host: &str,
    ) -> Result<(), AppError> {
        validate_ident(username, "username")?;
        validate_ident(db_name, "database name")?;
        let host = if host.is_empty() { "localhost" } else { host };
        validate_host(host)?;
        let sql = format!(
            "GRANT ALL PRIVILEGES ON `{}`.* TO '{}'@'{}'; FLUSH PRIVILEGES;",
            db_name, username, host
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn change_password(&self, new_password: &str) -> Result<(), AppError> {
        crate::database::validate_secret(new_password, "root 密码")?;
        let sql = format!(
            "ALTER USER 'root'@'localhost' IDENTIFIED BY '{}'; FLUSH PRIVILEGES;",
            crate::database::mysql_escape_literal(new_password)
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }
}
impl Default for MySqlManager {
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
            self.calls.lock().unwrap().push(format!(
                "{} {}{}",
                cmd.program,
                cmd.args.join(" "),
                cmd.stdin
                    .as_deref()
                    .map(|s| format!(" <<< {s}"))
                    .unwrap_or_default()
            ));
            Ok(CommandOutput {
                stdout: "5.7.44".into(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    #[tokio::test]
    async fn test_mysql_routes_through_runner() {
        let runner = std::sync::Arc::new(RecordingRunner::new());
        let mgr = MySqlManager::new(runner.clone());

        let _ = mgr.get_version().await.unwrap();
        mgr.create_database("testdb", "utf8mb4").await.unwrap();
        let _ = mgr.list_databases().await.unwrap();

        let programs = runner.programs();
        // 所有 mysql 调用均经统一端口（program 与参数拆分，无任意 shell 拼接）
        // Doc/33 P-2：SQL 经 stdin，argv 只有固定的 `-u root`
        assert!(programs.iter().any(|p| p.starts_with("mysql -u root <<< ")));
        assert!(!programs.iter().any(|p| p.contains(" -e ")));
        assert!(programs.iter().any(|p| p.contains("CREATE DATABASE")));
        assert!(programs.iter().any(|p| p.contains("SHOW DATABASES")));
        // 命令均以 "program args" 形式记录（非单条 shell 字符串）
        assert!(programs.iter().all(|p| p.starts_with("mysql ")));
    }

    /// 可定制 stdout 的 mock runner（验证输出解析逻辑）。
    struct FixedRunner {
        stdout: String,
        calls: Mutex<Vec<Vec<String>>>,
    }

    #[async_trait]
    impl PrivilegedCommandRunner for FixedRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            // 记录 argv；stdin（SQL）追加为末元素，便于断言语句内容
            let mut v = cmd.args.clone();
            if let Some(input) = &cmd.stdin {
                v.push(input.clone());
            }
            self.calls.lock().unwrap().push(v);
            Ok(CommandOutput {
                stdout: self.stdout.clone(),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    }

    fn fixed(stdout: &str) -> std::sync::Arc<FixedRunner> {
        std::sync::Arc::new(FixedRunner {
            stdout: stdout.into(),
            calls: Mutex::new(Vec::new()),
        })
    }

    #[tokio::test]
    async fn list_databases_hides_only_exact_system_schemas() {
        let runner = fixed(
            "Database\ninformation_schema\nmysql\nmysql_app\nperformance_schema\nsys\nanalysis\nshop\n",
        );
        let mgr = MySqlManager::new(runner);
        let dbs = mgr.list_databases().await.unwrap();
        assert_eq!(dbs, vec!["mysql_app", "analysis", "shop"]);
    }

    #[tokio::test]
    async fn create_user_escapes_backslash_quote_breakout() {
        let runner = fixed("");
        let mgr = MySqlManager::new(runner.clone());
        mgr.create_user("app", r"p\'; DROP DATABASE shop; -- ", "localhost")
            .await
            .unwrap();
        let calls = runner.calls.lock().unwrap();
        let sql = calls.last().unwrap().last().unwrap().clone();
        // 密码字面量内反斜杠与引号均被中和，语句仍是单条 CREATE USER
        assert!(
            sql.contains(r"IDENTIFIED BY 'p\\''; DROP DATABASE shop; -- '"),
            "{sql}"
        );
    }

    #[tokio::test]
    async fn create_user_password_never_reaches_argv() {
        // Doc/33 P-2：密码只出现在 stdin 的 SQL 里，argv 固定为 `-u root`
        struct ArgvRunner(Mutex<Vec<(Vec<String>, Option<String>)>>);
        #[async_trait]
        impl PrivilegedCommandRunner for ArgvRunner {
            async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
                self.0
                    .lock()
                    .unwrap()
                    .push((cmd.args.clone(), cmd.stdin.clone()));
                Ok(CommandOutput::default())
            }
        }
        let runner = std::sync::Arc::new(ArgvRunner(Mutex::new(Vec::new())));
        let mgr = MySqlManager::new(runner.clone());
        mgr.create_user("app", "Sup3rS3cret", "localhost")
            .await
            .unwrap();
        let calls = runner.0.lock().unwrap();
        let (argv, stdin) = calls.last().unwrap();
        assert_eq!(argv, &vec!["-u".to_string(), "root".to_string()]);
        assert!(stdin.as_deref().unwrap().contains("Sup3rS3cret"));
        assert!(!argv.concat().contains("Sup3rS3cret"));
    }

    #[tokio::test]
    async fn install_validates_inputs_before_package_manager() {
        let runner = fixed("");
        let mgr = MySqlManager::new(runner.clone());
        assert!(mgr.install(None, 3306, "a\nb").await.is_err());
        assert!(mgr.install(Some("8.0; rm -rf /"), 3306, "").await.is_err());
        assert!(runner.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn append_config_passes_line_as_argv() {
        let runner = fixed("");
        let mgr = MySqlManager::new(runner.clone());
        mgr.append_config("port = 3307").await;
        let calls = runner.calls.lock().unwrap();
        let args = calls.last().unwrap();
        assert_eq!(args[0], "-c");
        assert!(!args[1].contains("3307"), "脚本不得内联配置内容");
        assert_eq!(args[3], "port = 3307");
    }
}
