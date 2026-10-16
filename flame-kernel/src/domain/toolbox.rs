//! 系统工具箱 · 进程管理（S-12 · Doc/30 M-C）的**纯领域逻辑**。
//!
//! - [`ProcessInfo`]：一条进程记录（基础设施层从 `/proc` 采集，线程已剔除）；
//! - [`apply_query`]：关键字过滤 / 排序 / 截断，纯函数、可离线单测；
//! - [`protection_reason`]：结束进程前的保护判定——init（pid 1）、内核线程、面板自身及其祖先
//!   进程一律拒绝，避免\"一键把面板 / 整机杀掉\"；
//! - [`ProcessSignal`]：只允许 4 个语义明确的信号，不接受任意数字信号。
//!
//! 采集端口是 [`ProcessSource`]，由 `infrastructure::metrics::MetricsHub` 实现（与仪表盘进程 TOP
//! 共用同一个 `sysinfo::System`，TTL 内只扫一次 `/proc`）。
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 列表缺省条数
pub const DEFAULT_PROCESS_LIMIT: usize = 100;
/// 列表最大条数（前端虚拟滚动之外的兜底上限）
pub const MAX_PROCESS_LIMIT: usize = 500;
/// 关键字最大长度（超出截断，防止超长查询串）
pub const MAX_QUERY_LEN: usize = 128;
/// 命令行最大长度（超出截断，附 `…`）
pub const MAX_COMMAND_LEN: usize = 1024;
/// 祖先链最大回溯深度（防御 ppid 环）
const MAX_ANCESTOR_DEPTH: usize = 64;

/// 一条进程记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProcessInfo {
    pub pid: u32,
    /// 父进程 pid（内核根进程无）
    pub ppid: Option<u32>,
    /// 进程名（`/proc/<pid>/comm`）
    pub name: String,
    /// 属主用户名（无法解析时为 None）
    pub user: Option<String>,
    /// 属主 uid
    pub uid: Option<u32>,
    /// 命令行（参数以空格拼接，最长 1024 字符；内核线程为 `[name]`）
    pub command: String,
    /// CPU 占用（%，多核可超过 100）
    pub cpu: f32,
    /// 常驻内存（字节）
    pub memory_bytes: u64,
    /// 进程状态（Run / Sleep / Zombie …）
    pub status: String,
    /// 启动时间（Unix 秒）；结束进程时用于防 pid 复用
    pub started_at: u64,
    /// 是否内核线程
    pub kernel_thread: bool,
}

/// 结束进程被拒绝的原因（前端据此禁用按钮并提示）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedReason {
    /// pid 0 / 1（init / systemd）
    Init,
    /// 内核线程（kthreadd 及其子线程）
    KernelThread,
    /// 面板进程自身
    Panel,
    /// 面板进程的祖先（杀掉会连带面板退出，如容器 shim / 启动 shell）
    PanelAncestor,
}

impl ProtectedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ProtectedReason::Init => "init",
            ProtectedReason::KernelThread => "kernel_thread",
            ProtectedReason::Panel => "panel",
            ProtectedReason::PanelAncestor => "panel_ancestor",
        }
    }
}

/// 允许发送的信号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProcessSignal {
    /// SIGTERM：请求优雅退出（缺省）
    #[default]
    Term,
    /// SIGKILL：强制结束
    Kill,
    /// SIGHUP：通常用于重新加载配置
    Hup,
    /// SIGINT：等同 Ctrl-C
    Int,
}

impl ProcessSignal {
    /// `kill -s <NAME>` 使用的信号名
    pub fn as_str(self) -> &'static str {
        match self {
            ProcessSignal::Term => "TERM",
            ProcessSignal::Kill => "KILL",
            ProcessSignal::Hup => "HUP",
            ProcessSignal::Int => "INT",
        }
    }
}

/// 排序字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    Pid,
    Name,
    Started,
}

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// 列表查询（已规整：关键字小写去空白、limit 夹在 1..=500）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcessQuery {
    /// 关键字：匹配 pid（精确）/ 进程名 / 用户 / 命令行（不区分大小写）
    pub q: Option<String>,
    /// 仅某用户的进程（精确匹配）
    pub user: Option<String>,
    pub sort: ProcessSort,
    pub order: SortOrder,
    pub limit: usize,
    /// 是否包含内核线程（缺省不含）
    pub include_kernel: bool,
}

impl ProcessQuery {
    /// 规整原始参数。
    pub fn normalized(
        q: Option<&str>,
        user: Option<&str>,
        sort: Option<ProcessSort>,
        order: Option<SortOrder>,
        limit: Option<usize>,
        include_kernel: bool,
    ) -> Self {
        let clean = |s: Option<&str>| {
            s.map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(MAX_QUERY_LEN).collect::<String>())
        };
        Self {
            q: clean(q).map(|s| s.to_lowercase()),
            user: clean(user),
            sort: sort.unwrap_or_default(),
            order: order.unwrap_or_default(),
            limit: limit
                .unwrap_or(DEFAULT_PROCESS_LIMIT)
                .clamp(1, MAX_PROCESS_LIMIT),
            include_kernel,
        }
    }
}

/// 过滤 + 排序结果：`matched` 为截断前命中数，`items` 已截断到 `limit`。
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub matched: usize,
    pub items: Vec<ProcessInfo>,
}

/// 进程采集端口（`MetricsHub` 实现；测试可注入 [`StaticProcessSource`]）。
#[async_trait]
pub trait ProcessSource: Send + Sync {
    /// 进程表快照（已剔除用户态线程）。`fresh = true` 时忽略缓存重新扫描。
    async fn processes(&self, fresh: bool) -> Arc<Vec<ProcessInfo>>;
    /// 让下一次读取重新扫描（结束进程后调用，列表立即反映结果）。
    fn invalidate(&self) {}
}

/// 固定进程表（测试 / 演示）。
pub struct StaticProcessSource(pub Vec<ProcessInfo>);

#[async_trait]
impl ProcessSource for StaticProcessSource {
    async fn processes(&self, _fresh: bool) -> Arc<Vec<ProcessInfo>> {
        Arc::new(self.0.clone())
    }
}

// ───────────────────────────── 纯函数 ─────────────────────────────

fn matches(p: &ProcessInfo, q: &ProcessQuery) -> bool {
    if p.kernel_thread && !q.include_kernel {
        return false;
    }
    if let Some(user) = &q.user {
        if p.user.as_deref() != Some(user.as_str()) {
            return false;
        }
    }
    match &q.q {
        None => true,
        Some(needle) => {
            if let Ok(pid) = needle.parse::<u32>() {
                if p.pid == pid {
                    return true;
                }
            }
            p.name.to_lowercase().contains(needle.as_str())
                || p.command.to_lowercase().contains(needle.as_str())
                || p.user
                    .as_deref()
                    .is_some_and(|u| u.to_lowercase().contains(needle.as_str()))
        }
    }
}

fn compare(a: &ProcessInfo, b: &ProcessInfo, sort: ProcessSort) -> Ordering {
    match sort {
        ProcessSort::Cpu => a.cpu.partial_cmp(&b.cpu).unwrap_or(Ordering::Equal),
        ProcessSort::Memory => a.memory_bytes.cmp(&b.memory_bytes),
        ProcessSort::Pid => a.pid.cmp(&b.pid),
        ProcessSort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        ProcessSort::Started => a.started_at.cmp(&b.started_at),
    }
}

/// 过滤 → 排序 → 截断。相同排序键时按 pid 升序，结果稳定可测。
pub fn apply_query(processes: &[ProcessInfo], query: &ProcessQuery) -> QueryResult {
    let mut items: Vec<ProcessInfo> = processes
        .iter()
        .filter(|p| matches(p, query))
        .cloned()
        .collect();
    let matched = items.len();
    items.sort_by(|a, b| {
        let ord = compare(a, b, query.sort);
        let ord = match query.order {
            SortOrder::Asc => ord,
            SortOrder::Desc => ord.reverse(),
        };
        ord.then_with(|| a.pid.cmp(&b.pid))
    });
    items.truncate(query.limit.max(1));
    QueryResult { matched, items }
}

/// 面板进程的祖先 pid 集合（沿 ppid 回溯到 init 为止，不含 0 / 1 与自身）。
pub fn ancestors_of(pid: u32, processes: &[ProcessInfo]) -> HashSet<u32> {
    let parent_of = |pid: u32| processes.iter().find(|p| p.pid == pid).and_then(|p| p.ppid);
    let mut out = HashSet::new();
    let mut cur = pid;
    for _ in 0..MAX_ANCESTOR_DEPTH {
        match parent_of(cur) {
            Some(ppid) if ppid > 1 && ppid != pid && out.insert(ppid) => cur = ppid,
            _ => break,
        }
    }
    out
}

/// 结束进程保护判定：`None` 表示允许。
pub fn protection_reason(
    p: &ProcessInfo,
    panel_pid: u32,
    panel_ancestors: &HashSet<u32>,
) -> Option<ProtectedReason> {
    if p.pid <= 1 {
        Some(ProtectedReason::Init)
    } else if p.pid == 2 || p.kernel_thread || p.ppid == Some(2) {
        Some(ProtectedReason::KernelThread)
    } else if p.pid == panel_pid {
        Some(ProtectedReason::Panel)
    } else if panel_ancestors.contains(&p.pid) {
        Some(ProtectedReason::PanelAncestor)
    } else {
        None
    }
}

/// 视为敏感的参数键片段（小写匹配）。
const SECRET_KEYS: &[&str] = &[
    "password",
    "passwd",
    "requirepass",
    "pwd",
    "secret",
    "token",
    "apikey",
    "api-key",
    "api_key",
    "access-key",
    "access_key",
    "credential",
    "auth",
];
/// 脱敏占位
pub const REDACTED: &str = "***";

fn is_secret_key(key: &str) -> bool {
    let k = key.trim_start_matches('-').to_ascii_lowercase();
    !k.is_empty() && SECRET_KEYS.iter().any(|s| k.contains(s))
}

/// `scheme://user:pass@host` → `scheme://user:***@host`
fn redact_url_password(arg: &str) -> Option<String> {
    let scheme_end = arg.find("://")? + 3;
    let rest = &arg[scheme_end..];
    let at = rest.find('@')?;
    let userinfo = &rest[..at];
    if userinfo.contains('/') {
        return None;
    }
    let colon = userinfo.find(':')?;
    Some(format!(
        "{}{}:{REDACTED}{}",
        &arg[..scheme_end],
        &userinfo[..colon],
        &rest[at..]
    ))
}

/// 命令行参数脱敏：`--password=x` / `DB_TOKEN=x` / `--secret x` / URL 内嵌口令一律替换为 `***`。
///
/// 进程列表对持有 `toolbox:read` 的角色（含 viewer）可见，而命令行常带口令（`mysql -u root
/// --password=...`、`redis-server --requirepass ...`），因此在采集时即脱敏，缓存里不留明文。
pub fn redact_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut mask_next = false;
    for arg in args {
        if mask_next {
            mask_next = false;
            if !arg.starts_with('-') {
                out.push(REDACTED.to_string());
                continue;
            }
        }
        if let Some((key, _)) = arg.split_once('=') {
            if is_secret_key(key) {
                out.push(format!("{key}={REDACTED}"));
                continue;
            }
        } else if arg.starts_with('-') && is_secret_key(arg) {
            // `--password value` / `--requirepass value`：值在下一个参数
            mask_next = true;
        }
        match redact_url_password(arg) {
            Some(masked) => out.push(masked),
            None => out.push(arg.clone()),
        }
    }
    out
}

/// 命令行拼接：先脱敏，参数以空格连接并截断；为空（内核线程 / 僵尸）时回落为 `[name]`。
pub fn format_command(args: &[String], name: &str) -> String {
    let joined = redact_args(args)
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() {
        return format!("[{name}]");
    }
    if joined.chars().count() > MAX_COMMAND_LEN {
        let mut s: String = joined.chars().take(MAX_COMMAND_LEN).collect();
        s.push('…');
        s
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, ppid: Option<u32>, name: &str, cpu: f32, mem: u64) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid,
            name: name.into(),
            user: Some(if pid < 100 { "root" } else { "www-data" }.into()),
            uid: Some(if pid < 100 { 0 } else { 33 }),
            command: format!("/usr/bin/{name} --flag"),
            cpu,
            memory_bytes: mem,
            status: "Sleep".into(),
            started_at: 1_700_000_000 + pid as u64,
            kernel_thread: false,
        }
    }

    fn table() -> Vec<ProcessInfo> {
        let mut kworker = p(40, Some(2), "kworker/0:1", 0.0, 0);
        kworker.kernel_thread = true;
        kworker.command = "[kworker/0:1]".into();
        vec![
            p(1, None, "systemd", 0.1, 10 << 20),
            kworker,
            p(50, Some(1), "sshd", 0.0, 8 << 20),
            p(51, Some(50), "bash", 0.0, 4 << 20),
            p(120, Some(51), "flamepanel", 2.0, 60 << 20),
            p(300, Some(1), "nginx", 12.5, 30 << 20),
            p(301, Some(300), "Nginx-worker", 12.5, 20 << 20),
            p(400, Some(1), "node", 55.0, 300 << 20),
        ]
    }

    #[test]
    fn query_normalization_trims_lowercases_and_clamps() {
        let q = ProcessQuery::normalized(Some("  NGINX "), Some(" "), None, None, Some(0), false);
        assert_eq!(q.q.as_deref(), Some("nginx"));
        assert_eq!(q.user, None);
        assert_eq!(q.limit, 1);
        assert_eq!(q.sort, ProcessSort::Cpu);
        assert_eq!(q.order, SortOrder::Desc);
        let q = ProcessQuery::normalized(None, None, None, None, Some(10_000), false);
        assert_eq!(q.limit, MAX_PROCESS_LIMIT);
        let long = "x".repeat(500);
        let q = ProcessQuery::normalized(Some(&long), None, None, None, None, false);
        assert_eq!(q.q.unwrap().len(), MAX_QUERY_LEN);
        assert_eq!(
            ProcessQuery::normalized(None, None, None, None, None, false).limit,
            DEFAULT_PROCESS_LIMIT
        );
    }

    #[test]
    fn default_query_hides_kernel_threads_and_sorts_by_cpu_desc() {
        let q = ProcessQuery::normalized(None, None, None, None, None, false);
        let r = apply_query(&table(), &q);
        assert_eq!(r.matched, 7);
        let pids: Vec<u32> = r.items.iter().map(|p| p.pid).collect();
        // cpu 相同（300 / 301）时按 pid 升序稳定
        assert_eq!(pids[..4], [400, 300, 301, 120]);
        assert!(!pids.contains(&40));

        let q = ProcessQuery::normalized(None, None, None, None, None, true);
        assert_eq!(apply_query(&table(), &q).matched, 8);
    }

    #[test]
    fn keyword_matches_pid_name_command_and_user_case_insensitively() {
        let find = |kw: &str| {
            let q = ProcessQuery::normalized(
                Some(kw),
                None,
                Some(ProcessSort::Pid),
                Some(SortOrder::Asc),
                None,
                false,
            );
            apply_query(&table(), &q)
                .items
                .iter()
                .map(|p| p.pid)
                .collect::<Vec<_>>()
        };
        assert_eq!(find("nginx"), vec![300, 301]);
        assert_eq!(find("400"), vec![400]);
        assert_eq!(find("--FLAG").len(), 7);
        assert_eq!(find("www"), vec![120, 300, 301, 400]);
        assert!(find("no-such-thing").is_empty());
    }

    #[test]
    fn user_filter_sorting_variants_and_limit() {
        let q = ProcessQuery::normalized(
            None,
            Some("root"),
            Some(ProcessSort::Memory),
            Some(SortOrder::Desc),
            None,
            false,
        );
        let r = apply_query(&table(), &q);
        assert_eq!(
            r.items.iter().map(|p| p.pid).collect::<Vec<_>>(),
            vec![1, 50, 51]
        );

        let q = ProcessQuery::normalized(
            None,
            None,
            Some(ProcessSort::Name),
            Some(SortOrder::Asc),
            Some(3),
            false,
        );
        let r = apply_query(&table(), &q);
        assert_eq!(r.matched, 7, "matched 为截断前命中数");
        assert_eq!(
            r.items.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            vec!["bash", "flamepanel", "nginx"]
        );

        let q = ProcessQuery::normalized(
            None,
            None,
            Some(ProcessSort::Started),
            Some(SortOrder::Desc),
            Some(1),
            false,
        );
        assert_eq!(apply_query(&table(), &q).items[0].pid, 400);
    }

    #[test]
    fn ancestors_walk_ppid_chain_and_survive_cycles() {
        let t = table();
        let anc = ancestors_of(120, &t);
        assert_eq!(anc, HashSet::from([51, 50]));
        assert!(ancestors_of(999, &t).is_empty());

        let cyclic = vec![
            p(200, Some(201), "a", 0.0, 0),
            p(201, Some(200), "b", 0.0, 0),
        ];
        assert_eq!(ancestors_of(200, &cyclic), HashSet::from([201]));
    }

    #[test]
    fn protection_covers_init_kernel_panel_and_ancestors() {
        let t = table();
        let anc = ancestors_of(120, &t);
        let reason = |pid: u32| {
            let proc = t.iter().find(|p| p.pid == pid).unwrap();
            protection_reason(proc, 120, &anc)
        };
        assert_eq!(reason(1), Some(ProtectedReason::Init));
        assert_eq!(reason(40), Some(ProtectedReason::KernelThread));
        assert_eq!(reason(120), Some(ProtectedReason::Panel));
        assert_eq!(reason(51), Some(ProtectedReason::PanelAncestor));
        assert_eq!(reason(50), Some(ProtectedReason::PanelAncestor));
        assert_eq!(reason(300), None);
        assert_eq!(reason(400), None);
        let kthreadd = p(2, Some(0), "kthreadd", 0.0, 0);
        assert_eq!(
            protection_reason(&kthreadd, 120, &anc),
            Some(ProtectedReason::KernelThread)
        );
        assert_eq!(ProtectedReason::PanelAncestor.as_str(), "panel_ancestor");
    }

    #[test]
    fn signal_names_and_serde_are_uppercase() {
        assert_eq!(ProcessSignal::default(), ProcessSignal::Term);
        assert_eq!(ProcessSignal::Kill.as_str(), "KILL");
        assert_eq!(
            serde_json::to_string(&ProcessSignal::Hup).unwrap(),
            "\"HUP\""
        );
        let s: ProcessSignal = serde_json::from_str("\"INT\"").unwrap();
        assert_eq!(s, ProcessSignal::Int);
        assert!(serde_json::from_str::<ProcessSignal>("\"9\"").is_err());
    }

    #[test]
    fn secrets_in_command_line_are_redacted() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            redact_args(&args(&["mysqld", "--user=mysql", "--password=hunter2"])),
            args(&["mysqld", "--user=mysql", "--password=***"])
        );
        assert_eq!(
            redact_args(&args(&[
                "redis-server",
                "--requirepass",
                "s3cr3t",
                "--port",
                "6379"
            ])),
            args(&["redis-server", "--requirepass", "***", "--port", "6379"])
        );
        assert_eq!(
            redact_args(&args(&["env", "DB_PASSWORD=x", "API_TOKEN=y", "PATH=/bin"])),
            args(&["env", "DB_PASSWORD=***", "API_TOKEN=***", "PATH=/bin"])
        );
        assert_eq!(
            redact_args(&args(&["app", "--db", "postgres://app:pw@db:5432/x"])),
            args(&["app", "--db", "postgres://app:***@db:5432/x"])
        );
        // 值缺失（下一个仍是选项）时不误吞
        assert_eq!(
            redact_args(&args(&["x", "--token", "--verbose"])),
            args(&["x", "--token", "--verbose"])
        );
        // 普通 URL / 参数原样保留
        assert_eq!(
            redact_args(&args(&["curl", "https://example.com/a@b", "-v"])),
            args(&["curl", "https://example.com/a@b", "-v"])
        );
        assert_eq!(
            format_command(&args(&["a", "--secret=z"]), "a"),
            "a --secret=***"
        );
    }

    #[test]
    fn command_formatting_falls_back_and_truncates() {
        assert_eq!(format_command(&[], "kworker/0:1"), "[kworker/0:1]");
        assert_eq!(
            format_command(&["nginx:".into(), "".into(), "worker".into()], "nginx"),
            "nginx: worker"
        );
        let long = vec!["a".repeat(MAX_COMMAND_LEN + 10)];
        let out = format_command(&long, "x");
        assert_eq!(out.chars().count(), MAX_COMMAND_LEN + 1);
        assert!(out.ends_with('…'));
    }
}
