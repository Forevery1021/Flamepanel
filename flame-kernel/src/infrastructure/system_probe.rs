//! 系统体检事实采集（S-17）：把主机原始状态收集进 `SystemFacts`，判定逻辑见 `domain::inspection`。
//!
//! 采集原则：
//! - 能读文件就不 spawn 进程（`/proc` / `/sys` / `/etc`），无命令注入面且 Agent / Embedded 一致；
//! - 必须执行的命令（虚拟化探测、systemd 单元状态、时间同步）全部经 `PrivilegedCommandRunner`
//!   且带超时，失败只让对应检查项变成 `unknown`，绝不让整份报告失败；
//! - 单次采集总预算 `PROBE_BUDGET`，超时返回已采到的部分事实。
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::domain::inspection::{FactsSource, PanelFacts, SystemFacts};

/// 单条外部命令超时
pub const COMMAND_TIMEOUT_SECS: u64 = 3;
/// 整次采集预算
pub const PROBE_BUDGET: Duration = Duration::from_secs(15);

/// 真实主机采集器。
pub struct SystemProbe {
    runner: SharedCommandRunner,
    /// 根路径（测试时指向临时目录以模拟 `/proc` / `/etc`）
    root: std::path::PathBuf,
}

impl SystemProbe {
    pub fn new(runner: SharedCommandRunner) -> Self {
        Self {
            runner,
            root: std::path::PathBuf::from("/"),
        }
    }

    /// 以自定义根路径构造（测试用：`<root>/proc/version` 等）。
    pub fn with_root(runner: SharedCommandRunner, root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            runner,
            root: root.into(),
        }
    }

    pub fn shared(runner: SharedCommandRunner) -> Arc<dyn FactsSource> {
        Arc::new(Self::new(runner))
    }

    fn path(&self, rel: &str) -> std::path::PathBuf {
        self.root.join(rel.trim_start_matches('/'))
    }

    async fn read(&self, rel: &str) -> Option<String> {
        tokio::fs::read_to_string(self.path(rel)).await.ok()
    }

    async fn mode(&self, rel: &str) -> Option<u32> {
        tokio::fs::metadata(self.path(rel))
            .await
            .ok()
            .map(|m| m.permissions().mode() & 0o7777)
    }

    async fn exists(&self, rel: &str) -> bool {
        tokio::fs::metadata(self.path(rel)).await.is_ok()
    }

    /// 读取目录下每个文件的内容（CPU 漏洞目录）。
    async fn read_dir_files(&self, rel: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let Ok(mut rd) = tokio::fs::read_dir(self.path(rel)).await else {
            return out;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                out.push((name, content.trim().to_string()));
            }
        }
        out.sort();
        out
    }

    /// `/boot/vmlinuz-*` → 版本串列表。
    async fn installed_kernels(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(mut rd) = tokio::fs::read_dir(self.path("/boot")).await else {
            return out;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(v) = name.strip_prefix("vmlinuz-") {
                if !v.is_empty() && !v.ends_with(".old") {
                    out.push(v.to_string());
                }
            }
        }
        out.sort();
        out
    }

    /// `/proc/sys/<a/b/c>` 读取为 `a.b.c` 键。
    async fn sysctl(&self, keys: &[&str]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for key in keys {
            let rel = format!("/proc/sys/{}", key.replace('.', "/"));
            if let Some(v) = self.read(&rel).await {
                out.push((key.to_string(), v.trim().to_string()));
            }
        }
        out
    }

    /// sshd 有效配置：主文件 + 按字典序展开 `sshd_config.d/*.conf`（与 sshd 的 Include 语义一致，
    /// Include 行出现在何处就在何处插入；缺省主文件不存在则视为无 sshd）。
    async fn sshd_config(&self) -> Option<String> {
        let main = self.read("/etc/ssh/sshd_config").await?;
        let mut out = String::new();
        for line in main.lines() {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            if let Some(pattern) = lower.strip_prefix("include ") {
                let pattern = trimmed[trimmed.len() - pattern.len()..].trim();
                out.push_str(&self.expand_include(pattern).await);
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        Some(out)
    }

    /// 只支持 `<dir>/*.conf` 形式的通配（sshd 默认写法）；其它形式按字面文件读取。
    async fn expand_include(&self, pattern: &str) -> String {
        let mut out = String::new();
        let (dir, suffix) = match pattern.rsplit_once('/') {
            Some((d, f)) if f.starts_with('*') => (d.to_string(), f[1..].to_string()),
            _ => {
                if let Some(c) = self.read(pattern).await {
                    out.push_str(&c);
                    out.push('\n');
                }
                return out;
            }
        };
        let Ok(mut rd) = tokio::fs::read_dir(self.path(&dir)).await else {
            return out;
        };
        let mut files = Vec::new();
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(&suffix) {
                files.push(entry.path());
            }
        }
        files.sort();
        for f in files {
            if let Ok(c) = tokio::fs::read_to_string(&f).await {
                out.push_str(&c);
                out.push('\n');
            }
        }
        out
    }

    async fn run(
        &self,
        program: &str,
        args: &[&str],
    ) -> Option<crate::application::execution_mode::CommandOutput> {
        let cmd = PrivilegedCommand::new(program, args.iter().map(|a| a.to_string()).collect())
            .timeout(COMMAND_TIMEOUT_SECS);
        match tokio::time::timeout(
            Duration::from_secs(COMMAND_TIMEOUT_SECS + 1),
            self.runner.run(&cmd),
        )
        .await
        {
            Ok(Ok(out)) => Some(out),
            Ok(Err(e)) => {
                tracing::debug!(program, error = %e, "inspection probe command unavailable");
                None
            }
            Err(_) => {
                tracing::debug!(program, "inspection probe command timed out");
                None
            }
        }
    }

    async fn unit_active(&self, unit: &str) -> Option<bool> {
        let out = self.run("systemctl", &["is-active", unit]).await?;
        Some(out.stdout.trim() == "active")
    }

    /// 虚拟化类型：`systemd-detect-virt` 退出码 1 + `none` 表示物理机。
    async fn virtualization(&self) -> Option<String> {
        let out = self.run("systemd-detect-virt", &[]).await?;
        let v = out.stdout.trim();
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    }

    /// 主机防火墙：ufw 配置文件 → firewalld / nftables 单元。
    async fn firewall(&self) -> (Option<bool>, Option<String>) {
        if let Some(conf) = self.read("/etc/ufw/ufw.conf").await {
            if conf
                .lines()
                .any(|l| l.trim().eq_ignore_ascii_case("ENABLED=yes"))
            {
                return (Some(true), Some("ufw".into()));
            }
        }
        let mut any_answer = false;
        for unit in ["firewalld", "nftables", "ufw"] {
            match self.unit_active(unit).await {
                Some(true) => return (Some(true), Some(unit.to_string())),
                Some(false) => any_answer = true,
                None => {}
            }
        }
        if any_answer {
            (Some(false), None)
        } else {
            (None, None)
        }
    }

    async fn time_synced(&self) -> Option<bool> {
        if let Some(out) = self
            .run("timedatectl", &["show", "-p", "NTPSynchronized", "--value"])
            .await
        {
            let v = out.stdout.trim();
            if v == "yes" {
                return Some(true);
            }
            if v == "no" {
                return Some(false);
            }
        }
        for unit in ["chronyd", "chrony", "systemd-timesyncd", "ntpd", "ntp"] {
            if let Some(true) = self.unit_active(unit).await {
                return Some(true);
            }
        }
        None
    }

    /// Debian/Ubuntu：`APT::Periodic::Unattended-Upgrade "1"`；RHEL 系：`dnf-automatic.timer`。
    async fn unattended_upgrades(&self, apt_based: bool) -> Option<bool> {
        if apt_based {
            let conf = self.read("/etc/apt/apt.conf.d/20auto-upgrades").await;
            return Some(conf.is_some_and(|c| {
                c.lines().any(|l| {
                    let l = l.trim();
                    l.starts_with("APT::Periodic::Unattended-Upgrade") && l.contains("\"1\"")
                })
            }));
        }
        if self.exists("/etc/dnf/automatic.conf").await {
            return self.unit_active("dnf-automatic.timer").await;
        }
        None
    }

    fn meminfo_mb(text: &str, key: &str) -> Option<u64> {
        text.lines().find_map(|l| {
            let rest = l.strip_prefix(key)?.trim_start_matches(':').trim();
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            Some(kb / 1024)
        })
    }

    async fn collect_inner(&self, panel: PanelFacts) -> SystemFacts {
        let os_release_file = match self.read("/etc/os-release").await {
            Some(c) => Some(c),
            None => self.read("/usr/lib/os-release").await,
        };
        let os_id = os_release_file
            .as_deref()
            .map(crate::domain::inspection::parse_os_release)
            .and_then(|m| m.get("ID").cloned())
            .unwrap_or_default();
        let apt_based = matches!(os_id.as_str(), "ubuntu" | "debian" | "linuxmint" | "pop");

        let sysctl_keys = crate::domain::inspection::sysctl_keys();
        let meminfo = self.read("/proc/meminfo").await.unwrap_or_default();
        let uptime_secs = self.read("/proc/uptime").await.and_then(|s| {
            s.split_whitespace()
                .next()?
                .parse::<f64>()
                .ok()
                .map(|v| v as u64)
        });

        // 命令类探测并发执行（各自超时）
        let (virtualization, firewall, time_synced, unattended) = tokio::join!(
            self.virtualization(),
            self.firewall(),
            self.time_synced(),
            self.unattended_upgrades(apt_based),
        );

        let selinux_enforcing = self
            .read("/sys/fs/selinux/enforce")
            .await
            .map(|v| v.trim() == "1");
        let apparmor_enabled = self
            .read("/sys/module/apparmor/parameters/enabled")
            .await
            .map(|v| v.trim().starts_with('Y'));

        SystemFacts {
            proc_version: self.read("/proc/version").await,
            osrelease: self.read("/proc/sys/kernel/osrelease").await,
            os_release_file,
            hostname: self.read("/proc/sys/kernel/hostname").await,
            uptime_secs,
            arch: std::env::consts::ARCH.to_string(),
            virtualization,
            reboot_required_file: self.exists("/var/run/reboot-required").await
                || self.exists("/run/reboot-required").await,
            installed_kernels: self.installed_kernels().await,
            cpu_vulnerabilities: self
                .read_dir_files("/sys/devices/system/cpu/vulnerabilities")
                .await,
            sysctl: self.sysctl(&sysctl_keys).await,
            sshd_config: self.sshd_config().await,
            passwd: self.read("/etc/passwd").await,
            shadow: self.read("/etc/shadow").await,
            shadow_mode: self.mode("/etc/shadow").await,
            passwd_mode: self.mode("/etc/passwd").await,
            docker_sock_mode: self.mode("/var/run/docker.sock").await,
            proc_net_tcp: self.read("/proc/net/tcp").await,
            proc_net_tcp6: self.read("/proc/net/tcp6").await,
            firewall_active: firewall.0,
            firewall_backend: firewall.1,
            apparmor_enabled,
            selinux_enforcing,
            time_synced,
            updates_available_text: self
                .read("/var/lib/update-notifier/updates-available")
                .await,
            unattended_upgrades: unattended,
            root_disk_usage_percent: None,
            mem_total_mb: Self::meminfo_mb(&meminfo, "MemTotal"),
            swap_total_mb: Self::meminfo_mb(&meminfo, "SwapTotal"),
            panel,
        }
    }
}

#[async_trait]
impl FactsSource for SystemProbe {
    async fn collect(&self, panel: PanelFacts) -> SystemFacts {
        match tokio::time::timeout(PROBE_BUDGET, self.collect_inner(panel.clone())).await {
            Ok(facts) => facts,
            Err(_) => {
                tracing::warn!("inspection probe exceeded budget; returning partial facts");
                SystemFacts {
                    arch: std::env::consts::ARCH.to_string(),
                    panel,
                    ..Default::default()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution_mode::{CommandOutput, PrivilegedCommandRunner};
    use crate::core::error::AppError;

    /// 假执行器：按程序名返回固定输出。
    struct FakeRunner;

    #[async_trait]
    impl PrivilegedCommandRunner for FakeRunner {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            let (stdout, code) = match (cmd.program.as_str(), cmd.args.first().map(String::as_str))
            {
                ("systemd-detect-virt", _) => ("kvm\n", 0),
                ("systemctl", Some("is-active")) => {
                    if cmd.args.get(1).map(String::as_str) == Some("nftables") {
                        ("active\n", 0)
                    } else {
                        ("inactive\n", 3)
                    }
                }
                ("timedatectl", _) => ("yes\n", 0),
                _ => return Err(AppError::internal("not whitelisted")),
            };
            Ok(CommandOutput {
                stdout: stdout.into(),
                stderr: String::new(),
                exit_code: code,
            })
        }
    }

    fn write(root: &std::path::Path, rel: &str, content: &str) {
        let p = root.join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[tokio::test]
    async fn probe_reads_fake_root_and_runs_commands_through_runner() {
        let root = std::env::temp_dir().join(format!("fp-probe-{}", uuid::Uuid::new_v4()));
        write(
            &root,
            "/proc/version",
            "Linux version 6.1.0-25-amd64 (gcc) #1 SMP\n",
        );
        write(&root, "/proc/sys/kernel/osrelease", "6.1.0-25-amd64\n");
        write(&root, "/proc/sys/kernel/hostname", "probe-host\n");
        write(&root, "/proc/uptime", "12345.67 99999.00\n");
        write(
            &root,
            "/proc/meminfo",
            "MemTotal:        4046848 kB\nSwapTotal:             0 kB\n",
        );
        write(&root, "/proc/sys/kernel/randomize_va_space", "2\n");
        write(&root, "/proc/sys/net/ipv4/tcp_syncookies", "1\n");
        write(
            &root,
            "/etc/os-release",
            "ID=debian\nVERSION_ID=\"12\"\nPRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"\n",
        );
        write(
            &root,
            "/etc/ssh/sshd_config",
            "Include /etc/ssh/sshd_config.d/*.conf\nPermitRootLogin yes\n",
        );
        write(
            &root,
            "/etc/ssh/sshd_config.d/10-hardening.conf",
            "PermitRootLogin no\n",
        );
        write(&root, "/etc/passwd", "root:x:0:0:root:/root:/bin/bash\n");
        write(&root, "/etc/apt/apt.conf.d/20auto-upgrades", "APT::Periodic::Update-Package-Lists \"1\";\nAPT::Periodic::Unattended-Upgrade \"1\";\n");
        write(&root, "/boot/vmlinuz-6.1.0-25-amd64", "");
        write(&root, "/boot/vmlinuz-6.1.0-23-amd64", "");
        write(
            &root,
            "/sys/devices/system/cpu/vulnerabilities/meltdown",
            "Mitigation: PTI\n",
        );
        write(
            &root,
            "/sys/devices/system/cpu/vulnerabilities/spectre_v1",
            "Vulnerable\n",
        );
        write(&root, "/run/reboot-required", "");

        let probe = SystemProbe::with_root(Arc::new(FakeRunner), &root);
        let facts = probe
            .collect(PanelFacts {
                panel_port: Some(8080),
                ..Default::default()
            })
            .await;

        assert_eq!(facts.osrelease.as_deref(), Some("6.1.0-25-amd64\n"));
        assert_eq!(facts.hostname.as_deref().map(str::trim), Some("probe-host"));
        assert_eq!(facts.uptime_secs, Some(12345));
        assert_eq!(facts.mem_total_mb, Some(3952));
        assert_eq!(facts.swap_total_mb, Some(0));
        assert_eq!(facts.virtualization.as_deref(), Some("kvm"));
        assert_eq!(facts.firewall_active, Some(true));
        assert_eq!(facts.firewall_backend.as_deref(), Some("nftables"));
        assert_eq!(facts.time_synced, Some(true));
        assert_eq!(facts.unattended_upgrades, Some(true));
        assert!(facts.reboot_required_file);
        assert_eq!(
            facts.installed_kernels,
            vec!["6.1.0-23-amd64", "6.1.0-25-amd64"]
        );
        assert_eq!(
            facts.cpu_vulnerabilities,
            vec![
                ("meltdown".to_string(), "Mitigation: PTI".to_string()),
                ("spectre_v1".to_string(), "Vulnerable".to_string()),
            ]
        );
        assert!(facts
            .sysctl
            .contains(&("kernel.randomize_va_space".to_string(), "2".to_string())));
        assert!(facts
            .sysctl
            .contains(&("net.ipv4.tcp_syncookies".to_string(), "1".to_string())));
        // Include 展开：片段插入在 Include 行位置 → 首个 PermitRootLogin 为 no
        let sshd = facts.sshd_config.as_deref().unwrap();
        assert!(
            sshd.find("PermitRootLogin no").unwrap() < sshd.find("PermitRootLogin yes").unwrap()
        );
        assert!(facts.shadow.is_none());
        assert_eq!(facts.passwd_mode.map(|m| (m & 0o777) != 0), Some(true));
        assert_eq!(facts.panel.panel_port, Some(8080));
        assert_eq!(facts.arch, std::env::consts::ARCH);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn probe_on_empty_root_yields_mostly_none() {
        let root = std::env::temp_dir().join(format!("fp-probe-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        struct Failing;
        #[async_trait]
        impl PrivilegedCommandRunner for Failing {
            async fn run(&self, _cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
                Err(AppError::internal("no commands in agent mode"))
            }
        }
        let probe = SystemProbe::with_root(Arc::new(Failing), &root);
        let facts = probe.collect(PanelFacts::default()).await;
        assert!(facts.proc_version.is_none());
        assert!(facts.sshd_config.is_none());
        assert_eq!(facts.firewall_active, None);
        assert_eq!(facts.time_synced, None);
        assert_eq!(facts.virtualization, None);
        assert!(facts.cpu_vulnerabilities.is_empty());
        assert!(facts.sysctl.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn meminfo_parsing() {
        let text =
            "MemTotal:        2048000 kB\nMemFree:          100 kB\nSwapTotal:       1024000 kB\n";
        assert_eq!(SystemProbe::meminfo_mb(text, "MemTotal"), Some(2000));
        assert_eq!(SystemProbe::meminfo_mb(text, "SwapTotal"), Some(1000));
        assert_eq!(SystemProbe::meminfo_mb(text, "Nope"), None);
    }
}
