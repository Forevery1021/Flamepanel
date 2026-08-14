use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 缺省配置时生成的安全随机值来源：基于 `uuid::Uuid::new_v4()`（熵来自 getrandom）。
fn random_secret_hex() -> String {
    // 4 个 UUID（各 32 hex）拼接，共 128 hex 字符 = 64 字节随机密钥，远超 32 字节强度下限。
    let mut out = String::with_capacity(128);
    for _ in 0..4 {
        let u = uuid::Uuid::new_v4();
        out.push_str(&u.simple().to_string());
    }
    out
}

/// 随机初始管理员密码：字母/数字混排（剔除易混淆字符），约 16 字符。
fn random_password() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
    let mut out = String::with_capacity(16);
    for _ in 0..16 {
        let u = uuid::Uuid::new_v4();
        let idx = (u.as_u128() % CHARS.len() as u128) as usize;
        out.push(CHARS[idx] as char);
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub notifications: NotificationsConfig,
    pub jwt_secret: String,
    pub admin_password: String,
    /// 文件/终端沙箱白名单根目录（默认取当前目录，安全默认值见 install.sh）
    pub file_root: String,
    /// 终端启动工作目录（必须位于 file_root 内）
    pub terminal_cwd: String,
    /// 特权命令执行模式：`embedded`（面板本地执行，默认）| `agent`（委托远端 Agent 白名单执行）
    #[serde(default)]
    pub execution_mode: String,
    /// 普通 API 每窗口请求上限（rate limit，默认 120）
    #[serde(default = "default_rate_max")]
    pub rate_limit_max: u64,
    /// 限流窗口秒数（默认 60）
    #[serde(default = "default_rate_window")]
    pub rate_limit_window_secs: u64,
    /// MySQL 配置文件路径（T16 配置化，默认 `/etc/mysql/mysql.conf.d/mysqld.cnf`）
    #[serde(default = "default_mysql_config")]
    pub mysql_config_file: String,
    /// Redis 配置文件路径（T16 配置化，默认 `/etc/redis/redis.conf`）
    #[serde(default = "default_redis_config")]
    pub redis_config_file: String,
}

fn default_mysql_config() -> String {
    "/etc/mysql/mysql.conf.d/mysqld.cnf".into()
}
fn default_redis_config() -> String {
    "/etc/redis/redis.conf".into()
}

fn default_rate_max() -> u64 {
    120
}
fn default_rate_window() -> u64 {
    60
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
            // 安全默认值：不再内置公开弱密钥。缺配置时由 load() 生成随机密钥并持久化。
            jwt_secret: String::new(),
            admin_password: String::new(),
            file_root: ".".to_string(),
            terminal_cwd: ".".to_string(),
            execution_mode: "embedded".to_string(),
            rate_limit_max: default_rate_max(),
            rate_limit_window_secs: default_rate_window(),
            mysql_config_file: default_mysql_config(),
            redis_config_file: default_redis_config(),
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
        if let Ok(val) = std::env::var("OP_FILE_ROOT") {
            if !val.is_empty() {
                self.file_root = val;
            }
        }
        if let Ok(val) = std::env::var("OP_TERMINAL_CWD") {
            if !val.is_empty() {
                self.terminal_cwd = val;
            }
        }
        if let Ok(val) = std::env::var("OP_EXECUTION_MODE") {
            if !val.is_empty() {
                self.execution_mode = val;
            }
        }
        if let Ok(val) = std::env::var("OP_RATE_LIMIT_MAX") {
            if let Ok(v) = val.parse::<u64>() {
                self.rate_limit_max = v;
            }
        }
        if let Ok(val) = std::env::var("OP_RATE_LIMIT_WINDOW") {
            if let Ok(v) = val.parse::<u64>() {
                self.rate_limit_window_secs = v;
            }
        }
        // T16：数据库配置文件路径可配置化（OP_MYSQL_CONFIG / OP_REDIS_CONFIG）
        if let Ok(val) = std::env::var("OP_MYSQL_CONFIG") {
            if !val.is_empty() {
                self.mysql_config_file = val;
            }
        }
        if let Ok(val) = std::env::var("OP_REDIS_CONFIG") {
            if !val.is_empty() {
                self.redis_config_file = val;
            }
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
        let config_file_missing = !Path::new(config_path).exists();
        let mut config = Self::load_from_file(config_path).unwrap_or_default();

        config.apply_env_overrides();

        // T1：消除弱默认密钥。
        // jwt_secret 未提供（无配置文件 / 未设 OP_JWT_SECRET）时：若 data 目录已有持久化密钥则复用，
        // 否则启动期生成随机 64 字节密钥并写入 data 目录，保证重启后签名密钥稳定、且不落公开弱值。
        if config.jwt_secret.is_empty() {
            let existing = Self::read_persisted_secret();
            let generated = match existing {
                Some(prev) => prev,
                None => {
                    let fresh = random_secret_hex();
                    if let Err(e) = Self::persist_generated_secret(&fresh) {
                        tracing::warn!("failed to persist generated jwt_secret: {e}");
                    }
                    fresh
                }
            };
            config.jwt_secret = generated;
            tracing::warn!("No OP_JWT_SECRET provided; using a generated signing secret");
        }
        // admin_password 未提供时生成随机密码（启动日志仅打印一次，供首登使用，配合强制改密）。
        if config.admin_password.is_empty() {
            let generated = random_password();
            config.admin_password = generated;
            tracing::info!(
                "Seeded admin password (auto-generated, change after first login): {}",
                config.admin_password
            );
        }
        if config_file_missing {
            tracing::warn!(
                "config file {} not found; running with defaults (secrets auto-generated)",
                config_path
            );
        }

        Ok(config)
    }

    /// 若已存在持久化密钥则读取之（供重启复用，保证令牌稳定）。
    fn read_persisted_secret() -> Option<String> {
        let secret_path = Path::new("./data").join("jwt_secret.key");
        fs::read_to_string(&secret_path)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
    }

    /// 将生成的随机 JWT 密钥持久化到 data 目录，避免重启后令牌失效。
    fn persist_generated_secret(secret: &str) -> Result<(), AppError> {
        let data_dir = Path::new("./data");
        fs::create_dir_all(data_dir)
            .map_err(|e| AppError::internal(format!("Failed to create data dir: {e}")))?;
        let secret_path = data_dir.join("jwt_secret.key");
        fs::write(&secret_path, secret)
            .map_err(|e| AppError::internal(format!("Failed to persist jwt_secret: {e}")))?;
        // 仅本人可读写，防止其他系统用户读取签名密钥。
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}
