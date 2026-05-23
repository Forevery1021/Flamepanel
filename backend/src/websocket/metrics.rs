use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::application::AppState;

pub async fn metrics_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_metrics(socket, state))
}

async fn handle_metrics(mut socket: WebSocket, state: AppState) {
    // 先发送历史数据
    {
        let history = state.metrics_history.lock().await;
        let snapshots = history.get_all();
        if let Ok(payload) = serde_json::to_string(&serde_json::json!({
            "type": "init",
            "data": snapshots,
        })) {
            if socket.send(Message::Text(payload.into())).await.is_err() {
                return;
            }
        }
    }

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 广播接收 → WebSocket 发送
    let mut rx = state.metrics_tx.subscribe();
    let send_task = tokio::spawn(async move {
        while let Ok(snapshot) = rx.recv().await {
            if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                "type": "tick",
                "data": snapshot,
            })) {
                if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // 接收端仅用于检测关闭
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
