//! WAF → Caddy 2 原生 Caddyfile 片段（Doc/30 v1.1 · 分发见 [`super::waf_engines`]）。
//!
//! 只用标准发行版 Caddy 自带的指令（不需要 xcaddy 插件），Caddy ≥ 2.6 验证：
//!
//! - 站点块内 `import /etc/caddy/flamepanel/waf/site*.caddy`（glob 无匹配不报错）。
//! - 每条规则：命名匹配器 `@fp_waf_rN`（`host` 作用域 + `remote_ip` / `header_regexp` /
//!   `path_regexp` / `expression`）→ `vars @fp_waf_rN { fp_waf_rN 1; fp_waf_block|hit|wl 1 }`。
//!   `vars` 在指令顺序里早于 `rewrite` / `try_files`，因此匹配的是**原始请求**。
//! - 汇总：`@fp_waf_deny { vars fp_waf_block 1; not vars fp_waf_wl 1 }` → `respond 403`。
//! - 命中日志：`log { output file … ; format json }` + `skip_log`（未命中 / 白名单不记）；
//!   命中的规则经 `map … {http.auth.user.id}` 写进访问日志的 `user_id` 字段
//!   （`fpwaf b:1 r3:1 r5:`，惰性求值，记录日志时 `vars` 已确定）。站点若启用了
//!   `basicauth`，`user_id` 为真实用户名，该请求的规则标签丢失（拦截不受影响）。
//!
//! 安全：字面量转 `\xHH` 后放进反引号令牌（Caddyfile 反引号内不做任何转义）与 CEL 原始字符串
//! `r'…'`，不含反引号 / 单引号 / 花括号，无法突破语法。CC 不下发（限频需第三方插件）。

use super::waf_engines::{
    normalize, normalize_host, parse_rule_tags, re_literal, re_signatures, Cond, Kind, NormRule,
    WafHit, HEADER_NOTE,
};
use super::waf_nginx::{CompiledWaf, WafSite};
use crate::domain::entity::WafRule;
use std::collections::HashMap;

fn header() -> String {
    format!("# Managed by Flamepanel WAF (webserver::waf_caddy). {HEADER_NOTE}\n")
}

/// 匹配器主体（不含作用域）。
fn matcher_line(r: &NormRule) -> String {
    match &r.cond {
        Cond::Ips(items) => format!("remote_ip {}", items.join(" ")),
        Cond::Ua(s) => format!("header_regexp User-Agent `(?i){}`", re_literal(s)),
        Cond::Path(s) => format!("path_regexp `^{}`", re_literal(s)),
        // 原始请求 URI（路径 + 查询串，未解码）；特征容忍百分号编码
        Cond::Sig(sigs) => format!(
            "expression `{{http.request.uri}}.matches(r'(?i){}')`",
            re_signatures(sigs)
        ),
    }
}

/// 编译。`log_path` 为命中日志路径。
pub fn compile(rules: &[WafRule], sites: &[WafSite], log_path: &str) -> CompiledWaf {
    let (norm, skipped) = normalize(rules, sites, "Caddy", true);
    let mut out = header();
    if norm.is_empty() {
        return CompiledWaf {
            http_conf: String::new(),
            server_conf: out,
            enforced: Vec::new(),
            skipped,
        };
    }
    let mut tags = vec!["b:{http.vars.fp_waf_block}".to_string()];
    for r in &norm {
        let name = format!("fp_waf_r{}", r.id);
        out.push_str(&r.comment());
        out.push('\n');
        out.push_str(&format!("@{name} {{\n"));
        if let Some(domains) = &r.scope {
            // 域名来自面板网站表（已校验），仍逐个转义为安全令牌：只保留 DNS 合法字符
            let hosts: Vec<String> = domains
                .iter()
                .filter(|d| {
                    !d.is_empty()
                        && d.chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '*'))
                })
                .cloned()
                .collect();
            if hosts.is_empty() {
                // 域名全被过滤：宁可不下发也不能扩大为全局
                out.push_str("    not path *\n");
            } else {
                out.push_str(&format!("    host {}\n", hosts.join(" ")));
            }
        }
        out.push_str(&format!("    {}\n}}\n", matcher_line(r)));
        let flag = match r.kind {
            Kind::Whitelist => "fp_waf_wl",
            Kind::Block => "fp_waf_block",
            Kind::Warn => "fp_waf_hit",
        };
        out.push_str(&format!("vars @{name} {{\n    {name} 1\n    {flag} 1\n"));
        if r.kind == Kind::Block {
            out.push_str("    fp_waf_hit 1\n");
        }
        out.push_str("}\n");
        tags.push(format!("r{}:{{http.vars.{name}}}", r.id));
    }
    out.push_str(
        "# summary (whitelist wins)\n\
@fp_waf_deny {\n    vars fp_waf_block 1\n    not vars fp_waf_wl 1\n}\n\
respond @fp_waf_deny 403\n\
# hit log (log + block actions); rule tags travel in the access log `user_id` field\n\
@fp_waf_quiet not vars fp_waf_hit 1\n\
skip_log @fp_waf_quiet\n\
@fp_waf_allowed vars fp_waf_wl 1\n\
skip_log @fp_waf_allowed\n",
    );
    out.push_str(&format!(
        "map {{http.request.method}} {{http.auth.user.id}} {{\n    default \"fpwaf {}\"\n}}\n",
        tags.join(" ")
    ));
    out.push_str(&format!(
        "log {{\n    output file {log_path}\n    format json\n}}\n"
    ));
    CompiledWaf {
        http_conf: String::new(),
        server_conf: out,
        enforced: norm.iter().map(|r| r.id).collect(),
        skipped,
    }
}

#[derive(serde::Deserialize, Default)]
struct RawRequest {
    #[serde(default)]
    remote_ip: String,
    #[serde(default)]
    client_ip: String,
    #[serde(default)]
    method: String,
    #[serde(default)]
    host: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    headers: HashMap<String, Vec<String>>,
}

#[derive(serde::Deserialize)]
struct RawLine {
    #[serde(default)]
    ts: f64,
    #[serde(default)]
    request: RawRequest,
    #[serde(default)]
    user_id: String,
}

/// 解析 Caddy JSON 访问日志一行（仅带 `fpwaf` 标签的命中记录）。
pub fn parse_hit_line(line: &str) -> Option<WafHit> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let raw: RawLine = serde_json::from_str(line).ok()?;
    let tags = raw.user_id.strip_prefix("fpwaf ")?;
    let rules = parse_rule_tags(tags);
    if rules.is_empty() {
        return None;
    }
    let blocked = tags.split_whitespace().any(|t| t == "b:1");
    let req = raw.request;
    let ua = req
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("user-agent"))
        .and_then(|(_, v)| v.first().cloned())
        .unwrap_or_default();
    let time = if raw.ts > 0.0 {
        let secs = raw.ts.trunc() as i64;
        let nanos = ((raw.ts.fract()) * 1e9) as u32;
        chrono::DateTime::from_timestamp(secs, nanos)
    } else {
        None
    };
    Some(WafHit {
        time,
        host: normalize_host(&req.host),
        // Caddy ≥ 2.7 记录经 trusted_proxies 解析的 client_ip；否则用直连地址
        client_ip: if req.client_ip.is_empty() {
            req.remote_ip
        } else {
            req.client_ip
        },
        method: req.method,
        uri: req.uri,
        user_agent: ua,
        blocked,
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
    fn compiles_matchers_vars_and_log() {
        let rules = vec![
            rule(1, "ip_blacklist", "10.9.0.0/16,127.0.0.2", "block", 0),
            rule(2, "ua_blacklist", "BadBot/", "block", 0),
            rule(3, "path_block", "/admin", "block", 7),
            rule(4, "xss", "", "log", 0),
            rule(5, "ip_whitelist", "127.0.0.3", "block", 0),
            rule(6, "cc", "", "block", 0),
        ];
        let c = compile(
            &rules,
            &[site(7, "t1.test")],
            "/var/log/caddy/flamepanel-waf.log",
        );
        let s = &c.server_conf;
        assert!(c.http_conf.is_empty());
        assert!(s.contains("@fp_waf_r1 {\n    remote_ip 10.9.0.0/16 127.0.0.2/32\n}\nvars @fp_waf_r1 {\n    fp_waf_r1 1\n    fp_waf_block 1\n    fp_waf_hit 1\n}\n"));
        assert!(s.contains("@fp_waf_r2 {\n    header_regexp User-Agent `(?i)BadBot\\x2f`\n}\n"));
        assert!(s.contains("@fp_waf_r3 {\n    host t1.test\n    path_regexp `^\\x2fadmin`\n}\n"));
        assert!(s.contains("expression `{http.request.uri}.matches(r'(?i)(?:(?:\\x3c|%3c)script"));
        assert!(s.contains("vars @fp_waf_r4 {\n    fp_waf_r4 1\n    fp_waf_hit 1\n}\n"));
        assert!(s.contains("vars @fp_waf_r5 {\n    fp_waf_r5 1\n    fp_waf_wl 1\n}\n"));
        assert!(s.contains("respond @fp_waf_deny 403\n"));
        assert!(s.contains("default \"fpwaf b:{http.vars.fp_waf_block} r1:{http.vars.fp_waf_r1} r2:{http.vars.fp_waf_r2} r3:{http.vars.fp_waf_r3} r4:{http.vars.fp_waf_r4} r5:{http.vars.fp_waf_r5}\"\n"));
        assert!(s.contains("output file /var/log/caddy/flamepanel-waf.log\n"));
        assert_eq!(c.enforced, vec![1, 2, 3, 4, 5]);
        assert_eq!(c.skipped.iter().map(|s| s.0).collect::<Vec<_>>(), vec![6]);
    }

    #[test]
    fn hostile_literals_stay_inside_tokens() {
        let rules = vec![
            rule(1, "ua_blacklist", "`}\nrespond 200 {'", "block", 0),
            rule(2, "path_block", "/a` b {", "block", 0),
        ];
        let c = compile(&rules, &[], "/var/log/caddy/flamepanel-waf.log");
        assert!(!c.server_conf.contains("respond 200"));
        for line in c.server_conf.lines().filter(|l| l.contains("_regexp")) {
            assert_eq!(line.matches('`').count(), 2, "{line}");
            assert!(!line.contains('\''), "{line}");
        }
    }

    #[test]
    fn empty_ruleset_is_header_only() {
        let c = compile(&[], &[], "/var/log/caddy/flamepanel-waf.log");
        assert!(c.server_conf.lines().all(|l| l.starts_with('#')));
    }

    #[test]
    fn parses_caddy_access_log() {
        // Caddy 2.6.2 实测输出（节选）
        let line = r#"{"level":"error","ts":1790525922.7891815,"logger":"http.log.access.log0","msg":"handled request","request":{"remote_ip":"127.0.0.1","remote_port":"43704","proto":"HTTP/1.1","method":"GET","host":"T1.test:443","uri":"/admin/x?a=\"b\\","headers":{"User-Agent":["curl/8.14.1"],"Accept":["*/*"]}},"user_id":"fpwaf b:1 r1: r3:1","duration":0.0001,"size":0,"status":403}"#;
        let h = parse_hit_line(line).expect("parse");
        assert_eq!(h.host, "t1.test");
        assert_eq!(h.client_ip, "127.0.0.1");
        assert_eq!(h.rules, vec![3]);
        assert!(h.blocked);
        assert_eq!(h.user_agent, "curl/8.14.1");
        assert_eq!(h.uri, "/admin/x?a=\"b\\");
        assert_eq!(h.time.map(|t| t.timestamp()), Some(1790525922));
        // 告警命中 + client_ip 优先
        let warn = r#"{"ts":1,"request":{"remote_ip":"10.0.0.1","client_ip":"1.2.3.4","host":"a","uri":"/"},"user_id":"fpwaf b: r4:1"}"#;
        let h = parse_hit_line(warn).unwrap();
        assert!(!h.blocked);
        assert_eq!(h.client_ip, "1.2.3.4");
        // 非 WAF 记录（普通访问日志 / basicauth 用户名）
        assert!(parse_hit_line(r#"{"ts":1,"request":{},"user_id":""}"#).is_none());
        assert!(parse_hit_line(r#"{"ts":1,"request":{},"user_id":"alice"}"#).is_none());
        assert!(parse_hit_line(r#"{"ts":1,"request":{},"user_id":"fpwaf b: r1:"}"#).is_none());
    }

    /// 真实 `caddy validate` 校验编译产物（本机无 Caddy 时跳过）。
    #[test]
    fn real_caddy_validates_compiled_snippet() {
        if std::process::Command::new("caddy")
            .arg("version")
            .output()
            .is_err()
        {
            eprintln!("caddy not installed; skipping");
            return;
        }
        let dir = std::env::temp_dir().join(format!("fp-waf-caddy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rules = vec![
            rule(1, "ip_blacklist", "1.2.3.4, 10.0.0.0/8", "block", 0),
            rule(2, "ip_whitelist", "127.0.0.1", "log", 0),
            rule(3, "ua_blacklist", "sqlmap\"; evil {`", "block", 0),
            rule(4, "ua_whitelist", "GoodBot", "log", 7),
            rule(5, "path_block", "/wp-admin$ {x}\\ '", "block", 7),
            rule(6, "sql_injection", "", "block", 0),
            rule(7, "xss", "", "log", 7),
        ];
        let d = dir.display().to_string();
        let c = compile(&rules, &[site(7, "a.test")], &format!("{d}/waf.log"));
        std::fs::write(dir.join("site.caddy"), &c.server_conf).unwrap();
        let main = format!(
            "{{\n    admin off\n    auto_https off\n}}\nhttp://a.test:18091 {{\n    root * {d}\n    import {d}/site*.caddy\n    file_server\n}}\n"
        );
        std::fs::write(dir.join("Caddyfile"), main).unwrap();
        let out = std::process::Command::new("caddy")
            .args(["validate", "--adapter", "caddyfile", "--config"])
            .arg(dir.join("Caddyfile"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.status.success(), "caddy validate failed:\n{stderr}");
    }
}
