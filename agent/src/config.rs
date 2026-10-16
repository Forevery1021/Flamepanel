//! Agent configuration, hostname discovery, and local IP detection.

use super::*;

/// `AGENT_MODE`: direct listens on `AGENT_PORT` (default); edge connects to
/// `/ws/agent/edge` without inbound ports; both enables migration with Edge preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentMode {
    Direct,
    Edge,
    Both,
}

impl AgentMode {
    pub(crate) fn parse(v: &str) -> Option<Self> {
        match v.trim().to_ascii_lowercase().as_str() {
            "" | "direct" => Some(Self::Direct),
            "edge" => Some(Self::Edge),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
    pub(crate) fn listens(self) -> bool {
        matches!(self, Self::Direct | Self::Both)
    }
    pub(crate) fn uses_edge(self) -> bool {
        matches!(self, Self::Edge | Self::Both)
    }
}

/// Accept `1`, `true`, `yes`, or `on`, ignoring case.
pub(crate) fn env_flag(v: Option<&str>) -> bool {
    matches!(
        v.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

#[derive(Debug, Clone)]
pub(crate) struct AgentConfig {
    pub(crate) panel_url: String,
    pub(crate) node_name: String,
    pub(crate) node_host: String,
    pub(crate) agent_port: u16,
    pub(crate) auth_token: String,
    /// Registration bootstrap credential sent in `X-Bootstrap-Token`.
    pub(crate) bootstrap_token: String,
    /// File sandbox root; defaults to the current directory.
    pub(crate) file_root: String,
    /// Invalid `AGENT_MODE` values warn and fall back to direct.
    pub(crate) mode: AgentMode,
    /// Opt-in terminal (`AGENT_TERMINAL=1`); bypasses the `/action` allowlist.
    pub(crate) terminal: bool,
}

impl AgentConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            panel_url: std::env::var("PANEL_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            node_name: std::env::var("NODE_NAME").unwrap_or_else(|_| hostname()),
            node_host: std::env::var("NODE_HOST").unwrap_or_else(|_| local_ip()),
            agent_port: std::env::var("AGENT_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9527),
            auth_token: std::env::var("AUTH_TOKEN")
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
            bootstrap_token: std::env::var("BOOTSTRAP_TOKEN").unwrap_or_default(),
            file_root: std::env::var("FILE_ROOT").unwrap_or_else(|_| ".".into()),
            mode: {
                let raw = std::env::var("AGENT_MODE").unwrap_or_default();
                AgentMode::parse(&raw).unwrap_or_else(|| {
                    eprintln!(
                        "[agent] 警告: AGENT_MODE={raw} 无效（direct|edge|both），按 direct 运行"
                    );
                    AgentMode::Direct
                })
            },
            terminal: env_flag(std::env::var("AGENT_TERMINAL").ok().as_deref()),
        }
    }
}

pub(crate) fn hostname() -> String {
    System::host_name().unwrap_or_else(|| "unknown".into())
}

pub(crate) fn local_ip() -> String {
    use std::net::UdpSocket;
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        if let Ok(()) = s.connect("8.8.8.8:80") {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".into()
}
