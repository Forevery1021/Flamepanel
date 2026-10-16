//! 面板侧：经特权助手访问 Web 引擎配置（`EngineHost`）与引擎命令（`PrivilegedCommandRunner`）。
//!
//! [`engine_access`] 在 `WebServerManager` 构造时选择实现：
//! - 面板以 root 运行 / Docker：本机直写（`LocalEngineHost`）+ 原执行器；
//! - 非 root 且助手 socket 存在：`HelperEngineHost` + `HelperCommandRunner`
//!   （白名单内命令交助手以 root 执行，其余仍走原执行器）；
//! - `OP_PRIV_HELPER=off` 强制本机直写；`OP_PRIV_HELPER_SOCKET` 覆盖 socket 路径。

use super::policy;
use super::protocol::{Op, Request, Response, RunOutput};
use super::server::DEFAULT_SOCKET;
use crate::application::execution_mode::{
    CommandOutput, PrivilegedCommand, PrivilegedCommandRunner, SharedCommandRunner,
};
use crate::core::error::AppError;
use crate::webserver::host::{EngineHost, LocalEngineHost, SharedEngineHost};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 文件类请求超时；命令类在命令自身超时之上再加余量。
const IO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct HelperClient {
    socket: PathBuf,
}

impl HelperClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    pub fn socket(&self) -> &std::path::Path {
        &self.socket
    }

    pub async fn call(&self, op: Op) -> Result<serde_json::Value, AppError> {
        let timeout = match &op {
            Op::Run { timeout_secs, .. } => {
                Duration::from_secs(timeout_secs.unwrap_or(60).saturating_add(10))
            }
            _ => IO_TIMEOUT,
        };
        let unavailable = |e: String| {
            AppError::ServiceUnavailable(format!(
                "privileged helper ({}) unavailable: {e}; check `systemctl status flamepanel-helper`",
                self.socket.display()
            ))
        };
        let exchange = async {
            let mut stream = tokio::net::UnixStream::connect(&self.socket)
                .await
                .map_err(|e| unavailable(e.to_string()))?;
            let mut line = serde_json::to_vec(&Request::new(op))
                .map_err(|e| AppError::internal(format!("encode helper request: {e}")))?;
            line.push(b'\n');
            stream
                .write_all(&line)
                .await
                .map_err(|e| unavailable(e.to_string()))?;
            let mut reader = BufReader::new(stream);
            let mut buf = String::new();
            reader
                .read_line(&mut buf)
                .await
                .map_err(|e| unavailable(e.to_string()))?;
            if buf.trim().is_empty() {
                return Err(unavailable("connection closed without a response".into()));
            }
            let resp: Response = serde_json::from_str(buf.trim_end())
                .map_err(|e| AppError::internal(format!("decode helper response: {e}")))?;
            resp.into_result()
        };
        tokio::time::timeout(timeout, exchange)
            .await
            .map_err(|_| unavailable(format!("timed out after {}s", timeout.as_secs())))?
    }

    pub async fn ping(&self) -> Result<serde_json::Value, AppError> {
        self.call(Op::Ping).await
    }
}

/// 经助手读写引擎配置。面板侧先做一次路径白名单预检，越界请求不出进程。
#[derive(Debug, Clone)]
pub struct HelperEngineHost {
    client: HelperClient,
}

impl HelperEngineHost {
    pub fn new(client: HelperClient) -> Self {
        Self { client }
    }
}

fn decode<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> Result<T, AppError> {
    serde_json::from_value(v).map_err(|e| AppError::internal(format!("helper response: {e}")))
}

#[async_trait]
impl EngineHost for HelperEngineHost {
    async fn read_log_chunk(
        &self,
        path: &str,
        offset: u64,
        inode: Option<u64>,
        max: usize,
    ) -> Result<Option<crate::webserver::host::LogChunk>, AppError> {
        policy::check_log_path(path)?;
        decode(
            self.client
                .call(Op::ReadLog {
                    path: path.into(),
                    offset,
                    inode,
                    max,
                })
                .await?,
        )
    }

    async fn read(&self, path: &str) -> Result<Option<String>, AppError> {
        policy::check_path(path)?;
        decode(self.client.call(Op::Read { path: path.into() }).await?)
    }

    async fn write_atomic(&self, path: &str, content: &str) -> Result<(), AppError> {
        policy::check_path(path)?;
        self.client
            .call(Op::Write {
                path: path.into(),
                content: content.into(),
            })
            .await
            .map(|_| ())
    }

    async fn remove(&self, path: &str) -> Result<bool, AppError> {
        policy::check_path(path)?;
        decode(self.client.call(Op::Remove { path: path.into() }).await?)
    }

    async fn exists(&self, path: &str) -> bool {
        if !policy::path_allowed(path) {
            return false;
        }
        match self.client.call(Op::Exists { path: path.into() }).await {
            Ok(v) => v.as_bool().unwrap_or(false),
            Err(e) => {
                tracing::warn!("helper exists({path}) failed: {e}");
                false
            }
        }
    }

    async fn writable(&self, dir: &str) -> bool {
        if !policy::path_allowed(dir) {
            return false;
        }
        match self.client.call(Op::Writable { path: dir.into() }).await {
            Ok(v) => v.as_bool().unwrap_or(false),
            Err(e) => {
                tracing::warn!("helper writable({dir}) failed: {e}");
                false
            }
        }
    }
}

/// 白名单内的引擎命令交助手以 root 执行；其余（`pgrep`、实例自定义二进制等）走 `fallback`。
pub struct HelperCommandRunner {
    client: HelperClient,
    fallback: SharedCommandRunner,
}

impl HelperCommandRunner {
    pub fn new(client: HelperClient, fallback: SharedCommandRunner) -> Self {
        Self { client, fallback }
    }
}

#[async_trait]
impl PrivilegedCommandRunner for HelperCommandRunner {
    async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
        if !policy::command_allowed(cmd) {
            return self.fallback.run(cmd).await;
        }
        let out: RunOutput = decode(self.client.call(Op::run(cmd)).await?)?;
        Ok(out.into())
    }
}

/// 助手启用策略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperMode {
    /// 本机直写（root / Docker / `OP_PRIV_HELPER=off`）。
    Local,
    /// 经助手（socket 路径）。
    Helper(PathBuf),
}

/// 纯函数决策（便于测试）：`setting` 为 `OP_PRIV_HELPER`（auto / on / off，默认 auto）。
///
/// - off → Local；on → Helper（即使 socket 暂不存在，调用时返回 503 提示，而非静默直写失败）；
/// - auto → 非 root 且 socket 存在才用 Helper。
pub fn decide(setting: &str, is_root: bool, socket: PathBuf, socket_exists: bool) -> HelperMode {
    match setting.trim().to_ascii_lowercase().as_str() {
        "off" | "0" | "false" | "no" => HelperMode::Local,
        "on" | "1" | "true" | "yes" => HelperMode::Helper(socket),
        _ if !is_root && socket_exists => HelperMode::Helper(socket),
        _ => HelperMode::Local,
    }
}

pub fn helper_socket_from_env() -> PathBuf {
    std::env::var("OP_PRIV_HELPER_SOCKET")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET))
}

/// 读取环境决定当前进程的助手模式（结果按进程缓存，日志只打一次）。
pub fn helper_mode() -> HelperMode {
    static MODE: std::sync::OnceLock<HelperMode> = std::sync::OnceLock::new();
    MODE.get_or_init(|| {
        let socket = helper_socket_from_env();
        let exists = socket.exists();
        let mode = decide(
            &std::env::var("OP_PRIV_HELPER").unwrap_or_default(),
            crate::infrastructure::os::is_root_process(),
            socket,
            exists,
        );
        if let HelperMode::Helper(p) = &mode {
            tracing::info!(
                socket = %p.display(),
                "web engine config writes are delegated to the privileged helper"
            );
        }
        mode
    })
    .clone()
}

/// 只需命令路由（无配置文件读写）的适配器用：助手模式下白名单内命令交助手，其余走 `runner`。
/// 防火墙适配器经此接入（Doc/30 助手扩展）；root / 未启用助手时原样返回 `runner`。
pub fn command_access(runner: SharedCommandRunner) -> SharedCommandRunner {
    match helper_mode() {
        HelperMode::Local => runner,
        HelperMode::Helper(socket) => {
            Arc::new(HelperCommandRunner::new(HelperClient::new(socket), runner))
        }
    }
}

/// `WebServerManager` 用的 (runner, host) 组合。
pub fn engine_access(runner: SharedCommandRunner) -> (SharedCommandRunner, SharedEngineHost) {
    match helper_mode() {
        HelperMode::Local => (runner, Arc::new(LocalEngineHost::from_env())),
        HelperMode::Helper(socket) => {
            let client = HelperClient::new(socket);
            (
                Arc::new(HelperCommandRunner::new(client.clone(), runner)),
                Arc::new(HelperEngineHost::new(client)),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_matrix() {
        let s = || PathBuf::from("/run/x.sock");
        assert_eq!(decide("", false, s(), true), HelperMode::Helper(s()));
        assert_eq!(decide("auto", false, s(), false), HelperMode::Local);
        assert_eq!(decide("", true, s(), true), HelperMode::Local);
        assert_eq!(decide("off", false, s(), true), HelperMode::Local);
        assert_eq!(decide("ON", true, s(), false), HelperMode::Helper(s()));
    }

    #[tokio::test]
    async fn missing_socket_is_service_unavailable() {
        let c = HelperClient::new("/nonexistent/flamepanel-helper.sock");
        assert!(matches!(
            c.ping().await,
            Err(AppError::ServiceUnavailable(_))
        ));
        // 越界路径在面板侧即被拒，不会触达 socket
        let h = HelperEngineHost::new(c);
        assert!(matches!(
            h.write_atomic("/etc/shadow", "x").await,
            Err(AppError::Forbidden(_))
        ));
        assert!(!h.exists("/etc/shadow").await);
    }
}
