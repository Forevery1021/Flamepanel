use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{PackageManager, ServiceManager};
use async_trait::async_trait;

pub struct MySqlManager {
    pub service_name: String,
    pub config_file: String,
    pub package_manager: PackageManager,
    pub service_manager: ServiceManager,
    runner: SharedCommandRunner,
}

impl MySqlManager {
    /// 注入特权命令执行器（`execution_mode=embedded|agent`）。
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            service_name: "mysql".into(),
            config_file: "/etc/mysql/mysql.conf.d/mysqld.cnf".into(),
            package_manager: PackageManager::new(runner.clone()),
            service_manager: ServiceManager::new(runner.clone()),
            runner,
        }
    }

    /// T16：覆盖 MySQL 配置文件路径（默认 `/etc/mysql/mysql.conf.d/mysqld.cnf`）。
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

    /// 经统一特权命令端口执行 `mysql -u root -e <sql>`（Phase A1：收敛到 PrivilegedCommandRunner）。
    async fn exec_mysql(&self, sql: &str) -> Result<String, AppError> {
        let out = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "mysql",
                vec!["-u".into(), "root".into(), "-e".into(), sql.to_string()],
            ))
            .await?;
        let stdout = out.stdout.trim().to_string();
        let stderr = out.stderr.trim().to_string();
        if !out.success() && !stderr.is_empty() {
            return Err(AppError::internal(format!("MySQL error: {}", stderr)));
        }
        Ok(stdout)
    }

    /// 追加一行到 MySQL 配置文件（经统一端口执行 `sh -c`，避免面板直接 spawn）。
    async fn append_config(&self, line: &str) {
        let script = format!("echo '{}' >> {}", line, self.config_file);
        let _ = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "sh",
                vec!["-c".into(), script],
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
        let pkg = if let Some(ver) = version {
            format!("mysql-server-{}", ver)
        } else {
            "mysql-server".to_string()
        };

        if self
            .package_manager
            .is_installed("mysql-server")
            .await
            .unwrap_or(false)
        {
            return Err(AppError::BadRequest("MySQL is already installed".into()));
        }

        self.package_manager.install(&pkg).await?;

        // Start and enable service
        self.service_manager.enable("mysql").await.ok();
        self.service_manager.start("mysql").await?;

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
                root_password.replace('\'', "\\'")
            );
            self.exec_mysql(&sql).await.ok();
        }

        // Change port if not default
        if port != 3306 {
            let port_line = format!("port = {}", port);
            self.append_config(&port_line).await;
            self.service_manager.restart("mysql").await?;
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        self.service_manager.stop("mysql").await.ok();
        self.service_manager.disable("mysql").await.ok();
        // 经统一端口卸载（多包管理器回退，best-effort，失败不阻断）
        let _ = self
            .runner
            .run(&crate::application::execution_mode::PrivilegedCommand::new(
                "sh",
                vec![
                    "-c".into(),
                    "apt remove -y mysql-server mysql-client 2>/dev/null || yum remove -y mysql-server 2>/dev/null || apk del mysql 2>/dev/null".into(),
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
        let s = self.exec_mysql("SELECT VERSION();").await?;
        for line in s.lines() {
            if line.contains('.') && !line.contains("VERSION") {
                return Ok(line.trim().to_string());
            }
        }
        Ok("unknown".into())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError> {
        // Use mysql CLI to set global variable
        let sql = format!("SET GLOBAL {} = '{}';", key, value.replace('\'', "\\'"));
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError> {
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
        let charset = if charset.is_empty() {
            "utf8mb4"
        } else {
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
        let sql = format!("DROP DATABASE IF EXISTS `{}`;", db_name);
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn list_databases(&self) -> Result<Vec<String>, AppError> {
        let out = self.exec_mysql("SHOW DATABASES;").await?;
        let dbs: Vec<String> = out
            .lines()
            .filter(|l| {
                !l.is_empty()
                    && !l.contains("Database")
                    && !l.contains("information_schema")
                    && !l.contains("performance_schema")
                    && !l.contains("mysql")
                    && !l.contains("sys")
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
        let host = if host.is_empty() { "localhost" } else { host };
        let sql = format!(
            "CREATE USER IF NOT EXISTS '{}'@'{}' IDENTIFIED BY '{}';",
            username.replace('\'', "\\'"),
            host,
            password.replace('\'', "\\'")
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn drop_user(&self, username: &str, host: &str) -> Result<(), AppError> {
        let host = if host.is_empty() { "localhost" } else { host };
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
        let host = if host.is_empty() { "localhost" } else { host };
        let sql = format!(
            "GRANT ALL PRIVILEGES ON `{}`.* TO '{}'@'{}'; FLUSH PRIVILEGES;",
            db_name, username, host
        );
        self.exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn change_password(&self, new_password: &str) -> Result<(), AppError> {
        let sql = format!(
            "ALTER USER 'root'@'localhost' IDENTIFIED BY '{}'; FLUSH PRIVILEGES;",
            new_password.replace('\'', "\\'")
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
            self.calls
                .lock()
                .unwrap()
                .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
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
        assert!(programs.iter().any(|p| p.starts_with("mysql -u root -e")));
        assert!(programs.iter().any(|p| p.contains("CREATE DATABASE")));
        assert!(programs.iter().any(|p| p.contains("SHOW DATABASES")));
        // 命令均以 "program args" 形式记录（非单条 shell 字符串）
        assert!(programs.iter().all(|p| p.starts_with("mysql ")));
    }
}
