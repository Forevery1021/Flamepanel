//! 手写参数解析（不引入 clap——「不加依赖」红线）。
//!
//! 规则：
//! - `--name value` / `--name=value` / `-x value`；布尔开关不吞下一个 token；
//! - 取值与否由 [`catalog`](super::catalog) 统一定义（同名选项全局一致）；
//! - `--` 之后全部视为位置参数；
//! - 命令 = 位置参数的**最长前缀**匹配目录中的命令路径（`logs files` 优先于 `logs`）；
//! - 选项必须属于全局或该命令，否则报参数错误（退出码 2）。

use std::collections::BTreeMap;

use super::catalog::{self, CommandSpec, GLOBAL_FLAGS};
use super::output::CliError;

#[derive(Debug, Default, Clone)]
pub struct Parsed {
    /// 匹配到的命令；`None` = 仅 `fpctl` / `fpctl --help`
    pub command: Option<&'static CommandSpec>,
    /// 命令路径之后的位置参数
    pub args: Vec<String>,
    /// 选项（长名 → 值；布尔开关值为 `None`）
    pub flags: BTreeMap<String, Option<String>>,
}

impl Parsed {
    pub fn has(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }
    pub fn value(&self, name: &str) -> Option<&str> {
        self.flags.get(name).and_then(|v| v.as_deref())
    }
    pub fn json(&self) -> bool {
        self.has("json")
    }
    pub fn dry_run(&self) -> bool {
        self.has("dry-run")
    }
    pub fn yes(&self) -> bool {
        self.has("yes")
    }
    pub fn help(&self) -> bool {
        self.has("help")
    }
    pub fn command_name(&self) -> &'static str {
        self.command.map(|c| c.name).unwrap_or("")
    }
}

/// 解析 argv（不含程序名）。
pub fn parse(argv: &[String]) -> Result<Parsed, CliError> {
    let mut positionals: Vec<String> = Vec::new();
    let mut flags: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut it = argv.iter().peekable();
    let mut rest_positional = false;

    while let Some(tok) = it.next() {
        if rest_positional {
            positionals.push(tok.clone());
            continue;
        }
        if tok == "--" {
            rest_positional = true;
            continue;
        }
        let (name, inline): (String, Option<String>) = if let Some(long) = tok.strip_prefix("--") {
            match long.split_once('=') {
                Some((n, v)) => (n.to_string(), Some(v.to_string())),
                None => (long.to_string(), None),
            }
        } else if tok.len() == 2 && tok.starts_with('-') && tok != "-" {
            let c = tok.chars().nth(1).unwrap_or('-');
            match catalog::short_flag(c) {
                Some(n) => (n.to_string(), None),
                None => return Err(CliError::usage(format!("unknown flag '{tok}'"))),
            }
        } else if tok.starts_with('-') && tok.len() > 2 && !tok.starts_with("--") {
            return Err(CliError::usage(format!(
                "unknown flag '{tok}' (combined short flags are not supported)"
            )));
        } else {
            positionals.push(tok.clone());
            continue;
        };

        if name.is_empty() {
            return Err(CliError::usage(format!("invalid flag '{tok}'")));
        }
        let takes_value = catalog::value_flag(&name);
        let value = if takes_value {
            match inline {
                Some(v) => Some(v),
                None => match it.next() {
                    Some(v) => Some(v.clone()),
                    None => return Err(CliError::usage(format!("flag --{name} requires a value"))),
                },
            }
        } else {
            if inline.is_some() {
                return Err(CliError::usage(format!(
                    "flag --{name} does not take a value"
                )));
            }
            None
        };
        flags.insert(name, value);
    }

    // `fpctl help [cmd…]` 等价于 `fpctl [cmd…] --help`
    if positionals.first().map(String::as_str) == Some("help") {
        positionals.remove(0);
        flags.insert("help".into(), None);
    }

    let command = match_command(&positionals);
    let args = match command {
        Some(c) => positionals[c.name.split(' ').count()..].to_vec(),
        None => {
            if let Some(first) = positionals.first() {
                // 仅有命令组前缀（如 `fpctl user`）时给出可选子命令
                let group: Vec<&str> = catalog::COMMANDS
                    .iter()
                    .filter(|c| c.name.split(' ').next() == Some(first.as_str()))
                    .map(|c| c.name)
                    .collect();
                if group.is_empty() {
                    return Err(CliError::usage(format!(
                        "unknown command '{first}' (run `fpctl --help`)"
                    )));
                }
                if !flags.contains_key("help") {
                    return Err(CliError::usage(format!(
                        "'{first}' needs a subcommand: {}",
                        group.join(", ")
                    )));
                }
            }
            Vec::new()
        }
    };

    // 选项归属校验
    for name in flags.keys() {
        let global = GLOBAL_FLAGS.iter().any(|f| f.name == name);
        let local = command.is_some_and(|c| c.flags.iter().any(|f| f.name == name));
        if !global && !local {
            return Err(match command {
                Some(c) => CliError::usage(format!("unknown flag --{name} for `{}`", c.name)),
                None => CliError::usage(format!("unknown flag --{name}")),
            });
        }
    }

    // 位置参数个数校验（--help 时跳过）
    if let Some(c) = command {
        if !flags.contains_key("help") {
            let required = c.args.iter().filter(|a| a.required).count();
            if args.len() < required {
                let missing: Vec<&str> = c.args[args.len()..]
                    .iter()
                    .filter(|a| a.required)
                    .map(|a| a.name)
                    .collect();
                return Err(CliError::usage(format!(
                    "`{}` missing argument(s): {}",
                    c.name,
                    missing.join(", ")
                )));
            }
            if args.len() > c.args.len() {
                return Err(CliError::usage(format!(
                    "`{}` got unexpected argument '{}'",
                    c.name,
                    args[c.args.len()]
                )));
            }
        }
    }

    Ok(Parsed {
        command,
        args,
        flags,
    })
}

/// 最长前缀匹配。
fn match_command(pos: &[String]) -> Option<&'static CommandSpec> {
    catalog::COMMANDS
        .iter()
        .filter(|c| {
            let parts: Vec<&str> = c.name.split(' ').collect();
            parts.len() <= pos.len() && parts.iter().zip(pos).all(|(a, b)| *a == b)
        })
        .max_by_key(|c| c.name.split(' ').count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::output::ExitCode;

    fn p(s: &str) -> Result<Parsed, CliError> {
        let v: Vec<String> = s.split_whitespace().map(String::from).collect();
        parse(&v)
    }

    #[test]
    fn empty_and_help() {
        let r = p("").unwrap();
        assert!(r.command.is_none());
        let r = p("--help").unwrap();
        assert!(r.help() && r.command.is_none());
        let r = p("help logs").unwrap();
        assert_eq!(r.command_name(), "logs");
        assert!(r.help());
        // --help 跳过缺参校验
        let r = p("api --help").unwrap();
        assert_eq!(r.command_name(), "api");
    }

    #[test]
    fn longest_prefix_wins() {
        assert_eq!(p("logs").unwrap().command_name(), "logs");
        assert_eq!(p("logs files").unwrap().command_name(), "logs files");
        assert_eq!(p("log-level set debug").unwrap().args, vec!["debug"]);
    }

    #[test]
    fn flags_anywhere_with_values() {
        let r = p("--json logs -n 5 --level=warn,error -f --dir /srv/fp").unwrap();
        assert_eq!(r.command_name(), "logs");
        assert!(r.json());
        assert_eq!(r.value("limit"), Some("5"));
        assert_eq!(r.value("level"), Some("warn,error"));
        assert!(r.has("follow"));
        assert_eq!(r.value("dir"), Some("/srv/fp"));
    }

    #[test]
    fn api_args_and_stdin_data() {
        let r = p("api post /api/backups -d - -y").unwrap();
        assert_eq!(r.args, vec!["post", "/api/backups"]);
        assert_eq!(r.value("data"), Some("-"));
        assert!(r.yes());
    }

    #[test]
    fn double_dash_makes_rest_positional() {
        let r = p("api GET -- --weird").unwrap();
        assert_eq!(r.args, vec!["GET", "--weird"]);
    }

    #[test]
    fn usage_errors() {
        for bad in [
            "nope",
            "user",
            "logs --bogus",
            "status --limit 5",
            "logs -n",
            "api GET",
            "log-level set debug extra",
            "status --json=1",
            "logs -nf",
            "-z",
        ] {
            let e = p(bad).unwrap_err();
            assert_eq!(e.exit, ExitCode::Usage, "{bad}: {}", e.message);
        }
        assert!(p("user").unwrap_err().message.contains("user list"));
    }
}
