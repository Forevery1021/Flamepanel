//! 面板设置项目录与校验（W4-a）。
//!
//! 此前 `PUT /api/settings` / `PATCH /api/settings/batch` 对任意键值照单全收：端口可以写成
//! `abc`、会话超时可以写成负数，而且大部分设置项（会话超时、日志级别、日志保留）写进库后
//! 没有任何代码读取——设置页上的开关是「装饰」。本模块把已知设置项登记为**类型化目录**：
//!
//! - 已知键按类型校验并规范化（布尔 / 整数区间 / 枚举 / IP 与 CIDR 列表 / 域名列表 / 文本）；
//! - **保留键**（签名密钥、内部游标）禁止经通用设置接口读写——旧版「轮换密钥」按钮把随机串
//!   写进 `jwt_secret` 设置项，既没有真正轮换，又让持有 `settings:read` 的账号能读到它；
//! - 未登记的键保持兼容（插件 / 旧前端写入的自定义键），只限制键名字符集与值长度。
//!
//! 生效策略（[`PanelPolicy`]）由设置快照一次性派生，供访问控制中间件、登录锁定与会话签发
//! 在热路径上无锁读取（见 `SettingsService::policy`）。

use std::collections::HashMap;
use std::net::IpAddr;

/// 值类型（决定校验与规范化方式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Enum(&'static [&'static str]),
    Text {
        max_len: usize,
    },
    /// 逗号 / 换行分隔的 IPv4 / IPv6 地址或 CIDR，空 = 不限制
    IpList,
    /// 逗号 / 换行分隔的域名（可带 `*.` 通配前缀），空 = 不限制
    DomainList,
    /// JSON 字符串数组
    JsonStringArray,
    /// 图片 data URL（或 http(s) URL）/ 空
    Image,
}

/// 已登记的设置项。
#[derive(Debug, Clone, Copy)]
pub struct SettingSpec {
    pub key: &'static str,
    pub kind: SettingKind,
    /// 修改后需要重启面板进程才生效（前端据此提示）
    pub restart_required: bool,
}

const LOG_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];
const LANGUAGES: &[&str] = &["zh-CN", "en-US", "ja-JP"];
const THEMES: &[&str] = &["light", "dark", "auto"];
const SIGNATURE_POLICIES: &[&str] = &["optional", "required"];

/// 背景图 data URL 上限（前端压缩到 ≤ 500 KB，base64 膨胀后留足余量）
const MAX_IMAGE_LEN: usize = 1024 * 1024;
/// 未登记键的值上限
const MAX_GENERIC_LEN: usize = 64 * 1024;
/// 列表类设置的条目上限
const MAX_LIST_ITEMS: usize = 64;

pub const SPECS: &[SettingSpec] = &[
    spec("panel_name", SettingKind::Text { max_len: 64 }),
    spec("theme", SettingKind::Enum(THEMES)),
    spec("language", SettingKind::Enum(LANGUAGES)),
    SettingSpec {
        key: "panel_port",
        kind: SettingKind::Int { min: 1, max: 65535 },
        restart_required: true,
    },
    spec(
        "session_timeout_minutes",
        SettingKind::Int {
            min: 5,
            max: 43_200,
        },
    ),
    spec("log_level", SettingKind::Enum(LOG_LEVELS)),
    spec("log_retention_days", SettingKind::Int { min: 1, max: 3650 }),
    spec(
        "audit_log_retention_days",
        SettingKind::Int { min: 7, max: 3650 },
    ),
    spec("two_factor_enabled", SettingKind::Bool),
    spec("auto_backup_enabled", SettingKind::Bool),
    spec(
        "auto_backup_interval_hours",
        SettingKind::Int { min: 1, max: 720 },
    ),
    spec("backup_retention", SettingKind::Int { min: 1, max: 365 }),
    spec("menu_accordion", SettingKind::Bool),
    spec("menu_collapsed", SettingKind::Bool),
    spec("hide_menu", SettingKind::JsonStringArray),
    spec("app_background", SettingKind::Image),
    spec("login_background", SettingKind::Image),
    spec(
        "app_signature_policy",
        SettingKind::Enum(SIGNATURE_POLICIES),
    ),
    spec(
        "appstore_registry_whitelist",
        SettingKind::Text { max_len: 4096 },
    ),
    // ── W4-a 新增：访问控制与登录防护（参考 1Panel「安全」/ 宝塔「面板设置」） ──
    spec("allowed_ips", SettingKind::IpList),
    spec("bind_domains", SettingKind::DomainList),
    spec("login_max_failures", SettingKind::Int { min: 3, max: 20 }),
    spec(
        "login_lockout_minutes",
        SettingKind::Int { min: 1, max: 1440 },
    ),
    spec("server_address", SettingKind::Text { max_len: 255 }),
];

const fn spec(key: &'static str, kind: SettingKind) -> SettingSpec {
    SettingSpec {
        key,
        kind,
        restart_required: false,
    }
}

/// 查登记表。
pub fn lookup(key: &str) -> Option<&'static SettingSpec> {
    SPECS.iter().find(|s| s.key == key)
}

/// 保留键：只能由专用接口 / 内核内部写入，通用设置接口既不返回也不接受。
pub fn is_reserved(key: &str) -> bool {
    matches!(key, "jwt_secret" | "jwt_secret_persisted")
        || key.starts_with("waf.ingest.")
        // M-AI-3：助手配置与加密 API Key 只经 `/api/assistant/config` 读写
        || key.starts_with("assistant.")
}

/// 校验一对键值，返回规范化后的值；错误信息面向用户、带键名。
pub fn validate(key: &str, value: &str) -> Result<String, String> {
    if is_reserved(key) {
        return Err(format!("{key}: reserved key, use the dedicated API"));
    }
    if !valid_key(key) {
        return Err(format!(
            "{key}: invalid key (1-64 chars of a-z, 0-9, '_', '.', '-')"
        ));
    }
    let Some(spec) = lookup(key) else {
        if value.len() > MAX_GENERIC_LEN {
            return Err(format!(
                "{key}: value too long (max {MAX_GENERIC_LEN} bytes)"
            ));
        }
        return Ok(value.to_string());
    };
    let v = value.trim();
    match spec.kind {
        SettingKind::Bool => match v {
            "true" | "false" => Ok(v.to_string()),
            _ => Err(format!("{key}: expected true or false")),
        },
        SettingKind::Int { min, max } => match v.parse::<i64>() {
            Ok(n) if (min..=max).contains(&n) => Ok(n.to_string()),
            _ => Err(format!("{key}: expected an integer in {min}..={max}")),
        },
        SettingKind::Enum(options) => {
            if options.contains(&v) {
                Ok(v.to_string())
            } else {
                Err(format!("{key}: expected one of {}", options.join(" / ")))
            }
        }
        SettingKind::Text { max_len } => {
            if v.chars().count() > max_len {
                Err(format!("{key}: too long (max {max_len} characters)"))
            } else if v.chars().any(|c| c.is_control()) {
                Err(format!("{key}: control characters are not allowed"))
            } else {
                Ok(v.to_string())
            }
        }
        SettingKind::IpList => {
            let items = split_list(v);
            if items.len() > MAX_LIST_ITEMS {
                return Err(format!("{key}: at most {MAX_LIST_ITEMS} entries"));
            }
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let rule = IpRule::parse(item)
                    .ok_or_else(|| format!("{key}: '{item}' is not an IP address or CIDR"))?;
                out.push(rule.to_string());
            }
            out.dedup();
            Ok(out.join(","))
        }
        SettingKind::DomainList => {
            let items = split_list(v);
            if items.len() > MAX_LIST_ITEMS {
                return Err(format!("{key}: at most {MAX_LIST_ITEMS} entries"));
            }
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let d = item.to_ascii_lowercase();
                if !valid_domain_pattern(&d) {
                    return Err(format!("{key}: '{item}' is not a valid domain"));
                }
                out.push(d);
            }
            out.dedup();
            Ok(out.join(","))
        }
        SettingKind::JsonStringArray => {
            let parsed: Result<Vec<String>, _> = serde_json::from_str(v);
            match parsed {
                Ok(items) if items.len() <= MAX_LIST_ITEMS => {
                    serde_json::to_string(&items).map_err(|e| format!("{key}: {e}"))
                }
                Ok(_) => Err(format!("{key}: at most {MAX_LIST_ITEMS} entries")),
                Err(_) => Err(format!("{key}: expected a JSON array of strings")),
            }
        }
        SettingKind::Image => {
            if v.is_empty() {
                Ok(String::new())
            } else if v.len() > MAX_IMAGE_LEN {
                Err(format!("{key}: image too large (max 1 MiB)"))
            } else if v.starts_with("data:image/")
                || v.starts_with("https://")
                || v.starts_with("http://")
            {
                Ok(v.to_string())
            } else {
                Err(format!("{key}: expected an image data URL or http(s) URL"))
            }
        }
    }
}

/// 批量校验：一次列出全部错误（与防火墙表单同口径，避免用户逐个试错）。
pub fn validate_all(entries: &[(String, String)]) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::with_capacity(entries.len());
    let mut errors = Vec::new();
    for (k, v) in entries {
        match validate(k, v) {
            Ok(norm) => out.push((k.clone(), norm)),
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors.join("; "))
    }
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        })
}

fn split_list(v: &str) -> Vec<&str> {
    v.split([',', '\n', ';', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

fn valid_domain_pattern(d: &str) -> bool {
    let host = d.strip_prefix("*.").unwrap_or(d);
    if host.is_empty() || host.len() > 253 || host.parse::<IpAddr>().is_ok() {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

/// 单条 IP / CIDR 规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpRule {
    net: IpAddr,
    prefix: u8,
}

impl IpRule {
    pub fn parse(s: &str) -> Option<Self> {
        let (addr, prefix) = match s.split_once('/') {
            Some((a, p)) => (a.trim(), Some(p.trim().parse::<u8>().ok()?)),
            None => (s.trim(), None),
        };
        let ip: IpAddr = addr.parse().ok()?;
        let max = if ip.is_ipv4() { 32 } else { 128 };
        let prefix = prefix.unwrap_or(max);
        if prefix > max {
            return None;
        }
        Some(Self {
            net: mask(ip, prefix),
            prefix,
        })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = canonical(ip);
        match (self.net, ip) {
            (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {
                mask(ip, self.prefix) == self.net
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for IpRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let max = if self.net.is_ipv4() { 32 } else { 128 };
        if self.prefix == max {
            write!(f, "{}", self.net)
        } else {
            write!(f, "{}/{}", self.net, self.prefix)
        }
    }
}

/// IPv4-mapped IPv6（`::ffff:1.2.3.4`）按 IPv4 比较——双栈监听下的对端地址常是这种形式。
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    }
}

fn mask(ip: IpAddr, prefix: u8) -> IpAddr {
    match canonical(ip) {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let m = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(prefix))
            };
            IpAddr::V4((bits & m).into())
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let m = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(prefix))
            };
            IpAddr::V6((bits & m).into())
        }
    }
}

/// 由设置快照派生的运行时策略（热路径只读）。
#[derive(Debug, Clone, PartialEq)]
pub struct PanelPolicy {
    pub allowed_ips: Vec<IpRule>,
    pub bind_domains: Vec<String>,
    pub session_timeout_minutes: u64,
    pub login_max_failures: u32,
    pub login_lockout_minutes: u64,
    /// 设置 `two_factor_enabled`：强制所有用户启用 TOTP 两步验证（未绑定者只能访问绑定相关端点）
    pub require_two_factor: bool,
}

impl Default for PanelPolicy {
    fn default() -> Self {
        Self {
            allowed_ips: Vec::new(),
            bind_domains: Vec::new(),
            session_timeout_minutes: 1440,
            login_max_failures: 5,
            login_lockout_minutes: 5,
            require_two_factor: false,
        }
    }
}

/// 访问控制判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessDecision {
    Allow,
    DenyIp,
    DenyHost,
}

impl PanelPolicy {
    /// 从设置 map 派生；非法值回退缺省（库里的旧脏数据不能让面板拒绝启动或锁死所有人）。
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let d = Self::default();
        let int = |k: &str, lo: i64, hi: i64| -> Option<i64> {
            map.get(k)
                .and_then(|v| v.trim().parse::<i64>().ok())
                .filter(|n| (lo..=hi).contains(n))
        };
        Self {
            allowed_ips: map
                .get("allowed_ips")
                .map(|v| {
                    split_list(v)
                        .into_iter()
                        .filter_map(IpRule::parse)
                        .collect()
                })
                .unwrap_or_default(),
            bind_domains: map
                .get("bind_domains")
                .map(|v| {
                    split_list(v)
                        .into_iter()
                        .map(str::to_ascii_lowercase)
                        .filter(|s| valid_domain_pattern(s))
                        .collect()
                })
                .unwrap_or_default(),
            session_timeout_minutes: int("session_timeout_minutes", 5, 43_200)
                .map(|n| n as u64)
                .unwrap_or(d.session_timeout_minutes),
            login_max_failures: int("login_max_failures", 3, 20)
                .map(|n| n as u32)
                .unwrap_or(d.login_max_failures),
            login_lockout_minutes: int("login_lockout_minutes", 1, 1440)
                .map(|n| n as u64)
                .unwrap_or(d.login_lockout_minutes),
            require_two_factor: map
                .get("two_factor_enabled")
                .map(|v| matches!(v.trim(), "true" | "1"))
                .unwrap_or(false),
        }
    }

    /// 判定一次请求。`local` = TCP 对端是回环且未经反代转发（本机 `fpctl` / 运维脚本），
    /// 始终放行——这是配置写错后的自救通道（`fpctl api PUT /api/settings …`）。
    pub fn check(
        &self,
        client_ip: Option<IpAddr>,
        host: Option<&str>,
        local: bool,
    ) -> AccessDecision {
        if local {
            return AccessDecision::Allow;
        }
        if !self.allowed_ips.is_empty() {
            let ok = client_ip.is_some_and(|ip| self.allowed_ips.iter().any(|r| r.contains(ip)));
            if !ok {
                return AccessDecision::DenyIp;
            }
        }
        if !self.bind_domains.is_empty() {
            let ok = host.map(strip_port).is_some_and(|h| self.host_matches(&h));
            if !ok {
                return AccessDecision::DenyHost;
            }
        }
        AccessDecision::Allow
    }

    fn host_matches(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        self.bind_domains
            .iter()
            .any(|pat| match pat.strip_prefix("*.") {
                Some(suffix) => {
                    host.len() > suffix.len() + 1 && host.ends_with(&format!(".{suffix}"))
                }
                None => *pat == host,
            })
    }
}

/// `Host` 头去端口（兼容 `[::1]:8080` 形式）。
pub fn strip_port(host: &str) -> String {
    let h = host.trim();
    if let Some(rest) = h.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest).to_string();
    }
    match h.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) && !name.contains(':') => {
            name.to_string()
        }
        _ => h.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn typed_values_are_validated_and_normalized() {
        assert_eq!(validate("panel_port", " 8443 ").unwrap(), "8443");
        assert!(validate("panel_port", "0").is_err());
        assert!(validate("panel_port", "abc").is_err());
        assert!(validate("session_timeout_minutes", "4").is_err());
        assert_eq!(validate("log_level", "warn").unwrap(), "warn");
        assert!(validate("log_level", "verbose").is_err());
        assert!(validate("menu_accordion", "yes").is_err());
        assert_eq!(
            validate("hide_menu", r#"["web", "apps"]"#).unwrap(),
            r#"["web","apps"]"#
        );
        assert!(validate("hide_menu", "web").is_err());
        assert!(validate("panel_name", "a\u{7}b").is_err());
        assert!(validate("app_background", "javascript:alert(1)").is_err());
        assert_eq!(validate("app_background", "").unwrap(), "");
    }

    #[test]
    fn reserved_keys_are_rejected() {
        assert!(validate("jwt_secret", "x".repeat(64).as_str()).is_err());
        assert!(validate("jwt_secret_persisted", "x").is_err());
        assert!(validate("waf.ingest.nginx", "1:2").is_err());
        assert!(validate("assistant.api_key", "sk-x").is_err());
        assert!(validate("assistant.config", "{}").is_err());
    }

    #[test]
    fn unknown_keys_stay_compatible_but_bounded() {
        assert_eq!(validate("plugin.foo.bar", "anything").unwrap(), "anything");
        assert!(validate("Bad Key", "x").is_err());
        assert!(validate("k", &"x".repeat(MAX_GENERIC_LEN + 1)).is_err());
    }

    #[test]
    fn ip_list_normalizes_and_rejects_garbage() {
        assert_eq!(
            validate("allowed_ips", "10.0.0.7/8, 192.168.1.5\n2001:db8::1/32").unwrap(),
            "10.0.0.0/8,192.168.1.5,2001:db8::/32"
        );
        assert!(validate("allowed_ips", "10.0.0.0/33").is_err());
        assert!(validate("allowed_ips", "example.com").is_err());
        assert_eq!(validate("allowed_ips", "  ").unwrap(), "");
    }

    #[test]
    fn domain_list_accepts_wildcards_only_as_prefix() {
        assert_eq!(
            validate("bind_domains", "Panel.Example.com, *.ops.example.com").unwrap(),
            "panel.example.com,*.ops.example.com"
        );
        assert!(validate("bind_domains", "pa*nel.example.com").is_err());
        assert!(validate("bind_domains", "1.2.3.4").is_err());
    }

    #[test]
    fn validate_all_reports_every_error() {
        let err = validate_all(&[
            ("panel_port".into(), "x".into()),
            ("log_level".into(), "y".into()),
            ("panel_name".into(), "ok".into()),
        ])
        .unwrap_err();
        assert!(err.contains("panel_port") && err.contains("log_level"));
    }

    #[test]
    fn cidr_matching_handles_v4_v6_and_mapped() {
        let r = IpRule::parse("192.168.0.0/16").unwrap();
        assert!(r.contains("192.168.3.4".parse().unwrap()));
        assert!(r.contains("::ffff:192.168.3.4".parse().unwrap()));
        assert!(!r.contains("10.0.0.1".parse().unwrap()));
        let r6 = IpRule::parse("2001:db8::/32").unwrap();
        assert!(r6.contains("2001:db8:1::5".parse().unwrap()));
        assert!(!r6.contains("2001:db9::1".parse().unwrap()));
        let any = IpRule::parse("0.0.0.0/0").unwrap();
        assert!(any.contains("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn policy_ip_allowlist_and_local_escape_hatch() {
        let p = PanelPolicy::from_map(&map(&[("allowed_ips", "10.0.0.0/8")]));
        assert_eq!(
            p.check(Some("10.1.2.3".parse().unwrap()), None, false),
            AccessDecision::Allow
        );
        assert_eq!(
            p.check(Some("8.8.8.8".parse().unwrap()), None, false),
            AccessDecision::DenyIp
        );
        assert_eq!(p.check(None, None, false), AccessDecision::DenyIp);
        assert_eq!(
            p.check(Some("8.8.8.8".parse().unwrap()), None, true),
            AccessDecision::Allow
        );
    }

    #[test]
    fn policy_bind_domains_match_host_header() {
        let p = PanelPolicy::from_map(&map(&[(
            "bind_domains",
            "panel.example.com,*.ops.example.com",
        )]));
        assert_eq!(
            p.check(None, Some("panel.example.com:8443"), false),
            AccessDecision::Allow
        );
        assert_eq!(
            p.check(None, Some("a.ops.example.com"), false),
            AccessDecision::Allow
        );
        assert_eq!(
            p.check(None, Some("ops.example.com"), false),
            AccessDecision::DenyHost
        );
        assert_eq!(
            p.check(None, Some("1.2.3.4:8080"), false),
            AccessDecision::DenyHost
        );
        assert_eq!(p.check(None, None, false), AccessDecision::DenyHost);
    }

    #[test]
    fn policy_ignores_dirty_values() {
        let p = PanelPolicy::from_map(&map(&[
            ("session_timeout_minutes", "-5"),
            ("login_max_failures", "999"),
            ("allowed_ips", "garbage"),
        ]));
        assert_eq!(p, PanelPolicy::default());
    }

    #[test]
    fn strip_port_variants() {
        assert_eq!(strip_port("example.com:8080"), "example.com");
        assert_eq!(strip_port("example.com"), "example.com");
        assert_eq!(strip_port("[::1]:8080"), "::1");
        assert_eq!(strip_port("::1"), "::1");
    }

    #[test]
    fn every_default_setting_is_valid_under_its_spec() {
        for s in crate::domain::entity::default_settings() {
            assert!(
                validate(&s.key, &s.value).is_ok(),
                "default value of {} must pass validation",
                s.key
            );
        }
    }
}
