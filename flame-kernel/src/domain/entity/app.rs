//! 应用商店：清单、格式 / 模式、表单、钩子、包、源、已安装应用、内置应用。

use super::*;

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
            hooks: AppHooks::default(),
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

/// 应用包生命周期钩子（Doc/20 §4.3）
///
/// 宿主机脚本路径（相对包根/版本目录）。容器模式安装/升级/卸载时由执行器
/// 经统一任务中心按序执行：超时 300s、输出落任务日志、失败即中止。
/// 脚本须通过 security_scanner 静态扫描（高危模式告警/阻断）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AppHooks {
    /// 容器安装前执行（相对包根，如 `scripts/pre_install.sh`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<String>,
    /// 容器安装（compose up）成功后执行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_install: Option<String>,
    /// 升级（换镜像 compose up）前执行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_upgrade: Option<String>,
    /// 升级成功后执行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_upgrade: Option<String>,
    /// 卸载（compose down）前执行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_uninstall: Option<String>,
}

impl AppHooks {
    /// 是否包含任意钩子脚本
    pub fn is_empty(&self) -> bool {
        self.pre_install.is_none()
            && self.post_install.is_none()
            && self.pre_upgrade.is_none()
            && self.post_upgrade.is_none()
            && self.pre_uninstall.is_none()
    }
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
    /// 生命周期钩子（Doc/20 §4.3；serde 默认空，旧包/旧 API 响应兼容）
    #[serde(default)]
    pub hooks: AppHooks,
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
    /// 包来源（M-A Issue 1.1）：local 本地导入 | git 远程 Git 源 | http 远程 HTTP 源
    #[serde(default = "default_package_source")]
    pub source: String,
    /// 来源地址（本地导入为 None；远程源为 source 的 url）
    #[serde(default)]
    pub source_url: Option<String>,
    /// 从远程源同步到本地的时刻（本地包为 None）
    #[serde(default)]
    pub synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_package_source() -> String {
    "local".to_string()
}

/// 远程应用源（M-A Issue 1.1，Doc/20 §4.1）
///
/// 可配置多源：git（1Panel appstore 仓库镜像）| http（自建源 JSON manifest）。
/// 远程包同步走统一任务中心；安装时才按需拉取包体到本地缓存。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppSource {
    pub id: i64,
    pub name: String,
    /// 源类型：git | http
    pub source_type: String,
    pub url: String,
    pub enabled: bool,
    /// Ed25519 公钥（应用包验签，Doc/20 §4.2；未配置则不做验签）
    pub public_key: Option<String>,
    /// 最近一次索引同步时刻
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for AppSource {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            source_type: "git".to_string(),
            url: String::new(),
            enabled: true,
            public_key: None,
            last_synced_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl AppSource {
    /// 合法源类型白名单：git | http
    pub fn is_valid_source_type(t: &str) -> bool {
        matches!(t, "git" | "http")
    }
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
        // M-AI-3：本地大模型运行时，供面板内置助手使用（Ollama 协议，默认地址 127.0.0.1:11434）。
        // Ollama 自身无鉴权——端口只绑定回环地址，禁止直接暴露到公网。
        AppManifest {
            key: "ollama".into(),
            name: "Ollama".into(),
            category: "AI / 大模型".into(),
            description:
                "本地运行开源大模型（Qwen、Llama 等），可作为面板 AI 助手的模型服务；端口仅监听本机"
                    .into(),
            version: "0.12".into(),
            default_port: 11434,
            icon: "ollama".into(),
            compose: r#"version: '3'
services:
  ollama:
    image: ollama/ollama:0.12.3
    ports:
      - "127.0.0.1:{port}:11434"
    volumes:
      - {data_dir}:/root/.ollama
    restart: unless-stopped
"#
            .into(),
        },
    ]
}
