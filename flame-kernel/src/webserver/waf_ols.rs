//! WAF → OpenLiteSpeed 原生 rewrite 规则（Doc/30 v1.1 · 分发见 [`super::waf_engines`]）。
//!
//! OLS 的 vhost 配置不支持通用 include，也没有条件日志 / 表达式 IP 匹配
//! （`RewriteCond expr "-R …"` 仅 LSWS 企业版 6.2+ 支持），因此：
//!
//! - 编译产物是**按站点分段的规则库** `conf/flamepanel/waf/vhost.rules`
//!   （`# @default` 段 = 全局规则；`# @site <domain>` 段 = 全局 + 该站点规则）。
//! - `WebServerManager` 在部署站点 / 下发 WAF 时把对应段**拼接**进站点 `vhconf.conf` 的
//!   `rewrite` 规则最前面（带 `# >>> flamepanel:waf` / `# <<< flamepanel:waf` 标记，可精确替换 /
//!   移除；站点已有伪静态 `rules <<<…` 时插入其中，否则新建 `rewrite {}` 块）。
//! - 每条阻断规则：`RewriteCond` 匹配 + 全部白名单取反 `RewriteCond … !…` → `RewriteRule ^ - [F]`
//!   （403；OLS 遇到 `[F]` 即结束规则处理）。IP / CIDR 编译为精确的 `%{REMOTE_ADDR}` 正则。
//! - 仅记录（告警）规则与 CC 规则不下发（原因写入 `skipped`）；命中日志不回灌。
//!
//! 安全：字面量一律 `\xHH`（正则里不出现空格 / 引号 / 换行），无法突破一行一条指令的语法，
//! 也无法提前结束 heredoc。

use super::waf_engines::{normalize, re_literal, re_signatures, Cond, Kind, NormRule, HEADER_NOTE};
use super::waf_nginx::{CompiledWaf, WafSite};
use crate::domain::entity::WafRule;

pub const BEGIN: &str = "# >>> flamepanel:waf";
pub const END: &str = "# <<< flamepanel:waf";
const DEFAULT_SECTION: &str = "# @default";
const SITE_SECTION: &str = "# @site ";
/// 新建 rewrite 块时使用的 heredoc 结束标记（与伪静态模板的 `END_REWRITE_RULES` 区分）。
const HEREDOC: &str = "END_FP_WAF_RULES";

/// 单个八位组范围 [lo, hi] → 正则交替。
fn octet_range(lo: u8, hi: u8) -> String {
    if lo == 0 && hi == 255 {
        return "[0-9]{1,3}".into();
    }
    if lo == hi {
        return lo.to_string();
    }
    let alts: Vec<String> = (lo..=hi).map(|v| v.to_string()).collect();
    format!("(?:{})", alts.join("|"))
}

/// `a.b.c.d/p` → 精确匹配该网段的正则（不含锚点）。输入已由 `ip_items` 校验。
pub fn cidr_regex(cidr: &str) -> Option<String> {
    let (ip, prefix) = cidr.split_once('/')?;
    let ip: std::net::Ipv4Addr = ip.parse().ok()?;
    let prefix: u32 = prefix.parse().ok().filter(|p| *p <= 32)?;
    let o = ip.octets();
    let mut parts = Vec::with_capacity(4);
    for (i, octet) in o.iter().enumerate() {
        let bits_before = (i as u32) * 8;
        let bits_here = prefix.saturating_sub(bits_before).min(8);
        let mask: u8 = if bits_here == 0 {
            0
        } else {
            (0xffu16 << (8 - bits_here)) as u8
        };
        let lo = octet & mask;
        let hi = lo | !mask;
        parts.push(octet_range(lo, hi));
    }
    Some(parts.join("\\."))
}

fn ips_regex(items: &[String]) -> String {
    let alts: Vec<String> = items.iter().filter_map(|c| cidr_regex(c)).collect();
    format!("^(?:{})$", alts.join("|"))
}

/// 规则的匹配条件行（多行时为 OR 组合，最后一行不带 OR）。
fn match_lines(r: &NormRule) -> Vec<String> {
    match &r.cond {
        Cond::Ips(items) => vec![format!("RewriteCond %{{REMOTE_ADDR}} {}", ips_regex(items))],
        Cond::Ua(s) => vec![format!(
            "RewriteCond %{{HTTP_USER_AGENT}} {} [NC]",
            re_literal(s)
        )],
        Cond::Path(s) => vec![format!("RewriteCond %{{REQUEST_URI}} ^{}", re_literal(s))],
        Cond::Sig(sigs) => {
            let re = re_signatures(sigs);
            vec![
                format!("RewriteCond %{{REQUEST_URI}} {re} [NC,OR]"),
                format!("RewriteCond %{{QUERY_STRING}} {re} [NC]"),
            ]
        }
    }
}

/// 白名单规则的取反条件。
fn negated_line(r: &NormRule) -> Option<String> {
    match &r.cond {
        Cond::Ips(items) => Some(format!(
            "RewriteCond %{{REMOTE_ADDR}} !{}",
            ips_regex(items)
        )),
        Cond::Ua(s) => Some(format!(
            "RewriteCond %{{HTTP_USER_AGENT}} !{} [NC]",
            re_literal(s)
        )),
        _ => None,
    }
}

/// 某站点（`None` = 未单独分段的站点，只含全局规则）的规则文本。
fn rules_for(norm: &[NormRule], domain: Option<&str>) -> String {
    let applies = |r: &&NormRule| match domain {
        Some(d) => r.applies_to(d),
        None => r.scope.is_none(),
    };
    let whitelist: Vec<&NormRule> = norm
        .iter()
        .filter(applies)
        .filter(|r| r.kind == Kind::Whitelist)
        .collect();
    let mut out = String::new();
    for r in norm
        .iter()
        .filter(applies)
        .filter(|r| r.kind == Kind::Block)
    {
        out.push_str(&r.comment());
        out.push('\n');
        for l in match_lines(r) {
            out.push_str(&l);
            out.push('\n');
        }
        for w in &whitelist {
            if let Some(l) = negated_line(w) {
                out.push_str(&l);
                out.push('\n');
            }
        }
        out.push_str("RewriteRule ^ - [F]\n");
    }
    out
}

/// 编译为分段规则库（`server_conf`）；`http_conf` 为空。
pub fn compile(rules: &[WafRule], sites: &[WafSite]) -> CompiledWaf {
    let (norm, skipped) = normalize(rules, sites, "OpenLiteSpeed", false);
    let mut out = format!("# Managed by Flamepanel WAF (webserver::waf_ols). {HEADER_NOTE}\n");
    out.push_str(DEFAULT_SECTION);
    out.push('\n');
    out.push_str(&rules_for(&norm, None));
    let mut domains: Vec<String> = sites
        .iter()
        .flat_map(|s| s.domains.iter().map(|d| d.to_ascii_lowercase()))
        .collect();
    domains.sort();
    domains.dedup();
    for d in domains {
        out.push_str(&format!("{SITE_SECTION}{d}\n"));
        out.push_str(&rules_for(&norm, Some(&d)));
    }
    // 白名单只作为取反条件出现；没有阻断规则时白名单不产生任何行，但仍算已下发
    CompiledWaf {
        http_conf: String::new(),
        server_conf: out,
        enforced: norm.iter().map(|r| r.id).collect(),
        skipped,
    }
}

/// 从规则库取站点的段（无单独分段 → `@default`；规则库缺失 → 空）。
pub fn section_for(library: &str, domain: &str) -> String {
    #[derive(PartialEq)]
    enum Sec {
        Preamble,
        Default,
        Wanted,
        Other,
    }
    let want = format!("{SITE_SECTION}{}", domain.to_ascii_lowercase());
    let (mut default, mut site) = (String::new(), None::<String>);
    let mut sec = Sec::Preamble;
    for line in library.lines() {
        if line == DEFAULT_SECTION {
            sec = Sec::Default;
            continue;
        }
        if line.starts_with(SITE_SECTION) {
            sec = if line == want {
                Sec::Wanted
            } else {
                Sec::Other
            };
            if sec == Sec::Wanted {
                site.get_or_insert_with(String::new);
            }
            continue;
        }
        let buf = match sec {
            Sec::Default => &mut default,
            Sec::Wanted => site.get_or_insert_with(String::new),
            Sec::Preamble | Sec::Other => continue,
        };
        buf.push_str(line);
        buf.push('\n');
    }
    site.unwrap_or(default)
}

/// 规则库中单独分段的站点域名（部署时据此逐站点重新拼接）。
pub fn site_domains(library: &str) -> Vec<String> {
    library
        .lines()
        .filter_map(|l| l.strip_prefix(SITE_SECTION))
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .collect()
}

/// 移除 vhconf 中的 WAF 标记块（未拼接过时原样返回；新建的空 `rewrite {}` 块整体移除）。
pub fn strip(vhconf: &str) -> String {
    let mut out = String::with_capacity(vhconf.len());
    let mut skipping = false;
    for line in vhconf.split_inclusive('\n') {
        let t = line.trim();
        if skipping {
            if t == END {
                skipping = false;
            }
            continue;
        }
        if t == BEGIN {
            skipping = true;
            continue;
        }
        out.push_str(line);
    }
    out
}

/// 把站点段拼接进 vhconf（先移除旧块；`rules` 为空时只移除）。
///
/// - 已有 `rewrite { … rules <<<TAG … }`：插在 heredoc 首行之后（WAF 先于伪静态执行）。
/// - 已有 `rewrite {` 但无 `rules`：在其后插入 `rules <<<END_FP_WAF_RULES … END_FP_WAF_RULES`。
/// - 没有 rewrite 块：末尾追加 `rewrite { enable 1; rules <<<… }`。
pub fn splice(vhconf: &str, rules: &str) -> String {
    let base = strip(vhconf);
    if rules.trim().is_empty() {
        return base;
    }
    let block_rules = format!("{BEGIN}\n{rules}{END}\n");
    let lines: Vec<&str> = base.split_inclusive('\n').collect();
    let is_rewrite_open = |l: &str| {
        let t = l.trim();
        t.strip_prefix("rewrite")
            .is_some_and(|rest| rest.trim() == "{")
    };
    if let Some(rw) = lines.iter().position(|l| is_rewrite_open(l)) {
        // rewrite 块内（按花括号深度）找 `rules <<<TAG`
        let mut depth = 0i32;
        let mut heredoc_at = None;
        for (i, l) in lines.iter().enumerate().skip(rw) {
            let t = l.trim();
            if i > rw && t.starts_with("rules") && t.contains("<<<") {
                heredoc_at = Some(i);
                break;
            }
            depth += t.matches('{').count() as i32 - t.matches('}').count() as i32;
            if depth <= 0 && i > rw {
                break;
            }
        }
        let mut out = String::with_capacity(base.len() + block_rules.len() + 64);
        for (i, l) in lines.iter().enumerate() {
            out.push_str(l);
            if !l.ends_with('\n') {
                out.push('\n');
            }
            match heredoc_at {
                Some(h) if i == h => out.push_str(&block_rules),
                None if i == rw => out.push_str(&format!(
                    "{BEGIN}\n  rules                   <<<{HEREDOC}\n{rules}{HEREDOC}\n{END}\n"
                )),
                _ => {}
            }
        }
        return out;
    }
    let mut out = base;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(
        "{BEGIN}\nrewrite  {{\n  enable                  1\n  rules                   <<<{HEREDOC}\n{rules}{HEREDOC}\n}}\n{END}\n"
    ));
    out
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

    fn re_match(re: &str, s: &str) -> bool {
        // 测试里用 regex crate 语义校验 CIDR 正则的正确性（无 `\x` / 回溯差异）
        regex::Regex::new(re).unwrap().is_match(s)
    }

    #[test]
    fn cidr_regex_is_exact() {
        let re = format!("^(?:{})$", cidr_regex("10.9.0.0/16").unwrap());
        assert!(re_match(&re, "10.9.0.1") && re_match(&re, "10.9.255.255"));
        assert!(!re_match(&re, "10.10.0.1") && !re_match(&re, "110.9.0.1"));
        let re = format!("^(?:{})$", cidr_regex("192.168.1.77/32").unwrap());
        assert!(re_match(&re, "192.168.1.77") && !re_match(&re, "192.168.1.7"));
        assert!(!re_match(&re, "192.168.1.770"));
        let re = format!("^(?:{})$", cidr_regex("172.16.0.0/12").unwrap());
        assert!(re_match(&re, "172.16.0.1") && re_match(&re, "172.31.9.9"));
        assert!(!re_match(&re, "172.32.0.1") && !re_match(&re, "172.15.0.1"));
        let re = format!("^(?:{})$", cidr_regex("0.0.0.0/0").unwrap());
        assert!(re_match(&re, "8.8.8.8"));
        // 非对齐主机位按网段归零
        assert_eq!(
            cidr_regex("10.1.2.3/24").unwrap(),
            "10\\.1\\.2\\.[0-9]{1,3}"
        );
        assert!(cidr_regex("bogus").is_none());
    }

    #[test]
    fn compiles_sections_with_whitelist_negation() {
        let rules = vec![
            rule(1, "ip_blacklist", "10.9.0.0/16", "block", 0),
            rule(2, "path_block", "/admin", "block", 7),
            rule(3, "ua_whitelist", "GoodBot", "block", 0),
            rule(4, "ip_whitelist", "127.0.0.3", "block", 7),
            rule(5, "sql_injection", "", "block", 0),
            rule(6, "xss", "", "log", 0),
            rule(8, "cc", "", "block", 0),
        ];
        let c = compile(&rules, &[site(7, "t1.test"), site(9, "b.test")]);
        assert!(c.http_conf.is_empty());
        assert_eq!(c.enforced, vec![1, 2, 3, 4, 5]);
        assert_eq!(
            c.skipped.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![6, 8]
        );

        let t1 = section_for(&c.server_conf, "T1.test");
        assert!(t1.contains("# rule 2 [path_block]"));
        assert!(t1.contains("RewriteCond %{REQUEST_URI} ^\\x2fadmin\n"));
        // 站点级 IP 白名单只在该站点段出现
        assert!(t1.contains("RewriteCond %{REMOTE_ADDR} !^(?:127\\.0\\.0\\.3)$\n"));
        assert!(t1.contains("RewriteCond %{HTTP_USER_AGENT} !GoodBot [NC]\n"));
        assert!(t1.contains("[NC,OR]\nRewriteCond %{QUERY_STRING} (?:"));
        assert_eq!(t1.matches("RewriteRule ^ - [F]").count(), 3);

        let b = section_for(&c.server_conf, "b.test");
        assert!(!b.contains("rule 2") && !b.contains("127\\.0\\.0\\.3"));
        assert_eq!(b.matches("RewriteRule ^ - [F]").count(), 2);
        // 未分段站点 → 默认段（仅全局规则）
        let other = section_for(&c.server_conf, "unknown.test");
        assert_eq!(other, b);
        assert!(!other.contains("Managed by"));
        assert_eq!(section_for("", "t1.test"), "");
        assert_eq!(site_domains(&c.server_conf), vec!["b.test", "t1.test"]);
        // 一行一条指令：规则字面量里没有空格
        for l in t1.lines().filter(|l| l.starts_with("RewriteCond")) {
            let n = l.split_whitespace().count();
            assert!(n == 3 || n == 4, "{l}");
        }
    }

    #[test]
    fn splice_into_existing_heredoc_and_is_reversible() {
        let vh = "docRoot $VH_ROOT/\n\nrewrite {\n    enable 1\n    rules <<<END_REWRITE_RULES\nRewriteRule ^(.*)$ index.php [L]\nEND_REWRITE_RULES\n}\n";
        let rules =
            "# rule 1 [path_block] x\nRewriteCond %{REQUEST_URI} ^\\x2fa\nRewriteRule ^ - [F]\n";
        let out = splice(vh, rules);
        let waf_at = out.find(BEGIN).unwrap();
        assert!(waf_at > out.find("<<<END_REWRITE_RULES").unwrap());
        assert!(waf_at < out.find("RewriteRule ^(.*)$").unwrap());
        assert_eq!(out.matches("rewrite {").count(), 1);
        // 幂等：重复拼接不叠加
        assert_eq!(splice(&out, rules), out);
        // 空规则 → 恢复原样
        assert_eq!(splice(&out, ""), vh);
        assert_eq!(strip(&out), vh);
    }

    #[test]
    fn splice_creates_rewrite_block_when_absent() {
        let vh = "docRoot $VH_ROOT/\nindex  {\n  useServer 0\n}\n";
        let out = splice(vh, "RewriteRule ^ - [F]\n");
        assert!(out.starts_with(vh));
        assert!(out.contains("rewrite  {\n  enable                  1\n  rules                   <<<END_FP_WAF_RULES\nRewriteRule ^ - [F]\nEND_FP_WAF_RULES\n}\n"));
        assert_eq!(splice(&out, ""), vh);

        // rewrite 块存在但没有 rules：在块内新建 heredoc
        let vh2 = "rewrite {\n  enable 1\n  autoLoadHtaccess 1\n}\n";
        let out2 = splice(vh2, "RewriteRule ^ - [F]\n");
        assert!(out2.starts_with("rewrite {\n# >>> flamepanel:waf\n  rules"));
        assert_eq!(out2.matches("rewrite").count(), 1);
        assert_eq!(splice(&out2, ""), vh2);
    }
}
