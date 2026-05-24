use std::collections::VecDeque;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{broadcast, Mutex};
use tokio::time::{interval, Duration};

fn clamp_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

fn clamp_f64(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

#[derive(Debug, Clone, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_usage_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_used_mb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_total_mb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_temperature: Option<f32>,
}

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

fn sample_gpu() -> (Option<f32>, Option<u64>, Option<u64>, Option<f32>) {
    match nvml_wrapper::Nvml::init() {
        Ok(nvml) => {
            if let Ok(count) = nvml.device_count() {
                if count > 0 {
                    if let Ok(device) = nvml.device_by_index(0) {
                        let util = device.utilization_rates().map(|u| clamp_f32(u.gpu as f32)).ok();
                        let mem = device.memory_info().map(|m| (m.used / 1024 / 1024, m.total / 1024 / 1024)).ok();
                        let temp = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                            .map(|t| clamp_f32(t as f32)).ok();
                        return (
                            util,
                            mem.as_ref().map(|m| m.0),
                            mem.as_ref().map(|m| m.1),
                            temp,
                        );
                    }
                }
            }
            (None, None, None, None)
        }
        Err(_) => (None, None, None, None),
    }
}

pub fn spawn_metrics_collector(
    history: Arc<Mutex<MetricsHistory>>,
    tx: broadcast::Sender<MetricsSnapshot>,
) {
    tokio::spawn(async move {
        use sysinfo::System;

        let mut sys = System::new_all();
        // 跳过首次 tick，让 sysinfo 累积真实的 CPU 增量
        let mut tick = interval(Duration::from_secs(3));
        let mut gpu_tick = interval(Duration::from_secs(5));
        let mut gpu_cache: (Option<f32>, Option<u64>, Option<u64>, Option<f32>) =
            (None, None, None, None);
        tick.tick().await;

        loop {
            tokio::select! {
                _ = tick.tick() => {
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
                        timestamp: chrono::Utc::now().timestamp_millis(),
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
                        gpu_usage_percent: gpu_cache.0,
                        gpu_memory_used_mb: gpu_cache.1,
                        gpu_memory_total_mb: gpu_cache.2,
                        gpu_temperature: gpu_cache.3,
                    };

                    history.lock().await.push(snapshot.clone());
                    let _ = tx.send(snapshot);
                }
                _ = gpu_tick.tick() => {
                    // GPU sampling is slower; update at 5s interval to avoid blocking NVML
                    gpu_cache = tokio::task::spawn_blocking(sample_gpu)
                        .await
                        .unwrap_or((None, None, None, None));
                }
            }
        }
    });
}
