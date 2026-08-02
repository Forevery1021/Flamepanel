use std::process::Stdio;
use crate::core::error::AppError;
use crate::domain::entity::WebServerInstance;
use super::engine::WebServerEngine;

#[derive(Clone)]
pub struct WebServerManager;

impl WebServerManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn check_status(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_str(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let output = tokio::process::Command::new("pgrep")
            .arg("-x")
            .arg(engine.binary_name())
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to check process: {}", e)))?;

        if output.status.success() {
            Ok("running".into())
        } else {
            Ok("stopped".into())
        }
    }

    pub async fn start(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_str(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let binary = instance.binary_path.as_deref().unwrap_or(engine.binary_name());
        let output = tokio::process::Command::new(binary)
            .arg("-c")
            .arg(&instance.config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to start {}: {}", engine.as_str(), e)))?;

        if output.status.success() {
            Ok(format!("{} started successfully", engine.as_str()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(AppError::internal(format!("Failed to start {}: {}", engine.as_str(), stderr)))
        }
    }

    pub async fn stop(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_str(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let output = tokio::process::Command::new("killall")
            .arg(engine.binary_name())
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to stop {}: {}", engine.as_str(), e)))?;

        if output.status.success() {
            Ok(format!("{} stopped successfully", engine.as_str()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(AppError::internal(format!("Failed to stop {}: {}", engine.as_str(), stderr)))
        }
    }

    pub async fn restart(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        self.stop(instance).await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start(instance).await
    }

    pub async fn reload(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_str(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(engine.reload_command())
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Failed to reload {}: {}", engine.as_str(), e)))?;

        if output.status.success() {
            Ok(format!("{} reloaded successfully", engine.as_str()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(AppError::internal(format!("Failed to reload {}: {}", engine.as_str(), stderr)))
        }
    }

    pub async fn config_test(&self, instance: &WebServerInstance) -> Result<String, AppError> {
        let engine = WebServerEngine::from_str(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(engine.config_test_command())
            .output()
            .await
            .map_err(|e| AppError::internal(format!("Config test failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            Err(AppError::internal(format!("Config test failed: {}", stderr)))
        }
    }

    pub async fn write_config_file(&self, path: &str, content: &str) -> Result<(), AppError> {
        tokio::fs::write(path, content)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write config file {}: {}", path, e)))
    }

    pub async fn enable_site(&self, engine: &WebServerEngine, domain: &str, config_path: &str) -> Result<(), AppError> {
        let enabled_dir = engine.sites_enabled_dir();
        let target = format!("{}/{}", enabled_dir, domain);
        tokio::fs::write(&target, config_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to enable site: {}", e)))
    }

    pub async fn disable_site(&self, engine: &WebServerEngine, domain: &str) -> Result<(), AppError> {
        let enabled_dir = engine.sites_enabled_dir();
        let target = format!("{}/{}", enabled_dir, domain);
        if tokio::fs::try_exists(&target).await.unwrap_or(false) {
            tokio::fs::remove_file(&target)
                .await
                .map_err(|e| AppError::internal(format!("Failed to disable site: {}", e)))?;
        }
        Ok(())
    }
}
