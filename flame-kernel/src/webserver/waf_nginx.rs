//! WAF → Nginx / OpenResty 原生配置编译（Doc/32 W3 · WAF 前置联动）。
//!
//! 进程内 [`super::waf::WafEngine`] 只能评估「交给面板的请求」，而站点流量由 Web 引擎直接处理；
//! 本模块把启用规则编译为**纯原生 Nginx 指令**（`geo` / `map` / `limit_req`，不依赖 Lua /
//! ModSecurity），由 `WebServerManager::deploy_waf` 原子落盘 + 校验 + reload。
//!
//! 产物两个文件（均位于 `<配置根>/flamepanel/waf/`）：
//! - `http.conf`（http 级，主配置 `http {` 内 include）：规则匹配变量、白名单 / 阻断 / 告警汇总、
//!   CC 限频 zone、WAF 命中日志（JSON Lines）。
//! - `server.conf`（server 级，面板渲染的站点 `include …/server*.conf;`，glob 缺失不报错）：
//!   `if ($fp_waf_block) { return 403; }` + CC `limit_req`。
//!
//! 语义对齐进程内引擎：白名单优先；IP（精确 / CIDR、逗号分隔，IPv4）、UA 子串（不区分大小写）、
//! 路径前缀（区分大小写，解码后的 `$uri`）、SQLi / XSS 内置特征（请求行小写匹配，额外容忍
//! 百分号编码）；网站级规则只作用于该网站域名。差异（写入文档）：同一请求同时命中「告警」与
//! 「阻断」规则时 Nginx 侧**以阻断为准**（引擎为先命中先生效）；CC 仅下发 `block` 动作
//! （Nginx 无法对单条限频规则只记录），超限返回 429。
//!
//! 安全：所有来自规则 / 域名的字面量都转成 PCRE `\xHH` 转义后放进双引号，无法突破配置语法。

use super::engine::WebServerEngine;
use crate::domain::entity::WafRule;
use crate::webserver::waf::{SQLI_PATTERNS, XSS_PATTERNS};

/// 参与编译的站点（该引擎上的面板网站）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WafSite {
    pub website_id: i64,
    pub domains: Vec<String>,
}

/// 编译结果。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompiledWaf {
    pub http_conf: String,
    pub server_conf: String,
    /// 已下发的规则 ID。
    pub enforced: Vec<i64>,
    /// 未下发的规则及原因（规则所属网站不在该引擎上的不计入）。
    pub skipped: Vec<(i64, String)>,
}

/// 支持 WAF 下发的引擎。
pub fn supports(engine: &WebServerEngine) -> bool {
    matches!(engine, WebServerEngine::Nginx | WebServerEngine::OpenResty)
}

fn config_root(engine: &WebServerEngine) -> &'static str {
    match engine {
        WebServerEngine::OpenResty => "/usr/local/openresty/nginx/conf",
        _ => "/etc/nginx",
    }
}

/// `<配置根>/flamepanel/waf`
pub fn waf_dir(engine: &WebServerEngine) -> String {
    format!("{}/flamepanel/waf", config_root(engine))
}

pub fn http_conf_path(engine: &WebServerEngine) -> String {
    format!("{}/http.conf", waf_dir(engine))
}

pub fn server_conf_path(engine: &WebServerEngine) -> String {
    format!("{}/server.conf", waf_dir(engine))
}

/// 主配置 `http {` 内的 include（glob：文件缺失时 `nginx -t` 不报错）。
pub fn http_include_directive(engine: &WebServerEngine) -> String {
    format!("include {}/http*.conf;", waf_dir(engine))
}

/// 站点 `server {}` 内的 include 行（渲染器使用；glob 缺失不报错）。
pub fn server_include_directive(engine: &WebServerEngine) -> String {
    format!("include {}/server*.conf;", waf_dir(engine))
}

/// WAF 命中日志路径（JSON Lines）。
pub fn log_path(engine: &WebServerEngine) -> &'static str {
    match engine {
        WebServerEngine::OpenResty => "/usr/local/openresty/nginx/logs/flamepanel-waf.log",
        _ => "/var/log/nginx/flamepanel-waf.log",
    }
}

const HEADER: &str = "# Managed by Flamepanel WAF (webserver::waf_nginx). Do not edit: \
regenerated on every rule change / `POST /api/waf/apply`.\n";

/// 字面量 → PCRE：字母数字与 `_` `-` 原样，其余每个 UTF-8 字节写成 `\xHH`。
///
/// `\x` 不是 Nginx 引号字符串的转义序列（原样保留给 PCRE），因此引号、反斜杠、`$`、`;`、
/// 花括号等都无法逃出 `"…"`。
pub fn pcre_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

/// 特征串 → 容忍百分号编码的 PCRE：`'` 同时匹配 `'` 与 `%27`，空格匹配 ` ` / `+` / `%20`。
pub(crate) fn pcre_signature(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            out.push(b as char);
        } else if b == b' ' {
            out.push_str("(?:\\x20|\\x2b|%20)");
        } else {
            out.push_str(&format!("(?:\\x{b:02x}|%{b:02x})"));
        }
    }
    out
}

/// 域名集合 → 主机正则片段（锚定在键首）。全局规则匹配任意主机（不含分隔符）。
fn host_regex(domains: Option<&[String]>) -> String {
    match domains {
        None => "[^\\x7c]*".into(),
        Some(ds) => {
            let alts: Vec<String> = ds
                .iter()
                .map(|d| pcre_literal(&d.to_ascii_lowercase()))
                .collect();
            format!("(?:{})", alts.join("|"))
        }
    }
}

/// 解析 IP 规则值（逗号分隔，精确 IPv4 或 CIDR），非法项丢弃。
pub(crate) fn ip_items(pattern: &str) -> Vec<String> {
    pattern
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            let (net, prefix) = match p.split_once('/') {
                Some((n, pre)) => (n, Some(pre)),
                None => (p, None),
            };
            let ip: std::net::Ipv4Addr = net.trim().parse().ok()?;
            let prefix: u8 = match prefix {
                Some(pre) => pre.trim().parse().ok().filter(|v| *v <= 32)?,
                None => 32,
            };
            Some(format!("{ip}/{prefix}"))
        })
        .collect()
}

struct Emitter {
    out: String,
}

impl Emitter {
    fn line(&mut self, s: impl AsRef<str>) {
        self.out.push_str(s.as_ref());
        self.out.push('\n');
    }
}

/// 把规则编译为 Nginx 配置。`sites` 为该引擎上的面板网站（决定网站级规则的作用域）。
pub fn compile(rules: &[WafRule], sites: &[WafSite], engine: &WebServerEngine) -> CompiledWaf {
    let mut rules: Vec<&WafRule> = rules.iter().filter(|r| r.enabled).collect();
    rules.sort_by_key(|r| r.id);

    let mut http = Emitter { out: HEADER.into() };
    let mut enforced = Vec::new();
    let mut skipped: Vec<(i64, String)> = Vec::new();
    let mut whitelist_vars = Vec::new();
    let mut block_vars = Vec::new();
    let mut warn_vars = Vec::new();
    let mut tag_parts = Vec::new();
    // (rule id, threshold) — CC 规则在汇总变量之后输出（依赖 $fp_waf_wl）
    let mut cc_rules: Vec<(&WafRule, Option<Vec<String>>)> = Vec::new();

    for r in &rules {
        // 作用域：0 = 全局；网站级规则的网站不在本引擎上 → 与本引擎无关，静默忽略
        let scope: Option<Vec<String>> = if r.website_id == 0 {
            None
        } else {
            match sites.iter().find(|s| s.website_id == r.website_id) {
                Some(s) if !s.domains.is_empty() => Some(s.domains.clone()),
                _ => continue,
            }
        };
        let host = host_regex(scope.as_deref());
        let var = format!("$fp_waf_r{}", r.id);
        let comment = format!(
            "# rule {} [{}] {}",
            r.id,
            r.rule_type,
            r.name.replace('\n', " ")
        );

        let emitted = match r.rule_type.as_str() {
            "ip_blacklist" | "ip_whitelist" => {
                let items = ip_items(&r.pattern);
                if items.is_empty() {
                    skipped.push((r.id, "no valid IPv4 / CIDR in pattern".into()));
                    false
                } else {
                    http.line(&comment);
                    let geo_var = if scope.is_some() {
                        format!("$fp_waf_ip{}", r.id)
                    } else {
                        var.clone()
                    };
                    http.line(format!("geo {geo_var} {{"));
                    http.line("    default 0;");
                    for it in items {
                        http.line(format!("    {it} 1;"));
                    }
                    http.line("}");
                    if scope.is_some() {
                        http.line(format!("map \"$host|{geo_var}\" {var} {{"));
                        http.line("    default 0;");
                        http.line(format!("    \"~^{host}\\x7c1$\" 1;"));
                        http.line("}");
                    }
                    true
                }
            }
            "ua_blacklist" | "ua_whitelist" | "path_block" => {
                let p = r.pattern.trim();
                if p.is_empty() {
                    skipped.push((r.id, "empty pattern".into()));
                    false
                } else {
                    let (key, re) = if r.rule_type == "path_block" {
                        ("$host|$uri", format!("~^{host}\\x7c{}", pcre_literal(p)))
                    } else {
                        (
                            "$host|$http_user_agent",
                            format!("~*^{host}\\x7c.*{}", pcre_literal(p)),
                        )
                    };
                    http.line(&comment);
                    http.line(format!("map \"{key}\" {var} {{"));
                    http.line("    default 0;");
                    http.line(format!("    \"{re}\" 1;"));
                    http.line("}");
                    true
                }
            }
            "sql_injection" | "xss" => {
                let sigs: &[&str] = if r.rule_type == "xss" {
                    &XSS_PATTERNS
                } else {
                    &SQLI_PATTERNS
                };
                let alts: Vec<String> = sigs.iter().map(|s| pcre_signature(s)).collect();
                http.line(&comment);
                http.line(format!(
                    "map \"$host|$request_method $request_uri\" {var} {{"
                ));
                http.line("    default 0;");
                http.line(format!("    \"~*^{host}\\x7c.*(?:{})\" 1;", alts.join("|")));
                http.line("}");
                true
            }
            "cc" => {
                if r.action == "block" {
                    cc_rules.push((r, scope.clone()));
                    enforced.push(r.id);
                } else {
                    skipped.push((
                        r.id,
                        "CC rules are enforced by Nginx only with action=block (limit_req has no per-rule log-only mode)".into(),
                    ));
                }
                continue;
            }
            other => {
                skipped.push((r.id, format!("rule type {other} is not supported by Nginx")));
                false
            }
        };
        if !emitted {
            continue;
        }
        enforced.push(r.id);
        tag_parts.push(format!("r{}:{var}", r.id));
        match (r.rule_type.as_str(), r.action.as_str()) {
            ("ip_whitelist" | "ua_whitelist", _) => whitelist_vars.push(var),
            (_, "block") => block_vars.push(var),
            _ => warn_vars.push(var),
        }
    }

    // ── 汇总：白名单 → 阻断 / 告警 ──
    http.line("# summary (whitelist wins)");
    if whitelist_vars.is_empty() {
        http.line("map $host $fp_waf_wl { default 0; }");
    } else {
        http.line(format!("map \"{}\" $fp_waf_wl {{", whitelist_vars.concat()));
        http.line("    default 1;");
        http.line("    \"~^0+$\" 0;");
        http.line("}");
    }
    for (name, vars) in [("$fp_waf_block", &block_vars), ("$fp_waf_warn", &warn_vars)] {
        if vars.is_empty() {
            http.line(format!("map $host {name} {{ default 0; }}"));
        } else {
            http.line(format!("map \"$fp_waf_wl|{}\" {name} {{", vars.concat()));
            http.line("    default 0;");
            http.line("    \"~^0\\x7c.*1\" 1;");
            http.line("}");
        }
    }
    if cc_rules.is_empty() {
        http.line("map \"$fp_waf_block$fp_waf_warn\" $fp_waf_hit { default 1; \"00\" 0; }");
    } else {
        // CC 拒绝也记命中日志：$limit_req_status 在 log 阶段已确定（nginx ≥ 1.17.6）
        http.line("map \"$fp_waf_block$fp_waf_warn$limit_req_status\" $fp_waf_hit {");
        http.line("    default 0;");
        http.line("    \"~1\" 1;");
        http.line("    \"~REJECTED\" 1;");
        http.line("}");
    }

    // ── CC：白名单与作用域外请求的 key 为空（limit_req 不计数）──
    let mut server = Emitter { out: HEADER.into() };
    server.line("if ($fp_waf_block) {");
    server.line("    return 403;");
    server.line("}");
    for (r, scope) in &cc_rules {
        let (threshold, window_secs) = r.cc_params();
        let per_min = ((threshold as u64 * 60).div_ceil(window_secs)).max(1);
        let host = host_regex(scope.as_deref());
        http.line(format!(
            "# rule {} [cc] {} ({threshold} req / {window_secs}s)",
            r.id,
            r.name.replace('\n', " ")
        ));
        http.line(format!("map \"$fp_waf_wl|$host\" $fp_waf_cck{} {{", r.id));
        http.line("    default \"\";");
        http.line(format!("    \"~^0\\x7c{host}$\" $binary_remote_addr;"));
        http.line("}");
        http.line(format!(
            "limit_req_zone $fp_waf_cck{id} zone=fp_waf_cc{id}:10m rate={per_min}r/m;",
            id = r.id
        ));
        // 命中日志标记：该请求是否在本规则作用域内（拒绝时据此归因到规则）
        http.line(format!(
            "map $fp_waf_cck{id} $fp_waf_ccs{id} {{ default 1; \"\" 0; }}",
            id = r.id
        ));
        tag_parts.push(format!("c{id}:$fp_waf_ccs{id}", id = r.id));
        server.line(format!(
            "limit_req zone=fp_waf_cc{} burst={threshold} nodelay;",
            r.id
        ));
    }
    if !cc_rules.is_empty() {
        server.line("limit_req_status 429;");
    }

    // ── 命中日志（JSON Lines；http 级 access_log 与主配置的 access_log 并存继承）──
    http.line("# hit log (log + block actions)");
    let tags = if tag_parts.is_empty() {
        String::new()
    } else {
        tag_parts.join(" ")
    };
    let cc = if cc_rules.is_empty() {
        ""
    } else {
        "$limit_req_status"
    };
    http.line(format!(
        "log_format fp_waf escape=json '{{\"time\":\"$time_iso8601\",\"host\":\"$host\",\"ip\":\"$remote_addr\",\"method\":\"$request_method\",\"uri\":\"$request_uri\",\"ua\":\"$http_user_agent\",\"status\":\"$status\",\"block\":\"$fp_waf_block\",\"rules\":\"{tags}\",\"cc\":\"{cc}\"}}';"
    ));
    http.line(format!(
        "access_log {} fp_waf if=$fp_waf_hit;",
        log_path(engine)
    ));

    CompiledWaf {
        http_conf: http.out,
        server_conf: server.out,
        enforced,
        skipped,
    }
}

/// 命中日志一行（`log_format fp_waf`）的解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NginxHit {
    pub time: Option<chrono::DateTime<chrono::Utc>>,
    /// 小写主机名
    pub host: String,
    pub client_ip: String,
    pub method: String,
    pub uri: String,
    pub user_agent: String,
    /// 请求被 WAF 阻断（403）
    pub blocked: bool,
    /// 命中的规则（`rN:1`；白名单规则也会出现，由调用方按类型过滤）
    pub rules: Vec<i64>,
    /// 被 CC 拒绝（429）时，作用域内的 CC 规则（`cN:1` 且 `cc=REJECTED`）
    pub cc_rules: Vec<i64>,
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
    #[serde(default)]
    cc: String,
}

/// 解析命中日志一行；非本格式（空行、截断行、手工写入）返回 `None`。
pub fn parse_hit_line(line: &str) -> Option<NginxHit> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let raw: RawHit = serde_json::from_str(line).ok()?;
    let rejected = raw.cc == "REJECTED";
    let mut rules = Vec::new();
    let mut cc_rules = Vec::new();
    for tag in raw.rules.split_whitespace() {
        let Some((name, val)) = tag.split_once(':') else {
            continue;
        };
        if val != "1" {
            continue;
        }
        if let Some(id) = name.strip_prefix('r').and_then(|n| n.parse().ok()) {
            rules.push(id);
        } else if let Some(id) = name.strip_prefix('c').and_then(|n| n.parse().ok()) {
            if rejected {
                cc_rules.push(id);
            }
        }
    }
    if rules.is_empty() && cc_rules.is_empty() {
        return None;
    }
    Some(NginxHit {
        time: chrono::DateTime::parse_from_rfc3339(&raw.time)
            .ok()
            .map(|t| t.with_timezone(&chrono::Utc)),
        host: raw.host.to_ascii_lowercase(),
        client_ip: raw.ip,
        method: raw.method,
        uri: raw.uri,
        user_agent: raw.ua,
        blocked: raw.block == "1",
        rules,
        cc_rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: i64, t: &str, pattern: &str, action: &str, website_id: i64) -> WafRule {
        WafRule {
            id,
            name: format!("r{id}"),
            rule_type: t.into(),
            website_id,
            pattern: pattern.into(),
            config_json: "{}".into(),
            action: action.into(),
            enabled: true,
            ..Default::default()
        }
    }

    fn site(id: i64, d: &str) -> WafSite {
        WafSite {
            website_id: id,
            domains: vec![d.into()],
        }
    }

    const NGX: WebServerEngine = WebServerEngine::Nginx;

    /// 简易括号 / 引号平衡检查（防模板拼错导致 nginx -t 失败）。
    fn balanced(conf: &str) -> bool {
        let mut depth = 0i32;
        let mut in_q = false;
        let mut prev = ' ';
        for c in conf.chars() {
            match c {
                '"' if prev != '\\' => in_q = !in_q,
                '{' if !in_q => depth += 1,
                '}' if !in_q => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
            prev = c;
        }
        depth == 0 && !in_q
    }

    #[test]
    fn empty_rules_still_define_all_variables() {
        let c = compile(&[], &[], &NGX);
        for v in ["$fp_waf_wl", "$fp_waf_block", "$fp_waf_warn", "$fp_waf_hit"] {
            assert!(
                c.http_conf.contains(&format!(" {v} ")),
                "{v}\n{}",
                c.http_conf
            );
        }
        assert!(c.server_conf.contains("if ($fp_waf_block)"));
        assert!(!c.server_conf.contains("limit_req"));
        assert!(balanced(&c.http_conf) && balanced(&c.server_conf));
        assert!(c.enforced.is_empty() && c.skipped.is_empty());
    }

    #[test]
    fn ip_rules_compile_to_geo_and_respect_scope() {
        let rules = vec![
            rule(
                1,
                "ip_blacklist",
                "10.0.0.0/8, 1.2.3.4, bogus, 300.1.1.1",
                "block",
                0,
            ),
            rule(2, "ip_whitelist", "10.1.1.1", "log", 0),
            rule(3, "ip_blacklist", "5.6.7.8", "block", 7),
            rule(4, "ip_blacklist", "9.9.9.9", "block", 99), // 网站不在本引擎
        ];
        let c = compile(&rules, &[site(7, "a.test")], &NGX);
        assert!(c
            .http_conf
            .contains("geo $fp_waf_r1 {\n    default 0;\n    10.0.0.0/8 1;\n    1.2.3.4/32 1;\n}"));
        assert!(!c.http_conf.contains("bogus") && !c.http_conf.contains("300.1.1.1"));
        // 网站级：geo + host 映射
        assert!(c.http_conf.contains("geo $fp_waf_ip3 {"));
        assert!(c
            .http_conf
            .contains("map \"$host|$fp_waf_ip3\" $fp_waf_r3 {"));
        assert!(c.http_conf.contains("\"~^(?:a\\x2etest)\\x7c1$\" 1;"));
        assert!(!c.http_conf.contains("fp_waf_r4"));
        // 汇总
        assert!(c.http_conf.contains("map \"$fp_waf_r2\" $fp_waf_wl {"));
        assert!(c
            .http_conf
            .contains("map \"$fp_waf_wl|$fp_waf_r1$fp_waf_r3\" $fp_waf_block {"));
        assert_eq!(c.enforced, vec![1, 2, 3]);
        assert!(balanced(&c.http_conf));
    }

    #[test]
    fn literals_cannot_escape_quotes() {
        let rules = vec![
            rule(
                1,
                "ua_blacklist",
                "evil\"; } server { listen 1; #",
                "block",
                0,
            ),
            rule(2, "path_block", "/wp-admin$ {x}\\", "log", 0),
        ];
        let c = compile(&rules, &[], &NGX);
        for bad in ["evil\"", "listen 1", "{x}", "\\\""] {
            assert!(!c.http_conf.contains(bad), "{bad} leaked:\n{}", c.http_conf);
        }
        assert!(c
            .http_conf
            .contains("\"~*^[^\\x7c]*\\x7c.*evil\\x22\\x3b\\x20"));
        assert!(c.http_conf.contains("\"~^[^\\x7c]*\\x7c\\x2fwp-admin\\x24"));
        assert!(balanced(&c.http_conf));
        // 告警规则进 warn 汇总
        assert!(c
            .http_conf
            .contains("map \"$fp_waf_wl|$fp_waf_r2\" $fp_waf_warn {"));
    }

    #[test]
    fn signatures_tolerate_percent_encoding() {
        let c = compile(&[rule(5, "sql_injection", "", "block", 0)], &[], &NGX);
        // "union select" → 空格容忍 +/%20
        assert!(c.http_conf.contains("union(?:\\x20|\\x2b|%20)select"));
        // "' or 1=1" → 引号容忍 %27
        assert!(c.http_conf.contains("(?:\\x27|%27)"));
        assert!(c
            .http_conf
            .contains("map \"$host|$request_method $request_uri\" $fp_waf_r5 {"));
        let x = compile(&[rule(6, "xss", "", "log", 0)], &[], &NGX);
        assert!(x.http_conf.contains("(?:\\x3c|%3c)script"));
    }

    #[test]
    fn cc_block_emits_zone_and_limit_log_is_skipped() {
        let mut cc = rule(8, "cc", "", "block", 7);
        cc.config_json = r#"{"threshold":30,"window_secs":10}"#.into();
        let cc_log = rule(9, "cc", "", "log", 0);
        let c = compile(&[cc, cc_log], &[site(7, "a.test")], &NGX);
        assert!(c
            .http_conf
            .contains("limit_req_zone $fp_waf_cck8 zone=fp_waf_cc8:10m rate=180r/m;"));
        assert!(c
            .http_conf
            .contains("\"~^0\\x7c(?:a\\x2etest)$\" $binary_remote_addr;"));
        assert!(c
            .server_conf
            .contains("limit_req zone=fp_waf_cc8 burst=30 nodelay;"));
        assert!(c.server_conf.contains("limit_req_status 429;"));
        assert_eq!(c.enforced, vec![8]);
        assert_eq!(c.skipped.len(), 1);
        assert_eq!(c.skipped[0].0, 9);
    }

    #[test]
    fn disabled_and_invalid_rules() {
        let mut off = rule(1, "ip_blacklist", "1.1.1.1", "block", 0);
        off.enabled = false;
        let c = compile(
            &[
                off,
                rule(2, "ua_blacklist", "  ", "block", 0),
                rule(3, "weird", "x", "block", 0),
            ],
            &[],
            &NGX,
        );
        assert!(!c.http_conf.contains("fp_waf_r1"));
        assert_eq!(
            c.skipped.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn engine_specific_paths() {
        let o = WebServerEngine::OpenResty;
        assert_eq!(http_conf_path(&NGX), "/etc/nginx/flamepanel/waf/http.conf");
        assert_eq!(
            server_include_directive(&o),
            "include /usr/local/openresty/nginx/conf/flamepanel/waf/server*.conf;"
        );
        assert!(compile(&[], &[], &o)
            .http_conf
            .contains("/usr/local/openresty/nginx/logs/flamepanel-waf.log"));
        assert!(supports(&o) && supports(&NGX) && !supports(&WebServerEngine::Caddy));
        // 路径必须落在特权助手白名单内
        for p in [http_conf_path(&NGX), server_conf_path(&o)] {
            assert!(crate::webserver::helper::policy::path_allowed(&p), "{p}");
        }
    }

    #[test]
    fn pcre_literal_escapes_everything_but_word_chars() {
        assert_eq!(pcre_literal("a.b-c_1"), "a\\x2eb-c_1");
        assert_eq!(pcre_literal("é"), "\\xc3\\xa9");
        assert_eq!(pcre_literal("\"$;{}\\"), "\\x22\\x24\\x3b\\x7b\\x7d\\x5c");
    }

    /// 真实 `nginx -t` 校验编译产物（本机无 nginx 时跳过；CNB 开发环境装 nginx 后执行）。
    #[test]
    fn real_nginx_accepts_compiled_config() {
        let Ok(out) = std::process::Command::new("nginx").arg("-v").output() else {
            eprintln!("nginx not installed; skipping");
            return;
        };
        let _ = out;
        let dir = std::env::temp_dir().join(format!("fp-waf-ngx-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("waf")).unwrap();
        let mut cc = rule(8, "cc", "", "block", 7);
        cc.config_json = r#"{"threshold":30,"window_secs":10}"#.into();
        let rules = vec![
            rule(1, "ip_blacklist", "1.2.3.4, 10.0.0.0/8", "block", 0),
            rule(2, "ip_whitelist", "127.0.0.1", "log", 0),
            rule(3, "ip_blacklist", "5.6.7.8", "log", 7),
            rule(4, "ua_blacklist", "sqlmap\"; evil {", "block", 0),
            rule(5, "ua_whitelist", "GoodBot", "log", 7),
            rule(6, "path_block", "/wp-admin$ {x}\\", "block", 7),
            rule(7, "sql_injection", "", "block", 0),
            rule(9, "xss", "", "log", 7),
            cc,
        ];
        let c = compile(&rules, &[site(7, "A.test"), site(8, "b.test")], &NGX);
        let log = dir.join("waf.log");
        let http = c.http_conf.replace(log_path(&NGX), log.to_str().unwrap());
        std::fs::write(dir.join("waf/http.conf"), http).unwrap();
        std::fs::write(dir.join("waf/server.conf"), &c.server_conf).unwrap();
        let d = dir.display();
        let main = format!(
            "pid {d}/nginx.pid;\nerror_log {d}/error.log;\nevents {{ worker_connections 64; }}\n\
             http {{\n    include {d}/waf/http*.conf;\n    access_log {d}/access.log;\n\
             server {{\n        listen 127.0.0.1:18089;\n        server_name a.test;\n        \
             include {d}/waf/server*.conf;\n        location / {{ return 200; }}\n    }}\n}}\n"
        );
        std::fs::write(dir.join("nginx.conf"), main).unwrap();
        let out = std::process::Command::new("nginx")
            .args(["-t", "-p"])
            .arg(&dir)
            .arg("-c")
            .arg(dir.join("nginx.conf"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.status.success(), "nginx -t failed:\n{stderr}");
    }

    #[test]
    fn hit_log_carries_cc_status_only_with_cc_rules() {
        let plain = compile(&[rule(1, "ua_blacklist", "x", "block", 0)], &[], &NGX);
        assert!(plain.http_conf.contains(r#""cc":""}"#));
        assert!(!plain.http_conf.contains("$limit_req_status"));
        let mut cc = rule(8, "cc", "", "block", 0);
        cc.config_json = r#"{"threshold":5,"window_secs":60}"#.into();
        let c = compile(&[cc], &[], &NGX);
        assert!(c
            .http_conf
            .contains("map $fp_waf_cck8 $fp_waf_ccs8 { default 1; \"\" 0; }"));
        assert!(c.http_conf.contains("c8:$fp_waf_ccs8"));
        assert!(c.http_conf.contains("\"~REJECTED\" 1;"));
        assert!(c.http_conf.contains(r#""cc":"$limit_req_status"}"#));
    }

    #[test]
    fn parse_hit_lines() {
        let l = r#"{"time":"2026-09-27T11:25:48+08:00","host":"A.test","ip":"1.2.3.4","method":"GET","uri":"/?q=\u003cscript","ua":"sqlmap/1.7","status":"403","block":"1","rules":"r1:1 r2:0 r5:1","cc":""}"#;
        let h = parse_hit_line(l).unwrap();
        assert_eq!(h.host, "a.test");
        assert_eq!(h.uri, "/?q=<script");
        assert!(h.blocked);
        assert_eq!(h.rules, vec![1, 5]);
        assert!(h.cc_rules.is_empty());
        assert_eq!(h.time.unwrap().to_rfc3339(), "2026-09-27T03:25:48+00:00");
        // CC：仅 REJECTED 时归因作用域内规则
        let c = r#"{"time":"x","host":"c.test","ip":"1.1.1.1","method":"GET","uri":"/","ua":"","status":"429","block":"0","rules":"r1:0 c8:1 c9:0","cc":"REJECTED"}"#;
        let h = parse_hit_line(c).unwrap();
        assert_eq!((h.rules.len(), h.cc_rules.clone()), (0, vec![8]));
        assert!(h.time.is_none());
        let passed = c.replace("REJECTED", "PASSED");
        assert!(parse_hit_line(&passed).is_none());
        // 旧格式（无 cc 字段）与垃圾行
        assert!(parse_hit_line(r#"{"host":"a","rules":"r3:1"}"#).is_some());
        for bad in [
            "",
            "not json",
            "{\"rules\":\"r1:0\"}",
            "{\"rules\":\"rx:1 :1\"}",
        ] {
            assert!(parse_hit_line(bad).is_none(), "{bad}");
        }
    }
}
