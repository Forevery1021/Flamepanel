use axum::extract::ws::Utf8Bytes;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::MissedTickBehavior;

use crate::api::types::{AppState, Username};
use crate::api::ws_auth::{
    authenticate_ws_user, close_message, extract_auth_token, WsSession, CLOSE_UNAUTHENTICATED,
};
use crate::api::ws_backpressure::{ws_stats, LagAction, LagPolicy, RESYNC_FRAME};

mod edge;

/// F-07: 心跳间隔
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
/// F-07: 空闲超时（无任何帧则关闭连接）
const IDLE_TIMEOUT: Duration = Duration::from_secs(600);
/// F-07: 心跳失败重试次数上限
const MAX_HEARTBEAT_FAILURES: u32 = 3;
/// F-23: 长连接期间 access token 复核周期
const TOKEN_RECHECK_INTERVAL: Duration = Duration::from_secs(60);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ws/metrics", get(metrics_handler))
        .route("/ws/logs", get(logs_handler))
        .route("/ws/terminal", get(terminal_handler))
        // Doc/30 Agent Edge：Agent 出站反向通道（节点 token 鉴权，不走用户 JWT）
        .route("/ws/agent/edge", get(agent_edge_handler))
}

async fn agent_edge_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.max_message_size(edge::EDGE_MAX_MESSAGE)
        .max_frame_size(edge::EDGE_MAX_MESSAGE)
        .on_upgrade(move |socket| edge::handle_edge(socket, state))
}

/// `/ws/terminal` 查询参数：缺省为面板本机终端；`node_id` 经 Agent Edge 打开远程终端。
#[derive(Debug, Default, serde::Deserialize)]
struct TerminalQuery {
    node_id: Option<i64>,
}

/// R2：读取 WS 首帧并完成鉴权（JWT + RBAC）。
/// 首帧约定为 `{"type":"auth","token":"<access_token>"}`。
/// 鉴权成功返回 Username 与会话；失败时发送带关闭码的 Close 帧（4001 未认证 / 4003 无权限）
/// 并返回 None（调用方直接结束，不产生任何数据）。
async fn authenticate_first_frame(
    socket: &mut WebSocket,
    state: &AppState,
    path: &'static str,
) -> Option<(Username, Arc<WsSession>)> {
    // 首帧必须在超时内到达，防未认证僵尸连接长期占用。
    let token = match tokio::time::timeout(Duration::from_secs(10), socket.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => extract_auth_token(&text),
        _ => None,
    };
    let result = match token {
        Some(t) => authenticate_ws_user(state, &t, path)
            .await
            .map(|(uid, user)| (user, WsSession::new(path, uid, t))),
        None => Err(crate::core::error::AppError::Unauthorized(
            "missing auth frame".into(),
        )),
    };
    match result {
        Ok(v) => Some(v),
        Err(e) => {
            let code = match e {
                crate::core::error::AppError::Forbidden(_) => crate::api::ws_auth::CLOSE_FORBIDDEN,
                _ => CLOSE_UNAUTHENTICATED,
            };
            let _ = socket.send(close_message(code)).await;
            None
        }
    }
}

/// `/ws/metrics` 查询参数：缺省为本机；`node_id` 订阅该节点（Agent 心跳）的指标流。
#[derive(Debug, Default, serde::Deserialize)]
struct MetricsQuery {
    node_id: Option<i64>,
}

/// 订阅的节点不存在（鉴权通过之后才判定，未认证连接无法借此探测节点 ID）。
const CLOSE_NOT_FOUND: u16 = 4004;

async fn metrics_handler(
    // 先解析查询：非法 node_id 直接 400，不进入升级
    axum::extract::Query(query): axum::extract::Query<MetricsQuery>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let node_id = query.node_id;
    ws.on_upgrade(move |socket| handle_metrics(socket, state, node_id))
}

async fn handle_metrics(mut socket: WebSocket, state: AppState, node_id: Option<i64>) {
    // R2：首帧鉴权，失败直接关闭。
    let Some((_username, session)) =
        authenticate_first_frame(&mut socket, &state, "/ws/metrics").await
    else {
        return;
    };

    // Doc/30 M-P：节点指标与本机同构（同一 MetricsHub 模型、同一 init / tick 帧协议）
    let hub = match node_id {
        None => state.metrics.clone(),
        Some(id) => {
            if state.node_service.get_node(id).await.is_err() {
                let _ = socket.send(close_message(CLOSE_NOT_FOUND)).await;
                return;
            }
            state.metrics.nodes().hub_or_create(id)
        }
    };

    // P-01：init 帧来自 Hub 缓存（同一历史窗口内 N 个连接共享一次序列化结果）
    if let Some(init) = hub.init_payload() {
        if socket
            .send(Message::Text(arc_str_to_utf8(&init)))
            .await
            .is_err()
        {
            return;
        }
    }

    let rx = hub.subscribe();
    // 不在会话期间持有 Hub：节点被删除时注册表释放最后一个引用 → 广播关闭 → 会话随之结束
    drop(hub);
    run_push_session(
        socket,
        state,
        session,
        "metrics",
        rx,
        // tick 帧已由 Hub 预序列化：这里只做 Arc<str> → Utf8Bytes 的零拷贝转换
        |frame: Arc<crate::infrastructure::metrics::MetricsFrame>| {
            Some(arc_str_to_utf8(&frame.json))
        },
    )
    .await;
}

/// `Arc<str>` → `Utf8Bytes`：`Utf8Bytes` 内部是 `Bytes`，`From<String>` 需要拷贝一次；
/// 单帧 ~300 B、单连接一次，成本可忽略，但相比旧实现已省去每连接的 `json!` 构树 + 序列化。
fn arc_str_to_utf8(s: &Arc<str>) -> Utf8Bytes {
    Utf8Bytes::from(&**s)
}

/// 单向推送会话（服务端 → 客户端）的通用骨架：**心跳 + 周期 token 复核 + 空闲超时 +
/// broadcast 背压治理**，metrics / logs 两条流共用，新增一条推送流只需提供
/// `broadcast::Receiver<T>` 与 `T → 文本帧` 的映射（返回 `None` 表示丢弃该条）。
///
/// 背压（S-08）：偶发 `Lagged` 记数后继续；**连续** 3 次判定为慢消费者——发一帧
/// `{type:"resync"}` 后主动断开，客户端重连拿到完整 `init` 历史窗口即可补齐曲线。
/// 计数经 `/metrics` 暴露（`flamepanel_ws_lag_events_total{stream}` 等）。
///
/// 重构前三处 handler 各自复制了整段 select! 循环（~80 行 × 3），任何心跳 / 鉴权策略调整
/// 都要改三处；现在集中在此。终端会话是双向的（有输入路由与 PTY 生命周期），单独保留。
async fn run_push_session<T, F>(
    socket: WebSocket,
    state: AppState,
    session: Arc<WsSession>,
    stream_name: &'static str,
    mut rx: broadcast::Receiver<T>,
    mut encode: F,
) where
    T: Clone + Send + 'static,
    F: FnMut(T) -> Option<Utf8Bytes> + Send + 'static,
{
    let (ws_sender, ws_receiver) = socket.split();
    let recv_state = state.clone();
    let recv_session = session.clone();

    let mut send_task = tokio::spawn(async move {
        let mut ws_sender: SplitSink<WebSocket, Message> = ws_sender;
        let mut kill_rx = session.kill_rx();
        // F-07: 带心跳的发送循环
        let mut ping = tokio::time::interval(HEARTBEAT_INTERVAL);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // F-23: 周期校验 token 有效性（每分钟），失效则关闭连接
        let mut token_check = tokio::time::interval(TOKEN_RECHECK_INTERVAL);
        token_check.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut heartbeat_failures = 0u32;
        let mut lag_policy = LagPolicy::default();
        let stats = ws_stats().stream(stream_name);

        loop {
            tokio::select! {
                _ = ping.tick() => {
                    if ws_sender.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                        heartbeat_failures += 1;
                        if heartbeat_failures >= MAX_HEARTBEAT_FAILURES {
                            tracing::debug!(stream = stream_name, "WS heartbeat failed {heartbeat_failures} times; closing");
                            break;
                        }
                    } else {
                        heartbeat_failures = 0;
                    }
                }
                _ = token_check.tick() => {
                    // F-23: token 失效（过期/吊销/改密）→ 4001；通道权限被收回 → 4003
                    if let Some(code) = session.recheck(&state).await.close_code() {
                        tracing::debug!(stream = stream_name, code, "WS session invalid; closing connection");
                        let _ = ws_sender.send(close_message(code)).await;
                        break;
                    }
                }
                r = kill_rx.changed() => {
                    // 接收端带内续期失败
                    let code = *kill_rx.borrow();
                    if r.is_err() || code != 0 {
                        let _ = ws_sender.send(close_message(code.max(CLOSE_UNAUTHENTICATED))).await;
                        break;
                    }
                }
                r = rx.recv() => {
                    match r {
                        Ok(item) => {
                            if let Some(payload) = encode(item) {
                                if ws_sender.send(Message::Text(payload)).await.is_err() {
                                    break;
                                }
                                heartbeat_failures = 0;
                                lag_policy.on_delivered();
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            stats.record_lag(n);
                            match lag_policy.on_lag() {
                                LagAction::Continue => {
                                    tracing::debug!(stream = stream_name, dropped = n, consecutive = lag_policy.consecutive(), "ws consumer lagged; continuing");
                                }
                                LagAction::Resync => {
                                    // 连续滞后：通知客户端重同步并断开，避免长期慢消费者反复丢帧
                                    stats.record_disconnect();
                                    tracing::warn!(stream = stream_name, dropped = n, consecutive = lag_policy.consecutive(), "ws consumer keeps lagging; sending resync and closing");
                                    let _ = ws_sender.send(Message::Text(Utf8Bytes::from(RESYNC_FRAME))).await;
                                    let _ = ws_sender.send(Message::Close(None)).await;
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(idle_guarded_drain(
        ws_receiver,
        stream_name,
        recv_state,
        recv_session,
    ));

    // F-07: 先完成者 abort 另一任务，防止僵尸连接泄漏
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
            std::mem::drop(recv_task);
        }
        _ = &mut recv_task => {
            send_task.abort();
            std::mem::drop(send_task);
        }
    }
}

/// 接收端：消费 Close 帧、带内续期帧与空闲超时（其余帧忽略，但会刷新空闲计时）。
async fn idle_guarded_drain(
    mut ws_receiver: futures_util::stream::SplitStream<WebSocket>,
    stream_name: &'static str,
    state: AppState,
    session: Arc<WsSession>,
) {
    loop {
        tokio::select! {
            r = ws_receiver.next() => {
                match r {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Text(text))) => {
                        if let Some(token) = extract_auth_token(&text) {
                            // 续期失败交给发送端发 Close 帧（发送端只在一个任务里）
                            session.kill(session.reauth(&state, &token).await);
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            _ = tokio::time::sleep(IDLE_TIMEOUT) => {
                tracing::debug!(stream = stream_name, "WS idle timeout; closing");
                break;
            }
        }
    }
}

async fn terminal_handler(
    axum::extract::Query(query): axum::extract::Query<TerminalQuery>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    match query.node_id {
        Some(node_id) => {
            ws.on_upgrade(move |socket| edge::handle_node_terminal(socket, state, node_id))
        }
        None => ws.on_upgrade(move |socket| handle_terminal(socket, state)),
    }
}

/// 终端会话审计：记录谁在何时打开/关闭了 Web 终端（P1：终端属任意命令执行面，须可追溯）。
fn audit_terminal(state: &AppState, username: &str, action: &str, session_id: u64) {
    let state = state.clone();
    let username = username.to_string();
    let action = action.to_string();
    tokio::spawn(async move {
        let _ = state
            .operation_log_service
            .log(
                &username,
                &action,
                Some(&format!("terminal/session/{}", session_id)),
                None,
            )
            .await;
    });
}

async fn handle_terminal(mut socket: WebSocket, state: AppState) {
    // R2：首帧鉴权，失败直接关闭（不创建终端会话，避免未认证占用资源）。
    let Some((username, session)) =
        authenticate_first_frame(&mut socket, &state, "/ws/terminal").await
    else {
        return;
    };

    let (id, mut rx) = match state.terminal_manager.create_session().await {
        Ok(v) => v,
        // T14：终端会话创建失败（如 shell 缺失）时优雅关闭而非 panic。
        Err(e) => {
            tracing::error!(error = %e, "terminal session create failed");
            let _ = socket.close().await;
            return;
        }
    };

    audit_terminal(&state, &username.0, "OPEN_WS_TERMINAL", id);

    let (ws_sender, ws_receiver) = socket.split();

    let state_for_token = state.clone();
    let session_for_check = session.clone();
    let mut send_task = tokio::spawn(async move {
        // F-07: 终端发送端也带心跳 + output 推送
        let mut ws_sender = ws_sender;
        let mut ping = tokio::time::interval(HEARTBEAT_INTERVAL);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // F-23: 周期校验 token 有效性
        let mut token_check = tokio::time::interval(TOKEN_RECHECK_INTERVAL);
        token_check.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut heartbeat_failures = 0u32;
        let mut kill_rx = session_for_check.kill_rx();

        loop {
            tokio::select! {
                _ = ping.tick() => {
                    if ws_sender.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                        heartbeat_failures += 1;
                        if heartbeat_failures >= MAX_HEARTBEAT_FAILURES {
                            break;
                        }
                    } else {
                        heartbeat_failures = 0;
                    }
                }
                _ = token_check.tick() => {
                    if let Some(code) = session_for_check.recheck(&state_for_token).await.close_code() {
                        tracing::debug!(code, "terminal WS session invalid; closing");
                        let _ = ws_sender.send(close_message(code)).await;
                        break;
                    }
                }
                r = kill_rx.changed() => {
                    let code = *kill_rx.borrow();
                    if r.is_err() || code != 0 {
                        let _ = ws_sender.send(close_message(code.max(CLOSE_UNAUTHENTICATED))).await;
                        break;
                    }
                }
                output = rx.recv() => {
                    match output {
                        Some(output) => {
                            if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                                "type": "output",
                                "data": output,
                            })) {
                                if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                                    break;
                                }
                                heartbeat_failures = 0;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        // F-07: 终端接收端带空闲超时
        let mut ws_receiver = ws_receiver;
        loop {
            tokio::select! {
                r = ws_receiver.next() => {
                    match r {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Text(text) => {
                                    // T5：限制 WS 消息大小，防止超大帧耗尽内存。
                                    if text.len() > 256 * 1024 {
                                        let _ = state_clone.terminal_manager.close(id).await;
                                        break;
                                    }
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                                        let msg_type = parsed["type"].as_str().unwrap_or("");
                                        match msg_type {
                                            "input" => {
                                                if let Some(data) = parsed["data"].as_str() {
                                                    let _ = state_clone.terminal_manager.write(id, data).await;
                                                }
                                            }
                                            "auth" => {
                                                // 带内续期：access token 轮换后终端不断开
                                                if let Some(token) = parsed["token"].as_str() {
                                                    session.kill(session.reauth(&state_clone, token).await);
                                                }
                                            }
                                            "resize" => {
                                                let cols = parsed["cols"].as_u64().unwrap_or(80) as u16;
                                                let rows = parsed["rows"].as_u64().unwrap_or(24) as u16;
                                                let _ = state_clone.terminal_manager.resize(id, cols, rows).await;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Message::Close(_) => break,
                                _ => {}
                            }
                        }
                        Some(Err(_)) | None => break,
                    }
                }
                _ = tokio::time::sleep(IDLE_TIMEOUT) => {
                    tracing::debug!("terminal WS idle timeout; closing");
                    break;
                }
            }
        }
    });

    // F-07: 先完成者 abort 另一任务
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
            std::mem::drop(recv_task);
        }
        _ = &mut recv_task => {
            send_task.abort();
            std::mem::drop(send_task);
        }
    }

    // T5：统一清理路径——无论 send_task 还是 recv_task 先结束，都在此关闭会话，避免终端会话泄漏。
    state.terminal_manager.close(id).await;
    audit_terminal(&state, &username.0, "CLOSE_WS_TERMINAL", id);
}

async fn logs_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs(socket, state))
}

async fn handle_logs(mut socket: WebSocket, state: AppState) {
    // R2：首帧鉴权，失败直接关闭。
    let Some((_username, session)) =
        authenticate_first_frame(&mut socket, &state, "/ws/logs").await
    else {
        return;
    };

    let recent_logs = state.log_service.list().await.unwrap_or_default();
    if !recent_logs.is_empty() {
        if let Ok(payload) = serde_json::to_string(&serde_json::json!({
            "type": "init",
            "data": recent_logs,
        })) {
            if socket.send(Message::Text(payload.into())).await.is_err() {
                return;
            }
        }
    }

    let rx = state.log_tx.subscribe();
    run_push_session(socket, state, session, "logs", rx, |log| {
        serde_json::to_string(&serde_json::json!({ "type": "tick", "data": log }))
            .ok()
            .map(Utf8Bytes::from)
    })
    .await;
}
