//! Panel registration and periodic metrics reporting.

use super::*;

/// Matches the panel `HeartbeatRequest` (Doc/23). The first four fields are
/// legacy-compatible; older panels ignore the additional snapshot fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HeartbeatPayload {
    pub(crate) cpu_usage: f32,
    pub(crate) memory_usage_percent: f32,
    pub(crate) disk_usage_percent: f32,
    pub(crate) load_one: f32,
    pub(crate) load_five: f32,
    pub(crate) load_fifteen: f32,
    pub(crate) cpu_cores: u64,
    pub(crate) memory_total_mb: u64,
    pub(crate) memory_used_mb: u64,
    pub(crate) disk_total_gb: f64,
    pub(crate) disk_used_gb: f64,
    pub(crate) network_rx_mbps: f64,
    pub(crate) network_tx_mbps: f64,
}

const MIB: f64 = 1024.0 * 1024.0;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// CPU discovery may be empty at startup; available parallelism is only a fallback.
fn cpu_core_count(reported: usize, available: Option<usize>) -> u64 {
    if reported > 0 {
        reported as u64
    } else {
        available.unwrap_or(1).max(1) as u64
    }
}

/// Convert byte counters to MiB/s; return zero on resets or invalid intervals.
pub(crate) fn rate_mbps(prev_bytes: u64, now_bytes: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= 0.0 || !elapsed_secs.is_finite() || now_bytes < prev_bytes {
        return 0.0;
    }
    (now_bytes - prev_bytes) as f64 / elapsed_secs / MIB
}

/// Retains the CPU window, network counters, and previous sample time.
pub(crate) struct MetricsCollector {
    sys: System,
    networks: sysinfo::Networks,
    last_net: Option<(std::time::Instant, u64, u64)>,
}

impl MetricsCollector {
    pub(crate) fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self {
            sys,
            networks: sysinfo::Networks::new_with_refreshed_list(),
            last_net: None,
        }
    }

    /// Cumulative received/sent bytes, excluding loopback.
    fn network_totals(&mut self) -> (u64, u64) {
        self.networks.refresh(true);
        self.networks
            .iter()
            .filter(|(name, _)| name.as_str() != "lo")
            .fold((0u64, 0u64), |(rx, tx), (_, d)| {
                (
                    rx.saturating_add(d.total_received()),
                    tx.saturating_add(d.total_transmitted()),
                )
            })
    }

    pub(crate) fn collect(&mut self) -> HeartbeatPayload {
        // Avoid scanning the process table on every heartbeat.
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        let cpu = self.sys.global_cpu_usage().clamp(0.0, 100.0);
        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let mem_pct = if mem_total > 0 {
            (mem_used as f32 / mem_total as f32) * 100.0
        } else {
            0.0
        };

        let disks = sysinfo::Disks::new_with_refreshed_list();
        let (disk_total, disk_used) = disks.iter().fold((0u64, 0u64), |(t, u), d| {
            (
                t.saturating_add(d.total_space()),
                u.saturating_add(d.total_space().saturating_sub(d.available_space())),
            )
        });
        let disk_pct = if disk_total > 0 {
            ((disk_used as f64 / disk_total as f64) * 100.0) as f32
        } else {
            0.0
        };

        let now = std::time::Instant::now();
        let (rx, tx) = self.network_totals();
        let (rx_rate, tx_rate) = match self.last_net {
            Some((at, prx, ptx)) => {
                let secs = now.duration_since(at).as_secs_f64();
                (rate_mbps(prx, rx, secs), rate_mbps(ptx, tx, secs))
            }
            // The first sample has no baseline.
            None => (0.0, 0.0),
        };
        self.last_net = Some((now, rx, tx));

        let reported_cores = self.sys.cpus().len();
        let available_cores = if reported_cores == 0 {
            std::thread::available_parallelism().ok().map(|n| n.get())
        } else {
            None
        };
        let load = System::load_average();
        HeartbeatPayload {
            cpu_usage: cpu,
            memory_usage_percent: mem_pct,
            disk_usage_percent: disk_pct,
            load_one: load.one as f32,
            load_five: load.five as f32,
            load_fifteen: load.fifteen as f32,
            cpu_cores: cpu_core_count(reported_cores, available_cores),
            memory_total_mb: mem_total / 1024 / 1024,
            memory_used_mb: mem_used / 1024 / 1024,
            disk_total_gb: disk_total as f64 / GIB,
            disk_used_gb: disk_used as f64 / GIB,
            network_rx_mbps: rx_rate,
            network_tx_mbps: tx_rate,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RegisterPayload {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) agent_port: u16,
    pub(crate) auth_token: String,
}

pub(crate) async fn register(config: &AgentConfig) -> Option<i64> {
    let payload = RegisterPayload {
        name: config.node_name.clone(),
        host: config.node_host.clone(),
        agent_port: config.agent_port,
        auth_token: config.auth_token.clone(),
    };

    let client = http_client();
    let mut builder = client
        .post(format!("{}/api/nodes/register", config.panel_url))
        .json(&payload)
        .timeout(Duration::from_secs(10));
    // A3.2：注册携带引导令牌（未配置时省略，由面板按 401 拒绝）
    if !config.bootstrap_token.is_empty() {
        builder = builder.header("X-Bootstrap-Token", config.bootstrap_token.clone());
    }
    match builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                #[derive(Deserialize)]
                struct R {
                    id: i64,
                }
                if let Ok(r) = resp.json::<R>().await {
                    eprintln!("[agent] 注册成功，节点 ID: {}", r.id);
                    return Some(r.id);
                }
            }
            eprintln!("[agent] 注册失败: HTTP {status}");
        }
        Err(e) => eprintln!("[agent] 注册请求失败: {e}"),
    }
    None
}

pub(crate) async fn send_heartbeat(
    panel_url: &str,
    node_id: i64,
    metrics: &HeartbeatPayload,
    auth_token: &str,
) {
    let client = http_client();
    match client
        .post(format!("{panel_url}/api/nodes/heartbeat/{node_id}"))
        .bearer_auth(auth_token)
        .json(metrics)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                eprintln!("[agent] 心跳失败: HTTP {}", resp.status());
            }
        }
        Err(e) => eprintln!("[agent] 心跳请求失败: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_metrics_reports_percentages_in_range() {
        let mut c = MetricsCollector::new();
        let _ = c.collect();
        let m = c.collect();
        for v in [m.cpu_usage, m.memory_usage_percent, m.disk_usage_percent] {
            assert!((0.0..=100.0).contains(&v), "{v}");
        }
        assert!(m.load_one >= 0.0);
        // 序列化字段名是与面板的协议（Doc/23）
        let json = serde_json::to_value(&m).unwrap();
        for k in [
            "cpu_usage",
            "memory_usage_percent",
            "disk_usage_percent",
            "load_one",
            "load_five",
            "cpu_cores",
            "memory_total_mb",
            "disk_total_gb",
            "network_rx_mbps",
            "network_tx_mbps",
        ] {
            assert!(json.get(k).is_some(), "{k}");
        }
        assert!(m.cpu_cores >= 1);
        assert!(m.memory_used_mb <= m.memory_total_mb);
        assert!(m.network_rx_mbps >= 0.0 && m.network_tx_mbps >= 0.0);
    }

    #[test]
    fn cpu_core_count_prefers_topology_and_handles_missing_probes() {
        assert_eq!(cpu_core_count(16, Some(8)), 16);
        assert_eq!(cpu_core_count(0, Some(8)), 8);
        assert_eq!(cpu_core_count(0, None), 1);
        assert_eq!(cpu_core_count(0, Some(0)), 1);
    }

    #[test]
    fn rate_mbps_handles_resets_and_bad_intervals() {
        assert_eq!(rate_mbps(0, 10 * 1024 * 1024, 10.0), 1.0);
        assert_eq!(rate_mbps(500, 100, 10.0), 0.0, "计数回绕记 0");
        assert_eq!(rate_mbps(0, 100, 0.0), 0.0);
        assert_eq!(rate_mbps(0, 100, f64::NAN), 0.0);
    }
}
