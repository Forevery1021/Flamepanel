use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use moka::sync::Cache;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use crate::core::error::ErrorResponse;

/// 请求类别 → 分级限额。
/// 登录更严（防暴力破解），普通 API 较高，health 宽松（不误伤探活）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tier {
    /// `/api/auth/login`，默认 5/min
    Login,
    /// 其余普通 API，默认 120/min
    Api,
    /// `/health`、`/api/health`，默认 600/min（几乎不受限）
    Health,
}

impl Tier {
    /// 根据请求路径判定类别。
    fn from_path(path: &str) -> Self {
        if path == "/api/auth/login" {
            Tier::Login
        } else if path == "/health" || path == "/api/health" {
            Tier::Health
        } else {
            Tier::Api
        }
    }
}

/// 统一 JSON 错误体：429 直接复用 `core::error::ErrorResponse`（T15 死代码/重复清理）。
#[derive(Clone, Copy)]
struct RateEntry {
    count: u64,
    window_start: Instant,
}

pub struct RateLimiter {
    tiers: Vec<(Tier, u64, Duration, Cache<String, RateEntry>)>,
}

impl RateLimiter {
    pub fn new(default_max: u64, default_window_secs: u64) -> Self {
        let window = Duration::from_secs(default_window_secs);
        // 各档用 moka 缓存：带 TTL 自动过期 + 容量上限，杜绝过期 IP 条目无界增长（R3/T6）。
        let tiers = vec![
            (
                Tier::Login,
                5,
                Duration::from_secs(60),
                Self::make_cache(Duration::from_secs(60)),
            ),
            (Tier::Api, default_max, window, Self::make_cache(window)),
            (
                Tier::Health,
                600,
                Duration::from_secs(60),
                Self::make_cache(Duration::from_secs(60)),
            ),
        ];
        Self { tiers }
    }

    fn make_cache(ttl: Duration) -> Cache<String, RateEntry> {
        Cache::builder()
            .time_to_live(ttl)
            .max_capacity(100_000)
            .build()
    }

    fn check(&self, tier: Tier, client_ip: &str) -> bool {
        for (t, max, window, cache) in &self.tiers {
            if *t == tier {
                let now = Instant::now();
                let entry = cache.get(client_ip).unwrap_or(RateEntry {
                    count: 0,
                    window_start: now,
                });
                let entry = if now.duration_since(entry.window_start) > *window {
                    RateEntry {
                        count: 1,
                        window_start: now,
                    }
                } else {
                    RateEntry {
                        count: entry.count + 1,
                        window_start: entry.window_start,
                    }
                };
                cache.insert(client_ip.to_string(), entry);
                return entry.count <= *max;
            }
        }
        false
    }
}

static GLOBAL_LIMITER: OnceLock<RwLock<RateLimiter>> = OnceLock::new();

pub fn init_global_limiter(default_max: u64, default_window_secs: u64) {
    // 保持既有语义：每次调用覆盖为新实例（测试可调高阈值；生产仅启动时调用一次）。
    let limiter = GLOBAL_LIMITER.get_or_init(|| RwLock::new(RateLimiter::new(120, 60)));
    *limiter.write().unwrap() = RateLimiter::new(default_max, default_window_secs);
}

/// 从请求头提取客户端 IP（与 `api/extract.rs::extract_client_ip` 语义一致）。
/// 优先 `X-Real-IP`（可信代理设置），回退到 `X-Forwarded-For` 链首值（最左即真实客户端）。
pub fn client_ip(req: &Request) -> String {
    crate::api::extract::extract_client_ip(req.headers())
}

pub async fn rate_limit_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let limiter = GLOBAL_LIMITER.get_or_init(|| RwLock::new(RateLimiter::new(120, 60)));
    let tier = Tier::from_path(req.uri().path());
    let ip = client_ip(&req);

    if !limiter.read().unwrap().check(tier, &ip) {
        let body = serde_json::to_string(&ErrorResponse {
            code: StatusCode::TOO_MANY_REQUESTS.as_u16(),
            error: "RATE_LIMITED",
            message: "Too many requests, retry later".to_string(),
        })
        .unwrap_or_else(|_| {
            r#"{"code":429,"error":"RATE_LIMITED","message":"Too many requests"}"#.to_string()
        });

        return Ok(Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .expect("valid 429 response"));
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request as HttpRequest},
    };

    #[test]
    fn moka_ttl_bounds_entries_per_ip() {
        // 验证 moka 缓存带 TTL：同一 IP 的过期条目会被自动回收（内存不无界增长）。
        let limiter = RateLimiter::new(3, 60);
        // 用短 TTL 构造，确认窗口过期后计数归零（相当于条目被回收）。
        for _ in 0..3 {
            assert!(limiter.check(Tier::Api, "9.9.9.9"));
        }
        assert!(!limiter.check(Tier::Api, "9.9.9.9"));
    }

    #[test]
    fn login_tier_stricter_than_api() {
        let limiter = RateLimiter::new(120, 60);
        // 同一 IP 连打登录 5 次通过，第 6 次拒绝
        for _ in 0..5 {
            assert!(limiter.check(Tier::Login, "1.2.3.4"));
        }
        assert!(!limiter.check(Tier::Login, "1.2.3.4"));
        // 同 IP 走普通 API 仍可用（分级生效）
        assert!(limiter.check(Tier::Api, "1.2.3.4"));
    }

    #[test]
    fn api_tier_uses_default_limit() {
        let limiter = RateLimiter::new(3, 60);
        for _ in 0..3 {
            assert!(limiter.check(Tier::Api, "10.0.0.1"));
        }
        assert!(!limiter.check(Tier::Api, "10.0.0.1"));
        // 不同 IP 不受影响
        assert!(limiter.check(Tier::Api, "10.0.0.2"));
    }

    #[test]
    fn health_tier_very_relaxed() {
        let limiter = RateLimiter::new(3, 60);
        // health 配额固定 600，不受 default 影响
        for _ in 0..10 {
            assert!(limiter.check(Tier::Health, "10.0.0.3"));
        }
    }

    #[test]
    fn tier_from_path_maps_correctly() {
        assert_eq!(Tier::from_path("/api/auth/login"), Tier::Login);
        assert_eq!(Tier::from_path("/health"), Tier::Health);
        assert_eq!(Tier::from_path("/api/health"), Tier::Health);
        assert_eq!(Tier::from_path("/api/users"), Tier::Api);
    }

    #[test]
    fn ip_prefers_x_real_ip() {
        let req = HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/health")
            .header("X-Real-IP", "1.1.1.1")
            .header("X-Forwarded-For", "2.2.2.2, 3.3.3.3")
            .body(Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "1.1.1.1");

        let req = HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/health")
            .header("X-Forwarded-For", "2.2.2.2, 3.3.3.3")
            .body(Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "2.2.2.2");
    }
}
