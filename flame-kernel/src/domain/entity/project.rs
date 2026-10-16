//! 运行环境与项目部署。

use super::*;

/// 运行时（M-B 主线二 Stage A-1 · Doc/20 §5.1/5.2）——全局可安装、多版本共存
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Runtime {
    pub id: i64,
    /// 语言种类：php | java | node | python | go | tomcat
    pub kind: String,
    /// 版本号：8.2 / 21 / 20.11 ...
    pub version: String,
    /// 安装路径：/opt/flamepanel/runtimes/<kind>/<version>
    pub install_path: String,
    /// 安装状态：not_installed | installing | installed | running | error
    pub status: String,
    /// 同 kind 下唯一默认版本（1 = 默认）
    pub is_default: bool,
    /// 扩展元数据 JSON（扩展列表、端口、env 等）
    pub metadata_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            id: 0,
            kind: "php".to_string(),
            version: String::new(),
            install_path: String::new(),
            status: "not_installed".to_string(),
            is_default: false,
            metadata_json: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Runtime {
    /// 合法语言种类白名单（Doc/20 §5.2；`tomcat` 为 Stage B-2 新增的
    /// Java WAR 容器运行时，与 JDK 分离以支持多版本并存）
    pub fn is_valid_kind(k: &str) -> bool {
        matches!(k, "php" | "java" | "node" | "python" | "go" | "tomcat")
    }

    /// 合法安装状态白名单
    pub fn is_valid_status(s: &str) -> bool {
        matches!(
            s,
            "not_installed" | "installing" | "installed" | "running" | "error"
        )
    }

    /// 是否已安装（installing/installed/running 均视为占用中）
    pub fn is_installed(&self) -> bool {
        matches!(self.status.as_str(), "installing" | "installed" | "running")
    }
}

/// 用户项目（M-B 主线二 Stage A-1 · Doc/20 §5.1/5.2）——绑定 Runtime + 运行方式
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Project {
    pub id: i64,
    pub name: String,
    /// 绑定的运行时 ID（static 项目可为空）
    pub runtime_id: Option<i64>,
    /// 项目类型：php | jar | war | node | python | go | static
    pub kind: String,
    /// 来源：local_path | git
    pub source: String,
    /// git 仓库地址（source = git 时必填）
    pub source_url: Option<String>,
    /// 部署路径：/opt/flamepanel/projects/<name>
    pub deploy_path: String,
    /// 启动命令（模板变量 $RUNTIME / $DEPLOY）
    pub run_command: Option<String>,
    pub port: Option<i64>,
    /// 可选绑定网站域名
    pub domain: Option<String>,
    /// 项目状态：building | running | stopped | error
    pub status: String,
    /// systemd unit 名（flame-proj-<id>.service）
    pub process_unit: Option<String>,
    /// 部署方式：manual | webhook（M-B 主线二 Stage A-7 · Doc/20 §5.8）
    #[serde(default = "default_deploy_type")]
    pub deploy_type: String,
    /// webhook 自动部署开关（deploy_type = webhook 时生效）
    #[serde(default)]
    pub auto_deploy: bool,
    /// Webhook 部署令牌（明文仅创建/重置时返回一次；服务层校验用）
    #[serde(default)]
    pub webhook_token: String,
    /// Procfile 进程类型声明 JSON（Stage A-8 · Doc/20 §5.8 ③）：
    /// `{"web":"node server.js","worker":"node worker.js"}`；无 Procfile 时为 None。
    #[serde(default)]
    pub procfile_json: Option<String>,
    /// 构建器协商结果（Stage A-8 · Doc/20 §5.8 ②）：
    /// dockerfile | buildpacks | runtime（部署时按 Dockerfile → pack-cli → Runtime 顺序探测落库）
    #[serde(default = "default_build_type")]
    pub build_type: String,
    /// 健康检查路径（Stage A-9 · Doc/20 §5.8 ④ 零停机切换）；
    /// 空串表示未启用健康检查（restart 直接切换）。
    #[serde(default)]
    pub health_check_path: String,
    /// 部署切换策略：restart（直接重启）| rolling（健康检查零停机）
    #[serde(default = "default_deploy_strategy")]
    pub deploy_strategy: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_deploy_type() -> String {
    "manual".to_string()
}

fn default_build_type() -> String {
    "runtime".to_string()
}

fn default_deploy_strategy() -> String {
    "restart".to_string()
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            runtime_id: None,
            kind: "static".to_string(),
            source: "local_path".to_string(),
            source_url: None,
            deploy_path: String::new(),
            run_command: None,
            port: None,
            domain: None,
            status: "stopped".to_string(),
            process_unit: None,
            deploy_type: "manual".to_string(),
            auto_deploy: false,
            webhook_token: String::new(),
            procfile_json: None,
            build_type: "runtime".to_string(),
            health_check_path: String::new(),
            deploy_strategy: "restart".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Project {
    /// 合法项目类型白名单（Doc/20 §5.2）
    pub fn is_valid_kind(k: &str) -> bool {
        matches!(
            k,
            "php" | "jar" | "war" | "node" | "python" | "go" | "static"
        )
    }

    /// 合法来源白名单
    pub fn is_valid_source(s: &str) -> bool {
        matches!(s, "local_path" | "git")
    }

    /// 合法项目状态白名单
    pub fn is_valid_status(s: &str) -> bool {
        matches!(s, "building" | "running" | "stopped" | "error")
    }

    /// 合法部署方式白名单（Doc/20 §5.8：部署通道先落 Git Webhook）
    pub fn is_valid_deploy_type(s: &str) -> bool {
        matches!(s, "manual" | "webhook")
    }

    /// 合法构建器白名单（Doc/20 §5.8 ② 构建器自动协商）
    pub fn is_valid_build_type(s: &str) -> bool {
        matches!(s, "dockerfile" | "buildpacks" | "runtime")
    }

    /// 合法切换策略白名单（Stage A-9 · Doc/20 §5.8 ④）
    pub fn is_valid_deploy_strategy(s: &str) -> bool {
        matches!(s, "restart" | "rolling")
    }

    /// 健康检查完整 URL（回环绑定；未配置路径或端口时返回 None）。
    pub fn health_check_url(&self) -> Option<String> {
        if self.health_check_path.is_empty() {
            return None;
        }
        let port = self.port?;
        Some(format!(
            "http://127.0.0.1:{}{}",
            port,
            if self.health_check_path.starts_with('/') {
                self.health_check_path.clone()
            } else {
                format!("/{}", self.health_check_path)
            }
        ))
    }

    /// 解析 Procfile 进程类型声明（Stage A-8 · Doc/20 §5.8 ③）。
    /// 返回 (进程类型 → 启动命令) 列表；未声明 Procfile 返回空表。
    pub fn procfile_processes(&self) -> Vec<(String, String)> {
        self.procfile_json
            .as_deref()
            .and_then(|j| {
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(j).ok()
            })
            .map(|m| {
                m.into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|c| (k, c.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// 项目数据库联链（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式）。
///
/// 一条记录 = 项目 ↔ 数据库实例的一次关联；连接串不落库——由服务层在
/// ensure_unit（systemd `Environment=` 注入）或 docker run（`-e` 注入）时
/// 按 `db_type` 现场生成，凭据只在服务内存与进程环境中流转。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProjectDbLink {
    pub id: i64,
    /// 项目 ID（指向 projects）
    pub project_id: i64,
    /// 数据库实例 ID（指向 database_instances）
    pub db_instance_id: i64,
    /// 环境变量名：DATABASE_URL | REDIS_URL | MYSQL_URL 等（按 db_type 推导的默认值，
    /// 允许用户自定义别名，同一项目内唯一）
    pub env_name: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl ProjectDbLink {
    /// 同一项目内环境变量名合法性：大写字母/数字/下划线，≤64 字符。
    pub fn is_valid_env_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    }
}
