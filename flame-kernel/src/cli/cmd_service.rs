//! 服务类命令：version / status / info / start / stop / restart / doctor。

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::domain::kernel_log::LogFilter;
use crate::infrastructure::logging::reader;

use super::args::Parsed;
use super::context::{Ctx, NGINX_CONF};
use super::db::Db;
use super::http::Client;
use super::output::{CliError, ExitCode, Outcome};
use super::system::{self, human_bytes, human_duration};

pub async fn version(ctx: &Ctx) -> Result<Outcome, CliError> {
    let client = Client::new(ctx)?;
    let panel = client.health().await.ok();
    let panel_version = panel
        .as_ref()
        .and_then(|h| h.get("version"))
        .and_then(Value::as_str)
        .map(String::from);
    let human = match &panel_version {
        Some(v) => format!(
            "fpctl {}\npanel {v} (running at {})",
            crate::VERSION,
            client.base
        ),
        None => format!("fpctl {}\npanel not running", crate::VERSION),
    };
    let mut o = Outcome::new(
        json!({ "cli": crate::VERSION, "panel": panel_version, "running": panel.is_some() }),
        human,
    );
    if let Some(v) = &panel_version {
        if v != crate::VERSION {
            o = o.warn(format!(
                "fpctl {} differs from panel {v}; reinstall to keep them in sync",
                crate::VERSION
            ));
        }
    }
    Ok(o)
}

pub async fn status(ctx: &Ctx) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let service = system::service_state();
    let client = Client::new(ctx)?;
    let health = client.health().await;
    let running = health.is_ok();
    let logging = ctx.logging();
    let db_path = ctx.database_path();
    let db_bytes = db_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len());
    let data_bytes = system::dir_size(&ctx.data_dir());
    let log_bytes = if logging.file_enabled {
        Some(system::dir_size(&logging.dir))
    } else {
        None
    };
    let disk = system::disk_space(&ctx.home);

    let h = health.as_ref().ok().cloned().unwrap_or(Value::Null);
    let data = json!({
        "running": running,
        "home": ctx.home,
        "url": client.base,
        "listen": format!("{}:{}", ctx.listen_host(), ctx.port()),
        "service": service,
        "health": h,
        "version": h.get("version"),
        "cli_version": crate::VERSION,
        "database": { "path": db_path, "bytes": db_bytes },
        "data_dir_bytes": data_bytes,
        "logs": { "enabled": logging.file_enabled, "dir": logging.dir, "bytes": log_bytes },
        "disk": disk.map(|(a, t)| json!({ "available": a, "total": t })),
    });

    let mut human = String::new();
    if running {
        let ver = h.get("version").and_then(Value::as_str).unwrap_or("?");
        let up = h.get("uptime_secs").and_then(Value::as_u64).unwrap_or(0);
        let overall = h.get("status").and_then(Value::as_str).unwrap_or("?");
        human.push_str(&format!(
            "Flamepanel   running · v{ver} · up {} · {overall}\n",
            human_duration(up)
        ));
    } else {
        human.push_str("Flamepanel   NOT RUNNING\n");
    }
    human.push_str(&format!(
        "Service      {} ({}{})\n",
        service.state,
        service.manager,
        match service.enabled {
            Some(true) => ", enabled",
            Some(false) => ", disabled",
            None => "",
        }
    ));
    human.push_str(&format!("URL          {}\n", client.base));
    if let Some(checks) = h.get("checks").and_then(Value::as_object) {
        let parts: Vec<String> = checks
            .iter()
            .map(|(k, v)| {
                format!(
                    "{k} {}",
                    v.get("status").and_then(Value::as_str).unwrap_or("?")
                )
            })
            .collect();
        human.push_str(&format!("Health       {}\n", parts.join(" · ")));
    }
    human.push_str(&format!("Home         {}\n", ctx.home.display()));
    human.push_str(&format!(
        "Data         {}{}\n",
        human_bytes(data_bytes),
        db_bytes
            .map(|b| format!(" (database {})", human_bytes(b)))
            .unwrap_or_default()
    ));
    match log_bytes {
        Some(b) => human.push_str(&format!(
            "Logs         {} in {}\n",
            human_bytes(b),
            logging.dir.display()
        )),
        None => human.push_str("Logs         file logging disabled (OP_LOG_FILE)\n"),
    }
    if let Some((a, t)) = disk {
        human.push_str(&format!(
            "Disk         {} free of {}\n",
            human_bytes(a),
            human_bytes(t)
        ));
    }

    // 未运行时附上最近的 error 日志——多数情况下它就是死因（如启动配置错误）
    let mut data = data;
    if !running {
        let recent = recent_errors(ctx, 3);
        if !recent.is_empty() {
            human.push_str("Last errors  (fpctl logs --level error for more)\n");
            for r in recent.iter().rev() {
                human.push_str(&format!(
                    "  {} {}\n",
                    r["timestamp"]
                        .as_str()
                        .unwrap_or("")
                        .get(..19)
                        .unwrap_or("")
                        .replace('T', " "),
                    r["message"].as_str().unwrap_or("")
                ));
            }
        }
        data["recent_errors"] = json!(recent);
    }
    let mut o = Outcome::new(data, human);
    if health.is_err() {
        o = o
            .warn(format!("panel not reachable at {}", client.base))
            .exit_with(ExitCode::NotRunning);
    }
    Ok(o)
}

pub async fn info(ctx: &Ctx) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let mut warnings = Vec::new();
    let admins: Option<Vec<String>> = match Db::open(ctx).await {
        Ok(db) => {
            use crate::domain::repository::UserRepository;
            let r = db.users().list().await;
            db.close().await;
            match r {
                Ok(users) => Some(
                    users
                        .into_iter()
                        .filter(|u| u.role == "admin")
                        .map(|u| u.username)
                        .collect(),
                ),
                Err(e) => {
                    warnings.push(format!("cannot list users: {e}"));
                    None
                }
            }
        }
        Err(e) => {
            warnings.push(format!("cannot open database: {}", e.message));
            None
        }
    };
    let logging = ctx.logging();
    let nginx = std::path::Path::new(NGINX_CONF).is_file();
    let data = json!({
        "url": ctx.base_url(),
        "nginx_proxy": nginx,
        "home": ctx.home,
        "env_file": ctx.env_file,
        "database": ctx.database_path(),
        "log_dir": logging.dir,
        "admins": admins,
        "version": crate::VERSION,
    });
    let mut human = format!(
        "Backend URL  {}{}\nHome         {}\nEnv file     {}\nDatabase     {}\nLog dir      {}\nAdmins       {}\n",
        ctx.base_url(),
        if nginx { "  (public access via nginx on port 80/443)" } else { "" },
        ctx.home.display(),
        ctx.env_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".into()),
        ctx.database_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ctx.database_url()),
        logging.dir.display(),
        admins
            .as_ref()
            .map(|a| if a.is_empty() { "(none — run the setup wizard)".into() } else { a.join(", ") })
            .unwrap_or_else(|| "?".into()),
    );
    human.push_str("Password     not stored in clear text — use `fpctl reset-password` if lost\n");
    let mut o = Outcome::new(data, human);
    o.warnings = warnings;
    Ok(o)
}

/// 等待健康检查通过。
pub(super) async fn wait_healthy(
    ctx: &Ctx,
    base: Option<String>,
    timeout: Duration,
) -> Result<Value, CliError> {
    let mut client = Client::new(ctx)?;
    if let Some(b) = base {
        client.base = b;
    }
    let start = Instant::now();
    loop {
        if let Ok(h) = client.health().await {
            return Ok(h);
        }
        if start.elapsed() >= timeout {
            let recent = recent_errors(ctx, 5);
            return Err(CliError::general(format!(
                "service did not become healthy at {} within {}s",
                client.base,
                timeout.as_secs()
            ))
            .with_detail(json!({ "recent_errors": recent })));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// 最近 N 条 error 级内核日志（排障线索）。
pub fn recent_errors(ctx: &Ctx, n: usize) -> Vec<Value> {
    let logging = ctx.logging();
    if !logging.file_enabled {
        return Vec::new();
    }
    let filter = LogFilter::from_params(Some("error"), None, None).unwrap_or_default();
    let name = crate::infrastructure::logging::current_file_name();
    reader::read_page(&logging.dir, &name, &filter, n, None)
        .map(|p| {
            p.entries
                .into_iter()
                .map(|e| json!({ "timestamp": e.timestamp, "target": e.target, "message": e.message }))
                .collect()
        })
        .unwrap_or_default()
}

pub async fn service_verb(ctx: &Ctx, p: &Parsed, verb: &str) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let cmd = format!("systemctl {verb} {}", super::context::SERVICE_NAME);
    if p.dry_run() {
        return Ok(Outcome::new(
            json!({ "dry_run": true, "actions": [cmd] }),
            format!("would run: {cmd}"),
        ));
    }
    system::systemctl(verb)?;
    if verb == "stop" || p.has("no-wait") {
        return Ok(Outcome::new(
            json!({ "action": verb, "service": system::service_state() }),
            format!("{verb}: ok"),
        ));
    }
    let h = wait_healthy(ctx, None, Duration::from_secs(30)).await?;
    let ver = h.get("version").cloned().unwrap_or(Value::Null);
    Ok(Outcome::new(
        json!({ "action": verb, "healthy": true, "version": ver, "url": ctx.base_url() }),
        format!(
            "{verb}: ok — healthy at {} (v{})",
            ctx.base_url(),
            ver.as_str().unwrap_or("?")
        ),
    ))
}

pub async fn service_restart_at(ctx: &Ctx, base: String) -> Result<Value, CliError> {
    system::systemctl("restart")?;
    wait_healthy(ctx, Some(base), Duration::from_secs(30)).await
}

// ─── doctor ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Check {
    Ok,
    Warn,
    Fail,
    Skip,
}

struct Report(Vec<Value>);

impl Report {
    fn add(&mut self, name: &str, status: Check, detail: impl Into<String>) {
        self.0
            .push(json!({ "name": name, "status": status, "detail": detail.into() }));
    }
    fn count(&self, c: Check) -> usize {
        let key = serde_json::to_value(c).unwrap_or(Value::Null);
        self.0.iter().filter(|v| v["status"] == key).count()
    }
}

fn mode_of(p: &std::path::Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .ok()
        .map(|m| m.permissions().mode() & 0o777)
}

/// 特权助手探测：socket 在线 → ping（并比对版本）；不在线 → 按是否安装了 unit 给出提示。
async fn helper_check(socket: &std::path::Path, unit_installed: bool) -> (Check, String) {
    use crate::core::error::AppError;
    if !socket.exists() {
        return if unit_installed {
            (
                Check::Warn,
                format!(
                    "flamepanel-helper installed but {} missing — systemctl restart flamepanel-helper; \
                     journalctl -u flamepanel-helper -n 50",
                    socket.display()
                ),
            )
        } else {
            (
                Check::Skip,
                "not installed (root / Docker deployments write engine config directly; \
                 re-run install.sh to add it)"
                    .into(),
            )
        };
    }
    match crate::webserver::helper::HelperClient::new(socket)
        .ping()
        .await
    {
        Ok(v) => {
            let ver = v.get("version").and_then(Value::as_str).unwrap_or("?");
            if ver == env!("CARGO_PKG_VERSION") {
                (Check::Ok, format!("{} (v{ver})", socket.display()))
            } else {
                (
                    Check::Warn,
                    format!(
                        "helper runs v{ver}, fpctl is v{} — systemctl restart flamepanel-helper",
                        env!("CARGO_PKG_VERSION")
                    ),
                )
            }
        }
        Err(AppError::Forbidden(_)) => (
            Check::Skip,
            "socket present; run fpctl as root or the panel user to probe it".into(),
        ),
        Err(e) => (Check::Fail, e.to_string()),
    }
}

pub async fn doctor(ctx: &Ctx) -> Result<Outcome, CliError> {
    let mut r = Report(Vec::new());

    // 1. 安装目录
    if ctx.installed {
        r.add("install", Check::Ok, ctx.home.display().to_string());
    } else {
        r.add(
            "install",
            Check::Fail,
            format!(
                "no installation at {} (use --dir or FP_HOME)",
                ctx.home.display()
            ),
        );
    }

    // 2. env 文件
    match &ctx.env_file {
        Some(p) => match mode_of(p) {
            Some(m) if m & 0o077 != 0 => r.add(
                "env-file",
                Check::Warn,
                format!("{} mode {m:o} exposes secrets; chmod 600", p.display()),
            ),
            _ => r.add("env-file", Check::Ok, p.display().to_string()),
        },
        None => r.add(
            "env-file",
            Check::Warn,
            "no flamepanel.env; built-in defaults in use",
        ),
    }
    for w in &ctx.warnings {
        r.add("config", Check::Warn, w.clone());
    }

    // 3. JWT 密钥
    match ctx.jwt_secret() {
        Some(s) => match crate::utils::jwt::validate_secret(&s) {
            Ok(()) => r.add("jwt-secret", Check::Ok, "present"),
            Err(e) => r.add("jwt-secret", Check::Fail, e.to_string()),
        },
        None => r.add(
            "jwt-secret",
            Check::Warn,
            "not readable by this user (panel generates one on first start)",
        ),
    }
    let key_file = ctx.data_dir().join("jwt_secret.key");
    if let Some(m) = mode_of(&key_file) {
        if m & 0o077 != 0 {
            r.add(
                "jwt-key-file",
                Check::Warn,
                format!("{} mode {m:o}; chmod 600", key_file.display()),
            );
        }
    }

    // 4. 数据库
    match Db::open(ctx).await {
        Ok(db) => {
            let integrity: Result<String, _> = sqlx::query_scalar("PRAGMA quick_check")
                .fetch_one(&db.pool)
                .await;
            match integrity {
                Ok(s) if s == "ok" => r.add("database", Check::Ok, db.path.display().to_string()),
                Ok(s) => r.add("database", Check::Fail, format!("quick_check: {s}")),
                Err(e) => r.add("database", Check::Fail, format!("quick_check failed: {e}")),
            }
            use crate::domain::repository::UserRepository;
            match db.users().list().await {
                Ok(u) if u.iter().any(|u| u.role == "admin") => {
                    r.add("admin", Check::Ok, format!("{} user(s)", u.len()))
                }
                Ok(_) => r.add(
                    "admin",
                    Check::Warn,
                    "no admin yet — finish the setup wizard",
                ),
                Err(e) => r.add("admin", Check::Fail, e.to_string()),
            }
            db.close().await;
        }
        Err(e) if e.exit == ExitCode::NotFound && !ctx.installed => {
            r.add("database", Check::Skip, e.message)
        }
        Err(e) if e.exit == ExitCode::NotFound => r.add(
            "database",
            Check::Warn,
            format!("{} (created on first start)", e.message),
        ),
        Err(e) => r.add("database", Check::Fail, e.message),
    }

    // 5. 磁盘
    match system::disk_space(&ctx.home) {
        Some((a, _)) if a < 200 * 1024 * 1024 => {
            r.add("disk", Check::Fail, format!("only {} free", human_bytes(a)))
        }
        Some((a, _)) if a < 1024 * 1024 * 1024 => {
            r.add("disk", Check::Warn, format!("{} free", human_bytes(a)))
        }
        Some((a, _)) => r.add("disk", Check::Ok, format!("{} free", human_bytes(a))),
        None => r.add("disk", Check::Skip, "statvfs failed"),
    }

    // 6. 日志目录 + 崩溃报告
    let logging = ctx.logging();
    if !logging.file_enabled {
        r.add(
            "logs",
            Check::Warn,
            "file logging disabled (OP_LOG_FILE=false)",
        );
    } else {
        match reader::list_files(&logging.dir, None) {
            Ok(files) => {
                let crashes: Vec<&str> = files
                    .iter()
                    .filter(|f| f.name.starts_with("crash-"))
                    .map(|f| f.name.as_str())
                    .collect();
                if crashes.is_empty() {
                    r.add(
                        "logs",
                        Check::Ok,
                        format!("{} file(s) in {}", files.len(), logging.dir.display()),
                    );
                } else {
                    let mut sorted = crashes.clone();
                    sorted.sort();
                    r.add(
                        "logs",
                        Check::Warn,
                        format!(
                            "{} crash report(s), latest {} — inspect in Logs → Kernel",
                            crashes.len(),
                            sorted.last().copied().unwrap_or("")
                        ),
                    );
                }
            }
            Err(e) => r.add(
                "logs",
                Check::Warn,
                format!("{}: {e}", logging.dir.display()),
            ),
        }
    }

    // 6b. 本机特权助手（Doc/32 W3-b）：非 root 面板写 Web 引擎配置的唯一通道
    let (c, msg) = helper_check(
        &crate::webserver::helper::client::helper_socket_from_env(),
        std::path::Path::new("/etc/systemd/system/flamepanel-helper.service").exists(),
    )
    .await;
    r.add("priv-helper", c, msg);

    // 7. 服务 + 健康
    let service = system::service_state();
    let health = Client::new(ctx)?.health().await;
    match service.state.as_str() {
        "unknown" => r.add("service", Check::Skip, "not managed by systemd"),
        "not-installed" => r.add(
            "service",
            Check::Skip,
            "no flamepanel systemd unit (manual run)",
        ),
        "active" => r.add("service", Check::Ok, "active"),
        "failed" => r.add(
            "service",
            Check::Fail,
            "failed — journalctl -u flamepanel -n 50; fpctl logs --level error",
        ),
        s => r.add("service", Check::Warn, s.to_string()),
    }
    match &health {
        Ok(h) => {
            let st = h.get("status").and_then(Value::as_str).unwrap_or("?");
            let c = if st == "ok" || st == "healthy" {
                Check::Ok
            } else {
                Check::Warn
            };
            r.add("health", c, format!("{} → {st}", ctx.base_url()));
        }
        Err(e) if service.state == "active" => r.add("health", Check::Fail, e.message.clone()),
        Err(_) => {
            // 未运行：检查端口是否被其它进程占用
            let port = ctx.port();
            match std::net::TcpListener::bind(("0.0.0.0", port)) {
                Ok(_) => r.add(
                    "health",
                    Check::Warn,
                    format!("panel not running; port {port} is free"),
                ),
                Err(e) => r.add(
                    "health",
                    Check::Fail,
                    format!("panel not running but port {port} is taken ({e})"),
                ),
            }
        }
    }

    // 8. nginx 反代端口一致性
    if let Ok(conf) = std::fs::read_to_string(NGINX_CONF) {
        let want = format!("127.0.0.1:{}", ctx.port());
        let proxies: Vec<&str> = conf.lines().filter(|l| l.contains("proxy_pass")).collect();
        if proxies.is_empty() || proxies.iter().all(|l| l.contains(&want)) {
            r.add("nginx", Check::Ok, NGINX_CONF);
        } else {
            r.add(
                "nginx",
                Check::Fail,
                format!(
                    "proxy_pass does not target {want}; run `fpctl port set {}`",
                    ctx.port()
                ),
            );
        }
    }

    // 9. 时间同步（签名令牌与证书依赖正确时钟）
    if system::which("timedatectl") {
        let (ok, out, _) =
            system::run("timedatectl", &["show", "-p", "NTPSynchronized", "--value"]);
        if ok && out == "no" {
            r.add("clock", Check::Warn, "system clock is not NTP-synchronized");
        } else if ok {
            r.add("clock", Check::Ok, "NTP synchronized");
        }
    }

    let fails = r.count(Check::Fail);
    let warns = r.count(Check::Warn);
    let mut human = String::new();
    for c in &r.0 {
        let tag = match c["status"].as_str().unwrap_or("") {
            "ok" => "  ok ",
            "warn" => " WARN",
            "fail" => " FAIL",
            _ => " skip",
        };
        human.push_str(&format!(
            "{tag}  {:<13}{}\n",
            c["name"].as_str().unwrap_or(""),
            c["detail"].as_str().unwrap_or("")
        ));
    }
    human.push_str(&format!("\n{fails} failure(s), {warns} warning(s)\n"));
    let o = Outcome::new(
        json!({ "checks": r.0, "failures": fails, "warnings": warns }),
        human,
    );
    Ok(if fails > 0 {
        o.exit_with(ExitCode::General)
    } else {
        o
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn helper_check_reports_missing_socket() {
        let p = std::path::Path::new("/nonexistent/flamepanel-helper.sock");
        let (c, msg) = helper_check(p, true).await;
        assert!(matches!(c, Check::Warn), "{msg}");
        assert!(msg.contains("systemctl restart flamepanel-helper"));
        let (c, _) = helper_check(p, false).await;
        assert!(matches!(c, Check::Skip));
    }

    #[tokio::test]
    async fn helper_check_pings_live_helper() {
        use crate::webserver::helper::HelperServer;
        use crate::webserver::host::LocalEngineHost;
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("h.sock");
        let listener = tokio::net::UnixListener::bind(&sock).unwrap();
        // SAFETY: getuid 无副作用
        let me = unsafe { libc::getuid() };
        let server = std::sync::Arc::new(HelperServer::new(
            LocalEngineHost::new(dir.path()),
            std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner),
            vec![me],
        ));
        tokio::spawn(server.serve(listener));
        let (c, msg) = helper_check(&sock, true).await;
        assert!(matches!(c, Check::Ok), "{msg}");
    }
}
