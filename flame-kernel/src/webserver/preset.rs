use serde::{Deserialize, Serialize};

use crate::application::app_store_service::SystemResources;

/// 性能预设：根据服务器资源自动选择最优配置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PerformancePreset {
    /// 低配：1核 / <2GB
    Low,
    /// 均衡：2核 / 2-4GB
    Medium,
    /// 高性能：4核 / 4-8GB
    High,
    /// 极限：8核+ / >8GB
    Ultra,
}

impl PerformancePreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "ultra" => Some(Self::Ultra),
            _ => None,
        }
    }

    pub fn description_zh(&self) -> &'static str {
        match self {
            Self::Low => "低配（1核 / <2GB）",
            Self::Medium => "均衡（2核 / 2-4GB）",
            Self::High => "高性能（4核 / 4-8GB）",
            Self::Ultra => "极限（8核+ / >8GB）",
        }
    }

    /// 根据系统资源自动推荐预设
    pub fn recommend(resources: &SystemResources) -> Self {
        if resources.cpu_cores >= 8 && resources.memory_mb >= 8192 {
            Self::Ultra
        } else if resources.cpu_cores >= 4 && resources.memory_mb >= 4096 {
            Self::High
        } else if resources.cpu_cores >= 2 && resources.memory_mb >= 2048 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// 各引擎的 worker 进程数
    pub fn worker_processes(&self, engine: &str) -> u32 {
        let cores_factor = match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 4,
            Self::Ultra => 8,
        };
        // OpenLiteSpeed 默认单进程 + 事件驱动，worker 数固定为 1
        if engine.eq_ignore_ascii_case("openlitespeed") || engine.eq_ignore_ascii_case("ols") {
            return 1;
        }
        cores_factor
    }

    /// 各引擎的 keepalive / 连接相关参数
    pub fn keepalive_timeout(&self) -> u32 {
        match self {
            Self::Low => 65,
            Self::Medium => 75,
            Self::High => 90,
            Self::Ultra => 120,
        }
    }

    pub fn gzip_enabled(&self) -> bool {
        true
    }

    /// 生成该预设下各引擎的全局配置片段（追加到引擎主配置）
    pub fn global_config_snippet(&self, engine: &str, port: u16) -> String {
        let workers = self.worker_processes(engine);
        let ka = self.keepalive_timeout();
        match engine.to_lowercase().as_str() {
            "nginx" | "openresty" => format!(
                "worker_processes {};\nworker_rlimit_nofile 65535;\nevents {{\n    worker_connections 10240;\n}}\nhttp {{\n    keepalive_timeout {};\n    gzip on;\n    gzip_comp_level 5;\n    gzip_types text/plain text/css application/json application/javascript;\n    server {{\n        listen {};\n        server_name localhost;\n        root /usr/share/nginx/html;\n    }}\n}}\n",
                workers, ka, port
            ),
            "apache" => format!(
                "ServerRoot \"/etc/httpd\"\nListen {}\n<IfModule prefork.c>\n    StartServers {}\n    MaxRequestWorkers {}\n</IfModule>\nKeepAlive On\nKeepAliveTimeout {}\n<Directory \"/var/www/html\">\n    AllowOverride All\n    Require all granted\n</Directory>\n",
                port, workers * 4, workers * 256, ka
            ),
            "caddy" => format!(
                ":{}\n{{\n    gzip\n    tls internal\n    root * /usr/share/caddy\n}}\n",
                port
            ),
            "openlitespeed" => format!(
                "listener default {{\n    address *:{}\n}}\nvhost default {{\n    root /var/www/html\n}}\n",
                port
            ),
            _ => format!("worker_processes {};\n", workers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(cpu: u32, mem: u64) -> SystemResources {
        SystemResources { cpu_cores: cpu, memory_mb: mem, is_ssd: true }
    }

    #[test]
    fn recommend_low() {
        assert_eq!(PerformancePreset::recommend(&res(1, 1024)), PerformancePreset::Low);
    }

    #[test]
    fn recommend_medium() {
        assert_eq!(PerformancePreset::recommend(&res(2, 4096)), PerformancePreset::Medium);
    }

    #[test]
    fn recommend_high() {
        assert_eq!(PerformancePreset::recommend(&res(4, 8192)), PerformancePreset::High);
    }

    #[test]
    fn recommend_ultra() {
        assert_eq!(PerformancePreset::recommend(&res(16, 32768)), PerformancePreset::Ultra);
    }

    #[test]
    fn worker_processes_scales() {
        assert_eq!(PerformancePreset::Ultra.worker_processes("nginx"), 8);
        assert_eq!(PerformancePreset::Low.worker_processes("apache"), 1);
    }

    #[test]
    fn ols_single_worker() {
        assert_eq!(PerformancePreset::Ultra.worker_processes("openlitespeed"), 1);
    }

    #[test]
    fn roundtrip_str() {
        for p in [PerformancePreset::Low, PerformancePreset::Medium, PerformancePreset::High, PerformancePreset::Ultra] {
            assert_eq!(PerformancePreset::from_str(p.as_str()), Some(p));
        }
    }

    #[test]
    fn snippets_not_empty() {
        for e in ["nginx", "apache", "caddy", "openlitespeed", "openresty"] {
            assert!(!PerformancePreset::High.global_config_snippet(e, 8080).is_empty());
        }
    }
}
