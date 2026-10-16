//! MCP 工具目录：M-AI-1 首批 12 个只读工具 + M-AI-2 写操作工具（首批 5 个 + Batch 2：4 个写工具与 2 个配套只读工具）（Doc/30 · Doc/34）。
//!
//! **变更闸门（M-AI-2）**：每个工具声明 [`RiskLevel`]。只读工具调用即执行；写 / 高危工具
//! 在 `tools/call` 时只做参数校验与 [`preview`]（dry-run），生成审批单，**必须**由面板用户在
//! 「AI 接入中心 → 审批」批准后才经 [`execute_approved`] 执行。[`execute`] 拒绝执行任何非只读工具。
//!
//! **权限不另立一套**：每个工具声明一个「等价 REST 路由」样例，所需权限在运行时经
//! [`crate::api::permissions::route_permission`] 从 REST 权限表推导——MCP 与 REST 权限永不漂移，
//! 单测 `every_tool_maps_to_a_read_permission` 保证推导结果存在且为只读动作。
//!
//! **输出脱敏**：结构化结果统一经 [`redact`] 处理（`password`/`secret`/`token` 等键、
//! `KEY=VALUE` 形式的环境变量值替换为 `***`）；`files_read` 拒绝读取密钥类文件。
use crate::api::types::AppState;
use crate::core::error::AppError;
pub use crate::domain::mcp::RiskLevel;
use serde_json::{json, Map, Value};

/// 单个工具输出文本上限（字节）；超出截断并提示。
pub const MAX_OUTPUT_BYTES: usize = 100 * 1024;
/// `files_read` 默认 / 最大读取字节数。
const FILE_READ_DEFAULT: usize = 64 * 1024;
const FILE_READ_MAX: usize = 256 * 1024;

/// 工具定义。
pub struct ToolDef {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// 等价 REST 路由样例，用于推导权限
    pub rest_equivalent: &'static str,
    /// 等价 REST 路由的方法（`GET` / `POST`）
    pub rest_method: &'static str,
    /// 风险分级：非只读工具必须经人工审批
    pub risk: RiskLevel,
    pub input_schema: fn() -> Value,
}

impl ToolDef {
    /// 由 REST 权限表推导出的 `(resource, action)`。
    pub fn permission(&self) -> Option<(&'static str, &'static str)> {
        let method = axum::http::Method::from_bytes(self.rest_method.as_bytes()).ok()?;
        crate::api::permissions::route_permission(&method, self.rest_equivalent)
    }

    pub fn is_read_only(&self) -> bool {
        self.risk == RiskLevel::Read
    }

    /// `tools/list` 中的描述对象。
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "title": self.title,
            "description": self.description,
            "inputSchema": (self.input_schema)(),
            "annotations": {
                "title": self.title,
                "readOnlyHint": self.is_read_only(),
                "destructiveHint": self.risk == RiskLevel::Destructive,
                "idempotentHint": self.is_read_only(),
                "openWorldHint": false,
            },
        })
    }
}

fn no_args() -> Value {
    json!({"type": "object", "properties": {}, "additionalProperties": false})
}

fn container_args() -> Value {
    json!({
        "type": "object",
        "properties": {"container": {"type": "string", "description": "容器 ID 或名称"}},
        "required": ["container"],
        "additionalProperties": false,
    })
}

fn docker_logs_args() -> Value {
    json!({
        "type": "object",
        "properties": {
            "container": {"type": "string", "description": "容器 ID 或名称"},
            "tail": {"type": "integer", "minimum": 1, "maximum": 2000, "default": 100, "description": "返回最近多少行"},
        },
        "required": ["container"],
        "additionalProperties": false,
    })
}

fn id_args() -> Value {
    json!({
        "type": "object",
        "properties": {"id": {"type": "integer", "minimum": 1}},
        "required": ["id"],
        "additionalProperties": false,
    })
}

fn logs_query_args() -> Value {
    json!({
        "type": "object",
        "properties": {
            "levels": {"type": "string", "description": "逗号分隔的级别过滤，如 `warn,error`；缺省不过滤"},
            "q": {"type": "string", "description": "关键字（不区分大小写）"},
            "target": {"type": "string", "description": "target 前缀，如 `flame_kernel::api`"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 500, "default": 100},
        },
        "additionalProperties": false,
    })
}

fn container_stop_args() -> Value {
    json!({
        "type": "object",
        "properties": {
            "container": {"type": "string", "description": "容器 ID 或名称"},
            "timeout": {"type": "integer", "minimum": 1, "maximum": 300, "default": 10, "description": "等待优雅退出的秒数，超时后强制终止"},
        },
        "required": ["container"],
        "additionalProperties": false,
    })
}

fn container_remove_args() -> Value {
    json!({
        "type": "object",
        "properties": {
            "container": {"type": "string", "description": "容器 ID 或名称"},
            "force": {"type": "boolean", "default": false, "description": "运行中的容器需 true 才能删除"},
        },
        "required": ["container"],
        "additionalProperties": false,
    })
}

fn files_read_args() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "文件路径（文件管理根目录内）"},
            "max_bytes": {"type": "integer", "minimum": 1, "maximum": FILE_READ_MAX, "default": FILE_READ_DEFAULT},
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

/// 工具目录（顺序即 `tools/list` 顺序）。
pub static TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "system_status",
        title: "主机状态",
        description: "当前主机 CPU / 内存 / 磁盘 / 负载 / 网络速率快照，以及面板版本。",
        rest_equivalent: "/api/metrics/history",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "docker_list",
        title: "容器列表",
        description: "列出本机全部 Docker 容器（ID、名称、镜像、状态、端口）。",
        rest_equivalent: "/api/docker/containers",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "docker_logs",
        title: "容器日志",
        description: "读取容器最近 N 行日志（默认 100，最多 2000）。判断 OOM 时配合 docker_inspect 的 summary.oom_killed / exit_code（137）使用。",
        rest_equivalent: "/api/docker/containers/x/logs",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: docker_logs_args,
    },
    ToolDef {
        name: "docker_inspect",
        title: "容器详情",
        description: "容器 inspect 详情（环境变量等敏感值已脱敏），附 summary：状态、退出码、是否被 OOM Killer 终止、重启次数、启动/结束时间。",
        rest_equivalent: "/api/docker/containers/x/inspect",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: container_args,
    },
    ToolDef {
        name: "task_get",
        title: "任务详情",
        description: "查询统一任务（安装、部署、备份等长任务）的状态、进度与消息。",
        rest_equivalent: "/api/tasks/1",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: id_args,
    },
    ToolDef {
        name: "logs_query",
        title: "面板日志",
        description: "按级别 / 关键字 / target 查询面板内核日志（最新在前）。",
        rest_equivalent: "/api/kernel-logs/entries",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: logs_query_args,
    },
    ToolDef {
        name: "files_read",
        title: "读取文件",
        description: "读取文件管理根目录内的文本文件（默认前 64 KiB，最多 256 KiB）。私钥、shadow、.env、数据库文件等敏感文件拒绝读取。",
        rest_equivalent: "/api/files/read",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: files_read_args,
    },
    ToolDef {
        name: "website_list",
        title: "网站列表",
        description: "列出面板管理的全部网站（域名、类型、引擎、状态、根目录）。",
        rest_equivalent: "/api/websites",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "website_config",
        title: "网站配置",
        description: "单个网站的完整配置（域名、SSL、反代、运行时绑定）及伪静态状态。",
        rest_equivalent: "/api/websites/1",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: id_args,
    },
    ToolDef {
        name: "database_list",
        title: "数据库实例",
        description: "列出数据库实例（类型、版本、端口、状态），不含任何口令。",
        rest_equivalent: "/api/databases",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "appstore_installed",
        title: "已装应用",
        description: "列出应用商店已安装应用（版本、模式、状态、访问地址、容器名）；安装参数不返回。",
        rest_equivalent: "/api/app-store/installed",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "firewall_rules",
        title: "防火墙规则",
        description: "列出面板管理的防火墙规则（端口、协议、来源、动作、启用状态）。",
        rest_equivalent: "/api/firewall/rules",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "approval_get",
        title: "审批状态",
        description: "查询本凭证发起的写操作审批单：状态（pending / executing / succeeded / failed / rejected / expired）、预览、执行结果或拒绝理由。",
        rest_equivalent: "/api/mcp/approvals/1",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: id_args,
    },
    ToolDef {
        name: "backup_job_list",
        title: "备份任务",
        description: "列出已配置的备份任务：范围、计划、目标账号、保留份数、是否启用、最近执行状态与信息、下次执行时间。",
        rest_equivalent: "/api/backup-jobs",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    ToolDef {
        name: "scheduled_task_list",
        title: "定时任务",
        description: "列出定时任务（最多 200 条）：计划、是否启用、最近执行状态与输出摘要、下次执行时间；命令中的密码类参数已脱敏。",
        rest_equivalent: "/api/scheduled-tasks",
        rest_method: "GET",
        risk: RiskLevel::Read,
        input_schema: no_args,
    },
    // ── M-AI-2 写操作工具：调用只生成审批单，批准后才执行 ──
    ToolDef {
        name: "docker_container_start",
        title: "启动容器",
        description: "启动已停止的容器。【需人工审批】调用后返回审批单号与预览，面板批准后才执行；用 approval_get 查询结果。",
        rest_equivalent: "/api/docker/containers/x/start",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: container_args,
    },
    ToolDef {
        name: "docker_container_stop",
        title: "停止容器",
        description: "停止运行中的容器（默认等待 10 秒优雅退出）。【需人工审批】",
        rest_equivalent: "/api/docker/containers/x/stop",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: container_stop_args,
    },
    ToolDef {
        name: "docker_container_restart",
        title: "重启容器",
        description: "重启容器（常用于 OOM / 卡死后恢复服务）。【需人工审批】",
        rest_equivalent: "/api/docker/containers/x/restart",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: container_stop_args,
    },
    ToolDef {
        name: "website_deploy",
        title: "重新下发站点配置",
        description: "按面板中的网站配置重新渲染并下发到 Web 引擎，校验通过后 reload（配置漂移 / 手工改坏后恢复）。【需人工审批】",
        rest_equivalent: "/api/websites/1/deploy",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: id_args,
    },
    ToolDef {
        name: "docker_container_remove",
        title: "删除容器",
        description: "删除容器（不删除镜像与数据卷）。不可逆。【需人工审批，且批准者需具备高危操作权限】",
        rest_equivalent: "/api/docker/containers/x/remove",
        rest_method: "POST",
        risk: RiskLevel::Destructive,
        input_schema: container_remove_args,
    },
    // ── M-AI-2 Batch 2 ──
    ToolDef {
        name: "app_native_restart",
        title: "重启原生应用",
        description: "重启应用商店中以原生模式安装的应用服务（容器模式应用请用 docker_container_restart）。【需人工审批】",
        rest_equivalent: "/api/app-store/installed/1/native-restart",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: id_args,
    },
    ToolDef {
        name: "backup_create",
        title: "立即备份面板",
        description: "立即为面板数据库创建一份本地快照备份（不中断服务），适合在变更前留存回滚点。【需人工审批】",
        rest_equivalent: "/api/backups",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: no_args,
    },
    ToolDef {
        name: "backup_job_run",
        title: "立即执行备份任务",
        description: "立即执行一次已配置的备份任务（上传到其目标账号，并按保留份数清理旧备份）。批准后在后台执行，结果见任务的最近执行状态。【需人工审批】",
        rest_equivalent: "/api/backup-jobs/1/run",
        rest_method: "POST",
        risk: RiskLevel::Write,
        input_schema: id_args,
    },
    ToolDef {
        name: "scheduled_task_run",
        title: "立即执行定时任务",
        description: "立即执行一次已配置的定时任务命令（以面板进程身份运行，最长 60 秒），返回退出状态与输出。命令内容由管理员预先配置，本工具不能修改命令。【需人工审批，且批准者需具备高危操作权限】",
        rest_equivalent: "/api/scheduled-tasks/1/run",
        rest_method: "POST",
        risk: RiskLevel::Destructive,
        input_schema: id_args,
    },
];

pub fn find(name: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.name == name)
}

/// 工具执行结果：`Ok(text)` 成功文本；`Err(msg)` 为**工具级**错误（`isError: true`），
/// 不是 JSON-RPC 协议错误——让模型能看到原因并自行调整参数。
pub type ToolResult = Result<String, String>;

/// 调用方上下文。协议端点中为凭证签发者；审批执行时为批准者（审计记在批准者名下）。
pub struct Caller<'a> {
    pub username: &'a str,
    /// 发起调用的凭证（`approval_get` 只能查本凭证的审批单）
    pub principal_id: i64,
}

// ─── 参数解析 ────────────────────────────────────────────────────────────

fn args_obj(args: &Value) -> Result<&Map<String, Value>, String> {
    match args {
        Value::Object(m) => Ok(m),
        Value::Null => {
            static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
            Ok(EMPTY.get_or_init(Map::new))
        }
        _ => Err("arguments 必须是对象".into()),
    }
}

fn req_str<'a>(m: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    m.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("缺少必填字符串参数 `{key}`"))
}

fn opt_str<'a>(m: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    m.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn opt_bool(m: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match m.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("参数 `{key}` 必须是布尔值")),
    }
}

fn opt_int(m: &Map<String, Value>, key: &str, min: i64, max: i64) -> Result<Option<i64>, String> {
    match m.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_i64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
                .ok_or_else(|| format!("参数 `{key}` 必须是整数"))?;
            if n < min || n > max {
                return Err(format!("参数 `{key}` 需在 {min}–{max} 之间"));
            }
            Ok(Some(n))
        }
    }
}

fn req_int(m: &Map<String, Value>, key: &str, min: i64, max: i64) -> Result<i64, String> {
    opt_int(m, key, min, max)?.ok_or_else(|| format!("缺少必填整数参数 `{key}`"))
}

fn app_err(e: AppError) -> String {
    e.to_string()
}

fn to_text(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn ser<T: serde::Serialize>(v: &T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| format!("序列化失败: {e}"))
}

// ─── 脱敏 ────────────────────────────────────────────────────────────────

const SENSITIVE_KEY_PARTS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "apikey",
    "api_key",
    "private_key",
    "privatekey",
    "access_key",
    "accesskey",
    "credential",
    "auth",
];

fn is_sensitive_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    // `author` / `authority` 之类不是凭据
    if k.starts_with("author") {
        return false;
    }
    SENSITIVE_KEY_PARTS.iter().any(|p| k.contains(p))
}

/// 递归脱敏：敏感键的标量值 → `***`；字符串数组中的 `KEY=VALUE`（环境变量）按键名脱敏。
pub fn redact(v: &mut Value) {
    match v {
        Value::Object(m) => {
            for (k, val) in m.iter_mut() {
                if is_sensitive_key(k) && !matches!(val, Value::Object(_) | Value::Array(_)) {
                    if !val.is_null() && val.as_str() != Some("") {
                        *val = Value::String("***".into());
                    }
                } else {
                    redact(val);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                if let Value::String(s) = item {
                    if let Some((k, _)) = s.split_once('=') {
                        if is_sensitive_key(k) {
                            *s = format!("{k}=***");
                        }
                    }
                } else {
                    redact(item);
                }
            }
        }
        _ => {}
    }
}

/// 密钥类文件（MCP 不对 AI 暴露，即使调用者有 `file:read`）。
pub fn is_secret_path(path: &std::path::Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    const NAMES: &[&str] = &[
        "shadow",
        "shadow-",
        "gshadow",
        "gshadow-",
        ".env",
        ".netrc",
        ".git-credentials",
        ".pgpass",
        ".my.cnf",
        "jwt_secret",
        "authorized_keys",
    ];
    const EXTS: &[&str] = &[
        ".key",
        ".p12",
        ".pfx",
        ".keystore",
        ".jks",
        ".db",
        ".sqlite",
        ".sqlite3",
        ".kdbx",
    ];
    if NAMES.contains(&name.as_str()) || name.starts_with(".env.") {
        return true;
    }
    if EXTS.iter().any(|e| name.ends_with(e)) {
        return true;
    }
    if (name.starts_with("id_") || lower.contains("/.ssh/")) && !name.ends_with(".pub") {
        return true;
    }
    lower.contains("/.gnupg/") || lower.contains("/private/") && name.ends_with(".pem")
}

// ─── 执行 ────────────────────────────────────────────────────────────────

/// 执行**只读**工具（权限已由调用方校验）。非只读工具一律拒绝——写操作只能经审批执行。
pub async fn execute(
    state: &AppState,
    caller: &Caller<'_>,
    tool: &ToolDef,
    args: &Value,
) -> ToolResult {
    if !tool.is_read_only() {
        return Err(format!("工具 {} 需人工审批，不能直接执行", tool.name));
    }
    let m = args_obj(args)?;
    let out = match tool.name {
        "system_status" => {
            let snap = state.metrics.latest();
            let mut v = json!({
                "panel_version": env!("CARGO_PKG_VERSION"),
                "metrics": snap.as_ref().map(ser).transpose()?,
            });
            if snap.is_none() {
                v["note"] = json!("指标采集尚未产出首个样本，请稍后重试");
            }
            to_text(&v)
        }
        "docker_list" => {
            let list = state
                .docker_service
                .list_containers(0)
                .await
                .map_err(app_err)?;
            let mut v = json!({"count": list.len(), "containers": ser(&list)?});
            redact(&mut v);
            to_text(&v)
        }
        "docker_logs" => {
            let id = req_str(m, "container")?;
            let tail = opt_int(m, "tail", 1, 2000)?.unwrap_or(100) as usize;
            let logs = state
                .docker_service
                .get_container_logs(id, tail)
                .await
                .map_err(app_err)?;
            if logs.trim().is_empty() {
                format!("容器 {id} 最近 {tail} 行无日志输出")
            } else {
                logs
            }
        }
        "docker_inspect" => {
            let id = req_str(m, "container")?;
            let mut raw = state
                .docker_service
                .inspect_container(id)
                .await
                .map_err(app_err)?;
            let st = raw.get("State").cloned().unwrap_or(Value::Null);
            let summary = json!({
                "name": raw.get("Name"),
                "image": raw.pointer("/Config/Image"),
                "status": st.get("Status"),
                "running": st.get("Running"),
                "exit_code": st.get("ExitCode"),
                "oom_killed": st.get("OOMKilled"),
                "error": st.get("Error"),
                "restart_count": raw.get("RestartCount"),
                "started_at": st.get("StartedAt"),
                "finished_at": st.get("FinishedAt"),
                "memory_limit": raw.pointer("/HostConfig/Memory"),
            });
            redact(&mut raw);
            to_text(&json!({"summary": summary, "inspect": raw}))
        }
        "task_get" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let rec = state.task_service.get_task(id as u64).map_err(app_err)?;
            to_text(&ser(&rec)?)
        }
        "logs_query" => {
            let filter = crate::domain::kernel_log::LogFilter::from_params(
                opt_str(m, "levels"),
                opt_str(m, "q"),
                opt_str(m, "target"),
            )?;
            let limit = opt_int(m, "limit", 1, 500)?.unwrap_or(100) as usize;
            let page = state
                .kernel_log_service
                .entries(None, filter, limit, None)
                .await
                .map_err(app_err)?;
            to_text(&json!({
                "file": page.file,
                "count": page.entries.len(),
                "entries": ser(&page.entries)?,
                "more": page.next_before.is_some(),
            }))
        }
        "files_read" => files_read(state, caller, m).await?,
        "website_list" => {
            let list = state
                .website_service
                .list_websites()
                .await
                .map_err(app_err)?;
            let mut v = json!({"count": list.len(), "websites": ser(&list)?});
            redact(&mut v);
            to_text(&v)
        }
        "website_config" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let site = state
                .website_service
                .get_website(id)
                .await
                .map_err(app_err)?;
            let rewrite = state.website_service.get_rewrite_preset(id).await.ok();
            let mut v = json!({
                "website": ser(&site)?,
                "rewrite": rewrite.as_ref().map(ser).transpose()?,
            });
            redact(&mut v);
            to_text(&v)
        }
        "database_list" => {
            let list = state
                .database_service
                .list_instances()
                .await
                .map_err(app_err)?;
            let mut v = json!({"count": list.len(), "instances": ser(&list)?});
            redact(&mut v);
            to_text(&v)
        }
        "appstore_installed" => {
            let list = state
                .app_store_service
                .list_installed()
                .await
                .map_err(app_err)?;
            let mut v = ser(&list)?;
            if let Value::Array(items) = &mut v {
                for it in items.iter_mut() {
                    if let Value::Object(o) = it {
                        // 安装参数常含数据库口令等，整体不下发
                        o.remove("params_json");
                    }
                }
            }
            let mut v = json!({"count": list.len(), "apps": v});
            redact(&mut v);
            to_text(&v)
        }
        "firewall_rules" => {
            let list = state.firewall_service.list_rules().await.map_err(app_err)?;
            to_text(&json!({"count": list.len(), "rules": ser(&list)?}))
        }
        "backup_job_list" => {
            let list = state
                .services
                .backup_job_service
                .list_jobs()
                .await
                .map_err(app_err)?;
            let mut v = json!({"count": list.len(), "jobs": ser(&list)?});
            redact(&mut v);
            to_text(&v)
        }
        "scheduled_task_list" => {
            let params = crate::api::types::PaginationParams {
                page: Some(1),
                page_size: Some(200),
            };
            let page = state
                .scheduled_task_service
                .list_tasks(&params)
                .await
                .map_err(app_err)?;
            let tasks: Vec<Value> = page
                .data
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "name": t.name,
                        "command": mask_command(&t.command),
                        "schedule": t.schedule,
                        "enabled": t.enabled,
                        "last_status": t.last_status,
                        "last_output": crate::application::mcp_service::clip_bytes(&t.last_output, 512),
                        "last_run_at": t.last_run_at,
                        "next_run_at": t.next_run_at,
                    })
                })
                .collect();
            to_text(&json!({"total": page.total, "tasks": tasks}))
        }
        "approval_get" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let a = state
                .mcp_service
                .get_approval(id)
                .await
                .ok()
                .filter(|a| a.principal_id == caller.principal_id)
                .ok_or_else(|| format!("审批单 #{id} 不存在"))?;
            to_text(&json!({
                "id": a.id,
                "tool": a.tool,
                "risk": a.risk,
                "status": a.status,
                "preview": a.preview,
                "requested_at": a.requested_at,
                "expires_at": a.expires_at,
                "decided_at": a.decided_at,
                "reason": a.reason,
                "result": a.result,
            }))
        }
        other => return Err(format!("未实现的工具: {other}")),
    };
    Ok(clip_output(out))
}

// ─── 写操作：预览（dry-run）与批准后执行 ──────────────────────────────────

/// 读取容器摘要（名称 / 镜像 / 状态），用于预览与前置校验。
async fn container_brief(state: &AppState, id: &str) -> Result<(String, String, bool), String> {
    let raw = state
        .docker_service
        .inspect_container(id)
        .await
        .map_err(|e| format!("找不到容器 {id}: {e}"))?;
    let name = raw
        .get("Name")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .trim_start_matches('/')
        .to_string();
    let image = raw
        .pointer("/Config/Image")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    let running = raw
        .pointer("/State/Running")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok((name, image, running))
}

/// 写工具的预览：校验参数与目标存在，描述将要发生的变更与当前状态。**不产生任何副作用。**
/// 返回 `Err` 时不创建审批单（让模型修正参数）。
pub async fn preview(state: &AppState, tool: &ToolDef, args: &Value) -> ToolResult {
    let m = args_obj(args)?;
    let state_word = |running: bool| if running { "运行中" } else { "已停止" };
    match tool.name {
        "docker_container_start" => {
            let (name, image, running) = container_brief(state, req_str(m, "container")?).await?;
            let note = if running {
                "（已在运行，执行后无变化）"
            } else {
                ""
            };
            Ok(format!(
                "启动容器 {name}（镜像 {image}，当前{}）{note}",
                state_word(running)
            ))
        }
        "docker_container_stop" | "docker_container_restart" => {
            let timeout = opt_int(m, "timeout", 1, 300)?.unwrap_or(10);
            let (name, image, running) = container_brief(state, req_str(m, "container")?).await?;
            let verb = if tool.name == "docker_container_stop" {
                "停止"
            } else {
                "重启"
            };
            Ok(format!(
                "{verb}容器 {name}（镜像 {image}，当前{}），优雅退出等待 {timeout} 秒；期间该容器提供的服务中断",
                state_word(running)
            ))
        }
        "docker_container_remove" => {
            let force = opt_bool(m, "force")?;
            let (name, image, running) = container_brief(state, req_str(m, "container")?).await?;
            if running && !force {
                return Err(format!(
                    "容器 {name} 正在运行：先停止，或传 force=true 强制删除"
                ));
            }
            Ok(format!(
                "删除容器 {name}（镜像 {image}，当前{}{}）。不可逆；镜像与数据卷保留",
                state_word(running),
                if force { "，强制删除" } else { "" }
            ))
        }
        "website_deploy" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let site = state
                .website_service
                .get_website(id)
                .await
                .map_err(app_err)?;
            Ok(format!(
                "重新渲染并下发网站 #{id} {}（引擎 {}，类型 {}）的站点配置，校验通过后 reload 引擎",
                site.domain, site.engine, site.site_type
            ))
        }
        "app_native_restart" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let app = state
                .app_store_service
                .get_installed(id)
                .await
                .map_err(app_err)?;
            if !app.mode.eq_ignore_ascii_case("native") {
                let hint = app
                    .container_name
                    .as_deref()
                    .map(|c| format!("，可用 docker_container_restart 重启容器 {c}"))
                    .unwrap_or_default();
                return Err(format!(
                    "应用 {}（#{id}）是 {} 模式，不是原生应用{hint}",
                    app.name, app.mode
                ));
            }
            Ok(format!(
                "重启原生应用 {}（#{id}，{} {}，当前状态 {}）；重启期间该服务短暂不可用",
                app.name, app.package_key, app.version, app.status
            ))
        }
        "backup_create" => {
            Ok("立即为面板数据库创建一份本地快照备份（写入备份目录，不中断服务）".to_string())
        }
        "backup_job_run" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let job = state
                .services
                .backup_job_service
                .get_job(id)
                .await
                .map_err(app_err)?;
            let scope = match job.scope_key.as_deref() {
                Some(k) if !k.is_empty() => format!("{}:{k}", job.scope),
                _ => job.scope.clone(),
            };
            let retention = if job.retention > 0 {
                format!("保留 {} 份，超出将删除最旧的远端备份", job.retention)
            } else {
                "不清理旧备份".to_string()
            };
            Ok(format!(
                "立即执行备份任务「{}」（#{id}，范围 {scope}，目标账号 #{}，{retention}）；批准后在后台执行，上次结果：{}",
                job.name, job.target_account_id, job.last_status
            ))
        }
        "scheduled_task_run" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let task = state
                .scheduled_task_service
                .get_task(id)
                .await
                .map_err(app_err)?;
            Ok(format!(
                "立即执行定时任务「{}」（#{id}，计划 {}）。将以面板进程身份运行以下命令（最长 60 秒）：\n{}",
                task.name,
                task.schedule,
                mask_command(&task.command)
            ))
        }
        other => Err(format!("工具 {other} 不是写操作工具")),
    }
}

/// 预览中展示 shell 命令前的脱敏：`KEY=VALUE` / `--opt=VALUE` 中敏感键的值、`-p<密码>`（mysql 风格）替换为 `***`，
/// 并截断到 500 字节。仅用于展示——执行时使用原命令。
fn mask_command(cmd: &str) -> String {
    let masked: Vec<String> = cmd
        .split(' ')
        .map(|tok| {
            if let Some((k, _)) = tok.split_once('=') {
                if is_sensitive_key(k.trim_start_matches('-')) {
                    return format!("{k}=***");
                }
            }
            if tok.len() > 2 && tok.starts_with("-p") && !tok.starts_with("--") {
                return "-p***".to_string();
            }
            tok.to_string()
        })
        .collect();
    let joined = masked.join(" ");
    if joined.len() <= 500 {
        joined
    } else {
        format!(
            "{}…",
            crate::application::mcp_service::clip_bytes(&joined, 500)
        )
    }
}

/// 执行**已批准**的写工具（仅由审批接口调用；权限与状态机已由调用方校验）。
pub async fn execute_approved(
    state: &AppState,
    _caller: &Caller<'_>,
    tool: &ToolDef,
    args: &Value,
) -> ToolResult {
    if tool.is_read_only() {
        return Err(format!("工具 {} 是只读工具", tool.name));
    }
    let m = args_obj(args)?;
    let out = match tool.name {
        "docker_container_start" => {
            let id = req_str(m, "container")?;
            state
                .docker_service
                .start_container(id)
                .await
                .map_err(app_err)?;
            format!("容器 {id} 已启动")
        }
        "docker_container_stop" => {
            let id = req_str(m, "container")?;
            let timeout = opt_int(m, "timeout", 1, 300)?.unwrap_or(10) as u64;
            state
                .docker_service
                .stop_container(id, timeout)
                .await
                .map_err(app_err)?;
            format!("容器 {id} 已停止")
        }
        "docker_container_restart" => {
            let id = req_str(m, "container")?;
            let timeout = opt_int(m, "timeout", 1, 300)?.unwrap_or(10) as u64;
            state
                .docker_service
                .restart_container(id, timeout)
                .await
                .map_err(app_err)?;
            format!("容器 {id} 已重启")
        }
        "docker_container_remove" => {
            let id = req_str(m, "container")?;
            let force = opt_bool(m, "force")?;
            state
                .docker_service
                .remove_container(id, force)
                .await
                .map_err(app_err)?;
            format!("容器 {id} 已删除")
        }
        "website_deploy" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let report = state
                .website_service
                .deploy_website(id)
                .await
                .map_err(app_err)?;
            to_text(&ser(&report)?)
        }
        "app_native_restart" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            state
                .app_store_service
                .native_app_restart(id)
                .await
                .map_err(app_err)?;
            format!("应用 #{id} 已重启")
        }
        "backup_create" => {
            let backup = state
                .backup_service
                .create_backup()
                .await
                .map_err(app_err)?;
            let _ = state
                .event_bus
                .publish(crate::domain::entity::DomainEvent::BackupCreated {
                    filename: backup.filename.clone(),
                })
                .await;
            format!("已创建备份 {}（{} 字节）", backup.filename, backup.size)
        }
        "backup_job_run" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let job = state
                .services
                .backup_job_service
                .start_now(id)
                .await
                .map_err(app_err)?;
            format!(
                "备份任务「{}」已在后台开始执行；稍后查询 GET /api/backup-jobs 的 last_status / last_message 获取结果",
                job.name
            )
        }
        "scheduled_task_run" => {
            let id = req_int(m, "id", 1, i64::MAX)?;
            let task = state
                .scheduled_task_service
                .run_now_exclusive(id)
                .await
                .map_err(app_err)?;
            let out = format!(
                "定时任务「{}」执行完毕，状态 {}\n输出：\n{}",
                task.name, task.last_status, task.last_output
            );
            if task.last_status != "success" {
                return Err(clip_output(out));
            }
            out
        }
        other => return Err(format!("未实现的写工具: {other}")),
    };
    // 操作日志由审批接口写入（带审批单号与批准者）
    Ok(clip_output(out))
}

async fn files_read(state: &AppState, caller: &Caller<'_>, m: &Map<String, Value>) -> ToolResult {
    let path = req_str(m, "path")?;
    let max = opt_int(m, "max_bytes", 1, FILE_READ_MAX as i64)?.unwrap_or(FILE_READ_DEFAULT as i64)
        as usize;
    let svc = crate::file::FileService::new(state.file_root.clone());
    // 请求路径与解析后的真实路径都要过敏感名单（防符号链接绕过）
    let resolved = svc.sanitize(path).map_err(app_err)?;
    if is_secret_path(std::path::Path::new(path)) || is_secret_path(&resolved) {
        return Err(format!("出于安全考虑，MCP 不允许读取敏感文件: {path}"));
    }
    let content = svc.read(path).await.map_err(app_err)?;
    state
        .operation_log_service
        .log(caller.username, "mcp_file_read", Some(path), None)
        .await
        .ok();
    let total = content.len();
    if total <= max {
        return Ok(content);
    }
    let clipped = crate::application::mcp_service::clip_bytes(&content, max);
    Ok(format!(
        "{clipped}\n\n[已截断：显示前 {max} 字节，文件共 {total} 字节；可调大 max_bytes（上限 {FILE_READ_MAX}）]"
    ))
}

fn clip_output(s: String) -> String {
    if s.len() <= MAX_OUTPUT_BYTES {
        return s;
    }
    let total = s.len();
    format!(
        "{}\n\n[输出已截断：{} / {} 字节]",
        crate::application::mcp_service::clip_bytes(&s, MAX_OUTPUT_BYTES),
        MAX_OUTPUT_BYTES,
        total
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_level_matches_derived_permission() {
        assert_eq!(TOOLS.len(), 24);
        let mut names = std::collections::HashSet::new();
        for t in TOOLS {
            assert!(names.insert(t.name), "duplicate tool {}", t.name);
            let (res, act) = t
                .permission()
                .unwrap_or_else(|| panic!("tool {} has no REST permission", t.name));
            // 只读工具必须映射到 read；写工具必须映射到非 read（否则闸门形同虚设）
            assert_eq!(
                t.is_read_only(),
                act == "read",
                "tool {} risk {:?} vs {res}:{act}",
                t.name,
                t.risk
            );
            assert_eq!(t.is_read_only(), t.rest_method == "GET", "{}", t.name);
            let schema = (t.input_schema)();
            assert_eq!(schema["type"], "object");
            let ann = &t.to_json()["annotations"];
            assert_eq!(ann["readOnlyHint"], t.is_read_only(), "{}", t.name);
            assert_eq!(
                ann["destructiveHint"],
                t.risk == RiskLevel::Destructive,
                "{}",
                t.name
            );
        }
    }

    #[test]
    fn expected_permissions() {
        let p = |n: &str| find(n).unwrap().permission().unwrap();
        assert_eq!(p("docker_logs"), ("docker", "read"));
        assert_eq!(p("files_read"), ("file", "read"));
        assert_eq!(p("logs_query"), ("log", "read"));
        assert_eq!(p("firewall_rules").0, "firewall");
        assert_eq!(p("approval_get"), ("mcp", "read"));
        assert_eq!(p("docker_container_start"), ("docker", "start"));
        assert_eq!(p("docker_container_stop"), ("docker", "stop"));
        assert_eq!(p("docker_container_restart"), ("docker", "start"));
        assert_eq!(p("docker_container_remove"), ("docker", "delete"));
        assert_eq!(p("website_deploy"), ("website", "update"));
        assert_eq!(p("backup_job_list"), ("backup", "read"));
        assert_eq!(p("scheduled_task_list"), ("scheduled_task", "read"));
        assert_eq!(p("app_native_restart"), ("app_store", "update"));
        assert_eq!(p("backup_create"), ("backup", "create"));
        assert_eq!(p("backup_job_run"), ("backup", "update"));
        assert_eq!(p("scheduled_task_run"), ("scheduled_task", "execute"));
        assert_eq!(
            find("scheduled_task_run").unwrap().risk,
            RiskLevel::Destructive
        );
        assert_eq!(
            find("docker_container_remove").unwrap().risk,
            RiskLevel::Destructive
        );
    }

    #[test]
    fn runbooks_reference_existing_tools() {
        for rb in crate::domain::runbook::RUNBOOKS {
            for part in rb.template.split('`').skip(1).step_by(2) {
                assert!(
                    find(part).is_some(),
                    "runbook {} references unknown tool {part}",
                    rb.name
                );
            }
        }
    }

    #[test]
    fn mask_command_hides_secrets() {
        let m =
            mask_command("mysqldump -uroot -phunter2 --password=x DB_PASSWORD=y db > /tmp/a.sql");
        assert!(
            !m.contains("hunter2") && !m.contains("=x ") && !m.contains("=y"),
            "{m}"
        );
        assert!(
            m.contains("-p***") && m.contains("--password=***") && m.contains("DB_PASSWORD=***")
        );
        assert!(m.contains("-uroot") && m.contains("db > /tmp/a.sql"));
        // 普通参数保持原样；超长截断
        assert_eq!(mask_command("ls -la /var/log"), "ls -la /var/log");
        assert!(mask_command(&"x".repeat(800)).ends_with('…'));
    }

    #[test]
    fn opt_bool_is_strict() {
        let v = json!({"t": true, "s": "true"});
        let m = args_obj(&v).unwrap();
        assert!(opt_bool(m, "t").unwrap());
        assert!(!opt_bool(m, "missing").unwrap());
        assert!(opt_bool(m, "s").is_err());
    }

    #[test]
    fn redact_masks_sensitive_keys_and_env() {
        let mut v = json!({
            "name": "db",
            "Password": "hunter2",
            "author": "keep",
            "empty_token": "",
            "Config": {"Env": ["MYSQL_ROOT_PASSWORD=pw", "PATH=/usr/bin", "API_TOKEN=x"]},
            "nested": [{"secret_key": 42}],
        });
        redact(&mut v);
        assert_eq!(v["Password"], "***");
        assert_eq!(v["author"], "keep");
        assert_eq!(v["empty_token"], "");
        assert_eq!(v["Config"]["Env"][0], "MYSQL_ROOT_PASSWORD=***");
        assert_eq!(v["Config"]["Env"][1], "PATH=/usr/bin");
        assert_eq!(v["Config"]["Env"][2], "API_TOKEN=***");
        assert_eq!(v["nested"][0]["secret_key"], "***");
    }

    #[test]
    fn secret_paths_are_denied() {
        use std::path::Path;
        for p in [
            "/etc/shadow",
            "/root/.ssh/id_ed25519",
            "/home/a/.ssh/config",
            "/srv/app/.env",
            "/srv/app/.env.production",
            "/etc/ssl/site.key",
            "/opt/flamepanel/data/flamepanel.db",
            "/root/.gnupg/pubring.kbx",
        ] {
            assert!(is_secret_path(Path::new(p)), "{p} should be denied");
        }
        for p in [
            "/etc/nginx/nginx.conf",
            "/root/.ssh/id_ed25519.pub",
            "/var/log/syslog",
            "/etc/ssl/site.pem",
        ] {
            assert!(!is_secret_path(Path::new(p)), "{p} should be allowed");
        }
    }

    #[test]
    fn arg_helpers() {
        let v = json!({"a": " x ", "n": "5", "big": 99});
        let m = args_obj(&v).unwrap();
        assert_eq!(req_str(m, "a").unwrap(), "x");
        assert!(req_str(m, "missing").is_err());
        assert_eq!(opt_int(m, "n", 1, 10).unwrap(), Some(5));
        assert!(opt_int(m, "big", 1, 10).is_err());
        assert!(args_obj(&json!([1])).is_err());
        assert!(args_obj(&Value::Null).unwrap().is_empty());
    }

    #[test]
    fn clip_output_marks_truncation() {
        let s = "a".repeat(MAX_OUTPUT_BYTES + 10);
        let c = clip_output(s);
        assert!(c.contains("输出已截断"));
    }
}
