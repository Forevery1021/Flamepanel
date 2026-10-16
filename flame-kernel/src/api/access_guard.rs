//! 面板访问控制（W4-a）：授权 IP / 绑定域名。
//!
//! 对标 1Panel「授权 IP / 绑定域名」与宝塔「授权 IP / 绑定域名」：策略来自面板设置
//! （`allowed_ips` / `bind_domains`，空 = 不限制），由 `SettingsService::policy` 缓存，
//! 每个请求只做一次读锁 + CIDR 比较，不查库。
//!
//! **自救通道**：TCP 对端是回环地址且请求未经反向代理转发（无 `X-Forwarded-For` /
//! `X-Real-IP`）时始终放行——本机 `fpctl api PUT /api/settings …` 可以随时改回配置。
//! 经本机反代进来的请求按转发头里的真实客户端判定，不享受豁免。
//!
//! `/health` 存活探针不受限制（负载均衡 / 监控探测）；`/metrics` 另有令牌保护。

use crate::api::types::AppState;
use crate::core::error::ErrorResponse;
use crate::domain::settings_schema::AccessDecision;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::net::{IpAddr, SocketAddr};

/// 稳定错误码：前端据此渲染「访问受限」页而非通用无权限提示。
pub const ACCESS_DENIED: &str = "ACCESS_DENIED";

/// 一次请求的来源（真实客户端 IP / 访问域名 / 是否本机直连）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestOrigin {
    pub ip: Option<IpAddr>,
    pub host: Option<String>,
    pub local: bool,
}

pub fn request_origin(peer: &SocketAddr, headers: &HeaderMap) -> RequestOrigin {
    let forwarded = headers.contains_key("x-forwarded-for") || headers.contains_key("x-real-ip");
    let peer_loopback = peer.ip().is_loopback()
        || matches!(peer.ip(), IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback()));
    let ip = crate::api::extract::extract_client_ip_trusted(Some(peer), headers)
        .parse::<IpAddr>()
        .ok();
    // 反代场景下 Host 常被改写为上游地址：仅信任回环对端带来的 X-Forwarded-Host
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let host = if peer_loopback {
        header("x-forwarded-host").or_else(|| header("host"))
    } else {
        header("host")
    };
    RequestOrigin {
        ip,
        host,
        local: peer_loopback && !forwarded,
    }
}

fn exempt(path: &str) -> bool {
    path == "/health" || path.starts_with("/health/")
}

pub async fn access_guard(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    if exempt(req.uri().path()) {
        return next.run(req).await;
    }
    let policy = state.settings_service.policy();
    if policy.allowed_ips.is_empty() && policy.bind_domains.is_empty() {
        return next.run(req).await;
    }
    let origin = request_origin(&peer, req.headers());
    match policy.check(origin.ip, origin.host.as_deref(), origin.local) {
        AccessDecision::Allow => next.run(req).await,
        decision => {
            let message = match decision {
                AccessDecision::DenyIp => format!(
                    "access denied: client IP {} is not in the panel allowlist",
                    origin
                        .ip
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| "unknown".into())
                ),
                _ => format!(
                    "access denied: host '{}' is not a bound panel domain",
                    origin.host.as_deref().unwrap_or("")
                ),
            };
            tracing::warn!(
                path = %req.uri().path(),
                ip = ?origin.ip,
                host = ?origin.host,
                "panel access denied by settings policy"
            );
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    code: 403,
                    error: ACCESS_DENIED,
                    message,
                }),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn peer(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    #[test]
    fn direct_loopback_is_local_but_proxied_loopback_is_not() {
        let mut h = HeaderMap::new();
        h.insert("host", HeaderValue::from_static("127.0.0.1:8080"));
        let o = request_origin(&peer("127.0.0.1:5000"), &h);
        assert!(o.local);

        h.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.9"));
        h.insert(
            "x-forwarded-host",
            HeaderValue::from_static("panel.example.com"),
        );
        let o = request_origin(&peer("127.0.0.1:5000"), &h);
        assert!(!o.local);
        assert_eq!(o.ip, Some("203.0.113.9".parse().unwrap()));
        assert_eq!(o.host.as_deref(), Some("panel.example.com"));
    }

    #[test]
    fn remote_peer_cannot_spoof_forwarding_headers() {
        let mut h = HeaderMap::new();
        h.insert("host", HeaderValue::from_static("1.2.3.4"));
        h.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1"));
        h.insert(
            "x-forwarded-host",
            HeaderValue::from_static("panel.example.com"),
        );
        let o = request_origin(&peer("198.51.100.7:443"), &h);
        assert!(!o.local);
        assert_eq!(o.ip, Some("198.51.100.7".parse().unwrap()));
        assert_eq!(o.host.as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn health_probe_is_exempt() {
        assert!(exempt("/health"));
        assert!(!exempt("/api/health"));
    }
}
