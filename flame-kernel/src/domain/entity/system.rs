//! 用户、节点、权限与角色、面板设置、操作 / 审计日志。

use super::*;

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Memo {
    pub id: i64,
    pub content: String,
    /// memo | todo
    pub kind: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
            key: "menu_accordion".into(),
            value: "false".into(),
            description: "侧边栏手风琴模式（同时只展开一个分组）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "app_signature_policy".into(),
            value: "optional".into(),
            description: "应用包验签策略（optional：源配公钥才验签 / required：远程包强制验签）"
                .into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "appstore_registry_whitelist".into(),
            value: String::new(),
            description: "应用商店镜像仓库白名单（逗号分隔，追加到内置白名单之后）".into(),
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
        PanelSetting {
            key: "allowed_ips".into(),
            value: "".into(),
            description: "授权访问 IP / CIDR（逗号分隔，空 = 不限制；本机回环始终放行）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "bind_domains".into(),
            value: "".into(),
            description: "绑定访问域名（逗号分隔，支持 *. 前缀，空 = 不限制）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "login_max_failures".into(),
            value: "5".into(),
            description: "登录连续失败锁定阈值（次，按用户名 + IP）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "login_lockout_minutes".into(),
            value: "5".into(),
            description: "登录失败锁定时长（分钟）".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "audit_log_retention_days".into(),
            value: "90".into(),
            description: "操作审计日志保留天数".into(),
            updated_at: Utc::now(),
        },
        PanelSetting {
            key: "server_address".into(),
            value: "".into(),
            description: "面板对外访问地址（通知与链接中使用，可留空）".into(),
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
        ("certificate", "read", "查看证书"),
        ("certificate", "update", "签发/上传/绑定证书"),
        ("certificate", "delete", "删除证书"),
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
        ("project", "read", "查看项目"),
        ("project", "create", "创建/部署项目"),
        ("project", "update", "启停/修改项目"),
        ("project", "delete", "删除项目"),
        ("runtime", "read", "查看运行环境"),
        ("runtime", "create", "安装运行时"),
        ("runtime", "update", "设默认运行时"),
        ("runtime", "delete", "卸载运行时"),
        ("backup", "read", "查看备份"),
        ("backup", "create", "创建/恢复备份"),
        ("backup", "update", "更新备份账号/触发同步"),
        ("backup", "delete", "删除备份"),
        ("dns_provider", "read", "查看 DNS 厂商账号"),
        ("dns_provider", "create", "创建 DNS 厂商账号"),
        ("dns_provider", "update", "修改/测试 DNS 厂商账号"),
        ("dns_provider", "delete", "删除 DNS 厂商账号"),
        ("waf", "read", "查看 WAF 规则与拦截日志"),
        ("waf", "create", "创建 WAF 规则"),
        ("waf", "update", "修改 WAF 规则/清理拦截日志"),
        ("waf", "delete", "删除 WAF 规则"),
        ("scheduled_task", "read", "查看定时任务"),
        ("scheduled_task", "create", "创建定时任务"),
        ("scheduled_task", "update", "修改定时任务"),
        ("scheduled_task", "delete", "删除定时任务"),
        ("scheduled_task", "execute", "执行定时任务"),
        ("task", "read", "查看任务"),
        ("task", "execute", "取消/执行任务"),
        ("task", "delete", "清理任务"),
        ("terminal", "execute", "打开 Web 终端执行命令"),
        ("toolbox", "read", "查看系统体检报告与进程列表"),
        ("toolbox", "execute", "执行系统体检、结束进程"),
        ("mcp", "read", "查看 AI 接入凭证、会话与工具调用审计"),
        ("mcp", "create", "签发 / 吊销本人的 AI 接入凭证"),
        (
            "mcp",
            "delete",
            "吊销他人的 AI 接入凭证；批准高危（destructive）AI 变更",
        ),
        // 追加在末尾：权限 id 按位置生成，插在中间会让已有 id 漂移
        ("mcp", "update", "审批 AI 写操作请求（批准 / 拒绝）"),
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
