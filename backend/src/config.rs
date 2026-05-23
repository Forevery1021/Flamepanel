use figment::{Figment, providers::{Serialized, Env, Format, Toml}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password: String, // 生产环境建议仅首次使用
}

impl Config {
    pub fn load() -> Result<Self, figment::Error> {
        let figment = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Env::prefixed("OP_"));

        let figment = if std::path::Path::new("config.toml").exists() {
            figment.merge(Toml::file("config.toml"))
        } else {
            figment
        };

        figment.extract()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            // mode=rwc：文件不存在时自动创建；路径相对于运行时工作目录
            database_url: "sqlite:data/ops_panel.db?mode=rwc".to_string(),
            jwt_secret: "your-super-secret-jwt-key-change-in-production".to_string(),
            admin_username: "admin".to_string(),
            admin_password: "admin123".to_string(),
        }
    }
}