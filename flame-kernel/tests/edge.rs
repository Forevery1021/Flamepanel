//! Agent Edge 反向通道 + 远程交互式终端端到端测试（Doc/23 §8、Doc/30）。
//!
//! 在随机端口上启动完整面板路由（含中间件），伪 Agent 与伪浏览器均通过**真实 WebSocket**
//! 连接，覆盖：hello 鉴权（4001）、请求经 Edge 回放（远程命令 / 文件列表）、节点状态透出、
//! 远程终端双向数据流与退出、Agent 未开启终端、权限（401 / 4003）、删除节点断开通道。

use std::time::Duration;

use axum::http::{header, Method, Request, StatusCode};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;

mod common;
use common::setup_router;

use flame_kernel::api::middleware;
use flame_kernel::utils::jwt::JwtUtils;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const T: Duration = Duration::from_secs(10);

fn b64(s: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(s)
}
fn unb64(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}

struct Panel {
    app: axum::Router,
    addr: std::net::SocketAddr,
    admin: String,
    viewer: String,
}

async fn start_panel() -> Panel {
    flame_kernel::api::rate_limiter::init_global_limiter(100000, 60);
    let (router, state) = setup_router().await;
    let viewer = {
        let u = state
            .user_service
            .create_user("edge-viewer", "hash", "viewer")
            .await
            .unwrap();
        JwtUtils::new("test-secret", 24).sign(u.id).unwrap()
    };
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let served = app.clone();
    tokio::spawn(async move {
        let _ = axum::serve(listener, served.into_make_service()).await;
    });
    Panel {
        app,
        addr,
        admin: JwtUtils::new("test-secret", 24).sign(1).unwrap(),
        viewer,
    }
}

async fn call(
    app: &axum::Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(b.to_string())),
        None => req.body(axum::body::Body::empty()),
    };
    let res = app.clone().oneshot(req.unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn create_node(p: &Panel, name: &str, token: &str) -> i64 {
    let (s, id) = call(
        &p.app,
        Method::POST,
        "/api/nodes",
        Some(&p.admin),
        Some(json!({"node": {
            "id": 0, "name": name, "hostname": format!("{name}.example.com"),
            // 不可达地址：证明请求确实经 Edge 而非直连
            "ip_address": "192.0.2.1", "status": "online",
            "created_at": "2026-01-01T00:00:00Z", "auth_token": token
        }})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{id}");
    id.as_i64().unwrap()
}

async fn ws_connect(p: &Panel, path: &str) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{}{}", p.addr, path))
        .await
        .unwrap();
    ws
}

/// 读下一条文本帧（跳过 Ping/Pong）；收到 Close 返回 `Err(code)`。
async fn next_text(ws: &mut Ws) -> Result<Value, u16> {
    loop {
        let m = tokio::time::timeout(T, ws.next())
            .await
            .expect("ws read timeout");
        match m {
            Some(Ok(Message::Text(t))) => return Ok(serde_json::from_str(&t).unwrap()),
            Some(Ok(Message::Close(f))) => return Err(f.map(|f| u16::from(f.code)).unwrap_or(0)),
            Some(Ok(_)) => continue,
            None | Some(Err(_)) => return Err(0),
        }
    }
}

async fn send_json(ws: &mut Ws, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

/// 伪 Agent 握手：发送 hello，期望 welcome。
async fn agent_hello(p: &Panel, node_id: i64, token: &str, terminal: bool) -> Result<Ws, u16> {
    let mut ws = ws_connect(p, "/ws/agent/edge").await;
    send_json(
        &mut ws,
        json!({"type":"hello","node_id":node_id,"token":token,"version":"9.9.9","terminal":terminal}),
    )
    .await;
    let v = next_text(&mut ws).await?;
    assert_eq!(v["type"], "welcome");
    assert_eq!(v["node_id"], node_id);
    Ok(ws)
}

#[tokio::test]
async fn edge_hello_rejects_bad_token_and_unknown_node() {
    let p = start_panel().await;
    let id = create_node(&p, "edge-auth", "edge-secret").await;
    assert_eq!(agent_hello(&p, id, "wrong", false).await.err(), Some(4001));
    assert_eq!(
        agent_hello(&p, 987654, "edge-secret", false).await.err(),
        Some(4001)
    );
    // 首帧不是 hello
    let mut ws = ws_connect(&p, "/ws/agent/edge").await;
    send_json(&mut ws, json!({"type":"auth","token":p.admin})).await;
    assert_eq!(next_text(&mut ws).await.err(), Some(4001));
    // 未建立连接时状态为离线
    let (s, st) = call(
        &p.app,
        Method::GET,
        &format!("/api/nodes/{id}/status"),
        Some(&p.admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(st["edge"]["connected"], false);
}

#[tokio::test]
async fn edge_replays_requests_and_reports_status() {
    let p = start_panel().await;
    let id = create_node(&p, "edge-req", "edge-secret").await;
    let mut agent = agent_hello(&p, id, "edge-secret", false).await.unwrap();

    let (_, st) = call(
        &p.app,
        Method::GET,
        &format!("/api/nodes/{id}/status"),
        Some(&p.admin),
        None,
    )
    .await;
    assert_eq!(st["edge"]["connected"], true, "{st}");
    assert_eq!(st["edge"]["version"], "9.9.9");
    assert_eq!(st["edge"]["terminal"], false);

    // 伪 Agent：应答两个请求，并断言帧内不含 token
    let agent_task = tokio::spawn(async move {
        let mut seen = Vec::new();
        for _ in 0..2 {
            let req = next_text(&mut agent).await.unwrap();
            assert_eq!(req["type"], "req");
            assert!(req.get("token").is_none(), "Edge 请求帧不得携带 token");
            let path = req["path"].as_str().unwrap().to_string();
            let body = if path == "/action" {
                let sent: Value =
                    serde_json::from_slice(&unb64(req["body"].as_str().unwrap())).unwrap();
                assert_eq!(sent["action"], "whitelisted_command");
                assert_eq!(sent["params"]["command"], "uptime");
                json!({"status":"ok","data":{"output":"up 3 days","exit_code":0}})
            } else {
                assert!(path.starts_with("/files/list?path="), "{path}");
                json!([{"name":"etc","is_dir":true,"size":0,"modified":"2026-01-01T00:00:00Z"}])
            };
            send_json(
                &mut agent,
                json!({"type":"resp","id":req["id"],"status":200,
                       "content_type":"application/json","body":b64(body.to_string().as_bytes())}),
            )
            .await;
            seen.push(path);
        }
        (agent, seen)
    });

    let (s, out) = call(
        &p.app,
        Method::POST,
        &format!("/api/nodes/{id}/execute"),
        Some(&p.admin),
        Some(json!({"command":"uptime"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{out}");
    assert_eq!(out["output"], "up 3 days");
    assert_eq!(out["exit_code"], 0);

    let (s, files) = call(
        &p.app,
        Method::GET,
        &format!("/api/nodes/{id}/files?path=/"),
        Some(&p.admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{files}");
    assert_eq!(files[0]["name"], "etc");

    let (mut agent, seen) = agent_task.await.unwrap();
    assert_eq!(seen[0], "/action");
    assert_eq!(seen[1], "/files/list?path=%2F");

    // 删除节点 → 面板主动断开 Edge 通道
    let (s, _) = call(
        &p.app,
        Method::DELETE,
        &format!("/api/nodes/{id}"),
        Some(&p.admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(next_text(&mut agent).await.is_err(), "节点删除后通道应关闭");
}

#[tokio::test]
async fn remote_terminal_streams_both_ways() {
    let p = start_panel().await;
    let id = create_node(&p, "edge-pty", "edge-secret").await;
    let mut agent = agent_hello(&p, id, "edge-secret", true).await.unwrap();

    let mut browser = ws_connect(&p, &format!("/ws/terminal?node_id={id}")).await;
    send_json(&mut browser, json!({"type":"auth","token":p.admin})).await;

    let open = next_text(&mut agent).await.unwrap();
    assert_eq!(open["type"], "pty_open");
    let sid = open["sid"].as_str().unwrap().to_string();

    // Agent → 浏览器：多字节 UTF-8 被拆在两帧之间也能正确拼接
    let bytes = "你好 edge\r\n".as_bytes();
    send_json(
        &mut agent,
        json!({"type":"pty_out","sid":sid,"data":b64(&bytes[..2])}),
    )
    .await;
    send_json(
        &mut agent,
        json!({"type":"pty_out","sid":sid,"data":b64(&bytes[2..])}),
    )
    .await;
    let mut text = String::new();
    while !text.contains("edge") {
        let v = next_text(&mut browser).await.unwrap();
        assert_eq!(v["type"], "output");
        text.push_str(v["data"].as_str().unwrap());
    }
    assert_eq!(text, "你好 edge\r\n");

    // 浏览器 → Agent：输入与 resize
    send_json(&mut browser, json!({"type":"input","data":"ls\r"})).await;
    send_json(&mut browser, json!({"type":"resize","cols":132,"rows":40})).await;
    let mut got_in = false;
    let mut got_resize = false;
    while !(got_in && got_resize) {
        let v = next_text(&mut agent).await.unwrap();
        assert_eq!(v["sid"], sid.as_str());
        match v["type"].as_str().unwrap() {
            "pty_in" => {
                assert_eq!(unb64(v["data"].as_str().unwrap()), b"ls\r");
                got_in = true;
            }
            "pty_resize" => {
                assert_eq!(
                    (v["cols"].as_u64(), v["rows"].as_u64()),
                    (Some(132), Some(40))
                );
                got_resize = true;
            }
            other => panic!("unexpected frame {other}"),
        }
    }

    // shell 退出 → 浏览器收到 exit
    send_json(&mut agent, json!({"type":"pty_exit","sid":sid,"code":0})).await;
    let v = next_text(&mut browser).await.unwrap();
    assert_eq!(v, json!({"type":"exit","code":0}));
}

#[tokio::test]
async fn remote_terminal_errors_and_permissions() {
    let p = start_panel().await;
    let id = create_node(&p, "edge-pty-off", "edge-secret").await;

    // 节点未建立 Edge 通道
    let mut b = ws_connect(&p, &format!("/ws/terminal?node_id={id}")).await;
    send_json(&mut b, json!({"type":"auth","token":p.admin})).await;
    let v = next_text(&mut b).await.unwrap();
    assert_eq!(v["type"], "error");
    assert_eq!(v["code"], "EDGE_OFFLINE");

    // Agent 在线但未开启终端
    let _agent = agent_hello(&p, id, "edge-secret", false).await.unwrap();
    let mut b = ws_connect(&p, &format!("/ws/terminal?node_id={id}")).await;
    send_json(&mut b, json!({"type":"auth","token":p.admin})).await;
    let v = next_text(&mut b).await.unwrap();
    assert_eq!(v["code"], "TERMINAL_DISABLED");

    // 401：非法 token
    let mut b = ws_connect(&p, &format!("/ws/terminal?node_id={id}")).await;
    send_json(&mut b, json!({"type":"auth","token":"invalid"})).await;
    assert_eq!(next_text(&mut b).await.err(), Some(4001));

    // 403：viewer 无 terminal:execute / node:execute
    let mut b = ws_connect(&p, &format!("/ws/terminal?node_id={id}")).await;
    send_json(&mut b, json!({"type":"auth","token":p.viewer})).await;
    assert_eq!(next_text(&mut b).await.err(), Some(4003));

    // 404：节点不存在
    let mut b = ws_connect(&p, "/ws/terminal?node_id=987654").await;
    send_json(&mut b, json!({"type":"auth","token":p.admin})).await;
    assert_eq!(next_text(&mut b).await.err(), Some(4004));
}
