//! Agent Edge 反向通道端点与远程终端桥接（Doc/23 §8、Doc/30 Agent Edge）。
//!
//! - `GET /ws/agent/edge`：Agent 主动连入。**不走用户 JWT**——首帧
//!   `{"type":"hello","node_id","token"}` 以节点注册时的 `auth_token` 常量时间比对鉴权，
//!   失败统一以 4001 关闭（节点不存在与 token 错误不可区分，防探测）。
//! - `GET /ws/terminal?node_id=<id>`：浏览器远程终端。首帧走用户鉴权（`terminal:execute`），
//!   另需 `node:execute`；经 Edge 通道转发 PTY 流，浏览器侧帧协议与本机终端一致。

use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::time::MissedTickBehavior;

use super::{authenticate_first_frame, HEARTBEAT_INTERVAL, IDLE_TIMEOUT, TOKEN_RECHECK_INTERVAL};
use crate::api::types::AppState;
use crate::api::ws_auth::{close_message, CLOSE_FORBIDDEN, CLOSE_UNAUTHENTICATED};
use crate::core::error::AppError;
use crate::infrastructure::edge::{
    decode, encode, utf8_stream_decode, AgentFrame, PanelFrame, PtyEvent, EDGE_MAX_FRAME_BYTES,
};

/// Agent 首帧超时
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// Edge 连接静默上限：面板每 20 s 发 Ping，Agent 的 Pong / 任意帧都会刷新
const EDGE_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
/// 订阅对象不存在（与 /ws/metrics 同一关闭码）
const CLOSE_NOT_FOUND: u16 = 4004;
/// 单帧上限（base64 膨胀 4/3 + JSON 包装余量）
pub(super) const EDGE_MAX_MESSAGE: usize = EDGE_MAX_FRAME_BYTES / 3 * 4 + 64 * 1024;

fn audit(state: &AppState, actor: String, action: &'static str, target: String) {
    let state = state.clone();
    tokio::spawn(async move {
        let _ = state
            .operation_log_service
            .log(&actor, action, Some(&target), None)
            .await;
    });
}

/// 解析并校验 Agent 首帧；成功返回 (node_id, terminal, version)。
async fn edge_hello(socket: &mut WebSocket, state: &AppState) -> Option<(i64, bool, String)> {
    let frame = match tokio::time::timeout(HELLO_TIMEOUT, socket.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => decode(&text),
        _ => None,
    };
    let Some(AgentFrame::Hello {
        node_id,
        token,
        version,
        terminal,
    }) = frame
    else {
        return None;
    };
    match state
        .node_service
        .verify_agent_token(node_id, Some(&token))
        .await
    {
        Ok(true) => Some((node_id, terminal, version.chars().take(64).collect())),
        _ => None,
    }
}

pub(super) async fn handle_edge(mut socket: WebSocket, state: AppState) {
    let Some((node_id, terminal, version)) = edge_hello(&mut socket, &state).await else {
        // 失败稍作延迟再关闭，拖慢在线猜测 token
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = socket.send(close_message(CLOSE_UNAUTHENTICATED)).await;
        return;
    };
    let hub = state.node_service.agent_client.edge().clone();
    let mut conn = hub.attach(node_id, terminal, version.clone());
    tracing::info!(node_id, terminal, %version, "agent edge connected");
    audit(
        &state,
        format!("agent:{node_id}"),
        "EDGE_CONNECT",
        format!("nodes/{node_id}/edge"),
    );

    let welcome = encode(&PanelFrame::Welcome { node_id }).unwrap_or_default();
    if socket.send(Message::Text(welcome.into())).await.is_ok() {
        let mut ping = tokio::time::interval(HEARTBEAT_INTERVAL);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut last_seen = Instant::now();
        loop {
            tokio::select! {
                out = conn.outbound.recv() => {
                    let Some(text) = out else { break };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                _ = ping.tick() => {
                    if last_seen.elapsed() > EDGE_IDLE_TIMEOUT {
                        tracing::warn!(node_id, "agent edge idle timeout; closing");
                        break;
                    }
                    if socket.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                        break;
                    }
                }
                msg = socket.next() => {
                    last_seen = Instant::now();
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            let ok = decode(&text).map(|f| conn.dispatch(f)).unwrap_or(false);
                            if !ok {
                                tracing::warn!(node_id, "agent edge protocol error; closing");
                                break;
                            }
                        }
                        Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                        Some(Ok(Message::Binary(_))) | Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    }
                }
            }
        }
    }

    hub.detach(&conn);
    tracing::info!(node_id, "agent edge disconnected");
    audit(
        &state,
        format!("agent:{node_id}"),
        "EDGE_DISCONNECT",
        format!("nodes/{node_id}/edge"),
    );
}

/// 远程终端额外权限：`node:execute`（admin 恒放行，与 ws_auth 口径一致）。
async fn has_node_execute(state: &AppState, username: &str) -> bool {
    let Ok(Some(user)) = state.user_service.find_by_username(username).await else {
        return false;
    };
    if user.role == "admin" {
        return true;
    }
    state
        .role_service
        .check_permission(&user.role, "node", "execute")
        .await
        .unwrap_or(false)
}

/// 浏览器侧控制帧：错误说明后正常关闭（前端据 `code` 给出可操作提示，不重连）。
fn error_frame(code: &str, message: &str) -> Message {
    let v = serde_json::json!({ "type": "error", "code": code, "message": message });
    Message::Text(v.to_string().into())
}

pub(super) async fn handle_node_terminal(mut socket: WebSocket, state: AppState, node_id: i64) {
    let Some((username, session)) =
        authenticate_first_frame(&mut socket, &state, "/ws/terminal").await
    else {
        return;
    };
    if !has_node_execute(&state, &username.0).await {
        let _ = socket.send(close_message(CLOSE_FORBIDDEN)).await;
        return;
    }
    if state.node_service.get_node(node_id).await.is_err() {
        let _ = socket.send(close_message(CLOSE_NOT_FOUND)).await;
        return;
    }
    let hub = state.node_service.agent_client.edge().clone();
    let (sid, mut events) = match hub.open_pty(node_id, 80, 24).await {
        Ok(v) => v,
        Err(e) => {
            let code = match &e {
                AppError::Forbidden(_) => "TERMINAL_DISABLED",
                AppError::RateLimited(_) => "TOO_MANY_TERMINALS",
                _ => "EDGE_OFFLINE",
            };
            let _ = socket.send(error_frame(code, &e.to_string())).await;
            let _ = socket.close().await;
            return;
        }
    };
    let target = format!("nodes/{node_id}/terminal/{sid}");
    audit(
        &state,
        username.0.clone(),
        "OPEN_REMOTE_TERMINAL",
        target.clone(),
    );

    let mut ping = tokio::time::interval(HEARTBEAT_INTERVAL);
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut token_check = tokio::time::interval(TOKEN_RECHECK_INTERVAL);
    token_check.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut kill_rx = session.kill_rx();
    let mut carry = Vec::new();
    let mut last_input = Instant::now();

    loop {
        tokio::select! {
            ev = events.recv() => {
                let msg = match ev {
                    Some(PtyEvent::Output(bytes)) => {
                        let text = utf8_stream_decode(&mut carry, &bytes);
                        if text.is_empty() { continue; }
                        serde_json::json!({ "type": "output", "data": text })
                    }
                    Some(PtyEvent::Exit(code)) => {
                        let _ = socket.send(Message::Text(serde_json::json!({ "type": "exit", "code": code }).to_string().into())).await;
                        break;
                    }
                    Some(PtyEvent::Error(m)) => {
                        let _ = socket.send(error_frame("EDGE_OFFLINE", &m)).await;
                        break;
                    }
                    None => {
                        let _ = socket.send(error_frame("EDGE_OFFLINE", "edge connection closed")).await;
                        break;
                    }
                };
                if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                    break;
                }
            }
            _ = ping.tick() => {
                if last_input.elapsed() > IDLE_TIMEOUT {
                    break;
                }
                if socket.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                    break;
                }
            }
            _ = token_check.tick() => {
                if let Some(code) = session.recheck(&state).await.close_code() {
                    let _ = socket.send(close_message(code)).await;
                    break;
                }
                // 权限被收回（node:execute）同样终止
                if !has_node_execute(&state, &username.0).await {
                    let _ = socket.send(close_message(CLOSE_FORBIDDEN)).await;
                    break;
                }
            }
            r = kill_rx.changed() => {
                let code = *kill_rx.borrow();
                if r.is_err() || code != 0 {
                    let _ = socket.send(close_message(code.max(CLOSE_UNAUTHENTICATED))).await;
                    break;
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if text.len() > 256 * 1024 {
                            break;
                        }
                        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
                        match v["type"].as_str().unwrap_or("") {
                            "input" => {
                                if let Some(data) = v["data"].as_str() {
                                    last_input = Instant::now();
                                    if hub.pty_input(node_id, &sid, data.as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            "resize" => {
                                let cols = v["cols"].as_u64().unwrap_or(80).min(1000) as u16;
                                let rows = v["rows"].as_u64().unwrap_or(24).min(1000) as u16;
                                let _ = hub.pty_resize(node_id, &sid, cols, rows).await;
                            }
                            "auth" => {
                                if let Some(token) = v["token"].as_str() {
                                    session.kill(session.reauth(&state, token).await);
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }

    hub.pty_close(node_id, &sid).await;
    audit(&state, username.0, "CLOSE_REMOTE_TERMINAL", target);
}
