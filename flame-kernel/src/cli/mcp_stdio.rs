//! `flamepanel mcp` —— MCP stdio ↔ Streamable HTTP 桥（M-AI-1 · PR-B，Doc/34 §3）。
//!
//! Claude Desktop 等客户端的本地配置只能拉起 stdio 进程。本桥逐行读取 stdin 上的
//! JSON-RPC 消息，转发到面板 `POST /api/mcp`，把响应写回 stdout（一行一条），
//! 免装 Node 版 `mcp-remote`。
//!
//! - 令牌只从环境变量 `FLAMEPANEL_MCP_TOKEN` 读取（不进 argv，`ps` 不可见）；
//! - 自动携带 `Mcp-Session-Id`（`initialize` 响应下发）与 `MCP-Protocol-Version`；
//! - 传输失败 / 非 2xx 时，对带 `id` 的请求合成 JSON-RPC 错误（`-32000`），通知静默丢弃，
//!   客户端不会因为桥吞掉响应而永远等待；
//! - stdin 关闭时 `DELETE /api/mcp` 结束会话；
//! - `--insecure` 接受自签证书（`install.sh -t` 默认自签），仅限可信网络。
//!
//! stdout 只输出协议消息；诊断信息一律写 stderr（客户端会把 stderr 记入日志）。

use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 环境变量：MCP 访问令牌（`fpk_…`）。
pub const TOKEN_ENV: &str = "FLAMEPANEL_MCP_TOKEN";
/// 单条响应上限（服务端工具输出已限 100 KiB，留足余量）。
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
/// 与服务端 30 s 工具超时对齐，再加网络余量。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeArgs {
    pub url: String,
    pub insecure: bool,
}

const USAGE: &str =
    "用法: FLAMEPANEL_MCP_TOKEN=fpk_… flamepanel mcp --url https://面板地址/api/mcp [--insecure]";

/// 解析 `mcp` 子命令参数（不含令牌：令牌只走环境变量）。
pub fn parse_args(args: &[String]) -> Result<BridgeArgs, String> {
    let mut url = None;
    let mut insecure = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--url" => url = it.next().cloned(),
            s if s.starts_with("--url=") => url = Some(s["--url=".len()..].to_string()),
            "--insecure" | "-k" => insecure = true,
            "--token" => {
                return Err(format!(
                    "出于安全考虑不接受 --token（会出现在进程列表），请设置环境变量 {TOKEN_ENV}"
                ))
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("未知参数 {other:?}\n{USAGE}")),
        }
    }
    let url = url.ok_or_else(|| USAGE.to_string())?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(format!("--url 须为 http(s) 地址：{url}"));
    }
    Ok(BridgeArgs { url, insecure })
}

/// 请求 id 列表（单条或批量；通知无 id）。
fn request_ids(msg: &Value) -> Vec<Value> {
    let one = |m: &Value| {
        m.get("method")
            .is_some()
            .then(|| m.get("id").cloned())
            .flatten()
    };
    match msg {
        Value::Array(items) => items.iter().filter_map(one).collect(),
        m => one(m).into_iter().collect(),
    }
}

/// 为每个待响应的请求合成错误；单条消息返回对象，批量返回数组，无请求返回 None。
fn error_responses(msg: &Value, code: i64, message: &str) -> Option<Value> {
    let errs: Vec<Value> = request_ids(msg)
        .into_iter()
        .map(|id| json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}))
        .collect();
    match (msg.is_array(), errs.len()) {
        (_, 0) => None,
        (false, _) => errs.into_iter().next(),
        (true, _) => Some(Value::Array(errs)),
    }
}

fn is_initialize(msg: &Value) -> bool {
    msg.get("method").and_then(Value::as_str) == Some("initialize")
}

pub struct Bridge {
    client: reqwest::Client,
    url: String,
    token: String,
    session: Option<String>,
    protocol: Option<String>,
}

impl Bridge {
    pub fn new(args: &BridgeArgs, token: String) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .danger_accept_invalid_certs(args.insecure)
            .build()
            .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))?;
        Ok(Self {
            client,
            url: args.url.clone(),
            token,
            session: None,
            protocol: None,
        })
    }

    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// 转发一行 stdin 消息；返回需要写回 stdout 的一行（None = 无需输出）。
    pub async fn forward(&mut self, line: &str) -> Option<String> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                return Some(
                    json!({"jsonrpc": "2.0", "id": null,
                           "error": {"code": -32700, "message": format!("parse error: {e}")}})
                    .to_string(),
                )
            }
        };
        match self.post(&msg).await {
            Ok(out) => out,
            Err(why) => {
                eprintln!("flamepanel mcp: {why}");
                error_responses(&msg, -32000, &why).map(|v| v.to_string())
            }
        }
    }

    async fn post(&mut self, msg: &Value) -> Result<Option<String>, String> {
        let mut req = self
            .client
            .post(&self.url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json, text/event-stream")
            .json(msg);
        if let Some(s) = &self.session {
            req = req.header("Mcp-Session-Id", s);
        }
        if let Some(p) = &self.protocol {
            req = req.header("MCP-Protocol-Version", p);
        }
        let resp = req.send().await.map_err(|e| format!("请求面板失败: {e}"))?;
        let status = resp.status();
        if let Some(s) = resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
        {
            self.session = Some(s.to_string());
        }
        if status == reqwest::StatusCode::ACCEPTED {
            return Ok(None);
        }
        if status == reqwest::StatusCode::NOT_FOUND && self.session.is_some() {
            // 会话已过期 / 被吊销：清掉，客户端重新 initialize 即可恢复
            self.session = None;
            return Err("会话已失效（HTTP 404），请让客户端重新连接".into());
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| format!("读取面板响应失败: {e}"))?;
        if body.len() > MAX_RESPONSE_BYTES {
            return Err(format!("面板响应过大（{} 字节）", body.len()));
        }
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            let brief: String = text.chars().take(200).collect();
            return Err(format!(
                "面板返回 HTTP {}: {}",
                status.as_u16(),
                brief.trim()
            ));
        }
        if body.is_empty() {
            return Ok(None);
        }
        let value: Value =
            serde_json::from_slice(&body).map_err(|e| format!("面板响应不是 JSON: {e}"))?;
        if is_initialize(msg) {
            if let Some(v) = value
                .pointer("/result/protocolVersion")
                .and_then(Value::as_str)
            {
                self.protocol = Some(v.to_string());
            }
        }
        // 统一压成单行（stdio 传输以换行分隔消息）
        Ok(Some(value.to_string()))
    }

    /// 结束会话（best-effort）。
    pub async fn close(&mut self) {
        let Some(session) = self.session.take() else {
            return;
        };
        let _ = self
            .client
            .delete(&self.url)
            .bearer_auth(&self.token)
            .header("Mcp-Session-Id", session)
            .send()
            .await;
    }
}

/// `flamepanel mcp …` 入口。
pub async fn run_cli(args: &[String]) -> std::process::ExitCode {
    let parsed = match parse_args(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::from(2);
        }
    };
    let token = match std::env::var(TOKEN_ENV) {
        Ok(t) if t.starts_with("fpk_") => t,
        Ok(_) => {
            eprintln!("{TOKEN_ENV} 格式不正确（应以 fpk_ 开头，在面板「AI 接入中心」签发）");
            return std::process::ExitCode::from(2);
        }
        Err(_) => {
            eprintln!("缺少环境变量 {TOKEN_ENV}\n{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };
    let mut bridge = match Bridge::new(&parsed, token) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                eprintln!("flamepanel mcp: 读取 stdin 失败: {e}");
                break;
            }
        };
        if let Some(out) = bridge.forward(&line).await {
            if stdout.write_all(out.as_bytes()).await.is_err()
                || stdout.write_all(b"\n").await.is_err()
                || stdout.flush().await.is_err()
            {
                break;
            }
        }
    }
    bridge.close().await;
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::post,
        Json, Router,
    };
    use std::sync::{Arc, Mutex};

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn args_require_url_and_reject_token_flag() {
        assert_eq!(
            parse_args(&s(&["--url", "https://p/api/mcp"])).unwrap(),
            BridgeArgs {
                url: "https://p/api/mcp".into(),
                insecure: false
            }
        );
        assert!(
            parse_args(&s(&["--url=http://p/api/mcp", "-k"]))
                .unwrap()
                .insecure
        );
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&s(&["--url", "ftp://x"])).is_err());
        let e = parse_args(&s(&["--url", "https://p", "--token", "fpk_x"])).unwrap_err();
        assert!(e.contains(TOKEN_ENV));
        assert!(parse_args(&s(&["--url", "https://p", "--what"])).is_err());
    }

    #[test]
    fn synthesized_errors_only_for_requests() {
        let req = json!({"jsonrpc":"2.0","id":7,"method":"tools/list"});
        let e = error_responses(&req, -32000, "down").unwrap();
        assert_eq!(e["id"], 7);
        assert_eq!(e["error"]["code"], -32000);
        let note = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(error_responses(&note, -32000, "x").is_none());
        // 响应消息（客户端回给服务端的 result）同样不需要合成
        assert!(error_responses(&json!({"jsonrpc":"2.0","id":1,"result":{}}), -1, "x").is_none());
        let batch = json!([req, note, {"jsonrpc":"2.0","id":"a","method":"ping"}]);
        let e = error_responses(&batch, -32000, "x").unwrap();
        assert_eq!(e.as_array().unwrap().len(), 2);
    }

    /// 模拟面板 MCP 端点：initialize 下发会话；通知 202；其余请求回显收到的头。
    async fn mock_server() -> (String, Arc<Mutex<Vec<String>>>) {
        let deleted = Arc::new(Mutex::new(Vec::<String>::new()));
        let d2 = deleted.clone();
        let app = Router::new().route(
            "/api/mcp",
            post(|headers: HeaderMap, Json(body): Json<Value>| async move {
                let h = |k: &str| {
                    headers
                        .get(k)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string()
                };
                if h("authorization") != "Bearer fpk_test" {
                    return (StatusCode::UNAUTHORIZED, "bad token").into_response();
                }
                if body.get("id").is_none() {
                    return StatusCode::ACCEPTED.into_response();
                }
                if body["method"] == "initialize" {
                    let mut resp = Json(json!({"jsonrpc":"2.0","id":body["id"],
                        "result":{"protocolVersion":"2025-06-18"}}))
                    .into_response();
                    resp.headers_mut()
                        .insert("mcp-session-id", "sess-1".parse().unwrap());
                    return resp;
                }
                if h("mcp-session-id") == "gone" {
                    return StatusCode::NOT_FOUND.into_response();
                }
                Json(json!({"jsonrpc":"2.0","id":body["id"],"result":{
                    "session": h("mcp-session-id"), "proto": h("mcp-protocol-version")}}))
                .into_response()
            })
            .delete(move |headers: HeaderMap| {
                let d = d2.clone();
                async move {
                    d.lock().unwrap().push(
                        headers
                            .get("mcp-session-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string(),
                    );
                    StatusCode::NO_CONTENT
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}/api/mcp"), deleted)
    }

    fn bridge(url: &str, token: &str) -> Bridge {
        Bridge::new(
            &BridgeArgs {
                url: url.into(),
                insecure: false,
            },
            token.into(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn bridge_tracks_session_and_protocol_then_closes() {
        let (url, deleted) = mock_server().await;
        let mut b = bridge(&url, "fpk_test");
        let init = b
            .forward(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .await
            .unwrap();
        assert!(!init.contains('\n'));
        assert_eq!(b.session(), Some("sess-1"));
        // 通知：202 → 无输出
        assert!(b
            .forward(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .await
            .is_none());
        // 后续请求自动带上会话与协议版本
        let out: Value = serde_json::from_str(
            &b.forward(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(out["result"]["session"], "sess-1");
        assert_eq!(out["result"]["proto"], "2025-06-18");
        // 空行忽略；非法 JSON → -32700
        assert!(b.forward("   ").await.is_none());
        let bad: Value = serde_json::from_str(&b.forward("{oops").await.unwrap()).unwrap();
        assert_eq!(bad["error"]["code"], -32700);
        b.close().await;
        assert_eq!(deleted.lock().unwrap().as_slice(), ["sess-1"]);
    }

    #[tokio::test]
    async fn bridge_turns_http_errors_into_jsonrpc_errors() {
        let (url, _) = mock_server().await;
        let mut b = bridge(&url, "fpk_wrong");
        let out: Value = serde_json::from_str(
            &b.forward(r#"{"jsonrpc":"2.0","id":9,"method":"tools/list"}"#)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(out["id"], 9);
        assert!(out["error"]["message"].as_str().unwrap().contains("401"));
        // 通知失败不输出（没有可回应的 id）
        assert!(b
            .forward(r#"{"jsonrpc":"2.0","method":"notifications/x"}"#)
            .await
            .is_none());

        // 会话失效：404 → 清会话并报错
        let mut b = bridge(&url, "fpk_test");
        b.session = Some("gone".into());
        let out = b
            .forward(r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#)
            .await
            .unwrap();
        assert!(out.contains("404"));
        assert!(b.session().is_none());

        // 连接失败
        let mut b = bridge("http://127.0.0.1:9/api/mcp", "fpk_test");
        let out = b
            .forward(r#"{"jsonrpc":"2.0","id":4,"method":"ping"}"#)
            .await
            .unwrap();
        assert!(out.contains("-32000"));
    }
}
