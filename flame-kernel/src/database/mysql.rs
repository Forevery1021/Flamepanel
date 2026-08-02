use async_trait::async_trait;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{ServiceManager, PackageManager};

pub struct MySqlManager {
    pub service_name: String,
    pub config_file: String,
}

impl MySqlManager {
    pub fn new() -> Self {
        Self {
            service_name: "mysql".into(),
            config_file: "/etc/mysql/mysql.conf.d/mysqld.cnf".into(),
        }
    }

    async fn exec_mysql(sql: &str) -> Result<String, AppError> {
        let out = tokio::process::Command::new("mysql")
            .args(["-u", "root", "-e", sql])
            .output()
            .await
            .map_err(|e| AppError::internal(format!("MySQL exec failed: {}", e)))?;
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if !out.status.success() && !stderr.is_empty() {
            return Err(AppError::internal(format!("MySQL error: {}", stderr)));
        }
        Ok(stdout)
    }
}

#[async_trait]
impl NativeDbManager for MySqlManager {
    async fn install(&self, version: Option<&str>, port: i32, root_password: &str) -> Result<(), AppError> {
        let pkg = if let Some(ver) = version {
            format!("mysql-server-{}", ver)
        } else {
            "mysql-server".to_string()
        };

        if PackageManager::is_installed("mysql-server").await.unwrap_or(false) {
            return Err(AppError::BadRequest("MySQL is already installed".into()));
        }

        PackageManager::install(&pkg).await?;

        // Start and enable service
        ServiceManager::enable("mysql").await.ok();
        ServiceManager::start("mysql").await?;

        // Wait for MySQL to be ready
        for _ in 0..30 {
            let out = tokio::process::Command::new("mysqladmin")
                .args(["ping", "-u", "root"])
                .output()
                .await;
            if let Ok(o) = out {
                if o.status.success() {
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
            Self::exec_mysql(&sql).await.ok();
        }

        // Change port if not default
        if port != 3306 {
            let port_line = format!("port = {}", port);
            tokio::process::Command::new("sh")
                .args(["-c", &format!("echo '{}' >> {}", port_line, self.config_file)])
                .output()
                .await.ok();
            ServiceManager::restart("mysql").await?;
        }

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        ServiceManager::stop("mysql").await.ok();
        ServiceManager::disable("mysql").await.ok();
        tokio::process::Command::new("sh")
            .args(["-c", "apt remove -y mysql-server mysql-client 2>/dev/null || yum remove -y mysql-server 2>/dev/null || apk del mysql 2>/dev/null"])
            .output()
            .await.ok();
        Ok(())
    }

    async fn start(&self) -> Result<(), AppError> {
        ServiceManager::start(&self.service_name).await
    }

    async fn stop(&self) -> Result<(), AppError> {
        ServiceManager::stop(&self.service_name).await
    }

    async fn restart(&self) -> Result<(), AppError> {
        ServiceManager::restart(&self.service_name).await
    }

    async fn is_running(&self) -> Result<bool, AppError> {
        ServiceManager::is_running(&self.service_name).await
    }

    async fn get_version(&self) -> Result<String, AppError> {
        let out = tokio::process::Command::new("mysql")
            .args(["-u", "root", "-e", "SELECT VERSION();"])
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get MySQL version: {}", e)))?;
        let s = String::from_utf8_lossy(&out.stdout);
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
        Self::exec_mysql(&sql).await?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError> {
        let sql = format!("SHOW VARIABLES LIKE '{}';", key);
        let out = Self::exec_mysql(&sql).await?;
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
        let charset = if charset.is_empty() { "utf8mb4" } else { charset };
        let sql = format!("CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET {};", db_name, charset);
        Self::exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn drop_database(&self, db_name: &str) -> Result<(), AppError> {
        let sql = format!("DROP DATABASE IF EXISTS `{}`;", db_name);
        Self::exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn list_databases(&self) -> Result<Vec<String>, AppError> {
        let out = Self::exec_mysql("SHOW DATABASES;").await?;
        let dbs: Vec<String> = out.lines()
            .filter(|l| !l.is_empty() && !l.contains("Database") && !l.contains("information_schema")
                && !l.contains("performance_schema") && !l.contains("mysql") && !l.contains("sys"))
            .map(|l| l.trim().to_string())
            .collect();
        Ok(dbs)
    }

    pub async fn create_user(&self, username: &str, password: &str, host: &str) -> Result<(), AppError> {
        let host = if host.is_empty() { "localhost" } else { host };
        let sql = format!(
            "CREATE USER IF NOT EXISTS '{}'@'{}' IDENTIFIED BY '{}';",
            username.replace('\'', "\\'"),
            host,
            password.replace('\'', "\\'")
        );
        Self::exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn drop_user(&self, username: &str, host: &str) -> Result<(), AppError> {
        let host = if host.is_empty() { "localhost" } else { host };
        let sql = format!("DROP USER IF EXISTS '{}'@'{}';", username, host);
        Self::exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn grant_privileges(&self, username: &str, db_name: &str, host: &str) -> Result<(), AppError> {
        let host = if host.is_empty() { "localhost" } else { host };
        let sql = format!("GRANT ALL PRIVILEGES ON `{}`.* TO '{}'@'{}'; FLUSH PRIVILEGES;", db_name, username, host);
        Self::exec_mysql(&sql).await?;
        Ok(())
    }

    pub async fn change_password(&self, new_password: &str) -> Result<(), AppError> {
        let sql = format!("ALTER USER 'root'@'localhost' IDENTIFIED BY '{}'; FLUSH PRIVILEGES;", new_password.replace('\'', "\\'"));
        Self::exec_mysql(&sql).await?;
        Ok(())
    }
}
