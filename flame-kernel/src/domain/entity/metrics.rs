//! 监控指标（历史点、快照、分辨率）。

use super::*;

/// 监控历史分辨率（S-07）：`OneMinute` 由 3 s 采样在内存聚合后落库，
/// `OneHour` 由已完成小时的 1 min 点二次汇总得到。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MetricsResolution {
    OneMinute,
    OneHour,
}

impl MetricsResolution {
    /// 桶宽（秒）
    pub const fn step_secs(self) -> i64 {
        match self {
            MetricsResolution::OneMinute => 60,
            MetricsResolution::OneHour => 3600,
        }
    }

    /// 持久化用的稳定标识（表内判别列）
    pub const fn as_str(self) -> &'static str {
        match self {
            MetricsResolution::OneMinute => "1m",
            MetricsResolution::OneHour => "1h",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "1m" => Some(MetricsResolution::OneMinute),
            "1h" => Some(MetricsResolution::OneHour),
            _ => None,
        }
    }

    /// 时间戳所在桶的起点（Unix 秒）
    pub const fn bucket_start(self, ts_secs: i64) -> i64 {
        let step = self.step_secs();
        ts_secs - ts_secs.rem_euclid(step)
    }
}

/// 监控历史降采样点（S-07）。一个桶内 `samples` 条 3 s 快照的均值 / 峰值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MetricsHistoryPoint {
    /// 桶起点（Unix 秒）
    pub bucket_start: i64,
    /// 桶内样本数
    pub samples: u32,
    pub cpu_avg: f32,
    pub cpu_max: f32,
    pub memory_avg: f32,
    pub memory_max: f32,
    pub disk_avg: f32,
    pub load_one_avg: f64,
    /// 网络接收速率均值（MB/s）
    pub net_rx_avg_mbps: f64,
    /// 网络发送速率均值（MB/s）
    pub net_tx_avg_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetricsSnapshot {
    pub timestamp: i64,
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub memory_usage_percent: f32,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub disk_usage_percent: f32,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub load_one: f64,
    pub load_five: f64,
    pub load_fifteen: f64,
    /// 网络接收速率（MB/s）
    pub network_rx_mbps: f64,
    /// 网络发送速率（MB/s）
    pub network_tx_mbps: f64,
}
