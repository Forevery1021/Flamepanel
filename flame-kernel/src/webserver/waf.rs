//! WAF 规则引擎（主线三 §6.2 · WAF 基础防护 · Doc/20）。
//!
//! 引擎与引擎适配分离：本模块只做**请求特征评估**（纯逻辑，可离线单测），
//! 与具体 Web 服务器（Nginx/OpenResty/Caddy 等）的配置落地解耦。
//!
//! 首期规则集（对标 1Panel 社区版/宝塔免费层）：
//! - CC 频率限制（固定窗口计数，对齐 `api/rate_limiter.rs` 思想）
//! - SQL 注入 / XSS 特征（OWASP CRS 精选特征子集）
//! - IP / UA 黑白名单、路径封禁
//!
//! 默认动作 `log`：仅告警不阻断（§6.2 风险对策——误拦截优先可观测，
//! 观察期后一键切阻断）；白名单优先级最高（命中白名单直接放行）。
use crate::domain::entity::WafRule;
use crate::utils::sync::lock_or_recover;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// 一次请求的入站特征（从 HTTP 请求提取，引擎只依赖此快照，不依赖具体框架）。
#[derive(Debug, Clone, Default)]
pub struct WafRequest {
    pub method: String,
    pub path: String,
    /// 查询串（不含 `?`）
    pub query: String,
    pub client_ip: String,
    pub user_agent: String,
}

/// 评估结论。
#[derive(Debug, Clone, PartialEq)]
pub enum WafVerdict {
    /// 放行
    Allow,
    /// 命中规则，仅告警（action = log）
    Warn {
        rule_id: i64,
        rule_name: String,
        rule_type: String,
    },
    /// 命中规则，阻断（action = block）
    Block {
        rule_id: i64,
        rule_name: String,
        rule_type: String,
    },
}

/// SQL 注入特征（OWASP CRS 精选子集，小写匹配请求行）。
pub const SQLI_PATTERNS: [&str; 14] = [
    "' or '1'='1",
    "' or 1=1",
    "\" or \"1\"=\"1",
    "' or 'a'='a",
    "union select",
    "union all select",
    "drop table",
    "insert into",
    "select * from",
    "information_schema",
    "sleep(",
    "benchmark(",
    "load_file(",
    ";--",
];

/// XSS 特征（OWASP CRS 精选子集，小写匹配请求行）。
pub const XSS_PATTERNS: [&str; 12] = [
    "<script",
    "</script",
    "javascript:",
    "onerror=",
    "onload=",
    "onclick=",
    "<iframe",
    "<svg onload",
    "<img src=x",
    "document.cookie",
    "eval(",
    "alert(",
];

/// IP 是否命中 CIDR（支持 `1.2.3.4` 与 `10.0.0.0/8` 两种写法；IPv4）。
pub fn ip_matches_cidr(ip: &str, cidr: &str) -> bool {
    let (net, prefix) = match cidr.split_once('/') {
        Some((n, p)) => (n, p.parse::<u32>().unwrap_or(32)),
        None => (cidr, 32),
    };
    let (Some(ip_u), Some(net_u)) = (ipv4_to_u32(ip), ipv4_to_u32(net)) else {
        return false;
    };
    if prefix == 0 {
        return true;
    }
    if prefix > 32 {
        return false;
    }
    let mask = u32::MAX << (32 - prefix);
    (ip_u & mask) == (net_u & mask)
}

fn ipv4_to_u32(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut out: u32 = 0;
    for p in parts {
        let oct: u8 = p.parse().ok()?;
        out = (out << 8) | oct as u32;
    }
    Some(out)
}

/// UA 匹配（子串，大小写不敏感）。
fn ua_matches(ua: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    ua.to_lowercase().contains(&pattern.to_lowercase())
}

/// 路径匹配（前缀匹配，大小写敏感；pattern 即路径前缀如 `/wp-admin`）。
fn path_matches(path: &str, pattern: &str) -> bool {
    let p = pattern.trim();
    if p.is_empty() {
        return false;
    }
    path.starts_with(p)
}

/// 请求行（method + path + query）小写拼接，供特征匹配。
fn request_line(req: &WafRequest) -> String {
    let mut line = format!("{} {}", req.method, req.path);
    if !req.query.is_empty() {
        line.push('?');
        line.push_str(&req.query);
    }
    line.to_lowercase()
}

/// CC 计数器：固定窗口（对齐 rate_limiter 思想；进程内，单实例面板够用）。
#[derive(Default)]
pub struct CcCounter {
    /// key: (rule_id, client_ip, window_start_instant) → count
    windows: Mutex<HashMap<(i64, String, u64), u32>>,
}

impl CcCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记一次请求并返回窗口内累计次数。
    fn hit(&self, rule_id: i64, ip: &str, window_secs: u64) -> u32 {
        // 固定窗口按「进程启动以来的秒数」分槽。旧实现用 `Instant::now().elapsed()`（恒≈0），
        // slot 永远为 0、窗口从不重置——超过阈值后该 IP 被永久判定超限。
        static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let base = *BASE.get_or_init(Instant::now);
        let slot = base.elapsed().as_secs() / window_secs.max(1);
        let mut windows = lock_or_recover(&self.windows);
        // 简单自愈：条目过多时整体清空（窗口语义按绝对 slot 计算，清空安全）
        if windows.len() > 100_000 {
            windows.clear();
        }
        let key = (rule_id, ip.to_string(), slot);
        let count = windows.entry(key).or_insert(0);
        *count += 1;
        *count
    }
}

/// WAF 引擎：持有启用规则集 + CC 计数器，对入站请求做评估。
///
/// 白名单优先级最高 → 黑名单/特征 → CC 限频；同一优先级内先命中先生效。
pub struct WafEngine {
    rules: Vec<WafRule>,
    cc: CcCounter,
}

impl WafEngine {
    pub fn new(rules: Vec<WafRule>) -> Self {
        Self {
            rules,
            cc: CcCounter::new(),
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 评估一个请求（全部启用规则，不区分网站作用域）。
    pub fn evaluate(&self, req: &WafRequest) -> WafVerdict {
        self.evaluate_scoped(None, req)
    }

    /// 按网站评估：只应用全局规则（`website_id = 0`）与该网站的规则。
    ///
    /// 与 Nginx 下发（`waf_nginx`）的作用域语义一致——此前 `inspect(website_id, …)` 忽略了
    /// 作用域，网站级规则会误伤其它网站。
    pub fn evaluate_for_site(&self, website_id: i64, req: &WafRequest) -> WafVerdict {
        self.evaluate_scoped(Some(website_id), req)
    }

    fn evaluate_scoped(&self, website_id: Option<i64>, req: &WafRequest) -> WafVerdict {
        let enabled = self.rules.iter().filter(|r| {
            r.enabled && website_id.is_none_or(|w| r.website_id == 0 || r.website_id == w)
        });
        let line = request_line(req);

        // ① 白名单最高优先级：命中即放行
        for r in enabled.clone() {
            match r.rule_type.as_str() {
                "ip_whitelist" if self.pattern_hit(r, req) => return WafVerdict::Allow,
                "ua_whitelist" if ua_matches(&req.user_agent, &r.pattern) => {
                    return WafVerdict::Allow
                }
                _ => {}
            }
        }

        // ② 黑名单 / 特征 / 路径封禁
        for r in enabled.clone() {
            let hit = match r.rule_type.as_str() {
                "ip_blacklist" => self.pattern_hit(r, req),
                "ua_blacklist" => ua_matches(&req.user_agent, &r.pattern),
                "path_block" => path_matches(&req.path, &r.pattern),
                "sql_injection" => SQLI_PATTERNS.iter().any(|p| line.contains(p)),
                "xss" => XSS_PATTERNS.iter().any(|p| line.contains(p)),
                _ => false,
            };
            if hit {
                return verdict_of(r);
            }
        }

        // ③ CC 频率限制（漏桶思想简化为固定窗口）
        for r in enabled {
            if r.rule_type != "cc" {
                continue;
            }
            let (threshold, window_secs) = r.cc_params();
            let count = self.cc.hit(r.id, &req.client_ip, window_secs);
            if count > threshold {
                return verdict_of(r);
            }
        }

        WafVerdict::Allow
    }

    /// IP 类规则匹配：支持 CIDR 与精确 IP；pattern 允许逗号分隔多个值。
    fn pattern_hit(&self, rule: &WafRule, req: &WafRequest) -> bool {
        rule.pattern
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .any(|p| ip_matches_cidr(&req.client_ip, p))
    }
}

fn verdict_of(rule: &WafRule) -> WafVerdict {
    let (rule_id, rule_name, rule_type) = (rule.id, rule.name.clone(), rule.rule_type.clone());
    if rule.action == "block" {
        WafVerdict::Block {
            rule_id,
            rule_name,
            rule_type,
        }
    } else {
        WafVerdict::Warn {
            rule_id,
            rule_name,
            rule_type,
        }
    }
}

#[cfg(test)]
mod waf_engine_tests {
    use super::*;

    fn req(ip: &str, ua: &str, path: &str, query: &str) -> WafRequest {
        WafRequest {
            method: "GET".into(),
            path: path.into(),
            query: query.into(),
            client_ip: ip.into(),
            user_agent: ua.into(),
        }
    }

    fn rule(id: i64, rule_type: &str, pattern: &str, action: &str, website_id: i64) -> WafRule {
        WafRule {
            id,
            name: format!("rule-{rule_type}-{id}"),
            rule_type: rule_type.into(),
            website_id,
            pattern: pattern.into(),
            config_json: "{}".into(),
            action: action.into(),
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn site_scoped_rules_only_hit_their_site() {
        let engine = WafEngine::new(vec![
            rule(1, "ip_blacklist", "1.1.1.1", "block", 7),
            rule(2, "ua_blacklist", "badbot", "block", 0),
        ]);
        let r = req("1.1.1.1", "curl", "/", "");
        assert!(matches!(
            engine.evaluate_for_site(7, &r),
            WafVerdict::Block { rule_id: 1, .. }
        ));
        assert_eq!(engine.evaluate_for_site(8, &r), WafVerdict::Allow);
        // 全局规则对所有网站生效
        let b = req("2.2.2.2", "BadBot/1.0", "/", "");
        assert!(matches!(
            engine.evaluate_for_site(8, &b),
            WafVerdict::Block { rule_id: 2, .. }
        ));
    }

    #[test]
    fn cc_window_resets() {
        let c = CcCounter::new();
        // 1 秒窗口：同一 slot 内累计；跨过窗口后重新计数（旧实现 slot 恒为 0，永不重置）
        let first = c.hit(1, "1.1.1.1", 1);
        assert_eq!(first, 1);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(c.hit(1, "1.1.1.1", 1), 1);
    }

    #[test]
    fn no_rules_allows() {
        let engine = WafEngine::new(vec![]);
        assert_eq!(
            engine.evaluate(&req("1.2.3.4", "curl", "/", "")),
            WafVerdict::Allow
        );
    }

    #[test]
    fn whitelist_beats_blacklist() {
        let engine = WafEngine::new(vec![
            rule(1, "ip_blacklist", "10.0.0.0/8", "block", 0),
            rule(2, "ip_whitelist", "10.1.1.1", "log", 0),
        ]);
        // 白名单 IP 命中 → 放行（虽然也在黑名单段内）
        assert_eq!(
            engine.evaluate(&req("10.1.1.1", "curl", "/", "")),
            WafVerdict::Allow
        );
        // 段内其他 IP → 黑名单命中
        assert!(matches!(
            engine.evaluate(&req("10.2.0.1", "curl", "/", "")),
            WafVerdict::Block { rule_id: 1, .. }
        ));
    }

    #[test]
    fn cidr_and_multi_ip_pattern() {
        let engine = WafEngine::new(vec![rule(
            1,
            "ip_blacklist",
            "192.168.1.0/24, 172.16.0.5",
            "block",
            0,
        )]);
        assert!(matches!(
            engine.evaluate(&req("192.168.1.99", "", "/", "")),
            WafVerdict::Block { .. }
        ));
        assert!(matches!(
            engine.evaluate(&req("172.16.0.5", "", "/", "")),
            WafVerdict::Block { .. }
        ));
        assert_eq!(
            engine.evaluate(&req("192.168.2.1", "", "/", "")),
            WafVerdict::Allow
        );
        // 非法 CIDR 不命中
        assert_eq!(
            engine.evaluate(&req("not-an-ip", "", "/", "")),
            WafVerdict::Allow
        );
    }

    #[test]
    fn sqli_and_xss_detection() {
        let engine = WafEngine::new(vec![
            rule(1, "sql_injection", "", "block", 0),
            rule(2, "xss", "", "log", 0),
        ]);
        // SQL 注入（query）
        assert!(matches!(
            engine.evaluate(&req("1.1.1.1", "", "/login", "id=1' OR '1'='1")),
            WafVerdict::Block { rule_id: 1, .. }
        ));
        // SQL 注入（大小写不敏感）
        assert!(matches!(
            engine.evaluate(&req("1.1.1.1", "", "/x", "q=UNION SELECT *")),
            WafVerdict::Block { .. }
        ));
        // XSS（log 动作 → Warn）
        assert!(matches!(
            engine.evaluate(&req("1.1.1.1", "", "/search", "q=<script>alert(1)")),
            WafVerdict::Warn { rule_id: 2, .. }
        ));
        // 正常请求放行
        assert_eq!(
            engine.evaluate(&req("1.1.1.1", "", "/about", "page=2")),
            WafVerdict::Allow
        );
    }

    #[test]
    fn ua_and_path_rules() {
        let engine = WafEngine::new(vec![
            rule(1, "ua_blacklist", "sqlmap", "block", 0),
            rule(2, "path_block", "/wp-admin", "block", 0),
        ]);
        assert!(matches!(
            engine.evaluate(&req("1.1.1.1", "sqlmap/1.5", "/", "")),
            WafVerdict::Block { rule_id: 1, .. }
        ));
        assert!(matches!(
            engine.evaluate(&req("1.1.1.1", "Mozilla", "/wp-admin/setup", "")),
            WafVerdict::Block { rule_id: 2, .. }
        ));
        assert_eq!(
            engine.evaluate(&req("1.1.1.1", "Mozilla", "/app", "")),
            WafVerdict::Allow
        );
    }

    #[test]
    fn cc_rate_limit_triggers_after_threshold() {
        let mut r = rule(1, "cc", "", "block", 0);
        r.config_json = r#"{"threshold":3,"window_secs":60}"#.into();
        let engine = WafEngine::new(vec![r]);
        let ip = "5.5.5.5";
        for _ in 0..3 {
            assert_eq!(engine.evaluate(&req(ip, "", "/", "")), WafVerdict::Allow);
        }
        // 第 4 次超阈值
        assert!(matches!(
            engine.evaluate(&req(ip, "", "/", "")),
            WafVerdict::Block { rule_id: 1, .. }
        ));
        // 其他 IP 不受影响
        assert_eq!(
            engine.evaluate(&req("6.6.6.6", "", "/", "")),
            WafVerdict::Allow
        );
    }

    #[test]
    fn disabled_rules_ignored() {
        let mut r = rule(1, "ip_blacklist", "1.2.3.4", "block", 0);
        r.enabled = false;
        let engine = WafEngine::new(vec![r]);
        assert_eq!(
            engine.evaluate(&req("1.2.3.4", "", "/", "")),
            WafVerdict::Allow
        );
    }

    #[test]
    fn ip_cidr_util_cases() {
        assert!(ip_matches_cidr("10.0.0.1", "10.0.0.1"));
        assert!(ip_matches_cidr("10.1.2.3", "10.0.0.0/8"));
        assert!(!ip_matches_cidr("11.0.0.1", "10.0.0.0/8"));
        assert!(ip_matches_cidr("8.8.8.8", "0.0.0.0/0"));
        assert!(!ip_matches_cidr("8.8.8.8", "0.0.0.0/33"));
        assert!(!ip_matches_cidr("abc", "10.0.0.0/8"));
    }
}
