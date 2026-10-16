//! 宿主机服务：Docker 容器、数据库实例、防火墙、计划任务。

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerContainer {
    pub id: String,
    pub image: String,
    pub name: String,
    pub status: String,
    pub node_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseType {
    Mysql,
    MariaDB,
    Redis,
    Postgres,
    Mongodb,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "mysql",
            DatabaseType::MariaDB => "mariadb",
            DatabaseType::Redis => "redis",
            DatabaseType::Postgres => "postgres",
            DatabaseType::Mongodb => "mongodb",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mysql" => Some(DatabaseType::Mysql),
            "mariadb" => Some(DatabaseType::MariaDB),
            "redis" => Some(DatabaseType::Redis),
            "postgres" | "postgresql" | "pgsql" => Some(DatabaseType::Postgres),
            "mongodb" | "mongo" => Some(DatabaseType::Mongodb),
            _ => None,
        }
    }

    pub fn default_port(&self) -> i32 {
        match self {
            DatabaseType::Mysql | DatabaseType::MariaDB => 3306,
            DatabaseType::Redis => 6379,
            DatabaseType::Postgres => 5432,
            DatabaseType::Mongodb => 27017,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "MySQL 关系型数据库",
            DatabaseType::MariaDB => "MariaDB 关系型数据库",
            DatabaseType::Redis => "Redis 内存缓存数据库",
            DatabaseType::Postgres => "PostgreSQL 关系型数据库",
            DatabaseType::Mongodb => "MongoDB 文档数据库",
        }
    }

    pub fn service_name(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "mysql",
            DatabaseType::MariaDB => "mariadb",
            DatabaseType::Redis => "redis-server",
            DatabaseType::Postgres => "postgresql",
            DatabaseType::Mongodb => "mongod",
        }
    }

    pub fn package_name(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "mysql-server",
            DatabaseType::MariaDB => "mariadb-server",
            DatabaseType::Redis => "redis-server",
            DatabaseType::Postgres => "postgresql",
            DatabaseType::Mongodb => "mongodb-org",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInstance {
    pub id: i64,
    pub db_type: String,
    pub name: String,
    pub version: String,
    pub port: i32,
    pub status: String,
    pub install_path: String,
    pub data_dir: String,
    pub config_file: String,
    pub root_user: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
    #[serde(default)]
    pub resource_version: i64,
}

/// 防火墙后端类型（探测到的可用防火墙工具）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallBackend {
    Ufw,
    Firewalld,
    Iptables,
    /// 未检测到可用防火墙工具（携带原因）。
    Unsupported(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub protocol: String,
    pub port: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub action: String,
    pub enabled: bool,
    pub priority: i32,
    pub direction: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn default_firewall_rules() -> Vec<FirewallRule> {
    vec![
        FirewallRule {
            id: 1,
            name: "允许 SSH".into(),
            description: Some("允许 SSH 远程连接".into()),
            protocol: "tcp".into(),
            port: Some("22".into()),
            source: Some("0.0.0.0/0".into()),
            destination: None,
            action: "allow".into(),
            enabled: true,
            priority: 10,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        FirewallRule {
            id: 2,
            name: "允许 HTTP".into(),
            description: Some("允许 HTTP Web 流量".into()),
            protocol: "tcp".into(),
            port: Some("80".into()),
            source: Some("0.0.0.0/0".into()),
            destination: None,
            action: "allow".into(),
            enabled: true,
            priority: 20,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        FirewallRule {
            id: 3,
            name: "允许 HTTPS".into(),
            description: Some("允许 HTTPS Web 流量".into()),
            protocol: "tcp".into(),
            port: Some("443".into()),
            source: Some("0.0.0.0/0".into()),
            destination: None,
            action: "allow".into(),
            enabled: true,
            priority: 30,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        FirewallRule {
            id: 4,
            name: "允许面板端口".into(),
            description: Some("允许 FlamePanel 面板 Web 访问".into()),
            protocol: "tcp".into(),
            port: Some("8080".into()),
            source: Some("0.0.0.0/0".into()),
            destination: None,
            action: "allow".into(),
            enabled: true,
            priority: 40,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        FirewallRule {
            id: 5,
            name: "拒绝所有入站".into(),
            description: Some("拒绝所有其他入站流量（兜底规则）".into()),
            protocol: "any".into(),
            port: None,
            source: Some("0.0.0.0/0".into()),
            destination: None,
            action: "deny".into(),
            enabled: true,
            priority: 100,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduledTask {
    pub id: i64,
    pub name: String,
    pub command: String,
    /// 标准 5 字段 cron 表达式
    pub schedule: String,
    pub enabled: bool,
    /// never | success | failed
    pub last_status: String,
    pub last_output: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
