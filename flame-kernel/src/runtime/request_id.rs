//! Request-Id：生成 / 传播 `x-request-id`，并注入 tracing span 字段实现日志串联。
//!
//! - 请求进入时：优先沿用上游 `x-request-id`（反向代理链路），否则生成新的 UUID；
//! - 响应返回时：`x-request-id` 写回响应头；
//! - 同一请求的所有日志通过 `http_request` span 的 `request_id` 字段串联；
//! - 认证中间件解析出 `user_id` 后，再注入 `auth` 子 span 的 `user_id` / `username` 字段。

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use tracing::Instrument;

/// `x-request-id` 请求/响应头名称。
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// 生成的请求 ID 存入 request extensions 的键（供 handler 读取）。
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// 生成全局唯一的请求 ID（UUID v4 简化形式）。
pub fn generate_request_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// 全局请求 ID 中间件（作为最外层 layer）：
/// 生成/传播 `x-request-id`，并以 `request_id` 为字段开启 `http_request` span。
pub async fn request_id_middleware(mut req: Request, next: Next) -> axum::response::Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_request_id);

    req.extensions_mut().insert(RequestId(request_id.clone()));

    let method = req.method().to_string();
    let uri = req.uri().path().to_string();
    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        uri = %uri,
    );

    let mut response = next.run(req).instrument(span).await;

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use axum::middleware;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    async fn ping() -> &'static str {
        "pong"
    }

    #[tokio::test]
    async fn request_id_generated_and_returned() {
        let app = Router::new()
            .route("/", get(ping))
            .layer(middleware::from_fn(request_id_middleware));

        let res = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let rid = res
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("x-request-id must be set")
            .to_str()
            .unwrap()
            .to_string();
        assert!(!rid.is_empty());
    }

    #[tokio::test]
    async fn request_id_propagated_from_upstream() {
        let app = Router::new()
            .route("/", get(ping))
            .layer(middleware::from_fn(request_id_middleware));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "upstream-abc-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get(REQUEST_ID_HEADER).unwrap(),
            "upstream-abc-123",
            "upstream request-id must be honored"
        );
    }

    #[tokio::test]
    async fn request_id_unique_across_requests() {
        let app = Router::new()
            .route("/", get(ping))
            .layer(middleware::from_fn(request_id_middleware));

        let res1 = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let res2 = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let id1 = res1
            .headers()
            .get(REQUEST_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let id2 = res2
            .headers()
            .get(REQUEST_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_ne!(id1, id2);
        let _ = header::AUTHORIZATION; // silence unused import under some cfgs
    }
}
