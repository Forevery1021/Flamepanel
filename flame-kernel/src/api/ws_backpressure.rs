//! WS 推送流背压治理（S-08 · Doc/30 M-P）。
//!
//! `/ws/metrics`、`/ws/logs` 都是「`broadcast` 扇出 → 每连接一个发送任务」的单向推送流。
//! 慢消费者（网络差 / 页签被浏览器节流 / 客户端卡死）跟不上发布速率时，`broadcast`
//! 会丢弃其未读的旧帧并返回 `RecvError::Lagged(n)`。此前的处理是"告警并继续"：
//!
//! - 客户端**不知道**自己丢了帧——仪表盘曲线出现无法解释的缺口；
//! - 一个长期滞后的连接会反复触发 warn 日志，却永远不会被清理；
//! - 运维侧没有任何指标能看出"有多少订阅者在掉帧"。
//!
//! 本模块提供：
//! 1. [`LagPolicy`]：纯状态机——**连续** `Lagged` 达到阈值（默认 3）判定为慢消费者，
//!    要求会话发送一帧 [`RESYNC_FRAME`] 后主动断开；期间任何一次成功发送都会清零计数
//!    （偶发抖动不惩罚）。客户端收到 `resync` / 或连接关闭后重连，`init` 帧会带上完整
//!    历史窗口，曲线自动补齐。
//! 2. [`WsStats`]：进程级原子计数（按流），经 `/metrics` 以 Prometheus 计数器暴露：
//!    `flamepanel_ws_lag_events_total{stream}`、`flamepanel_ws_lag_dropped_frames_total{stream}`、
//!    `flamepanel_ws_slow_consumer_disconnects_total{stream}`。

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

/// 连续滞后多少次判定为慢消费者（对应 metrics 流约 3 个采集周期）。
pub const DEFAULT_LAG_THRESHOLD: u32 = 3;

/// 慢消费者断开前发给客户端的最后一帧：告知"你落后了，请重连并重新同步"。
/// 与既有 `{type:"init"|"tick"}` 线格式同构；`reason` 固定为 `lagged`。
pub const RESYNC_FRAME: &str = r#"{"type":"resync","reason":"lagged"}"#;

/// 会话对一次 `Lagged` 的处置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LagAction {
    /// 偶发滞后：记录并继续推送。
    Continue,
    /// 连续滞后达阈值：发送 `resync` 帧并断开。
    Resync,
}

/// 单连接的滞后判定状态机。
#[derive(Debug, Clone)]
pub struct LagPolicy {
    threshold: u32,
    consecutive: u32,
}

impl Default for LagPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_LAG_THRESHOLD)
    }
}

impl LagPolicy {
    /// `threshold == 0` 视为 1（第一次滞后即断开）。
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold: threshold.max(1),
            consecutive: 0,
        }
    }

    /// 收到 `Lagged` 时调用。
    pub fn on_lag(&mut self) -> LagAction {
        self.consecutive = self.consecutive.saturating_add(1);
        if self.consecutive >= self.threshold {
            LagAction::Resync
        } else {
            LagAction::Continue
        }
    }

    /// 成功向客户端发出一帧后调用：连续计数清零。
    pub fn on_delivered(&mut self) {
        self.consecutive = 0;
    }

    pub fn consecutive(&self) -> u32 {
        self.consecutive
    }
}

/// 单条推送流的计数器。
#[derive(Debug, Default)]
pub struct StreamStats {
    /// 收到 `Lagged` 的次数
    pub lag_events: AtomicU64,
    /// `Lagged(n)` 中 n 的累计（被 broadcast 丢弃、客户端未收到的帧数）
    pub lag_dropped_frames: AtomicU64,
    /// 因连续滞后被服务端主动断开的连接数
    pub slow_consumer_disconnects: AtomicU64,
}

impl StreamStats {
    pub fn record_lag(&self, dropped: u64) {
        self.lag_events.fetch_add(1, Ordering::Relaxed);
        self.lag_dropped_frames
            .fetch_add(dropped, Ordering::Relaxed);
    }

    pub fn record_disconnect(&self) {
        self.slow_consumer_disconnects
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.lag_events.load(Ordering::Relaxed),
            self.lag_dropped_frames.load(Ordering::Relaxed),
            self.slow_consumer_disconnects.load(Ordering::Relaxed),
        )
    }
}

/// 全部推送流的计数器（进程级；Prometheus 计数器天然是进程级单调量）。
#[derive(Debug, Default)]
pub struct WsStats {
    pub metrics: StreamStats,
    pub logs: StreamStats,
}

impl WsStats {
    /// 按流名取计数器；未知流名归入 `logs` 之外的兜底——目前只有两条流，直接映射。
    pub fn stream(&self, name: &str) -> &StreamStats {
        match name {
            "metrics" => &self.metrics,
            _ => &self.logs,
        }
    }

    /// 以 Prometheus 文本格式追加到 `out`（每个指标一组 HELP/TYPE + 两条带 `stream` 标签的样本）。
    pub fn write_prometheus(&self, out: &mut String) {
        type Reader = fn(&StreamStats) -> u64;
        let rows = [("metrics", &self.metrics), ("logs", &self.logs)];
        let series: [(&str, &str, Reader); 3] = [
            (
                "flamepanel_ws_lag_events_total",
                "Times a push-stream subscriber lagged behind the broadcast channel.",
                |s| s.lag_events.load(Ordering::Relaxed),
            ),
            (
                "flamepanel_ws_lag_dropped_frames_total",
                "Frames dropped by the broadcast channel for lagging subscribers.",
                |s| s.lag_dropped_frames.load(Ordering::Relaxed),
            ),
            (
                "flamepanel_ws_slow_consumer_disconnects_total",
                "Push-stream connections closed by the server after consecutive lag (resync sent).",
                |s| s.slow_consumer_disconnects.load(Ordering::Relaxed),
            ),
        ];
        for (name, help, read) in series {
            let _ = writeln!(out, "# HELP {name} {help}\n# TYPE {name} counter");
            for (stream, stats) in rows {
                let _ = writeln!(out, "{name}{{stream=\"{stream}\"}} {}", read(stats));
            }
        }
    }
}

/// 进程级计数器实例。
pub fn ws_stats() -> &'static WsStats {
    static STATS: WsStats = WsStats {
        metrics: StreamStats {
            lag_events: AtomicU64::new(0),
            lag_dropped_frames: AtomicU64::new(0),
            slow_consumer_disconnects: AtomicU64::new(0),
        },
        logs: StreamStats {
            lag_events: AtomicU64::new(0),
            lag_dropped_frames: AtomicU64::new(0),
            slow_consumer_disconnects: AtomicU64::new(0),
        },
    };
    &STATS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_requires_consecutive_lags() {
        let mut p = LagPolicy::default();
        assert_eq!(p.on_lag(), LagAction::Continue);
        assert_eq!(p.on_lag(), LagAction::Continue);
        // 一次成功投递即清零：偶发抖动不惩罚
        p.on_delivered();
        assert_eq!(p.consecutive(), 0);
        assert_eq!(p.on_lag(), LagAction::Continue);
        assert_eq!(p.on_lag(), LagAction::Continue);
        assert_eq!(p.on_lag(), LagAction::Resync);
    }

    #[test]
    fn policy_threshold_zero_means_first_lag_disconnects() {
        let mut p = LagPolicy::new(0);
        assert_eq!(p.on_lag(), LagAction::Resync);
        let mut p = LagPolicy::new(1);
        assert_eq!(p.on_lag(), LagAction::Resync);
    }

    #[test]
    fn policy_keeps_returning_resync_once_reached() {
        let mut p = LagPolicy::new(2);
        p.on_lag();
        assert_eq!(p.on_lag(), LagAction::Resync);
        assert_eq!(p.on_lag(), LagAction::Resync);
    }

    #[test]
    fn stats_accumulate_per_stream() {
        let stats = WsStats::default();
        stats.stream("metrics").record_lag(5);
        stats.stream("metrics").record_lag(7);
        stats.stream("metrics").record_disconnect();
        stats.stream("logs").record_lag(1);
        assert_eq!(stats.metrics.snapshot(), (2, 12, 1));
        assert_eq!(stats.logs.snapshot(), (1, 1, 0));
    }

    #[test]
    fn prometheus_output_has_labelled_counters() {
        let stats = WsStats::default();
        stats.metrics.record_lag(3);
        stats.metrics.record_disconnect();
        let mut out = String::new();
        stats.write_prometheus(&mut out);
        assert!(out.contains("# TYPE flamepanel_ws_lag_events_total counter\n"));
        assert!(out.contains("flamepanel_ws_lag_events_total{stream=\"metrics\"} 1\n"));
        assert!(out.contains("flamepanel_ws_lag_events_total{stream=\"logs\"} 0\n"));
        assert!(out.contains("flamepanel_ws_lag_dropped_frames_total{stream=\"metrics\"} 3\n"));
        assert!(
            out.contains("flamepanel_ws_slow_consumer_disconnects_total{stream=\"metrics\"} 1\n")
        );
        // 每个指标恰好一组 HELP/TYPE
        assert_eq!(out.matches("# HELP ").count(), 3);
        assert_eq!(out.matches("# TYPE ").count(), 3);
    }

    #[test]
    fn resync_frame_is_valid_json_with_type_resync() {
        let v: serde_json::Value = serde_json::from_str(RESYNC_FRAME).unwrap();
        assert_eq!(v["type"], "resync");
        assert_eq!(v["reason"], "lagged");
    }

    #[test]
    fn global_stats_is_a_singleton() {
        assert!(std::ptr::eq(ws_stats(), ws_stats()));
    }
}
