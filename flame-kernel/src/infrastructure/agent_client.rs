//! 面板 → Agent 远程调用客户端（Stage5 多节点能力）
//!
//! 通过 Agent 暴露的 HTTP 端点（`/action`、`/files/list`、`/files/download`、`/files/upload`）
//! 实现远程命令执行与远程文件管理，并叠加重试 + 熔断（复用 resilience 模块）。
//! 请求携带 `Authorization: Bearer <auth_token>` 完成 Agent 侧鉴权。
//! S4：远程命令统一走 `/action` 动作枚举（`whitelisted_command`），不再使用已删除的 `/exec`。
//!
//! **传输分发**（Doc/30 Agent Edge）：`base_url` 为 `http(s)://…` 时直连 Agent；为
//! `edge://<node_id>` 时经 [`EdgeHub`] 反向通道回放同一请求（方法 + 路径 + body 不变），
//! 上层调用方无需区分节点处于直连还是 Edge 模式。

use crate::core::error::AppError;
use crate::infrastructure::edge::{parse_edge_base_url, EdgeHub};
use crate::resilience::{retry_with_backoff, CircuitBreaker, CircuitBreakerConfig, RetryConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 远程文件条目（与 Agent `/files/list` 响应一致）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteFileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

/// 远程命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteExecResult {
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Agent 动作枚举请求（Phase A1）
///
/// 序列化格式：`{"action":"ping"|"system_info"|...,"params":{...}}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActionRequest {
    pub action: String,
    pub params: serde_json::Value,
}

/// Agent HTTP 客户端：负责与单台 Agent 通信的端口实现
#[derive(Clone)]
pub struct AgentClient {
    /// 面板到 Agent 的 HTTP 客户端（连接复用）
    http: reqwest::Client,
    /// 远程调用熔断器（每客户端实例独立，节点粒度）
    circuit_breaker: CircuitBreaker,
    /// 重试配置（指数退避）
    retry_config: RetryConfig,
    /// 默认请求超时
    timeout: Duration,
    /// Agent Edge 会话注册表（`edge://` base_url 经此转发）
    edge: EdgeHub,
}

/// 一次 Agent 调用的原始结果（两种传输统一形态）。
struct RawResponse {
    status: u16,
    body: Vec<u8>,
}

/// 请求体：原始字节或 JSON。
enum Payload {
    Empty,
    Bytes(Vec<u8>),
    Json(serde_json::Value),
}

/// 拼接 `path?query`（query 按 `application/x-www-form-urlencoded` 编码）。
fn path_with_query(path: &str, query: &[(&str, &str)]) -> String {
    if query.is_empty() {
        return path.to_string();
    }
    let qs: String = query
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{qs}")
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl AgentClient {
    /// 构造客户端
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            circuit_breaker: CircuitBreaker::new(CircuitBreakerConfig::default()),
            retry_config: RetryConfig {
                max_retries: 2,
                initial_delay: Duration::from_millis(200),
                max_delay: Duration::from_secs(2),
                backoff_multiplier: 2.0,
            },
            timeout: Duration::from_secs(30),
            edge: EdgeHub::new(),
        }
    }

    /// Edge 会话注册表（WS 端点注册连接、节点服务判定是否走反向通道）。
    pub fn edge(&self) -> &EdgeHub {
        &self.edge
    }

    /// 单次调用（不含重试 / 熔断）：按 base_url 选择直连 HTTP 或 Edge 反向通道。
    async fn send_once(
        &self,
        base_url: &str,
        method: &str,
        path: &str,
        token: &str,
        payload: &Payload,
    ) -> Result<RawResponse, String> {
        if let Some(node_id) = parse_edge_base_url(base_url) {
            let (ct, body) = match payload {
                Payload::Empty => (None, Vec::new()),
                Payload::Bytes(b) => (Some("application/octet-stream"), b.clone()),
                Payload::Json(v) => (
                    Some("application/json"),
                    serde_json::to_vec(v).map_err(|e| format!("encode body: {e}"))?,
                ),
            };
            // Edge 帧内不再携带 token：Agent 回放时自行注入本机 token（通道本身已在握手时鉴权）
            let resp = self
                .edge
                .request(node_id, method, path, ct, &body, self.timeout)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(RawResponse {
                status: resp.status,
                body: resp.body,
            });
        }
        let url = format!("{}{}", base_url.trim_end_matches('/'), path);
        let req = match method {
            "POST" => self.http.post(&url),
            _ => self.http.get(&url),
        }
        .bearer_auth(token)
        .timeout(self.timeout);
        let req = match payload {
            Payload::Empty => req,
            Payload::Bytes(b) => req.body(b.clone()),
            Payload::Json(v) => req.json(v),
        };
        let resp = req
            .send()
            .await
            .map_err(|e| format!("agent request failed: {e}"))?;
        let status = resp.status().as_u16();
        let body = resp
            .bytes()
            .await
            .map_err(|e| format!("agent response read failed: {e}"))?
            .to_vec();
        Ok(RawResponse { status, body })
    }

    /// 带重试 + 熔断的调用；非 2xx 视为失败（与原各方法语义一致）。
    async fn send(
        &self,
        base_url: &str,
        method: &str,
        path: String,
        token: &str,
        payload: Payload,
    ) -> Result<Vec<u8>, AppError> {
        self.circuit_breaker
            .call(|| async {
                retry_with_backoff(&self.retry_config, || async {
                    let r = self
                        .send_once(base_url, method, &path, token, &payload)
                        .await?;
                    if !(200..300).contains(&r.status) {
                        return Err(format!("agent returned HTTP {}", r.status));
                    }
                    Ok(r.body)
                })
                .await
            })
            .await
    }

    /// 在远程节点执行命令
    ///
    /// - `base_url`：`http://<host>:<agent_port>`
    /// - `token`：Agent 注册时下发的 `auth_token`
    /// - `command`：要执行的 shell 命令
    /// - `timeout_secs`：命令超时（None 使用客户端默认）
    ///
    /// S4：统一走 `/action` 动作枚举（`whitelisted_command`），不再调用已删除的 `/exec` 端点。
    /// Agent 侧只接受白名单命令，非白名单命令返回 `ACTION_NOT_ALLOWED`。
    pub async fn execute(
        &self,
        base_url: &str,
        token: &str,
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<RemoteExecResult, AppError> {
        let value = self
            .whitelisted_command(base_url, token, command, timeout_secs)
            .await?;

        // `whitelisted_command` 结果：`{status: ok, data:{output, exit_code}}`
        // 或 `{status: err, data:{code, message}}`。
        if value.get("status").and_then(|v| v.as_str()) == Some("err") {
            let code = value
                .pointer("/data/code")
                .and_then(|v| v.as_str())
                .unwrap_or("ACTION_ERROR");
            let message = value
                .pointer("/data/message")
                .and_then(|v| v.as_str())
                .unwrap_or("agent rejected action");
            return Err(AppError::internal(format!(
                "agent rejected privileged command: {} ({})",
                message, code
            )));
        }

        let output = value
            .pointer("/data/output")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let exit_code = value
            .pointer("/data/exit_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        let duration_ms = value
            .pointer("/data/duration_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(RemoteExecResult {
            output,
            exit_code: exit_code as i32,
            duration_ms,
        })
    }

    /// 列出远程节点目录内容
    pub async fn list_files(
        &self,
        base_url: &str,
        token: &str,
        path: &str,
    ) -> Result<Vec<RemoteFileEntry>, AppError> {
        let body = self
            .send(
                base_url,
                "GET",
                path_with_query("/files/list", &[("path", path)]),
                token,
                Payload::Empty,
            )
            .await?;
        serde_json::from_slice(&body)
            .map_err(|e| AppError::internal(format!("agent response parse failed: {e}")))
    }

    /// 下载远程节点文件（返回原始字节）
    pub async fn download_file(
        &self,
        base_url: &str,
        token: &str,
        path: &str,
    ) -> Result<Vec<u8>, AppError> {
        self.send(
            base_url,
            "GET",
            path_with_query("/files/download", &[("path", path)]),
            token,
            Payload::Empty,
        )
        .await
    }

    /// 上传文件到远程节点（返回写入字节数）
    pub async fn upload_file(
        &self,
        base_url: &str,
        token: &str,
        path: &str,
        content: &[u8],
    ) -> Result<u64, AppError> {
        let body = self
            .send(
                base_url,
                "POST",
                path_with_query("/files/upload", &[("path", path)]),
                token,
                Payload::Bytes(content.to_vec()),
            )
            .await?;
        let v: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| AppError::internal(format!("agent response parse failed: {e}")))?;
        v.get("size")
            .and_then(|s| s.as_u64())
            .ok_or_else(|| AppError::internal("agent response missing size"))
    }

    // ── Phase A1：Agent 动作枚举客户端 ────────────────────────────────────

    /// 调用 Agent 动作枚举端点（`POST /action`）。
    /// 返回动作结果 JSON；`AgentAction` 序列化为 `{"action":"...","params":{...}}`。
    pub async fn call_action(
        &self,
        base_url: &str,
        token: &str,
        action: &AgentActionRequest,
    ) -> Result<serde_json::Value, AppError> {
        let payload = serde_json::to_value(action)
            .map_err(|e| AppError::internal(format!("AgentAction serialize failed: {e}")))?;
        let body = self
            .send(
                base_url,
                "POST",
                "/action".into(),
                token,
                Payload::Json(payload),
            )
            .await?;
        serde_json::from_slice(&body)
            .map_err(|e| AppError::internal(format!("agent action response parse failed: {e}")))
    }

    /// 便捷：Ping 探活
    pub async fn ping(&self, base_url: &str, token: &str) -> Result<serde_json::Value, AppError> {
        self.call_action(
            base_url,
            token,
            &AgentActionRequest {
                action: "ping".into(),
                params: serde_json::json!({}),
            },
        )
        .await
    }

    /// 便捷：获取系统信息
    pub async fn system_info(
        &self,
        base_url: &str,
        token: &str,
    ) -> Result<serde_json::Value, AppError> {
        self.call_action(
            base_url,
            token,
            &AgentActionRequest {
                action: "system_info".into(),
                params: serde_json::json!({}),
            },
        )
        .await
    }

    /// 便捷：白名单命令执行（Agent 侧拒绝非白名单命令）
    pub async fn whitelisted_command(
        &self,
        base_url: &str,
        token: &str,
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<serde_json::Value, AppError> {
        let params = if let Some(t) = timeout_secs {
            serde_json::json!({ "command": command, "timeout_secs": t })
        } else {
            serde_json::json!({ "command": command })
        };
        self.call_action(
            base_url,
            token,
            &AgentActionRequest {
                action: "whitelisted_command".into(),
                params,
            },
        )
        .await
    }

    /// 结构化命令执行（Doc/33 P-1：`exec` 动作，Agent 侧 `Command::new(program).args(..)`
    /// 直接执行、不经 shell；env / stdin 结构化传递，凭据与含密码的 SQL 不进 argv）。
    ///
    /// `params` 形如 `{"program","args":[..],"env":{..},"stdin"?,"timeout_secs"?}`；
    /// 旧版 Agent 不认识该动作会返回 HTTP 400（调用方据此回退 `whitelisted_command`）。
    pub async fn exec_command(
        &self,
        base_url: &str,
        token: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        self.call_action(
            base_url,
            token,
            &AgentActionRequest {
                action: "exec".into(),
                params,
            },
        )
        .await
    }

    /// 当前熔断器状态（供诊断/测试）
    pub async fn circuit_state(&self) -> crate::resilience::CircuitState {
        self.circuit_breaker.state().await
    }
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷构造：`http://<ip>:<port>`（R1：ip 若已带 scheme（如 `https://...`）则透传，
/// 允许面板在反向代理 TLS 终结后以 https 访问 agent；否则默认 http。
/// 安全部署要求：agent 应仅监听内网/loopback，并经反向代理做 TLS 终结，避免 token/命令明文嗅探。
pub fn agent_base_url(ip: &str, port: u16) -> String {
    let ip = ip.trim();
    if ip.starts_with("http://") || ip.starts_with("https://") {
        format!("{}:{}", ip.trim_end_matches('/'), port)
    } else {
        format!("http://{}:{}", ip, port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Bytes,
        http::{HeaderMap, StatusCode},
        routing::{get, post},
        Json, Router,
    };
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn check_auth(headers: &HeaderMap) -> bool {
        headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s == "Bearer test-token")
            .unwrap_or(false)
    }

    fn test_router(exec_calls: Arc<Mutex<u32>>) -> Router {
        use axum::extract::Query;
        use serde_json::Value;

        let calls = exec_calls.clone();
        Router::new()
            .route(
                "/action",
                post(
                    move |headers: HeaderMap, Json(body): Json<Value>| async move {
                        if !check_auth(&headers) {
                            return Err((
                                StatusCode::UNAUTHORIZED,
                                Json(serde_json::json!({"error": "unauthorized"})),
                            ));
                        }
                        *calls.lock().await += 1;
                        let action = body["action"].as_str().unwrap_or("");
                        if action == "whitelisted_command" {
                            let cmd = body["params"]["command"].as_str().unwrap_or("");
                            Ok::<_, (StatusCode, Json<Value>)>(Json(serde_json::json!({
                                "status": "ok",
                                "data": {
                                    "output": format!("executed: {cmd}"),
                                    "exit_code": 0,
                                    "duration_ms": 1,
                                }
                            })))
                        } else {
                            Ok::<_, (StatusCode, Json<Value>)>(Json(serde_json::json!({
                                "status": "ok",
                                "data": {"pong": true}
                            })))
                        }
                    },
                ),
            )
            .route(
                "/files/list",
                get(|headers: HeaderMap, Query(q): Query<Value>| async move {
                    if !check_auth(&headers) {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({"error": "unauthorized"})),
                        ));
                    }
                    Ok::<_, (StatusCode, Json<Value>)>(Json(vec![serde_json::json!({
                        "name": q["path"].as_str().unwrap_or("."),
                        "is_dir": true,
                        "size": 0,
                        "modified": "0",
                    })]))
                }),
            )
            .route(
                "/files/download",
                get(|headers: HeaderMap, Query(q): Query<Value>| async move {
                    if !check_auth(&headers) {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({"error": "unauthorized"})),
                        ));
                    }
                    let content = format!("content:{}", q["path"].as_str().unwrap_or(""));
                    Ok::<_, (StatusCode, Json<Value>)>(content)
                }),
            )
            .route(
                "/files/upload",
                post(
                    |headers: HeaderMap, Query(q): Query<Value>, body: Bytes| async move {
                        if !check_auth(&headers) {
                            return Err((
                                StatusCode::UNAUTHORIZED,
                                Json(serde_json::json!({"error": "unauthorized"})),
                            ));
                        }
                        Ok::<_, (StatusCode, Json<Value>)>(Json(serde_json::json!({
                            "message": "ok",
                            "size": body.len(),
                            "path": q["path"].as_str().unwrap_or(""),
                        })))
                    },
                ),
            )
    }

    async fn spawn_test_server(exec_calls: Arc<Mutex<u32>>) -> String {
        let app = test_router(exec_calls);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn agent_client_execute() {
        let calls = Arc::new(Mutex::new(0u32));
        let base = spawn_test_server(calls.clone()).await;
        let client = AgentClient::new();
        let result = client
            .execute(&base, "test-token", "echo hi", Some(5))
            .await
            .expect("execute should succeed");
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("executed: echo hi"));
        assert_eq!(*calls.lock().await, 1);
    }

    #[tokio::test]
    async fn agent_client_rejects_bad_token() {
        let calls = Arc::new(Mutex::new(0u32));
        let base = spawn_test_server(calls.clone()).await;
        let client = AgentClient::new();
        let err = client
            .execute(&base, "wrong-token", "echo hi", None)
            .await
            .expect_err("should fail with bad token");
        assert!(err.to_string().contains("401"), "got: {err}");
    }

    #[tokio::test]
    async fn agent_client_list_files() {
        let calls = Arc::new(Mutex::new(0u32));
        let base = spawn_test_server(calls.clone()).await;
        let client = AgentClient::new();
        let entries = client
            .list_files(&base, "test-token", "/tmp")
            .await
            .expect("list should succeed");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_dir);
    }

    #[tokio::test]
    async fn agent_client_upload_and_download() {
        let calls = Arc::new(Mutex::new(0u32));
        let base = spawn_test_server(calls.clone()).await;
        let client = AgentClient::new();
        let size = client
            .upload_file(&base, "test-token", "/tmp/a.txt", b"hello")
            .await
            .expect("upload should succeed");
        assert_eq!(size, 5);

        let content = client
            .download_file(&base, "test-token", "/tmp/a.txt")
            .await
            .expect("download should succeed");
        assert!(String::from_utf8_lossy(&content).contains("content:/tmp/a.txt"));
    }

    #[tokio::test]
    async fn agent_client_retries_transient_failure() {
        // 首次返回 500，第二次成功（重试生效）
        let call_count = Arc::new(Mutex::new(0u32));
        let count = call_count.clone();
        let app = Router::new().route(
            "/action",
            post(
                move |headers: HeaderMap, Json(_body): Json<serde_json::Value>| async move {
                    if !check_auth(&headers) {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({"error": "unauthorized"})),
                        ));
                    }
                    let mut c = count.lock().await;
                    *c += 1;
                    if *c == 1 {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": "transient"})),
                        ))
                    } else {
                        Ok::<_, (StatusCode, Json<serde_json::Value>)>(Json(serde_json::json!({
                            "status": "ok",
                            "data": {
                                "output": "ok-after-retry",
                                "exit_code": 0,
                                "duration_ms": 1,
                            }
                        })))
                    }
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let base = format!("http://{}", addr);

        let client = AgentClient::new();
        let result = client
            .execute(&base, "test-token", "echo retry", None)
            .await
            .expect("should succeed after retry");
        assert!(result.output.contains("ok-after-retry"));
        assert_eq!(*call_count.lock().await, 2);
    }
}
