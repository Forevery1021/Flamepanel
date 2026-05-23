pub mod metrics;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::application::{AppState, SessionHandle};

#[derive(Deserialize)]
pub struct TerminalQuery {
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

fn default_cols() -> u16 { 120 }
fn default_rows() -> u16 { 40 }

#[derive(Deserialize)]
struct TerminalMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    #[allow(dead_code)]
    data: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/terminal", get(terminal_handler))
        .route("/terminal/sessions", get(list_sessions))
        .route("/metrics", get(metrics::metrics_handler))
}

async fn terminal_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<TerminalQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal(socket, query, state.sessions))
}

async fn handle_terminal(mut socket: WebSocket, query: TerminalQuery, sessions: Arc<Mutex<std::collections::HashMap<String, Arc<Mutex<SessionHandle>>>>>) {
    let session_id = uuid::Uuid::new_v4().to_string();

    let mut child = match Command::new("bash")
        .arg("-i")
        .env("TERM", "xterm-256color")
        .env("COLUMNS", query.cols.to_string())
        .env("LINES", query.rows.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => Command::new("sh")
            .arg("-i")
            .env("TERM", "xterm-256color")
            .env("COLUMNS", query.cols.to_string())
            .env("LINES", query.rows.to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("无法启动 shell"),
    };

    let mut child_stdin = child.stdin.take().expect("无法获取 stdin");
    let child_stdout = child.stdout.take().expect("无法获取 stdout");
    let child_stderr = child.stderr.take().expect("无法获取 stderr");

    let handle = Arc::new(Mutex::new(SessionHandle {
        child,
        cols: query.cols,
        rows: query.rows,
    }));

    sessions.lock().await.insert(session_id.clone(), handle.clone());

    let _ = socket.send(Message::Text(format!("{{\"type\":\"session\",\"id\":\"{}\"}}", session_id).into())).await;

    let _ = child_stdin.write_all(
        format!("stty cols {} rows {}\n", query.cols, query.rows).as_bytes()
    ).await;
    let _ = child_stdin.flush().await;

    let (ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    let tx1 = tx.clone();
    tokio::spawn(async move {
        let mut reader = BufReader::new(child_stdout);
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if tx1.send(buf[..n].to_vec()).await.is_err() { break; }
                }
                Err(_) => break,
            }
        }
    });

    tokio::spawn(async move {
        let mut reader = BufReader::new(child_stderr);
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).await.is_err() { break; }
                }
                Err(_) => break,
            }
        }
    });

    let mut ws_sender = ws_sender;
    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if ws_sender.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });

    let stdin_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ctrl) = serde_json::from_str::<TerminalMessage>(&text) {
                        match ctrl.msg_type.as_deref() {
                            Some("resize") => {
                                if let (Some(c), Some(r)) = (ctrl.cols, ctrl.rows) {
                                    let cmd = format!("stty cols {} rows {}\n", c, r);
                                    let _ = child_stdin.write_all(cmd.as_bytes()).await;
                                    let _ = child_stdin.flush().await;
                                    continue;
                                }
                            }
                            Some("ping") => continue,
                            _ => {}
                        }
                    }
                    let _ = child_stdin.write_all(text.as_bytes()).await;
                    let _ = child_stdin.flush().await;
                }
                Message::Binary(data) => {
                    let _ = child_stdin.write_all(&data).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = stdin_task => {}
    }

    let mut h = handle.lock().await;
    let _ = h.child.kill().await;
    let _ = h.child.wait().await;
    drop(h);

    sessions.lock().await.remove(&session_id);
}

async fn list_sessions(
    State(state): State<AppState>,
) -> axum::Json<serde_json::Value> {
    let sessions = state.sessions.lock().await;
    let ids: Vec<&String> = sessions.keys().collect();
    axum::Json(serde_json::json!({
        "count": ids.len(),
        "sessions": ids,
    }))
}
