//! 版本管理命令：releases / adopt / upgrade / rollback（Doc/32 §5）。
//!
//! 升级流程：取包 → 校验 → 解到 `releases/.staging-*` → 探测版本 → （旧布局先收编）→
//! 数据库 `VACUUM INTO` 备份 → 就位并原子切换 `current` → 重启 + 健康检查（校验版本号）→
//! 失败自动切回并重启 → 清理旧版本。

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use super::args::Parsed;
use super::artifacts::{self, Artifacts};
use super::cmd_service::wait_healthy;
use super::context::Ctx;
use super::db::Db;
use super::output::{CliError, Outcome};
use super::release::{
    move_path, parse_version_output, valid_version, version_key, Layout, LayoutState,
    DEFAULT_BIN_DIR,
};
use super::system::{self, human_bytes};

const DEFAULT_KEEP: usize = 3;
const HEALTH_TIMEOUT: Duration = Duration::from_secs(60);

fn bin_dir() -> PathBuf {
    std::env::var("FP_BIN_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BIN_DIR))
}

fn layout(ctx: &Ctx) -> Layout {
    Layout::new(&ctx.home, &bin_dir())
}

fn writable(p: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(p.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: c 是以 NUL 结尾的有效 C 字符串；access 不保留指针
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

/// 写操作前置：安装目录与二进制目录可写（替代「必须 root」，便于非 /usr/local 的自定义部署与测试）。
fn require_writable(l: &Layout) -> Result<(), CliError> {
    for d in [&l.home, &l.bin_dir] {
        if !writable(d) {
            return Err(CliError::permission(format!(
                "{} is not writable; re-run with sudo",
                d.display()
            )));
        }
    }
    Ok(())
}

/// 已安装（未版本化或版本化）二进制的版本：执行 `flamepanel --version`，失败回落到 fpctl 自身版本。
fn installed_version(l: &Layout) -> (String, bool) {
    let out = std::process::Command::new(l.bin_dir.join("flamepanel"))
        .arg("--version")
        .output();
    match out {
        Ok(o) if o.status.success() => {
            if let Some(v) = parse_version_output(&String::from_utf8_lossy(&o.stdout)) {
                return (v, true);
            }
        }
        _ => {}
    }
    (env!("CARGO_PKG_VERSION").to_string(), false)
}

fn parse_keep(p: &Parsed) -> Result<usize, CliError> {
    match p.value("keep") {
        None => Ok(DEFAULT_KEEP),
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n >= 1 => Ok(n),
            _ => Err(CliError::usage(format!(
                "--keep must be an integer >= 1, got '{s}'"
            ))),
        },
    }
}

fn version_flag(p: &Parsed, name: &str) -> Result<Option<String>, CliError> {
    match p.value(name) {
        None => Ok(None),
        Some(v) => {
            let v = v.trim_start_matches('v');
            if valid_version(v) {
                Ok(Some(v.to_string()))
            } else {
                Err(CliError::usage(format!("invalid version '{v}'")))
            }
        }
    }
}

// ─── releases ───────────────────────────────────────────────────────────────

pub async fn releases(ctx: &Ctx) -> Result<Outcome, CliError> {
    let l = layout(ctx);
    let state = l.state();
    let list = l.list();
    let mut human = format!(
        "layout: {}",
        serde_json::to_value(state)
            .unwrap_or_default()
            .as_str()
            .unwrap_or("")
    );
    match state {
        LayoutState::Legacy => human.push_str(
            " (binaries are plain files; `sudo fpctl adopt` converts to versioned releases)",
        ),
        LayoutState::Empty => human.push_str(" (no installed binaries found)"),
        LayoutState::Versioned => {}
    }
    human.push('\n');
    if !list.is_empty() {
        let rows: Vec<Vec<String>> = list
            .iter()
            .map(|r| {
                vec![
                    format!(
                        "{}{}",
                        r.version,
                        if r.current {
                            " (current)"
                        } else if r.previous {
                            " (previous)"
                        } else {
                            ""
                        }
                    ),
                    if r.has_frontend { "yes" } else { "no" }.into(),
                    human_bytes(r.bytes),
                ]
            })
            .collect();
        human.push_str(&system::table(&["VERSION", "FRONTEND", "SIZE"], &rows));
    }
    Ok(Outcome::new(
        json!({
            "layout": state,
            "home": ctx.home,
            "bin_dir": l.bin_dir,
            "current": l.current_version(),
            "previous": l.previous_version(),
            "releases": list,
        }),
        human,
    ))
}

// ─── adopt ──────────────────────────────────────────────────────────────────

pub async fn adopt(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let l = layout(ctx);
    let state = l.state();
    let version = match version_flag(p, "version")? {
        Some(v) => v,
        None => installed_version(&l).0,
    };
    if p.dry_run() {
        let action = match state {
            LayoutState::Legacy => {
                format!("move binaries and frontend into releases/{version}, link current")
            }
            LayoutState::Versioned => "verify stable links point at current".into(),
            LayoutState::Empty => "nothing to adopt".into(),
        };
        return Ok(Outcome::new(
            json!({ "dry_run": true, "layout": state, "version": version, "actions": [action] }),
            format!("would {action}"),
        ));
    }
    if state == LayoutState::Empty {
        return Err(CliError::not_found(format!(
            "nothing to adopt: {} not found",
            l.bin_dir.join("flamepanel").display()
        )));
    }
    require_writable(&l)?;
    let actions = l.adopt(&version)?;
    let current = l.current_version();
    let human = if actions.is_empty() {
        format!(
            "already versioned (current: {})",
            current.as_deref().unwrap_or("?")
        )
    } else {
        format!("adopted as release {version}\n  {}", actions.join("\n  "))
    };
    Ok(Outcome::new(
        json!({ "layout": l.state(), "was": state, "current": current, "changed": !actions.is_empty(), "actions": actions }),
        human,
    ))
}

// ─── upgrade ────────────────────────────────────────────────────────────────

async fn fetch(p: &Parsed, want: Option<&str>) -> Result<Artifacts, CliError> {
    if let Some(dir) = p.value("from-dir") {
        return artifacts::from_dir(Path::new(dir));
    }
    let base = p
        .value("source")
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| artifacts::GH_BASE.to_string());
    let version = match want {
        Some(v) => v.to_string(),
        None => artifacts::resolve_latest(&base).await?,
    };
    artifacts::download_release(&base, &version).await
}

/// `VACUUM INTO` 一致性快照（面板运行中也安全）。非 SQLite 部署返回 `None`。
async fn backup_database(ctx: &Ctx, from: &str, to: &str) -> Result<Option<PathBuf>, CliError> {
    let Some(path) = ctx.database_path() else {
        return Ok(None);
    };
    if !path.is_file() {
        return Ok(None);
    }
    let dir = ctx.home.join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| CliError::io(dir.display(), &e))?;
    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let out = dir.join(format!("pre-upgrade-{from}-to-{to}-{ts}.db"));
    let db = Db::open(ctx).await?;
    let res = sqlx::query("VACUUM INTO ?")
        .bind(out.to_string_lossy().into_owned())
        .execute(&db.pool)
        .await;
    db.close().await;
    res.map_err(|e| CliError::general(format!("database backup failed: {e}")))?;
    super::db::fix_sidecar_ownership(&out);
    Ok(Some(out))
}

/// 重启并确认新版本健康；返回健康检查报告的版本号。
async fn restart_and_verify(ctx: &Ctx, expect: &str) -> Result<String, CliError> {
    system::systemctl("restart")?;
    let h = wait_healthy(ctx, None, HEALTH_TIMEOUT).await?;
    let got = h
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if got != expect {
        return Err(CliError::general(format!(
            "service is healthy but reports version '{got}', expected '{expect}'"
        )));
    }
    Ok(got)
}

fn cleanup(p: &Path) {
    let _ = std::fs::remove_dir_all(p);
}

pub async fn upgrade(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let l = layout(ctx);
    let keep = parse_keep(p)?;
    let restart = !p.has("no-restart");
    if restart && !p.dry_run() && !system::has_systemd() {
        return Err(CliError::usage(
            "systemd is not available; re-run with --no-restart and restart the panel yourself",
        ));
    }
    if !p.dry_run() {
        require_writable(&l)?;
    }
    let want = version_flag(p, "version")?;
    let state = l.state();
    let from = match state {
        LayoutState::Versioned => l.current_version().unwrap_or_default(),
        _ => installed_version(&l).0,
    };

    // 1. 取包 + 校验 + 解到暂存目录（dry-run 放系统临时目录，不碰安装目录）
    let art = fetch(p, want.as_deref()).await?;
    let mut warnings = Vec::new();
    if art.checksums.is_none() {
        warnings
            .push("no checksums file next to the artifacts; integrity not verified".to_string());
    }
    let tmp_holder;
    let staging = if p.dry_run() {
        tmp_holder = tempfile::tempdir().map_err(|e| CliError::io("create temp dir", &e))?;
        tmp_holder.path().join("release")
    } else {
        std::fs::create_dir_all(l.releases_dir())
            .map_err(|e| CliError::io(l.releases_dir().display(), &e))?;
        l.releases_dir()
            .join(format!(".staging-{}", std::process::id()))
    };
    cleanup(&staging);
    let staged = artifacts::assemble_release(&art.backend, &art.frontend, &staging)
        .and_then(|_| artifacts::probe_version(&staging.join("bin/flamepanel")));
    let to = match staged {
        Ok(v) => v,
        Err(e) => {
            cleanup(&staging);
            return Err(e);
        }
    };
    let expected = want.clone().or(art.version_hint.clone());
    if let Some(w) = expected.as_deref().filter(|w| *w != to) {
        cleanup(&staging);
        return Err(CliError::general(format!(
            "artifact binary reports version {to}, expected {w}"
        )));
    }

    // 2. 同版本：除非 --force 否则无事可做
    if to == from && !p.has("force") {
        cleanup(&staging);
        return Ok(Outcome::new(
            json!({ "changed": false, "from": from, "to": to, "reason": "already on this version" }),
            format!("already on {to} (use --force to reinstall)"),
        ));
    }
    let downgrade = version_key(&to) < version_key(&from);
    if downgrade {
        warnings.push(format!(
            "{to} is older than {from}; database migrations are not reverted — prefer `fpctl rollback`"
        ));
    }

    if p.dry_run() {
        let mut actions = Vec::new();
        if state == LayoutState::Legacy {
            actions.push(format!("adopt legacy install as release {from}"));
        }
        actions.push(format!(
            "backup database to backups/pre-upgrade-{from}-to-{to}-<ts>.db"
        ));
        actions.push(format!("install release {to} and switch current"));
        if restart {
            actions.push(
                "systemctl restart flamepanel and wait for health (auto-rollback on failure)"
                    .into(),
            );
        }
        actions.push(format!("prune releases beyond the newest {keep}"));
        let mut o = Outcome::new(
            json!({ "dry_run": true, "from": from, "to": to, "actions": actions }),
            format!("would upgrade {from} -> {to}:\n  {}", actions.join("\n  ")),
        );
        o.warnings = warnings;
        return Ok(o);
    }

    // 3. 旧布局先收编，使回滚目标存在
    if state == LayoutState::Legacy {
        if let Err(e) = l.adopt(&from) {
            cleanup(&staging);
            return Err(e);
        }
    }

    // 4. 数据库备份（失败即中止，不切版本）
    let backup = match backup_database(ctx, &from, &to).await {
        Ok(b) => b,
        Err(e) => {
            cleanup(&staging);
            return Err(e);
        }
    };
    if backup.is_none() {
        warnings.push("database is not a local SQLite file; no pre-upgrade backup taken".into());
    }

    // 5. 就位 + 原子切换
    let dest = l.release_dir(&to);
    let aside = l
        .releases_dir()
        .join(format!(".{to}.replaced-{}", std::process::id()));
    if dest.exists() {
        move_path(&dest, &aside)?;
    }
    if let Err(e) = move_path(&staging, &dest) {
        if aside.exists() {
            let _ = move_path(&aside, &dest);
        }
        cleanup(&staging);
        return Err(e);
    }
    cleanup(&aside);
    let before = l.switch(&to)?;
    let rollback_to = before.filter(|b| *b != to && l.release_dir(b).is_dir());

    // 6. 重启 + 健康检查；失败自动回滚
    let mut restarted = false;
    if restart {
        if let Err(e) = restart_and_verify(ctx, &to).await {
            let Some(prev) = rollback_to else {
                return Err(e);
            };
            let rb = match l.switch(&prev) {
                Ok(_) => match restart_and_verify(ctx, &prev).await {
                    Ok(_) => format!("rolled back to {prev}; the service is healthy"),
                    Err(e2) => format!(
                        "rolled back to {prev} but it is not healthy: {}",
                        e2.message
                    ),
                },
                Err(e2) => format!("rollback to {prev} failed: {}", e2.message),
            };
            let detail = json!({
                "from": from, "to": to, "rolled_back_to": prev,
                "rollback": rb, "backup": backup, "cause": e.detail,
            });
            return Err(
                CliError::general(format!("upgrade to {to} failed: {} — {rb}", e.message))
                    .with_detail(detail),
            );
        }
        restarted = true;
    }

    // 7. 清理
    let pruned = l.prune(keep);
    let mut human = format!("upgraded {from} -> {to}");
    if let Some(b) = &backup {
        human.push_str(&format!("\n  database backup: {}", b.display()));
    }
    if !restarted {
        human.push_str("\n  not restarted (--no-restart): run `sudo fpctl restart` to activate");
    }
    if !pruned.is_empty() {
        human.push_str(&format!("\n  pruned: {}", pruned.join(", ")));
    }
    let mut o = Outcome::new(
        json!({
            "changed": true, "from": from, "to": to, "restarted": restarted,
            "backup": backup, "pruned": pruned, "adopted": state == LayoutState::Legacy,
        }),
        human,
    );
    o.warnings = warnings;
    Ok(o)
}

// ─── rollback ───────────────────────────────────────────────────────────────

pub async fn rollback(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let l = layout(ctx);
    if l.state() != LayoutState::Versioned {
        return Err(CliError::general(
            "not a versioned install; nothing to roll back to (run `fpctl adopt` / `fpctl upgrade` first)",
        ));
    }
    let restart = !p.has("no-restart");
    if restart && !p.dry_run() && !system::has_systemd() {
        return Err(CliError::usage(
            "systemd is not available; re-run with --no-restart and restart the panel yourself",
        ));
    }
    let current = l.current_version().unwrap_or_default();
    let target = match version_flag(p, "to")? {
        Some(v) => v,
        None => l.previous_version().ok_or_else(|| {
            CliError::not_found("no previous release to roll back to (see `fpctl releases`)")
        })?,
    };
    if target == current {
        return Err(CliError::usage(format!(
            "{target} is already the current release"
        )));
    }
    if !l.release_dir(&target).join("bin/flamepanel").is_file() {
        return Err(CliError::not_found(format!(
            "release {target} is not installed"
        )));
    }
    let note = "the database is not rolled back; restore a backups/pre-upgrade-*.db file if the newer version migrated it";
    if p.dry_run() {
        return Ok(Outcome::new(
            json!({ "dry_run": true, "from": current, "to": target, "restart": restart }),
            format!(
                "would switch current {current} -> {target}{}",
                if restart { " and restart" } else { "" }
            ),
        )
        .warn(note));
    }
    require_writable(&l)?;
    l.switch(&target)?;
    let mut restarted = false;
    if restart {
        if let Err(e) = restart_and_verify(ctx, &target).await {
            let back =
                l.switch(&current).is_ok() && restart_and_verify(ctx, &current).await.is_ok();
            return Err(CliError::general(format!(
                "rollback to {target} failed: {}{}",
                e.message,
                if back {
                    format!(" — switched back to {current}")
                } else {
                    String::new()
                }
            )));
        }
        restarted = true;
    }
    Ok(Outcome::new(
        json!({ "changed": true, "from": current, "to": target, "restarted": restarted }),
        format!(
            "rolled back {current} -> {target}{}",
            if restarted {
                ""
            } else {
                " (not restarted; run `sudo fpctl restart`)"
            }
        ),
    )
    .warn(note))
}
