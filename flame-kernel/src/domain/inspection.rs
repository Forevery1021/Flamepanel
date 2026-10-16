//! 系统体检（S-17 · Doc/30 M-C）：内核信息 + 风险 / 漏洞分析的**纯领域逻辑**。
//!
//! 设计：基础设施层只负责把系统原始事实（文件内容、命令输出）收集进 [`SystemFacts`]，
//! 全部判定都在 [`evaluate`] 里完成——纯函数、可离线单测、与 Embedded / Agent 执行模式无关。
//! 每条结论是一条 [`Finding`]：稳定 `code`（前端据此做三语言文案）、严重度、状态与
//! 语言无关的 `evidence`（原始取证值）。内核 / 发行版生命周期判断依赖内置表
//! （`DATA_VERSION`），属启发式，`heuristic = true` 由前端明确标注。
use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 内置生命周期表的数据版本（内核 LTS / 发行版 EOL），随版本迭代手工更新。
pub const DATA_VERSION: &str = "2026-09";

/// 严重度（升序排列便于比较：`Critical > High > …`）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// 评分扣分权重（仅 `Fail` 计分）。
    pub fn weight(self) -> u32 {
        match self {
            Severity::Critical => 25,
            Severity::High => 15,
            Severity::Medium => 8,
            Severity::Low => 3,
            Severity::Info => 0,
        }
    }
}

/// 检查项状态：`pass` 未发现风险 / `fail` 存在风险 / `unknown` 无法判定（数据缺失或不可读）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Unknown,
}

/// 检查项分类（前端分组展示）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Kernel,
    Cpu,
    Hardening,
    Access,
    Network,
    Updates,
    Storage,
    Panel,
}

/// 单条体检结论。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Finding {
    /// 稳定编码（如 `cpu_vulnerability` / `sshd_root_login`），前端据此映射标题与建议
    pub code: String,
    /// 细分主体（如漏洞名、端口、账号名），同一 `code` 可有多条
    pub subject: Option<String>,
    pub category: Category,
    pub severity: Severity,
    pub status: CheckStatus,
    /// 语言无关的原始取证（配置行、sysctl 值、文件模式等）
    pub evidence: String,
    /// 依赖内置生命周期表等启发式数据的结论
    pub heuristic: bool,
}

impl Finding {
    fn new(code: &str, category: Category, severity: Severity, status: CheckStatus) -> Self {
        Self {
            code: code.to_string(),
            subject: None,
            category,
            severity,
            status,
            evidence: String::new(),
            heuristic: false,
        }
    }

    fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    fn evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = evidence.into();
        self
    }

    fn heuristic(mut self) -> Self {
        self.heuristic = true;
        self
    }
}

/// 内核与主机基础信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Default)]
pub struct KernelInfo {
    /// `uname -r`（`/proc/sys/kernel/osrelease`）
    pub release: String,
    /// `/proc/version` 全文（编译器 / 构建时间）
    pub version: String,
    /// 面板进程架构（`x86_64` / `aarch64`）
    pub arch: String,
    /// 发行版展示名（`PRETTY_NAME`）
    pub os_name: String,
    /// 发行版 `ID`（`ubuntu` / `debian` / `rocky` …）
    pub os_id: String,
    /// 发行版 `VERSION_ID`
    pub os_version_id: String,
    pub hostname: String,
    pub uptime_secs: u64,
    /// 虚拟化类型（`systemd-detect-virt`：`kvm` / `docker` / `none` …），不可探测为 null
    pub virtualization: Option<String>,
    /// 存在 `/var/run/reboot-required` 或已安装更新内核但仍运行旧内核
    pub reboot_required: bool,
    /// 已安装的最新内核版本（`/boot/vmlinuz-*`），与 `release` 不一致即待重启生效
    pub newest_installed_kernel: Option<String>,
    /// 内核主线系列（`6.1`）
    pub series: Option<String>,
    /// 是否为内置表中的 LTS 系列（不在表中为 null）
    pub lts: Option<bool>,
    /// 系列上游 EOL 日期（`YYYY-MM-DD`，内置表）
    pub series_eol: Option<String>,
}

/// 汇总：评分 / 等级 / 各状态与严重度计数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub struct InspectionSummary {
    /// 0–100：100 减去所有 `fail` 项按严重度加权的扣分
    pub score: u32,
    /// `A`（≥ 90）/ `B`（≥ 75）/ `C`（≥ 60）/ `D`（≥ 40）/ `F`
    pub grade: String,
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub unknown: u32,
    /// 以下为 `fail` 项按严重度计数
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

/// 一次体检报告。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InspectionReport {
    pub generated_at: DateTime<Utc>,
    /// 采集 + 判定耗时
    pub duration_ms: u64,
    /// 命中服务端缓存时为缓存年龄，刚生成为 0
    pub cache_age_secs: u64,
    /// 内置生命周期表版本
    pub data_version: String,
    pub kernel: KernelInfo,
    pub summary: InspectionSummary,
    pub findings: Vec<Finding>,
}

/// 面板自身的安全事实（由应用层从设置 / 配置读取）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelFacts {
    pub session_timeout_minutes: Option<u32>,
    /// `/metrics` 是否配置了访问令牌
    pub metrics_token_set: bool,
    /// 面板进程是否 root
    pub running_as_root: Option<bool>,
    /// 面板监听端口（用于把自身从"暴露服务"中识别出来）
    pub panel_port: Option<u16>,
}

/// 基础设施层收集到的原始系统事实；任何字段缺失都只让对应检查项变为 `unknown`。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemFacts {
    pub proc_version: Option<String>,
    pub osrelease: Option<String>,
    pub os_release_file: Option<String>,
    pub hostname: Option<String>,
    pub uptime_secs: Option<u64>,
    pub arch: String,
    pub virtualization: Option<String>,
    pub reboot_required_file: bool,
    /// `/boot/vmlinuz-*` 去掉前缀后的版本串
    pub installed_kernels: Vec<String>,
    /// `/sys/devices/system/cpu/vulnerabilities/<name>` → 内容
    pub cpu_vulnerabilities: Vec<(String, String)>,
    /// `/proc/sys/<a/b/c>` → 值（key 用点号形式 `kernel.randomize_va_space`）
    pub sysctl: Vec<(String, String)>,
    /// 已展开 `Include` 的 sshd 有效配置文本；无 sshd 为 None
    pub sshd_config: Option<String>,
    pub passwd: Option<String>,
    /// `/etc/shadow` 内容（非 root 不可读时为 None）
    pub shadow: Option<String>,
    /// 文件权限位（低 12 位）
    pub shadow_mode: Option<u32>,
    pub passwd_mode: Option<u32>,
    pub docker_sock_mode: Option<u32>,
    pub proc_net_tcp: Option<String>,
    pub proc_net_tcp6: Option<String>,
    /// 主机防火墙是否生效（ufw / firewalld / nftables 任一），无法判定为 None
    pub firewall_active: Option<bool>,
    pub firewall_backend: Option<String>,
    pub apparmor_enabled: Option<bool>,
    pub selinux_enforcing: Option<bool>,
    pub time_synced: Option<bool>,
    /// `/var/lib/update-notifier/updates-available` 全文（Ubuntu）
    pub updates_available_text: Option<String>,
    pub unattended_upgrades: Option<bool>,
    pub root_disk_usage_percent: Option<f32>,
    pub mem_total_mb: Option<u64>,
    pub swap_total_mb: Option<u64>,
    pub panel: PanelFacts,
}

/// 事实来源端口：生产实现为 `infrastructure::system_probe::SystemProbe`，测试注入固定事实。
#[async_trait]
pub trait FactsSource: Send + Sync {
    /// 采集原始事实；`panel` 为应用层已算好的面板事实，实现需原样带回。
    async fn collect(&self, panel: PanelFacts) -> SystemFacts;
}

/// 固定事实来源（测试 / 演示）。
pub struct StaticFactsSource(pub SystemFacts);

#[async_trait]
impl FactsSource for StaticFactsSource {
    async fn collect(&self, panel: PanelFacts) -> SystemFacts {
        let mut facts = self.0.clone();
        facts.panel = panel;
        facts
    }
}

// ───────────────────────────── 解析工具（纯函数） ─────────────────────────────

/// `/etc/os-release` → 键值表（去引号）。
pub fn parse_os_release(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(k.trim().to_string(), v);
        }
    }
    map
}

/// 内核版本串 → `(major, minor, patch)`；`5.15.0-113-generic` → `(5, 15, 0)`。
pub fn parse_kernel_release(release: &str) -> Option<(u32, u32, u32)> {
    let core: String = release
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

/// 版本串比较键：`(major, minor, patch, 其余数字段)`，用于挑选"最新已安装内核"。
fn kernel_sort_key(release: &str) -> Vec<u32> {
    release
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

/// 内置内核 LTS 系列表：`(major, minor, 上游 EOL)`。数据版本见 [`DATA_VERSION`]。
const KERNEL_LTS_EOL: &[(u32, u32, &str)] = &[
    (6, 18, "2027-12-31"),
    (6, 12, "2026-12-31"),
    (6, 6, "2026-12-31"),
    (6, 1, "2027-12-31"),
    (5, 15, "2026-10-31"),
    (5, 10, "2026-12-31"),
    (5, 4, "2025-12-31"),
    (4, 19, "2024-12-31"),
    (4, 14, "2024-01-31"),
    (4, 9, "2023-01-31"),
    (4, 4, "2022-02-28"),
    (3, 16, "2020-06-30"),
    (3, 10, "2017-11-30"),
];

/// 内核系列生命周期判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelSeriesStatus {
    /// 内置表中的 LTS 系列，附 EOL 日期与是否已过期
    Lts { eol: NaiveDate, expired: bool },
    /// 早于内置表最新 LTS 的非 LTS 系列：上游必然已停止维护
    NonLtsExpired,
    /// 比内置表更新的系列，无法判定
    Unknown,
}

/// 判定内核主线系列的上游维护状态（纯启发式：只看 `major.minor`）。
pub fn kernel_series_status(major: u32, minor: u32, today: NaiveDate) -> KernelSeriesStatus {
    if let Some((_, _, eol)) = KERNEL_LTS_EOL
        .iter()
        .find(|(ma, mi, _)| *ma == major && *mi == minor)
    {
        let eol = NaiveDate::parse_from_str(eol, "%Y-%m-%d").unwrap_or(today);
        return KernelSeriesStatus::Lts {
            eol,
            expired: today > eol,
        };
    }
    let newest_lts = KERNEL_LTS_EOL[0];
    if (major, minor) < (newest_lts.0, newest_lts.1) {
        KernelSeriesStatus::NonLtsExpired
    } else {
        KernelSeriesStatus::Unknown
    }
}

/// 发行版内核（厂商长期回溯补丁，上游系列 EOL 不直接适用）。
pub fn is_vendor_kernel(release: &str) -> bool {
    const MARKERS: &[&str] = &[
        ".el",
        "-generic",
        "-aws",
        "-azure",
        "-gcp",
        "-oracle",
        "-kvm",
        "-lowlatency",
        "-amd64",
        "-arm64",
        "-cloud-amd64",
        "-cloud-arm64",
        ".fc",
        "-lts",
        "-virt",
        ".amzn",
        "-tegra",
    ];
    MARKERS.iter().any(|m| release.contains(m))
}

/// 内置发行版 EOL 表：`(ID, VERSION_ID 前缀, EOL)`。数据版本见 [`DATA_VERSION`]。
const OS_EOL: &[(&str, &str, &str)] = &[
    ("ubuntu", "14.04", "2019-04-30"),
    ("ubuntu", "16.04", "2021-04-30"),
    ("ubuntu", "18.04", "2023-05-31"),
    ("ubuntu", "20.04", "2025-05-31"),
    ("ubuntu", "22.04", "2027-04-30"),
    ("ubuntu", "24.04", "2029-04-30"),
    ("ubuntu", "25.04", "2026-01-31"),
    ("ubuntu", "25.10", "2026-07-31"),
    ("ubuntu", "26.04", "2031-04-30"),
    ("debian", "9", "2022-06-30"),
    ("debian", "10", "2024-06-30"),
    ("debian", "11", "2026-08-31"),
    ("debian", "12", "2028-06-30"),
    ("debian", "13", "2030-06-30"),
    ("centos", "7", "2024-06-30"),
    ("centos", "8", "2021-12-31"),
    ("centos", "9", "2027-05-31"),
    ("rhel", "7", "2024-06-30"),
    ("rhel", "8", "2029-05-31"),
    ("rhel", "9", "2032-05-31"),
    ("rocky", "8", "2029-05-31"),
    ("rocky", "9", "2032-05-31"),
    ("almalinux", "8", "2029-05-31"),
    ("almalinux", "9", "2032-05-31"),
    ("alpine", "3.18", "2025-05-09"),
    ("alpine", "3.19", "2025-11-01"),
    ("alpine", "3.20", "2026-04-01"),
    ("alpine", "3.21", "2026-11-01"),
    ("alpine", "3.22", "2027-05-01"),
    ("amzn", "2", "2026-06-30"),
];

/// 发行版 EOL 日期（仅内置表命中时返回）。
pub fn os_eol(os_id: &str, version_id: &str) -> Option<NaiveDate> {
    let id = os_id.to_ascii_lowercase();
    let mut best: Option<(&str, NaiveDate)> = None;
    for (oid, prefix, eol) in OS_EOL {
        if *oid != id {
            continue;
        }
        let matches = version_id == *prefix
            || version_id
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('.'));
        if !matches {
            continue;
        }
        let date = NaiveDate::parse_from_str(eol, "%Y-%m-%d").ok()?;
        // 取最长前缀（`3.18` 优先于 `3`）
        let longer = match best {
            None => true,
            Some((p, _)) => prefix.len() > p.len(),
        };
        if longer {
            best = Some((prefix, date));
        }
    }
    best.map(|(_, d)| d)
}

/// sshd 有效全局配置：首个出现的键生效、大小写不敏感、`Match` 块之后的行不计入全局。
pub fn parse_sshd_config(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let key = parts.next().unwrap_or("").to_ascii_lowercase();
        if key == "match" {
            break;
        }
        let value = parts
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('=')
            .trim()
            .to_string();
        map.entry(key).or_insert(value);
    }
    map
}

/// 一个监听套接字（来自 `/proc/net/tcp{,6}`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listener {
    pub port: u16,
    /// 绑定在 `0.0.0.0` / `::` 上（对所有网卡开放）
    pub public: bool,
    pub ipv6: bool,
}

/// 解析 `/proc/net/tcp` / `tcp6` 中 `LISTEN`（状态 `0A`）的条目。
pub fn parse_proc_net_tcp(text: &str, ipv6: bool) -> Vec<Listener> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 || cols[3] != "0A" {
            continue;
        }
        let Some((addr, port_hex)) = cols[1].rsplit_once(':') else {
            continue;
        };
        let Ok(port) = u16::from_str_radix(port_hex, 16) else {
            continue;
        };
        let public = addr.chars().all(|c| c == '0');
        out.push(Listener { port, public, ipv6 });
    }
    out
}

/// 对公网暴露即高风险的服务端口 → 名称。
pub fn risky_port_name(port: u16) -> Option<&'static str> {
    Some(match port {
        2375 => "docker-api",
        3306 => "mysql",
        5432 => "postgresql",
        6379 => "redis",
        27017 => "mongodb",
        11211 => "memcached",
        9200 => "elasticsearch",
        5900 => "vnc",
        3389 => "rdp",
        2049 => "nfs",
        111 => "rpcbind",
        _ => return None,
    })
}

/// `/etc/passwd` 中 uid 为 0 且非 `root` 的账号。
pub fn uid0_accounts(passwd: &str) -> Vec<String> {
    passwd
        .lines()
        .filter_map(|l| {
            let cols: Vec<&str> = l.split(':').collect();
            (cols.len() >= 3 && cols[2] == "0" && cols[0] != "root").then(|| cols[0].to_string())
        })
        .collect()
}

/// `/etc/shadow` 中密码字段为空（免密登录）的账号。
pub fn empty_password_accounts(shadow: &str) -> Vec<String> {
    shadow
        .lines()
        .filter_map(|l| {
            let cols: Vec<&str> = l.split(':').collect();
            (cols.len() >= 2 && cols[1].is_empty()).then(|| cols[0].to_string())
        })
        .collect()
}

/// Ubuntu `update-notifier` 文本 → `(可更新数, 其中安全更新数)`。
pub fn parse_updates_available(text: &str) -> Option<(u32, u32)> {
    let mut total = None;
    let mut security = 0;
    for line in text.lines() {
        let nums: Vec<u32> = line
            .split_whitespace()
            .filter_map(|w| w.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok())
            .collect();
        let lower = line.to_ascii_lowercase();
        if lower.contains("security") {
            if let Some(n) = nums.first() {
                security = *n;
            }
        } else if (lower.contains("can be applied") || lower.contains("update")) && total.is_none()
        {
            total = nums.first().copied();
        }
    }
    total.map(|t| (t, security))
}

fn sysctl_value<'a>(facts: &'a SystemFacts, key: &str) -> Option<&'a str> {
    facts
        .sysctl
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.trim())
}

fn sysctl_int(facts: &SystemFacts, key: &str) -> Option<i64> {
    sysctl_value(facts, key).and_then(|v| v.split_whitespace().next()?.parse().ok())
}

// ───────────────────────────── 判定 ─────────────────────────────

/// 由原始事实生成内核信息。
pub fn kernel_info(facts: &SystemFacts, today: NaiveDate) -> KernelInfo {
    let osr = facts
        .os_release_file
        .as_deref()
        .map(parse_os_release)
        .unwrap_or_default();
    let release = facts
        .osrelease
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let newest_installed = facts
        .installed_kernels
        .iter()
        .max_by_key(|k| kernel_sort_key(k))
        .cloned();
    let newer_installed = newest_installed
        .as_deref()
        .is_some_and(|n| !release.is_empty() && kernel_sort_key(n) > kernel_sort_key(&release));
    let parsed = parse_kernel_release(&release);
    let (series, lts, series_eol) = match parsed {
        Some((ma, mi, _)) => {
            let status = kernel_series_status(ma, mi, today);
            let (lts, eol) = match status {
                KernelSeriesStatus::Lts { eol, .. } => (Some(true), Some(eol.to_string())),
                KernelSeriesStatus::NonLtsExpired => (Some(false), None),
                KernelSeriesStatus::Unknown => (None, None),
            };
            (Some(format!("{ma}.{mi}")), lts, eol)
        }
        None => (None, None, None),
    };
    KernelInfo {
        release,
        version: facts
            .proc_version
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
        arch: facts.arch.clone(),
        os_name: osr.get("PRETTY_NAME").cloned().unwrap_or_default(),
        os_id: osr.get("ID").cloned().unwrap_or_default(),
        os_version_id: osr.get("VERSION_ID").cloned().unwrap_or_default(),
        hostname: facts
            .hostname
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
        uptime_secs: facts.uptime_secs.unwrap_or(0),
        virtualization: facts.virtualization.clone(),
        reboot_required: facts.reboot_required_file || newer_installed,
        newest_installed_kernel: newest_installed,
        series,
        lts,
        series_eol,
    }
}

/// 全部检查项（顺序固定：内核 → CPU → 加固 → 访问 → 网络 → 更新 → 存储 → 面板）。
pub fn evaluate(facts: &SystemFacts, today: NaiveDate) -> Vec<Finding> {
    let mut out = Vec::new();
    let info = kernel_info(facts, today);
    check_kernel(facts, &info, today, &mut out);
    check_cpu(facts, &mut out);
    check_hardening(facts, &mut out);
    check_access(facts, &mut out);
    check_network(facts, &mut out);
    check_updates(facts, &info, &mut out);
    check_storage(facts, &mut out);
    check_panel(facts, &mut out);
    out
}

fn check_kernel(facts: &SystemFacts, info: &KernelInfo, today: NaiveDate, out: &mut Vec<Finding>) {
    // 内核系列生命周期
    let mut f = Finding::new(
        "kernel_eol",
        Category::Kernel,
        Severity::Medium,
        CheckStatus::Unknown,
    )
    .heuristic();
    match parse_kernel_release(&info.release) {
        None => {
            f = f.evidence(if info.release.is_empty() {
                "kernel release unavailable".to_string()
            } else {
                info.release.clone()
            });
        }
        Some((ma, mi, _)) => {
            let vendor = is_vendor_kernel(&info.release);
            match kernel_series_status(ma, mi, today) {
                KernelSeriesStatus::Lts { eol, expired } => {
                    f.evidence = format!("{} · LTS {ma}.{mi} · upstream EOL {eol}", info.release);
                    if expired && vendor {
                        f.status = CheckStatus::Pass;
                        f.severity = Severity::Info;
                        f.evidence.push_str(" · vendor-maintained kernel");
                    } else if expired {
                        f.status = CheckStatus::Fail;
                    } else {
                        f.status = CheckStatus::Pass;
                    }
                }
                KernelSeriesStatus::NonLtsExpired => {
                    f.evidence = format!("{} · non-LTS series {ma}.{mi}", info.release);
                    if vendor {
                        f.status = CheckStatus::Pass;
                        f.severity = Severity::Info;
                        f.evidence.push_str(" · vendor-maintained kernel");
                    } else {
                        f.status = CheckStatus::Fail;
                    }
                }
                KernelSeriesStatus::Unknown => {
                    f.evidence = format!(
                        "{} · series {ma}.{mi} newer than built-in table {DATA_VERSION}",
                        info.release
                    );
                }
            }
        }
    }
    out.push(f);

    // 发行版生命周期
    let mut f = Finding::new(
        "os_eol",
        Category::Kernel,
        Severity::High,
        CheckStatus::Unknown,
    )
    .heuristic();
    if info.os_id.is_empty() {
        f = f.evidence("os-release unavailable");
    } else {
        match os_eol(&info.os_id, &info.os_version_id) {
            Some(eol) => {
                f.status = if today > eol {
                    CheckStatus::Fail
                } else {
                    CheckStatus::Pass
                };
                f.evidence = format!("{} {} · EOL {eol}", info.os_id, info.os_version_id);
            }
            None => {
                f.evidence = format!(
                    "{} {} not in built-in table {DATA_VERSION}",
                    info.os_id, info.os_version_id
                );
            }
        }
    }
    out.push(f);

    // 待重启（内核已更新未生效）
    let mut f = Finding::new(
        "reboot_required",
        Category::Kernel,
        Severity::Medium,
        CheckStatus::Pass,
    );
    if info.reboot_required {
        f.status = CheckStatus::Fail;
        f.evidence = match &info.newest_installed_kernel {
            Some(n) if n != &info.release => {
                format!("running {} · installed {n}", info.release)
            }
            _ => "/var/run/reboot-required present".to_string(),
        };
    } else {
        f.evidence = format!("running {}", info.release);
    }
    out.push(f);

    // 强制访问控制
    let mut f = Finding::new(
        "mac_disabled",
        Category::Kernel,
        Severity::Low,
        CheckStatus::Unknown,
    );
    match (facts.apparmor_enabled, facts.selinux_enforcing) {
        (None, None) => f.evidence = "apparmor/selinux state unavailable".into(),
        (aa, se) => {
            let apparmor = aa.unwrap_or(false);
            let selinux = se.unwrap_or(false);
            f.status = if apparmor || selinux {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            };
            f.evidence = format!("apparmor={apparmor} selinux_enforcing={selinux}");
        }
    }
    out.push(f);
}

fn check_cpu(facts: &SystemFacts, out: &mut Vec<Finding>) {
    if facts.cpu_vulnerabilities.is_empty() {
        out.push(
            Finding::new(
                "cpu_vulnerability",
                Category::Cpu,
                Severity::High,
                CheckStatus::Unknown,
            )
            .evidence("/sys/devices/system/cpu/vulnerabilities unavailable"),
        );
        return;
    }
    let mut items: Vec<&(String, String)> = facts.cpu_vulnerabilities.iter().collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, content) in items {
        let content = content.trim();
        let lower = content.to_ascii_lowercase();
        let status = if lower.starts_with("vulnerable") {
            CheckStatus::Fail
        } else if lower.starts_with("mitigation") || lower.starts_with("not affected") {
            CheckStatus::Pass
        } else {
            CheckStatus::Unknown
        };
        out.push(
            Finding::new("cpu_vulnerability", Category::Cpu, Severity::High, status)
                .subject(name.clone())
                .evidence(content),
        );
    }
}

/// sysctl 加固基线：`(key, 期望, 严重度)`；`期望` 为闭区间 `[min, max]`。
const SYSCTL_BASELINE: &[(&str, i64, i64, Severity)] = &[
    ("kernel.randomize_va_space", 2, 2, Severity::High),
    ("kernel.kptr_restrict", 1, 2, Severity::Low),
    ("kernel.dmesg_restrict", 1, 1, Severity::Low),
    ("kernel.yama.ptrace_scope", 1, 3, Severity::Low),
    ("kernel.unprivileged_bpf_disabled", 1, 2, Severity::Low),
    ("fs.protected_symlinks", 1, 1, Severity::Medium),
    ("fs.protected_hardlinks", 1, 1, Severity::Medium),
    ("fs.suid_dumpable", 0, 0, Severity::Low),
    ("net.ipv4.tcp_syncookies", 1, 1, Severity::Medium),
    ("net.ipv4.conf.all.rp_filter", 1, 2, Severity::Low),
    ("net.ipv4.conf.all.accept_redirects", 0, 0, Severity::Low),
    ("net.ipv4.conf.all.send_redirects", 0, 0, Severity::Low),
    ("net.ipv4.conf.all.accept_source_route", 0, 0, Severity::Low),
];

/// 采集层需要读取的 sysctl 键（与基线表同源）。
pub fn sysctl_keys() -> Vec<&'static str> {
    SYSCTL_BASELINE.iter().map(|(k, _, _, _)| *k).collect()
}

fn check_hardening(facts: &SystemFacts, out: &mut Vec<Finding>) {
    for (key, min, max, severity) in SYSCTL_BASELINE {
        let mut f = Finding::new(
            "sysctl",
            Category::Hardening,
            *severity,
            CheckStatus::Unknown,
        )
        .subject(*key);
        match sysctl_int(facts, key) {
            Some(v) => {
                f.status = if (*min..=*max).contains(&v) {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                };
                f.evidence = if min == max {
                    format!("{key} = {v} (expected {min})")
                } else {
                    format!("{key} = {v} (expected {min}..{max})")
                };
            }
            None => {
                // 可选特性（yama / bpf）缺失是常态：不计入 unknown 噪音
                if *key == "kernel.yama.ptrace_scope" || *key == "kernel.unprivileged_bpf_disabled"
                {
                    continue;
                }
                f.evidence = format!("{key} unavailable");
            }
        }
        out.push(f);
    }
}

fn check_access(facts: &SystemFacts, out: &mut Vec<Finding>) {
    match facts.sshd_config.as_deref() {
        None => out.push(
            Finding::new(
                "sshd_config",
                Category::Access,
                Severity::Info,
                CheckStatus::Unknown,
            )
            .evidence("sshd_config not found"),
        ),
        Some(text) => {
            let cfg = parse_sshd_config(text);
            let get = |k: &str| cfg.get(k).map(|v| v.to_ascii_lowercase());

            let root = get("permitrootlogin").unwrap_or_else(|| "prohibit-password".into());
            let mut f = Finding::new(
                "sshd_root_login",
                Category::Access,
                Severity::High,
                CheckStatus::Pass,
            )
            .evidence(format!("PermitRootLogin {root}"));
            if root == "yes" {
                f.status = CheckStatus::Fail;
            }
            out.push(f);

            let pw = get("passwordauthentication").unwrap_or_else(|| "yes".into());
            let mut f = Finding::new(
                "sshd_password_auth",
                Category::Access,
                Severity::Medium,
                CheckStatus::Pass,
            )
            .evidence(format!("PasswordAuthentication {pw}"));
            if pw == "yes" {
                f.status = CheckStatus::Fail;
            }
            out.push(f);

            let empty = get("permitemptypasswords").unwrap_or_else(|| "no".into());
            let mut f = Finding::new(
                "sshd_empty_passwords",
                Category::Access,
                Severity::Critical,
                CheckStatus::Pass,
            )
            .evidence(format!("PermitEmptyPasswords {empty}"));
            if empty == "yes" {
                f.status = CheckStatus::Fail;
            }
            out.push(f);

            let port = get("port").unwrap_or_else(|| "22".into());
            let mut f = Finding::new(
                "sshd_default_port",
                Category::Access,
                Severity::Low,
                CheckStatus::Pass,
            )
            .evidence(format!("Port {port}"));
            if port == "22" {
                f.status = CheckStatus::Fail;
            }
            out.push(f);

            let tries: u32 = get("maxauthtries")
                .and_then(|v| v.parse().ok())
                .unwrap_or(6);
            let mut f = Finding::new(
                "sshd_max_auth_tries",
                Category::Access,
                Severity::Low,
                CheckStatus::Pass,
            )
            .evidence(format!("MaxAuthTries {tries}"));
            if tries > 6 {
                f.status = CheckStatus::Fail;
            }
            out.push(f);
        }
    }

    // uid 0 账号
    let mut f = Finding::new(
        "uid0_accounts",
        Category::Access,
        Severity::Critical,
        CheckStatus::Unknown,
    );
    if let Some(passwd) = facts.passwd.as_deref() {
        let extra = uid0_accounts(passwd);
        f.status = if extra.is_empty() {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        };
        f.evidence = if extra.is_empty() {
            "only root has uid 0".into()
        } else {
            extra.join(", ")
        };
    } else {
        f.evidence = "/etc/passwd unreadable".into();
    }
    out.push(f);

    // 空密码账号
    let mut f = Finding::new(
        "empty_password_accounts",
        Category::Access,
        Severity::Critical,
        CheckStatus::Unknown,
    );
    if let Some(shadow) = facts.shadow.as_deref() {
        let empty = empty_password_accounts(shadow);
        f.status = if empty.is_empty() {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        };
        f.evidence = if empty.is_empty() {
            "no empty password".into()
        } else {
            empty.join(", ")
        };
    } else {
        f.evidence = "/etc/shadow unreadable".into();
    }
    out.push(f);

    // 关键文件权限
    push_mode_check(
        out,
        "shadow_permissions",
        "/etc/shadow",
        facts.shadow_mode,
        0o007,
        Severity::High,
    );
    push_mode_check(
        out,
        "passwd_permissions",
        "/etc/passwd",
        facts.passwd_mode,
        0o022,
        Severity::High,
    );
    if facts.docker_sock_mode.is_some() {
        push_mode_check(
            out,
            "docker_socket_permissions",
            "/var/run/docker.sock",
            facts.docker_sock_mode,
            0o002,
            Severity::High,
        );
    }
}

fn push_mode_check(
    out: &mut Vec<Finding>,
    code: &str,
    path: &str,
    mode: Option<u32>,
    forbidden_bits: u32,
    severity: Severity,
) {
    let mut f = Finding::new(code, Category::Access, severity, CheckStatus::Unknown).subject(path);
    match mode {
        Some(m) => {
            let m = m & 0o7777;
            f.status = if m & forbidden_bits == 0 {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            };
            f.evidence = format!("{path} mode {m:04o}");
        }
        None => f.evidence = format!("{path} not found"),
    }
    out.push(f);
}

fn check_network(facts: &SystemFacts, out: &mut Vec<Finding>) {
    let mut f = Finding::new(
        "firewall_inactive",
        Category::Network,
        Severity::Medium,
        CheckStatus::Unknown,
    );
    match facts.firewall_active {
        Some(true) => {
            f.status = CheckStatus::Pass;
            f.evidence = format!(
                "{} active",
                facts.firewall_backend.as_deref().unwrap_or("firewall")
            );
        }
        Some(false) => {
            f.status = CheckStatus::Fail;
            f.evidence = "no host firewall (ufw / firewalld / nftables) active".into();
        }
        None => f.evidence = "firewall state unavailable".into(),
    }
    out.push(f);

    let mut listeners: Vec<Listener> = Vec::new();
    if let Some(t) = facts.proc_net_tcp.as_deref() {
        listeners.extend(parse_proc_net_tcp(t, false));
    }
    if let Some(t) = facts.proc_net_tcp6.as_deref() {
        listeners.extend(parse_proc_net_tcp(t, true));
    }
    if facts.proc_net_tcp.is_none() && facts.proc_net_tcp6.is_none() {
        out.push(
            Finding::new(
                "exposed_service",
                Category::Network,
                Severity::High,
                CheckStatus::Unknown,
            )
            .evidence("/proc/net/tcp unavailable"),
        );
    } else {
        check_listeners(facts, &listeners, out);
    }

    // 时间同步（证书 / 日志 / TOTP 都依赖）
    let mut f = Finding::new(
        "time_sync",
        Category::Network,
        Severity::Low,
        CheckStatus::Unknown,
    );
    match facts.time_synced {
        Some(true) => {
            f.status = CheckStatus::Pass;
            f.evidence = "NTP synchronized".into();
        }
        Some(false) => {
            f.status = CheckStatus::Fail;
            f.evidence = "clock not synchronized (no NTP client active)".into();
        }
        None => f.evidence = "time sync state unavailable".into(),
    }
    out.push(f);
}

fn check_listeners(facts: &SystemFacts, listeners: &[Listener], out: &mut Vec<Finding>) {
    let mut public_ports: Vec<u16> = listeners
        .iter()
        .filter(|l| l.public)
        .map(|l| l.port)
        .collect();
    public_ports.sort_unstable();
    public_ports.dedup();

    let mut risky = 0;
    for port in &public_ports {
        if let Some(name) = risky_port_name(*port) {
            risky += 1;
            let severity = if *port == 2375 || *port == 6379 {
                Severity::Critical
            } else {
                Severity::High
            };
            out.push(
                Finding::new(
                    "exposed_service",
                    Category::Network,
                    severity,
                    CheckStatus::Fail,
                )
                .subject(format!("{port}/tcp"))
                .evidence(format!("{name} listening on 0.0.0.0:{port}")),
            );
        }
    }
    if risky == 0 {
        out.push(
            Finding::new(
                "exposed_service",
                Category::Network,
                Severity::High,
                CheckStatus::Pass,
            )
            .evidence("no database / remote-admin service bound to 0.0.0.0"),
        );
    }
    // 公网监听端口总览（信息项）
    let ports_text = if public_ports.is_empty() {
        "none".to_string()
    } else {
        public_ports
            .iter()
            .map(|p| match facts.panel.panel_port {
                Some(pp) if pp == *p => format!("{p} (panel)"),
                _ => p.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push(
        Finding::new(
            "public_listeners",
            Category::Network,
            Severity::Info,
            CheckStatus::Pass,
        )
        .evidence(format!(
            "{} public listener(s): {ports_text}",
            public_ports.len()
        )),
    );
}

fn check_updates(facts: &SystemFacts, info: &KernelInfo, out: &mut Vec<Finding>) {
    let apt_based = matches!(
        info.os_id.as_str(),
        "ubuntu" | "debian" | "linuxmint" | "pop"
    );
    let mut f = Finding::new(
        "pending_security_updates",
        Category::Updates,
        Severity::Medium,
        CheckStatus::Unknown,
    );
    match facts
        .updates_available_text
        .as_deref()
        .and_then(parse_updates_available)
    {
        Some((total, security)) => {
            f.evidence = format!("{total} update(s) available, {security} security");
            if security > 0 {
                f.status = CheckStatus::Fail;
            } else if total > 0 {
                f.status = CheckStatus::Fail;
                f.severity = Severity::Low;
            } else {
                f.status = CheckStatus::Pass;
            }
        }
        None => {
            f.evidence = if apt_based {
                "update-notifier data unavailable".into()
            } else {
                "package manager not supported by this check".into()
            };
        }
    }
    out.push(f);

    let mut f = Finding::new(
        "unattended_upgrades",
        Category::Updates,
        Severity::Low,
        CheckStatus::Unknown,
    );
    match facts.unattended_upgrades {
        Some(true) => {
            f.status = CheckStatus::Pass;
            f.evidence = "automatic security updates enabled".into();
        }
        Some(false) => {
            f.status = CheckStatus::Fail;
            f.evidence = "automatic security updates disabled".into();
        }
        None => f.evidence = "automatic update state unavailable".into(),
    }
    out.push(f);
}

fn check_storage(facts: &SystemFacts, out: &mut Vec<Finding>) {
    let mut f = Finding::new(
        "root_disk_usage",
        Category::Storage,
        Severity::High,
        CheckStatus::Unknown,
    );
    match facts.root_disk_usage_percent {
        Some(p) => {
            f.evidence = format!("/ {p:.1}% used");
            if p >= 95.0 {
                f.status = CheckStatus::Fail;
            } else if p >= 85.0 {
                f.status = CheckStatus::Fail;
                f.severity = Severity::Medium;
            } else {
                f.status = CheckStatus::Pass;
            }
        }
        None => f.evidence = "disk usage unavailable".into(),
    }
    out.push(f);

    let mut f = Finding::new(
        "swap_missing",
        Category::Storage,
        Severity::Low,
        CheckStatus::Unknown,
    );
    match (facts.mem_total_mb, facts.swap_total_mb) {
        (Some(mem), Some(swap)) => {
            f.evidence = format!("memory {mem} MB · swap {swap} MB");
            f.status = if swap == 0 && mem < 2048 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            };
        }
        _ => f.evidence = "memory / swap totals unavailable".into(),
    }
    out.push(f);
}

fn check_panel(facts: &SystemFacts, out: &mut Vec<Finding>) {
    let panel = &facts.panel;

    let mut f = Finding::new(
        "panel_session_timeout",
        Category::Panel,
        Severity::Low,
        CheckStatus::Unknown,
    );
    match panel.session_timeout_minutes {
        Some(m) => {
            f.evidence = format!("session_timeout_minutes = {m}");
            f.status = if m > 720 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            };
        }
        None => f.evidence = "session_timeout_minutes unset".into(),
    }
    out.push(f);

    let mut f = Finding::new(
        "panel_metrics_token",
        Category::Panel,
        Severity::Low,
        CheckStatus::Pass,
    )
    .evidence("/metrics protected by token");
    if !panel.metrics_token_set {
        f.status = CheckStatus::Fail;
        f.evidence = "/metrics readable without token (OP_METRICS_TOKEN unset)".into();
    }
    out.push(f);

    let mut f = Finding::new(
        "panel_process_user",
        Category::Panel,
        Severity::Info,
        CheckStatus::Pass,
    );
    match panel.running_as_root {
        Some(true) => f.evidence = "panel runs as root (required for host management)".into(),
        Some(false) => f.evidence = "panel runs as non-root".into(),
        None => {
            f.status = CheckStatus::Unknown;
            f.evidence = "process uid unavailable".into();
        }
    }
    out.push(f);
}

/// 汇总评分：`fail` 项按严重度扣分，未知项不扣分但计数。
pub fn summarize(findings: &[Finding]) -> InspectionSummary {
    let mut s = InspectionSummary {
        total: findings.len() as u32,
        ..Default::default()
    };
    let mut penalty = 0u32;
    for f in findings {
        match f.status {
            CheckStatus::Pass => s.passed += 1,
            CheckStatus::Unknown => s.unknown += 1,
            CheckStatus::Fail => {
                s.failed += 1;
                penalty += f.severity.weight();
                match f.severity {
                    Severity::Critical => s.critical += 1,
                    Severity::High => s.high += 1,
                    Severity::Medium => s.medium += 1,
                    Severity::Low => s.low += 1,
                    Severity::Info => {}
                }
            }
        }
    }
    s.score = 100u32.saturating_sub(penalty);
    s.grade = match s.score {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        40..=59 => "D",
        _ => "F",
    }
    .to_string();
    s
}

/// 组装完整报告（应用层在采集后调用）。
pub fn build_report(
    facts: &SystemFacts,
    generated_at: DateTime<Utc>,
    duration_ms: u64,
) -> InspectionReport {
    let today = generated_at.date_naive();
    let findings = evaluate(facts, today);
    InspectionReport {
        generated_at,
        duration_ms,
        cache_age_secs: 0,
        data_version: DATA_VERSION.to_string(),
        kernel: kernel_info(facts, today),
        summary: summarize(&findings),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 24).unwrap()
    }

    fn find<'a>(findings: &'a [Finding], code: &str) -> &'a Finding {
        findings
            .iter()
            .find(|f| f.code == code)
            .unwrap_or_else(|| panic!("finding {code} missing"))
    }

    fn healthy_facts() -> SystemFacts {
        SystemFacts {
            proc_version: Some("Linux version 6.1.0-25-amd64 (debian-kernel@lists.debian.org) (gcc-12 (Debian 12.2.0-14) 12.2.0) #1 SMP".into()),
            osrelease: Some("6.1.0-25-amd64\n".into()),
            os_release_file: Some("PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"\nNAME=\"Debian GNU/Linux\"\nVERSION_ID=\"12\"\nID=debian\n".into()),
            hostname: Some("web-01\n".into()),
            uptime_secs: Some(86_400),
            arch: "x86_64".into(),
            virtualization: Some("kvm".into()),
            reboot_required_file: false,
            installed_kernels: vec!["6.1.0-25-amd64".into(), "6.1.0-23-amd64".into()],
            cpu_vulnerabilities: vec![
                ("meltdown".into(), "Mitigation: PTI\n".into()),
                ("spectre_v2".into(), "Mitigation: Retpolines; IBPB: conditional".into()),
                ("mds".into(), "Not affected".into()),
            ],
            sysctl: SYSCTL_BASELINE
                .iter()
                .map(|(k, min, _, _)| (k.to_string(), min.to_string()))
                .collect(),
            sshd_config: Some("Include /etc/ssh/sshd_config.d/*.conf\nPort 2222\nPermitRootLogin prohibit-password\nPasswordAuthentication no\nMatch User deploy\n    PasswordAuthentication yes\n".into()),
            passwd: Some("root:x:0:0:root:/root:/bin/bash\ndeploy:x:1000:1000::/home/deploy:/bin/bash\n".into()),
            shadow: Some("root:$6$abc:19000:0:99999:7:::\ndeploy:!:19000:0:99999:7:::\n".into()),
            shadow_mode: Some(0o640),
            passwd_mode: Some(0o644),
            docker_sock_mode: Some(0o660),
            proc_net_tcp: Some("  sl  local_address rem_address   st\n   0: 0100007F:0CEA 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 1 1 0000000000000000 100 0 0 10 0\n   1: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 2 1 0000000000000000 100 0 0 10 0\n".into()),
            proc_net_tcp6: Some("  sl  local_address rem_address st\n   0: 00000000000000000000000000000000:0016 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 3 1 0000000000000000 100 0 0 10 0\n".into()),
            firewall_active: Some(true),
            firewall_backend: Some("nftables".into()),
            apparmor_enabled: Some(true),
            selinux_enforcing: Some(false),
            time_synced: Some(true),
            updates_available_text: Some("0 updates can be applied immediately.\n".into()),
            unattended_upgrades: Some(true),
            root_disk_usage_percent: Some(42.5),
            mem_total_mb: Some(4096),
            swap_total_mb: Some(0),
            panel: PanelFacts {
                session_timeout_minutes: Some(120),
                metrics_token_set: true,
                running_as_root: Some(true),
                panel_port: Some(8080),
            },
        }
    }

    #[test]
    fn os_release_parsing_strips_quotes() {
        let m = parse_os_release(
            "ID=ubuntu\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\n# c\nVERSION_ID='24.04'\n",
        );
        assert_eq!(m["ID"], "ubuntu");
        assert_eq!(m["PRETTY_NAME"], "Ubuntu 24.04.1 LTS");
        assert_eq!(m["VERSION_ID"], "24.04");
    }

    #[test]
    fn kernel_release_parsing() {
        assert_eq!(parse_kernel_release("5.15.0-113-generic"), Some((5, 15, 0)));
        assert_eq!(parse_kernel_release("6.1.0-25-amd64"), Some((6, 1, 0)));
        assert_eq!(
            parse_kernel_release("4.18.0-553.el8_10.x86_64"),
            Some((4, 18, 0))
        );
        assert_eq!(parse_kernel_release("6.12"), Some((6, 12, 0)));
        assert_eq!(parse_kernel_release("garbage"), None);
    }

    #[test]
    fn kernel_series_lifecycle() {
        assert_eq!(
            kernel_series_status(6, 1, today()),
            KernelSeriesStatus::Lts {
                eol: NaiveDate::from_ymd_opt(2027, 12, 31).unwrap(),
                expired: false
            }
        );
        assert_eq!(
            kernel_series_status(5, 4, today()),
            KernelSeriesStatus::Lts {
                eol: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
                expired: true
            }
        );
        assert_eq!(
            kernel_series_status(6, 8, today()),
            KernelSeriesStatus::NonLtsExpired
        );
        assert_eq!(
            kernel_series_status(7, 3, today()),
            KernelSeriesStatus::Unknown
        );
    }

    #[test]
    fn vendor_kernels_downgrade_eol_to_info() {
        let mut facts = healthy_facts();
        facts.osrelease = Some("4.18.0-553.el8_10.x86_64".into());
        let findings = evaluate(&facts, today());
        let f = find(&findings, "kernel_eol");
        assert_eq!(f.status, CheckStatus::Pass);
        assert_eq!(f.severity, Severity::Info);
        assert!(f.evidence.contains("vendor-maintained"));
        assert!(f.heuristic);

        facts.osrelease = Some("6.8.9-custom".into());
        let findings = evaluate(&facts, today());
        let f = find(&findings, "kernel_eol");
        assert_eq!(f.status, CheckStatus::Fail);
        assert_eq!(f.severity, Severity::Medium);
    }

    #[test]
    fn os_eol_prefers_longest_prefix_and_handles_point_releases() {
        assert_eq!(
            os_eol("ubuntu", "22.04"),
            NaiveDate::from_ymd_opt(2027, 4, 30)
        );
        assert_eq!(os_eol("debian", "12"), NaiveDate::from_ymd_opt(2028, 6, 30));
        assert_eq!(
            os_eol("alpine", "3.18.4"),
            NaiveDate::from_ymd_opt(2025, 5, 9)
        );
        assert_eq!(os_eol("rocky", "9.4"), NaiveDate::from_ymd_opt(2032, 5, 31));
        assert_eq!(os_eol("fedora", "40"), None);
        // `12` 不能匹配 `120`
        assert_eq!(os_eol("debian", "120"), None);
    }

    #[test]
    fn sshd_config_first_wins_and_match_blocks_are_ignored() {
        let cfg = parse_sshd_config(
            "# comment\nPermitRootLogin no\npermitrootlogin yes\nPasswordAuthentication=no\nMatch Address 10.0.0.0/8\nPasswordAuthentication yes\n",
        );
        assert_eq!(cfg["permitrootlogin"], "no");
        assert_eq!(cfg["passwordauthentication"], "no");
    }

    #[test]
    fn proc_net_tcp_listeners() {
        let facts = healthy_facts();
        let v4 = parse_proc_net_tcp(facts.proc_net_tcp.as_deref().unwrap(), false);
        assert_eq!(
            v4,
            vec![
                Listener {
                    port: 3306,
                    public: false,
                    ipv6: false
                },
                Listener {
                    port: 8080,
                    public: true,
                    ipv6: false
                },
            ]
        );
        let v6 = parse_proc_net_tcp(facts.proc_net_tcp6.as_deref().unwrap(), true);
        assert_eq!(
            v6,
            vec![Listener {
                port: 22,
                public: true,
                ipv6: true
            }]
        );
    }

    #[test]
    fn account_parsers() {
        assert_eq!(
            uid0_accounts("root:x:0:0::/root:/bin/sh\ntoor:x:0:0::/root:/bin/sh\nbob:x:1000:1000::/home/bob:/bin/sh\n"),
            vec!["toor"]
        );
        assert_eq!(
            empty_password_accounts("root:$6$x:1:0:99999:7:::\nguest::1:0:99999:7:::\n"),
            vec!["guest"]
        );
    }

    #[test]
    fn update_notifier_parsing() {
        assert_eq!(
            parse_updates_available("\n12 updates can be applied immediately.\n5 of these updates are standard security updates.\n"),
            Some((12, 5))
        );
        assert_eq!(
            parse_updates_available("0 updates can be applied immediately.\n"),
            Some((0, 0))
        );
        assert_eq!(parse_updates_available(""), None);
    }

    #[test]
    fn healthy_system_scores_a_with_only_expected_failures() {
        let facts = healthy_facts();
        let report = build_report(&facts, Utc::now(), 12);
        let failed: Vec<&str> = report
            .findings
            .iter()
            .filter(|f| f.status == CheckStatus::Fail)
            .map(|f| f.code.as_str())
            .collect();
        assert!(failed.is_empty(), "unexpected failures: {failed:?}");
        assert_eq!(report.summary.score, 100);
        assert_eq!(report.summary.grade, "A");
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.total as usize, report.findings.len());
        assert_eq!(report.kernel.release, "6.1.0-25-amd64");
        assert_eq!(report.kernel.os_id, "debian");
        assert_eq!(report.kernel.hostname, "web-01");
        assert_eq!(report.kernel.series.as_deref(), Some("6.1"));
        assert_eq!(report.kernel.lts, Some(true));
        assert!(!report.kernel.reboot_required);
        assert_eq!(report.data_version, DATA_VERSION);
        // 信息项：公网监听端口带面板标注
        let listeners = find(&report.findings, "public_listeners");
        assert!(
            listeners.evidence.contains("8080 (panel)"),
            "{}",
            listeners.evidence
        );
        assert!(listeners.evidence.contains("22"));
    }

    #[test]
    fn risky_system_accumulates_findings_and_penalties() {
        let mut facts = healthy_facts();
        facts.osrelease = Some("5.4.0-150-custom".into());
        facts.os_release_file =
            Some("ID=ubuntu\nVERSION_ID=\"18.04\"\nPRETTY_NAME=\"Ubuntu 18.04.6 LTS\"\n".into());
        facts.reboot_required_file = true;
        facts.cpu_vulnerabilities.push((
            "spectre_v1".into(),
            "Vulnerable: __user pointer sanitization".into(),
        ));
        facts
            .sysctl
            .retain(|(k, _)| k != "kernel.randomize_va_space");
        facts
            .sysctl
            .push(("kernel.randomize_va_space".into(), "0".into()));
        facts.sshd_config = Some(
            "PermitRootLogin yes\nPasswordAuthentication yes\nPermitEmptyPasswords yes\n".into(),
        );
        facts.passwd = Some("root:x:0:0::/root:/bin/sh\ntoor:x:0:0::/root:/bin/sh\n".into());
        facts.shadow = Some("root:$6$x:1:0:99999:7:::\ntoor::1:0:99999:7:::\n".into());
        facts.shadow_mode = Some(0o644);
        facts.docker_sock_mode = Some(0o666);
        facts.proc_net_tcp = Some("  sl  local_address rem_address st\n   0: 00000000:0CEA 00000000:0000 0A 0 0 0 0 0 0\n   1: 00000000:18EB 00000000:0000 0A 0 0 0 0 0 0\n   2: 00000000:0947 00000000:0000 0A 0 0 0 0 0 0\n".into());
        facts.firewall_active = Some(false);
        facts.time_synced = Some(false);
        facts.updates_available_text = Some("30 updates can be applied immediately.\n7 of these updates are standard security updates.\n".into());
        facts.unattended_upgrades = Some(false);
        facts.root_disk_usage_percent = Some(96.0);
        facts.mem_total_mb = Some(1024);
        facts.panel.metrics_token_set = false;
        facts.panel.session_timeout_minutes = Some(10_080);

        let report = build_report(&facts, Utc::now(), 5);
        let f = &report.findings;

        assert_eq!(find(f, "kernel_eol").status, CheckStatus::Fail);
        assert_eq!(find(f, "os_eol").status, CheckStatus::Fail);
        assert_eq!(find(f, "reboot_required").status, CheckStatus::Fail);
        assert!(report.kernel.reboot_required);
        let spectre = f
            .iter()
            .find(|x| x.code == "cpu_vulnerability" && x.subject.as_deref() == Some("spectre_v1"))
            .unwrap();
        assert_eq!(spectre.status, CheckStatus::Fail);
        let aslr = f
            .iter()
            .find(|x| {
                x.code == "sysctl" && x.subject.as_deref() == Some("kernel.randomize_va_space")
            })
            .unwrap();
        assert_eq!(aslr.status, CheckStatus::Fail);
        assert_eq!(aslr.severity, Severity::High);
        assert_eq!(find(f, "sshd_root_login").status, CheckStatus::Fail);
        assert_eq!(find(f, "sshd_password_auth").status, CheckStatus::Fail);
        assert_eq!(find(f, "sshd_empty_passwords").status, CheckStatus::Fail);
        assert_eq!(find(f, "sshd_default_port").status, CheckStatus::Fail);
        assert_eq!(find(f, "uid0_accounts").evidence, "toor");
        assert_eq!(find(f, "empty_password_accounts").evidence, "toor");
        assert_eq!(find(f, "shadow_permissions").status, CheckStatus::Fail);
        assert_eq!(
            find(f, "docker_socket_permissions").status,
            CheckStatus::Fail
        );
        assert_eq!(find(f, "firewall_inactive").status, CheckStatus::Fail);
        let exposed: Vec<(&str, Severity)> = f
            .iter()
            .filter(|x| x.code == "exposed_service" && x.status == CheckStatus::Fail)
            .map(|x| (x.subject.as_deref().unwrap(), x.severity))
            .collect();
        assert_eq!(
            exposed,
            vec![
                ("2375/tcp", Severity::Critical),
                ("3306/tcp", Severity::High),
                ("6379/tcp", Severity::Critical)
            ]
        );
        assert_eq!(find(f, "time_sync").status, CheckStatus::Fail);
        let upd = find(f, "pending_security_updates");
        assert_eq!(upd.status, CheckStatus::Fail);
        assert_eq!(upd.severity, Severity::Medium);
        assert_eq!(find(f, "unattended_upgrades").status, CheckStatus::Fail);
        let disk = find(f, "root_disk_usage");
        assert_eq!(
            (disk.status, disk.severity),
            (CheckStatus::Fail, Severity::High)
        );
        assert_eq!(find(f, "swap_missing").status, CheckStatus::Fail);
        assert_eq!(find(f, "panel_metrics_token").status, CheckStatus::Fail);
        assert_eq!(find(f, "panel_session_timeout").status, CheckStatus::Fail);

        assert_eq!(report.summary.score, 0);
        assert_eq!(report.summary.grade, "F");
        assert!(report.summary.critical >= 4, "{:?}", report.summary);
        assert_eq!(
            report.summary.passed + report.summary.failed + report.summary.unknown,
            report.summary.total
        );
    }

    #[test]
    fn missing_facts_become_unknown_not_failures() {
        let facts = SystemFacts {
            arch: "aarch64".into(),
            panel: PanelFacts {
                metrics_token_set: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let report = build_report(&facts, Utc::now(), 1);
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.score, 100);
        assert!(report.summary.unknown > 10);
        assert_eq!(
            find(&report.findings, "cpu_vulnerability").status,
            CheckStatus::Unknown
        );
        assert_eq!(
            find(&report.findings, "sshd_config").status,
            CheckStatus::Unknown
        );
        assert_eq!(report.kernel.arch, "aarch64");
        assert_eq!(report.kernel.series, None);
        // 可选 sysctl（yama / bpf）缺失时不产生 unknown 噪音
        assert!(!report
            .findings
            .iter()
            .any(|f| f.subject.as_deref() == Some("kernel.yama.ptrace_scope")));
    }

    #[test]
    fn newer_installed_kernel_flags_reboot() {
        let mut facts = healthy_facts();
        facts.installed_kernels = vec!["6.1.0-25-amd64".into(), "6.1.0-27-amd64".into()];
        let report = build_report(&facts, Utc::now(), 1);
        assert!(report.kernel.reboot_required);
        assert_eq!(
            report.kernel.newest_installed_kernel.as_deref(),
            Some("6.1.0-27-amd64")
        );
        let f = find(&report.findings, "reboot_required");
        assert_eq!(f.status, CheckStatus::Fail);
        assert!(f.evidence.contains("installed 6.1.0-27-amd64"));
    }

    #[test]
    fn disk_usage_thresholds() {
        let mut facts = healthy_facts();
        facts.root_disk_usage_percent = Some(88.0);
        let findings = evaluate(&facts, today());
        let f = find(&findings, "root_disk_usage");
        assert_eq!(
            (f.status, f.severity),
            (CheckStatus::Fail, Severity::Medium)
        );
    }

    #[test]
    fn summary_grades() {
        let mk = |sev: Severity, n: usize| -> Vec<Finding> {
            (0..n)
                .map(|_| Finding::new("x", Category::Panel, sev, CheckStatus::Fail))
                .collect()
        };
        assert_eq!(summarize(&[]).grade, "A");
        assert_eq!(summarize(&mk(Severity::Low, 1)).score, 97);
        assert_eq!(summarize(&mk(Severity::Medium, 2)).grade, "B");
        assert_eq!(summarize(&mk(Severity::High, 2)).grade, "C");
        assert_eq!(summarize(&mk(Severity::Critical, 2)).grade, "D");
        assert_eq!(summarize(&mk(Severity::Critical, 5)).score, 0);
    }
}
