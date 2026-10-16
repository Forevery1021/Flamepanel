//! WAF → Apache 2.4 原生配置编译（Doc/30 v1.1 · 分发见 [`super::waf_engines`]）。
//!
//! 只用 Apache 核心与发行版默认启用的模块（`mod_setenvif` / `mod_authz_core` /
//! `mod_log_config`），不依赖 ModSecurity：
//!
//! - `vhost.conf`（站点 `<VirtualHost>` 内 `IncludeOptional …/vhost*.conf`）：
//!   每条规则一行 `SetEnvIfExpr "<ap_expr>" fp_waf_rN=1`（post-read-request 阶段，
//!   即伪静态重写之前；路径规则对 `unescape(%{REQUEST_URI})` 匹配）→ 汇总 `fp_waf_wl` / `fp_waf_block` / `fp_waf_hit` →
//!   `<If "-n reqenv('fp_waf_block')"> Require all denied </If>`（403；`<If>` 最后合并，
//!   覆盖站点 `<Directory>` 的 `Require all granted`）。
//! - `http.conf`（主配置末尾 `IncludeOptional …/http*.conf`）：`LogFormat fp_waf`（JSON）+
//!   `GlobalLog <log> fp_waf env=fp_waf_hit`（`GlobalLog` 不会替换各虚拟主机自身的日志）。
//!
//! 安全：规则字面量一律转成 `\xHH` 后放进 `/…/` 正则，IP 经解析校验后才进入 `-R '…'`，
//! 无法突破 ap_expr 或配置语法。CC 规则不下发（Apache 无原生按客户端限频）。

use super::waf_engines::{
    normalize, normalize_host, parse_rule_tags, re_literal, re_signatures, Cond, Kind, NormRule,
    WafHit, HEADER_NOTE,
};
use super::waf_nginx::{CompiledWaf, WafSite};
use crate::domain::entity::WafRule;

fn header() -> String {
    format!("# Managed by Flamepanel WAF (webserver::waf_apache). {HEADER_NOTE}\n")
}

/// 单条规则的 ap_expr（作用域 AND 匹配条件）。
fn rule_expr(r: &NormRule) -> String {
    let cond = match &r.cond {
        Cond::Ips(items) => {
            let parts: Vec<String> = items.iter().map(|ip| format!("-R '{ip}'")).collect();
            format!("({})", parts.join(" || "))
        }
        Cond::Ua(s) => format!("%{{HTTP_USER_AGENT}} =~ /{}/i", re_literal(s)),
        // post-read-request 阶段 REQUEST_URI 尚未解码：显式 unescape，对齐 Nginx 解码后的 `$uri`
        // （否则 `/%61dmin` 可绕过 `/admin`）
        Cond::Path(s) => format!("unescape(%{{REQUEST_URI}}) =~ /^{}/", re_literal(s)),
        // 请求行（方法 + 原始 URI + 协议），与 Nginx `$request_method $request_uri` 对齐
        Cond::Sig(sigs) => format!("%{{THE_REQUEST}} =~ /{}/i", re_signatures(sigs)),
    };
    match &r.scope {
        None => cond,
        Some(domains) => {
            let alts: Vec<String> = domains.iter().map(|d| re_literal(d)).collect();
            format!(
                "%{{HTTP_HOST}} =~ /^(?:{})(?::[0-9]+)?$/i && ({cond})",
                alts.join("|")
            )
        }
    }
}

fn any_of(vars: &[String]) -> String {
    vars.iter()
        .map(|v| format!("-n reqenv('{v}')"))
        .collect::<Vec<_>>()
        .join(" || ")
}

/// 编译。`log_path` 为该发行版的命中日志路径。
pub fn compile(rules: &[WafRule], sites: &[WafSite], log_path: &str) -> CompiledWaf {
    let (norm, skipped) = normalize(rules, sites, "Apache", true);
    let mut vhost = header();
    let mut block_vars = Vec::new();
    let mut warn_vars = Vec::new();
    let mut tags = Vec::new();
    for r in &norm {
        let var = format!("fp_waf_r{}", r.id);
        let extra = if r.kind == Kind::Whitelist {
            " fp_waf_wl=1"
        } else {
            ""
        };
        vhost.push_str(&r.comment());
        vhost.push('\n');
        vhost.push_str(&format!(
            "SetEnvIfExpr \"{}\" {var}=1{extra}\n",
            rule_expr(r)
        ));
        tags.push(format!("r{}:%{{{var}}}e", r.id));
        match r.kind {
            Kind::Whitelist => {}
            Kind::Block => block_vars.push(var),
            Kind::Warn => warn_vars.push(var),
        }
    }
    vhost.push_str("# summary (whitelist wins)\n");
    if !block_vars.is_empty() {
        vhost.push_str(&format!(
            "SetEnvIfExpr \"-z reqenv('fp_waf_wl') && ({})\" fp_waf_block=1 fp_waf_hit=1\n",
            any_of(&block_vars)
        ));
    }
    if !warn_vars.is_empty() {
        vhost.push_str(&format!(
            "SetEnvIfExpr \"-z reqenv('fp_waf_wl') && ({})\" fp_waf_hit=1\n",
            any_of(&warn_vars)
        ));
    }
    if !block_vars.is_empty() {
        vhost.push_str("<If \"-n reqenv('fp_waf_block')\">\n    Require all denied\n</If>\n");
    }

    // 命中日志：Apache 对 %{…}i 中的 `"` `\` 做转义，不可见字符写成 `\xHH`（解析时兼容）
    let mut http = header();
    http.push_str("# hit log (log + block actions)\n");
    http.push_str(&format!(
        "LogFormat \"{{\\\"time\\\":\\\"%{{%Y-%m-%dT%H:%M:%S%z}}t\\\",\\\"host\\\":\\\"%{{Host}}i\\\",\\\"ip\\\":\\\"%a\\\",\\\"method\\\":\\\"%m\\\",\\\"uri\\\":\\\"%U%q\\\",\\\"ua\\\":\\\"%{{User-Agent}}i\\\",\\\"status\\\":\\\"%>s\\\",\\\"block\\\":\\\"%{{fp_waf_block}}e\\\",\\\"rules\\\":\\\"{}\\\"}}\" fp_waf\n",
        tags.join(" ")
    ));
    http.push_str(&format!("GlobalLog {log_path} fp_waf env=fp_waf_hit\n"));

    CompiledWaf {
        http_conf: http,
        server_conf: vhost,
        enforced: norm.iter().map(|r| r.id).collect(),
        skipped,
    }
}

/// Apache 日志转义 `\xHH` → JSON `\u00HH`（其余 `\"` `\\` 已是合法 JSON 转义）。
fn apache_to_json(line: &str) -> String {
    let b = line.as_bytes();
    let mut out = String::with_capacity(line.len() + 8);
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            if b[i + 1] == b'x'
                && i + 3 < b.len()
                && b[i + 2].is_ascii_hexdigit()
                && b[i + 3].is_ascii_hexdigit()
            {
                out.push_str("\\u00");
                out.push(b[i + 2] as char);
                out.push(b[i + 3] as char);
                i += 4;
                continue;
            }
            // 保留完整转义对（`\\`、`\"` 等），避免把 `\\x41` 的第二个反斜杠误判
            out.push('\\');
            out.push(b[i + 1] as char);
            i += 2;
            continue;
        }
        // 多字节 UTF-8：按字符推进
        let ch = line[i..].chars().next().unwrap_or('\u{FFFD}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[derive(serde::Deserialize)]
struct RawHit {
    #[serde(default)]
    time: String,
    #[serde(default)]
    host: String,
    #[serde(default)]
    ip: String,
    #[serde(default)]
    method: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    ua: String,
    #[serde(default)]
    block: String,
    #[serde(default)]
    rules: String,
}

/// 解析 `LogFormat fp_waf` 一行；非本格式返回 `None`。
pub fn parse_hit_line(line: &str) -> Option<WafHit> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let raw: RawHit = serde_json::from_str(&apache_to_json(line)).ok()?;
    let rules = parse_rule_tags(&raw.rules);
    if rules.is_empty() {
        return None;
    }
    let dash = |s: String| if s == "-" { String::new() } else { s };
    Some(WafHit {
        time: chrono::DateTime::parse_from_str(&raw.time, "%Y-%m-%dT%H:%M:%S%z")
            .ok()
            .map(|t| t.with_timezone(&chrono::Utc)),
        host: normalize_host(&raw.host),
        client_ip: raw.ip,
        method: raw.method,
        uri: raw.uri,
        user_agent: dash(raw.ua),
        blocked: raw.block == "1",
        rules,
        cc_rules: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::waf_engines::tests::rule;

    fn site(id: i64, d: &str) -> WafSite {
        WafSite {
            website_id: id,
            domains: vec![d.into()],
        }
    }

    #[test]
    fn compiles_setenvif_summary_and_global_log() {
        let rules = vec![
            rule(1, "ip_blacklist", "10.9.0.0/16, 127.0.0.2", "block", 0),
            rule(2, "ua_blacklist", "BadBot/", "block", 0),
            rule(3, "path_block", "/admin", "block", 7),
            rule(4, "sql_injection", "", "block", 0),
            rule(5, "ip_whitelist", "127.0.0.3", "block", 0),
            rule(6, "xss", "", "log", 0),
            rule(7, "cc", "10/60", "block", 0),
        ];
        let c = compile(
            &rules,
            &[site(7, "T1.test")],
            "/var/log/apache2/flamepanel-waf.log",
        );
        let v = &c.server_conf;
        assert!(
            v.contains("SetEnvIfExpr \"(-R '10.9.0.0/16' || -R '127.0.0.2/32')\" fp_waf_r1=1\n")
        );
        assert!(v.contains("SetEnvIfExpr \"%{HTTP_USER_AGENT} =~ /BadBot\\x2f/i\" fp_waf_r2=1\n"));
        assert!(v.contains(
            "SetEnvIfExpr \"%{HTTP_HOST} =~ /^(?:t1\\x2etest)(?::[0-9]+)?$/i && (unescape(%{REQUEST_URI}) =~ /^\\x2fadmin/)\" fp_waf_r3=1\n"
        ));
        assert!(v.contains("%{THE_REQUEST} =~ /(?:"));
        assert!(v.contains("union(?:\\x20|\\x2b|%20)select"));
        assert!(v.contains("SetEnvIfExpr \"(-R '127.0.0.3/32')\" fp_waf_r5=1 fp_waf_wl=1\n"));
        assert!(v.contains("SetEnvIfExpr \"-z reqenv('fp_waf_wl') && (-n reqenv('fp_waf_r1') || -n reqenv('fp_waf_r2') || -n reqenv('fp_waf_r3') || -n reqenv('fp_waf_r4'))\" fp_waf_block=1 fp_waf_hit=1\n"));
        assert!(v.contains(
            "SetEnvIfExpr \"-z reqenv('fp_waf_wl') && (-n reqenv('fp_waf_r6'))\" fp_waf_hit=1\n"
        ));
        assert!(v.contains("<If \"-n reqenv('fp_waf_block')\">\n    Require all denied\n</If>\n"));
        assert_eq!(c.enforced, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(c.skipped.len(), 1);
        assert_eq!(c.skipped[0].0, 7);

        let h = &c.http_conf;
        assert!(h.contains("\\\"rules\\\":\\\"r1:%{fp_waf_r1}e r2:%{fp_waf_r2}e r3:%{fp_waf_r3}e r4:%{fp_waf_r4}e r5:%{fp_waf_r5}e r6:%{fp_waf_r6}e\\\"}\" fp_waf\n"));
        assert!(
            h.ends_with("GlobalLog /var/log/apache2/flamepanel-waf.log fp_waf env=fp_waf_hit\n")
        );
    }

    #[test]
    fn hostile_literals_cannot_break_out() {
        let rules = vec![
            rule(1, "ua_blacklist", "a\" fp_waf_wl=1 \n<If true>", "block", 0),
            rule(2, "path_block", "/x/ || true || /", "block", 0),
        ];
        let c = compile(&rules, &[], "/var/log/httpd/flamepanel-waf.log");
        for line in c
            .server_conf
            .lines()
            .filter(|l| l.starts_with("SetEnvIfExpr"))
        {
            // 每行恰好一对界定引号，规则字面量里没有裸 `"` `'` `/` 空格
            assert_eq!(line.matches('"').count(), 2, "{line}");
        }
        assert!(c
            .server_conf
            .contains("/a\\x22\\x20fp_waf_wl\\x3d1\\x20\\x0a\\x3cIf\\x20true\\x3e/i"));
        assert!(c
            .server_conf
            .contains("/^\\x2fx\\x2f\\x20\\x7c\\x7c\\x20true"));
        assert!(!c.server_conf.contains("\n<If true>"));
    }

    #[test]
    fn empty_ruleset_has_no_deny_block() {
        let c = compile(&[], &[], "/var/log/apache2/flamepanel-waf.log");
        assert!(!c.server_conf.contains("Require all denied"));
        assert!(!c.server_conf.contains("SetEnvIfExpr"));
        assert!(c.http_conf.contains("GlobalLog"));
    }

    #[test]
    fn parses_hit_lines_with_apache_escapes() {
        let line = r#"{"time":"2026-09-27T16:17:53+0800","host":"T1.test:8080","ip":"127.0.0.1","method":"GET","uri":"/admin/x?a=\"b","ua":"x\x01y\\x41","status":"403","block":"1","rules":"r1:- r2:- r3:1 r4:-"}"#;
        let h = parse_hit_line(line).expect("parse");
        assert_eq!(h.host, "t1.test");
        assert_eq!(h.rules, vec![3]);
        assert!(h.blocked);
        assert_eq!(h.uri, "/admin/x?a=\"b");
        assert_eq!(h.user_agent, "x\u{1}y\\x41");
        assert_eq!(
            h.time.map(|t| t.to_rfc3339()),
            Some("2026-09-27T08:17:53+00:00".into())
        );
        // 告警命中：block 为 `-`
        let warn = r#"{"time":"x","host":"a","ip":"1.1.1.1","method":"GET","uri":"/","ua":"-","status":"200","block":"-","rules":"r6:1"}"#;
        let h = parse_hit_line(warn).unwrap();
        assert!(!h.blocked && h.time.is_none() && h.user_agent.is_empty());
        // 无命中标签 / 非本格式
        assert!(parse_hit_line(r#"{"rules":"r1:-"}"#).is_none());
        assert!(parse_hit_line("127.0.0.1 - - [..] \"GET /\"").is_none());
        assert!(parse_hit_line("").is_none());
    }

    /// 真实 `apache2 -t` / `httpd -t` 校验编译产物（本机无 Apache 时跳过）。
    #[test]
    fn real_apache_accepts_compiled_config() {
        let bin = ["/usr/sbin/apache2", "/usr/sbin/httpd", "apache2", "httpd"]
            .into_iter()
            .find(|b| std::process::Command::new(b).arg("-v").output().is_ok());
        let modules = [
            "/usr/lib/apache2/modules",
            "/usr/lib64/httpd/modules",
            "/usr/lib/httpd/modules",
        ]
        .into_iter()
        .find(|d| std::path::Path::new(d).join("mod_setenvif.so").exists());
        let (Some(bin), Some(m)) = (bin, modules) else {
            eprintln!("apache not installed; skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fp-waf-apache-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("waf")).unwrap();
        let rules = vec![
            rule(1, "ip_blacklist", "1.2.3.4, 10.0.0.0/8", "block", 0),
            rule(2, "ip_whitelist", "127.0.0.1", "log", 0),
            rule(3, "ua_blacklist", "sqlmap\"; evil {", "block", 0),
            rule(4, "ua_whitelist", "GoodBot", "log", 7),
            rule(5, "path_block", "/wp-admin$ {x}\\ '/", "block", 7),
            rule(6, "sql_injection", "", "block", 0),
            rule(7, "xss", "", "log", 7),
        ];
        let d = dir.display().to_string();
        let log = format!("{d}/waf.log");
        let c = compile(&rules, &[site(7, "A.test")], &log);
        std::fs::write(dir.join("waf/http.conf"), &c.http_conf).unwrap();
        std::fs::write(dir.join("waf/vhost.conf"), &c.server_conf).unwrap();
        let mut main = format!(
            "ServerRoot {d}\nPidFile {d}/httpd.pid\nErrorLog {d}/error.log\nListen 127.0.0.1:18090\n"
        );
        for (name, file) in [
            ("mpm_event_module", "mod_mpm_event.so"),
            ("authz_core_module", "mod_authz_core.so"),
            ("setenvif_module", "mod_setenvif.so"),
            ("log_config_module", "mod_log_config.so"),
        ] {
            main.push_str(&format!(
                "<IfModule !{name}>\nLoadModule {name} {m}/{file}\n</IfModule>\n"
            ));
        }
        main.push_str(&format!(
            "IncludeOptional {d}/waf/http*.conf\n<VirtualHost 127.0.0.1:18090>\n    ServerName a.test\n    IncludeOptional {d}/waf/vhost*.conf\n</VirtualHost>\n"
        ));
        std::fs::write(dir.join("httpd.conf"), main).unwrap();
        let out = std::process::Command::new(bin)
            .arg("-t")
            .arg("-f")
            .arg(dir.join("httpd.conf"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.status.success(), "{bin} -t failed:\n{stderr}");
    }
}
