use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub notifications: NotificationsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_tls: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".into(),
            smtp_port: 25,
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_from: "noreply@flamepanel.local".into(),
            smtp_tls: false,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                url: "sqlite://data/app.db".to_string(),
            },
            notifications: NotificationsConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = fs::read_to_string(&path)
            .map_err(|e| AppError::Internal(format!("Failed to read config file: {}", e)))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| AppError::Internal(format!("Failed to parse config: {}", e)))?;
        
        Ok(config)
    }
    
    pub fn load() -> Result<Self, AppError> {
        // Try to load from default location
        let config_path = "./config/app.toml";
        if let Ok(config) = Self::load_from_file(config_path) {
            return Ok(config);
        }
        
        // Return default config
        Ok(Self::default())
    }
}
