//! Agent Edge 出站反向通道（Doc/23 §8、Doc/30 Agent Edge）。
//!
//! 节点在 NAT / 防火墙后、面板无法直连 `:9527` 时，Agent 主动以 WebSocket 连到面板
//! `/ws/agent/edge`，之后面板对该节点的全部调用经这条连接转发：
//!
//! - **请求回放**：面板发 `req` 帧（方法 + 路径 + body），Agent 在进程内把它交给**自己的**
//!   axum 路由处理，回 `resp` 帧——`/action` 白名单、文件根目录约束、Agent token 校验全部
//!   复用直连模式的同一实现，不存在第二套语义；
//! - **远程终端**：`pty_*` 帧双向转发 PTY 字节流（Agent 侧需 `AGENT_TERMINAL=1` 显式开启）。
//!
//! 本模块只管会话注册与帧编解码（纯内存，不触网）；握手鉴权在 `api/handler/ws/edge.rs`。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::core::error::AppError;
use crate::utils::sync::lock_or_recover;

/// Edge 帧大小上限（请求 / 响应 body 解码前）：与直连模式下载上限同量级，防止单帧耗尽内存。
pub const EDGE_MAX_FRAME_BYTES: usize = 48 * 1024 * 1024;
/// 单节点并发远程终端上限（Agent 侧另有自己的上限）。
pub const EDGE_MAX_PTYS_PER_NODE: usize = 8;
/// 出站帧队列深度：写端阻塞时背压到调用方，而不是无限堆积。
const OUTBOUND_QUEUE: usize = 256;
/// 终端输出队列深度：浏览器消费过慢时丢弃会话而不是拖垮整条 Edge 连接。
const PTY_QUEUE: usize = 512;

/// 面板 → Agent 帧。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PanelFrame {
    /// 握手成功
    Welcome {
        node_id: i64,
    },
    /// 回放一次 HTTP 请求到 Agent 自身路由
    Req {
        id: u64,
        method: String,
        /// 含 query 的路径，如 `/files/list?path=.`
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content_type: Option<String>,
        /// base64 body（可空）
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

/// Agent → 面板帧。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentFrame {
    /// 首帧：节点 ID + 注册时的 auth_token
    Hello {
        node_id: i64,
        token: String,
        #[serde(default)]
        version: String,
        /// Agent 是否开启远程终端（`AGENT_TERMINAL=1`）
        #[serde(default)]
        terminal: bool,
    },
    Resp {
        id: u64,
        status: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content_type: Option<String>,
        #[serde(default)]
        body: String,
    },
    PtyOut {
        sid: String,
        data: String,
    },
    PtyExit {
        sid: String,
        #[serde(default)]
        code: Option<i32>,
    },
    PtyError {
        sid: String,
        message: String,
    },
}

/// 一次回放请求的结果。
#[derive(Debug, Clone)]
pub struct EdgeResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

/// 终端事件（Edge → 浏览器桥接）。
#[derive(Debug, Clone, PartialEq)]
pub enum PtyEvent {
    Output(Vec<u8>),
    Exit(Option<i32>),
    Error(String),
}

/// 节点 Edge 连接概况（`GET /api/nodes/{id}/status` 透出）。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct EdgeInfo {
    pub connected: bool,
    /// Agent 是否开启远程终端
    pub terminal: bool,
    pub version: String,
    /// 连接建立时刻（RFC 3339）
    pub connected_at: Option<String>,
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<EdgeResponse>>>>;
type Ptys = Arc<Mutex<HashMap<String, mpsc::Sender<PtyEvent>>>>;

struct Session {
    /// 连接代际：同一节点重连时新连接顶替旧连接，旧连接收尾不得误删新会话
    conn_id: u64,
    tx: mpsc::Sender<String>,
    pending: Pending,
    ptys: Ptys,
    terminal: bool,
    version: String,
    connected_at: chrono::DateTime<chrono::Utc>,
}

/// Edge 会话注册表（节点 ID → 当前连接）。克隆共享同一份状态。
#[derive(Clone, Default)]
pub struct EdgeHub {
    inner: Arc<Mutex<HashMap<i64, Session>>>,
    next_id: Arc<AtomicU64>,
}

/// 新连接的句柄：WS 处理器据此取出站帧、分发入站帧、断开时注销。
pub struct EdgeConnection {
    pub node_id: i64,
    pub conn_id: u64,
    pub outbound: mpsc::Receiver<String>,
    pending: Pending,
    ptys: Ptys,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

impl EdgeHub {
    pub fn new() -> Self {
        Self::default()
    }

    fn next(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// 注册新连接；同节点旧连接被顶替（其挂起请求立即失败、终端收到 Exit）。
    pub fn attach(&self, node_id: i64, terminal: bool, version: String) -> EdgeConnection {
        let (tx, outbound) = mpsc::channel(OUTBOUND_QUEUE);
        let pending: Pending = Arc::default();
        let ptys: Ptys = Arc::default();
        let conn_id = self.next();
        let old = lock_or_recover(&self.inner).insert(
            node_id,
            Session {
                conn_id,
                tx,
                pending: pending.clone(),
                ptys: ptys.clone(),
                terminal,
                version,
                connected_at: chrono::Utc::now(),
            },
        );
        if let Some(old) = old {
            fail_session(&old);
        }
        EdgeConnection {
            node_id,
            conn_id,
            outbound,
            pending,
            ptys,
        }
    }

    /// 连接结束：仅当注册表里仍是本连接时移除（重连顶替后旧连接收尾为 no-op）。
    pub fn detach(&self, conn: &EdgeConnection) {
        let mut map = lock_or_recover(&self.inner);
        if map.get(&conn.node_id).map(|s| s.conn_id) == Some(conn.conn_id) {
            if let Some(s) = map.remove(&conn.node_id) {
                fail_session(&s);
            }
        }
    }

    /// 节点被删除：断开其 Edge 连接。
    pub fn remove_node(&self, node_id: i64) {
        if let Some(s) = lock_or_recover(&self.inner).remove(&node_id) {
            fail_session(&s);
        }
    }

    pub fn is_connected(&self, node_id: i64) -> bool {
        lock_or_recover(&self.inner).contains_key(&node_id)
    }

    pub fn info(&self, node_id: i64) -> EdgeInfo {
        match lock_or_recover(&self.inner).get(&node_id) {
            Some(s) => EdgeInfo {
                connected: true,
                terminal: s.terminal,
                version: s.version.clone(),
                connected_at: Some(s.connected_at.to_rfc3339()),
            },
            None => EdgeInfo {
                connected: false,
                terminal: false,
                version: String::new(),
                connected_at: None,
            },
        }
    }

    pub fn len(&self) -> usize {
        lock_or_recover(&self.inner).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn sender(
        &self,
        node_id: i64,
    ) -> Result<(mpsc::Sender<String>, Pending, Ptys, bool), AppError> {
        lock_or_recover(&self.inner)
            .get(&node_id)
            .map(|s| (s.tx.clone(), s.pending.clone(), s.ptys.clone(), s.terminal))
            .ok_or_else(|| {
                AppError::ServiceUnavailable(format!(
                    "node {node_id} has no active edge connection"
                ))
            })
    }

    /// 经 Edge 回放一次请求，等待 Agent 响应（超时 / 断线返回 503）。
    pub async fn request(
        &self,
        node_id: i64,
        method: &str,
        path: &str,
        content_type: Option<&str>,
        body: &[u8],
        timeout: Duration,
    ) -> Result<EdgeResponse, AppError> {
        if body.len() > EDGE_MAX_FRAME_BYTES {
            return Err(AppError::BadRequest(format!(
                "request body exceeds edge frame limit ({} bytes)",
                EDGE_MAX_FRAME_BYTES
            )));
        }
        let (tx, pending, _, _) = self.sender(node_id)?;
        let id = self.next();
        let frame = PanelFrame::Req {
            id,
            method: method.to_string(),
            path: path.to_string(),
            content_type: content_type.map(str::to_string),
            body: b64().encode(body),
        };
        let text = encode(&frame)?;
        let (done_tx, done_rx) = oneshot::channel();
        lock_or_recover(&pending).insert(id, done_tx);
        // 发送失败 / 超时都要清理挂起表，避免泄漏
        let outcome = async {
            tx.send(text)
                .await
                .map_err(|_| AppError::ServiceUnavailable("edge connection closed".into()))?;
            match tokio::time::timeout(timeout, done_rx).await {
                Ok(Ok(resp)) => Ok(resp),
                Ok(Err(_)) => Err(AppError::ServiceUnavailable(
                    "edge connection closed before response".into(),
                )),
                Err(_) => Err(AppError::ServiceUnavailable(format!(
                    "edge request timed out after {}s",
                    timeout.as_secs()
                ))),
            }
        }
        .await;
        lock_or_recover(&pending).remove(&id);
        outcome
    }

    /// 打开远程终端：返回会话 ID 与输出流。Agent 未开启终端时 403。
    pub async fn open_pty(
        &self,
        node_id: i64,
        cols: u16,
        rows: u16,
    ) -> Result<(String, mpsc::Receiver<PtyEvent>), AppError> {
        let (tx, _, ptys, terminal) = self.sender(node_id)?;
        if !terminal {
            return Err(AppError::Forbidden(
                "remote terminal is disabled on this agent (set AGENT_TERMINAL=1 on the node)"
                    .into(),
            ));
        }
        let sid = format!("{node_id}-{}", self.next());
        let (ev_tx, ev_rx) = mpsc::channel(PTY_QUEUE);
        {
            let mut map = lock_or_recover(&ptys);
            if map.len() >= EDGE_MAX_PTYS_PER_NODE {
                return Err(AppError::RateLimited(format!(
                    "too many terminals on node {node_id} (max {EDGE_MAX_PTYS_PER_NODE})"
                )));
            }
            map.insert(sid.clone(), ev_tx);
        }
        let frame = encode(&PanelFrame::PtyOpen {
            sid: sid.clone(),
            cols: cols.clamp(1, 1000),
            rows: rows.clamp(1, 1000),
        })?;
        if tx.send(frame).await.is_err() {
            lock_or_recover(&ptys).remove(&sid);
            return Err(AppError::ServiceUnavailable(
                "edge connection closed".into(),
            ));
        }
        Ok((sid, ev_rx))
    }

    pub async fn pty_input(&self, node_id: i64, sid: &str, data: &[u8]) -> Result<(), AppError> {
        let (tx, ..) = self.sender(node_id)?;
        let frame = encode(&PanelFrame::PtyIn {
            sid: sid.to_string(),
            data: b64().encode(data),
        })?;
        tx.send(frame)
            .await
            .map_err(|_| AppError::ServiceUnavailable("edge connection closed".into()))
    }

    pub async fn pty_resize(
        &self,
        node_id: i64,
        sid: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), AppError> {
        let (tx, ..) = self.sender(node_id)?;
        let frame = encode(&PanelFrame::PtyResize {
            sid: sid.to_string(),
            cols: cols.clamp(1, 1000),
            rows: rows.clamp(1, 1000),
        })?;
        tx.send(frame)
            .await
            .map_err(|_| AppError::ServiceUnavailable("edge connection closed".into()))
    }

    /// 关闭远程终端（幂等；连接已断时静默）。
    pub async fn pty_close(&self, node_id: i64, sid: &str) {
        if let Ok((tx, _, ptys, _)) = self.sender(node_id) {
            let existed = lock_or_recover(&ptys).remove(sid).is_some();
            if existed {
                if let Ok(frame) = encode(&PanelFrame::PtyClose {
                    sid: sid.to_string(),
                }) {
                    let _ = tx.send(frame).await;
                }
            }
        }
    }
}

impl EdgeConnection {
    /// 分发一条 Agent 入站帧（Hello 之后）。返回 false 表示协议错误、应断开。
    pub fn dispatch(&self, frame: AgentFrame) -> bool {
        match frame {
            AgentFrame::Hello { .. } => false,
            AgentFrame::Resp {
                id,
                status,
                content_type,
                body,
            } => {
                let Ok(body) = b64().decode(body.as_bytes()) else {
                    return false;
                };
                if let Some(tx) = lock_or_recover(&self.pending).remove(&id) {
                    let _ = tx.send(EdgeResponse {
                        status,
                        content_type,
                        body,
                    });
                }
                true
            }
            AgentFrame::PtyOut { sid, data } => {
                let Ok(bytes) = b64().decode(data.as_bytes()) else {
                    return false;
                };
                self.pty_event(&sid, PtyEvent::Output(bytes), false);
                true
            }
            AgentFrame::PtyExit { sid, code } => {
                self.pty_event(&sid, PtyEvent::Exit(code), true);
                true
            }
            AgentFrame::PtyError { sid, message } => {
                self.pty_event(&sid, PtyEvent::Error(message), true);
                true
            }
        }
    }

    fn pty_event(&self, sid: &str, ev: PtyEvent, last: bool) {
        let mut map = lock_or_recover(&self.ptys);
        let Some(tx) = map.get(sid) else { return };
        // 浏览器消费不过来（队列满）即视为会话失效，丢弃而不是阻塞整条 Edge 连接
        if tx.try_send(ev).is_err() || last {
            map.remove(sid);
        }
    }
}

/// 连接失效：挂起请求立即失败（drop sender → 调用方收到「连接已关闭」），终端收到 Exit。
fn fail_session(s: &Session) {
    lock_or_recover(&s.pending).clear();
    for (_, tx) in lock_or_recover(&s.ptys).drain() {
        let _ = tx.try_send(PtyEvent::Error("edge connection closed".into()));
    }
}

pub fn encode(frame: &PanelFrame) -> Result<String, AppError> {
    serde_json::to_string(frame).map_err(|e| AppError::internal(format!("edge frame encode: {e}")))
}

/// 解析 Agent 帧（超限 / 非法 JSON 返回 None）。
pub fn decode(text: &str) -> Option<AgentFrame> {
    if text.len() > EDGE_MAX_FRAME_BYTES * 4 / 3 + 1024 {
        return None;
    }
    serde_json::from_str(text).ok()
}

/// `edge://<node_id>` 形式的 base_url（`AgentClient` 据此改走反向通道）。
pub fn edge_base_url(node_id: i64) -> String {
    format!("edge://{node_id}")
}

/// 从 base_url 解析 Edge 节点 ID。
pub fn parse_edge_base_url(base: &str) -> Option<i64> {
    base.strip_prefix("edge://")?
        .trim_end_matches('/')
        .parse()
        .ok()
}

/// 增量 UTF-8 解码：PTY 输出按字节分块，多字节字符可能被拆在两块之间。
/// 把 `carry`（上一块残留）与新数据拼接，返回可完整解码的部分，末尾不完整的字节留在 `carry`；
/// 真正非法的字节以 U+FFFD 替换（与 `from_utf8_lossy` 一致），不会无限滞留。
pub fn utf8_stream_decode(carry: &mut Vec<u8>, chunk: &[u8]) -> String {
    carry.extend_from_slice(chunk);
    let mut out = String::new();
    let mut rest: &[u8] = carry;
    loop {
        match std::str::from_utf8(rest) {
            Ok(s) => {
                out.push_str(s);
                rest = &[];
                break;
            }
            Err(e) => {
                let (valid, after) = rest.split_at(e.valid_up_to());
                // valid_up_to 之前已验证为合法 UTF-8
                out.push_str(&String::from_utf8_lossy(valid));
                match e.error_len() {
                    // 末尾不完整序列：留待下一块
                    None => {
                        rest = after;
                        break;
                    }
                    Some(n) => {
                        out.push('\u{FFFD}');
                        rest = &after[n..];
                    }
                }
            }
        }
    }
    let keep = rest.to_vec();
    *carry = keep;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(id: u64, status: u16, body: &[u8]) -> AgentFrame {
        AgentFrame::Resp {
            id,
            status,
            content_type: Some("application/json".into()),
            body: b64().encode(body),
        }
    }

    /// 取出下一条出站帧并解析为 PanelFrame
    async fn next_out(conn: &mut EdgeConnection) -> PanelFrame {
        let text = conn.outbound.recv().await.unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn frames_use_snake_case_tags() {
        let f = PanelFrame::PtyResize {
            sid: "s".into(),
            cols: 80,
            rows: 24,
        };
        let v: serde_json::Value = serde_json::from_str(&encode(&f).unwrap()).unwrap();
        assert_eq!(v["type"], "pty_resize");
        let hello = decode(r#"{"type":"hello","node_id":3,"token":"t"}"#).unwrap();
        assert_eq!(
            hello,
            AgentFrame::Hello {
                node_id: 3,
                token: "t".into(),
                version: String::new(),
                terminal: false
            }
        );
        assert!(decode("{not json").is_none());
        assert!(decode(r#"{"type":"unknown"}"#).is_none());
    }

    #[test]
    fn utf8_stream_decode_handles_split_and_invalid_bytes() {
        let bytes = "你好".as_bytes(); // 6 字节
        let mut carry = Vec::new();
        assert_eq!(utf8_stream_decode(&mut carry, &bytes[..4]), "你");
        assert_eq!(carry.len(), 1);
        assert_eq!(utf8_stream_decode(&mut carry, &bytes[4..]), "好");
        assert!(carry.is_empty());
        assert_eq!(utf8_stream_decode(&mut carry, b"a\xffb"), "a\u{FFFD}b");
        assert!(carry.is_empty());
    }

    #[test]
    fn base_url_roundtrip() {
        assert_eq!(parse_edge_base_url(&edge_base_url(42)), Some(42));
        assert_eq!(parse_edge_base_url("http://10.0.0.1:9527"), None);
        assert_eq!(parse_edge_base_url("edge://x"), None);
    }

    #[tokio::test]
    async fn request_roundtrip_through_connection() {
        let hub = EdgeHub::new();
        let mut conn = hub.attach(7, false, "1.0.0".into());
        assert!(hub.is_connected(7));
        let h = hub.clone();
        let call = tokio::spawn(async move {
            h.request(
                7,
                "GET",
                "/files/list?path=.",
                None,
                b"",
                Duration::from_secs(5),
            )
            .await
        });
        let PanelFrame::Req {
            id, method, path, ..
        } = next_out(&mut conn).await
        else {
            panic!("expected req")
        };
        assert_eq!(
            (method.as_str(), path.as_str()),
            ("GET", "/files/list?path=.")
        );
        assert!(conn.dispatch(resp(id, 200, b"[]")));
        let r = call.await.unwrap().unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, b"[]".as_slice()));
    }

    #[tokio::test]
    async fn request_without_connection_is_503_and_timeout_cleans_pending() {
        let hub = EdgeHub::new();
        let e = hub
            .request(1, "GET", "/x", None, b"", Duration::from_millis(10))
            .await
            .unwrap_err();
        assert!(matches!(e, AppError::ServiceUnavailable(_)));

        let mut conn = hub.attach(1, false, String::new());
        let e = hub
            .request(1, "GET", "/x", None, b"", Duration::from_millis(20))
            .await
            .unwrap_err();
        assert!(e.to_string().contains("timed out"), "{e}");
        let _ = next_out(&mut conn).await;
        assert!(
            lock_or_recover(&conn.pending).is_empty(),
            "超时后清理挂起表"
        );
    }

    #[tokio::test]
    async fn reconnect_replaces_old_session_and_fails_its_pending() {
        let hub = EdgeHub::new();
        let mut old = hub.attach(5, false, "a".into());
        let h = hub.clone();
        let call = tokio::spawn(async move {
            h.request(5, "GET", "/x", None, b"", Duration::from_secs(5))
                .await
        });
        let _ = next_out(&mut old).await;
        let new = hub.attach(5, true, "b".into());
        let e = call.await.unwrap().unwrap_err();
        assert!(e.to_string().contains("closed"), "{e}");
        // 旧连接收尾不得移除新会话
        hub.detach(&old);
        assert!(hub.is_connected(5));
        assert_eq!(hub.info(5).version, "b");
        hub.detach(&new);
        assert!(!hub.is_connected(5));
        assert!(!hub.info(5).connected);
    }

    #[tokio::test]
    async fn pty_requires_agent_opt_in_and_routes_events() {
        let hub = EdgeHub::new();
        let _off = hub.attach(2, false, String::new());
        let e = hub.open_pty(2, 80, 24).await.unwrap_err();
        assert!(matches!(e, AppError::Forbidden(_)));

        let mut conn = hub.attach(2, true, String::new());
        let (sid, mut rx) = hub.open_pty(2, 0, 5000).await.unwrap();
        assert_eq!(
            next_out(&mut conn).await,
            PanelFrame::PtyOpen {
                sid: sid.clone(),
                cols: 1,
                rows: 1000
            },
            "尺寸钳制"
        );
        hub.pty_input(2, &sid, b"ls\n").await.unwrap();
        let PanelFrame::PtyIn { data, .. } = next_out(&mut conn).await else {
            panic!()
        };
        assert_eq!(b64().decode(data).unwrap(), b"ls\n");

        assert!(conn.dispatch(AgentFrame::PtyOut {
            sid: sid.clone(),
            data: b64().encode(b"hi")
        }));
        assert_eq!(rx.recv().await, Some(PtyEvent::Output(b"hi".to_vec())));
        assert!(conn.dispatch(AgentFrame::PtyExit {
            sid: sid.clone(),
            code: Some(0)
        }));
        assert_eq!(rx.recv().await, Some(PtyEvent::Exit(Some(0))));
        assert!(lock_or_recover(&conn.ptys).is_empty(), "退出后注销");
        // 非法 base64 视为协议错误
        assert!(!conn.dispatch(AgentFrame::PtyOut {
            sid,
            data: "!!".into()
        }));
    }

    #[tokio::test]
    async fn pty_limit_per_node() {
        let hub = EdgeHub::new();
        let _conn = hub.attach(9, true, String::new());
        let mut keep = Vec::new();
        for _ in 0..EDGE_MAX_PTYS_PER_NODE {
            keep.push(hub.open_pty(9, 80, 24).await.unwrap());
        }
        let e = hub.open_pty(9, 80, 24).await.unwrap_err();
        assert!(matches!(e, AppError::RateLimited(_)));
        hub.pty_close(9, &keep[0].0).await;
        assert!(hub.open_pty(9, 80, 24).await.is_ok());
    }

    #[tokio::test]
    async fn remove_node_disconnects() {
        let hub = EdgeHub::new();
        let _conn = hub.attach(3, false, String::new());
        hub.remove_node(3);
        assert!(hub.is_empty());
    }
}
