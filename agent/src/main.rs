use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    extract::Query,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sysinfo::System;

// ─── Agent 动作枚举（Phase A1：危险动作枚举化，禁止任意拼 Shell）────
// 每个变体对应一个明确、受限的操作，杜绝 Agent 成为无差别 Shell 网关。
/// Agent 动作枚举（Phase A1）：危险动作枚举化，禁止任意拼 Shell。
/// 使用 `#[serde(untagged)]` 以便同时支持 `{"action":"ping"}` 与 `{"action":"ping","params":{}}`。
#[derive(Debug, Deserialize)]
#[serde(tag = "action", content = "params", rename_all = "snake_case")]
pub enum AgentAction {
    /// 健康检查 / 存活探针
    #[serde(rename_all = "snake_case")]
    Ping {},
    /// 系统基本信息（主机名、CPU、内存、磁盘）
    #[serde(rename_all = "snake_case")]
    SystemInfo {},
    /// 服务状态查询（systemd unit）
    ServiceStatus { name: String },
    /// 服务启动（systemd）
    ServiceStart { name: String },
    /// 服务停止（systemd）
    ServiceStop { name: String },
    /// 服务重启（systemd）
    ServiceRestart { name: String },
    /// 文件是否存在
    FileExists { path: String },
    /// 路径是否目录
    PathIsDir { path: String },
    /// 仅允许白名单内的命令（禁止任意 Shell）
    WhitelistedCommand {
        command: String,
        timeout_secs: Option<u64>,
    },
}

/// 白名单命令前缀（精确匹配，防止 `&&`、`;`、管道拼接绕过）
const WHITELISTED_CMD_PREFIXES: &[&str] = &[
    "systemctl status ",
    "systemctl start ",
    "systemctl stop ",
    "systemctl restart ",
    "systemctl is-active ",
    "systemctl is-enabled ",
    "systemctl enable ",
    "systemctl disable ",
    "pgrep -x ",
    "nginx -t",
    "docker ps",
    "docker inspect ",
    "docker logs ",
    "ss -tlnp",
    "free -h",
    "df -h",
    "uptime",
    "uname -a",
    // 防火墙（Phase A1 扩展：防火墙规则随 execution_mode=agent 迁移）
    "which ",
    "ufw status",
    "ufw --force enable",
    "ufw disable",
    "ufw delete ",
    "ufw allow",
    "ufw deny",
    "ufw reject",
    "firewall-cmd --state",
    "firewall-cmd --reload",
    "firewall-cmd --permanent --add-rich-rule=",
    "firewall-cmd --permanent --remove-rich-rule=",
    "firewall-cmd --permanent --add-port=",
    "firewall-cmd --permanent --remove-port=",
    "iptables -L",
    "iptables -A ",
    "iptables -D ",
    "iptables -F",
    // 包管理（Phase A1 扩展：PackageManager/ServiceManager 随 execution_mode=agent 迁移）
    "apt install ",
    "apt-get remove ",
    "dpkg -l ",
    "yum install ",
    "dnf remove ",
    "rpm -q ",
    "apk add ",
    "apk del ",
    "apk info ",
    // Web 引擎管理（Phase A1 扩展：WebServerManager 引擎 reload/启停/config_test 迁移）
    "killall ",
    "nginx -s ",
    "nginx -t",
    "httpd -k ",
    "httpd -t",
    "lshttpd -c ",
    "/usr/local/lsws/bin/lswsctrl ",
    "openresty -s ",
    "openresty -t",
    "caddy reload",
    "caddy validate",
    // Docker compose（Phase A1 扩展：Bollard 降级路径迁移）
    "docker compose ",
    "docker-compose ",
    // Web 引擎原生检测（Phase A1 扩展：WebServerNativeManager 版本/端口扫描迁移）
    "nginx -v",
    "httpd -v",
    "openresty -v",
    "lshttpd -v",
    "caddy version",
    "ss -tln",
    "netstat -tln",
    // 原生数据库管理（Phase A1 扩展：MySqlManager/RedisManager 迁移到统一端口）
    "mysql -u root -e ",
    "mysqladmin ping ",
    "mysqladmin -u root ping ",
    "redis-cli ",
    "redis-server --version",
];

fn is_whitelisted(cmd: &str) -> bool {
    let cmd = cmd.trim();
    // 禁止危险 shell 元字符注入（&&, ;, |, $(), `, >, <, newline）
    let dangerous = [';', '|', '&', '$', '`', '>', '<', '\n', '\r', '\0'];
    if cmd.is_empty() {
        return false;
    }
    // 不允许空命令或只含空白
    if cmd.chars().all(char::is_whitespace) {
        return false;
    }
    // 不允许括号/子shell/重定向/变量扩展等
    if cmd.contains("$(") || cmd.contains("${") {
        return false;
    }
    for c in dangerous {
        if cmd.contains(c) {
            return false;
        }
    }
    WHITELISTED_CMD_PREFIXES.iter().any(|p| cmd.starts_with(p))
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum AgentActionResult {
    Ok(serde_json::Value),
    Err { code: String, message: String },
}

impl AgentActionResult {
    fn ok(v: serde_json::Value) -> Self {
        Self::Ok(v)
    }
    fn err(code: &str, msg: impl Into<String>) -> Self {
        Self::Err {
            code: code.into(),
            message: msg.into(),
        }
    }
}

// ─── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AgentConfig {
    panel_url: String,
    node_name: String,
    node_host: String,
    agent_port: u16,
    auth_token: String,
    /// A3.2：面板节点注册引导令牌（`X-Bootstrap-Token` 头，注册请求携带）
    bootstrap_token: String,
    /// 是否开启原始 `/exec` 任意命令端点（默认关闭，需显式 `ALLOW_EXEC=1`）。
    allow_exec: bool,
    /// 文件读写白名单根目录（默认当前目录；未配置则拒绝文件读写）。
    file_root: String,
}

impl AgentConfig {
    fn from_env() -> Self {
        Self {
            panel_url: std::env::var("PANEL_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            node_name: std::env::var("NODE_NAME").unwrap_or_else(|_| hostname()),
            node_host: std::env::var("NODE_HOST").unwrap_or_else(|_| local_ip()),
            agent_port: std::env::var("AGENT_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9527),
            auth_token: std::env::var("AUTH_TOKEN")
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
            bootstrap_token: std::env::var("BOOTSTRAP_TOKEN").unwrap_or_default(),
            allow_exec: std::env::var("ALLOW_EXEC")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            file_root: std::env::var("FILE_ROOT").unwrap_or_else(|_| ".".into()),
        }
    }
}

fn hostname() -> String {
    System::host_name().unwrap_or_else(|| "unknown".into())
}

fn local_ip() -> String {
    use std::net::UdpSocket;
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        if let Ok(()) = s.connect("8.8.8.8:80") {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".into()
}

// ─── Metrics ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeartbeatPayload {
    cpu_usage: f32,
    memory_usage_percent: f32,
    disk_usage_percent: f32,
    load_one: f32,
}

fn collect_metrics(sys: &mut System) -> HeartbeatPayload {
    sys.refresh_all();

    let cpu = sys.global_cpu_usage().clamp(0.0, 100.0);
    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();
    let mem_pct = if mem_total > 0 {
        (mem_used as f32 / mem_total as f32) * 100.0
    } else {
        0.0
    };

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let (disk_total, disk_used) = disks.iter().fold((0u64, 0u64), |(t, u), d| {
        (
            t + d.total_space(),
            u + d.total_space() - d.available_space(),
        )
    });
    let disk_pct = if disk_total > 0 {
        ((disk_used as f64 / disk_total as f64) * 100.0) as f32
    } else {
        0.0
    };

    let load = System::load_average();

    HeartbeatPayload {
        cpu_usage: cpu,
        memory_usage_percent: mem_pct,
        disk_usage_percent: disk_pct,
        load_one: load.one as f32,
    }
}

// ─── Register ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RegisterPayload {
    name: String,
    host: String,
    agent_port: u16,
    auth_token: String,
}

async fn register(config: &AgentConfig) -> Option<i64> {
    let payload = RegisterPayload {
        name: config.node_name.clone(),
        host: config.node_host.clone(),
        agent_port: config.agent_port,
        auth_token: config.auth_token.clone(),
    };

    let client = reqwest::Client::new();
    let mut builder = client
        .post(format!("{}/api/nodes/register", config.panel_url))
        .json(&payload)
        .timeout(Duration::from_secs(10));
    // A3.2：注册携带引导令牌（未配置时省略，由面板按 401 拒绝）
    if !config.bootstrap_token.is_empty() {
        builder = builder.header("X-Bootstrap-Token", config.bootstrap_token.clone());
    }
    match builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                #[derive(Deserialize)]
                struct R {
                    id: i64,
                }
                if let Ok(r) = resp.json::<R>().await {
                    eprintln!("[agent] 注册成功，节点 ID: {}", r.id);
                    return Some(r.id);
                }
            }
            eprintln!("[agent] 注册失败: HTTP {status}");
        }
        Err(e) => eprintln!("[agent] 注册请求失败: {e}"),
    }
    None
}

// ─── Heartbeat ─────────────────────────────────────────────────────────────────

async fn send_heartbeat(
    panel_url: &str,
    node_id: i64,
    metrics: &HeartbeatPayload,
    auth_token: &str,
) {
    let client = reqwest::Client::new();
    match client
        .post(format!("{panel_url}/api/nodes/heartbeat/{node_id}"))
        .bearer_auth(auth_token)
        .json(metrics)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                eprintln!("[agent] 心跳失败: HTTP {}", resp.status());
            }
        }
        Err(e) => eprintln!("[agent] 心跳请求失败: {e}"),
    }
}

// ─── Agent HTTP Server ─────────────────────────────────────────────────────────

// 启动时由 main 初始化一次；各 handler 经常量时间比较校验 token。
static AUTH_TOKEN_CELL: OnceLock<String> = OnceLock::new();
// 是否允许原始 /exec 任意命令端点（默认关闭）。
static ALLOW_EXEC_CELL: OnceLock<bool> = OnceLock::new();
// 文件读写白名单根目录。
static FILE_ROOT_CELL: OnceLock<PathBuf> = OnceLock::new();

/// 常量时间字符串比较，避免基于比较时长的时序侧信道。
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut diff: u8 = 0;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn check_auth(token: &str) -> bool {
    let expected = AUTH_TOKEN_CELL.get().map(|s| s.as_str()).unwrap_or("");
    !expected.is_empty() && constant_time_eq(token, expected)
}

/// 校验路径是否位于文件白名单根目录内，防止任意绝对路径读写（S3）。
///
/// A3.1：写目标（可能不存在）不能直接 canonicalize（失败回退原路径会绕过符号链接校验）——
/// 改为对**父目录** canonicalize（解析父链上的符号链接）后拼接文件名，再校验 `starts_with(root)`，
/// 对齐 kernel 侧 `file/mod.rs::sanitize_write_target` 的实现，杜绝 symlink 逃逸。
fn ensure_within_file_root(
    path: &PathBuf,
) -> Result<PathBuf, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let root = FILE_ROOT_CELL
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let root_abs = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
    let raw = if path.is_absolute() {
        path.clone()
    } else {
        root_abs.join(path)
    };
    let normalized = normalize_path(&raw);
    if !normalized.starts_with(&root_abs) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "path outside allowed file root"})),
        ));
    }
    // 已存在路径：canonicalize 解析符号链接后再校验（读/列目录路径）
    if normalized.exists() {
        let canonical = std::fs::canonicalize(&normalized).map_err(|_| {
            (
                axum::http::StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "path cannot be resolved"})),
            )
        })?;
        if !canonical.starts_with(&root_abs) {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "path escapes allowed file root via symlink"})),
            ));
        }
        return Ok(canonical);
    }
    // 未创建目标（写/上传）：父目录必须存在且位于根内，返回 父目录canonical + 文件名
    let name = normalized
        .file_name()
        .filter(|n| n.to_string_lossy() != "..")
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid path"})),
            )
        })?;
    let parent = normalized.parent().ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid path"})),
        )
    })?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "parent directory does not exist"})),
        )
    })?;
    if !canonical_parent.starts_with(&root_abs) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "path escapes allowed file root via symlink"})),
        ));
    }
    Ok(canonical_parent.join(name))
}

/// 词法规范化路径（解析 `.`/`..` 段，不触碰文件系统），防 `..` 词法穿越。
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn auth_error() -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "Unauthorized"})),
    )
}

#[derive(Debug, Deserialize)]
struct ExecRequest {
    command: String,
    timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct UploadQuery {
    path: String,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
    modified: String,
}

#[derive(Debug, Serialize)]
struct ExecResponse {
    output: String,
    exit_code: i32,
    duration_ms: u64,
}

async fn exec_endpoint(
    headers: HeaderMap,
    Json(body): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    // T2：/exec 默认关闭，需 agent 侧显式 ALLOW_EXEC=1 开启，且非空 token 才放行。
    if !ALLOW_EXEC_CELL.get().copied().unwrap_or(false) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(
                serde_json::json!({"error": "exec endpoint disabled (set ALLOW_EXEC=1 to enable)"}),
            ),
        ));
    }

    let timeout = body.timeout_secs.unwrap_or(30);
    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        Duration::from_secs(timeout),
        tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
            .arg(if cfg!(windows) { "/C" } else { "-c" })
            .arg(&body.command)
            .output(),
    )
    .await;

    let response = match result {
        Ok(Ok(output)) => ExecResponse {
            output: String::from_utf8_lossy(&output.stdout).to_string()
                + String::from_utf8_lossy(&output.stderr).as_ref(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: start.elapsed().as_millis() as u64,
        },
        Ok(Err(e)) => ExecResponse {
            output: format!("Command execution error: {e}"),
            exit_code: -1,
            duration_ms: start.elapsed().as_millis() as u64,
        },
        Err(_) => ExecResponse {
            output: "Command timed out".into(),
            exit_code: -1,
            duration_ms: start.elapsed().as_millis() as u64,
        },
    };

    Ok(Json(response))
}

async fn list_files_endpoint(
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<FileEntry>>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let base = q.path.as_deref().unwrap_or(".");
    let dir = match ensure_within_file_root(&PathBuf::from(base)) {
        Ok(d) => d,
        Err(e) => return Err(e),
    };

    let mut entries = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let metadata = entry.metadata().await;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = metadata
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let secs = t
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    secs.to_string()
                })
                .unwrap_or_default();
            entries.push(FileEntry {
                name,
                is_dir,
                size,
                modified,
            });
        }
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(Json(entries))
}

async fn download_file_endpoint(
    headers: HeaderMap,
    Query(q): Query<DownloadQuery>,
) -> Result<Vec<u8>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let path = match ensure_within_file_root(&PathBuf::from(&q.path)) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    if !path.is_file() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "File not found"})),
        ));
    }

    tokio::fs::read(&path).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Read error: {e}")})),
        )
    })
}

async fn upload_file_endpoint(
    headers: HeaderMap,
    Query(q): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let target = match ensure_within_file_root(&PathBuf::from(&q.path)) {
        Ok(t) => t,
        Err(e) => return Err(e),
    };
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // A3.1：写文件加 O_NOFOLLOW 直接写入句柄（避免二次打开 TOCTOU），
    // 拒绝写入解析为符号链接的目标（与父目录 canonicalize 校验互为纵深防御）
    #[cfg(unix)]
    {
        use tokio::io::AsyncWriteExt;
        let mut opts = tokio::fs::OpenOptions::new();
        opts.create(true)
            .write(true)
            .truncate(true)
            .custom_flags(libc::O_NOFOLLOW);
        let mut file = opts.open(&target).await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {e}")})),
            )
        })?;
        file.write_all(&body).await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {e}")})),
            )
        })?;
    }
    #[cfg(not(unix))]
    {
        tokio::fs::write(&target, &body).await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Write error: {e}")})),
            )
        })?;
    }

    Ok(Json(
        serde_json::json!({"message": "ok", "size": body.len()}),
    ))
}

// ─── Action 分发（Phase A1）────────────────────────────────────────────

async fn run_whitelisted_cmd(cmd: &str, timeout_secs: Option<u64>) -> (String, i32) {
    if !is_whitelisted(cmd) {
        return (format!("command not in whitelist: {cmd}"), -1);
    }
    let timeout = timeout_secs.unwrap_or(30);
    let result = tokio::time::timeout(
        Duration::from_secs(timeout),
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => (
            String::from_utf8_lossy(&output.stdout).to_string()
                + String::from_utf8_lossy(&output.stderr).as_ref(),
            output.status.code().unwrap_or(-1),
        ),
        Ok(Err(e)) => (format!("Command execution error: {e}"), -1),
        Err(_) => ("Command timed out".into(), -1),
    }
}

/// 动作分发端点：`POST /action`，body 为 `{"action":"...","params":{...}}`
async fn action_endpoint(
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<AgentActionResult>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    // 解析动作（失败返回 JSON 400）
    let action: AgentAction = match serde_json::from_slice(&body) {
        Ok(a) => a,
        Err(e) => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("invalid agent action: {e}"),
                    "expected": "{\"action\":\"ping\"|\"system_info\"|...}"
                })),
            ))
        }
    };

    let result = match action {
        AgentAction::Ping {} => {
            AgentActionResult::ok(serde_json::json!({ "pong": true, "ts": chrono_timestamp() }))
        }
        AgentAction::SystemInfo {} => {
            let mut sys = System::new_all();
            sys.refresh_all();
            let host = System::host_name().unwrap_or_default();
            let cpu = sys.global_cpu_usage().clamp(0.0, 100.0);
            let mem_total = sys.total_memory();
            let mem_used = sys.used_memory();
            let load = System::load_average();
            AgentActionResult::ok(serde_json::json!({
                "hostname": host,
                "cpu_usage": cpu,
                "memory_total_bytes": mem_total,
                "memory_used_bytes": mem_used,
                "load_one": load.one,
                "load_five": load.five,
                "load_fifteen": load.fifteen,
            }))
        }
        AgentAction::ServiceStatus { name } => {
            let (out, code) = run_whitelisted_cmd(&format!("systemctl status {name}"), None).await;
            AgentActionResult::ok(
                serde_json::json!({"service": name, "output": out, "exit_code": code}),
            )
        }
        AgentAction::ServiceStart { name } => {
            let (out, code) = run_whitelisted_cmd(&format!("systemctl start {name}"), None).await;
            AgentActionResult::ok(
                serde_json::json!({"service": name, "output": out, "exit_code": code}),
            )
        }
        AgentAction::ServiceStop { name } => {
            let (out, code) = run_whitelisted_cmd(&format!("systemctl stop {name}"), None).await;
            AgentActionResult::ok(
                serde_json::json!({"service": name, "output": out, "exit_code": code}),
            )
        }
        AgentAction::ServiceRestart { name } => {
            let (out, code) = run_whitelisted_cmd(&format!("systemctl restart {name}"), None).await;
            AgentActionResult::ok(
                serde_json::json!({"service": name, "output": out, "exit_code": code}),
            )
        }
        AgentAction::FileExists { path } => {
            let ok = std::path::Path::new(&path).exists();
            AgentActionResult::ok(serde_json::json!({"path": path, "exists": ok}))
        }
        AgentAction::PathIsDir { path } => {
            let ok = std::path::Path::new(&path).is_dir();
            AgentActionResult::ok(serde_json::json!({"path": path, "is_dir": ok}))
        }
        AgentAction::WhitelistedCommand {
            command,
            timeout_secs,
        } => {
            if !is_whitelisted(&command) {
                return Ok(Json(AgentActionResult::err(
                    "ACTION_NOT_ALLOWED",
                    format!("command not in whitelist: {command}"),
                )));
            }
            let (output, exit_code) = run_whitelisted_cmd(&command, timeout_secs).await;
            AgentActionResult::ok(serde_json::json!({"output": output, "exit_code": exit_code}))
        }
    };

    Ok(Json(result))
}

fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn agent_routes() -> Router {
    Router::new()
        .route("/exec", post(exec_endpoint))
        .route("/action", post(action_endpoint))
        .route("/files/list", get(list_files_endpoint))
        .route("/files/download", get(download_file_endpoint))
        .route("/files/upload", post(upload_file_endpoint))
}

// ─── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let config = AgentConfig::from_env();

    // T2：初始化供各 handler 使用的鉴权与权限配置。
    let _ = AUTH_TOKEN_CELL.set(config.auth_token.clone());
    let _ = ALLOW_EXEC_CELL.set(config.allow_exec);
    let _ = FILE_ROOT_CELL.set(PathBuf::from(&config.file_root));

    eprintln!("[agent] Flamepanel Agent 启动");
    eprintln!("[agent] 名称: {}", config.node_name);
    eprintln!("[agent] 主机: {}:{}", config.node_host, config.agent_port);
    eprintln!("[agent] 面板: {}", config.panel_url);
    if config.allow_exec {
        eprintln!("[agent] 警告: /exec 任意命令端点已开启 (ALLOW_EXEC=1)");
    }
    eprintln!("[agent] 文件白名单根目录: {}", config.file_root);
    if config.auth_token.len() < 16 {
        eprintln!("[agent] 警告: AUTH_TOKEN 过短，建议设置强随机值");
    }

    // Register with the panel
    let node_id = loop {
        if let Some(id) = register(&config).await {
            break id;
        }
        eprintln!("[agent] 5 秒后重试...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    };

    let config_clone = config.clone();

    // Spawn heartbeat task
    let panel_url = config.panel_url.clone();
    let auth_token = config.auth_token.clone();
    tokio::spawn(async move {
        let mut sys = System::new_all();
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let metrics = collect_metrics(&mut sys);
            send_heartbeat(&panel_url, node_id, &metrics, &auth_token).await;
        }
    });

    // Start HTTP server for agent commands
    let addr = SocketAddr::from(([0, 0, 0, 0], config_clone.agent_port));
    eprintln!("[agent] HTTP 服务监听: {addr}");

    let app = agent_routes();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Agent HTTP 端口绑定失败");

    axum::serve(listener, app.into_make_service())
        .await
        .expect("Agent HTTP 服务运行异常");
}

// ── Phase A1 单元测试 ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitelist_accepts_allowed_commands() {
        assert!(is_whitelisted("systemctl status nginx"));
        assert!(is_whitelisted("systemctl start docker"));
        assert!(is_whitelisted("nginx -t"));
        assert!(is_whitelisted("docker ps"));
        assert!(is_whitelisted("ss -tlnp"));
        assert!(is_whitelisted("uptime"));
        assert!(is_whitelisted("free -h"));
        assert!(is_whitelisted("df -h"));
        // 防火墙命令（Phase A1 扩展）
        assert!(is_whitelisted("which ufw"));
        assert!(is_whitelisted("ufw status"));
        assert!(is_whitelisted("ufw --force enable"));
        assert!(is_whitelisted("ufw disable"));
        assert!(is_whitelisted("ufw allow 80/tcp"));
        assert!(is_whitelisted("ufw delete allow 80/tcp"));
        assert!(is_whitelisted("firewall-cmd --state"));
        assert!(is_whitelisted("firewall-cmd --reload"));
        assert!(is_whitelisted("firewall-cmd --permanent --add-port=80/tcp"));
        assert!(is_whitelisted(
            "firewall-cmd --permanent --remove-port=80/tcp"
        ));
        assert!(is_whitelisted("iptables -L -n --line-numbers"));
        assert!(is_whitelisted(
            "iptables -A INPUT -p tcp --dport 80 -j ACCEPT"
        ));
        assert!(is_whitelisted(
            "iptables -D INPUT -p tcp --dport 80 -j ACCEPT"
        ));
        assert!(is_whitelisted("iptables -F"));
        // 包管理 / 服务管理命令（Phase A1 扩展：PackageManager/ServiceManager 迁移）
        assert!(is_whitelisted("systemctl enable nginx"));
        assert!(is_whitelisted("systemctl disable nginx"));
        assert!(is_whitelisted("pgrep -x nginx"));
        assert!(is_whitelisted("apt install -y nginx"));
        assert!(is_whitelisted("apt-get remove -y nginx"));
        assert!(is_whitelisted("dpkg -l nginx"));
        assert!(is_whitelisted("yum install -y nginx"));
        assert!(is_whitelisted("dnf remove -y nginx"));
        assert!(is_whitelisted("rpm -q nginx"));
        assert!(is_whitelisted("apk add nginx"));
        assert!(is_whitelisted("apk del nginx"));
        assert!(is_whitelisted("apk info -e nginx"));
        // Web 引擎管理（Phase A1 扩展：WebServerManager 迁移）
        assert!(is_whitelisted("nginx -s reload"));
        assert!(is_whitelisted("httpd -k graceful"));
        assert!(is_whitelisted("httpd -t"));
        assert!(is_whitelisted("openresty -s reload"));
        assert!(is_whitelisted("caddy reload"));
        assert!(is_whitelisted("killall nginx"));
        assert!(is_whitelisted("/usr/local/lsws/bin/lswsctrl reload"));
        // Docker compose 降级路径（Phase A1 扩展：Bollard fallback 迁移）
        assert!(is_whitelisted("docker compose -p proj up -d"));
        assert!(is_whitelisted("docker compose ls --format json"));
        assert!(is_whitelisted("docker-compose -p proj down"));
        // Web 引擎原生检测（Phase A1 扩展：WebServerNativeManager 版本/端口扫描迁移）
        assert!(is_whitelisted("which nginx"));
        assert!(is_whitelisted("nginx -v"));
        assert!(is_whitelisted("httpd -v"));
        assert!(is_whitelisted("openresty -v"));
        assert!(is_whitelisted("lshttpd -v"));
        assert!(is_whitelisted("caddy version"));
        assert!(is_whitelisted("ss -tln"));
        assert!(is_whitelisted("netstat -tln"));
        // 原生数据库管理（Phase A1 扩展：MySqlManager/RedisManager 迁移）
        assert!(is_whitelisted("mysql -u root -e SELECT 1"));
        assert!(is_whitelisted("mysqladmin ping -u root"));
        assert!(is_whitelisted("redis-cli CONFIG GET maxmemory"));
        assert!(is_whitelisted("redis-server --version"));
    }

    #[test]
    fn test_whitelist_rejects_arbitrary_commands() {
        assert!(!is_whitelisted("rm -rf /"));
        assert!(!is_whitelisted("cat /etc/shadow"));
        assert!(!is_whitelisted("echo hello"));
        assert!(!is_whitelisted("curl http://evil.com"));
        assert!(!is_whitelisted("systemctl status nginx && rm -rf /"));
        assert!(!is_whitelisted("bash -c 'ls'"));
        assert!(!is_whitelisted("sh -c 'rm -rf /'"));
        assert!(!is_whitelisted("sh -c 'echo x > /etc/passwd'"));
        assert!(!is_whitelisted("sudo docker ps"));
        assert!(!is_whitelisted("systemctl start nginx; rm -rf /"));
        assert!(!is_whitelisted(""));
    }

    #[test]
    fn test_action_serialization() {
        // ping 无参数
        let v = serde_json::json!({"action": "ping", "params": {}});
        let action: AgentAction = serde_json::from_value(v).unwrap();
        assert!(matches!(action, AgentAction::Ping {}));

        // whitelisted_command 带参数
        let v = serde_json::json!({
            "action": "whitelisted_command",
            "params": {"command": "nginx -t", "timeout_secs": 10}
        });
        let action: AgentAction = serde_json::from_value(v).unwrap();
        match action {
            AgentAction::WhitelistedCommand {
                command,
                timeout_secs,
            } => {
                assert_eq!(command, "nginx -t");
                assert_eq!(timeout_secs, Some(10));
            }
            other => panic!("expected WhitelistedCommand, got {:?}", other),
        }
    }
}
