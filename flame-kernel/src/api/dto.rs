use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub username: String,
    pub password_hash: Option<String>,
    pub role: String,
}

/// 节点注册请求：兼容两种格式
/// - 面板/测试：`{"node": {name, hostname, ip_address, status, ...}}`
/// - Agent 平铺：`{name, host, agent_port, auth_token}`
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNodeRequest {
    #[serde(default)]
    pub node: Option<crate::domain::entity::ServerNode>,
    // 平铺字段（Agent 格式：{name, host, agent_port, auth_token}）
    #[serde(default)]
    pub name: String,
    #[serde(default, alias = "host")]
    pub hostname: String,
    #[serde(default)]
    pub ip_address: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Agent HTTP 服务端口（默认 9527）
    #[serde(default = "default_agent_port")]
    pub agent_port: u16,
}

fn default_agent_port() -> u16 {
    9527
}

impl CreateNodeRequest {
    pub fn to_node(&self) -> crate::domain::entity::ServerNode {
        if let Some(node) = &self.node {
            return node.clone();
        }
        crate::domain::entity::ServerNode {
            id: 0,
            name: if self.name.is_empty() {
                self.hostname.clone()
            } else {
                self.name.clone()
            },
            hostname: self.hostname.clone(),
            ip_address: self.ip_address.clone(),
            status: if self.status.is_empty() {
                "unknown".into()
            } else {
                self.status.clone()
            },
            created_at: chrono::Utc::now(),
            last_heartbeat_at: None,
            metrics_json: None,
            auth_token: self.auth_token.clone(),
            agent_port: self.agent_port,
        }
    }
}

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateWebsiteRequest {
    pub website: crate::domain::entity::Website,
}

#[derive(Debug, Serialize)]
pub struct WebServerResponse {
    pub id: i64,
    pub engine: String,
    pub version: Option<String>,
    pub status: String,
    pub config_path: String,
    pub binary_path: Option<String>,
    pub port: i32,
    pub created_at: String,
    pub resource_version: i64,
}

#[derive(Debug, Deserialize)]
pub struct PluginSettingRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct PluginMetricsResponse {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_ms: f64,
    pub max_execution_ms: u64,
    pub min_execution_ms: u64,
    pub last_execution_ms: u64,
    pub peak_memory_bytes: usize,
}

#[derive(Debug, Deserialize)]
pub struct PluginReloadRequest {
    pub wasm_base64: String,
    pub memory_limit_bytes: Option<usize>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebServerInstanceRequest {
    pub engine: String,
    pub version: Option<String>,
    pub config_path: Option<String>,
    pub binary_path: Option<String>,
    pub port: Option<i32>,
}
