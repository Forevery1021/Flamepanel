use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    /// 是否强制修改密码（新装面板的种子 admin 为 true）
    pub must_change_password: bool,
}

impl User {
    /// 业务规则：是否强制修改密码（登录后必须改密）
    pub fn must_change_password(&self) -> bool {
        self.must_change_password
    }

    /// 业务规则：当前用户能否修改密码（可扩展：admin 强制策略等）。
    /// 返回 `DomainError::Forbidden` 表示规则不允许。
    pub fn ensure_can_change_password(&self) -> Result<(), crate::domain::error::DomainError> {
        // 保留未来策略：例如最近改密间隔、弱密码拒绝等在此集中
        Ok(())
    }

    /// 业务规则：标记密码已修改（清强制改密标志）。
    pub fn mark_password_changed(&mut self) {
        self.must_change_password = false;
    }

    /// 业务规则：校验用户名格式（字母/数字/下划线，长度 2~32）。
    pub fn validate_username(&self) -> Result<(), crate::domain::error::DomainError> {
        let ok = !self.username.is_empty()
            && self.username.len() >= 2
            && self.username.len() <= 32
            && self
                .username
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if ok {
            Ok(())
        } else {
            Err(crate::domain::error::DomainError::validation(
                "用户名须为 2-32 位字母/数字/下划线/连字符",
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerNode {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    /// 最近一次心跳时间（Agent 上报）
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    /// 最近指标快照 JSON：{cpu_usage, memory_usage_percent, disk_usage_percent, load_one}
    pub metrics_json: Option<String>,
    /// Agent 注册时携带的认证令牌
    pub auth_token: Option<String>,
    /// Agent HTTP 服务端口（面板据此构造 `http://<ip>:<agent_port>` 发起远程调用）
    #[serde(default = "default_node_agent_port")]
    pub agent_port: u16,
}

fn default_node_agent_port() -> u16 {
    9527
}

impl ServerNode {
    pub fn is_online(&self, now: DateTime<Utc>, timeout_secs: i64) -> bool {
        match self.last_heartbeat_at {
            Some(t) => (now - t).num_seconds() <= timeout_secs,
            None => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Website {
    pub id: i64,
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub status: String,
    pub node_id: i64,
    pub engine: String,
    pub ssl_enabled: bool,
    pub proxy_enabled: bool,
    pub proxy_pass: Option<String>,
    pub created_at: DateTime<Utc>,
    /// 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
    #[serde(default)]
    pub resource_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerInstance {
    pub id: i64,
    pub engine: String,
    pub version: Option<String>,
    pub status: String,
    pub config_path: String,
    pub binary_path: Option<String>,
    pub port: i32,
    pub created_at: DateTime<Utc>,
    /// 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
    #[serde(default)]
    pub resource_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainer {
    pub id: String,
    pub image: String,
    pub name: String,
    pub status: String,
    pub node_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DomainEvent {
    UserCreated {
        user_id: i64,
        username: String,
    },
    NodeRegistered {
        node_id: i64,
        node_name: String,
    },
    WebsiteCreated {
        website_id: i64,
        domain: String,
    },
    NodeHeartbeat {
        node_id: i64,
        node_name: String,
    },
    AppInstalled {
        app_key: String,
        app_name: String,
        version: String,
    },
    AppUninstalled {
        app_key: String,
        app_name: String,
    },
    AppUpgraded {
        app_key: String,
        app_name: String,
        from: String,
        to: String,
    },
    FirewallRulesApplied {
        rule_count: usize,
    },
    BackupCreated {
        filename: String,
    },
    UserLoggedIn {
        username: String,
    },
    PasswordChanged {
        username: String,
    },
    NodeOffline {
        node_id: i64,
        node_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub wasm_hash: String,
    /// WASM 字节码（base64 编码），用于持久化与恢复
    pub wasm_base64: String,
    pub enabled: bool,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub config_schema: Option<String>,
    pub dependencies: Vec<PluginDependency>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_requirement: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: i64,
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub memory_usage_percent: f32,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub disk_usage_percent: f32,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub load_one: f64,
    pub load_five: f64,
    pub load_fifteen: f64,
    /// 网络接收速率（MB/s）
    pub network_rx_mbps: f64,
    /// 网络发送速率（MB/s）
    pub network_tx_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub key: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub version: String,
    pub default_port: i32,
    pub icon: String,
    pub compose: String,
}

impl AppManifest {
    /// 内置应用是否上推荐位
    pub fn is_recommended(&self) -> bool {
        matches!(self.key.as_str(), "wordpress" | "portainer" | "uptime-kuma")
    }

    /// 从内置清单生成商店元数据（领域规则：Flame 格式 + 容器模式）
    pub fn to_metadata(&self) -> AppMetadata {
        AppMetadata {
            key: self.key.clone(),
            name: self.name.clone(),
            category: self.category.clone(),
            short_desc_zh: self.description.clone(),
            short_desc_en: None,
            tags: vec![],
            format: AppFormat::Flame,
            modes: vec![InstallMode::Container],
            versions: vec![self.version.clone()],
            default_version: self.version.clone(),
            logo: Some(self.icon.clone()),
            min_memory_mb: None,
            architectures: vec![],
            readme: None,
            recommended: self.is_recommended(),
        }
    }

    /// 从内置清单生成版本信息（含默认表单字段）
    pub fn to_version(&self) -> AppVersionInfo {
        AppVersionInfo {
            version: self.version.clone(),
            mode: InstallMode::Container,
            default_port: Some(self.default_port),
            form_fields: vec![
                FormField {
                    env_key: "PORT".into(),
                    label_zh: "服务端口".into(),
                    label_en: Some("Port".into()),
                    field_type: FieldType::Port,
                    default: Some(self.default_port.to_string()),
                    required: true,
                    pattern: None,
                    min: Some(1),
                    max: Some(65535),
                    min_length: None,
                    max_length: None,
                    options: vec![],
                    description: None,
                    group: Some("基础".into()),
                },
                FormField {
                    env_key: "NAME".into(),
                    label_zh: "实例名称".into(),
                    label_en: Some("Instance name".into()),
                    field_type: FieldType::Text,
                    default: Some(self.key.clone()),
                    required: true,
                    pattern: Some(r"^[a-zA-Z0-9_-]+$".into()),
                    min: None,
                    max: None,
                    min_length: Some(2),
                    max_length: Some(32),
                    options: vec![],
                    description: None,
                    group: Some("基础".into()),
                },
            ],
            compose_template: Some(self.compose.clone()),
            native_scripts: vec![],
            wasm_base64: None,
            min_memory_mb: None,
            architectures: vec![],
        }
    }
}

// ─── 应用商店 (App Store) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AppFormat {
    OnePanel,
    Baota,
    Flame,
}

impl AppFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppFormat::OnePanel => "onepanel",
            AppFormat::Baota => "baota",
            AppFormat::Flame => "flame",
        }
    }

    pub fn from_name(s: &str) -> Option<AppFormat> {
        match s.to_lowercase().as_str() {
            "onepanel" | "1panel" => Some(AppFormat::OnePanel),
            "baota" | "bt" => Some(AppFormat::Baota),
            "flame" => Some(AppFormat::Flame),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum InstallMode {
    Container,
    Native,
    Wasm,
}

impl InstallMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            InstallMode::Container => "container",
            InstallMode::Native => "native",
            InstallMode::Wasm => "wasm",
        }
    }

    pub fn from_name(s: &str) -> Option<InstallMode> {
        match s.to_lowercase().as_str() {
            "container" | "docker" | "compose" => Some(InstallMode::Container),
            "native" | "host" => Some(InstallMode::Native),
            "wasm" | "plugin" => Some(InstallMode::Wasm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Password,
    Select,
    Port,
    Switch,
    Path,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number",
            FieldType::Password => "password",
            FieldType::Select => "select",
            FieldType::Port => "port",
            FieldType::Switch => "switch",
            FieldType::Path => "path",
        }
    }

    pub fn from_name(s: &str) -> Option<FieldType> {
        match s.to_lowercase().as_str() {
            "text" | "string" | "env" => Some(FieldType::Text),
            "number" | "int" | "integer" => Some(FieldType::Number),
            "password" | "secret" => Some(FieldType::Password),
            "select" | "radio" | "checkbox" => Some(FieldType::Select),
            "port" => Some(FieldType::Port),
            "switch" | "boolean" | "bool" => Some(FieldType::Switch),
            "path" | "dir" | "directory" => Some(FieldType::Path),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub env_key: String,
    pub label_zh: String,
    pub label_en: Option<String>,
    pub field_type: FieldType,
    pub default: Option<String>,
    pub required: bool,
    pub pattern: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub options: Vec<SelectOption>,
    pub description: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVersionInfo {
    pub version: String,
    pub mode: InstallMode,
    pub default_port: Option<i32>,
    pub form_fields: Vec<FormField>,
    pub compose_template: Option<String>,
    /// 原生安装脚本步骤（bash 命令，按序执行）
    pub native_scripts: Vec<String>,
    /// WASM 应用字节码（base64）
    pub wasm_base64: Option<String>,
    pub min_memory_mb: Option<u32>,
    pub architectures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppMetadata {
    pub key: String,
    pub name: String,
    pub category: String,
    pub short_desc_zh: String,
    pub short_desc_en: Option<String>,
    pub tags: Vec<String>,
    pub format: AppFormat,
    pub modes: Vec<InstallMode>,
    pub versions: Vec<String>,
    pub default_version: String,
    pub logo: Option<String>,
    pub min_memory_mb: Option<u32>,
    pub architectures: Vec<String>,
    pub readme: Option<String>,
    /// 商店首页推荐位标记（v0.7.0）
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPackage {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub category: String,
    pub format: String,
    pub description: String,
    pub logo: Option<String>,
    pub metadata_json: String,
    pub source_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstalledApp {
    pub id: i64,
    pub package_key: String,
    pub name: String,
    pub version: String,
    pub mode: String,
    pub status: String,
    pub access_url: Option<String>,
    pub install_path: String,
    pub container_name: Option<String>,
    pub port: Option<i32>,
    pub params_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 启动次数（常用应用排序，v0.7.0）
    #[serde(default)]
    pub launch_count: i64,
}

impl InstalledApp {
    /// 简单版本号比较（x.y.z 数字段）。非数字/不同长度时退化为字符串比较。
    pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
        fn parts(v: &str) -> Vec<u64> {
            v.trim()
                .split('.')
                .map(|s| s.trim().parse::<u64>().unwrap_or(0))
                .collect()
        }
        let pa = parts(a);
        let pb = parts(b);
        let len = pa.len().max(pb.len());
        for i in 0..len {
            let x = pa.get(i).copied().unwrap_or(0);
            let y = pb.get(i).copied().unwrap_or(0);
            match x.cmp(&y) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        std::cmp::Ordering::Equal
    }

    /// 业务规则：能否升级到目标版本。
    /// - 目标版本与当前相同 → 允许（幂等，调用方可短路）
    /// - 目标版本低于当前 → 不允许（防降级）
    pub fn can_upgrade_to(
        &self,
        target_version: &str,
    ) -> Result<(), crate::domain::error::DomainError> {
        if Self::version_cmp(target_version, &self.version) == std::cmp::Ordering::Less {
            return Err(crate::domain::error::DomainError::rule_violation(format!(
                "不能降级：当前 {}，目标 {}",
                self.version, target_version
            )));
        }
        Ok(())
    }

    /// 业务规则：记录一次启动。
    pub fn record_launch(&mut self) {
        self.launch_count += 1;
    }

    /// 业务规则：标记升级到目标版本。
    pub fn mark_upgraded(&mut self, target_version: &str) {
        self.version = target_version.to_string();
        self.status = "running".into();
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub id: i64,
    pub username: String,
    pub action: String,
    pub target: Option<String>,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 事件落库（Outbox）记录：每条领域事件持久化存档，保证审计不丢（Stage6）。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutboxEvent {
    pub id: i64,
    /// 事件类型名（如 `AppInstalled` / `UserLoggedIn`）
    pub event_type: String,
    /// 事件载荷（JSON 序列化的结构化字段）
    pub payload: String,
    /// 是否已送达通知渠道（预留；当前仅持久化审计）
    pub published: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memo {
    pub id: i64,
    pub content: String,
    /// memo | todo
    pub kind: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: i64,
    pub source: String,
    pub level: String,
    pub message: String,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelSetting {
    pub key: String,
    pub value: String,
    pub description: String,
    pub updated_at: DateTime<Utc>,
}

pub fn default_settings() -> Vec<PanelSetting> {
    vec![
        PanelSetting {
            key: "panel_name".into(),
            value: "FlamePanel".into(),
            description: "面板名称".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "theme".into(),
            value: "light".into(),
            description: "主题 (light/dark)".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "language".into(),
            value: "zh-CN".into(),
            description: "界面语言".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "panel_port".into(),
            value: "8080".into(),
            description: "面板端口".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "session_timeout_minutes".into(),
            value: "1440".into(),
            description: "会话超时时间(分钟)".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "log_level".into(),
            value: "info".into(),
            description: "日志级别 (trace/debug/info/warn/error)".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "log_retention_days".into(),
            value: "30".into(),
            description: "日志保留天数".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "two_factor_enabled".into(),
            value: "false".into(),
            description: "是否启用两步验证".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "auto_backup_enabled".into(),
            value: "false".into(),
            description: "自动备份开关".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "auto_backup_interval_hours".into(),
            value: "24".into(),
            description: "自动备份间隔（小时）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "backup_retention".into(),
            value: "7".into(),
            description: "备份保留份数".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "open_menu_tabs".into(),
            value: "true".into(),
            description: "是否开启多页签".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "menu_accordion".into(),
            value: "false".into(),
            description: "侧边栏手风琴模式（同时只展开一个分组）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "menu_collapsed".into(),
            value: "false".into(),
            description: "侧边栏折叠状态（多端同步）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "hide_menu".into(),
            value: "[]".into(),
            description: "隐藏的侧边栏菜单分组 key（JSON 数组）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "app_background".into(),
            value: "".into(),
            description: "主界面自定义背景（data URL 或空）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "login_background".into(),
            value: "".into(),
            description: "登录页自定义背景（data URL 或空）".into(),
            updated_at: Utc::now(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub id: i64,
    pub resource: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: String,
}

pub fn default_permissions() -> Vec<Permission> {
    let perms = vec![
        ("user", "create", "创建用户"),
        ("user", "read", "查看用户"),
        ("user", "update", "修改用户"),
        ("user", "delete", "删除用户"),
        ("node", "create", "注册节点"),
        ("node", "read", "查看节点"),
        ("node", "update", "修改节点"),
        ("node", "delete", "删除节点"),
        ("node", "execute", "远程执行命令/文件管理"),
        ("website", "create", "创建网站"),
        ("website", "read", "查看网站"),
        ("website", "update", "修改网站"),
        ("website", "delete", "删除网站"),
        ("docker", "read", "查看容器"),
        ("docker", "create", "创建容器/网络/卷"),
        ("docker", "start", "启动容器"),
        ("docker", "stop", "停止容器"),
        ("docker", "update", "修改容器/网络/卷"),
        ("docker", "delete", "删除容器/镜像"),
        ("memo", "read", "查看备忘录"),
        ("memo", "create", "创建备忘录"),
        ("memo", "update", "修改备忘录"),
        ("memo", "delete", "删除备忘录"),
        ("operation_log", "read", "查看审计日志"),
        ("operation_log", "delete", "删除审计日志"),
        ("outbox", "read", "查看事件落库"),
        ("log", "read", "查看系统日志"),
        ("log", "delete", "删除系统日志"),
        ("plugin", "read", "查看插件"),
        ("plugin", "create", "加载插件"),
        ("plugin", "execute", "执行插件"),
        ("plugin", "delete", "卸载插件"),
        ("plugin", "config", "配置插件"),
        ("web_server", "read", "查看服务器"),
        ("web_server", "create", "创建服务器"),
        ("web_server", "update", "修改服务器"),
        ("web_server", "delete", "删除服务器"),
        ("web_server", "start", "启动服务器"),
        ("web_server", "stop", "停止服务器"),
        ("web_server", "reload", "重载配置"),
        ("web_server", "configtest", "测试配置"),
        ("database", "read", "查看数据库"),
        ("database", "create", "安装/创建数据库"),
        ("database", "update", "修改数据库配置"),
        ("database", "delete", "卸载/删除数据库"),
        ("database", "start", "启动数据库"),
        ("database", "stop", "停止数据库"),
        ("file", "read", "查看文件/目录"),
        ("file", "write", "编辑/创建/删除文件"),
        ("file", "upload", "上传/下载文件"),
        ("settings", "read", "查看面板设置"),
        ("settings", "update", "修改面板设置"),
        ("firewall", "read", "查看防火墙规则"),
        ("firewall", "create", "创建防火墙规则"),
        ("firewall", "update", "修改防火墙规则"),
        ("firewall", "delete", "删除防火墙规则"),
        ("firewall", "enable", "启用/禁用防火墙规则"),
        ("firewall", "apply", "应用防火墙规则"),
        ("app_store", "read", "查看应用商店"),
        ("app_store", "create", "安装应用"),
        ("app_store", "update", "升级应用"),
        ("app_store", "delete", "卸载应用"),
        ("backup", "read", "查看备份"),
        ("backup", "create", "创建/恢复备份"),
        ("backup", "delete", "删除备份"),
        ("scheduled_task", "read", "查看定时任务"),
        ("scheduled_task", "create", "创建定时任务"),
        ("scheduled_task", "update", "修改定时任务"),
        ("scheduled_task", "delete", "删除定时任务"),
        ("scheduled_task", "execute", "执行定时任务"),
        ("task", "read", "查看任务"),
        ("task", "execute", "取消/执行任务"),
        ("task", "delete", "清理任务"),
    ];
    perms
        .into_iter()
        .enumerate()
        .map(|(i, (r, a, d))| Permission {
            id: (i + 1) as i64,
            resource: r.to_string(),
            action: a.to_string(),
            description: d.to_string(),
        })
        .collect()
}

pub fn default_roles() -> Vec<Role> {
    vec![
        Role {
            id: 1,
            name: "admin".into(),
            description: "超级管理员，拥有所有权限".into(),
        },
        Role {
            id: 2,
            name: "operator".into(),
            description: "运维操作员，读写大部分资源".into(),
        },
        Role {
            id: 3,
            name: "viewer".into(),
            description: "只读用户，仅可查看".into(),
        },
    ]
}

pub fn role_permissions(role_name: &str) -> HashSet<i64> {
    let all_perms = default_permissions();
    let all_ids: HashSet<i64> = all_perms.iter().map(|p| p.id).collect();
    match role_name {
        "admin" => all_ids,
        "operator" => all_perms
            .iter()
            .filter(|p| p.action != "delete")
            .map(|p| p.id)
            .collect(),
        "viewer" => all_perms
            .iter()
            .filter(|p| p.action == "read")
            .map(|p| p.id)
            .collect(),
        _ => HashSet::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseType {
    Mysql,
    MariaDB,
    Redis,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "mysql",
            DatabaseType::MariaDB => "mariadb",
            DatabaseType::Redis => "redis",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mysql" => Some(DatabaseType::Mysql),
            "mariadb" => Some(DatabaseType::MariaDB),
            "redis" => Some(DatabaseType::Redis),
            _ => None,
        }
    }

    pub fn default_port(&self) -> i32 {
        match self {
            DatabaseType::Mysql | DatabaseType::MariaDB => 3306,
            DatabaseType::Redis => 6379,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "MySQL 关系型数据库",
            DatabaseType::MariaDB => "MariaDB 关系型数据库",
            DatabaseType::Redis => "Redis 内存缓存数据库",
        }
    }

    pub fn service_name(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "mysql",
            DatabaseType::MariaDB => "mariadb",
            DatabaseType::Redis => "redis-server",
        }
    }

    pub fn package_name(&self) -> &'static str {
        match self {
            DatabaseType::Mysql => "mysql-server",
            DatabaseType::MariaDB => "mariadb-server",
            DatabaseType::Redis => "redis-server",
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

pub fn builtin_apps() -> Vec<AppManifest> {
    vec![
        AppManifest {
            key: "wordpress".into(),
            name: "WordPress".into(),
            category: "CMS / 博客".into(),
            description: "全球最流行的内容管理系统，适合搭建博客和企业网站".into(),
            version: "6.7".into(),
            default_port: 8081,
            icon: "wordpress".into(),
            compose: r#"version: '3'
services:
  wordpress:
    image: wordpress:6.7
    ports:
      - "{port}:80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: wordpress
      WORDPRESS_DB_PASSWORD: wp_{name}_pass
      WORDPRESS_DB_NAME: wordpress
    restart: unless-stopped
    depends_on:
      - db
  db:
    image: mysql:8
    environment:
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wordpress
      MYSQL_PASSWORD: wp_{name}_pass
      MYSQL_ROOT_PASSWORD: root_{name}_pass
    restart: unless-stopped
"#
            .into(),
        },
        AppManifest {
            key: "portainer".into(),
            name: "Portainer".into(),
            category: "DevOps / 工具".into(),
            description: "Docker 可视化容器管理界面".into(),
            version: "2.21".into(),
            default_port: 9443,
            icon: "portainer".into(),
            compose: r#"version: '3'
services:
  portainer:
    image: portainer/portainer-ce:2.21.5
    ports:
      - "{port}:9000"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - {data_dir}:/data
    restart: unless-stopped
"#
            .into(),
        },
        AppManifest {
            key: "nginx".into(),
            name: "Nginx".into(),
            category: "Web / 反向代理".into(),
            description: "高性能 HTTP 服务器和反向代理".into(),
            version: "1.27".into(),
            default_port: 8083,
            icon: "nginx".into(),
            compose: r#"version: '3'
services:
  nginx:
    image: nginx:1.27-alpine
    ports:
      - "{port}:80"
    volumes:
      - {data_dir}/html:/usr/share/nginx/html:ro
    restart: unless-stopped
"#
            .into(),
        },
        AppManifest {
            key: "redis".into(),
            name: "Redis 缓存".into(),
            category: "缓存 / 消息队列".into(),
            description: "高性能内存键值存储，适用于缓存、会话存储和消息队列".into(),
            version: "7.4".into(),
            default_port: 6379,
            icon: "redis".into(),
            compose: r#"version: '3'
services:
  redis:
    image: redis:7.4-alpine
    ports:
      - "{port}:6379"
    volumes:
      - {data_dir}:/data
    command: redis-server --appendonly yes --requirepass app_{name}_pass
    restart: unless-stopped
"#
            .into(),
        },
        AppManifest {
            key: "uptime-kuma".into(),
            name: "Uptime Kuma".into(),
            category: "监控 / 告警".into(),
            description: "自托管网站和服务状态监控面板".into(),
            version: "1.23".into(),
            default_port: 8084,
            icon: "uptime-kuma".into(),
            compose: r#"version: '3'
services:
  uptime-kuma:
    image: louislam/uptime-kuma:1
    ports:
      - "{port}:3001"
    volumes:
      - {data_dir}:/app/data
    restart: unless-stopped
"#
            .into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(username: &str) -> User {
        User {
            id: 1,
            username: username.into(),
            password_hash: "x".into(),
            role: "admin".into(),
            created_at: chrono::Utc::now(),
            must_change_password: true,
        }
    }

    fn installed(version: &str) -> InstalledApp {
        InstalledApp {
            id: 1,
            package_key: "wordpress".into(),
            name: "wp".into(),
            version: version.into(),
            mode: "container".into(),
            status: "running".into(),
            access_url: None,
            install_path: "/tmp/app".into(),
            container_name: None,
            port: None,
            params_json: "{}".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            launch_count: 0,
        }
    }

    #[test]
    fn user_validate_username() {
        assert!(user("admin").validate_username().is_ok());
        assert!(user("a_b-c1").validate_username().is_ok());
        assert!(user("a").validate_username().is_err());
        assert!(user("bad name!").validate_username().is_err());
    }

    #[test]
    fn user_mark_password_changed() {
        let mut u = user("admin");
        assert!(u.must_change_password());
        u.mark_password_changed();
        assert!(!u.must_change_password());
        assert!(u.ensure_can_change_password().is_ok());
    }

    #[test]
    fn server_node_online_status() {
        let mut node = ServerNode {
            id: 1,
            name: "n".into(),
            hostname: "h".into(),
            ip_address: "1.2.3.4".into(),
            status: "active".into(),
            created_at: chrono::Utc::now(),
            last_heartbeat_at: None,
            metrics_json: None,
            auth_token: None,
            agent_port: 9527,
        };
        assert!(!node.is_online(chrono::Utc::now(), 30));
        node.last_heartbeat_at = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
        assert!(node.is_online(chrono::Utc::now(), 30));
        node.last_heartbeat_at = Some(chrono::Utc::now() - chrono::Duration::seconds(60));
        assert!(!node.is_online(chrono::Utc::now(), 30));
    }

    #[test]
    fn installed_app_can_upgrade() {
        let app = installed("1.0.0");
        assert!(app.can_upgrade_to("1.0.0").is_ok()); // 同版本允许（幂等短路）
        assert!(app.can_upgrade_to("1.0.1").is_ok());
        assert!(app.can_upgrade_to("1.1.0").is_ok());
        assert!(app.can_upgrade_to("2.0").is_ok());
        assert!(app.can_upgrade_to("0.9.0").is_err()); // 降级拒绝
    }

    #[test]
    fn installed_app_mark_upgraded_and_launch() {
        let mut app = installed("1.0.0");
        app.record_launch();
        app.record_launch();
        assert_eq!(app.launch_count, 2);
        app.mark_upgraded("2.0.0");
        assert_eq!(app.version, "2.0.0");
        assert_eq!(app.status, "running");
    }

    #[test]
    fn version_cmp_works() {
        assert_eq!(
            InstalledApp::version_cmp("1.0.0", "1.0.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            InstalledApp::version_cmp("1.0.1", "1.0.0"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            InstalledApp::version_cmp("1.2", "1.10"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            InstalledApp::version_cmp("2.0", "1.9.9"),
            std::cmp::Ordering::Greater
        );
    }
}
