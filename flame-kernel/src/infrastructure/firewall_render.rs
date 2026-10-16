//! 防火墙规则 → 各后端命令参数（S-13 · 纯函数，便于逐后端断言 argv）。
//!
//! 输入是已落库的规则（写入时已经 `domain::firewall::normalize_rule` 规范化）；这里再解析一次
//! 端口与地址，对历史遗留的非法数据给出 400 而不是把脏字符串交给系统命令。
//!
//! | | ufw | firewalld | iptables |
//! |---|---|---|---|
//! | 端口段 | `8000:9000` | `8000-9000` | `8000:9000` |
//! | 多端口 | `80,443,8000:9000`（需 proto） | 每项一条 rich rule | `-m multiport --dports` |
//! | 方向 | `in` / `out` | 仅入站（rich rule） | `INPUT` / `OUTPUT` |
//! | 远端地址 | 入站 `from X`，出站 `to X` | `source address="X"` + family | 入站 `-s`，出站 `-d`；IPv6 → `ip6tables` |
//! | ICMP | 不支持（CLI 无 `proto icmp`） | `protocol value="icmp"` | `-p icmp` / `-p ipv6-icmp` |
use crate::core::error::AppError;
use crate::domain::entity::FirewallRule;
use crate::domain::firewall::{parse_address, parse_ports, AddressSpec, PortItem};

/// 添加还是删除。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOp {
    Add,
    Delete,
}

/// 从规则里解析出的渲染要素。
struct Parts {
    ports: Vec<PortItem>,
    /// 远端地址：入站 = 来源，出站 = 目标；`None` = 任意
    remote: Option<AddressSpec>,
    inbound: bool,
}

fn invalid(rule: &FirewallRule, reason: impl std::fmt::Display) -> AppError {
    AppError::BadRequest(format!(
        "firewall rule '{}' is invalid ({reason}); edit and save it again",
        rule.name
    ))
}

fn parts(rule: &FirewallRule) -> Result<Parts, AppError> {
    let ports = parse_ports(rule.port.as_deref().unwrap_or("")).map_err(|r| invalid(rule, r))?;
    if !ports.is_empty() && !matches!(rule.protocol.as_str(), "tcp" | "udp") {
        return Err(invalid(rule, "ports require protocol tcp or udp"));
    }
    if !matches!(rule.protocol.as_str(), "tcp" | "udp" | "icmp" | "any") {
        return Err(invalid(
            rule,
            format!("unknown protocol '{}'", rule.protocol),
        ));
    }
    let remote =
        parse_address(rule.source.as_deref().unwrap_or("")).map_err(|r| invalid(rule, r))?;
    Ok(Parts {
        ports,
        remote,
        inbound: rule.direction != "out",
    })
}

fn join_ports(ports: &[PortItem], sep: char) -> String {
    ports
        .iter()
        .map(|p| p.render(sep))
        .collect::<Vec<_>>()
        .join(",")
}

/// `ufw` 完整语法：`[delete] ACTION in|out [proto P] from A to B [port PORTS]`（不含程序名）。
pub fn ufw_args(rule: &FirewallRule, op: RuleOp) -> Result<Vec<String>, AppError> {
    let p = parts(rule)?;
    let action = match rule.action.as_str() {
        "deny" => "deny",
        "reject" => "reject",
        _ => "allow",
    };
    let mut args: Vec<String> = Vec::new();
    if op == RuleOp::Delete {
        args.push("delete".into());
    }
    args.push(action.into());
    args.push(if p.inbound { "in" } else { "out" }.into());
    match rule.protocol.as_str() {
        "tcp" | "udp" => {
            args.push("proto".into());
            args.push(rule.protocol.clone());
        }
        "icmp" => {
            return Err(AppError::BadRequest(
                "ufw does not support ICMP rules from the command line; use protocol 'any' or edit /etc/ufw/before.rules".into(),
            ))
        }
        _ => {}
    }
    let remote = p
        .remote
        .map(|a| a.to_string())
        .unwrap_or_else(|| "any".into());
    let (from, to) = if p.inbound {
        (remote, "any".to_string())
    } else {
        ("any".to_string(), remote)
    };
    args.extend(["from".into(), from, "to".into(), to]);
    if !p.ports.is_empty() {
        args.push("port".into());
        args.push(join_ports(&p.ports, ':'));
    }
    Ok(args)
}

/// firewalld rich rules（每个端口项一条）；只支持入站。
pub fn firewalld_rich_rules(rule: &FirewallRule) -> Result<Vec<String>, AppError> {
    let p = parts(rule)?;
    if !p.inbound {
        return Err(AppError::BadRequest(
            "firewalld rich rules only filter inbound traffic; outbound rules need ufw or iptables"
                .into(),
        ));
    }
    let mut head = String::from("rule");
    if let Some(addr) = p.remote {
        let family = if addr.is_ipv6() { "ipv6" } else { "ipv4" };
        head.push_str(&format!(" family=\"{family}\" source address=\"{addr}\""));
    }
    let action = match rule.action.as_str() {
        "deny" => "drop",
        "reject" => "reject",
        _ => "accept",
    };
    let ipv6 = p.remote.is_some_and(|a| a.is_ipv6());
    let rules = if p.ports.is_empty() {
        let element = match rule.protocol.as_str() {
            "tcp" | "udp" => format!(" protocol value=\"{}\"", rule.protocol),
            "icmp" if ipv6 => " protocol value=\"ipv6-icmp\"".to_string(),
            "icmp" => " protocol value=\"icmp\"".to_string(),
            _ => String::new(),
        };
        vec![format!("{head}{element} {action}")]
    } else {
        p.ports
            .iter()
            .map(|port| {
                format!(
                    "{head} port port=\"{}\" protocol=\"{}\" {action}",
                    port.render('-'),
                    rule.protocol
                )
            })
            .collect()
    };
    Ok(rules)
}

/// `(program, args)`：IPv6 远端地址走 `ip6tables`。
pub fn iptables_command(
    rule: &FirewallRule,
    op: RuleOp,
) -> Result<(&'static str, Vec<String>), AppError> {
    let p = parts(rule)?;
    let ipv6 = p.remote.is_some_and(|a| a.is_ipv6());
    let program = if ipv6 { "ip6tables" } else { "iptables" };
    let mut args: Vec<String> = vec![
        match op {
            RuleOp::Add => "-A",
            RuleOp::Delete => "-D",
        }
        .into(),
        if p.inbound { "INPUT" } else { "OUTPUT" }.into(),
    ];
    if let Some(addr) = p.remote {
        args.push(if p.inbound { "-s" } else { "-d" }.into());
        args.push(addr.to_string());
    }
    match rule.protocol.as_str() {
        "tcp" | "udp" => {
            args.push("-p".into());
            args.push(rule.protocol.clone());
        }
        "icmp" => {
            args.push("-p".into());
            args.push(if ipv6 { "ipv6-icmp" } else { "icmp" }.into());
        }
        _ => {}
    }
    match p.ports.as_slice() {
        [] => {}
        [single] => {
            args.push("--dport".into());
            args.push(single.render(':'));
        }
        many => {
            args.extend(["-m".into(), "multiport".into(), "--dports".into()]);
            args.push(join_ports(many, ':'));
        }
    }
    let target = match rule.action.as_str() {
        "deny" => "DROP",
        "reject" => "REJECT",
        _ => "ACCEPT",
    };
    args.push("-j".into());
    args.push(target.into());
    Ok((program, args))
}

// ─── 特权助手白名单：往返校验（Doc/30 助手扩展 · Doc/33 §5-1）────────────────
//
// 助手以 root 执行防火墙命令前，把 argv 反解析为候选规则，再用上面**同一组**渲染函数重新渲染，
// 逐字节一致才放行。于是助手只会执行「某条合法规则的规范渲染结果」：端口 / 地址 / 协议 / 动作
// 都已经过 `parts` 校验，任何多余参数、注释、换行或非规范写法都会因渲染不一致被拒。

/// 往返校验用的候选规则（只填渲染用到的字段）。
fn candidate(
    protocol: &str,
    port: Option<&str>,
    remote: Option<&str>,
    action: &str,
    direction: &str,
) -> FirewallRule {
    let now = chrono::Utc::now();
    FirewallRule {
        id: 0,
        name: "privileged-helper".into(),
        description: None,
        protocol: protocol.into(),
        port: port.map(str::to_owned),
        source: remote.map(str::to_owned),
        destination: None,
        action: action.into(),
        enabled: true,
        priority: 0,
        direction: direction.into(),
        created_at: now,
        updated_at: now,
    }
}

/// 不带规则参数的防火墙命令（探测 / 状态 / 启停）。
pub const FIREWALL_FIXED_COMMANDS: &[&[&str]] = &[
    &["ufw"],
    &["ufw", "status"],
    &["ufw", "--force", "enable"],
    &["ufw", "disable"],
    &["firewall-cmd"],
    &["firewall-cmd", "--state"],
    &["firewall-cmd", "--reload"],
    &["iptables"],
    &["iptables", "-L", "-n", "--line-numbers"],
    &["iptables", "-F"],
    &["systemctl", "start", "firewalld"],
    &["systemctl", "stop", "firewalld"],
];

fn ufw_roundtrip(args: &[&str]) -> bool {
    let (op, rest) = match args {
        ["delete", rest @ ..] => (RuleOp::Delete, rest),
        rest => (RuleOp::Add, rest),
    };
    let [action, dir, rest @ ..] = rest else {
        return false;
    };
    if !matches!(*action, "allow" | "deny" | "reject") || !matches!(*dir, "in" | "out") {
        return false;
    }
    let (protocol, rest) = match rest {
        ["proto", p, r @ ..] => (*p, r),
        r => ("any", r),
    };
    let ["from", from, "to", to, rest @ ..] = rest else {
        return false;
    };
    let port = match rest {
        [] => None,
        ["port", p] => Some(*p),
        _ => return false,
    };
    let remote = if *dir == "in" { *from } else { *to };
    let remote = (remote != "any").then_some(remote);
    let rule = candidate(protocol, port, remote, action, dir);
    ufw_args(&rule, op).is_ok_and(|v| v == args)
}

/// 解析 `firewalld_rich_rules` 的输出形态；不认识的写法返回 `None`。
fn rich_candidate(rich: &str) -> Option<FirewallRule> {
    fn quoted<'a>(s: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
        s.strip_prefix(prefix)?.split_once('"')
    }
    let mut s = rich.strip_prefix("rule")?;
    let mut remote = None;
    if let Some((_family, r)) = quoted(s, " family=\"") {
        let (addr, r) = quoted(r, " source address=\"")?;
        remote = Some(addr);
        s = r;
    }
    let (protocol, port, s) = if let Some((port, r)) = quoted(s, " port port=\"") {
        let (proto, r) = quoted(r, " protocol=\"")?;
        (proto, Some(port), r)
    } else if let Some((value, r)) = quoted(s, " protocol value=\"") {
        let proto = match value {
            "tcp" | "udp" => value,
            "icmp" | "ipv6-icmp" => "icmp",
            _ => return None,
        };
        (proto, None, r)
    } else {
        ("any", None, s)
    };
    let action = match s {
        " accept" => "allow",
        " drop" => "deny",
        " reject" => "reject",
        _ => return None,
    };
    Some(candidate(protocol, port, remote, action, "in"))
}

fn firewalld_roundtrip(args: &[&str]) -> bool {
    let ["--permanent", flag] = args else {
        return false;
    };
    if let Some(rich) = flag
        .strip_prefix("--add-rich-rule=")
        .or_else(|| flag.strip_prefix("--remove-rich-rule="))
    {
        return rich_candidate(rich)
            .and_then(|rule| firewalld_rich_rules(&rule).ok())
            .is_some_and(|rules| rules.iter().any(|r| r == rich));
    }
    // S-13 之前的旧形态放行规则，只在删除时出现：`--remove-port=<单端口或段>/<tcp|udp>`
    if let Some(spec) = flag.strip_prefix("--remove-port=") {
        let Some((port, proto)) = spec.split_once('/') else {
            return false;
        };
        return matches!(proto, "tcp" | "udp")
            && matches!(parse_ports(port).as_deref(), Ok([item]) if item.render('-') == port);
    }
    false
}

fn iptables_roundtrip(program: &str, args: &[&str]) -> bool {
    let [op, chain, rest @ ..] = args else {
        return false;
    };
    let mut rest = rest;
    let op = match *op {
        "-A" => RuleOp::Add,
        "-D" => RuleOp::Delete,
        _ => return false,
    };
    let direction = match *chain {
        "INPUT" => "in",
        "OUTPUT" => "out",
        _ => return false,
    };
    let mut remote = None;
    if let ["-s" | "-d", addr, r @ ..] = rest {
        remote = Some(*addr);
        rest = r;
    }
    let mut protocol = "any";
    if let ["-p", p, r @ ..] = rest {
        protocol = match *p {
            "tcp" | "udp" => *p,
            "icmp" | "ipv6-icmp" => "icmp",
            _ => return false,
        };
        rest = r;
    }
    let mut port = None;
    match rest {
        ["--dport", p, r @ ..] | ["-m", "multiport", "--dports", p, r @ ..] => {
            port = Some(*p);
            rest = r;
        }
        _ => {}
    }
    let action = match rest {
        ["-j", "ACCEPT"] => "allow",
        ["-j", "DROP"] => "deny",
        ["-j", "REJECT"] => "reject",
        _ => return false,
    };
    let rule = candidate(protocol, port, remote, action, direction);
    iptables_command(&rule, op).is_ok_and(|(prog, v)| prog == program && v == args)
}

/// 该 argv（含程序名）是否为防火墙适配器会发出的命令：固定命令，或某条合法规则的规范渲染。
pub fn firewall_command_allowed(argv: &[&str]) -> bool {
    if FIREWALL_FIXED_COMMANDS.contains(&argv) {
        return true;
    }
    match argv {
        ["ufw", args @ ..] => ufw_roundtrip(args),
        ["firewall-cmd", args @ ..] => firewalld_roundtrip(args),
        [program @ ("iptables" | "ip6tables"), args @ ..] => iptables_roundtrip(program, args),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn rule(
        protocol: &str,
        port: Option<&str>,
        source: &str,
        action: &str,
        dir: &str,
    ) -> FirewallRule {
        FirewallRule {
            id: 1,
            name: "r".into(),
            description: None,
            protocol: protocol.into(),
            port: port.map(Into::into),
            source: Some(source.into()),
            destination: None,
            action: action.into(),
            enabled: true,
            priority: 50,
            direction: dir.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn ufw_renders_full_syntax_with_ranges_direction_and_delete() {
        let r = rule("tcp", Some("80,443,8000-9000"), "0.0.0.0/0", "allow", "in");
        assert_eq!(
            ufw_args(&r, RuleOp::Add).unwrap(),
            s(&[
                "allow",
                "in",
                "proto",
                "tcp",
                "from",
                "any",
                "to",
                "any",
                "port",
                "80,443,8000:9000"
            ])
        );
        let r = rule("udp", Some("53"), "10.0.0.0/8", "deny", "out");
        assert_eq!(
            ufw_args(&r, RuleOp::Delete).unwrap(),
            s(&[
                "delete",
                "deny",
                "out",
                "proto",
                "udp",
                "from",
                "any",
                "to",
                "10.0.0.0/8",
                "port",
                "53"
            ])
        );
        let r = rule("any", None, "192.168.1.5", "reject", "in");
        assert_eq!(
            ufw_args(&r, RuleOp::Add).unwrap(),
            s(&["reject", "in", "from", "192.168.1.5", "to", "any"])
        );
        let icmp = rule("icmp", None, "0.0.0.0/0", "allow", "in");
        assert!(matches!(
            ufw_args(&icmp, RuleOp::Add),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn firewalld_emits_one_rich_rule_per_port_with_family_and_action() {
        let r = rule("tcp", Some("22,8000-9000"), "203.0.113.0/24", "allow", "in");
        assert_eq!(
            firewalld_rich_rules(&r).unwrap(),
            s(&[
                r#"rule family="ipv4" source address="203.0.113.0/24" port port="22" protocol="tcp" accept"#,
                r#"rule family="ipv4" source address="203.0.113.0/24" port port="8000-9000" protocol="tcp" accept"#,
            ])
        );
        let r = rule("udp", Some("53"), "any", "deny", "in");
        assert_eq!(
            firewalld_rich_rules(&r).unwrap(),
            s(&[r#"rule port port="53" protocol="udp" drop"#])
        );
        let r = rule("icmp", None, "2001:db8::/32", "reject", "in");
        assert_eq!(
            firewalld_rich_rules(&r).unwrap(),
            s(&[
                r#"rule family="ipv6" source address="2001:db8::/32" protocol value="ipv6-icmp" reject"#
            ])
        );
        let out = rule("tcp", Some("443"), "any", "allow", "out");
        assert!(matches!(
            firewalld_rich_rules(&out),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn iptables_uses_multiport_remote_by_direction_and_ip6tables() {
        let r = rule("tcp", Some("8000-9000"), "10.0.0.0/8", "allow", "in");
        assert_eq!(
            iptables_command(&r, RuleOp::Add).unwrap(),
            (
                "iptables",
                s(&[
                    "-A",
                    "INPUT",
                    "-s",
                    "10.0.0.0/8",
                    "-p",
                    "tcp",
                    "--dport",
                    "8000:9000",
                    "-j",
                    "ACCEPT"
                ])
            )
        );
        let r = rule("udp", Some("53,123"), "2001:db8::1", "deny", "out");
        assert_eq!(
            iptables_command(&r, RuleOp::Delete).unwrap(),
            (
                "ip6tables",
                s(&[
                    "-D",
                    "OUTPUT",
                    "-d",
                    "2001:db8::1",
                    "-p",
                    "udp",
                    "-m",
                    "multiport",
                    "--dports",
                    "53,123",
                    "-j",
                    "DROP"
                ])
            )
        );
        let r = rule("icmp", None, "0.0.0.0/0", "reject", "in");
        assert_eq!(
            iptables_command(&r, RuleOp::Add).unwrap(),
            (
                "iptables",
                s(&["-A", "INPUT", "-p", "icmp", "-j", "REJECT"])
            )
        );
    }

    #[test]
    fn legacy_invalid_rules_are_rejected_before_reaching_the_shell() {
        for r in [
            rule("tcp", Some("80; rm -rf /"), "any", "allow", "in"),
            rule("tcp", Some("80"), "1.2.3.4\" accept", "allow", "in"),
            rule("any", Some("80"), "any", "allow", "in"),
            rule("sctp", None, "any", "allow", "in"),
        ] {
            let err = firewalld_rich_rules(&r).unwrap_err();
            assert!(
                matches!(err, AppError::BadRequest(ref m) if m.contains("edit and save it again")),
                "{err:?}"
            );
            assert!(ufw_args(&r, RuleOp::Add).is_err());
            assert!(iptables_command(&r, RuleOp::Add).is_err());
        }
    }

    // ── 特权助手往返校验 ──

    fn allowed(program: &str, args: &[String]) -> bool {
        let mut argv = vec![program];
        argv.extend(args.iter().map(String::as_str));
        firewall_command_allowed(&argv)
    }

    fn sample_rules() -> Vec<FirewallRule> {
        let mut out = Vec::new();
        for (proto, port) in [
            ("tcp", Some("22")),
            ("tcp", Some("80,443,8000-9000")),
            ("udp", Some("53")),
            ("any", None),
            ("icmp", None),
            ("tcp", None),
        ] {
            for source in [
                None,
                Some("10.0.0.0/8"),
                Some("192.168.1.5"),
                Some("2001:db8::/32"),
            ] {
                for action in ["allow", "deny", "reject"] {
                    for direction in ["in", "out"] {
                        out.push(candidate(proto, port, source, action, direction));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn every_rendered_command_passes_the_helper_roundtrip() {
        let mut checked = 0;
        for r in sample_rules() {
            for op in [RuleOp::Add, RuleOp::Delete] {
                if let Ok(args) = ufw_args(&r, op) {
                    assert!(allowed("ufw", &args), "ufw {args:?}");
                    checked += 1;
                }
                if let Ok((prog, args)) = iptables_command(&r, op) {
                    assert!(allowed(prog, &args), "{prog} {args:?}");
                    checked += 1;
                }
            }
            if let Ok(rules) = firewalld_rich_rules(&r) {
                for rich in rules {
                    for flag in ["--add-rich-rule=", "--remove-rich-rule="] {
                        let args = vec!["--permanent".to_string(), format!("{flag}{rich}")];
                        assert!(allowed("firewall-cmd", &args), "{args:?}");
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 500, "{checked}");
        for fixed in FIREWALL_FIXED_COMMANDS {
            assert!(firewall_command_allowed(fixed));
        }
        assert!(firewall_command_allowed(&[
            "firewall-cmd",
            "--permanent",
            "--remove-port=8000-9000/tcp"
        ]));
    }

    #[test]
    fn tampered_or_foreign_firewall_commands_are_refused() {
        for argv in [
            // 追加参数 / 注释 / 非规范写法
            &[
                "ufw", "allow", "in", "proto", "tcp", "from", "any", "to", "any", "port", "22",
                "comment", "x",
            ][..],
            &[
                "ufw", "allow", "in", "proto", "tcp", "from", "any", "to", "any", "port", "022",
            ],
            &[
                "ufw", "allow", "in", "proto", "tcp", "from", "any", "to", "10.0.0.1", "port", "22",
            ],
            &[
                "ufw", "allow", "in", "proto", "tcp", "from", "any", "to", "any", "port", "22;id",
            ],
            &["ufw", "allow", "22"],
            &["ufw", "reset"],
            &["ufw", "--force", "reset"],
            &["ufw", "default", "allow"],
            &[
                "ufw", "allow", "in", "proto", "icmp", "from", "any", "to", "any",
            ],
            // firewalld：直通规则 / 任意区域 / 注入属性
            &[
                "firewall-cmd",
                "--permanent",
                "--add-rich-rule=rule port port=\"22\" protocol=\"tcp\" accept log",
            ],
            &[
                "firewall-cmd",
                "--permanent",
                "--add-rich-rule=rule family=\"ipv4\" source address=\"0.0.0.0/0\" accept",
            ],
            &[
                "firewall-cmd",
                "--permanent",
                "--add-rich-rule=rule family=\"ipv6\" source address=\"10.0.0.1\" accept",
            ],
            &[
                "firewall-cmd",
                "--direct",
                "--add-rule",
                "ipv4",
                "filter",
                "INPUT",
                "0",
                "-j",
                "ACCEPT",
            ],
            &["firewall-cmd", "--permanent", "--add-port=22/tcp"],
            &["firewall-cmd", "--permanent", "--remove-port=22/sctp"],
            &["firewall-cmd", "--permanent", "--remove-port=22,80/tcp"],
            &["firewall-cmd", "--panic-on"],
            &["firewall-cmd", "--set-default-zone=trusted"],
            // iptables：其它链 / 表 / 目标 / 插入位置
            &["iptables", "-A", "FORWARD", "-j", "ACCEPT"],
            &["iptables", "-t", "nat", "-A", "PREROUTING", "-j", "DNAT"],
            &["iptables", "-I", "INPUT", "1", "-j", "ACCEPT"],
            &["iptables", "-A", "INPUT", "-j", "LOG"],
            &[
                "iptables",
                "-A",
                "INPUT",
                "-s",
                "2001:db8::1",
                "-j",
                "ACCEPT",
            ],
            &["ip6tables", "-A", "INPUT", "-s", "10.0.0.1", "-j", "ACCEPT"],
            &[
                "iptables", "-A", "INPUT", "-p", "tcp", "--dport", "22", "-j", "ACCEPT", "-m",
                "comment",
            ],
            &["iptables", "-P", "INPUT", "ACCEPT"],
            &["iptables", "-X"],
            &["systemctl", "disable", "firewalld"],
            &["nft", "flush", "ruleset"],
        ] {
            assert!(!firewall_command_allowed(argv), "{argv:?}");
        }
    }
}
