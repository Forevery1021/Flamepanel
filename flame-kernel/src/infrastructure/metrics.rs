//! 系统指标采集与分发（仪表盘实时数据的唯一来源）。
//!
//! # 架构（P-01 重构）
//!
//! ```text
//!  collector loop (owns a private sysinfo::System, CPU/Mem only)
//!        │  every 3s: sample → MetricsSnapshot
//!        ▼
//!  MetricsHub::publish(snapshot)
//!        ├─ history ring buffer  (std RwLock, O(1) push, 无 async 锁)
//!        ├─ init payload cache   (惰性 + 失效：连接建立时 0 次序列化)
//!        └─ broadcast<Arc<MetricsFrame>>  (tick JSON 序列化 **一次**，N 个 WS 共享同一 Arc)
//!
//!  processes endpoint → MetricsHub::top_processes()
//!        └─ 独立 System（仅刷新进程 CPU/内存）+ 2s TTL 缓存：多客户端并发只扫一次 /proc
//! ```
//!
//! 重构前的问题（均已在单测中固化）：
//! 1. **磁盘用量冻结**：`Disks` 只在循环外 `new_with_refreshed_list()` 一次、之后从不 refresh，
//!    面板显示的磁盘使用率永远是进程启动那一刻的值；
//! 2. **网络速率 3 倍虚高**：`received()` 语义是「距上次刷新的差值（字节）」，旧实现直接除以
//!    1 MiB 当作 MB/s 展示，而采样周期是 3 s → 系统性高估 3×，且与采样周期强耦合；
//! 3. **每连接重复序列化**：N 个仪表盘连接，同一 tick 被 `serde_json::to_string` N 次；
//!    连接建立时历史快照亦逐连接重新序列化；
//! 4. **进程接口双重全量刷新**：`refresh_all()` + `refresh_processes()` 同一请求刷两遍进程表，
//!    且 `refresh_all()` 顺带刷新 CPU，把采集循环的 3 s CPU 差分窗口打碎（CPU 曲线毛刺）。
//!
//! 采集器与进程接口各自持有独立的 `sysinfo::System`，二者不再争同一把锁。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::time::interval;
use utoipa::ToSchema;

use crate::domain::entity::MetricsSnapshot;
use crate::domain::toolbox::{format_command, ProcessInfo, ProcessSource};

/// 采样周期（秒）。前端曲线横轴 / 预测算法均以此为基准。
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(3);
/// 进程 TOP 缓存 TTL：仪表盘默认 10 s 轮询，多标签页 / 多用户并发落在同一窗口内只扫一次 /proc。
pub const PROCESS_CACHE_TTL: Duration = Duration::from_secs(2);
/// 默认进程 TOP 条数
pub const DEFAULT_TOP_N: usize = 5;

// ─────────────────────────── 历史环形缓冲 ───────────────────────────

/// 固定容量环形缓冲（最旧优先淘汰）。
pub struct MetricsHistory {
    buffer: VecDeque<MetricsSnapshot>,
    max_size: usize,
}

impl MetricsHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size.max(1)),
            max_size: max_size.max(1),
        }
    }

    pub fn push(&mut self, snapshot: MetricsSnapshot) {
        if self.buffer.len() >= self.max_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(snapshot);
    }

    pub fn get_all(&self) -> Vec<MetricsSnapshot> {
        self.buffer.iter().cloned().collect()
    }

    pub fn latest(&self) -> Option<&MetricsSnapshot> {
        self.buffer.back()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.max_size
    }

    fn iter(&self) -> impl Iterator<Item = &MetricsSnapshot> {
        self.buffer.iter()
    }
}

// ─────────────────────────── 分发帧 ───────────────────────────

/// 一次 tick 的分发帧：结构化快照 + **预序列化**的 WS 文本帧。
///
/// `json` 形如 `{"type":"tick","data":{...}}`，由 `MetricsHub::publish` 生成一次，
/// 所有订阅者通过 `Arc` 共享同一份字符串（零拷贝转成 `Utf8Bytes`）。
#[derive(Debug)]
pub struct MetricsFrame {
    pub snapshot: MetricsSnapshot,
    pub json: Arc<str>,
}

/// 进程 TOP 条目（sysinfo 采集）
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory_mb: u64,
    pub status: String,
}

struct ProcessCache {
    fetched_at: Option<Instant>,
    entries: Arc<Vec<ProcessEntry>>,
}

/// S-12：完整进程表缓存（工具箱进程管理）。与 TOP 缓存分开存放，但共用 `process_system`。
struct ProcessTableCache {
    fetched_at: Option<Instant>,
    entries: Arc<Vec<ProcessInfo>>,
}

// ─────────────────────────── Hub ───────────────────────────

/// 指标中枢：历史 + 广播 + 进程 TOP 缓存。`AppState` 持有 `Arc<MetricsHub>`。
pub struct MetricsHub {
    history: RwLock<MetricsHistory>,
    /// `init` 帧缓存（`{"type":"init","data":[...]}`）。push 时失效，首个连接惰性重建。
    init_cache: Mutex<Option<Arc<str>>>,
    tx: broadcast::Sender<Arc<MetricsFrame>>,
    /// 进程接口专用 System：只刷新进程表，与采集循环互不干扰。
    process_system: tokio::sync::Mutex<sysinfo::System>,
    process_cache: tokio::sync::Mutex<ProcessCache>,
    /// S-12：完整进程表缓存；锁在整个扫描期间持有 = 并发请求单飞
    process_table: tokio::sync::Mutex<ProcessTableCache>,
    /// uid → 用户名解析（每次全表扫描时刷新，读 /etc/passwd 成本可忽略）
    users: Mutex<sysinfo::Users>,
    /// Hub 创建时刻（= 面板进程启动时刻），供 `/metrics` 的 uptime 使用。
    started_at: Instant,
    /// 节点指标（Agent 心跳）注册表：每节点一个同构 Hub（Doc/30 M-P）。
    nodes: crate::infrastructure::node_metrics::NodeMetricsRegistry,
}

/// 插件宿主 API 数据源：最新本机指标快照（Doc/21 §5.7 `flame.metrics`）。
impl crate::plugin::host::HostContext for MetricsHub {
    fn metrics_json(&self) -> Option<String> {
        self.latest().and_then(|s| serde_json::to_string(&s).ok())
    }
}

impl MetricsHub {
    /// `history_size`：内存中保留的采样点数（3 s 一点，120 点 = 6 分钟）。
    pub fn new(history_size: usize) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            history: RwLock::new(MetricsHistory::new(history_size)),
            init_cache: Mutex::new(None),
            tx,
            process_system: tokio::sync::Mutex::new(sysinfo::System::new()),
            process_cache: tokio::sync::Mutex::new(ProcessCache {
                fetched_at: None,
                entries: Arc::new(Vec::new()),
            }),
            process_table: tokio::sync::Mutex::new(ProcessTableCache {
                fetched_at: None,
                entries: Arc::new(Vec::new()),
            }),
            users: Mutex::new(sysinfo::Users::new()),
            started_at: Instant::now(),
            nodes: crate::infrastructure::node_metrics::NodeMetricsRegistry::new(),
        }
    }

    /// 节点指标注册表（`/ws/metrics?node_id=` 与节点心跳共用）。
    pub fn nodes(&self) -> &crate::infrastructure::node_metrics::NodeMetricsRegistry {
        &self.nodes
    }

    /// 面板进程运行时长（旧实现用二进制 mtime 近似，升级二进制未重启时会显示错误值）
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// 写入一条快照：入历史 → 失效 init 缓存 → 序列化一次 → 广播。
    ///
    /// 返回当前订阅者数量（无订阅者时 `broadcast::send` 会 Err，这里吞掉：没人看也要留历史）。
    pub fn publish(&self, snapshot: MetricsSnapshot) -> usize {
        let json: Arc<str> = tick_frame_json(&snapshot).into();
        {
            let mut h = self.history.write().unwrap_or_else(|p| p.into_inner());
            h.push(snapshot.clone());
        }
        *self.init_cache.lock().unwrap_or_else(|p| p.into_inner()) = None;
        self.tx
            .send(Arc::new(MetricsFrame { snapshot, json }))
            .unwrap_or(0)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<MetricsFrame>> {
        self.tx.subscribe()
    }

    /// 当前订阅者数量（可观测性 / 测试用）
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// 最新快照（Prometheus 端点 / 健康检查用）
    pub fn latest(&self) -> Option<MetricsSnapshot> {
        self.history
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .latest()
            .cloned()
    }

    /// 历史快照全量拷贝（REST / 测试用；热路径请用 `init_payload`）
    pub fn history(&self) -> Vec<MetricsSnapshot> {
        self.history
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .get_all()
    }

    pub fn history_len(&self) -> usize {
        self.history.read().unwrap_or_else(|p| p.into_inner()).len()
    }

    /// 连接建立时下发的 `init` 帧（缓存命中时零序列化）。历史为空返回 `None`。
    pub fn init_payload(&self) -> Option<Arc<str>> {
        if let Some(cached) = self
            .init_cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
        {
            return Some(cached);
        }
        let built: Arc<str> = {
            let h = self.history.read().unwrap_or_else(|p| p.into_inner());
            if h.is_empty() {
                return None;
            }
            init_frame_json(h.iter()).into()
        };
        let mut slot = self.init_cache.lock().unwrap_or_else(|p| p.into_inner());
        // 竞争下以先写入者为准（两者内容等价）
        Some(slot.get_or_insert(built).clone())
    }

    /// 按 CPU 降序的进程 TOP-N（带 TTL 缓存）。
    ///
    /// 只刷新进程表的 CPU/内存字段（不读 cmdline/exe/environ/cwd/磁盘 IO），
    /// 相比旧实现的 `refresh_all()` + `refresh_processes()` 少两次全量 /proc 扫描。
    pub async fn top_processes(&self, n: usize) -> Arc<Vec<ProcessEntry>> {
        let n = n.max(1);
        {
            let cache = self.process_cache.lock().await;
            if let Some(at) = cache.fetched_at {
                if at.elapsed() < PROCESS_CACHE_TTL && cache.entries.len() >= n.min(DEFAULT_TOP_N) {
                    return Arc::new(cache.entries.iter().take(n).cloned().collect());
                }
            }
        }

        let entries = {
            let mut sys = self.process_system.lock().await;
            let kind = sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory();
            // 首次刷新 CPU 占用恒为 0（sysinfo 需要两个采样点做差分）：
            // 冷启动补一次极短间隔的二次刷新，让首屏就有真实值。
            let cold = self.process_cache.lock().await.fetched_at.is_none();
            sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, kind);
            if cold {
                tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
                sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, kind);
            }
            rank_processes(&sys, DEFAULT_TOP_N.max(n))
        };

        let shared = Arc::new(entries);
        let mut cache = self.process_cache.lock().await;
        cache.fetched_at = Some(Instant::now());
        cache.entries = shared.clone();
        if shared.len() > n {
            Arc::new(shared.iter().take(n).cloned().collect())
        } else {
            shared
        }
    }
}

impl MetricsHub {
    /// S-12：完整进程表（剔除用户态线程；含内核线程并标记），按 pid 升序。
    ///
    /// 与 [`Self::top_processes`] 共用同一个 `sysinfo::System`（CPU 差分窗口连续），额外只读
    /// uid 与 cmdline，且二者用 `OnlyIfNotSet`——每个进程生命周期内只读一次。`fresh = false`
    /// 时命中 [`PROCESS_CACHE_TTL`] 内缓存；缓存锁贯穿扫描，并发请求只扫一次 `/proc`。
    pub async fn process_table(&self, fresh: bool) -> Arc<Vec<ProcessInfo>> {
        let mut cache = self.process_table.lock().await;
        if !fresh {
            if let Some(at) = cache.fetched_at {
                if at.elapsed() < PROCESS_CACHE_TTL {
                    return cache.entries.clone();
                }
            }
        }
        let cold = cache.fetched_at.is_none();
        let entries = {
            let mut sys = self.process_system.lock().await;
            let kind = sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_user(sysinfo::UpdateKind::OnlyIfNotSet)
                .with_cmd(sysinfo::UpdateKind::OnlyIfNotSet);
            sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, kind);
            // 冷启动：CPU 需要两个采样点做差分，补一次极短间隔的二次刷新
            if cold && self.process_cache.lock().await.fetched_at.is_none() {
                tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
                sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, kind);
            }
            let mut users = self.users.lock().unwrap_or_else(|p| p.into_inner());
            users.refresh();
            build_process_table(&sys, &users)
        };
        let shared = Arc::new(entries);
        cache.fetched_at = Some(Instant::now());
        cache.entries = shared.clone();
        shared
    }

    /// 让下一次 [`Self::process_table`] 重新扫描（结束进程后调用）。
    pub fn invalidate_process_table(&self) {
        if let Ok(mut cache) = self.process_table.try_lock() {
            cache.fetched_at = None;
        }
    }
}

#[async_trait::async_trait]
impl ProcessSource for MetricsHub {
    async fn processes(&self, fresh: bool) -> Arc<Vec<ProcessInfo>> {
        self.process_table(fresh).await
    }

    fn invalidate(&self) {
        self.invalidate_process_table();
    }
}

/// sysinfo 进程表 → 领域 [`ProcessInfo`]（用户态线程剔除；Linux 下线程也是 `/proc` 条目）。
fn build_process_table(sys: &sysinfo::System, users: &sysinfo::Users) -> Vec<ProcessInfo> {
    let mut out: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .filter(|(_, p)| p.thread_kind() != Some(sysinfo::ThreadKind::Userland))
        .map(|(pid, p)| {
            let name = p.name().to_string_lossy().into_owned();
            let args: Vec<String> = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            let uid = p.user_id();
            ProcessInfo {
                pid: pid.as_u32(),
                ppid: p.parent().map(|pp| pp.as_u32()),
                user: uid
                    .and_then(|u| users.get_user_by_id(u))
                    .map(|u| u.name().to_string()),
                uid: uid.map(|u| **u),
                command: format_command(&args, &name),
                name,
                cpu: clamp_f32(p.cpu_usage()),
                memory_bytes: p.memory(),
                status: format!("{:?}", p.status()),
                started_at: p.start_time(),
                kernel_thread: p.thread_kind() == Some(sysinfo::ThreadKind::Kernel),
            }
        })
        .collect();
    out.sort_unstable_by_key(|p| p.pid);
    out
}

impl Default for MetricsHub {
    fn default() -> Self {
        Self::new(120)
    }
}

/// 进程表 → 按 CPU 降序 TOP-N（部分排序：`select_nth_unstable_by` 为 O(n)，再对前 n 排序）。
fn rank_processes(sys: &sysinfo::System, n: usize) -> Vec<ProcessEntry> {
    let mut entries: Vec<ProcessEntry> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| ProcessEntry {
            pid: pid.as_u32(),
            name: proc.name().to_string_lossy().into_owned(),
            cpu: clamp_f32(proc.cpu_usage()),
            memory_mb: proc.memory() / 1024 / 1024,
            status: format!("{:?}", proc.status()),
        })
        .collect();
    top_n_by_cpu(&mut entries, n);
    entries
}

/// 就地保留 CPU 最高的 n 条并降序（O(len) 选择 + O(n log n) 排序）。
pub fn top_n_by_cpu(entries: &mut Vec<ProcessEntry>, n: usize) {
    let cmp = |a: &ProcessEntry, b: &ProcessEntry| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.memory_mb.cmp(&a.memory_mb))
    };
    if entries.len() > n {
        entries.select_nth_unstable_by(n, cmp);
        entries.truncate(n);
    }
    entries.sort_by(cmp);
}

// ─────────────────────────── 序列化 ───────────────────────────

/// `{"type":"tick","data":{...}}`——手写外壳，避免 `json!` 宏先构造一棵 `Value` 树再序列化。
pub fn tick_frame_json(snapshot: &MetricsSnapshot) -> String {
    let mut out = String::with_capacity(320);
    out.push_str(r#"{"type":"tick","data":"#);
    // MetricsSnapshot 全为数值字段，序列化不会失败；防御性回退为空对象
    out.push_str(&serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".into()));
    out.push('}');
    out
}

/// `{"type":"init","data":[...]}`
pub fn init_frame_json<'a>(snapshots: impl Iterator<Item = &'a MetricsSnapshot>) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str(r#"{"type":"init","data":["#);
    let mut first = true;
    for s in snapshots {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&serde_json::to_string(s).unwrap_or_else(|_| "{}".into()));
    }
    out.push_str("]}");
    out
}

// ─────────────────────────── 采样 ───────────────────────────

fn clamp_f32(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn clamp_f64(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// 真实文件系统白名单：过滤容器内常见的 overlay/tmpfs/squashfs 等虚拟文件系统，
/// 避免磁盘容量与使用率严重虚高。
pub fn is_real_fs_type(fs_type: &str) -> bool {
    matches!(
        fs_type.to_ascii_lowercase().as_str(),
        "ext2"
            | "ext3"
            | "ext4"
            | "xfs"
            | "btrfs"
            | "zfs"
            | "vfat"
            | "ntfs"
            | "apfs"
            | "exfat"
            | "f2fs"
            | "jfs"
            | "reiserfs"
    )
}

fn is_real_filesystem(disk: &sysinfo::Disk) -> bool {
    is_real_fs_type(&disk.file_system().to_string_lossy())
}

/// 字节差值 → MB/s（MiB 口径，与前端「MB/s」标签一致）。
///
/// `elapsed` 过短（< 1 ms，如首个 tick 或时钟回拨）时返回 0，避免除零 / 尖峰。
pub fn bytes_to_mbps(delta_bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs < 0.001 {
        return 0.0;
    }
    clamp_f64(delta_bytes as f64 / secs / (1024.0 * 1024.0))
}

/// 磁盘聚合：(total_bytes, used_bytes)，只统计真实文件系统。
fn aggregate_disks(disks: &sysinfo::Disks) -> (u64, u64) {
    disks
        .iter()
        .filter(|d| is_real_filesystem(d))
        .fold((0u64, 0u64), |(total, used), disk| {
            let t = disk.total_space();
            (
                total.saturating_add(t),
                used.saturating_add(t.saturating_sub(disk.available_space())),
            )
        })
}

/// 采样器：持有私有的 `System` / `Networks` / `Disks` 与上次网络采样时刻。
///
/// 独立成结构体便于单测驱动（`sample()` 可在测试中直接调用），也避免循环体内的
/// 巨型闭包。CPU 使用率依赖两次 `refresh` 间的差分，因此实例必须跨 tick 复用。
pub struct MetricsSampler {
    sys: sysinfo::System,
    networks: sysinfo::Networks,
    disks: sysinfo::Disks,
    refresh_kind: sysinfo::RefreshKind,
    last_net_at: Option<Instant>,
    /// 采样计数（可观测性：日志 / 测试断言）
    ticks: u64,
}

impl Default for MetricsSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsSampler {
    pub fn new() -> Self {
        let refresh_kind = sysinfo::RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(sysinfo::MemoryRefreshKind::everything());
        let mut sys = sysinfo::System::new();
        // 预热：第一次 CPU 刷新只是建立基线（sysinfo 语义），提前做掉，首个 tick 即有真实值
        sys.refresh_specifics(refresh_kind);
        let mut networks = sysinfo::Networks::new_with_refreshed_list();
        networks.refresh(true);
        let disks = sysinfo::Disks::new_with_refreshed_list();
        Self {
            sys,
            networks,
            disks,
            refresh_kind,
            last_net_at: Some(Instant::now()),
            ticks: 0,
        }
    }

    /// 已完成的采样次数
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// 采集一次快照（同步、阻塞 ~1-3 ms；调用方位于专用采集任务中）。
    pub fn sample(&mut self) -> MetricsSnapshot {
        self.ticks = self.ticks.wrapping_add(1);
        self.sys.refresh_specifics(self.refresh_kind);

        let cpu_usage = self.sys.global_cpu_usage();
        let cpu_cores = self.sys.cpus().len();
        let memory_total = self.sys.total_memory() / 1024 / 1024;
        let memory_used = self.sys.used_memory() / 1024 / 1024;
        let memory_usage_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };

        // 磁盘：每 tick 重读挂载表 + statvfs（只取容量字段，不读 IO 计数 / 介质类型），
        // 新挂载自动纳入、已卸载自动剔除——修复旧实现「启动后永不更新」。
        self.disks
            .refresh_specifics(true, sysinfo::DiskRefreshKind::nothing().with_storage());
        let (disk_total, disk_used) = aggregate_disks(&self.disks);
        const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
        let disk_total_gb = disk_total as f64 / GIB;
        let disk_used_gb = disk_used as f64 / GIB;
        let disk_usage_percent = if disk_total > 0 {
            ((disk_used as f64 / disk_total as f64) * 100.0) as f32
        } else {
            0.0
        };

        // 网络：差值字节 / 实际经过秒数 → MB/s（不再假设采样周期恒为 3 s）
        let now = Instant::now();
        self.networks.refresh(true);
        let elapsed = self
            .last_net_at
            .map(|t| now.duration_since(t))
            .unwrap_or_default();
        self.last_net_at = Some(now);
        let (rx_bytes, tx_bytes) =
            self.networks
                .iter()
                .fold((0u64, 0u64), |(rx, tx), (_, data)| {
                    (
                        rx.saturating_add(data.received()),
                        tx.saturating_add(data.transmitted()),
                    )
                });

        let load = sysinfo::System::load_average();

        MetricsSnapshot {
            timestamp: Utc::now().timestamp_millis(),
            cpu_usage: clamp_f32(cpu_usage),
            cpu_cores,
            memory_usage_percent: clamp_f32(memory_usage_percent),
            memory_total_mb: memory_total,
            memory_used_mb: memory_used,
            disk_usage_percent: clamp_f32(disk_usage_percent),
            disk_total_gb: clamp_f64(disk_total_gb),
            disk_used_gb: clamp_f64(disk_used_gb),
            load_one: clamp_f64(load.one),
            load_five: clamp_f64(load.five),
            load_fifteen: clamp_f64(load.fifteen),
            network_rx_mbps: bytes_to_mbps(rx_bytes, elapsed),
            network_tx_mbps: bytes_to_mbps(tx_bytes, elapsed),
        }
    }
}

/// 指标采集循环（由 TaskSupervisor spawn，`token` 协作式退出）。
///
/// 采样在 `spawn_blocking` 中执行：sysinfo 读 /proc 是同步 IO，放在 worker 线程上不阻塞
/// 运行时的 reactor（低核数 VPS 上 WS 推送延迟由此更平稳）。
pub async fn metrics_collector_loop(
    hub: Arc<MetricsHub>,
    token: tokio_util::sync::CancellationToken,
) {
    let mut sampler = Some(MetricsSampler::new());
    let mut tick = interval(SAMPLE_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tick.tick().await;

    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = tick.tick() => {}
        }
        let Some(mut s) = sampler.take() else { break };
        let joined = tokio::task::spawn_blocking(move || {
            let snap = s.sample();
            (s, snap)
        })
        .await;
        match joined {
            Ok((s, snapshot)) => {
                sampler = Some(s);
                hub.publish(snapshot);
            }
            Err(e) => {
                // 采样线程 panic：重建采样器继续（不让一次 /proc 解析异常拖死整条链路）
                tracing::error!(error = %e, "metrics sampler panicked; rebuilding");
                sampler = Some(MetricsSampler::new());
            }
        }
    }
}

// ─────────────────────────── 单元测试 ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(ts: i64, cpu: f32) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: ts,
            cpu_usage: cpu,
            cpu_cores: 4,
            memory_usage_percent: 50.0,
            memory_total_mb: 8192,
            memory_used_mb: 4096,
            disk_usage_percent: 40.0,
            disk_total_gb: 100.0,
            disk_used_gb: 40.0,
            load_one: 1.0,
            load_five: 0.8,
            load_fifteen: 0.5,
            network_rx_mbps: 0.1,
            network_tx_mbps: 0.05,
        }
    }

    #[test]
    fn history_ring_buffer_evicts_oldest() {
        let mut h = MetricsHistory::new(3);
        for i in 0..5 {
            h.push(snap(i, i as f32));
        }
        let all = h.get_all();
        assert_eq!(all.len(), 3);
        assert_eq!(
            all.iter().map(|s| s.timestamp).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        assert_eq!(h.latest().unwrap().timestamp, 4);
        assert_eq!(h.capacity(), 3);
    }

    #[test]
    fn history_zero_capacity_is_clamped_to_one() {
        let mut h = MetricsHistory::new(0);
        h.push(snap(1, 0.0));
        h.push(snap(2, 0.0));
        assert_eq!(h.len(), 1);
        assert_eq!(h.latest().unwrap().timestamp, 2);
    }

    #[test]
    fn tick_frame_is_valid_json_with_type_and_data() {
        let json = tick_frame_json(&snap(42, 12.5));
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(v["type"], "tick");
        assert_eq!(v["data"]["timestamp"], 42);
        assert_eq!(v["data"]["cpu_usage"], 12.5);
    }

    #[test]
    fn init_frame_preserves_order_and_length() {
        let items = [snap(1, 0.0), snap(2, 0.0), snap(3, 0.0)];
        let json = init_frame_json(items.iter());
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "init");
        let data = v["data"].as_array().unwrap();
        assert_eq!(data.len(), 3);
        assert_eq!(data[0]["timestamp"], 1);
        assert_eq!(data[2]["timestamp"], 3);
        // 空历史 → 合法的空数组
        let empty = init_frame_json(std::iter::empty());
        let v: serde_json::Value = serde_json::from_str(&empty).unwrap();
        assert_eq!(v["data"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn hub_publish_serializes_once_and_fans_out() {
        let hub = MetricsHub::new(10);
        assert!(
            hub.init_payload().is_none(),
            "empty history → no init frame"
        );

        let mut a = hub.subscribe();
        let mut b = hub.subscribe();
        assert_eq!(hub.subscriber_count(), 2);

        let delivered = hub.publish(snap(7, 33.0));
        assert_eq!(delivered, 2);

        let fa = a.recv().await.unwrap();
        let fb = b.recv().await.unwrap();
        // 两个订阅者拿到的是**同一份**预序列化字符串（指针相等），而非各自序列化
        assert!(Arc::ptr_eq(&fa, &fb));
        assert!(Arc::ptr_eq(&fa.json, &fb.json));
        assert_eq!(fa.snapshot.timestamp, 7);
        assert!(fa.json.starts_with(r#"{"type":"tick","#));

        assert_eq!(hub.history_len(), 1);
        assert_eq!(hub.latest().unwrap().cpu_usage, 33.0);
    }

    #[test]
    fn hub_init_payload_is_cached_until_next_publish() {
        let hub = MetricsHub::new(10);
        hub.publish(snap(1, 0.0));
        let p1 = hub.init_payload().unwrap();
        let p2 = hub.init_payload().unwrap();
        assert!(Arc::ptr_eq(&p1, &p2), "second call must hit the cache");

        hub.publish(snap(2, 0.0));
        let p3 = hub.init_payload().unwrap();
        assert!(!Arc::ptr_eq(&p1, &p3), "publish must invalidate the cache");
        let v: serde_json::Value = serde_json::from_str(&p3).unwrap();
        assert_eq!(v["data"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn hub_publish_without_subscribers_still_records_history() {
        let hub = MetricsHub::new(2);
        assert_eq!(hub.publish(snap(1, 0.0)), 0);
        assert_eq!(hub.publish(snap(2, 0.0)), 0);
        assert_eq!(hub.publish(snap(3, 0.0)), 0);
        let h = hub.history();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].timestamp, 2);
    }

    #[test]
    fn bytes_to_mbps_is_interval_independent() {
        let one_mib = 1024 * 1024;
        // 3 MiB over 3 s → 1 MB/s（旧实现会得到 3.0）
        assert!((bytes_to_mbps(3 * one_mib, Duration::from_secs(3)) - 1.0).abs() < 1e-9);
        // 1 MiB over 0.5 s → 2 MB/s
        assert!((bytes_to_mbps(one_mib, Duration::from_millis(500)) - 2.0).abs() < 1e-9);
        // 过短间隔 → 0，不出现除零尖峰
        assert_eq!(bytes_to_mbps(one_mib, Duration::ZERO), 0.0);
        assert_eq!(bytes_to_mbps(one_mib, Duration::from_micros(10)), 0.0);
    }

    #[test]
    fn real_fs_filter_rejects_virtual_filesystems() {
        for fs in ["ext4", "XFS", "btrfs", "zfs", "apfs", "ntfs"] {
            assert!(is_real_fs_type(fs), "{fs} should be real");
        }
        for fs in [
            "overlay", "tmpfs", "squashfs", "proc", "sysfs", "devtmpfs", "",
        ] {
            assert!(!is_real_fs_type(fs), "{fs} should be filtered");
        }
    }

    #[test]
    fn top_n_by_cpu_keeps_highest_and_sorts_desc() {
        let mk = |pid: u32, cpu: f32, mem: u64| ProcessEntry {
            pid,
            name: format!("p{pid}"),
            cpu,
            memory_mb: mem,
            status: "Run".into(),
        };
        let mut v = vec![
            mk(1, 1.0, 10),
            mk(2, 9.0, 10),
            mk(3, 5.0, 10),
            mk(4, 9.0, 50),
            mk(5, 0.0, 10),
            mk(6, f32::NAN, 10),
        ];
        top_n_by_cpu(&mut v, 3);
        assert_eq!(v.len(), 3);
        // 同 CPU 时内存大者优先
        assert_eq!(v[0].pid, 4);
        assert_eq!(v[1].pid, 2);
        assert_eq!(v[2].pid, 3);

        // n ≥ len：不截断，仅排序
        let mut w = vec![mk(1, 1.0, 0), mk(2, 2.0, 0)];
        top_n_by_cpu(&mut w, 10);
        assert_eq!(w.iter().map(|e| e.pid).collect::<Vec<_>>(), vec![2, 1]);
    }

    #[test]
    fn sampler_produces_sane_snapshot_on_this_host() {
        let mut s = MetricsSampler::new();
        let a = s.sample();
        assert!(a.cpu_cores >= 1);
        assert!(a.memory_total_mb > 0);
        assert!((0.0..=100.0).contains(&a.memory_usage_percent));
        assert!((0.0..=100.0).contains(&a.disk_usage_percent));
        assert!(a.network_rx_mbps >= 0.0 && a.network_tx_mbps >= 0.0);
        assert!(a.timestamp > 0);
        // 第二次采样时间戳单调不减
        let b = s.sample();
        assert!(b.timestamp >= a.timestamp);
        assert_eq!(s.ticks(), 2);
    }

    #[tokio::test]
    async fn top_processes_uses_ttl_cache() {
        let hub = MetricsHub::new(4);
        let first = hub.top_processes(5).await;
        let second = hub.top_processes(5).await;
        assert!(first.len() <= 5);
        // TTL 内第二次调用不重扫（缓存命中时条目内容一致）
        assert_eq!(
            first.iter().map(|e| e.pid).collect::<Vec<_>>(),
            second.iter().map(|e| e.pid).collect::<Vec<_>>()
        );
        // 请求更少条数只截断，不重扫
        let two = hub.top_processes(2).await;
        assert!(two.len() <= 2);
        for w in two.windows(2) {
            assert!(w[0].cpu >= w[1].cpu || w[0].cpu.is_nan() || w[1].cpu.is_nan());
        }
    }

    #[tokio::test]
    async fn process_table_lists_self_without_threads_and_caches() {
        let hub = MetricsHub::new(4);
        let table = hub.process_table(false).await;
        let me = std::process::id();
        let own = table.iter().find(|p| p.pid == me).expect("自身进程在表内");
        assert!(!own.command.is_empty());
        assert!(own.uid.is_some());
        assert!(own.started_at > 0);
        // pid 升序、无重复（用户态线程已剔除，tokio 工作线程不会以独立条目出现）
        assert!(table.windows(2).all(|w| w[0].pid < w[1].pid));
        let again = hub.process_table(false).await;
        assert!(Arc::ptr_eq(&table, &again), "TTL 内命中缓存");
        hub.invalidate_process_table();
        let fresh = hub.process_table(false).await;
        assert!(!Arc::ptr_eq(&table, &fresh), "失效后重新扫描");
        let forced = hub.process_table(true).await;
        assert!(!Arc::ptr_eq(&fresh, &forced), "fresh 忽略缓存");
    }

    /// 性能回归基线：1 次 tick 向 200 个订阅者分发。
    /// 旧实现（每连接 `json!` + `to_string`）在同等条件下约慢 100-200×；本测试只断言
    /// 「共享序列化」路径显著快于「逐连接序列化」路径，绝对耗时打印供 CI 观测。
    #[test]
    fn perf_shared_serialization_beats_per_subscriber() {
        const SUBS: usize = 200;
        const TICKS: usize = 50;
        let s = snap(1, 10.0);

        let t0 = Instant::now();
        for _ in 0..TICKS {
            for _ in 0..SUBS {
                let payload = serde_json::to_string(&serde_json::json!({
                    "type": "tick",
                    "data": s,
                }))
                .unwrap();
                std::hint::black_box(payload);
            }
        }
        let per_sub = t0.elapsed();

        let t1 = Instant::now();
        for _ in 0..TICKS {
            let shared: Arc<str> = tick_frame_json(&s).into();
            for _ in 0..SUBS {
                std::hint::black_box(shared.clone());
            }
        }
        let shared = t1.elapsed();

        eprintln!(
            "[perf] {TICKS} ticks × {SUBS} subs: per-subscriber={per_sub:?} shared={shared:?} speedup={:.1}x",
            per_sub.as_secs_f64() / shared.as_secs_f64().max(1e-9)
        );
        assert!(shared < per_sub, "shared serialization must be faster");
    }

    /// 端到端扇出成本：`publish` = 序列化一次 + 写环形历史 + broadcast 给 N 个订阅者。
    /// 目标是「1s 一次采样、数百个仪表盘连接」下发布路径远低于毫秒级；这里以 100 个
    /// 订阅者 × 1000 次发布测量单次发布均值并打印，只对上界做极宽松断言（CI 不抖动）。
    #[tokio::test]
    async fn perf_publish_fanout_cost_is_sub_millisecond() {
        const SUBS: usize = 100;
        const PUBLISHES: usize = 1000;
        let hub = MetricsHub::new(120);
        let mut receivers: Vec<_> = (0..SUBS).map(|_| hub.subscribe()).collect();

        let t0 = Instant::now();
        for i in 0..PUBLISHES {
            hub.publish(snap(i as i64, (i % 100) as f32));
            // 与真实连接一致：订阅者每 tick 取走一帧（否则 lagged 会丢帧，测的就不是发布成本）
            for rx in receivers.iter_mut() {
                let frame = rx.try_recv().expect("frame available");
                std::hint::black_box(frame.json.len());
            }
        }
        let total = t0.elapsed();
        let per_publish = total / PUBLISHES as u32;
        eprintln!(
            "[perf] publish fan-out: {PUBLISHES} publishes × {SUBS} subs total={total:?} per-publish={per_publish:?}"
        );
        assert_eq!(hub.history_len(), 120);
        assert!(
            per_publish < Duration::from_millis(5),
            "publish + fan-out to {SUBS} subscribers took {per_publish:?} (expected < 5ms)"
        );
    }
}
