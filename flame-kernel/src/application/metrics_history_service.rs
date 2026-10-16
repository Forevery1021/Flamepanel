//! 监控历史持久化（S-07 · Doc/30 M-P）。
//!
//! `MetricsHub` 只保留 120 点（6 min）内存环形历史；本服务作为 Hub 的一个 **broadcast
//! 订阅者** 把 3 s 快照在内存里聚合成 **1 min 桶**（均值 / 峰值）落库，并在每个整点把
//! 上一小时的 1 min 点二次汇总成 **1 h 桶**，供 `GET /api/metrics/history?range=` 回看
//! 1 h … 30 d 的趋势。
//!
//! 设计要点：
//! - **写放大极低**：每分钟一行 1 min 点、每小时一行 1 h 点；SQLite 写池零压力。
//! - **纯函数聚合**：[`BucketAccumulator`] / [`rollup_hour`] 不依赖时钟与仓储，单测覆盖
//!   桶边界、均值 / 峰值、空桶与乱序样本。
//! - **保留策略**：1 min 点保留 [`RETENTION_1M_SECS`]（7 d），1 h 点保留
//!   [`RETENTION_1H_SECS`]（90 d），每小时随 rollup 一起裁剪。
//! - **查询口径**：`range` 决定分辨率与窗口——`1h/6h/24h` 用 1 min 点，`7d/30d` 用 1 h 点；
//!   返回 `step_secs` 让前端不必猜桶宽。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use utoipa::ToSchema;

use crate::core::error::AppError;
use crate::domain::entity::{MetricsHistoryPoint, MetricsResolution, MetricsSnapshot};
use crate::domain::repository::MetricsHistoryRepository;

/// 1 min 点保留 7 天
pub const RETENTION_1M_SECS: i64 = 7 * 86_400;
/// 1 h 点保留 90 天
pub const RETENTION_1H_SECS: i64 = 90 * 86_400;
/// 单次查询返回的最大点数（30 d × 24 = 720 个 1 h 点；24 h × 60 = 1440 个 1 min 点）
pub const MAX_POINTS: usize = 2048;

/// 历史查询时间范围（`range` 查询参数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HistoryRange {
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "6h")]
    Hour6,
    #[serde(rename = "24h")]
    Hour24,
    #[serde(rename = "7d")]
    Day7,
    #[serde(rename = "30d")]
    Day30,
}

impl HistoryRange {
    pub const ALL: [&'static str; 5] = ["1h", "6h", "24h", "7d", "30d"];

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "1h" => Some(Self::Hour1),
            "6h" => Some(Self::Hour6),
            "24h" => Some(Self::Hour24),
            "7d" => Some(Self::Day7),
            "30d" => Some(Self::Day30),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hour1 => "1h",
            Self::Hour6 => "6h",
            Self::Hour24 => "24h",
            Self::Day7 => "7d",
            Self::Day30 => "30d",
        }
    }

    /// 窗口长度（秒）
    pub const fn window_secs(self) -> i64 {
        match self {
            Self::Hour1 => 3_600,
            Self::Hour6 => 6 * 3_600,
            Self::Hour24 => 24 * 3_600,
            Self::Day7 => 7 * 86_400,
            Self::Day30 => 30 * 86_400,
        }
    }

    /// 该范围使用的分辨率：≤ 24 h 用 1 min 点（最多 1440 点），更长用 1 h 点。
    pub const fn resolution(self) -> MetricsResolution {
        match self {
            Self::Hour1 | Self::Hour6 | Self::Hour24 => MetricsResolution::OneMinute,
            Self::Day7 | Self::Day30 => MetricsResolution::OneHour,
        }
    }
}

/// `GET /api/metrics/history` 响应。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsHistoryResponse {
    /// 请求的范围（`1h` / `6h` / `24h` / `7d` / `30d`）
    pub range: String,
    /// 每个点的桶宽（秒）：60 或 3600
    pub step_secs: i64,
    /// 窗口起点（Unix 秒，含）
    pub from: i64,
    /// 窗口终点（Unix 秒，不含）
    pub to: i64,
    /// 按时间升序的降采样点；刚启动或尚未写满时可能少于窗口理论点数
    pub points: Vec<MetricsHistoryPoint>,
}

// ─────────────────────────── 纯聚合 ───────────────────────────

/// 一个正在累积的桶（3 s 快照 → 1 min 点）。
#[derive(Debug, Clone, PartialEq)]
pub struct BucketAccumulator {
    pub bucket_start: i64,
    samples: u32,
    cpu_sum: f64,
    cpu_max: f32,
    mem_sum: f64,
    mem_max: f32,
    disk_sum: f64,
    load_sum: f64,
    rx_sum: f64,
    tx_sum: f64,
}

impl BucketAccumulator {
    pub fn new(bucket_start: i64) -> Self {
        Self {
            bucket_start,
            samples: 0,
            cpu_sum: 0.0,
            cpu_max: 0.0,
            mem_sum: 0.0,
            mem_max: 0.0,
            disk_sum: 0.0,
            load_sum: 0.0,
            rx_sum: 0.0,
            tx_sum: 0.0,
        }
    }

    pub fn push(&mut self, s: &MetricsSnapshot) {
        self.samples += 1;
        self.cpu_sum += f64::from(s.cpu_usage);
        self.cpu_max = self.cpu_max.max(s.cpu_usage);
        self.mem_sum += f64::from(s.memory_usage_percent);
        self.mem_max = self.mem_max.max(s.memory_usage_percent);
        self.disk_sum += f64::from(s.disk_usage_percent);
        self.load_sum += s.load_one;
        self.rx_sum += s.network_rx_mbps;
        self.tx_sum += s.network_tx_mbps;
    }

    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// 结算为一个点；空桶返回 `None`。
    pub fn finish(&self) -> Option<MetricsHistoryPoint> {
        if self.samples == 0 {
            return None;
        }
        let n = f64::from(self.samples);
        Some(MetricsHistoryPoint {
            bucket_start: self.bucket_start,
            samples: self.samples,
            cpu_avg: (self.cpu_sum / n) as f32,
            cpu_max: self.cpu_max,
            memory_avg: (self.mem_sum / n) as f32,
            memory_max: self.mem_max,
            disk_avg: (self.disk_sum / n) as f32,
            load_one_avg: self.load_sum / n,
            net_rx_avg_mbps: self.rx_sum / n,
            net_tx_avg_mbps: self.tx_sum / n,
        })
    }
}

/// 把一小时内的 1 min 点汇总成一个 1 h 点（按样本数加权的均值 / 全局峰值）。
/// `hour_start` 必须是整点；`points` 为空返回 `None`。
pub fn rollup_hour(hour_start: i64, points: &[MetricsHistoryPoint]) -> Option<MetricsHistoryPoint> {
    let mut total: u64 = 0;
    let (mut cpu, mut mem, mut disk, mut load, mut rx, mut tx) = (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut cpu_max, mut mem_max) = (0.0f32, 0.0f32);
    for p in points {
        let w = f64::from(p.samples.max(1));
        total += u64::from(p.samples.max(1));
        cpu += f64::from(p.cpu_avg) * w;
        mem += f64::from(p.memory_avg) * w;
        disk += f64::from(p.disk_avg) * w;
        load += p.load_one_avg * w;
        rx += p.net_rx_avg_mbps * w;
        tx += p.net_tx_avg_mbps * w;
        cpu_max = cpu_max.max(p.cpu_max);
        mem_max = mem_max.max(p.memory_max);
    }
    if total == 0 {
        return None;
    }
    let n = total as f64;
    Some(MetricsHistoryPoint {
        bucket_start: hour_start,
        samples: total.min(u64::from(u32::MAX)) as u32,
        cpu_avg: (cpu / n) as f32,
        cpu_max,
        memory_avg: (mem / n) as f32,
        memory_max: mem_max,
        disk_avg: (disk / n) as f32,
        load_one_avg: load / n,
        net_rx_avg_mbps: rx / n,
        net_tx_avg_mbps: tx / n,
    })
}

/// 快照时间戳统一为 Unix 秒（`MetricsSnapshot.timestamp` 为毫秒；兼容秒级旧值）。
pub fn snapshot_secs(s: &MetricsSnapshot) -> i64 {
    if s.timestamp > 100_000_000_000 {
        s.timestamp / 1000
    } else {
        s.timestamp
    }
}

// ─────────────────────────── 服务 ───────────────────────────

#[derive(Debug, Default)]
struct WriterState {
    current: Option<BucketAccumulator>,
    /// 上一次完成 rollup 的整点（含）
    last_rollup_hour: Option<i64>,
}

pub struct MetricsHistoryService {
    repo: Arc<dyn MetricsHistoryRepository>,
    state: Mutex<WriterState>,
}

impl MetricsHistoryService {
    pub fn new(repo: Arc<dyn MetricsHistoryRepository>) -> Self {
        Self {
            repo,
            state: Mutex::new(WriterState::default()),
        }
    }

    /// 喂入一条快照：同桶累积；跨桶时把上一桶落库；跨整点时触发 rollup + 裁剪。
    pub async fn ingest(&self, snapshot: &MetricsSnapshot) -> Result<(), AppError> {
        let ts = snapshot_secs(snapshot);
        let bucket = MetricsResolution::OneMinute.bucket_start(ts);
        let mut st = self.state.lock().await;
        let flushed = match st.current.as_mut() {
            Some(acc) if acc.bucket_start == bucket => {
                acc.push(snapshot);
                None
            }
            Some(acc) if acc.bucket_start > bucket => {
                // 乱序旧样本（时钟回拨）：丢弃，不污染当前桶
                None
            }
            _ => {
                let prev = st.current.take().and_then(|a| a.finish());
                let mut acc = BucketAccumulator::new(bucket);
                acc.push(snapshot);
                st.current = Some(acc);
                prev
            }
        };
        let hour = MetricsResolution::OneHour.bucket_start(ts);
        let need_rollup = match st.last_rollup_hour {
            None => {
                st.last_rollup_hour = Some(hour);
                false
            }
            Some(h) if hour > h => {
                st.last_rollup_hour = Some(hour);
                true
            }
            _ => false,
        };
        drop(st);

        if let Some(point) = flushed {
            self.repo
                .upsert(MetricsResolution::OneMinute, &point)
                .await?;
        }
        if need_rollup {
            self.rollup_and_prune(hour).await?;
        }
        Ok(())
    }

    /// 把当前未满的桶也落库（关停 / 测试用）。
    pub async fn flush(&self) -> Result<(), AppError> {
        let point = self
            .state
            .lock()
            .await
            .current
            .as_ref()
            .and_then(|a| a.finish());
        if let Some(p) = point {
            self.repo.upsert(MetricsResolution::OneMinute, &p).await?;
        }
        Ok(())
    }

    /// 汇总 `current_hour` 之前那个小时的 1 min 点为 1 h 点，并按保留策略裁剪。
    /// 幂等：同一小时重复执行只会覆盖同一行。
    pub async fn rollup_and_prune(&self, current_hour: i64) -> Result<(), AppError> {
        let hour_start = current_hour - 3600;
        let minutes = self
            .repo
            .query(MetricsResolution::OneMinute, hour_start, current_hour, 60)
            .await?;
        if let Some(p) = rollup_hour(hour_start, &minutes) {
            self.repo.upsert(MetricsResolution::OneHour, &p).await?;
        }
        self.repo
            .prune(
                MetricsResolution::OneMinute,
                current_hour - RETENTION_1M_SECS,
            )
            .await?;
        self.repo
            .prune(MetricsResolution::OneHour, current_hour - RETENTION_1H_SECS)
            .await?;
        Ok(())
    }

    /// 查询 `now` 之前 `range` 窗口内的点。
    pub async fn query(
        &self,
        range: HistoryRange,
        now_secs: i64,
    ) -> Result<MetricsHistoryResponse, AppError> {
        let resolution = range.resolution();
        let to = resolution.bucket_start(now_secs) + resolution.step_secs();
        let from = to - range.window_secs();
        let mut points = self.repo.query(resolution, from, to, MAX_POINTS).await?;
        // 当前未落库的分钟桶也带上，让 1h 视图右端不空一格
        if resolution == MetricsResolution::OneMinute {
            let live = self
                .state
                .lock()
                .await
                .current
                .as_ref()
                .and_then(|a| a.finish());
            if let Some(p) = live {
                if p.bucket_start >= from
                    && p.bucket_start < to
                    && points.last().map(|l| l.bucket_start) != Some(p.bucket_start)
                {
                    points.push(p);
                }
            }
        }
        Ok(MetricsHistoryResponse {
            range: range.as_str().to_string(),
            step_secs: resolution.step_secs(),
            from,
            to,
            points,
        })
    }

    /// 后台写入循环：订阅 Hub 广播，直到 `token` 取消；退出前 flush 当前桶。
    pub async fn run_writer(
        self: Arc<Self>,
        mut rx: tokio::sync::broadcast::Receiver<Arc<crate::infrastructure::metrics::MetricsFrame>>,
        token: tokio_util::sync::CancellationToken,
    ) {
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                item = rx.recv() => match item {
                    Ok(frame) => {
                        if let Err(e) = self.ingest(&frame.snapshot).await {
                            tracing::warn!(error = %e, "metrics history ingest failed");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // 写库落后于采集（极端 IO 卡顿）：丢掉的样本只影响该分钟的均值精度
                        tracing::warn!(dropped = n, "metrics history writer lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
            }
        }
        if let Err(e) = self.flush().await {
            tracing::warn!(error = %e, "metrics history final flush failed");
        }
    }
}

// ─────────────────────────── 单元测试 ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::InMemoryMetricsHistoryRepository;

    fn snap(ts_secs: i64, cpu: f32, mem: f32) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: ts_secs * 1000,
            cpu_usage: cpu,
            cpu_cores: 4,
            memory_usage_percent: mem,
            memory_total_mb: 8192,
            memory_used_mb: 4096,
            disk_usage_percent: 40.0,
            disk_total_gb: 100.0,
            disk_used_gb: 40.0,
            load_one: 1.0,
            load_five: 0.8,
            load_fifteen: 0.5,
            network_rx_mbps: 0.2,
            network_tx_mbps: 0.1,
        }
    }

    fn service() -> (
        Arc<MetricsHistoryService>,
        Arc<InMemoryMetricsHistoryRepository>,
    ) {
        let repo = Arc::new(InMemoryMetricsHistoryRepository::new());
        (Arc::new(MetricsHistoryService::new(repo.clone())), repo)
    }

    #[test]
    fn range_parse_and_resolution() {
        assert_eq!(HistoryRange::parse(" 24H "), Some(HistoryRange::Hour24));
        assert_eq!(HistoryRange::parse("7d"), Some(HistoryRange::Day7));
        assert_eq!(HistoryRange::parse("2d"), None);
        assert_eq!(
            HistoryRange::Hour6.resolution(),
            MetricsResolution::OneMinute
        );
        assert_eq!(HistoryRange::Day30.resolution(), MetricsResolution::OneHour);
        assert_eq!(HistoryRange::Day7.window_secs(), 7 * 86_400);
        for r in HistoryRange::ALL {
            assert_eq!(HistoryRange::parse(r).unwrap().as_str(), r);
        }
    }

    #[test]
    fn bucket_start_and_snapshot_secs() {
        assert_eq!(
            MetricsResolution::OneMinute.bucket_start(1_700_000_059),
            1_700_000_040
        );
        assert_eq!(
            MetricsResolution::OneHour.bucket_start(1_700_003_599),
            1_700_002_800
        );
        assert_eq!(snapshot_secs(&snap(1_700_000_000, 0.0, 0.0)), 1_700_000_000);
        let mut legacy = snap(1, 0.0, 0.0);
        legacy.timestamp = 1_700_000_000; // 秒级旧值
        assert_eq!(snapshot_secs(&legacy), 1_700_000_000);
    }

    #[test]
    fn accumulator_avg_and_max() {
        let mut acc = BucketAccumulator::new(60);
        assert!(acc.finish().is_none());
        acc.push(&snap(61, 10.0, 50.0));
        acc.push(&snap(64, 30.0, 70.0));
        acc.push(&snap(67, 20.0, 60.0));
        let p = acc.finish().unwrap();
        assert_eq!(p.samples, 3);
        assert_eq!(p.bucket_start, 60);
        assert!((p.cpu_avg - 20.0).abs() < 1e-4);
        assert_eq!(p.cpu_max, 30.0);
        assert!((p.memory_avg - 60.0).abs() < 1e-4);
        assert_eq!(p.memory_max, 70.0);
        assert!((p.load_one_avg - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rollup_hour_is_sample_weighted() {
        let a = MetricsHistoryPoint {
            bucket_start: 0,
            samples: 20,
            cpu_avg: 10.0,
            cpu_max: 15.0,
            memory_avg: 50.0,
            memory_max: 55.0,
            disk_avg: 40.0,
            load_one_avg: 1.0,
            net_rx_avg_mbps: 1.0,
            net_tx_avg_mbps: 0.0,
        };
        let b = MetricsHistoryPoint {
            bucket_start: 60,
            samples: 60,
            cpu_avg: 50.0,
            cpu_max: 90.0,
            ..a.clone()
        };
        let h = rollup_hour(0, &[a, b]).unwrap();
        assert_eq!(h.samples, 80);
        assert!(
            (h.cpu_avg - 40.0).abs() < 1e-4,
            "weighted: (10*20+50*60)/80 = 40"
        );
        assert_eq!(h.cpu_max, 90.0);
        assert!(rollup_hour(0, &[]).is_none());
    }

    #[tokio::test]
    async fn ingest_flushes_previous_minute_and_query_includes_live_bucket() {
        let (svc, repo) = service();
        let t0 = 1_700_000_040; // 分钟桶起点
        svc.ingest(&snap(t0 + 1, 10.0, 50.0)).await.unwrap();
        svc.ingest(&snap(t0 + 4, 30.0, 50.0)).await.unwrap();
        // 尚未跨分钟：仓储为空，但 query 带上活动桶
        assert!(repo
            .query(MetricsResolution::OneMinute, 0, i64::MAX, 10)
            .await
            .unwrap()
            .is_empty());
        let res = svc.query(HistoryRange::Hour1, t0 + 5).await.unwrap();
        assert_eq!(res.step_secs, 60);
        assert_eq!(res.points.len(), 1);
        assert_eq!(res.points[0].samples, 2);
        assert_eq!(res.to - res.from, 3600);

        // 跨到下一分钟：上一桶落库
        svc.ingest(&snap(t0 + 61, 50.0, 50.0)).await.unwrap();
        let stored = repo
            .query(MetricsResolution::OneMinute, 0, i64::MAX, 10)
            .await
            .unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].bucket_start, t0);
        assert!((stored[0].cpu_avg - 20.0).abs() < 1e-4);

        // 乱序旧样本被忽略
        svc.ingest(&snap(t0 + 2, 99.0, 99.0)).await.unwrap();
        let res = svc.query(HistoryRange::Hour1, t0 + 62).await.unwrap();
        assert_eq!(res.points.len(), 2);
        assert_eq!(res.points[1].samples, 1);
        assert_eq!(res.points[1].cpu_avg, 50.0);

        // flush 把活动桶也写入
        svc.flush().await.unwrap();
        assert_eq!(
            repo.query(MetricsResolution::OneMinute, 0, i64::MAX, 10)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn crossing_the_hour_rolls_up_and_prunes() {
        let (svc, repo) = service();
        let hour = 1_700_002_800; // 整点（1_700_002_800 % 3600 == 0）
                                  // 上一小时的两个分钟点直接落库
        for (i, cpu) in [(0, 10.0f32), (1, 30.0)] {
            let p = BucketAccumulator::new(hour - 3600 + 60 * i);
            let mut p = p;
            p.push(&snap(hour - 3600 + 60 * i + 1, cpu, 50.0));
            repo.upsert(MetricsResolution::OneMinute, &p.finish().unwrap())
                .await
                .unwrap();
        }
        // 超期的 1 min 点（8 天前）与 1 h 点（91 天前）应被裁剪
        let mut old = BucketAccumulator::new(hour - 8 * 86_400);
        old.push(&snap(hour - 8 * 86_400 + 1, 1.0, 1.0));
        repo.upsert(MetricsResolution::OneMinute, &old.finish().unwrap())
            .await
            .unwrap();
        let mut old_h = BucketAccumulator::new(hour - 91 * 86_400);
        old_h.push(&snap(hour - 91 * 86_400 + 1, 1.0, 1.0));
        repo.upsert(MetricsResolution::OneHour, &old_h.finish().unwrap())
            .await
            .unwrap();

        // 第一条样本落在上一小时（初始化 last_rollup_hour），第二条跨入新整点触发 rollup
        svc.ingest(&snap(hour - 10, 5.0, 5.0)).await.unwrap();
        svc.ingest(&snap(hour + 1, 5.0, 5.0)).await.unwrap();

        let hourly = repo
            .query(MetricsResolution::OneHour, hour - 3600, hour, 10)
            .await
            .unwrap();
        assert_eq!(hourly.len(), 1, "one 1h point for the completed hour");
        assert_eq!(hourly[0].bucket_start, hour - 3600);
        // 三个分钟点各 1 个样本：(10 + 30 + 5) / 3
        assert!((hourly[0].cpu_avg - 15.0).abs() < 1e-4);
        assert_eq!(hourly[0].cpu_max, 30.0);

        assert!(repo
            .query(MetricsResolution::OneMinute, 0, hour - 7 * 86_400, 10)
            .await
            .unwrap()
            .is_empty());
        assert!(repo
            .query(MetricsResolution::OneHour, 0, hour - 90 * 86_400, 10)
            .await
            .unwrap()
            .is_empty());

        let week = svc.query(HistoryRange::Day7, hour + 1).await.unwrap();
        assert_eq!(week.step_secs, 3600);
        assert_eq!(week.points.len(), 1);
    }
}
