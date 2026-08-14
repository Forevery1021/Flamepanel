use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};

use crate::api::types::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ws/metrics", get(metrics_handler))
        .route("/ws/logs", get(logs_handler))
        .route("/ws/terminal", get(terminal_handler))
}

async fn metrics_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_metrics(socket, state))
}

async fn handle_metrics(mut socket: WebSocket, state: AppState) {
    let history = state.metrics_history.lock().await;
    let snapshots = history.get_all();
    drop(history);

    if !snapshots.is_empty() {
        if let Ok(payload) = serde_json::to_string(&serde_json::json!({
            "type": "init",
            "data": snapshots,
        })) {
            let _ = socket.send(Message::Text(payload.into())).await;
        }
    }

    let (mut ws_sender, mut ws_receiver) = socket.split();

    let mut rx = state.metrics_tx.subscribe();
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(snapshot) => {
                    if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                        "type": "tick",
                        "data": snapshot,
                    })) {
                        if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    // T4：慢消费者滞后时告警并继续，不中断 WS 推送。
                    tracing::warn!("metrics ws consumer lagged by {n} messages; continuing");
                }
                Err(_closed) => break,
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}

async fn terminal_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal(socket, state))
}

async fn handle_terminal(mut socket: WebSocket, state: AppState) {
    let (id, mut rx) = match state.terminal_manager.create_session().await {
        Ok(v) => v,
        // T14：终端会话创建失败（如 shell 缺失）时优雅关闭而非 panic。
        Err(e) => {
            tracing::error!(error = %e, "terminal session create failed");
            let _ = socket.close().await;
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        while let Some(output) = rx.recv().await {
            if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                "type": "output",
                "data": output,
            })) {
                if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
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
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // T5：统一清理路径——无论 send_task 还是 recv_task 先结束，都在此关闭会话，避免终端会话泄漏。
    state.terminal_manager.close(id).await;
}

async fn logs_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs(socket, state))
}

async fn handle_logs(mut socket: WebSocket, state: AppState) {
    let recent_logs = state.log_service.list().await.unwrap_or_default();
    if !recent_logs.is_empty() {
        if let Ok(payload) = serde_json::to_string(&serde_json::json!({
            "type": "init",
            "data": recent_logs,
        })) {
            let _ = socket.send(Message::Text(payload.into())).await;
        }
    }

    let (mut ws_sender, mut ws_receiver) = socket.split();

    let mut rx = state.log_tx.subscribe();
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(log) => {
                    if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                        "type": "tick",
                        "data": log,
                    })) {
                        if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    // T4：慢消费者滞后时告警并继续，不中断 WS 推送。
                    tracing::warn!("log ws consumer lagged by {n} messages; continuing");
                }
                Err(_closed) => break,
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}
