//! 动作执行器：危险动作枚举化（Phase A1）、命令白名单、结构化 exec 策略（Doc/33 P-1）与 `/api/action` 分发。

use super::*;
use crate::install_policy::{check_structured, INSTALL_PREFIXES, INSTALL_STDIN_ARGV};

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
    /// 结构化命令（Doc/33 P-1）：`program` + `args` 直接 exec、**不经 shell**；
    /// env 仅限凭据类白名单键，stdin 仅限固定 argv（SQL / 仓库文件内容经 stdin 传入）。
    Exec {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
        #[serde(default)]
        stdin: Option<String>,
        timeout_secs: Option<u64>,
    },
}

/// 白名单命令前缀（精确匹配，防止 `&&`、`;`、管道拼接绕过）
pub(crate) const WHITELISTED_CMD_PREFIXES: &[&str] = &[
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
    // Apache Debian 系（包名 apache2，控制工具 apache2ctl + a2enmod/a2dismod）
    "apache2ctl configtest",
    "apache2ctl graceful",
    "a2enmod ",
    "a2dismod ",
    "/usr/sbin/apache2 -v",
    "/usr/sbin/httpd -v",
    "lshttpd -c ",
    "lshttpd -v",
    "/usr/local/lsws/bin/lshttpd -v",
    "/usr/local/lsws/bin/lshttpd -c ",
    "/usr/local/lsws/bin/lswsctrl ",
    "/usr/local/lsws/bin/lswsctrl status",
    "killall lshttpd",
    // OpenLiteSpeed 原生安装（官方仓库源 + 包管理）
    // Doc/33 P-5：官方二进制 keyring（HTTPS）；源列表 / .repo 文件内容经结构化 exec 的 `tee` + stdin 写入
    "wget -qO /usr/share/keyrings/litespeed.gpg https://rpms.litespeedtech.com/debian/lst_repo.gpg",
    "apt-get update",
    "apt-get install ",
    "openresty -s ",
    "openresty -t",
    // Caddy 原生控制（生命周期 start/stop 走 caddy CLI，reload/configtest 走热加载）
    "caddy start --config ",
    "caddy stop --config ",
    "caddy reload --config ",
    "caddy validate --config ",
    "caddy fmt --overwrite ",
    "caddy fmt ",
    "killall caddy",
    // Caddy 官方仓库安装（Cloudsmith stable 源；无管道/无 &&，逐条白名单前缀匹配）
    "wget -qO /tmp/caddy-stable.gpg https://dl.cloudsmith.io/public/caddy/stable/gpg.key",
    "wget -qO /etc/apt/sources.list.d/caddy-stable.list https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt",
    "wget -qO /etc/yum.repos.d/caddy-stable.repo https://dl.cloudsmith.io/public/caddy/stable/rpm.repo.txt",
    "gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg /tmp/caddy-stable.gpg",
    // Docker compose（Phase A1 扩展：Bollard 降级路径迁移）
    "docker compose ",
    "docker-compose ",
    // Web 引擎原生检测（Phase A1 扩展：WebServerNativeManager 版本/端口扫描迁移）
    "nginx -v",
    "httpd -v",
    "apache2 -v",
    "/usr/sbin/apache2 -v",
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

pub(crate) fn is_whitelisted(cmd: &str) -> bool {
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
    // Apache 模块管理（a2enmod/a2dismod）参数是模块名，只允许字母数字下划线，
    // 防止 `a2enmod --evil` 之类以合法前缀夹带未知参数。
    for prefix in ["a2enmod ", "a2dismod "] {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            return !rest.is_empty()
                && rest.split_whitespace().all(|m| {
                    !m.is_empty() && m.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                });
        }
    }
    // Caddy fmt：唯一合法参数是配置路径（可带 --overwrite），不允许夹带其他
    // `--xxx` 选项，防止 `caddy fmt --overwrite /etc/caddy/Caddyfile --evil` 之类
    // 以合法前缀 + 未知参数绕过白名单。
    if let Some(rest) = cmd.strip_prefix("caddy fmt --overwrite ") {
        let mut parts = rest.split_whitespace();
        let Some(path) = parts.next() else {
            return false;
        };
        if !path.starts_with('/') {
            return false;
        }
        return parts.next().is_none();
    }
    if let Some(rest) = cmd.strip_prefix("caddy fmt ") {
        let mut parts = rest.split_whitespace();
        let Some(path) = parts.next() else {
            return false;
        };
        if !path.starts_with('/') {
            return false;
        }
        return parts.next().is_none();
    }
    WHITELISTED_CMD_PREFIXES.iter().any(|p| cmd.starts_with(p))
}

/// exec 允许注入的环境变量键（凭据 / 非交互开关）。`PATH` / `LD_PRELOAD` 之类可改变
/// 被执行程序身份的键一律拒绝。
pub(crate) const EXEC_ENV_KEYS: &[&str] = &[
    "MYSQL_PWD",
    "PGPASSWORD",
    "REDISCLI_AUTH",
    "DEBIAN_FRONTEND",
];

/// 允许携带 stdin 的**精确** argv：数据库客户端（SQL 经 stdin，密码不进 argv）与
/// LiteSpeed 仓库文件写入。精确匹配——`mysql -u root --init-command=…` 之类夹带不放行。
pub(crate) const EXEC_STDIN_ARGV: &[&[&str]] = &[
    &["mysql", "-u", "root"],
    &[
        "psql",
        "-X",
        "-U",
        "postgres",
        "-At",
        "-v",
        "ON_ERROR_STOP=1",
    ],
    &["tee", "/etc/apt/sources.list.d/litespeed.list"],
    &["tee", "/etc/yum.repos.d/litespeed.repo"],
];

/// stdin 上限（SQL / 仓库文件远小于此）。
pub(crate) const EXEC_MAX_STDIN: usize = 1024 * 1024;

/// 白名单前缀按 token 与 argv 对齐：前缀的完整 token 必须与 argv 逐元素相等；
/// 前缀不以空格结尾时，最后一个 token 只需是对应 argv 元素的前缀
/// （如 `firewall-cmd --permanent --add-rich-rule=` 匹配 `--add-rich-rule=rule family=…`）。
/// 含 shell 语法（引号 / 重定向）的条目只对旧 `whitelisted_command` 有意义，此处跳过。
pub(crate) fn argv_matches_prefix(argv: &[&str], prefix: &str) -> bool {
    if prefix.contains(['\'', '>', '<']) {
        return false;
    }
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    if tokens.is_empty() || argv.len() < tokens.len() {
        return false;
    }
    let open_ended = !prefix.ends_with(' ');
    let last = tokens.len() - 1;
    tokens.iter().enumerate().all(|(i, t)| {
        if i == last && open_ended {
            argv[i].starts_with(t)
        } else {
            argv[i] == *t
        }
    })
}

/// 结构化 exec 校验：返回 `Err(原因)` 即拒绝。
pub(crate) fn exec_allowed(
    program: &str,
    args: &[String],
    env: &std::collections::BTreeMap<String, String>,
    stdin: Option<&str>,
) -> Result<(), String> {
    if program.is_empty() || program.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("invalid program".into());
    }
    if args.iter().any(|a| a.contains('\0')) {
        return Err("NUL in argument".into());
    }
    for (k, v) in env {
        if !EXEC_ENV_KEYS.contains(&k.as_str()) {
            return Err(format!("env key not allowed: {k}"));
        }
        if v.contains('\0') {
            return Err(format!("NUL in env value: {k}"));
        }
    }
    let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 1);
    argv.push(program);
    argv.extend(args.iter().map(String::as_str));

    if let Some(input) = stdin {
        if input.len() > EXEC_MAX_STDIN {
            return Err("stdin too large".into());
        }
        return if EXEC_STDIN_ARGV.contains(&argv.as_slice())
            || INSTALL_STDIN_ARGV.contains(&argv.as_slice())
        {
            Ok(())
        } else {
            Err("stdin not allowed for this command".into())
        };
    }
    if EXEC_STDIN_ARGV.contains(&argv.as_slice()) {
        return Ok(());
    }
    // sh 常量脚本 / sed 端口改写：逐项结构化校验，结论即最终结论（绝不落入前缀匹配）
    if let Some(verdict) = check_structured(program, args) {
        return verdict;
    }
    // 参数形态有额外约束的程序（a2enmod 模块名 / caddy fmt 路径）：沿用字符串校验，
    // 且不允许参数内含空白（否则 join 后的校验与真实 argv 不一致）
    if matches!(program, "a2enmod" | "a2dismod" | "caddy")
        && args.iter().any(|a| a.chars().any(char::is_whitespace))
    {
        return Err("whitespace in argument".into());
    }
    if matches!(program, "a2enmod" | "a2dismod")
        || (program == "caddy" && args.first().map(String::as_str) == Some("fmt"))
    {
        return if is_whitelisted(&argv.join(" ")) {
            Ok(())
        } else {
            Err("command not in whitelist".into())
        };
    }
    if WHITELISTED_CMD_PREFIXES
        .iter()
        .chain(INSTALL_PREFIXES)
        .any(|p| argv_matches_prefix(&argv, p))
    {
        Ok(())
    } else {
        Err("command not in whitelist".into())
    }
}

/// 结构化执行：直接 exec，stdout / stderr 分开返回。
pub(crate) async fn run_exec(
    program: &str,
    args: &[String],
    env: &std::collections::BTreeMap<String, String>,
    stdin: Option<&str>,
    timeout_secs: Option<u64>,
) -> Result<(String, String, i32), String> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .envs(env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let child_id = child.id();
    if let (Some(input), Some(mut pipe)) = (stdin.map(str::to_owned), child.stdin.take()) {
        // 独立任务写入后关闭（EOF），与读输出并发，避免双向管道互等
        tokio::spawn(async move {
            let _ = pipe.write_all(input.as_bytes()).await;
            let _ = pipe.shutdown().await;
        });
    }
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(30).min(3600));
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(o)) => Ok((
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
            o.status.code().unwrap_or(-1),
        )),
        Ok(Err(e)) => Err(format!("wait failed: {e}")),
        Err(_) => {
            if let Some(pid) = child_id {
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
            Err("Command timed out".into())
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum AgentActionResult {
    Ok(serde_json::Value),
    Err { code: String, message: String },
}

impl AgentActionResult {
    pub(crate) fn ok(v: serde_json::Value) -> Self {
        Self::Ok(v)
    }
    pub(crate) fn err(code: &str, msg: impl Into<String>) -> Self {
        Self::Err {
            code: code.into(),
            message: msg.into(),
        }
    }
}

pub(crate) async fn run_whitelisted_cmd(command: &str, timeout_secs: Option<u64>) -> (String, i32) {
    if !is_whitelisted(command) {
        return (format!("command not in whitelist: {command}"), -1);
    }
    let timeout = timeout_secs.unwrap_or(30);
    // M10：超时后杀子进程，避免孤儿命令累积。
    let mut child_cmd = tokio::process::Command::new("sh");
    child_cmd.arg("-c").arg(command).kill_on_drop(true);
    // 必须显式接管 stdout/stderr 管道，否则 wait_with_output 捕获不到输出
    child_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = match child_cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (format!("Command execution error: {e}"), -1),
    };
    let child_id = child.id();
    let result = tokio::time::timeout(Duration::from_secs(timeout), child.wait_with_output()).await;

    match result {
        Ok(Ok(output)) => (
            String::from_utf8_lossy(&output.stdout).to_string()
                + String::from_utf8_lossy(&output.stderr).as_ref(),
            output.status.code().unwrap_or(-1),
        ),
        Ok(Err(e)) => (format!("Command execution error: {e}"), -1),
        Err(_) => {
            // child 已被 wait_with_output 消费；kill_on_drop(true) 会在 future drop 时杀进程，
            // 这里用 pid 再补一次 SIGKILL，确保及时终止并回收。
            if let Some(pid) = child_id {
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
            ("Command timed out".into(), -1)
        }
    }
}

/// 动作分发端点：`POST /action`，body 为 `{"action":"...","params":{...}}`
pub(crate) async fn action_endpoint(
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
            // S1：文件存在性探测也须限定在 file_root 内，避免信息泄露任意路径存在性。
            let p = match ensure_within_file_root(&PathBuf::from(&path)) {
                Ok(p) => p,
                Err(_) => {
                    return Ok(Json(AgentActionResult::err(
                        "PATH_OUTSIDE_ROOT",
                        "path outside allowed file root",
                    )))
                }
            };
            let ok = p.exists();
            AgentActionResult::ok(serde_json::json!({"path": path, "exists": ok}))
        }
        AgentAction::PathIsDir { path } => {
            let p = match ensure_within_file_root(&PathBuf::from(&path)) {
                Ok(p) => p,
                Err(_) => {
                    return Ok(Json(AgentActionResult::err(
                        "PATH_OUTSIDE_ROOT",
                        "path outside allowed file root",
                    )))
                }
            };
            let ok = p.is_dir();
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
        AgentAction::Exec {
            program,
            args,
            env,
            stdin,
            timeout_secs,
        } => {
            if let Err(why) = exec_allowed(&program, &args, &env, stdin.as_deref()) {
                // 只回显程序名与原因：参数 / stdin 可能含凭据
                return Ok(Json(AgentActionResult::err(
                    "ACTION_NOT_ALLOWED",
                    format!("exec rejected ({program}): {why}"),
                )));
            }
            match run_exec(&program, &args, &env, stdin.as_deref(), timeout_secs).await {
                Ok((stdout, stderr, exit_code)) => AgentActionResult::ok(serde_json::json!({
                    "output": format!("{stdout}{stderr}"),
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                })),
                Err(e) => AgentActionResult::err("EXEC_FAILED", e),
            }
        }
    };

    Ok(Json(result))
}

pub(crate) fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
