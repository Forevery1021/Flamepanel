use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub notifications: NotificationsConfig,
    pub jwt_secret: String,
    pub admin_password: String,
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
            jwt_secret: "flamepanel-secret".to_string(),
            admin_password: "admin123".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read config file: {}", e)))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse config: {}", e)))?;
        
        Ok(config)
    }
    
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("OP_PORT") {
            if let Ok(port) = val.parse::<u16>() {
                self.server.port = port;
            }
        }
        if let Ok(val) = std::env::var("OP_HOST") {
            self.server.host = val;
        }
        if let Ok(val) = std::env::var("OP_DATABASE_URL") {
            self.database.url = val;
        }
        if let Ok(val) = std::env::var("OP_JWT_SECRET") {
            self.jwt_secret = val;
        }
        if let Ok(val) = std::env::var("OP_ADMIN_PASSWORD") {
            self.admin_password = val;
        }
        if let Ok(val) = std::env::var("OP_SMTP_HOST") {
            self.notifications.smtp_host = val;
        }
        if let Ok(val) = std::env::var("OP_SMTP_PORT") {
            if let Ok(port) = val.parse::<u16>() {
                self.notifications.smtp_port = port;
            }
        }
        if let Ok(val) = std::env::var("OP_SMTP_USERNAME") {
            self.notifications.smtp_username = val;
        }
        if let Ok(val) = std::env::var("OP_SMTP_PASSWORD") {
            self.notifications.smtp_password = val;
        }
        if let Ok(val) = std::env::var("OP_SMTP_FROM") {
            self.notifications.smtp_from = val;
        }
        if let Ok(val) = std::env::var("OP_SMTP_TLS") {
            if let Ok(tls) = val.parse::<bool>() {
                self.notifications.smtp_tls = tls;
            }
        }
    }
    
    pub fn load() -> Result<Self, AppError> {
        let config_path = "./config/app.toml";
        let mut config = if let Ok(cfg) = Self::load_from_file(config_path) {
            cfg
        } else {
            Self::default()
        };

        config.apply_env_overrides();
        Ok(config)
    }
}
