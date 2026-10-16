//! WAF 多引擎下发分发（Doc/30 v1.1 · Doc/32 §9）：Nginx / OpenResty / Apache / Caddy /
//! OpenLiteSpeed 统一入口。
//!
//! 各引擎编译器只产出**纯原生配置**（不依赖 ModSecurity / Lua / 第三方插件）：
//!
//! | 引擎 | 编译器 | 匹配 / 阻断 | 命中日志 | CC |
//! |---|---|---|---|---|
//! | Nginx / OpenResty | [`super::waf_nginx`] | `geo` / `map` → `return 403` | `access_log if=` | `limit_req` |
//! | Apache 2.4 | [`super::waf_apache`] | `SetEnvIfExpr` → `<If>` `Require all denied` | `GlobalLog env=` | 不支持 |
//! | Caddy 2 | [`super::waf_caddy`] | 命名匹配器 + `vars` → `respond 403` | `log` + `skip_log` | 不支持 |
//! | OpenLiteSpeed | [`super::waf_ols`] | vhost `rewrite` `RewriteCond` → `[F]` | 不支持（仅下发阻断规则） | 不支持 |
//!
//! 共同语义（与进程内引擎一致）：白名单优先；IP（精确 / CIDR，IPv4）、UA 子串（不区分大小写）、
//! 路径前缀（区分大小写，解码后的路径）、SQLi / XSS 内置特征（容忍百分号编码）；
//! 网站级规则只作用于该网站域名。不支持的规则进入 `skipped` 并附原因。
//!
//! 产物统一为 [`CompiledWaf`]：`http_conf`（主配置级，Nginx `http {}` / Apache 服务器级；
//! Caddy / OLS 为空）+ `server_conf`（站点级 include；OLS 为按站点分段的规则库，由
//! `WebServerManager` 拼接进各站点 `vhconf.conf`）。

use super::engine::WebServerEngine;
use super::layout::{Hook, SiteLayout};
use super::waf_nginx::{self, ip_items, NginxHit};
use crate::domain::entity::WafRule;
use crate::infrastructure::os::DistroType;
use crate::webserver::waf::{SQLI_PATTERNS, XSS_PATTERNS};

pub use super::waf_nginx::{CompiledWaf, WafSite};

/// 命中日志一行的统一解析结果（沿用 Nginx 首个实现的结构体名）。
pub type WafHit = NginxHit;

/// 支持 WAF 下发的引擎（五引擎全部支持；能力差异见模块文档）。
pub fn supports(engine: &WebServerEngine) -> bool {
    matches!(
        engine,
        WebServerEngine::Nginx
            | WebServerEngine::OpenResty
            | WebServerEngine::Apache
            | WebServerEngine::Caddy
            | WebServerEngine::OpenLiteSpeed
    )
}

/// 该引擎是否产出可回灌的命中日志（OLS 无原生条件日志）。
pub fn has_hit_log(engine: &WebServerEngine) -> bool {
    !matches!(engine, WebServerEngine::OpenLiteSpeed)
}

/// 主配置级文件路径（Nginx `http.conf` / Apache `http.conf`）；Caddy / OLS 无。
pub fn http_conf_path(layout: &SiteLayout) -> Option<String> {
    match layout.engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => {
            Some(waf_nginx::http_conf_path(&layout.engine))
        }
        WebServerEngine::Apache => Some(format!("{}/http.conf", layout.waf_dir())),
        WebServerEngine::Caddy | WebServerEngine::OpenLiteSpeed => None,
    }
}

/// 站点级文件路径（Nginx `server.conf` / Apache `vhost.conf` / Caddy `site.caddy` /
/// OLS 规则库 `vhost.rules`）。
pub fn server_conf_path(layout: &SiteLayout) -> String {
    match layout.engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => {
            waf_nginx::server_conf_path(&layout.engine)
        }
        WebServerEngine::Apache => format!("{}/vhost.conf", layout.waf_dir()),
        WebServerEngine::Caddy => format!("{}/site.caddy", layout.waf_dir()),
        WebServerEngine::OpenLiteSpeed => format!("{}/vhost.rules", layout.waf_dir()),
    }
}

/// 主配置钩子（幂等插入）：Nginx `http {` 内 include；Apache 末尾 `IncludeOptional`。
pub fn http_hook(layout: &SiteLayout) -> Option<Hook> {
    match layout.engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => Some(Hook::HttpBlockInclude {
            directive: waf_nginx::http_include_directive(&layout.engine),
        }),
        WebServerEngine::Apache => Some(Hook::Append {
            directive: format!("IncludeOptional {}/http*.conf", layout.waf_dir()),
        }),
        WebServerEngine::Caddy | WebServerEngine::OpenLiteSpeed => None,
    }
}

/// 站点文件中标识「已接入 WAF」的 include 行；OLS 由部署流水线直接拼接规则，返回 `None`。
pub fn site_include_directive(layout: &SiteLayout) -> Option<String> {
    match layout.engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => {
            Some(waf_nginx::server_include_directive(&layout.engine))
        }
        WebServerEngine::Apache => {
            Some(format!("IncludeOptional {}/vhost*.conf", layout.waf_dir()))
        }
        WebServerEngine::Caddy => Some(caddy_import_directive()),
        WebServerEngine::OpenLiteSpeed => None,
    }
}

/// Apache 站点渲染器使用的 include 行：生成器不感知发行版，Debian（`/etc/apache2`）与
/// RHEL（`/etc/httpd`）两个位置都写（`IncludeOptional` + glob：目录不存在也不报错）。
pub fn apache_vhost_includes() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for distro in [DistroType::Debian, DistroType::RHEL] {
        let layout = SiteLayout::for_engine(WebServerEngine::Apache, &distro);
        if let Some(d) = site_include_directive(&layout) {
            if !out.contains(&d) {
                out.push(d);
            }
        }
    }
    out
}

/// Caddy 站点块内的 import 行（glob 无匹配时 Caddy 不报错）。
pub fn caddy_import_directive() -> String {
    let layout = SiteLayout::for_engine(WebServerEngine::Caddy, &DistroType::Debian);
    format!("import {}/site*.caddy", layout.waf_dir())
}

/// WAF 命中日志路径（JSON Lines）；OLS 无。
pub fn log_path(layout: &SiteLayout) -> Option<&'static str> {
    match layout.engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => {
            Some(waf_nginx::log_path(&layout.engine))
        }
        WebServerEngine::Apache => Some(if layout.waf_dir().starts_with("/etc/httpd") {
            "/var/log/httpd/flamepanel-waf.log"
        } else {
            "/var/log/apache2/flamepanel-waf.log"
        }),
        WebServerEngine::Caddy => Some("/var/log/caddy/flamepanel-waf.log"),
        WebServerEngine::OpenLiteSpeed => None,
    }
}

/// 全部可能的命中日志路径（特权助手 `read_log` 白名单；覆盖各发行版布局）。
pub fn all_log_paths() -> Vec<&'static str> {
    let mut out = Vec::new();
    for engine in [
        WebServerEngine::Nginx,
        WebServerEngine::OpenResty,
        WebServerEngine::Apache,
        WebServerEngine::Caddy,
    ] {
        for distro in [DistroType::Debian, DistroType::RHEL, DistroType::Alpine] {
            if let Some(p) = log_path(&SiteLayout::for_engine(engine.clone(), &distro)) {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// 按引擎编译。
pub fn compile(rules: &[WafRule], sites: &[WafSite], layout: &SiteLayout) -> CompiledWaf {
    match layout.engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => {
            waf_nginx::compile(rules, sites, &layout.engine)
        }
        WebServerEngine::Apache => {
            super::waf_apache::compile(rules, sites, log_path(layout).unwrap_or_default())
        }
        WebServerEngine::Caddy => {
            super::waf_caddy::compile(rules, sites, log_path(layout).unwrap_or_default())
        }
        WebServerEngine::OpenLiteSpeed => super::waf_ols::compile(rules, sites),
    }
}

/// 按引擎解析命中日志一行。
pub fn parse_hit_line(engine: &WebServerEngine, line: &str) -> Option<WafHit> {
    match engine {
        WebServerEngine::Nginx | WebServerEngine::OpenResty => waf_nginx::parse_hit_line(line),
        WebServerEngine::Apache => super::waf_apache::parse_hit_line(line),
        WebServerEngine::Caddy => super::waf_caddy::parse_hit_line(line),
        WebServerEngine::OpenLiteSpeed => None,
    }
}

// ───────────────────── 共享：规则归一化（非 Nginx 编译器使用）─────────────────────

/// 匹配条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cond {
    /// `a.b.c.d/p`（已校验的 IPv4 CIDR）
    Ips(Vec<String>),
    /// UA 子串（不区分大小写）
    Ua(String),
    /// 路径前缀（区分大小写）
    Path(String),
    /// 内置特征集（小写；容忍百分号编码）
    Sig(&'static [&'static str]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Whitelist,
    Block,
    /// 仅记录（告警）
    Warn,
}

/// 归一化后的一条可下发规则。
#[derive(Debug, Clone)]
pub(crate) struct NormRule {
    pub id: i64,
    pub rule_type: String,
    pub name: String,
    /// `None` = 全局；`Some` = 网站级（该网站的域名，小写）
    pub scope: Option<Vec<String>>,
    pub cond: Cond,
    pub kind: Kind,
}

impl NormRule {
    /// 一行注释（换行 / 回车替换为空格，避免突破注释行）。
    pub fn comment(&self) -> String {
        format!(
            "# rule {} [{}] {}",
            self.id,
            self.rule_type,
            self.name.replace(['\n', '\r'], " ")
        )
    }

    /// 是否作用于该域名（全局规则恒为真）。
    pub fn applies_to(&self, domain: &str) -> bool {
        match &self.scope {
            None => true,
            Some(ds) => ds.iter().any(|d| d.eq_ignore_ascii_case(domain)),
        }
    }
}

/// 启用规则 → 可下发规则 + 跳过原因。`engine_name` 用于原因文案；
/// `log_only_supported = false` 时仅记录（告警）规则跳过（引擎无原生条件日志）。
pub(crate) fn normalize(
    rules: &[WafRule],
    sites: &[WafSite],
    engine_name: &str,
    log_only_supported: bool,
) -> (Vec<NormRule>, Vec<(i64, String)>) {
    let mut rules: Vec<&WafRule> = rules.iter().filter(|r| r.enabled).collect();
    rules.sort_by_key(|r| r.id);
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for r in rules {
        let scope: Option<Vec<String>> = if r.website_id == 0 {
            None
        } else {
            match sites.iter().find(|s| s.website_id == r.website_id) {
                Some(s) if !s.domains.is_empty() => {
                    Some(s.domains.iter().map(|d| d.to_ascii_lowercase()).collect())
                }
                // 网站不在本引擎上：与本引擎无关，静默忽略
                _ => continue,
            }
        };
        let p = r.pattern.trim();
        let cond = match r.rule_type.as_str() {
            "ip_blacklist" | "ip_whitelist" => {
                let items = ip_items(p);
                if items.is_empty() {
                    skipped.push((r.id, "no valid IPv4 / CIDR in pattern".into()));
                    continue;
                }
                Cond::Ips(items)
            }
            "ua_blacklist" | "ua_whitelist" | "path_block" if p.is_empty() => {
                skipped.push((r.id, "empty pattern".into()));
                continue;
            }
            "ua_blacklist" | "ua_whitelist" => Cond::Ua(p.to_string()),
            "path_block" => Cond::Path(p.to_string()),
            "sql_injection" => Cond::Sig(&SQLI_PATTERNS),
            "xss" => Cond::Sig(&XSS_PATTERNS),
            "cc" => {
                skipped.push((
                    r.id,
                    format!(
                        "CC rate limiting is not enforced on {engine_name}: the engine has no native per-client rate limit (it would need a third-party module)"
                    ),
                ));
                continue;
            }
            other => {
                skipped.push((
                    r.id,
                    format!("rule type {other} is not supported by {engine_name}"),
                ));
                continue;
            }
        };
        let kind = match (r.rule_type.as_str(), r.action.as_str()) {
            ("ip_whitelist" | "ua_whitelist", _) => Kind::Whitelist,
            (_, "block") => Kind::Block,
            _ => Kind::Warn,
        };
        if kind == Kind::Warn && !log_only_supported {
            skipped.push((
                r.id,
                format!(
                    "{engine_name} has no native conditional logging; only action=block rules are enforced"
                ),
            ));
            continue;
        }
        out.push(NormRule {
            id: r.id,
            rule_type: r.rule_type.clone(),
            name: r.name.clone(),
            scope,
            cond,
            kind,
        });
    }
    (out, skipped)
}

/// 规则值字面量 → 正则（PCRE / RE2 通用）：字母数字 `_` `-` 原样，其余字节 `\xHH`。
pub(crate) fn re_literal(s: &str) -> String {
    waf_nginx::pcre_literal(s)
}

/// 特征集 → 容忍百分号编码的交替正则 `(?:a|b|…)`。
pub(crate) fn re_signatures(sigs: &[&str]) -> String {
    let alts: Vec<String> = sigs.iter().map(|s| waf_nginx::pcre_signature(s)).collect();
    format!("(?:{})", alts.join("|"))
}

/// 主机名（可能带 `:port`）→ 小写、去端口（IPv6 字面量 `[::1]:80` 保留方括号内部）。
pub(crate) fn normalize_host(host: &str) -> String {
    let h = host.trim();
    let h = if let Some(rest) = h.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest)
    } else {
        h.rsplit_once(':')
            .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
            .map(|(name, _)| name)
            .unwrap_or(h)
    };
    h.to_ascii_lowercase()
}

/// 解析命中日志中的 `rN:1` 标签（值不是 `1` 的忽略：未命中 / Apache 的 `-`）。
pub(crate) fn parse_rule_tags(tags: &str) -> Vec<i64> {
    tags.split_whitespace()
        .filter_map(|t| {
            let (name, val) = t.split_once(':')?;
            if val != "1" {
                return None;
            }
            name.strip_prefix('r')?.parse().ok()
        })
        .collect()
}

pub(crate) const HEADER_NOTE: &str =
    "Do not edit: regenerated on every rule change / `POST /api/waf/apply`.";

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn rule(id: i64, t: &str, pattern: &str, action: &str, website_id: i64) -> WafRule {
        WafRule {
            id,
            name: format!("rule-{id}"),
            rule_type: t.into(),
            website_id,
            pattern: pattern.into(),
            config_json: "{}".into(),
            action: action.into(),
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn every_engine_supported_and_paths_follow_layout() {
        for e in [
            WebServerEngine::Nginx,
            WebServerEngine::OpenResty,
            WebServerEngine::Apache,
            WebServerEngine::Caddy,
            WebServerEngine::OpenLiteSpeed,
        ] {
            assert!(supports(&e));
        }
        let deb = SiteLayout::for_engine(WebServerEngine::Apache, &DistroType::Debian);
        let rhel = SiteLayout::for_engine(WebServerEngine::Apache, &DistroType::RHEL);
        assert_eq!(
            server_conf_path(&deb),
            "/etc/apache2/flamepanel/waf/vhost.conf"
        );
        assert_eq!(
            http_conf_path(&rhel).as_deref(),
            Some("/etc/httpd/flamepanel/waf/http.conf")
        );
        assert_eq!(log_path(&deb), Some("/var/log/apache2/flamepanel-waf.log"));
        assert_eq!(log_path(&rhel), Some("/var/log/httpd/flamepanel-waf.log"));
        assert_eq!(
            http_hook(&rhel),
            Some(Hook::Append {
                directive: "IncludeOptional /etc/httpd/flamepanel/waf/http*.conf".into()
            })
        );
        assert_eq!(
            apache_vhost_includes(),
            vec![
                "IncludeOptional /etc/apache2/flamepanel/waf/vhost*.conf".to_string(),
                "IncludeOptional /etc/httpd/flamepanel/waf/vhost*.conf".to_string(),
            ]
        );
        let caddy = SiteLayout::for_engine(WebServerEngine::Caddy, &DistroType::Debian);
        assert_eq!(
            server_conf_path(&caddy),
            "/etc/caddy/flamepanel/waf/site.caddy"
        );
        assert!(http_conf_path(&caddy).is_none() && http_hook(&caddy).is_none());
        assert_eq!(
            caddy_import_directive(),
            "import /etc/caddy/flamepanel/waf/site*.caddy"
        );
        let ols = SiteLayout::for_engine(WebServerEngine::OpenLiteSpeed, &DistroType::Debian);
        assert_eq!(
            server_conf_path(&ols),
            "/usr/local/lsws/conf/flamepanel/waf/vhost.rules"
        );
        assert!(log_path(&ols).is_none() && site_include_directive(&ols).is_none());
        // Nginx 路径与既有实现一致（未迁移）
        let ngx = SiteLayout::for_engine(WebServerEngine::Nginx, &DistroType::Debian);
        assert_eq!(ngx.waf_dir(), waf_nginx::waf_dir(&WebServerEngine::Nginx));
        let all = all_log_paths();
        for p in [
            "/var/log/nginx/flamepanel-waf.log",
            "/var/log/apache2/flamepanel-waf.log",
            "/var/log/httpd/flamepanel-waf.log",
            "/var/log/caddy/flamepanel-waf.log",
        ] {
            assert!(all.contains(&p), "{p}");
        }
    }

    #[test]
    fn normalize_scopes_skips_and_kinds() {
        let sites = vec![WafSite {
            website_id: 7,
            domains: vec!["A.example.com".into()],
        }];
        let mut disabled = rule(9, "path_block", "/x", "block", 0);
        disabled.enabled = false;
        let rules = vec![
            rule(5, "cc", "10/60", "block", 0),
            rule(1, "ip_whitelist", "10.0.0.1, bogus", "block", 0),
            rule(2, "path_block", "/admin", "block", 7),
            rule(3, "ua_blacklist", "bot", "log", 0),
            rule(4, "path_block", "/other", "block", 99),
            rule(6, "ip_blacklist", "bogus", "block", 0),
            rule(8, "ua_blacklist", "  ", "block", 0),
            disabled,
        ];
        let (norm, skipped) = normalize(&rules, &sites, "Apache", true);
        let ids: Vec<i64> = norm.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(norm[0].kind, Kind::Whitelist);
        assert_eq!(norm[0].cond, Cond::Ips(vec!["10.0.0.1/32".into()]));
        assert_eq!(norm[1].scope, Some(vec!["a.example.com".into()]));
        assert!(norm[1].applies_to("A.EXAMPLE.com") && !norm[1].applies_to("b.example.com"));
        assert_eq!(norm[2].kind, Kind::Warn);
        let skipped_ids: Vec<i64> = skipped.iter().map(|s| s.0).collect();
        assert_eq!(skipped_ids, vec![5, 6, 8]);
        assert!(skipped[0].1.contains("CC") && skipped[0].1.contains("Apache"));

        // 引擎无条件日志：告警规则跳过
        let (norm, skipped) = normalize(&rules, &sites, "OpenLiteSpeed", false);
        assert_eq!(norm.iter().map(|r| r.id).collect::<Vec<_>>(), vec![1, 2]);
        assert!(skipped
            .iter()
            .any(|(id, why)| *id == 3 && why.contains("action=block")));
    }

    #[test]
    fn host_and_tag_helpers() {
        assert_eq!(normalize_host("A.Example.com:8080"), "a.example.com");
        assert_eq!(normalize_host("a.example.com"), "a.example.com");
        assert_eq!(normalize_host("[::1]:80"), "::1");
        assert_eq!(parse_rule_tags("r1:- r2:1 r3: x:1 rX:1 r10:1"), vec![2, 10]);
    }
}
