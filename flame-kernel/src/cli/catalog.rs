//! 命令目录：唯一事实来源。
//!
//! 参数解析、`--help`、`fpctl commands --json`（AI 自我发现）全部由这里驱动；
//! M-AI-1 的 MCP Server 复用同一份目录生成工具清单。新增命令 = 在 [`COMMANDS`] 加一条
//! + 在 `cli::dispatch` 加一个分支（单测保证两边一致）。

use serde::Serialize;
use serde_json::{json, Value};

use super::output::ExitCode;

/// 命令运行方式：离线（直读本机文件 / 数据库 / systemd）或在线（经本机回环调用 REST API）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 不依赖面板进程
    Offline,
    /// 需要面板在运行
    Online,
    /// 能在线时取更多信息，离线也可用
    Hybrid,
}

#[derive(Debug, Serialize)]
pub struct FlagSpec {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,
    /// 取值占位名；`None` 表示布尔开关
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<&'static str>,
    pub help: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ArgSpec {
    pub name: &'static str,
    pub required: bool,
    pub help: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CommandSpec {
    /// 空格分隔的命令路径，如 `user list`
    pub name: &'static str,
    pub summary: &'static str,
    pub args: &'static [ArgSpec],
    pub flags: &'static [FlagSpec],
    pub mode: Mode,
    /// 会改变系统状态（支持 `--dry-run`）
    pub writes: bool,
    /// 危险操作：非 `--dry-run` 时必须带 `--yes`
    pub dangerous: bool,
    /// 需要 root（或服务运行用户）权限
    pub needs_root: bool,
    pub example: &'static str,
}

const fn flag(name: &'static str, help: &'static str) -> FlagSpec {
    FlagSpec {
        name,
        short: None,
        value: None,
        help,
    }
}

const fn opt(name: &'static str, value: &'static str, help: &'static str) -> FlagSpec {
    FlagSpec {
        name,
        short: None,
        value: Some(value),
        help,
    }
}

const fn arg(name: &'static str, required: bool, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        required,
        help,
    }
}

/// 全局选项（所有命令可用）。
pub static GLOBAL_FLAGS: &[FlagSpec] = &[
    flag(
        "json",
        "machine-readable output: exactly one JSON document on stdout",
    ),
    opt(
        "dir",
        "PATH",
        "panel install dir (default: $FP_HOME, then /opt/flamepanel, then cwd)",
    ),
    opt(
        "url",
        "URL",
        "panel base URL for online commands (default: http://127.0.0.1:$OP_PORT)",
    ),
    flag(
        "dry-run",
        "print what a write command would do without doing it",
    ),
    FlagSpec {
        name: "yes",
        short: Some('y'),
        value: None,
        help: "confirm a dangerous operation",
    },
    FlagSpec {
        name: "help",
        short: Some('h'),
        value: None,
        help: "show help",
    },
];

const NONE_A: &[ArgSpec] = &[];
const NONE_F: &[FlagSpec] = &[];

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "commands",
        summary: "machine-readable command catalog (for scripts and AI agents)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl commands --json",
    },
    CommandSpec {
        name: "version",
        summary: "CLI version and running panel version",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Hybrid,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl version --json",
    },
    CommandSpec {
        name: "status",
        summary: "service state, health checks, listen address, disk usage (exit 3 if not running)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Hybrid,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl status --json",
    },
    CommandSpec {
        name: "info",
        summary: "access URL, admin usernames and install paths (never prints passwords)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl info",
    },
    CommandSpec {
        name: "start",
        summary: "start the panel service via systemd and wait until healthy",
        args: NONE_A,
        flags: &[flag("no-wait", "return immediately without waiting for health")],
        mode: Mode::Offline,
        writes: true,
        dangerous: false,
        needs_root: true,
        example: "sudo fpctl start",
    },
    CommandSpec {
        name: "stop",
        summary: "stop the panel service via systemd",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: true,
        dangerous: true,
        needs_root: true,
        example: "sudo fpctl stop --yes",
    },
    CommandSpec {
        name: "restart",
        summary: "restart the panel service via systemd and wait until healthy",
        args: NONE_A,
        flags: &[flag("no-wait", "return immediately without waiting for health")],
        mode: Mode::Offline,
        writes: true,
        dangerous: false,
        needs_root: true,
        example: "sudo fpctl restart",
    },
    CommandSpec {
        name: "doctor",
        summary: "diagnose install: config, secret, database, disk, log dir, service, health (exit 1 on failures)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Hybrid,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl doctor --json",
    },
    CommandSpec {
        name: "logs",
        summary: "read kernel log files directly (works while the panel is down)",
        args: NONE_A,
        flags: &[
            opt("file", "NAME", "log file name (default: today's file)"),
            opt("level", "LIST", "comma-separated levels: trace,debug,info,warn,error"),
            opt("grep", "TEXT", "case-insensitive substring (message / fields / request id)"),
            opt("target", "PREFIX", "module path prefix, e.g. flame_kernel::api"),
            FlagSpec {
                name: "limit",
                short: Some('n'),
                value: Some("N"),
                help: "number of newest entries (default 100, max 1000)",
            },
            FlagSpec {
                name: "follow",
                short: Some('f'),
                value: None,
                help: "keep printing new entries (NDJSON with --json)",
            },
        ],
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl logs --level warn,error -n 50",
    },
    CommandSpec {
        name: "logs files",
        summary: "list kernel log files (daily, rotated and crash reports)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl logs files --json",
    },
    CommandSpec {
        name: "log-level get",
        summary: "show the current kernel log level (runtime if online, else configured)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Hybrid,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl log-level get",
    },
    CommandSpec {
        name: "log-level set",
        summary: "change the kernel log level at runtime (reverts to OP_LOG_LEVEL on restart)",
        args: &[arg("level", true, "trace | debug | info | warn | error")],
        flags: NONE_F,
        mode: Mode::Online,
        writes: true,
        dangerous: false,
        needs_root: false,
        example: "fpctl log-level set debug",
    },
    CommandSpec {
        name: "user list",
        summary: "list panel users (id, username, role, must-change-password)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl user list --json",
    },
    CommandSpec {
        name: "reset-password",
        summary: "set a new password (random unless --password-stdin) and revoke the user's sessions",
        args: NONE_A,
        flags: &[
            opt("user", "NAME", "username (default: the first admin)"),
            flag("password-stdin", "read the new password from stdin instead of generating one"),
            flag("no-force-change", "do not require a password change at next login"),
        ],
        mode: Mode::Offline,
        writes: true,
        dangerous: true,
        needs_root: false,
        example: "sudo fpctl reset-password --user admin --yes",
    },
    CommandSpec {
        name: "reset-2fa",
        summary: "remove a user's two-factor authentication (lost authenticator and recovery codes)",
        args: NONE_A,
        flags: &[opt("user", "NAME", "username (default: the first admin)")],
        mode: Mode::Offline,
        writes: true,
        dangerous: true,
        needs_root: false,
        example: "sudo fpctl reset-2fa --user admin --yes",
    },
    CommandSpec {
        name: "port set",
        summary: "change the backend listen port (env file + nginx proxy_pass), optionally restart",
        args: &[arg("port", true, "1-65535")],
        flags: &[
            flag("restart", "restart the service afterwards"),
            flag("no-nginx", "do not touch /etc/nginx/conf.d/flamepanel.conf"),
        ],
        mode: Mode::Offline,
        writes: true,
        dangerous: false,
        needs_root: true,
        example: "sudo fpctl port set 9090 --restart",
    },
    CommandSpec {
        name: "api",
        summary: "call any REST endpoint as an admin with a 60 s token (full panel coverage)",
        args: &[
            arg("method", true, "GET | POST | PUT | PATCH | DELETE"),
            arg("path", true, "API path, e.g. /api/websites?page=1"),
        ],
        flags: &[
            FlagSpec {
                name: "data",
                short: Some('d'),
                value: Some("JSON"),
                help: "request body: inline JSON, @file, or - for stdin",
            },
            opt("user", "NAME", "act as this user (default: the first admin)"),
        ],
        mode: Mode::Online,
        writes: true,
        dangerous: false,
        needs_root: false,
        example: "fpctl api GET /api/websites --json",
    },
    CommandSpec {
        name: "openapi",
        summary: "print the OpenAPI 3 document embedded in this build",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl openapi > openapi.json",
    },
    CommandSpec {
        name: "backup list",
        summary: "list panel backups",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Online,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl backup list --json",
    },
    CommandSpec {
        name: "backup create",
        summary: "create a panel backup now",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Online,
        writes: true,
        dangerous: false,
        needs_root: false,
        example: "fpctl backup create",
    },
    CommandSpec {
        name: "releases",
        summary: "list installed releases (versioned layout: releases/<ver>, current symlink)",
        args: NONE_A,
        flags: NONE_F,
        mode: Mode::Offline,
        writes: false,
        dangerous: false,
        needs_root: false,
        example: "fpctl releases --json",
    },
    CommandSpec {
        name: "adopt",
        summary: "convert a legacy install (plain binaries) into the versioned release layout; idempotent",
        args: NONE_A,
        flags: &[opt("version", "VER", "release name (default: `flamepanel --version`)")],
        mode: Mode::Offline,
        writes: true,
        dangerous: false,
        needs_root: false,
        example: "sudo fpctl adopt",
    },
    CommandSpec {
        name: "upgrade",
        summary: "install a release (verify checksums, back up the database, switch, restart, auto-rollback on failure)",
        args: NONE_A,
        flags: &[
            opt("version", "VER", "release to install (default: latest)"),
            opt("from-dir", "DIR", "use local release artifacts instead of downloading"),
            opt("source", "URL", "release site base (default: GitHub + mirrors)"),
            opt("keep", "N", "releases to keep after upgrading (default 3)"),
            flag("no-restart", "switch only; do not restart the service"),
            flag("force", "reinstall even if already on this version"),
        ],
        mode: Mode::Offline,
        writes: true,
        dangerous: true,
        needs_root: false,
        example: "sudo fpctl upgrade --version 1.1.0 --yes",
    },
    CommandSpec {
        name: "rollback",
        summary: "switch back to the previous (or a given) installed release and restart",
        args: NONE_A,
        flags: &[
            opt("to", "VER", "release to switch to (default: previous)"),
            flag("no-restart", "switch only; do not restart the service"),
        ],
        mode: Mode::Offline,
        writes: true,
        dangerous: true,
        needs_root: false,
        example: "sudo fpctl rollback --yes",
    },
];

pub fn find(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// 所有取值型选项名（全局 + 各命令并集），供分词阶段判断是否吞下一个 token。
pub fn value_flag(name: &str) -> bool {
    GLOBAL_FLAGS
        .iter()
        .chain(COMMANDS.iter().flat_map(|c| c.flags.iter()))
        .any(|f| f.name == name && f.value.is_some())
}

/// 短选项 → 长选项名。
pub fn short_flag(c: char) -> Option<&'static str> {
    GLOBAL_FLAGS
        .iter()
        .chain(COMMANDS.iter().flat_map(|c| c.flags.iter()))
        .find(|f| f.short == Some(c))
        .map(|f| f.name)
}

/// `fpctl commands --json` 的数据体。
pub fn catalog_json() -> Value {
    let exit_codes: Vec<Value> = ExitCode::ALL
        .iter()
        .map(|c| json!({ "code": c.code(), "name": c.as_str(), "meaning": c.describe() }))
        .collect();
    json!({
        "cli": "fpctl",
        "version": crate::VERSION,
        "contract": {
            "json": "with --json, stdout carries exactly one JSON document: {ok, command, exit, data | error, warnings?}; logs --follow emits NDJSON",
            "dry_run": "commands with writes=true accept --dry-run",
            "confirm": "commands with dangerous=true require --yes unless --dry-run",
            "exit_codes": exit_codes,
        },
        "global_flags": GLOBAL_FLAGS,
        "commands": COMMANDS,
    })
}

/// 人类可读的总帮助。
pub fn help_text() -> String {
    let mut s = format!(
        "fpctl {} — Flamepanel command line\n\nUSAGE:\n  fpctl <command> [args] [flags]\n\nCOMMANDS:\n",
        crate::VERSION
    );
    let w = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for c in COMMANDS {
        s.push_str(&format!("  {:<w$}  {}\n", c.name, c.summary, w = w));
    }
    s.push_str("\nGLOBAL FLAGS:\n");
    s.push_str(&flags_text(GLOBAL_FLAGS));
    s.push_str("\nEXIT CODES:\n");
    for c in ExitCode::ALL {
        s.push_str(&format!("  {}  {}\n", c.code(), c.describe()));
    }
    s.push_str("\nRun `fpctl <command> --help` for details, `fpctl commands --json` for the machine-readable catalog.\n");
    s
}

fn flags_text(flags: &[FlagSpec]) -> String {
    let mut s = String::new();
    for f in flags {
        let short = f.short.map(|c| format!("-{c}, ")).unwrap_or_default();
        let val = f.value.map(|v| format!(" <{v}>")).unwrap_or_default();
        s.push_str(&format!(
            "  {:<28}{}\n",
            format!("{short}--{}{val}", f.name),
            f.help
        ));
    }
    s
}

/// 单个命令的帮助。
pub fn command_help(c: &CommandSpec) -> String {
    let args: String = c
        .args
        .iter()
        .map(|a| {
            if a.required {
                format!(" <{}>", a.name)
            } else {
                format!(" [{}]", a.name)
            }
        })
        .collect();
    let mut s = format!(
        "fpctl {}{args} — {}\n\nmode: {:?}{}{}{}\nexample: {}\n",
        c.name,
        c.summary,
        c.mode,
        if c.writes {
            " · writes (--dry-run)"
        } else {
            ""
        },
        if c.dangerous {
            " · dangerous (--yes)"
        } else {
            ""
        },
        if c.needs_root { " · needs root" } else { "" },
        c.example
    );
    if !c.args.is_empty() {
        s.push_str("\nARGS:\n");
        for a in c.args {
            s.push_str(&format!("  {:<28}{}\n", a.name, a.help));
        }
    }
    if !c.flags.is_empty() {
        s.push_str("\nFLAGS:\n");
        s.push_str(&flags_text(c.flags));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique_and_well_formed() {
        let mut seen = HashSet::new();
        for c in COMMANDS {
            assert!(seen.insert(c.name), "duplicate command {}", c.name);
            assert!(!c.summary.is_empty(), "{}", c.name);
            assert!(
                c.example.starts_with("fpctl ") || c.example.starts_with("sudo fpctl "),
                "{}",
                c.name
            );
            assert!(
                !c.dangerous || c.writes,
                "{} dangerous implies writes",
                c.name
            );
        }
    }

    #[test]
    fn flags_do_not_clash_with_globals_or_each_other() {
        let globals: HashSet<_> = GLOBAL_FLAGS.iter().map(|f| f.name).collect();
        let mut shorts: std::collections::HashMap<char, &str> = GLOBAL_FLAGS
            .iter()
            .filter_map(|f| f.short.map(|s| (s, f.name)))
            .collect();
        for c in COMMANDS {
            for f in c.flags {
                assert!(
                    !globals.contains(f.name),
                    "{} redefines global --{}",
                    c.name,
                    f.name
                );
                if let Some(s) = f.short {
                    let prev = shorts.insert(s, f.name);
                    assert!(
                        prev.is_none() || prev == Some(f.name),
                        "-{s} maps to two flags"
                    );
                }
            }
        }
        // 同名选项在不同命令中必须一致地取值/不取值（分词阶段依赖此约定）
        for c in COMMANDS {
            for f in c.flags {
                assert_eq!(
                    value_flag(f.name),
                    f.value.is_some(),
                    "--{} arity differs",
                    f.name
                );
            }
        }
    }

    #[test]
    fn catalog_json_lists_every_command() {
        let v = catalog_json();
        let names: Vec<&str> = v["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(names.len(), COMMANDS.len());
        assert!(names.contains(&"reset-password"));
        assert_eq!(v["contract"]["exit_codes"][3]["name"], "NOT_RUNNING");
        assert_eq!(v["global_flags"][0]["name"], "json");
    }

    #[test]
    fn help_mentions_every_command() {
        let h = help_text();
        for c in COMMANDS {
            assert!(h.contains(c.name));
            assert!(command_help(c).contains(c.example));
        }
    }
}
