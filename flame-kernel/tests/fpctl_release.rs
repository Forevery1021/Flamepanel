//! `fpctl` 版本管理端到端（Doc/32 §5）：在临时目录里模拟 install.sh 的旧布局，
//! 用真实的 fpctl 二进制走 adopt → upgrade → rollback → prune 全流程。
//!
//! 全程 `--no-restart` + `FP_BIN_DIR`（不需要 root / systemd）；「内核」是会回显版本号的 sh 脚本。
#![cfg(feature = "sqlite")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    bin: PathBuf,
    art: PathBuf,
}

fn fake_binary(version: &str) -> Vec<u8> {
    format!("#!/bin/sh\necho flamepanel {version}\n").into_bytes()
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "armv7",
        "loongarch64" => "loong64",
        other => other,
    }
}

fn tgz(path: &Path, files: &[(&str, Vec<u8>)], symlink: Option<(&str, &str)>) {
    let f = std::fs::File::create(path).unwrap();
    let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
    let mut b = tar::Builder::new(gz);
    for (name, data) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(data.len() as u64);
        h.set_mode(0o755);
        h.set_cksum();
        b.append_data(&mut h, name, data.as_slice()).unwrap();
    }
    if let Some((name, target)) = symlink {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Symlink);
        h.set_size(0);
        b.append_link(&mut h, name, target).unwrap();
    }
    b.into_inner().unwrap().finish().unwrap();
}

fn sha256(p: &Path) -> String {
    let d = ring::digest::digest(&ring::digest::SHA256, &std::fs::read(p).unwrap());
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// 生成与 release.yml 同名的一组发布包到 `dir`。
fn make_artifacts(dir: &Path, version: &str) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let be = dir.join(format!("flamepanel-linux-{}.tar.gz", arch()));
    let fe = dir.join("flamepanel-frontend.tar.gz");
    tgz(
        &be,
        &[
            ("flamepanel", fake_binary(version)),
            ("fpctl", b"#!/bin/sh\n".to_vec()),
        ],
        None,
    );
    tgz(
        &fe,
        &[
            ("index.html", format!("<html>{version}</html>").into_bytes()),
            ("assets/app.js", b"console.log(1)".to_vec()),
        ],
        None,
    );
    let sums = format!(
        "{}  {}\n{}  {}\n",
        sha256(&be),
        be.file_name().unwrap().to_string_lossy(),
        sha256(&fe),
        fe.file_name().unwrap().to_string_lossy()
    );
    std::fs::write(
        dir.join(format!("flamepanel-{version}-checksums.txt")),
        sums,
    )
    .unwrap();
}

/// install.sh 旧布局：`<bin>/flamepanel`（普通文件）+ `<home>/frontend/dist` + env + 空 SQLite 库。
fn legacy_install() -> Env {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("opt");
    let bin = tmp.path().join("bin");
    let art = tmp.path().join("artifacts");
    std::fs::create_dir_all(home.join("data")).unwrap();
    std::fs::create_dir_all(home.join("frontend/dist")).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(home.join("frontend/dist/index.html"), "<html>1.0.0</html>").unwrap();
    std::fs::write(home.join("data/flamepanel.db"), b"").unwrap();
    std::fs::write(
        home.join("flamepanel.env"),
        format!(
            "OP_PORT=1\nOP_DATABASE_URL=sqlite:{}/data/flamepanel.db?mode=rwc\n",
            home.display()
        ),
    )
    .unwrap();
    for (name, data) in [
        ("flamepanel", fake_binary("1.0.0")),
        ("fpctl", b"#!/bin/sh\n".to_vec()),
    ] {
        let p = bin.join(name);
        std::fs::write(&p, data).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    Env {
        _tmp: tmp,
        home,
        bin,
        art,
    }
}

fn run(env: &Env, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_fpctl"))
        .current_dir(std::env::temp_dir())
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("FP_BIN_DIR", &env.bin)
        .arg("--dir")
        .arg(&env.home)
        .args(args)
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "fpctl {args:?} stdout is not JSON ({e}):\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(v["exit"], code, "{v}");
    (code, v)
}

fn read(p: impl AsRef<Path>) -> String {
    std::fs::read_to_string(p).unwrap()
}

/// 安装目录里不应残留暂存 / 替换目录。
fn assert_clean(env: &Env) {
    for e in std::fs::read_dir(env.home.join("releases"))
        .unwrap()
        .flatten()
    {
        let n = e.file_name().to_string_lossy().into_owned();
        assert!(
            !n.contains(".staging-") && !n.contains(".replaced-"),
            "leftover {n}"
        );
    }
}

/// 升级流程依赖的版本探测契约：`flamepanel --version` 不读配置、立即输出并退出 0。
#[test]
fn kernel_binary_reports_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_flame-kernel"))
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .env_clear()
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        format!("flamepanel {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn release_lifecycle() {
    let env = legacy_install();
    let from_dir = env.art.to_string_lossy().into_owned();

    // ── 旧布局识别 ──────────────────────────────────────────────────
    let (code, v) = run(&env, &["releases"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["layout"], "legacy");

    // ── adopt：dry-run 不改动，正式执行后稳定路径变符号链接，且幂等 ──
    let (code, v) = run(&env, &["adopt", "--dry-run"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["version"], "1.0.0");
    assert!(!env.home.join("releases").exists());

    let (code, v) = run(&env, &["adopt"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["current"], "1.0.0");
    assert!(std::fs::symlink_metadata(env.bin.join("flamepanel"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        read(env.home.join("frontend/dist/index.html")),
        "<html>1.0.0</html>"
    );
    let (code, v) = run(&env, &["adopt"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["changed"], false);

    // ── upgrade 需要 --yes；dry-run 什么都不改 ─────────────────────
    make_artifacts(&env.art, "1.1.0");
    let (code, _) = run(&env, &["upgrade", "--from-dir", &from_dir, "--no-restart"]);
    assert_eq!(code, 2);
    let (code, v) = run(
        &env,
        &[
            "upgrade",
            "--from-dir",
            &from_dir,
            "--no-restart",
            "--dry-run",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["to"], "1.1.0");
    assert!(!env.home.join("releases/1.1.0").exists());
    assert!(!env.home.join("backups").exists());

    // ── 校验和不符 / 包内含符号链接：拒绝且不留残骸 ───────────────
    let be = env.art.join(format!("flamepanel-linux-{}.tar.gz", arch()));
    let good = std::fs::read(&be).unwrap();
    std::fs::write(&be, b"tampered").unwrap();
    let (code, v) = run(
        &env,
        &["upgrade", "--from-dir", &from_dir, "--no-restart", "--yes"],
    );
    assert_eq!(code, 1, "{v}");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("checksum"),
        "{v}"
    );
    std::fs::write(&be, good).unwrap();

    let evil = tempfile::tempdir().unwrap();
    make_artifacts(evil.path(), "6.6.6");
    let evil_be = evil
        .path()
        .join(format!("flamepanel-linux-{}.tar.gz", arch()));
    tgz(
        &evil_be,
        &[("flamepanel", fake_binary("6.6.6"))],
        Some(("fpctl", "/etc/passwd")),
    );
    std::fs::remove_file(evil.path().join("flamepanel-6.6.6-checksums.txt")).unwrap();
    let (code, v) = run(
        &env,
        &[
            "upgrade",
            "--from-dir",
            &evil.path().to_string_lossy(),
            "--no-restart",
            "--yes",
        ],
    );
    assert_eq!(code, 1, "{v}");
    assert!(!env.home.join("releases/6.6.6").exists());
    assert_clean(&env);
    assert_eq!(
        run(&env, &["releases"]).1["data"]["current"],
        "1.0.0",
        "failed upgrades keep current"
    );

    // ── 正式升级：备份库、切换、稳定路径指向新版本 ────────────────
    let (code, v) = run(
        &env,
        &["upgrade", "--from-dir", &from_dir, "--no-restart", "--yes"],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["from"], "1.0.0");
    assert_eq!(v["data"]["to"], "1.1.0");
    let backup = v["data"]["backup"].as_str().unwrap().to_string();
    assert!(Path::new(&backup).is_file(), "backup {backup}");
    assert!(backup.contains("pre-upgrade-1.0.0-to-1.1.0-"));
    assert_eq!(
        read(env.home.join("frontend/dist/index.html")),
        "<html>1.1.0</html>"
    );
    let out = Command::new(env.bin.join("flamepanel"))
        .arg("--version")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "flamepanel 1.1.0"
    );
    assert_clean(&env);

    // ── 同版本：无操作 ────────────────────────────────────────────
    let (code, v) = run(
        &env,
        &["upgrade", "--from-dir", &from_dir, "--no-restart", "--yes"],
    );
    assert_eq!(code, 0);
    assert_eq!(v["data"]["changed"], false);

    // ── rollback：回到 1.0.0，再切回 1.1.0 ────────────────────────
    let (code, v) = run(&env, &["rollback", "--no-restart", "--dry-run"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["to"], "1.0.0");
    let (code, v) = run(&env, &["rollback", "--no-restart", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        read(env.home.join("frontend/dist/index.html")),
        "<html>1.0.0</html>"
    );
    let (code, _) = run(
        &env,
        &["rollback", "--to", "9.9.9", "--no-restart", "--yes"],
    );
    assert_eq!(code, 5);
    let (code, _) = run(&env, &["rollback", "--no-restart", "--yes"]);
    assert_eq!(code, 0);
    assert_eq!(run(&env, &["releases"]).1["data"]["current"], "1.1.0");

    // ── prune：--keep 2 删除最旧版本，保留当前与上一个 ─────────────
    for ver in ["1.2.0", "1.3.0"] {
        make_artifacts(&env.art, ver);
        let (code, v) = run(
            &env,
            &[
                "upgrade",
                "--from-dir",
                &from_dir,
                "--no-restart",
                "--yes",
                "--keep",
                "2",
            ],
        );
        assert_eq!(code, 0, "{v}");
    }
    let (_, v) = run(&env, &["releases"]);
    let versions: Vec<&str> = v["data"]["releases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, vec!["1.3.0", "1.2.0"]);
    assert_eq!(v["data"]["current"], "1.3.0");
    assert_eq!(v["data"]["previous"], "1.2.0");
    assert_clean(&env);

    // ── 参数校验 ──────────────────────────────────────────────────
    let (code, _) = run(
        &env,
        &[
            "upgrade",
            "--from-dir",
            &from_dir,
            "--no-restart",
            "--yes",
            "--keep",
            "0",
        ],
    );
    assert_eq!(code, 2);
    let (code, _) = run(&env, &["adopt", "--version", "../x"]);
    assert_eq!(code, 2);
}
