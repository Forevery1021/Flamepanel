//! Agent Edge 出站反向通道（Doc/23 §8）。
//!
//! Agent 主动以 WebSocket 连到面板 `/ws/agent/edge`（`PANEL_URL` 的 http→ws / https→wss），
//! 首帧 `hello` 携带节点 ID 与 `AUTH_TOKEN`。之后：
//!
//! - `req` 帧在**进程内**回放到本机 axum 路由（与直连模式同一套 `/action` 白名单、文件根目录
//!   约束与 token 校验），结果以 `resp` 帧返回；
//! - `pty_*` 帧驱动远程终端（需 `AGENT_TERMINAL=1`，否则回 `pty_error`）。
//!
//! 断线后指数退避重连（1 s → 60 s），终端随连接一起关闭。

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::Router;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

use crate::pty::{PtyManager, PtyOutput};

/// 单帧上限（与面板 `EDGE_MAX_FRAME_BYTES` 48 MiB 的 base64 膨胀对齐）
const MAX_MESSAGE: usize = 48 * 1024 * 1024 / 3 * 4 + 64 * 1024;
/// 回放响应 body 上限（解码前）
const MAX_BODY: usize = 48 * 1024 * 1024;
const WELCOME_TIMEOUT: Duration = Duration::from_secs(10);
/// 面板每 20 s Ping 一次；超过该时长无任何入站帧视为半开连接，主动重连
const READ_IDLE: Duration = Duration::from_secs(75);

/// 面板 → Agent 帧（与 `flame-kernel/src/infrastructure/edge.rs::PanelFrame` 对应）
#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum PanelFrame {
    Welcome {
        node_id: i64,
    },
    Req {
        id: u64,
        method: String,
        path: String,
        #[serde(default)]
        content_type: Option<String>,
        #[serde(default)]
        body: String,
    },
    PtyOpen {
        sid: String,
        cols: u16,
        rows: u16,
    },
    PtyIn {
        sid: String,
        data: String,
    },
    PtyResize {
        sid: String,
        cols: u16,
        rows: u16,
    },
    PtyClose {
        sid: String,
    },
}

/// Agent → 面板帧
#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum AgentFrame {
    Hello {
        node_id: i64,
        token: String,
        version: String,
        terminal: bool,
    },
    Resp {
        id: u64,
        status: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        content_type: Option<String>,
        body: String,
    },
    PtyOut {
        sid: String,
        data: String,
    },
    PtyExit {
        sid: String,
        code: Option<i32>,
    },
    PtyError {
        sid: String,
        message: String,
    },
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// `PANEL_URL` → Edge 端点：http→ws、https→wss，去掉末尾 `/`。其他 scheme 返回 None。
pub(crate) fn edge_url(panel_url: &str) -> Option<String> {
    let p = panel_url.trim().trim_end_matches('/');
    let rest = match p.strip_prefix("https://") {
        Some(r) => format!("wss://{r}"),
        None => format!("ws://{}", p.strip_prefix("http://")?),
    };
    Some(format!("{rest}/ws/agent/edge"))
}

/// 重连退避：1, 2, 4 … 封顶 60 秒
pub(crate) fn backoff_secs(attempt: u32) -> u64 {
    1u64.checked_shl(attempt.min(6)).unwrap_or(64).min(60)
}

/// Edge 会话参数
#[derive(Clone)]
pub(crate) struct EdgeSettings {
    pub(crate) url: String,
    pub(crate) node_id: i64,
    pub(crate) token: String,
    pub(crate) terminal: bool,
    pub(crate) file_root: PathBuf,
}

/// 在进程内回放一次请求：注入本机 token 与回环 ConnectInfo（限流中间件需要），交给 `router`。
pub(crate) async fn replay(
    router: Router,
    token: &str,
    id: u64,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    body_b64: &str,
) -> AgentFrame {
    let fail = |status: u16, msg: &str| AgentFrame::Resp {
        id,
        status,
        content_type: Some("application/json".into()),
        body: b64().encode(serde_json::json!({ "error": msg }).to_string()),
    };
    let method = match method {
        "GET" => axum::http::Method::GET,
        "POST" => axum::http::Method::POST,
        _ => return fail(405, "method not allowed over edge"),
    };
    if !path.starts_with('/') {
        return fail(400, "path must be absolute");
    }
    let Ok(body) = b64().decode(body_b64.as_bytes()) else {
        return fail(400, "invalid base64 body");
    };
    let mut builder = axum::http::Request::builder()
        .method(method)
        .uri(path)
        .header("Authorization", format!("Bearer {token}"));
    if let Some(ct) = content_type {
        builder = builder.header("Content-Type", ct);
    }
    let Ok(mut req) = builder.body(axum::body::Body::from(body)) else {
        return fail(400, "invalid request");
    };
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let Ok(resp) = router.oneshot(req).await;
    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    match axum::body::to_bytes(resp.into_body(), MAX_BODY).await {
        Ok(bytes) => AgentFrame::Resp {
            id,
            status,
            content_type: ct,
            body: b64().encode(bytes),
        },
        Err(_) => fail(413, "response exceeds edge frame limit"),
    }
}

fn tls_connector() -> Result<tokio_tungstenite::Connector, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    // 工作区内 rustls 同时启用了 ring 与 aws-lc-rs，必须显式指定 provider，否则 builder() 会 panic
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| format!("tls config: {e}"))?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(tokio_tungstenite::Connector::Rustls(Arc::new(config)))
}

fn encode(frame: &AgentFrame) -> Option<Message> {
    serde_json::to_string(frame)
        .ok()
        .map(|s| Message::Text(s.into()))
}

/// 一次完整的 Edge 会话：连接 → hello → 处理帧直到断开。返回断开原因。
pub(crate) async fn run_session(settings: &EdgeSettings, router: Router) -> Result<(), String> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let connector = if settings.url.starts_with("wss://") {
        Some(tls_connector()?)
    } else {
        None
    };
    let (ws, _) = tokio_tungstenite::connect_async_tls_with_config(
        settings.url.as_str(),
        Some(config),
        true,
        connector,
    )
    .await
    .map_err(|e| format!("connect: {e}"))?;
    let (mut sink, mut stream) = ws.split();

    let hello = AgentFrame::Hello {
        node_id: settings.node_id,
        token: settings.token.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        terminal: settings.terminal,
    };
    sink.send(encode(&hello).ok_or("encode hello")?)
        .await
        .map_err(|e| format!("send hello: {e}"))?;
    match tokio::time::timeout(WELCOME_TIMEOUT, stream.next()).await {
        Ok(Some(Ok(Message::Text(t)))) => match serde_json::from_str::<PanelFrame>(&t) {
            Ok(PanelFrame::Welcome { .. }) => {}
            _ => return Err("unexpected first frame from panel".into()),
        },
        Ok(Some(Ok(Message::Close(f)))) => {
            return Err(format!(
                "panel rejected edge hello (close {:?}) — check NODE_ID / AUTH_TOKEN",
                f.map(|f| u16::from(f.code))
            ))
        }
        _ => return Err("no welcome from panel".into()),
    }
    eprintln!("[agent] Edge 通道已建立: {}", settings.url);

    // 单一写端：回放结果、PTY 输出都经此队列
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(256);
    let writer = tokio::spawn(async move {
        while let Some(m) = out_rx.recv().await {
            if sink.send(m).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    let ptys = PtyManager::default();
    let (pty_tx, mut pty_rx) = mpsc::channel::<PtyOutput>(1024);
    let pty_out_tx = out_tx.clone();
    let pty_pump = tokio::spawn(async move {
        while let Some(ev) = pty_rx.recv().await {
            let frame = match ev {
                PtyOutput::Data(sid, bytes) => AgentFrame::PtyOut {
                    sid,
                    data: b64().encode(bytes),
                },
                PtyOutput::Exit(sid, code) => AgentFrame::PtyExit { sid, code },
            };
            if let Some(m) = encode(&frame) {
                if pty_out_tx.send(m).await.is_err() {
                    break;
                }
            }
        }
    });

    let result = loop {
        let msg = match tokio::time::timeout(READ_IDLE, stream.next()).await {
            Err(_) => break Err("edge read idle timeout".to_string()),
            Ok(None) => break Err("panel closed edge connection".to_string()),
            Ok(Some(Err(e))) => break Err(format!("edge read: {e}")),
            Ok(Some(Ok(m))) => m,
        };
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break Ok(()),
            // Ping 由 tungstenite 自动回 Pong
            _ => continue,
        };
        let Ok(frame) = serde_json::from_str::<PanelFrame>(&text) else {
            break Err("protocol error: malformed panel frame".into());
        };
        handle_frame(frame, settings, &router, &ptys, &pty_tx, &out_tx).await;
    };

    ptys.close_all();
    drop(out_tx);
    pty_pump.abort();
    let _ = writer.await;
    result
}

async fn handle_frame(
    frame: PanelFrame,
    settings: &EdgeSettings,
    router: &Router,
    ptys: &PtyManager,
    pty_tx: &mpsc::Sender<PtyOutput>,
    out_tx: &mpsc::Sender<Message>,
) {
    let pty_error = |sid: String, message: String| encode(&AgentFrame::PtyError { sid, message });
    match frame {
        PanelFrame::Welcome { .. } => {}
        PanelFrame::Req {
            id,
            method,
            path,
            content_type,
            body,
        } => {
            // 每个请求独立任务：慢请求（大文件 / 长命令）不阻塞其他请求与终端
            let router = router.clone();
            let token = settings.token.clone();
            let tx = out_tx.clone();
            tokio::spawn(async move {
                let resp = replay(
                    router,
                    &token,
                    id,
                    &method,
                    &path,
                    content_type.as_deref(),
                    &body,
                )
                .await;
                if let Some(m) = encode(&resp) {
                    let _ = tx.send(m).await;
                }
            });
        }
        PanelFrame::PtyOpen { sid, cols, rows } => {
            let res = if settings.terminal {
                ptys.open(&sid, cols, rows, &settings.file_root, pty_tx.clone())
            } else {
                Err("remote terminal is disabled on this agent (AGENT_TERMINAL=1 to enable)".into())
            };
            if let Err(e) = res {
                if let Some(m) = pty_error(sid, e) {
                    let _ = out_tx.send(m).await;
                }
            }
        }
        PanelFrame::PtyIn { sid, data } => {
            let res = b64()
                .decode(data.as_bytes())
                .map_err(|_| "invalid base64".to_string())
                .and_then(|bytes| ptys.write(&sid, &bytes));
            if let Err(e) = res {
                if let Some(m) = pty_error(sid, e) {
                    let _ = out_tx.send(m).await;
                }
            }
        }
        PanelFrame::PtyResize { sid, cols, rows } => {
            let _ = ptys.resize(&sid, cols, rows);
        }
        PanelFrame::PtyClose { sid } => ptys.close(&sid),
    }
}

/// 常驻循环：会话结束后按退避重连。
pub(crate) async fn run_forever(settings: EdgeSettings, router: Router) {
    let mut attempt = 0u32;
    loop {
        let started = tokio::time::Instant::now();
        match run_session(&settings, router.clone()).await {
            Ok(()) => eprintln!("[agent] Edge 通道已关闭，准备重连"),
            Err(e) => eprintln!("[agent] Edge 通道断开: {e}"),
        }
        // 稳定运行过一段时间的连接断开后从头退避
        if started.elapsed() > Duration::from_secs(120) {
            attempt = 0;
        }
        let wait = backoff_secs(attempt);
        attempt = attempt.saturating_add(1);
        tokio::time::sleep(Duration::from_secs(wait)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::{get, post};

    #[test]
    fn edge_url_maps_scheme() {
        assert_eq!(
            edge_url("https://panel.example.com/").as_deref(),
            Some("wss://panel.example.com/ws/agent/edge")
        );
        assert_eq!(
            edge_url("http://10.0.0.1:8080").as_deref(),
            Some("ws://10.0.0.1:8080/ws/agent/edge")
        );
        assert_eq!(edge_url("ftp://x"), None);
    }

    #[test]
    fn backoff_caps_at_60() {
        let v: Vec<u64> = (0..9).map(backoff_secs).collect();
        assert_eq!(v, vec![1, 2, 4, 8, 16, 32, 60, 60, 60]);
    }

    #[test]
    fn panel_frames_decode() {
        let f: PanelFrame = serde_json::from_str(
            r#"{"type":"req","id":3,"method":"GET","path":"/files/list?path=.","body":""}"#,
        )
        .unwrap();
        assert!(matches!(f, PanelFrame::Req { id: 3, .. }));
        let f: PanelFrame =
            serde_json::from_str(r#"{"type":"pty_open","sid":"1-2","cols":80,"rows":24}"#).unwrap();
        assert_eq!(
            f,
            PanelFrame::PtyOpen {
                sid: "1-2".into(),
                cols: 80,
                rows: 24
            }
        );
        let v = serde_json::to_value(AgentFrame::PtyExit {
            sid: "s".into(),
            code: Some(0),
        })
        .unwrap();
        assert_eq!(v["type"], "pty_exit");
    }

    fn test_router() -> Router {
        Router::new()
            .route(
                "/echo",
                post(
                    |headers: axum::http::HeaderMap, body: axum::body::Bytes| async move {
                        let auth = headers
                            .get("Authorization")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string();
                        axum::Json(serde_json::json!({
                            "auth": auth,
                            "body": String::from_utf8_lossy(&body),
                        }))
                    },
                ),
            )
            .route(
                "/peer",
                get(|ConnectInfo(p): ConnectInfo<SocketAddr>| async move { p.ip().to_string() }),
            )
    }

    fn resp_body(f: &AgentFrame) -> (u16, String) {
        let AgentFrame::Resp { status, body, .. } = f else {
            panic!("not resp")
        };
        (
            *status,
            String::from_utf8(b64().decode(body).unwrap()).unwrap(),
        )
    }

    #[tokio::test]
    async fn replay_injects_bearer_token_and_loopback_peer() {
        let r = replay(
            test_router(),
            "tok",
            1,
            "POST",
            "/echo",
            Some("text/plain"),
            &b64().encode("hi"),
        )
        .await;
        let (status, body) = resp_body(&r);
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["auth"], "Bearer tok");
        assert_eq!(v["body"], "hi");

        let r = replay(test_router(), "tok", 2, "GET", "/peer", None, "").await;
        assert_eq!(resp_body(&r), (200, "127.0.0.1".into()));
    }

    #[tokio::test]
    async fn replay_rejects_bad_input() {
        let r = replay(test_router(), "t", 1, "DELETE", "/echo", None, "").await;
        assert_eq!(resp_body(&r).0, 405);
        let r = replay(test_router(), "t", 1, "GET", "http://evil/x", None, "").await;
        assert_eq!(resp_body(&r).0, 400);
        let r = replay(test_router(), "t", 1, "POST", "/echo", None, "!!").await;
        assert_eq!(resp_body(&r).0, 400);
        let r = replay(test_router(), "t", 1, "GET", "/missing", None, "").await;
        assert_eq!(resp_body(&r).0, 404);
    }

    /// 真实 Agent 路由经回放：`/action` 鉴权使用注入的 Bearer token（修复前恒 401）
    #[tokio::test]
    async fn replay_through_real_agent_routes_authenticates() {
        let _ = crate::server::AUTH_TOKEN_CELL.set("edge-test-token".into());
        let r = replay(
            crate::server::agent_routes(),
            "edge-test-token",
            9,
            "POST",
            "/action",
            Some("application/json"),
            &b64().encode(r#"{"action":"ping","params":{}}"#),
        )
        .await;
        let (status, body) = resp_body(&r);
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("ok"), "{body}");
    }
}
