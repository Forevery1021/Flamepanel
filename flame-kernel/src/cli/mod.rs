//! `fpctl` —— 面板命令行（对标 1pctl，面向运维脚本与 AI Agent）。设计见 Doc/32 §3。
//!
//! - [`catalog`]：命令目录（唯一事实来源，驱动解析 / 帮助 / `commands --json`）
//! - [`args`]：手写参数解析（不引入 clap）
//! - [`output`]：退出码与 JSON 信封契约
//! - [`context`]：安装目录定位与配置解析（与内核同优先级）
//! - `cmd_*`：各命令实现

pub mod args;
mod artifacts;
pub mod catalog;
mod cmd_api;
mod cmd_logs;
mod cmd_port;
mod cmd_release;
mod cmd_service;
mod cmd_user;
pub mod context;
mod db;
mod http;
pub mod mcp_stdio;
pub mod output;
pub mod release;
mod system;

use std::io::Write;

use args::Parsed;
use context::Ctx;
use output::{error_envelope, success_envelope, CliError, ExitCode, Outcome};

/// 已实现的命令（单测保证与 [`catalog::COMMANDS`] 一致）。
#[cfg(test)]
const IMPLEMENTED: &[&str] = &[
    "commands",
    "version",
    "status",
    "info",
    "start",
    "stop",
    "restart",
    "doctor",
    "logs",
    "logs files",
    "log-level get",
    "log-level set",
    "user list",
    "reset-password",
    "reset-2fa",
    "port set",
    "api",
    "openapi",
    "backup list",
    "backup create",
    "releases",
    "adopt",
    "upgrade",
    "rollback",
];

/// 不需要安装上下文的命令（不 chdir、不读配置）。
fn context_free(name: &str) -> bool {
    matches!(name, "commands" | "openapi")
}

async fn dispatch(p: &Parsed) -> Result<Outcome, CliError> {
    let name = p.command_name();
    if context_free(name) {
        return match name {
            "commands" => {
                let data = catalog::catalog_json();
                Ok(Outcome::new(data, catalog::help_text()))
            }
            _ => cmd_api::openapi().await,
        };
    }
    let ctx = Ctx::resolve(p.value("dir"), p.value("url"))?;
    let mut out = match name {
        "version" => cmd_service::version(&ctx).await,
        "status" => cmd_service::status(&ctx).await,
        "info" => cmd_service::info(&ctx).await,
        "start" | "stop" | "restart" => cmd_service::service_verb(&ctx, p, name).await,
        "doctor" => cmd_service::doctor(&ctx).await,
        "logs" => cmd_logs::logs(&ctx, p).await,
        "logs files" => cmd_logs::files(&ctx).await,
        "log-level get" => cmd_logs::level_get(&ctx).await,
        "log-level set" => cmd_logs::level_set(&ctx, p).await,
        "user list" => cmd_user::list(&ctx).await,
        "reset-password" => cmd_user::reset_password(&ctx, p).await,
        "reset-2fa" => cmd_user::reset_two_factor(&ctx, p).await,
        "port set" => cmd_port::port_set(&ctx, p).await,
        "api" => cmd_api::api(&ctx, p).await,
        "backup list" => cmd_api::backup_list(&ctx).await,
        "backup create" => cmd_api::backup_create(&ctx, p).await,
        "releases" => cmd_release::releases(&ctx).await,
        "adopt" => cmd_release::adopt(&ctx, p).await,
        "upgrade" => cmd_release::upgrade(&ctx, p).await,
        "rollback" => cmd_release::rollback(&ctx, p).await,
        other => Err(CliError::general(format!(
            "command '{other}' is not implemented"
        ))),
    }?;
    // doctor 已把配置告警纳入检查项，其余命令附带上下文告警
    if name != "doctor" {
        let mut w = ctx.warnings.clone();
        w.append(&mut out.warnings);
        out.warnings = w;
    }
    Ok(out)
}

/// 前置校验：危险操作确认、root 权限。
fn preflight(p: &Parsed) -> Result<(), CliError> {
    let Some(spec) = p.command else {
        return Ok(());
    };
    if p.dry_run() && !spec.writes {
        return Err(CliError::usage(format!(
            "`{}` is read-only; --dry-run applies to write commands",
            spec.name
        )));
    }
    if p.dry_run() {
        return Ok(());
    }
    if spec.dangerous && !p.yes() {
        return Err(CliError::usage(format!(
            "`{}` is dangerous; re-run with --yes (or preview with --dry-run)",
            spec.name
        )));
    }
    if spec.needs_root && !system::is_root() {
        return Err(CliError::permission(format!(
            "`{}` needs root; re-run with sudo",
            spec.name
        )));
    }
    Ok(())
}

fn print_json(v: &serde_json::Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(
        out,
        "{}",
        serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".into())
    );
}

fn report_error(command: &str, e: &CliError, json: bool) -> i32 {
    if json {
        print_json(&error_envelope(command, e));
    } else {
        eprintln!("fpctl: {}", e.message);
        if let Some(d) = &e.detail {
            if let Some(recent) = d.get("recent_errors").and_then(|r| r.as_array()) {
                for r in recent {
                    eprintln!(
                        "  {} {}: {}",
                        r["timestamp"].as_str().unwrap_or(""),
                        r["target"].as_str().unwrap_or(""),
                        r["message"].as_str().unwrap_or("")
                    );
                }
            }
        }
    }
    e.exit.code()
}

/// CLI 入口：返回进程退出码。
pub fn run(argv: Vec<String>) -> i32 {
    let wants_json = argv.iter().any(|a| a == "--json");
    let p = match args::parse(&argv) {
        Ok(p) => p,
        Err(e) => return report_error("", &e, wants_json),
    };

    if p.help() || p.command.is_none() {
        match p.command {
            Some(c) if p.json() => print_json(&serde_json::to_value(c).unwrap_or_default()),
            Some(c) => print!("{}", catalog::command_help(c)),
            None if p.json() => print_json(&catalog::catalog_json()),
            None => print!("{}", catalog::help_text()),
        }
        return ExitCode::Ok.code();
    }
    let name = p.command_name();
    if let Err(e) = preflight(&p) {
        return report_error(name, &e, p.json());
    }

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return report_error(name, &CliError::general(format!("runtime: {e}")), p.json()),
    };
    match rt.block_on(dispatch(&p)) {
        Ok(o) if o.streamed => ExitCode::Ok.code(),
        Ok(o) => {
            if p.json() {
                print_json(&success_envelope(name, &o));
            } else {
                for w in &o.warnings {
                    eprintln!("warning: {w}");
                }
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(o.human.as_bytes());
                if !o.human.is_empty() && !o.human.ends_with('\n') {
                    let _ = out.write_all(b"\n");
                }
            }
            o.exit.unwrap_or(ExitCode::Ok).code()
        }
        Err(e) => report_error(name, &e, p.json()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_command_is_implemented_and_vice_versa() {
        for c in catalog::COMMANDS {
            assert!(IMPLEMENTED.contains(&c.name), "{} missing handler", c.name);
        }
        for n in IMPLEMENTED {
            assert!(catalog::find(n).is_some(), "{n} missing from catalog");
        }
    }

    fn parsed(s: &str) -> Parsed {
        let v: Vec<String> = s.split_whitespace().map(String::from).collect();
        args::parse(&v).unwrap()
    }

    #[test]
    fn preflight_rules() {
        // 危险操作需 --yes，--dry-run 可预览
        assert_eq!(
            preflight(&parsed("stop")).unwrap_err().exit,
            ExitCode::Usage
        );
        assert_eq!(
            preflight(&parsed("reset-password")).unwrap_err().exit,
            ExitCode::Usage
        );
        assert!(preflight(&parsed("reset-password --dry-run")).is_ok());
        // 只读命令拒绝 --dry-run（避免误以为「预演」了什么）
        assert_eq!(
            preflight(&parsed("status --dry-run")).unwrap_err().exit,
            ExitCode::Usage
        );
        assert!(preflight(&parsed("status")).is_ok());
        if !system::is_root() {
            assert_eq!(
                preflight(&parsed("restart")).unwrap_err().exit,
                ExitCode::Permission
            );
            assert!(preflight(&parsed("restart --dry-run")).is_ok());
        }
    }

    #[test]
    fn run_returns_usage_code_for_bad_args() {
        assert_eq!(run(vec!["nope".into()]), 2);
        assert_eq!(run(vec!["--help".into()]), 0);
        assert_eq!(run(vec!["logs".into(), "--help".into()]), 0);
    }
}
