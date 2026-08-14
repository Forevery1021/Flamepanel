//! 面板 → Agent 远程调用客户端（Stage5 多节点能力）
//!
//! 通过 Agent 暴露的 HTTP 端点（`/exec`、`/files/list`、`/files/download`、`/files/upload`）
//! 实现远程命令执行与远程文件管理，并叠加重试 + 熔断（复用 resilience 模块）。
//! 请求携带 `Authorization: Bearer <auth_token>` 完成 Agent 侧鉴权。

use crate::core::error::AppError;
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
        }
    }

    /// 在远程节点上执行命令
    ///
    /// - `base_url`：`http://<host>:<agent_port>`
    /// - `token`：Agent 注册时下发的 `auth_token`
    /// - `command`：要执行的 shell 命令
    /// - `timeout_secs`：命令超时（None 使用客户端默认）
    pub async fn execute(
        &self,
        base_url: &str,
        token: &str,
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<RemoteExecResult, AppError> {
        let url = format!("{}/exec", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "command": command,
            "timeout_secs": timeout_secs,
        });
        let client = self.http.clone();
        let url_clone = url.clone();
        let token_owned = token.to_string();

        // 熔断 + 重试：网络抖动自动重试，连续失败熔断
        self.circuit_breaker
            .call(|| async {
                retry_with_backoff(&self.retry_config, || {
                    let client = client.clone();
                    let url = url_clone.clone();
                    let token = token_owned.clone();
                    let body = body.clone();
                    let timeout = self.timeout;
                    async move {
                        let resp = client
                            .post(&url)
                            .bearer_auth(&token)
                            .json(&body)
                            .timeout(timeout)
                            .send()
                            .await
                            .map_err(|e| format!("agent request failed: {e}"))?;
                        if !resp.status().is_success() {
                            return Err(format!("agent returned HTTP {}", resp.status()));
                        }
                        resp.json::<RemoteExecResult>()
                            .await
                            .map_err(|e| format!("agent response parse failed: {e}"))
                    }
                })
                .await
            })
            .await
    }

    /// 列出远程节点目录内容
    pub async fn list_files(
        &self,
        base_url: &str,
        token: &str,
        path: &str,
    ) -> Result<Vec<RemoteFileEntry>, AppError> {
        let url = format!("{}/files/list", base_url.trim_end_matches('/'));
        let client = self.http.clone();
        let url_clone = url.clone();
        let token_owned = token.to_string();
        let path_owned = path.to_string();

        self.circuit_breaker
            .call(|| async {
                retry_with_backoff(&self.retry_config, || {
                    let client = client.clone();
                    let url = url_clone.clone();
                    let token = token_owned.clone();
                    let path = path_owned.clone();
                    let timeout = self.timeout;
                    async move {
                        let resp = client
                            .get(&url)
                            .bearer_auth(&token)
                            .query(&[("path", path.as_str())])
                            .timeout(timeout)
                            .send()
                            .await
                            .map_err(|e| format!("agent request failed: {e}"))?;
                        if !resp.status().is_success() {
                            return Err(format!("agent returned HTTP {}", resp.status()));
                        }
                        resp.json::<Vec<RemoteFileEntry>>()
                            .await
                            .map_err(|e| format!("agent response parse failed: {e}"))
                    }
                })
                .await
            })
            .await
    }

    /// 下载远程节点文件（返回原始字节）
    pub async fn download_file(
        &self,
        base_url: &str,
        token: &str,
        path: &str,
    ) -> Result<Vec<u8>, AppError> {
        let url = format!("{}/files/download", base_url.trim_end_matches('/'));
        let client = self.http.clone();
        let url_clone = url.clone();
        let token_owned = token.to_string();
        let path_owned = path.to_string();

        self.circuit_breaker
            .call(|| async {
                retry_with_backoff(&self.retry_config, || {
                    let client = client.clone();
                    let url = url_clone.clone();
                    let token = token_owned.clone();
                    let path = path_owned.clone();
                    let timeout = self.timeout;
                    async move {
                        let resp = client
                            .get(&url)
                            .bearer_auth(&token)
                            .query(&[("path", path.as_str())])
                            .timeout(timeout)
                            .send()
                            .await
                            .map_err(|e| format!("agent request failed: {e}"))?;
                        if !resp.status().is_success() {
                            return Err(format!("agent returned HTTP {}", resp.status()));
                        }
                        resp.bytes()
                            .await
                            .map(|b| b.to_vec())
                            .map_err(|e| format!("agent response read failed: {e}"))
                    }
                })
                .await
            })
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
        let url = format!("{}/files/upload", base_url.trim_end_matches('/'));
        let client = self.http.clone();
        let url_clone = url.clone();
        let token_owned = token.to_string();
        let path_owned = path.to_string();
        let content_owned = content.to_vec();

        self.circuit_breaker
            .call(|| async {
                retry_with_backoff(&self.retry_config, || {
                    let client = client.clone();
                    let url = url_clone.clone();
                    let token = token_owned.clone();
                    let path = path_owned.clone();
                    let content = content_owned.clone();
                    let timeout = self.timeout;
                    async move {
                        let resp = client
                            .post(&url)
                            .bearer_auth(&token)
                            .query(&[("path", path.as_str())])
                            .body(content)
                            .timeout(timeout)
                            .send()
                            .await
                            .map_err(|e| format!("agent request failed: {e}"))?;
                        if !resp.status().is_success() {
                            return Err(format!("agent returned HTTP {}", resp.status()));
                        }
                        let v: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| format!("agent response parse failed: {e}"))?;
                        v.get("size")
                            .and_then(|s| s.as_u64())
                            .ok_or_else(|| "agent response missing size".to_string())
                    }
                })
                .await
            })
            .await
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
        let url = format!("{}/action", base_url.trim_end_matches('/'));
        let client = self.http.clone();
        let url_clone = url.clone();
        let token_owned = token.to_string();
        let body = serde_json::to_value(action)
            .map_err(|e| AppError::internal(format!("AgentAction serialize failed: {e}")))?;

        self.circuit_breaker
            .call(|| async {
                retry_with_backoff(&self.retry_config, || {
                    let client = client.clone();
                    let url = url_clone.clone();
                    let token = token_owned.clone();
                    let body = body.clone();
                    let timeout = self.timeout;
                    async move {
                        let resp = client
                            .post(&url)
                            .bearer_auth(&token)
                            .json(&body)
                            .timeout(timeout)
                            .send()
                            .await
                            .map_err(|e| format!("agent action request failed: {e}"))?;
                        if !resp.status().is_success() {
                            return Err(format!("agent returned HTTP {}", resp.status()));
                        }
                        resp.json::<serde_json::Value>()
                            .await
                            .map_err(|e| format!("agent action response parse failed: {e}"))
                    }
                })
                .await
            })
            .await
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

/// 便捷构造：`http://<ip>:<port>`
pub fn agent_base_url(ip: &str, port: u16) -> String {
    format!("http://{}:{}", ip, port)
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
                "/exec",
                post(move |headers: HeaderMap, Json(body): Json<Value>| async move {
                    if !check_auth(&headers) {
                        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))));
                    }
                    *calls.lock().await += 1;
                    Ok::<_, (StatusCode, Json<Value>)>(Json(serde_json::json!({
                        "output": format!("executed: {}", body["command"].as_str().unwrap_or("")),
                        "exit_code": 0,
                        "duration_ms": 1,
                    })))
                }),
            )
            .route(
                "/files/list",
                get(|headers: HeaderMap, Query(q): Query<Value>| async move {
                    if !check_auth(&headers) {
                        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))));
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
                        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))));
                    }
                    let content = format!("content:{}", q["path"].as_str().unwrap_or(""));
                    Ok::<_, (StatusCode, Json<Value>)>(content)
                }),
            )
            .route(
                "/files/upload",
                post(|headers: HeaderMap, Query(q): Query<Value>, body: Bytes| async move {
                    if !check_auth(&headers) {
                        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))));
                    }
                    Ok::<_, (StatusCode, Json<Value>)>(Json(serde_json::json!({
                        "message": "ok",
                        "size": body.len(),
                        "path": q["path"].as_str().unwrap_or(""),
                    })))
                }),
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
            "/exec",
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
                            "output": "ok-after-retry",
                            "exit_code": 0,
                            "duration_ms": 1,
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
