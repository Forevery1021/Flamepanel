//! `fpctl` 端到端：在临时安装目录里真实启动内核二进制，再用真实的 fpctl 二进制驱动它。
//!
//! 覆盖 Doc/32 §3 的 AI 协同契约：`--json` 信封、稳定退出码、`--dry-run`、`--yes`、
//! 离线（面板停止时）与在线（现签令牌调 API）两条路径。
#![cfg(feature = "sqlite")]

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const SECRET: &str = "fpctl-e2e-secret-0123456789abcdef0123456789";
const ADMIN: &str = "admin";
const ADMIN_PW: &str = "Initial-Passw0rd!";

struct Kernel(Child);

impl Drop for Kernel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn env_pairs(home: &Path, port: u16) -> Vec<(String, String)> {
    vec![
        ("OP_PORT".into(), port.to_string()),
        ("OP_HOST".into(), "127.0.0.1".into()),
        ("OP_ADMIN_USERNAME".into(), ADMIN.into()),
        ("OP_ADMIN_PASSWORD".into(), ADMIN_PW.into()),
        ("OP_JWT_SECRET".into(), SECRET.into()),
        (
            "OP_DATABASE_URL".into(),
            format!("sqlite:{}/data/flamepanel.db?mode=rwc", home.display()),
        ),
        (
            "OP_FILE_ROOT".into(),
            home.join("workspace").display().to_string(),
        ),
        (
            "OP_TERMINAL_CWD".into(),
            home.join("workspace").display().to_string(),
        ),
    ]
}

/// 与 install.sh 相同的目录形态：`<home>/flamepanel.env` + data/ + workspace/。
fn make_home(port: u16) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    std::fs::create_dir_all(home.join("data")).unwrap();
    std::fs::create_dir_all(home.join("workspace")).unwrap();
    let env: String = env_pairs(&home, port)
        .iter()
        .map(|(k, v)| format!("{k}={v}\n"))
        .collect();
    std::fs::write(home.join("flamepanel.env"), env).unwrap();
    (tmp, home)
}

fn start_kernel(home: &Path, port: u16) -> Kernel {
    start_kernel_with(home, port, &[])
}

fn start_kernel_with(home: &Path, port: u16, overrides: &[(&str, &str)]) -> Kernel {
    let child = Command::new(env!("CARGO_BIN_EXE_flame-kernel"))
        .current_dir(home)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", home)
        .envs(env_pairs(home, port))
        .envs(overrides.iter().copied())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn kernel");
    Kernel(child)
}

fn fpctl_in(home: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fpctl"));
    cmd.current_dir(std::env::temp_dir())
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .arg("--dir")
        .arg(home)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match stdin {
        Some(s) => {
            use std::io::Write;
            cmd.stdin(Stdio::piped());
            let mut child = cmd.spawn().unwrap();
            child.stdin.take().unwrap().write_all(s.as_bytes()).unwrap();
            child.wait_with_output().unwrap()
        }
        None => cmd.stdin(Stdio::null()).output().unwrap(),
    }
}

/// 运行并解析 `--json` 信封；断言 stdout 恰好是一个 JSON 文档。
fn json(home: &Path, args: &[&str]) -> (i32, Value) {
    json_stdin(home, args, None)
}

fn json_stdin(home: &Path, args: &[&str], stdin: Option<&str>) -> (i32, Value) {
    let mut a = args.to_vec();
    a.push("--json");
    let out = fpctl_in(home, &a, stdin);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "fpctl {args:?} stdout is not one JSON document ({e}):\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(v["exit"], code, "envelope exit mirrors process exit: {v}");
    (code, v)
}

fn wait_healthy(home: &Path) {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let (code, v) = json(home, &["version"]);
        if code == 0 && v["data"]["running"] == true {
            return;
        }
        assert!(Instant::now() < deadline, "kernel did not become healthy");
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[test]
fn fpctl_end_to_end() {
    let port = free_port();
    let (_tmp, home) = make_home(port);

    // ── 无需安装上下文 ───────────────────────────────────────────────
    let (code, v) = json(&home, &["commands"]);
    assert_eq!(code, 0);
    assert!(v["data"]["commands"].as_array().unwrap().len() >= 15);
    assert_eq!(
        v["data"]["contract"]["exit_codes"][3]["name"],
        "NOT_RUNNING"
    );

    let (code, v) = json(&home, &["openapi"]);
    assert_eq!(code, 0);
    assert!(v["data"]["paths"]["/api/health"].is_object());

    // 参数错误 → 2，且仍输出 JSON 错误信封
    let (code, v) = json(&home, &["frobnicate"]);
    assert_eq!(code, 2);
    assert_eq!(v["error"]["code"], "USAGE");

    // ── 面板未运行 ───────────────────────────────────────────────────
    let (code, v) = json(&home, &["status"]);
    assert_eq!(code, 3, "{v}");
    assert_eq!(v["data"]["running"], false);
    let (code, v) = json(&home, &["api", "GET", "/api/health"]);
    // 数据库尚不存在：离线身份解析先失败 → 5
    assert_eq!(code, 5, "{v}");

    // 不存在的安装目录 → 5
    let (code, _) = json(&home.join("nope"), &["info"]);
    assert_eq!(code, 5);

    // ── 启动即失败：致命错误必须落进日志文件，`status` 直接给出死因 ───
    {
        let mut bad = start_kernel_with(&home, port, &[("OP_TERMINAL_CWD", "/definitely/missing")]);
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if let Some(st) = bad.0.try_wait().unwrap() {
                assert!(
                    !st.success(),
                    "kernel must exit non-zero on fatal config error"
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "misconfigured kernel did not exit"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
        let (code, v) = json(&home, &["status"]);
        assert_eq!(code, 3, "{v}");
        assert!(
            v["data"]["recent_errors"]
                .to_string()
                .contains("OP_TERMINAL_CWD"),
            "fatal startup error missing from kernel log: {v}"
        );
    }

    // ── 启动内核 ─────────────────────────────────────────────────────
    let _kernel = start_kernel(&home, port);
    wait_healthy(&home);

    let (code, v) = json(&home, &["status"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["data"]["url"], format!("http://127.0.0.1:{port}"));

    let (code, v) = json(&home, &["version"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["cli"], v["data"]["panel"]);

    let (code, v) = json(&home, &["user", "list"]);
    assert_eq!(code, 0, "{v}");
    let users = v["data"]["users"].as_array().unwrap();
    assert_eq!(users[0]["username"], ADMIN);
    assert_eq!(users[0]["must_change_password"], true);

    let (code, v) = json(&home, &["info"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["admins"][0], ADMIN);
    assert!(
        !v.to_string().contains(ADMIN_PW),
        "info must never print passwords"
    );

    // 种子管理员须先改密：API 403 PASSWORD_CHANGE_REQUIRED → 退出码 4
    let (code, v) = json(&home, &["api", "GET", "/api/kernel-logs"]);
    assert_eq!(code, 4, "{v}");
    assert_eq!(
        v["error"]["detail"]["body"]["error"],
        "PASSWORD_CHANGE_REQUIRED"
    );

    // ── reset-password ───────────────────────────────────────────────
    // 危险操作缺 --yes → 2；--dry-run 不改库
    let (code, _) = json(&home, &["reset-password"]);
    assert_eq!(code, 2);
    let (code, v) = json(&home, &["reset-password", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["dry_run"], true);
    assert!(v["data"].get("password").is_none());

    let (code, v) = json(&home, &["reset-password", "--user", "ghost", "--yes"]);
    assert_eq!(code, 5, "{v}");

    // 弱密码 → 2
    let (code, _) = json_stdin(
        &home,
        &["reset-password", "--password-stdin", "--yes"],
        Some("short\n"),
    );
    assert_eq!(code, 2);

    let new_pw = "Rotated-Passw0rd!";
    let (code, v) = json_stdin(
        &home,
        &[
            "reset-password",
            "--password-stdin",
            "--no-force-change",
            "--yes",
        ],
        Some(&format!("{new_pw}\n")),
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["generated"], false);
    assert!(v["data"].get("password").is_none());

    // 新密码可登录、旧密码不可
    let login = |pw: &str| {
        let body = format!(r#"{{"username":"{ADMIN}","password":"{pw}"}}"#);
        json(&home, &["api", "POST", "/api/auth/login", "-d", &body])
    };
    let (code, v) = login(new_pw);
    assert_eq!(code, 0, "{v}");
    let (code, _) = login(ADMIN_PW);
    assert_eq!(code, 4);

    // ── 在线命令（现签 60 s 令牌）────────────────────────────────────
    // 运行中的面板鉴权缓存 15 s：强制改密标记的清除最多延迟 15 s 生效
    let deadline = Instant::now() + Duration::from_secs(30);
    let v = loop {
        let (code, v) = json(&home, &["api", "GET", "/api/kernel-logs"]);
        if code == 0 {
            break v;
        }
        assert_eq!(code, 4, "{v}");
        assert!(
            Instant::now() < deadline,
            "must-change flag not cleared: {v}"
        );
        std::thread::sleep(Duration::from_secs(1));
    };
    assert_eq!(v["data"]["enabled"], true);

    let (code, v) = json(&home, &["api", "GET", "/api/definitely-missing"]);
    assert_eq!(code, 5, "{v}");

    // 写请求 --dry-run 不发出
    let (code, v) = json(&home, &["api", "POST", "/api/backups", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["request"]["method"], "POST");

    let (code, v) = json(&home, &["log-level", "set", "debug"]);
    assert_eq!(code, 0, "{v}");
    let (code, v) = json(&home, &["log-level", "get"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["level"], "debug");
    assert_eq!(v["data"]["source"], "runtime");
    let (code, _) = json(&home, &["log-level", "set", "loud"]);
    assert_eq!(code, 2);

    let (code, v) = json(&home, &["backup", "list"]);
    assert_eq!(code, 0, "{v}");
    assert!(v["data"]["backups"].is_array());

    // ── 日志（直读文件）──────────────────────────────────────────────
    let (code, v) = json(&home, &["logs", "-n", "20"]);
    assert_eq!(code, 0, "{v}");
    assert!(v["data"]["count"].as_u64().unwrap() > 0, "{v}");
    let (code, v) = json(&home, &["logs", "files"]);
    assert_eq!(code, 0);
    assert!(v["data"]["files"][0]["name"]
        .as_str()
        .unwrap()
        .starts_with("flamepanel."));
    let (code, _) = json(&home, &["logs", "--level", "noisy"]);
    assert_eq!(code, 2);
    let (code, _) = json(&home, &["logs", "--file", "../etc/passwd"]);
    assert_eq!(code, 2);

    // ── doctor：临时目录下无 systemd / nginx 项，核心检查应通过 ───────
    let (code, v) = json(&home, &["doctor"]);
    let checks = v["data"]["checks"].as_array().unwrap();
    let status_of = |n: &str| {
        checks
            .iter()
            .find(|c| c["name"] == n)
            .map(|c| c["status"].as_str().unwrap().to_string())
    };
    assert_eq!(status_of("database").as_deref(), Some("ok"), "{v}");
    assert_eq!(status_of("jwt-secret").as_deref(), Some("ok"), "{v}");
    // CI 无 Docker 时整体健康为 degraded → warn，但不能是 fail
    assert!(
        matches!(status_of("health").as_deref(), Some("ok") | Some("warn")),
        "{v}"
    );
    assert_eq!(code == 0, v["data"]["failures"] == 0);

    // ── port set：非 root 预演可行，实际执行需 root ──────────────────
    let (code, v) = json(
        &home,
        &["port", "set", &free_port().to_string(), "--dry-run"],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["from"], port);
    // SAFETY: geteuid 无副作用
    if unsafe { libc::geteuid() } != 0 {
        let (code, _) = json(&home, &["port", "set", "9"]);
        assert_eq!(code, 4);
    }

    // 人类模式：结果在 stdout、无 JSON
    let out = fpctl_in(&home, &["user", "list"], None);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("ID") && text.contains(ADMIN), "{text}");
}
