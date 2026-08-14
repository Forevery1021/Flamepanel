use std::collections::VecDeque;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use crate::domain::entity::MetricsSnapshot;

pub struct MetricsHistory {
    buffer: VecDeque<MetricsSnapshot>,
    max_size: usize,
}

impl MetricsHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
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
}

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

/// 指标采集循环（由调用方负责 spawn，通常经 TaskSupervisor 管理生命周期）。
/// 通过 `token.cancelled()` 协作式退出。
pub async fn metrics_collector_loop(
    history: Arc<Mutex<MetricsHistory>>,
    tx: broadcast::Sender<MetricsSnapshot>,
    token: tokio_util::sync::CancellationToken,
) {
    use sysinfo::System;

    let mut sys = System::new_all();
    let mut tick = interval(Duration::from_secs(3));
    tick.tick().await;

    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = tick.tick() => {}
        }
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let cpu_cores = sys.cpus().len();
        let memory_total = sys.total_memory() / 1024 / 1024;
        let memory_used = sys.used_memory() / 1024 / 1024;
        let memory_usage_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };

        let disks = sysinfo::Disks::new_with_refreshed_list();
        let (disk_total, disk_used): (u64, u64) =
            disks.iter().fold((0, 0), |(total, used), disk| {
                (
                    total + disk.total_space(),
                    used + disk.total_space() - disk.available_space(),
                )
            });
        let disk_total_gb = disk_total as f64 / 1024.0 / 1024.0 / 1024.0;
        let disk_used_gb = disk_used as f64 / 1024.0 / 1024.0 / 1024.0;
        let disk_usage_percent = if disk_total_gb > 0.0 {
            ((disk_used_gb / disk_total_gb) * 100.0) as f32
        } else {
            0.0
        };

        // 网络 IO：Networks::refresh(true) 返回自上次刷新的差值，换算 MB/s
        let mut networks = sysinfo::Networks::new();
        networks.refresh(true);
        let (network_rx_mbps, network_tx_mbps): (f64, f64) =
            networks.iter().fold((0.0, 0.0), |(rx, tx), (_, data)| {
                (
                    rx + data.received() as f64 / 1024.0 / 1024.0,
                    tx + data.transmitted() as f64 / 1024.0 / 1024.0,
                )
            });

        let load = System::load_average();

        let snapshot = MetricsSnapshot {
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
            network_rx_mbps: clamp_f64(network_rx_mbps),
            network_tx_mbps: clamp_f64(network_tx_mbps),
        };

        history.lock().await.push(snapshot.clone());
        let _ = tx.send(snapshot);
    }
}
