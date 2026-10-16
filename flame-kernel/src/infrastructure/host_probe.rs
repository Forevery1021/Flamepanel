//! 主机详情事实采集（系统工具箱 · 主机详情）。解析 / 整形见 `domain::host_info`。
//!
//! 只读文件（`/etc/os-release` / `/etc/resolv.conf` / `/etc/hosts` / `/etc/localtime` 链接）
//! 与 sysinfo 读数，不 spawn 任何进程；整个采集放进 `spawn_blocking`，不阻塞运行时。
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::host_info::{HostFacts, HostFactsSource, RawDisk, RawInterface};
use crate::infrastructure::metrics::is_real_fs_type;

/// 单个文本文件读取上限（hosts 文件异常巨大时只读前 256 KiB）
const MAX_FILE_BYTES: usize = 256 * 1024;

pub struct HostProbe {
    /// 根路径（测试时指向临时目录模拟 `/etc`）
    root: PathBuf,
}

impl HostProbe {
    pub fn new() -> Self {
        Self {
            root: PathBuf::from("/"),
        }
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn shared() -> Arc<dyn HostFactsSource> {
        Arc::new(Self::new())
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel.trim_start_matches('/'))
    }

    fn read(&self, rel: &str) -> Option<String> {
        let bytes = std::fs::read(self.path(rel)).ok()?;
        let cut = bytes.len().min(MAX_FILE_BYTES);
        Some(String::from_utf8_lossy(&bytes[..cut]).into_owned())
    }

    /// 只采文件类事实（与 sysinfo 无关，便于用临时根目录单测）。
    fn collect_files(&self, facts: &mut HostFacts) {
        facts.os_release_file = self
            .read("etc/os-release")
            .or_else(|| self.read("usr/lib/os-release"));
        facts.resolv_conf = self.read("etc/resolv.conf");
        facts.hosts_file = self.read("etc/hosts");
        facts.timezone_file = self.read("etc/timezone");
        facts.localtime_target = std::fs::read_link(self.path("etc/localtime"))
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        if facts.hostname.is_none() {
            facts.hostname = self.read("etc/hostname");
        }
    }

    fn collect_blocking(&self) -> HostFacts {
        use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, System};

        let mut facts = HostFacts {
            hostname: System::host_name(),
            kernel_release: System::kernel_version(),
            arch: System::cpu_arch(),
            uptime_secs: Some(System::uptime()),
            boot_time: Some(System::boot_time()),
            cpu_physical: System::physical_core_count(),
            utc_offset: chrono::Local::now().offset().to_string(),
            ..Default::default()
        };
        let load = System::load_average();
        facts.load_average = [load.one, load.five, load.fifteen];

        let mut sys = System::new();
        sys.refresh_cpu_list(CpuRefreshKind::nothing());
        sys.refresh_memory_specifics(MemoryRefreshKind::everything());
        facts.cpu_logical = sys.cpus().len();
        facts.cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .filter(|b| !b.trim().is_empty());
        facts.mem_total = sys.total_memory();
        facts.mem_available = sys.available_memory();
        facts.swap_total = sys.total_swap();
        facts.swap_used = sys.used_swap();

        let networks = Networks::new_with_refreshed_list();
        facts.interfaces = networks
            .iter()
            .map(|(name, data)| {
                let mac = data.mac_address();
                RawInterface {
                    name: name.clone(),
                    mac: (!mac.is_unspecified()).then(|| mac.to_string()),
                    addresses: data
                        .ip_networks()
                        .iter()
                        .map(|n| format!("{}/{}", n.addr, n.prefix))
                        .collect(),
                    rx_bytes: data.total_received(),
                    tx_bytes: data.total_transmitted(),
                }
            })
            .collect();

        let disks = Disks::new_with_refreshed_list();
        facts.disks = disks
            .iter()
            .filter(|d| is_real_fs_type(&d.file_system().to_string_lossy()))
            .map(|d| RawDisk {
                device: d.name().to_string_lossy().into_owned(),
                mount_point: d.mount_point().to_string_lossy().into_owned(),
                fs_type: d.file_system().to_string_lossy().into_owned(),
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
            })
            .collect();

        self.collect_files(&mut facts);
        facts
    }
}

impl Default for HostProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HostFactsSource for HostProbe {
    async fn collect(&self) -> HostFacts {
        let probe = HostProbe::with_root(self.root.clone());
        tokio::task::spawn_blocking(move || probe.collect_blocking())
            .await
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_files_under_custom_root() {
        let dir = tempfile::tempdir().unwrap();
        let etc = dir.path().join("etc");
        std::fs::create_dir_all(&etc).unwrap();
        std::fs::write(etc.join("os-release"), "ID=debian\nVERSION_ID=12\n").unwrap();
        std::fs::write(etc.join("resolv.conf"), "nameserver 9.9.9.9\n").unwrap();
        std::fs::write(etc.join("hosts"), "127.0.0.1 localhost\n").unwrap();
        std::fs::write(etc.join("hostname"), "box\n").unwrap();
        std::os::unix::fs::symlink("/usr/share/zoneinfo/Europe/Paris", etc.join("localtime"))
            .unwrap();

        let probe = HostProbe::with_root(dir.path());
        let mut facts = HostFacts::default();
        probe.collect_files(&mut facts);
        assert_eq!(
            facts.os_release_file.as_deref(),
            Some("ID=debian\nVERSION_ID=12\n")
        );
        assert_eq!(facts.resolv_conf.as_deref(), Some("nameserver 9.9.9.9\n"));
        assert_eq!(facts.hostname.as_deref(), Some("box\n"));
        assert_eq!(
            facts.localtime_target.as_deref(),
            Some("/usr/share/zoneinfo/Europe/Paris")
        );
        assert_eq!(facts.timezone_file, None);
    }

    #[test]
    fn missing_files_are_none() {
        let dir = tempfile::tempdir().unwrap();
        let probe = HostProbe::with_root(dir.path());
        let mut facts = HostFacts::default();
        probe.collect_files(&mut facts);
        assert_eq!(facts, HostFacts::default());
    }

    #[tokio::test]
    async fn real_probe_returns_something_on_linux() {
        let facts = HostProbe::new().collect().await;
        assert!(!facts.arch.is_empty());
        assert!(facts.cpu_logical >= 1);
        assert!(facts.mem_total > 0);
    }
}
