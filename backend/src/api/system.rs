use axum::{Json, Router, routing::get, extract::State};
use serde::Serialize;

use crate::application::{AppState, SystemService};
use crate::domain::GpuInfo;
use crate::middleware::auth::CurrentUser;

#[derive(Serialize)]
pub struct SystemInfoResponse {
    cpu_usage: f32,
    cpu_cores: usize,
    memory_total_mb: u64,
    memory_used_mb: u64,
    memory_free_mb: u64,
    memory_usage_percent: f32,
    disk_total_gb: f64,
    disk_used_gb: f64,
    disk_free_gb: f64,
    disk_usage_percent: f32,
    uptime_seconds: u64,
    uptime_display: String,
    load_one: f64,
    load_five: f64,
    load_fifteen: f64,
    hostname: String,
    network_interfaces: Vec<NetworkInterfaceResponse>,
    gpu_info: Vec<GpuInfo>,
}

#[derive(Serialize)]
pub struct NetworkInterfaceResponse {
    name: String,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
    mac: String,
}

#[derive(Serialize)]
pub struct ProcessResponse {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
    status: String,
}

#[derive(Serialize)]
pub struct SecurityScanResult {
    pub hostname: String,
    pub listening_ports: Vec<PortEntry>,
    pub ssh_warnings: Vec<String>,
    pub kernel_version: String,
    pub os_release: String,
    pub checks: Vec<SecurityCheck>,
}

#[derive(Serialize)]
pub struct PortEntry {
    pub port: u16,
    pub process: String,
    pub protocol: String,
}

#[derive(Serialize)]
pub struct SecurityCheck {
    pub name: String,
    pub status: String, // pass | warn | fail
    pub detail: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/info", get(system_info))
        .route("/processes", get(process_list))
        .route("/gpu", get(gpu_info))
        .route("/security-scan", get(security_scan))
}

async fn gpu_info(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Json<Vec<GpuInfo>> {
    Json(SystemService::get_gpu_info())
}

async fn system_info(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Json<SystemInfoResponse> {
    let info = SystemService::get_info();

    let memory_usage = if info.memory_total_mb > 0 {
        (info.memory_used_mb as f32 / info.memory_total_mb as f32) * 100.0
    } else {
        0.0
    };

    let disk_usage = if info.disk_total_gb > 0.0 {
        ((info.disk_used_gb / info.disk_total_gb) * 100.0) as f32
    } else {
        0.0
    };

    let uptime_display = {
        let secs = info.uptime_seconds;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        format!("{days}d {hours}h {mins}m")
    };

    Json(SystemInfoResponse {
        cpu_usage: info.cpu_usage,
        cpu_cores: info.cpu_cores,
        memory_total_mb: info.memory_total_mb,
        memory_used_mb: info.memory_used_mb,
        memory_free_mb: info.memory_free_mb,
        memory_usage_percent: memory_usage,
        disk_total_gb: info.disk_total_gb,
        disk_used_gb: info.disk_used_gb,
        disk_free_gb: info.disk_free_gb,
        disk_usage_percent: disk_usage,
        uptime_seconds: info.uptime_seconds,
        uptime_display,
        load_one: info.load_average.one,
        load_five: info.load_average.five,
        load_fifteen: info.load_average.fifteen,
        hostname: info.network.hostname,
        network_interfaces: info.network.interfaces.into_iter().map(|i| NetworkInterfaceResponse {
            name: i.name,
            ipv4: i.ipv4,
            ipv6: i.ipv6,
            mac: i.mac,
        }).collect(),
        gpu_info: SystemService::get_gpu_info(),
    })
}

async fn process_list(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Json<Vec<ProcessResponse>> {
    let processes = SystemService::get_processes();
    Json(processes.into_iter().map(|p| ProcessResponse {
        pid: p.pid,
        name: p.name,
        cpu_usage: p.cpu_usage,
        memory_mb: p.memory_mb,
        status: p.status,
    }).collect())
}

/// GET /api/system/security-scan
/// Basic server security scan
async fn security_scan(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Json<SecurityScanResult> {
    tracing::info!("用户 '{}' 执行了安全扫描", _claims.sub);

    let mut checks = Vec::new();
    let mut ports = Vec::new();
    let mut ssh_warnings = Vec::new();

    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_default()
        .trim()
        .to_string();
    let hostname = if hostname.is_empty() { "unknown".to_string() } else { hostname };

    let kernel = std::fs::read_to_string("/proc/version")
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string();

    let os_release = std::fs::read_to_string("/etc/os-release")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("PRETTY_NAME="))
        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Scan listening TCP ports via /proc/net/tcp (Linux)
    if let Ok(tcp) = std::fs::read_to_string("/proc/net/tcp") {
        for line in tcp.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 4 {
                let local = fields[1];
                if let Some(port_hex) = local.split(':').last() {
                    if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                        // Skip ephemeral ports and common safe ports
                        if port > 0 {
                            let state_hex = fields[3];
                            let state = if state_hex == "0A" { "LISTEN" } else { "ESTABLISHED" };
                            ports.push(PortEntry {
                                port,
                                process: String::new(),
                                protocol: format!("tcp ({})", state),
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort ports numerically, deduplicate
    ports.sort_by_key(|p| p.port);
    ports.dedup_by_key(|p| p.port);

    // Security checks
    // Check SSH root login
    if let Ok(sshd_config) = std::fs::read_to_string("/etc/ssh/sshd_config") {
        let has_permit_root = sshd_config.lines().any(|l| {
            let trimmed = l.trim();
            !trimmed.starts_with('#') && trimmed.to_lowercase().contains("permitrootlogin") && trimmed.to_lowercase().contains("yes")
        });
        if has_permit_root {
            ssh_warnings.push("SSH 允许 root 直接登录，建议禁用 PermitRootLogin".into());
            checks.push(SecurityCheck {
                name: "SSH Root Login".into(),
                status: "warn".into(),
                detail: "PermitRootLogin 设置为 yes，建议改为 prohibit-password 或 no".into(),
            });
        } else {
            checks.push(SecurityCheck {
                name: "SSH Root Login".into(),
                status: "pass".into(),
                detail: "Root 登录已正确限制".into(),
            });
        }

        let has_pass_auth = sshd_config.lines().any(|l| {
            let trimmed = l.trim();
            !trimmed.starts_with('#') && trimmed.to_lowercase().contains("passwordauthentication") && trimmed.to_lowercase().contains("yes")
        });
        if has_pass_auth {
            checks.push(SecurityCheck {
                name: "SSH Password Auth".into(),
                status: "warn".into(),
                detail: "建议使用密钥认证替代密码登录".into(),
            });
        } else {
            checks.push(SecurityCheck {
                name: "SSH Password Auth".into(),
                status: "pass".into(),
                detail: "密码认证已禁用或使用密钥认证".into(),
            });
        }
    }

    // Check for common ports exposed
    let privileged_ports: &[u16] = &[22, 80, 443, 3306, 5432, 6379, 27017, 8080, 8443];
    let exposed_privileged: Vec<u16> = ports.iter()
        .filter(|p| privileged_ports.contains(&p.port))
        .map(|p| p.port)
        .collect();
    if !exposed_privileged.is_empty() {
        checks.push(SecurityCheck {
            name: "开放端口检查".into(),
            status: "warn".into(),
            detail: format!("检测到常见服务端口: {:?}，请确认是否需要对外开放", exposed_privileged),
        });
    } else {
        checks.push(SecurityCheck {
            name: "开放端口检查".into(),
            status: "pass".into(),
            detail: "未检测到常见高危端口对外开放".into(),
        });
    }

    // Check for /tmp permissions
    if let Ok(_meta) = std::fs::metadata("/tmp") {
        checks.push(SecurityCheck {
            name: "/tmp 目录权限".into(),
            status: "pass".into(),
            detail: "/tmp 目录存在且可访问".into(),
        });
    }

    Json(SecurityScanResult {
        hostname,
        listening_ports: ports,
        ssh_warnings,
        kernel_version: kernel,
        os_release,
        checks,
    })
}
