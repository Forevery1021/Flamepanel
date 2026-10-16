//! 节点指标并入 `MetricsHub` 统一模型（Doc/30 M-P / M-D 前置「节点指标经 MetricsHub」）。
//!
//! 此前 Agent 心跳只把 4 个百分比写进 `server_nodes.metrics_json`，面板只能看到「最近一次」
//! 数字，没有曲线、没有实时推送，前端也无法复用仪表盘。现在：
//!
//! ```text
//!  Agent 心跳（10 s）──► heartbeat handler ──► snapshot_from_heartbeat()（钳制 / 服务端时间戳）
//!                                               │
//!                                               ▼
//!                       NodeMetricsRegistry::publish(node_id, snapshot)
//!                               └─ 每节点一个 MetricsHub（历史环 + init 缓存 + broadcast）
//!                                               │
//!        /ws/metrics?node_id=N  ◄───────────────┘  与本机同一套 init / tick 帧协议
//!        GET /api/nodes/{id}/metrics/history
//! ```
//!
//! - 每节点保留 [`NODE_HISTORY_POINTS`] 个点（10 s × 120 = 20 分钟），只在内存中，
//!   面板重启后随下一次心跳重新积累（持久化历史仍只覆盖本机，见 Doc/30）。
//! - Agent 上报的数值不可信：百分比钳到 0–100、非有限值归零、负数归零；时间戳一律用
//!   面板接收时刻，避免节点时钟偏差让曲线错位。
//! - 节点删除时同步移除其 Hub，订阅者随广播关闭而结束。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::domain::entity::MetricsSnapshot;
use crate::infrastructure::metrics::MetricsHub;

/// 每节点内存历史点数（Agent 10 s 一次心跳 → 20 分钟）。
pub const NODE_HISTORY_POINTS: usize = 120;

/// Agent 心跳中的指标字段（旧 Agent 只带前 4 项，其余缺省为 0）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeartbeatMetrics {
    pub cpu_usage: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub load_one: f64,
    pub load_five: f64,
    pub load_fifteen: f64,
    pub cpu_cores: u64,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub network_rx_mbps: f64,
    pub network_tx_mbps: f64,
}

fn finite_non_negative(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

fn percent(v: f64) -> f32 {
    finite_non_negative(v).min(100.0) as f32
}

/// 心跳 → 统一快照（纯函数）：钳制不可信输入，时间戳取面板接收时刻（毫秒）。
pub fn snapshot_from_heartbeat(m: &HeartbeatMetrics, received_at_ms: i64) -> MetricsSnapshot {
    let memory_total_mb = m.memory_total_mb;
    let disk_total_gb = finite_non_negative(m.disk_total_gb);
    MetricsSnapshot {
        timestamp: received_at_ms,
        cpu_usage: percent(m.cpu_usage),
        // 合理上限：防止异常值撑爆前端格式化（4096 核已远超现实机器）
        cpu_cores: m.cpu_cores.min(4096) as usize,
        memory_usage_percent: percent(m.memory_usage_percent),
        memory_total_mb,
        memory_used_mb: if memory_total_mb > 0 {
            m.memory_used_mb.min(memory_total_mb)
        } else {
            m.memory_used_mb
        },
        disk_usage_percent: percent(m.disk_usage_percent),
        disk_total_gb,
        disk_used_gb: finite_non_negative(m.disk_used_gb),
        load_one: finite_non_negative(m.load_one),
        load_five: finite_non_negative(m.load_five),
        load_fifteen: finite_non_negative(m.load_fifteen),
        network_rx_mbps: finite_non_negative(m.network_rx_mbps),
        network_tx_mbps: finite_non_negative(m.network_tx_mbps),
    }
}

/// 节点 → 指标中枢注册表。挂在本机 `MetricsHub` 上（`state.metrics.nodes()`），
/// 节点 Hub 惰性创建：首个心跳到达时建立。
#[derive(Default)]
pub struct NodeMetricsRegistry {
    hubs: RwLock<HashMap<i64, Arc<MetricsHub>>>,
}

impl NodeMetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 已有的节点 Hub（未收到过心跳返回 `None`）。
    pub fn hub(&self, node_id: i64) -> Option<Arc<MetricsHub>> {
        self.hubs
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .get(&node_id)
            .cloned()
    }

    /// 取节点 Hub，不存在则创建（WS 订阅可先于首个心跳建立）。
    pub fn hub_or_create(&self, node_id: i64) -> Arc<MetricsHub> {
        if let Some(h) = self.hub(node_id) {
            return h;
        }
        let mut map = self.hubs.write().unwrap_or_else(|p| p.into_inner());
        map.entry(node_id)
            .or_insert_with(|| Arc::new(MetricsHub::new(NODE_HISTORY_POINTS)))
            .clone()
    }

    /// 写入一条节点快照；返回该节点当前订阅者数量。
    pub fn publish(&self, node_id: i64, snapshot: MetricsSnapshot) -> usize {
        self.hub_or_create(node_id).publish(snapshot)
    }

    /// 节点删除：移除 Hub（广播发送端随之释放，订阅会话收到关闭后结束）。
    pub fn remove(&self, node_id: i64) -> bool {
        self.hubs
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&node_id)
            .is_some()
    }

    /// 当前持有 Hub 的节点数（可观测性 / 测试）。
    pub fn len(&self) -> usize {
        self.hubs.read().unwrap_or_else(|p| p.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> HeartbeatMetrics {
        HeartbeatMetrics {
            cpu_usage: 37.5,
            memory_usage_percent: 50.0,
            disk_usage_percent: 20.0,
            load_one: 0.5,
            load_five: 0.4,
            load_fifteen: 0.3,
            cpu_cores: 8,
            memory_total_mb: 16384,
            memory_used_mb: 8192,
            disk_total_gb: 100.0,
            disk_used_gb: 20.0,
            network_rx_mbps: 1.5,
            network_tx_mbps: 0.25,
        }
    }

    #[test]
    fn snapshot_maps_fields_and_uses_server_timestamp() {
        let s = snapshot_from_heartbeat(&sample(), 1_700_000_000_000);
        assert_eq!(s.timestamp, 1_700_000_000_000);
        assert_eq!(s.cpu_usage, 37.5);
        assert_eq!(s.cpu_cores, 8);
        assert_eq!(s.memory_total_mb, 16384);
        assert_eq!(s.memory_used_mb, 8192);
        assert_eq!(s.disk_used_gb, 20.0);
        assert_eq!(s.load_fifteen, 0.3);
        assert_eq!(s.network_tx_mbps, 0.25);
    }

    #[test]
    fn snapshot_clamps_untrusted_values() {
        let m = HeartbeatMetrics {
            cpu_usage: 250.0,
            memory_usage_percent: f64::NAN,
            disk_usage_percent: -5.0,
            load_one: f64::INFINITY,
            cpu_cores: 1_000_000,
            network_rx_mbps: -1.0,
            memory_used_mb: 99_999,
            ..sample()
        };
        let s = snapshot_from_heartbeat(&m, 1);
        assert_eq!(s.cpu_usage, 100.0);
        assert_eq!(s.memory_usage_percent, 0.0);
        assert_eq!(s.disk_usage_percent, 0.0);
        assert_eq!(s.load_one, 0.0);
        assert_eq!(s.cpu_cores, 4096);
        assert_eq!(s.network_rx_mbps, 0.0);
        assert_eq!(s.memory_used_mb, 16384, "已用内存不应超过总量");
    }

    #[test]
    fn legacy_heartbeat_fills_missing_fields_with_zero() {
        let m = HeartbeatMetrics {
            cpu_usage: 10.0,
            memory_usage_percent: 20.0,
            disk_usage_percent: 30.0,
            load_one: 1.0,
            ..Default::default()
        };
        let s = snapshot_from_heartbeat(&m, 5);
        assert_eq!(s.memory_total_mb, 0);
        assert_eq!(s.cpu_cores, 0);
        assert_eq!(s.disk_usage_percent, 30.0);
    }

    #[test]
    fn registry_isolates_nodes_and_supports_removal() {
        let reg = NodeMetricsRegistry::new();
        assert!(reg.hub(1).is_none());
        let mut rx = reg.hub_or_create(1).subscribe();
        assert_eq!(reg.publish(1, snapshot_from_heartbeat(&sample(), 10)), 1);
        reg.publish(2, snapshot_from_heartbeat(&sample(), 20));
        assert_eq!(reg.len(), 2);
        let frame = rx.try_recv().expect("node 1 tick");
        assert_eq!(frame.snapshot.timestamp, 10);
        assert!(rx.try_recv().is_err(), "node 2 的帧不应进入 node 1 的订阅");
        assert_eq!(reg.hub(1).map(|h| h.history_len()), Some(1));
        assert!(reg.hub(1).and_then(|h| h.init_payload()).is_some());
        assert!(reg.remove(1));
        assert!(!reg.remove(1));
        assert!(reg.hub(1).is_none());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn history_is_capped_per_node() {
        let reg = NodeMetricsRegistry::new();
        for i in 0..(NODE_HISTORY_POINTS as i64 + 10) {
            reg.publish(7, snapshot_from_heartbeat(&sample(), i));
        }
        let hist = reg.hub(7).map(|h| h.history()).unwrap_or_default();
        assert_eq!(hist.len(), NODE_HISTORY_POINTS);
        assert_eq!(hist.first().map(|s| s.timestamp), Some(10));
    }
}
