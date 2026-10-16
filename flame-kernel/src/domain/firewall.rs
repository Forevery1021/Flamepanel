//! 防火墙规则校验与规范化（S-13 · 纯函数，无 IO）。
//!
//! 规则字段全部以字符串落库（兼容既有数据），写入前经 [`normalize_rule`] 统一：
//!
//! - **协议** `tcp | udp | icmp | any`（大小写不敏感）；
//! - **端口** 规范形态 `80,443,8000-9000`：单端口 / 端口段（`-` 或 `:` 分隔均可输入，统一存 `-`），
//!   逗号分隔，至多 [`MAX_PORT_SLOTS`] 个槽位（端口段占 2 个，对齐 iptables multiport 上限），
//!   去重；端口仅在 `tcp` / `udp` 下允许（`icmp` 无端口，`any` 带端口在各后端语义不一致）；
//! - **来源 / 目标地址**：空 / `any` / `0.0.0.0/0` / `::/0` 视为任意（统一存 `0.0.0.0/0` / `None`），
//!   否则须为 IPv4 / IPv6 地址或 CIDR；主机位不为 0 的 CIDR 规范化为网络地址
//!   （`192.168.1.10/24` → `192.168.1.0/24`），与 iptables 行为一致、避免删除规则时对不上；
//! - **动作** `allow | deny | reject`、**方向** `in | out`、**优先级** 1–999、名称 1–64 字符。
//!
//! 严格白名单同时堵住 firewalld rich rule 的字符串注入（地址 / 端口里不可能再出现引号或空白）。
//! 各后端的命令渲染见 `infrastructure/firewall.rs`，消费这里解析出的 [`PortItem`] / [`AddressSpec`]。
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::domain::entity::FirewallRule;

/// 端口槽位上限（iptables `-m multiport` 最多 15 个，端口段计 2 个）。
pub const MAX_PORT_SLOTS: usize = 15;
pub const MAX_NAME_CHARS: usize = 64;
pub const MAX_DESCRIPTION_CHARS: usize = 255;
pub const PRIORITY_RANGE: std::ops::RangeInclusive<i32> = 1..=999;
/// 规范化后的"任意来源"。
pub const ANY_SOURCE: &str = "0.0.0.0/0";

pub const PROTOCOLS: &[&str] = &["tcp", "udp", "icmp", "any"];
pub const ACTIONS: &[&str] = &["allow", "deny", "reject"];
pub const DIRECTIONS: &[&str] = &["in", "out"];

/// 单个端口或端口段（闭区间，`start == end` 为单端口）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortItem {
    pub start: u16,
    pub end: u16,
}

impl PortItem {
    pub fn is_range(&self) -> bool {
        self.start != self.end
    }

    /// 用给定分隔符渲染（ufw / iptables 用 `:`，firewalld 与存储用 `-`）。
    pub fn render(&self, sep: char) -> String {
        if self.is_range() {
            format!("{}{sep}{}", self.start, self.end)
        } else {
            self.start.to_string()
        }
    }
}

/// 地址（已规范化）：单个主机或网段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressSpec {
    pub addr: IpAddr,
    /// 前缀长度；单主机为 32 / 128
    pub prefix: u8,
}

impl AddressSpec {
    pub fn is_ipv6(&self) -> bool {
        self.addr.is_ipv6()
    }

    fn max_prefix(addr: &IpAddr) -> u8 {
        if addr.is_ipv6() {
            128
        } else {
            32
        }
    }

    pub fn is_host(&self) -> bool {
        self.prefix == Self::max_prefix(&self.addr)
    }
}

impl std::fmt::Display for AddressSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_host() {
            write!(f, "{}", self.addr)
        } else {
            write!(f, "{}/{}", self.addr, self.prefix)
        }
    }
}

/// 字段级校验错误：`field` 为请求体字段名，`reason` 为稳定的机器可读原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: &'static str,
    pub reason: String,
}

impl FieldError {
    fn new(field: &'static str, reason: impl Into<String>) -> Self {
        Self {
            field,
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.reason)
    }
}

/// 解析端口表达式；空串 → 空列表（= 所有端口）。
pub fn parse_ports(raw: &str) -> Result<Vec<PortItem>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut items: Vec<PortItem> = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err("empty item in port list".into());
        }
        let (a, b) = match part.split_once(['-', ':']) {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (part, part),
        };
        let start = parse_port(a)?;
        let end = parse_port(b)?;
        if start > end {
            return Err(format!("port range {start}-{end} is reversed"));
        }
        let item = PortItem { start, end };
        if !items.contains(&item) {
            items.push(item);
        }
    }
    let slots: usize = items.iter().map(|p| if p.is_range() { 2 } else { 1 }).sum();
    if slots > MAX_PORT_SLOTS {
        return Err(format!(
            "too many ports ({slots} slots, max {MAX_PORT_SLOTS}; a range counts as 2)"
        ));
    }
    Ok(items)
}

fn parse_port(s: &str) -> Result<u16, String> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid port '{s}'"));
    }
    match s.parse::<u32>() {
        Ok(n) if (1..=65535).contains(&n) => Ok(n as u16),
        _ => Err(format!("port {s} out of range 1-65535")),
    }
}

/// 端口列表的规范存储形态（`-` 表示端口段）。
pub fn format_ports(items: &[PortItem]) -> String {
    items
        .iter()
        .map(|p| p.render('-'))
        .collect::<Vec<_>>()
        .join(",")
}

/// 是否表示"任意地址"。
pub fn is_any_address(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "" | "any" | "0.0.0.0/0" | "::/0"
    )
}

/// 解析地址 / CIDR；任意地址返回 `Ok(None)`。主机位不为 0 时归一到网络地址。
pub fn parse_address(raw: &str) -> Result<Option<AddressSpec>, String> {
    let raw = raw.trim();
    if is_any_address(raw) {
        return Ok(None);
    }
    let (addr_part, prefix_part) = match raw.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (raw, None),
    };
    let addr: IpAddr = addr_part
        .parse()
        .map_err(|_| format!("invalid IP address '{addr_part}'"))?;
    let max = AddressSpec::max_prefix(&addr);
    let prefix = match prefix_part {
        None => max,
        Some(p) => {
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("invalid prefix length '/{p}'"));
            }
            match p.parse::<u8>() {
                Ok(n) if n <= max => n,
                _ => return Err(format!("prefix length /{p} exceeds /{max}")),
            }
        }
    };
    if prefix == 0 {
        // `1.2.3.4/0` 等价于任意地址——显式写 any 更清楚，避免误以为只放行一个网段
        return Err("prefix length /0 matches everything; use 'any' instead".into());
    }
    Ok(Some(AddressSpec {
        addr: network_of(addr, prefix),
        prefix,
    }))
}

fn network_of(addr: IpAddr, prefix: u8) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(prefix))
            };
            IpAddr::V4(Ipv4Addr::from(u32::from(v4) & mask))
        }
        IpAddr::V6(v6) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(prefix))
            };
            IpAddr::V6(Ipv6Addr::from(u128::from(v6) & mask))
        }
    }
}

fn one_of(field: &'static str, value: &str, allowed: &[&str]) -> Result<String, FieldError> {
    let v = value.trim().to_ascii_lowercase();
    if allowed.contains(&v.as_str()) {
        Ok(v)
    } else {
        Err(FieldError::new(
            field,
            format!("must be one of {}", allowed.join(" / ")),
        ))
    }
}

fn opt_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// 校验并就地规范化一条规则；返回全部字段错误（而非遇错即停），便于前端逐项提示。
pub fn normalize_rule(rule: &mut FirewallRule) -> Result<(), Vec<FieldError>> {
    let mut errors = Vec::new();

    let name = rule.name.trim().to_string();
    if name.is_empty() {
        errors.push(FieldError::new("name", "required"));
    } else if name.chars().count() > MAX_NAME_CHARS {
        errors.push(FieldError::new(
            "name",
            format!("at most {MAX_NAME_CHARS} characters"),
        ));
    } else if name.chars().any(char::is_control) {
        errors.push(FieldError::new(
            "name",
            "must not contain control characters",
        ));
    }
    rule.name = name;

    rule.description = opt_text(rule.description.as_deref());
    if let Some(desc) = &rule.description {
        if desc.chars().count() > MAX_DESCRIPTION_CHARS {
            errors.push(FieldError::new(
                "description",
                format!("at most {MAX_DESCRIPTION_CHARS} characters"),
            ));
        }
    }

    match one_of("protocol", &rule.protocol, PROTOCOLS) {
        Ok(v) => rule.protocol = v,
        Err(e) => errors.push(e),
    }
    match one_of("action", &rule.action, ACTIONS) {
        Ok(v) => rule.action = v,
        Err(e) => errors.push(e),
    }
    match one_of("direction", &rule.direction, DIRECTIONS) {
        Ok(v) => rule.direction = v,
        Err(e) => errors.push(e),
    }
    if !PRIORITY_RANGE.contains(&rule.priority) {
        errors.push(FieldError::new(
            "priority",
            format!(
                "must be between {} and {}",
                PRIORITY_RANGE.start(),
                PRIORITY_RANGE.end()
            ),
        ));
    }

    match parse_ports(rule.port.as_deref().unwrap_or("")) {
        Ok(items) if items.is_empty() => rule.port = None,
        Ok(items) => {
            if matches!(rule.protocol.as_str(), "tcp" | "udp") {
                rule.port = Some(format_ports(&items));
            } else if PROTOCOLS.contains(&rule.protocol.as_str()) {
                errors.push(FieldError::new("port", "ports require protocol tcp or udp"));
            }
        }
        Err(reason) => errors.push(FieldError::new("port", reason)),
    }

    match parse_address(rule.source.as_deref().unwrap_or("")) {
        Ok(Some(spec)) => rule.source = Some(spec.to_string()),
        Ok(None) => rule.source = Some(ANY_SOURCE.into()),
        Err(reason) => errors.push(FieldError::new("source", reason)),
    }
    match parse_address(rule.destination.as_deref().unwrap_or("")) {
        Ok(Some(spec)) => rule.destination = Some(spec.to_string()),
        Ok(None) => rule.destination = None,
        Err(reason) => errors.push(FieldError::new("destination", reason)),
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 把字段错误拼成一条消息（`field: reason; field: reason`），作为 400 的 message。
pub fn describe_errors(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn rule() -> FirewallRule {
        FirewallRule {
            id: 0,
            name: "web".into(),
            description: None,
            protocol: "tcp".into(),
            port: Some("80".into()),
            source: None,
            destination: None,
            action: "allow".into(),
            enabled: true,
            priority: 50,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fields(errors: &[FieldError]) -> Vec<&'static str> {
        errors.iter().map(|e| e.field).collect()
    }

    #[test]
    fn ports_accept_single_range_list_and_normalize() {
        assert_eq!(parse_ports("").unwrap(), vec![]);
        assert_eq!(
            parse_ports(" 80 , 443,8000:9000, 22-22 ").unwrap(),
            vec![
                PortItem { start: 80, end: 80 },
                PortItem {
                    start: 443,
                    end: 443
                },
                PortItem {
                    start: 8000,
                    end: 9000
                },
                PortItem { start: 22, end: 22 },
            ]
        );
        assert_eq!(
            format_ports(&parse_ports("443,80,443,8000:9000").unwrap()),
            "443,80,8000-9000"
        );
    }

    #[test]
    fn ports_reject_bad_values_and_too_many_slots() {
        for bad in [
            "0",
            "65536",
            "abc",
            "80,",
            "9000-8000",
            "1-2-3",
            "+80",
            "80 443",
            "8000-",
        ] {
            assert!(parse_ports(bad).is_err(), "{bad} should be rejected");
        }
        let fifteen: Vec<String> = (1..=15).map(|n| n.to_string()).collect();
        assert!(parse_ports(&fifteen.join(",")).is_ok());
        let sixteen: Vec<String> = (1..=16).map(|n| n.to_string()).collect();
        assert!(parse_ports(&sixteen.join(",")).is_err());
        // 8 个端口段 = 16 槽位
        let ranges: Vec<String> = (1..=8).map(|n| format!("{0}-{0}0", n * 1000)).collect();
        assert!(parse_ports(&ranges.join(",")).is_err());
    }

    #[test]
    fn addresses_parse_hosts_cidrs_and_normalize_host_bits() {
        assert_eq!(parse_address("").unwrap(), None);
        assert_eq!(parse_address("ANY").unwrap(), None);
        assert_eq!(parse_address("::/0").unwrap(), None);
        assert_eq!(
            parse_address("10.0.0.5").unwrap().unwrap().to_string(),
            "10.0.0.5"
        );
        assert_eq!(
            parse_address("192.168.1.10/24")
                .unwrap()
                .unwrap()
                .to_string(),
            "192.168.1.0/24"
        );
        assert_eq!(
            parse_address("10.0.0.5/32").unwrap().unwrap().to_string(),
            "10.0.0.5"
        );
        let v6 = parse_address("2001:db8::1/48").unwrap().unwrap();
        assert!(v6.is_ipv6());
        assert_eq!(v6.to_string(), "2001:db8::/48");
        // 与前端预览同口径：IPv4 映射地址保留点分写法
        assert_eq!(
            parse_address("::ffff:c0a8:101/120")
                .unwrap()
                .unwrap()
                .to_string(),
            "::ffff:192.168.1.0/120"
        );
    }

    #[test]
    fn addresses_reject_garbage_and_injection() {
        for bad in [
            "1.2.3",
            "256.1.1.1",
            "10.0.0.0/33",
            "2001:db8::/129",
            "10.0.0.0/",
            "10.0.0.0/-1",
            "1.2.3.4/0",
            "example.com",
            "1.2.3.4\" accept",
            "1.2.3.4 port port=22",
        ] {
            assert!(parse_address(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn normalize_canonicalizes_fields() {
        let mut r = rule();
        r.name = "  SSH  ".into();
        r.description = Some("   ".into());
        r.protocol = "TCP".into();
        r.action = "Deny".into();
        r.direction = "OUT".into();
        r.port = Some("22, 2200:2210".into());
        r.source = Some("any".into());
        r.destination = Some("10.1.2.3/8".into());
        normalize_rule(&mut r).unwrap();
        assert_eq!(r.name, "SSH");
        assert_eq!(r.description, None);
        assert_eq!(
            (r.protocol.as_str(), r.action.as_str(), r.direction.as_str()),
            ("tcp", "deny", "out")
        );
        assert_eq!(r.port.as_deref(), Some("22,2200-2210"));
        assert_eq!(r.source.as_deref(), Some(ANY_SOURCE));
        assert_eq!(r.destination.as_deref(), Some("10.0.0.0/8"));

        let mut empty_port = rule();
        empty_port.port = Some("  ".into());
        normalize_rule(&mut empty_port).unwrap();
        assert_eq!(empty_port.port, None);
    }

    #[test]
    fn built_in_default_rules_pass_validation_unchanged() {
        for original in crate::domain::entity::default_firewall_rules() {
            let mut r = original.clone();
            normalize_rule(&mut r).unwrap_or_else(|e| panic!("{}: {e:?}", original.name));
            assert_eq!(
                (r.protocol, r.port, r.source, r.action, r.direction),
                (
                    original.protocol,
                    original.port,
                    original.source,
                    original.action,
                    original.direction
                ),
            );
        }
    }

    #[test]
    fn ports_require_tcp_or_udp() {
        for proto in ["icmp", "any"] {
            let mut r = rule();
            r.protocol = proto.into();
            let errs = normalize_rule(&mut r).unwrap_err();
            assert_eq!(fields(&errs), vec!["port"], "{proto}");
            let mut ok = rule();
            ok.protocol = proto.into();
            ok.port = None;
            assert!(normalize_rule(&mut ok).is_ok(), "{proto} without port");
        }
    }

    #[test]
    fn normalize_collects_all_field_errors() {
        let mut r = rule();
        r.name = " ".into();
        r.protocol = "sctp".into();
        r.action = "drop".into();
        r.direction = "both".into();
        r.priority = 0;
        r.port = Some("70000".into());
        r.source = Some("nope".into());
        r.destination = Some("1.1.1.1/40".into());
        r.description = Some("x".repeat(MAX_DESCRIPTION_CHARS + 1));
        let errs = normalize_rule(&mut r).unwrap_err();
        assert_eq!(
            fields(&errs),
            vec![
                "name",
                "description",
                "protocol",
                "action",
                "direction",
                "priority",
                "port",
                "source",
                "destination"
            ]
        );
        let msg = describe_errors(&errs);
        assert!(msg.starts_with("name: required; description:"), "{msg}");

        let mut long = rule();
        long.name = "x".repeat(MAX_NAME_CHARS + 1);
        assert_eq!(
            fields(&normalize_rule(&mut long).unwrap_err()),
            vec!["name"]
        );
        let mut ctrl = rule();
        ctrl.name = "a\nb".into();
        assert_eq!(
            fields(&normalize_rule(&mut ctrl).unwrap_err()),
            vec!["name"]
        );
    }
}
