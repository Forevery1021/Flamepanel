//! 系统工具箱 · 主机详情（Doc/30「前端 ToolboxView + 主机详情」）。
//!
//! 只读汇总：系统 / CPU / 内存与 Swap / 时区 / DNS / hosts / 网卡 / 磁盘 / 面板自身。
//! 基础设施层（`infrastructure::host_probe`）只负责把原始文件内容与 sysinfo 读数装进
//! [`HostFacts`]；解析与整形全部在本模块，纯函数、可单测。任何事实缺失都只让对应字段为空，
//! 绝不让整份结果失败。
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::inspection::parse_os_release;

/// `/etc/hosts` 最多返回的条目数（防止超大 hosts 文件撑爆响应）
pub const MAX_HOSTS_ENTRIES: usize = 200;
/// 最多返回的网卡数（容器宿主机上 veth 可能成百上千）
pub const MAX_INTERFACES: usize = 64;
/// 最多返回的磁盘挂载点数
pub const MAX_DISKS: usize = 64;

/// 网卡原始读数（sysinfo）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawInterface {
    pub name: String,
    pub mac: Option<String>,
    /// `addr/prefix`
    pub addresses: Vec<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// 磁盘原始读数（sysinfo，已由基础设施层过滤掉伪文件系统）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawDisk {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

/// 基础设施层采集到的原始主机事实。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostFacts {
    pub hostname: Option<String>,
    /// `/etc/os-release` 全文
    pub os_release_file: Option<String>,
    pub kernel_release: Option<String>,
    pub arch: String,
    pub uptime_secs: Option<u64>,
    /// 开机时刻（Unix 秒）
    pub boot_time: Option<u64>,
    pub cpu_model: Option<String>,
    pub cpu_logical: usize,
    pub cpu_physical: Option<usize>,
    pub load_average: [f64; 3],
    pub mem_total: u64,
    pub mem_available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    /// `/etc/localtime` 符号链接目标
    pub localtime_target: Option<String>,
    /// `/etc/timezone` 全文（Debian 系）
    pub timezone_file: Option<String>,
    /// 面板进程看到的本地 UTC 偏移（`+08:00`）
    pub utc_offset: String,
    /// `/etc/resolv.conf` 全文
    pub resolv_conf: Option<String>,
    /// `/etc/hosts` 全文
    pub hosts_file: Option<String>,
    pub interfaces: Vec<RawInterface>,
    pub disks: Vec<RawDisk>,
}

/// 主机事实来源（基础设施层实现；测试注入静态数据）。
#[async_trait]
pub trait HostFactsSource: Send + Sync {
    async fn collect(&self) -> HostFacts;
}

/// 空来源：未注入采集器时返回全空事实（各字段降级为空，而不是报错）。
pub struct EmptyHostFacts;

#[async_trait]
impl HostFactsSource for EmptyHostFacts {
    async fn collect(&self) -> HostFacts {
        HostFacts::default()
    }
}

/// 固定事实（单测 / 集成测试）。
pub struct StaticHostFacts(pub HostFacts);

#[async_trait]
impl HostFactsSource for StaticHostFacts {
    async fn collect(&self) -> HostFacts {
        self.0.clone()
    }
}

// ─────────────────────────────── 输出模型 ───────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Default)]
pub struct HostSystem {
    pub hostname: String,
    /// 发行版展示名（`PRETTY_NAME`）
    pub os_name: String,
    pub os_id: String,
    pub os_version_id: String,
    pub kernel_release: String,
    pub arch: String,
    pub uptime_secs: u64,
    /// 开机时刻（不可探测为 null）
    pub boot_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Default)]
pub struct HostCpu {
    pub model: String,
    pub logical_cores: usize,
    pub physical_cores: Option<usize>,
    /// 1 / 5 / 15 分钟负载
    pub load_average: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Default)]
pub struct HostMemory {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Default)]
pub struct HostTime {
    /// IANA 时区名（`Asia/Shanghai`），不可探测为 null
    pub timezone: Option<String>,
    /// 面板进程本地 UTC 偏移（`+08:00`）
    pub utc_offset: String,
    /// 服务器当前时间（用于和浏览器时间对比时钟偏差）
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub struct DnsConfig {
    pub nameservers: Vec<String>,
    pub search: Vec<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HostsEntry {
    pub ip: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HostInterface {
    pub name: String,
    /// 全零 MAC（如 `lo`）为 null
    pub mac: Option<String>,
    /// `addr/prefix`，IPv4 在前
    pub addresses: Vec<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    /// 回环 / 容器虚拟网卡（`lo` / `veth*` / `docker*` / `br-*` / `cni*` / `flannel*` …）
    pub virtual_iface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HostDisk {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PanelRuntime {
    pub version: String,
    pub pid: u32,
    pub uid: u32,
    pub running_as_root: bool,
}

/// `GET /api/toolbox/host` 响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HostDetails {
    pub generated_at: DateTime<Utc>,
    pub system: HostSystem,
    pub cpu: HostCpu,
    pub memory: HostMemory,
    pub time: HostTime,
    pub dns: DnsConfig,
    pub hosts: Vec<HostsEntry>,
    /// hosts 条目超过上限被截断
    pub hosts_truncated: bool,
    pub interfaces: Vec<HostInterface>,
    pub disks: Vec<HostDisk>,
    pub panel: PanelRuntime,
}

// ─────────────────────────────── 解析 ───────────────────────────────

fn strip_comment(line: &str) -> &str {
    let cut = line.find(['#', ';']).unwrap_or(line.len());
    line[..cut].trim()
}

/// 解析 `/etc/resolv.conf`：`nameserver` / `search`（`domain` 视同单项 search）/ `options`。
pub fn parse_resolv_conf(text: &str) -> DnsConfig {
    let mut dns = DnsConfig::default();
    for raw in text.lines() {
        let line = strip_comment(raw);
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("nameserver") => {
                if let Some(ns) = parts.next() {
                    if !dns.nameservers.iter().any(|x| x == ns) {
                        dns.nameservers.push(ns.to_string());
                    }
                }
            }
            // 后出现的 search / domain 覆盖前者（与 glibc 行为一致）
            Some("search") | Some("domain") => {
                dns.search = parts.map(str::to_string).collect();
            }
            Some("options") => dns.options.extend(parts.map(str::to_string)),
            _ => {}
        }
    }
    dns
}

/// 解析 `/etc/hosts`：返回 (条目, 是否截断)。忽略注释、空行和没有主机名的行。
pub fn parse_hosts(text: &str, limit: usize) -> (Vec<HostsEntry>, bool) {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        let mut parts = line.split_whitespace();
        let (Some(ip), names) = (parts.next(), parts.map(str::to_string).collect::<Vec<_>>())
        else {
            continue;
        };
        if names.is_empty() {
            continue;
        }
        if out.len() == limit {
            return (out, true);
        }
        out.push(HostsEntry {
            ip: ip.to_string(),
            names,
        });
    }
    (out, false)
}

/// 由 `/etc/localtime` 链接目标或 `/etc/timezone` 推出 IANA 时区名。
///
/// `/usr/share/zoneinfo/Asia/Shanghai`、`../usr/share/zoneinfo/posix/UTC` 都能识别；
/// 链接目标优先（systemd `timedatectl set-timezone` 只改链接，`/etc/timezone` 可能过期）。
pub fn timezone_name(
    localtime_target: Option<&str>,
    timezone_file: Option<&str>,
) -> Option<String> {
    if let Some(target) = localtime_target {
        if let Some(idx) = target.find("zoneinfo/") {
            let mut name = &target[idx + "zoneinfo/".len()..];
            for prefix in ["posix/", "right/"] {
                if let Some(rest) = name.strip_prefix(prefix) {
                    name = rest;
                }
            }
            if is_valid_tz_name(name) {
                return Some(name.to_string());
            }
        }
    }
    timezone_file
        .and_then(|t| {
            t.lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with('#'))
        })
        .filter(|n| is_valid_tz_name(n))
        .map(str::to_string)
}

fn is_valid_tz_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'))
}

/// 回环 / 容器 / 隧道等虚拟网卡。
pub fn is_virtual_iface(name: &str) -> bool {
    name == "lo"
        || [
            "veth", "docker", "br-", "virbr", "cni", "flannel", "cali", "vxlan", "tun", "tap",
            "kube", "podman", "lxc",
        ]
        .iter()
        .any(|p| name.starts_with(p))
}

fn ip_family_rank(addr: &str) -> u8 {
    if addr.contains(':') {
        1
    } else {
        0
    }
}

/// 原始事实 → 响应模型。
pub fn build_host_details(
    facts: HostFacts,
    panel: PanelRuntime,
    now: DateTime<Utc>,
) -> HostDetails {
    let osr = facts
        .os_release_file
        .as_deref()
        .map(parse_os_release)
        .unwrap_or_default();
    let os_get = |k: &str| osr.get(k).cloned().unwrap_or_default();
    let mut os_name = os_get("PRETTY_NAME");
    if os_name.is_empty() {
        os_name = format!("{} {}", os_get("NAME"), os_get("VERSION_ID"))
            .trim()
            .to_string();
    }

    let system = HostSystem {
        hostname: facts
            .hostname
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string(),
        os_name,
        os_id: os_get("ID"),
        os_version_id: os_get("VERSION_ID"),
        kernel_release: facts
            .kernel_release
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string(),
        arch: facts.arch.clone(),
        uptime_secs: facts.uptime_secs.unwrap_or(0),
        boot_time: facts
            .boot_time
            .filter(|t| *t > 0)
            .and_then(|t| DateTime::<Utc>::from_timestamp(i64::try_from(t).ok()?, 0)),
    };

    let cpu = HostCpu {
        model: facts
            .cpu_model
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string(),
        logical_cores: facts.cpu_logical,
        physical_cores: facts.cpu_physical,
        load_average: facts
            .load_average
            .iter()
            .map(|v| {
                if v.is_finite() {
                    (v * 100.0).round() / 100.0
                } else {
                    0.0
                }
            })
            .collect(),
    };

    let memory = HostMemory {
        total_bytes: facts.mem_total,
        available_bytes: facts.mem_available.min(facts.mem_total),
        swap_total_bytes: facts.swap_total,
        swap_used_bytes: facts.swap_used.min(facts.swap_total),
    };

    let time = HostTime {
        timezone: timezone_name(
            facts.localtime_target.as_deref(),
            facts.timezone_file.as_deref(),
        ),
        utc_offset: facts.utc_offset.clone(),
        now,
    };

    let dns = facts
        .resolv_conf
        .as_deref()
        .map(parse_resolv_conf)
        .unwrap_or_default();
    let (hosts, hosts_truncated) = facts
        .hosts_file
        .as_deref()
        .map(|t| parse_hosts(t, MAX_HOSTS_ENTRIES))
        .unwrap_or_default();

    let mut interfaces: Vec<HostInterface> = facts
        .interfaces
        .into_iter()
        .map(|i| {
            let mut addresses = i.addresses;
            addresses.sort_by_key(|a| ip_family_rank(a));
            HostInterface {
                virtual_iface: is_virtual_iface(&i.name),
                mac: i.mac.filter(|m| !m.is_empty() && m != "00:00:00:00:00:00"),
                name: i.name,
                addresses,
                rx_bytes: i.rx_bytes,
                tx_bytes: i.tx_bytes,
            }
        })
        .collect();
    // 物理网卡在前；同类按名称
    interfaces.sort_by(|a, b| (a.virtual_iface, &a.name).cmp(&(b.virtual_iface, &b.name)));
    interfaces.truncate(MAX_INTERFACES);

    let mut disks: Vec<HostDisk> = Vec::new();
    for d in facts.disks {
        // 同一挂载点（bind mount / 重复上报）只保留第一条
        if disks.iter().any(|x| x.mount_point == d.mount_point) {
            continue;
        }
        disks.push(HostDisk {
            device: d.device,
            mount_point: d.mount_point,
            fs_type: d.fs_type,
            total_bytes: d.total_bytes,
            available_bytes: d.available_bytes.min(d.total_bytes),
        });
    }
    disks.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    disks.truncate(MAX_DISKS);

    HostDetails {
        generated_at: now,
        system,
        cpu,
        memory,
        time,
        dns,
        hosts,
        hosts_truncated,
        interfaces,
        disks,
        panel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> PanelRuntime {
        PanelRuntime {
            version: "1.0.0".into(),
            pid: 42,
            uid: 1000,
            running_as_root: false,
        }
    }

    #[test]
    fn resolv_conf_parses_nameservers_search_options() {
        let dns = parse_resolv_conf(
            "# generated\nnameserver 1.1.1.1\nnameserver 8.8.8.8 # g\nnameserver 1.1.1.1\n\
             domain old.local\nsearch corp.local lan\noptions edns0 trust-ad\n; x\n",
        );
        assert_eq!(dns.nameservers, vec!["1.1.1.1", "8.8.8.8"]);
        assert_eq!(dns.search, vec!["corp.local", "lan"]);
        assert_eq!(dns.options, vec!["edns0", "trust-ad"]);
    }

    #[test]
    fn resolv_conf_empty_is_default() {
        assert_eq!(parse_resolv_conf(""), DnsConfig::default());
    }

    #[test]
    fn hosts_parse_skips_comments_and_bare_ips() {
        let (entries, truncated) = parse_hosts(
            "127.0.0.1 localhost\n# c\n\n::1 localhost ip6-localhost # tail\n10.0.0.5\n10.0.0.6 db\tdb.lan\n",
            10,
        );
        assert!(!truncated);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].names, vec!["localhost", "ip6-localhost"]);
        assert_eq!(entries[2].names, vec!["db", "db.lan"]);
    }

    #[test]
    fn hosts_parse_truncates_at_limit() {
        let text = (0..5)
            .map(|i| format!("10.0.0.{i} h{i}\n"))
            .collect::<String>();
        let (entries, truncated) = parse_hosts(&text, 3);
        assert_eq!(entries.len(), 3);
        assert!(truncated);
        let (entries, truncated) = parse_hosts(&text, 5);
        assert_eq!(entries.len(), 5);
        assert!(!truncated);
    }

    #[test]
    fn timezone_from_link_target_and_file() {
        assert_eq!(
            timezone_name(Some("/usr/share/zoneinfo/Asia/Shanghai"), None).as_deref(),
            Some("Asia/Shanghai")
        );
        assert_eq!(
            timezone_name(Some("../usr/share/zoneinfo/posix/Etc/UTC"), None).as_deref(),
            Some("Etc/UTC")
        );
        // 链接目标不可用时回落 /etc/timezone
        assert_eq!(
            timezone_name(Some("/etc/weird"), Some("# c\nEurope/Berlin\n")).as_deref(),
            Some("Europe/Berlin")
        );
        // 链接优先于（可能过期的）/etc/timezone
        assert_eq!(
            timezone_name(Some("/usr/share/zoneinfo/UTC"), Some("Asia/Tokyo")).as_deref(),
            Some("UTC")
        );
        assert_eq!(timezone_name(None, Some("../../etc/passwd")), None);
        assert_eq!(timezone_name(None, None), None);
    }

    #[test]
    fn virtual_iface_detection() {
        for n in [
            "lo",
            "veth12ab",
            "docker0",
            "br-3f2a",
            "cni0",
            "flannel.1",
            "tun0",
        ] {
            assert!(is_virtual_iface(n), "{n}");
        }
        for n in ["eth0", "ens3", "enp0s31f6", "wlan0", "bond0"] {
            assert!(!is_virtual_iface(n), "{n}");
        }
    }

    #[test]
    fn build_details_shapes_everything() {
        let facts = HostFacts {
            hostname: Some("web-01\n".into()),
            os_release_file: Some("NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nID=ubuntu\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n".into()),
            kernel_release: Some("6.8.0-45-generic\n".into()),
            arch: "x86_64".into(),
            uptime_secs: Some(3600),
            boot_time: Some(1_700_000_000),
            cpu_model: Some(" Intel(R) Xeon(R) ".into()),
            cpu_logical: 8,
            cpu_physical: Some(4),
            load_average: [0.123, 1.0, f64::NAN],
            mem_total: 100,
            mem_available: 150,
            swap_total: 10,
            swap_used: 20,
            localtime_target: Some("/usr/share/zoneinfo/Asia/Shanghai".into()),
            timezone_file: None,
            utc_offset: "+08:00".into(),
            resolv_conf: Some("nameserver 1.1.1.1\n".into()),
            hosts_file: Some("127.0.0.1 localhost\n".into()),
            interfaces: vec![
                RawInterface {
                    name: "lo".into(),
                    mac: Some("00:00:00:00:00:00".into()),
                    addresses: vec!["::1/128".into(), "127.0.0.1/8".into()],
                    ..Default::default()
                },
                RawInterface {
                    name: "veth1".into(),
                    ..Default::default()
                },
                RawInterface {
                    name: "eth0".into(),
                    mac: Some("52:54:00:aa:bb:cc".into()),
                    addresses: vec!["fe80::1/64".into(), "10.0.0.2/24".into()],
                    rx_bytes: 5,
                    tx_bytes: 6,
                },
            ],
            disks: vec![
                RawDisk {
                    device: "/dev/vda1".into(),
                    mount_point: "/".into(),
                    fs_type: "ext4".into(),
                    total_bytes: 10,
                    available_bytes: 4,
                },
                RawDisk {
                    device: "/dev/vda1".into(),
                    mount_point: "/".into(),
                    fs_type: "ext4".into(),
                    total_bytes: 10,
                    available_bytes: 4,
                },
                RawDisk {
                    device: "/dev/vdb".into(),
                    mount_point: "/data".into(),
                    fs_type: "xfs".into(),
                    total_bytes: 5,
                    available_bytes: 9,
                },
            ],
        };
        let now = Utc::now();
        let d = build_host_details(facts, panel(), now);
        assert_eq!(d.system.hostname, "web-01");
        assert_eq!(d.system.os_name, "Ubuntu 24.04 LTS");
        assert_eq!(d.system.os_id, "ubuntu");
        assert_eq!(d.system.kernel_release, "6.8.0-45-generic");
        assert_eq!(
            d.system.boot_time.map(|t| t.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(d.cpu.model, "Intel(R) Xeon(R)");
        assert_eq!(d.cpu.load_average, vec![0.12, 1.0, 0.0]);
        assert_eq!(d.memory.available_bytes, 100, "available 不超过 total");
        assert_eq!(d.memory.swap_used_bytes, 10, "swap used 不超过 total");
        assert_eq!(d.time.timezone.as_deref(), Some("Asia/Shanghai"));
        assert_eq!(d.time.now, now);
        assert_eq!(d.dns.nameservers, vec!["1.1.1.1"]);
        assert_eq!(d.hosts.len(), 1);
        let names: Vec<_> = d.interfaces.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["eth0", "lo", "veth1"], "物理网卡在前");
        assert_eq!(
            d.interfaces[0].addresses,
            vec!["10.0.0.2/24", "fe80::1/64"],
            "IPv4 在前"
        );
        assert_eq!(d.interfaces[1].mac, None, "全零 MAC 置空");
        assert!(d.interfaces[1].virtual_iface);
        assert_eq!(d.disks.len(), 2, "重复挂载点去重");
        assert_eq!(d.disks[1].available_bytes, 5);
        assert_eq!(d.panel.pid, 42);
    }

    #[test]
    fn build_details_tolerates_empty_facts() {
        let d = build_host_details(HostFacts::default(), panel(), Utc::now());
        assert_eq!(d.system.hostname, "");
        assert_eq!(d.system.os_name, "");
        assert_eq!(d.system.boot_time, None);
        assert_eq!(d.time.timezone, None);
        assert!(d.hosts.is_empty() && !d.hosts_truncated);
        assert!(d.interfaces.is_empty() && d.disks.is_empty());
    }

    #[test]
    fn os_name_falls_back_to_name_and_version() {
        let facts = HostFacts {
            os_release_file: Some("NAME=Debian\nVERSION_ID=12\n".into()),
            ..Default::default()
        };
        let d = build_host_details(facts, panel(), Utc::now());
        assert_eq!(d.system.os_name, "Debian 12");
    }
}
