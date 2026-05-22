use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, Query},
    response::IntoResponse,
    Router, routing::get, Extension,
};
use axum::body::Bytes;
use serde::Deserialize;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::middleware::auth::CurrentUser;

type SessionMap = Arc<Mutex<HashMap<String, Arc<Mutex<Child>>>>>;

#[derive(Deserialize)]
pub struct TerminalQuery {
    cols: u16,
    rows: u16,
}

pub fn routes() -> Router {
    Router::new().route("/terminal", get(terminal_handler))
}

async fn terminal_handler(
    ws: WebSocketUpgrade,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<TerminalQuery>,
    Extension(_sessions): Extension<SessionMap>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal(socket, query))
}

async fn handle_terminal(mut socket: WebSocket, query: TerminalQuery) {
    let mut child = Command::new("bash")
        .env("TERM", "xterm-256color")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn shell");

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    // 初始窗口大小（可选通过 stty 设置）
    let mut stdin = stdin;
    let mut stdout = BufReader::new(stdout);

    let _ = stdin.write_all(format!("stty cols {} rows {}\n", query.cols, query.rows).as_bytes()).await;

    loop {
        tokio::select! {
            // 从前端接收数据发送给 shell
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        let _ = stdin.write_all(text.as_bytes()).await;
                        let _ = stdin.flush().await;
                    }
                    Some(Ok(axum::extract::ws::Message::Binary(data))) => {
                        let _ = stdin.write_all(&data).await;
                    }
                    _ => break,
                }
            }
            // 从 shell 读取输出发送给前端
            res = stdout.fill_buf() => {
                match res {
                    Ok(n) if n.is_empty() => break,
                    Ok(buf) => {
                        let data = buf.to_vec();
                        let len = buf.len();
                        drop(buf);
                        stdout.consume(len);
                        if socket.send(axum::extract::ws::Message::Binary(Bytes::from(data))).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    let _ = child.kill().await;
}