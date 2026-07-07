use async_trait::async_trait;
use crate::core::error::AppError;
use crate::database::NativeDbManager;
use crate::infrastructure::os::{ServiceManager, PackageManager};

pub struct RedisManager {
    service_name: String,
    config_file: String,
}

impl RedisManager {
    pub fn new() -> Self {
        Self {
            service_name: "redis-server".into(),
            config_file: "/etc/redis/redis.conf".into(),
        }
    }
}

#[async_trait]
impl NativeDbManager for RedisManager {
    async fn install(&self, _version: Option<&str>, port: i32, password: &str) -> Result<(), AppError> {
        if PackageManager::is_installed("redis-server").await.unwrap_or(false) {
            return Err(AppError::BadRequest("Redis is already installed".into()));
        }

        PackageManager::install("redis-server").await?;

        // Configure port
        if port != 6379 {
            tokio::process::Command::new("sh")
                .args(["-c", &format!("sed -i 's/^port .*/port {}/' {}", port, self.config_file)])
                .output()
                .await.ok();
        }

        // Configure password
        if !password.is_empty() {
            tokio::process::Command::new("sh")
                .args(["-c", &format!("echo 'requirepass {}' >> {}", password, self.config_file)])
                .output()
                .await.ok();
        }

        ServiceManager::enable("redis-server").await.ok();
        ServiceManager::start("redis-server").await?;

        Ok(())
    }

    async fn uninstall(&self) -> Result<(), AppError> {
        ServiceManager::stop("redis-server").await.ok();
        ServiceManager::disable("redis-server").await.ok();
        tokio::process::Command::new("sh")
            .args(["-c", "apt remove -y redis-server 2>/dev/null || yum remove -y redis 2>/dev/null || apk del redis 2>/dev/null"])
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
        let out = tokio::process::Command::new("redis-server")
            .arg("--version")
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get Redis version: {}", e)))?;
        let s = String::from_utf8_lossy(&out.stdout);
        // redis-server x.y.z
        for word in s.split_whitespace() {
            if word.contains('.') && word.chars().any(|c| c.is_ascii_digit()) {
                return Ok(word.trim_matches(',').to_string());
            }
        }
        Ok("unknown".into())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), AppError> {
        let cmd = format!("redis-cli CONFIG SET {} {}", key, value);
        tokio::process::Command::new("sh")
            .args(["-c", &cmd])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Redis config set failed: {}", e)))?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, AppError> {
        let out = tokio::process::Command::new("redis-cli")
            .args(["CONFIG", "GET", key])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Redis config get failed: {}", e)))?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
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
        tokio::process::Command::new("redis-cli")
            .args(["FLUSHALL"])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Redis flushall failed: {}", e)))?;
        Ok(())
    }

    pub async fn info(&self) -> Result<String, AppError> {
        let out = tokio::process::Command::new("redis-cli")
            .args(["INFO"])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Redis info failed: {}", e)))?;
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    pub async fn set_max_memory(&self, max_mb: usize) -> Result<(), AppError> {
        self.set_config("maxmemory", &format!("{}", max_mb * 1024 * 1024)).await
    }
}
