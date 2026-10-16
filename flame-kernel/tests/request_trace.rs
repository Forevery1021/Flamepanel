//! 请求编号贯穿日志（W0 · Doc/32 §4）回归测试。
//!
//! 单独成一个测试二进制、且只有一个用例：`tracing::subscriber::set_default` 是线程局部的，而回调点兴趣缓存是
//! 进程全局的——与其他并行用例同进程时，别的线程先以「无订阅器」命中同一回调点会让本用例
//! 偶发收不到事件。独立进程内只有本用例，结果确定。

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::setup_router;

use flame_kernel::api::middleware;

/// 捕获订阅器输出的内存缓冲
#[derive(Clone, Default)]
struct CaptureBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for CaptureBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 回归：TraceLayer 的「请求完成」事件（状态码 + 耗时）必须带 request_id——
/// 否则没有业务日志的请求（如 4xx）无法在内核日志中按错误提示里的编号检索。
#[tokio::test]
async fn request_completion_event_carries_request_id() {
    let buf = CaptureBuf::default();
    let writer = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    // 回调点兴趣是进程级缓存：设置线程局部订阅器后强制重算，避免先前「无订阅器」时缓存的 never
    tracing::callsite::rebuild_interest_cache();

    let (router, state) = setup_router().await;
    let app = middleware::add_middleware_with_mock_peer(router, state);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/kernel-logs")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let rid = res.headers()["x-request-id"].to_str().unwrap().to_string();

    let out = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    let entry = out
        .lines()
        .filter_map(flame_kernel::domain::kernel_log::parse_line)
        .find(|e| e.message == "finished processing request")
        .unwrap_or_else(|| panic!("no completion event in:\n{out}"));
    assert_eq!(entry.request_id.as_deref(), Some(rid.as_str()));

    // 客户端自带 `x-request-id` 时沿用（便于上游网关 / 前端串联），回显到响应头并进入日志
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/kernel-logs")
        .header("x-request-id", "upstream-123")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.headers()["x-request-id"], "upstream-123");
    let out = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    assert!(out
        .lines()
        .filter_map(flame_kernel::domain::kernel_log::parse_line)
        .any(|e| e.message == "finished processing request"
            && e.request_id.as_deref() == Some("upstream-123")));
}
