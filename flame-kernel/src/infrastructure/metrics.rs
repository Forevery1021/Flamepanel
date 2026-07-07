use std::collections::VecDeque;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tokio::sync::broadcast;

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
    if v.is_finite() { v } else { 0.0 }
}

fn clamp_f64(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

pub fn spawn_metrics_collector(
    history: Arc<Mutex<MetricsHistory>>,
    tx: broadcast::Sender<MetricsSnapshot>,
) {
    tokio::spawn(async move {
        use sysinfo::System;

        let mut sys = System::new_all();
        let mut tick = interval(Duration::from_secs(3));
        tick.tick().await;

        loop {
            tick.tick().await;
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
            let (disk_total, disk_used): (u64, u64) = disks.iter().fold(
                (0, 0),
                |(total, used), disk| {
                    (
                        total + disk.total_space(),
                        used + disk.total_space() - disk.available_space(),
                    )
                },
            );
            let disk_total_gb = disk_total as f64 / 1024.0 / 1024.0 / 1024.0;
            let disk_used_gb = disk_used as f64 / 1024.0 / 1024.0 / 1024.0;
            let disk_usage_percent = if disk_total_gb > 0.0 {
                ((disk_used_gb / disk_total_gb) * 100.0) as f32
            } else {
                0.0
            };

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
            };

            history.lock().await.push(snapshot.clone());
            let _ = tx.send(snapshot);
        }
    });
}