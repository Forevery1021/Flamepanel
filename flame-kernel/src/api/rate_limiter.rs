use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::sync::OnceLock;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{Response, IntoResponse},
};

struct RateEntry {
    count: u64,
    window_start: Instant,
}

pub struct RateLimiter {
    max_requests: u64,
    window: Duration,
    clients: Mutex<HashMap<String, RateEntry>>,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            clients: Mutex::new(HashMap::new()),
        }
    }

    fn check(&self, client_ip: &str) -> bool {
        let mut clients = self.clients.lock().unwrap();
        let now = Instant::now();
        let entry = clients.entry(client_ip.to_string()).or_insert(RateEntry {
            count: 0,
            window_start: now,
        });
        if now.duration_since(entry.window_start) > self.window {
            entry.count = 0;
            entry.window_start = now;
        }
        entry.count += 1;
        entry.count <= self.max_requests
    }
}

static GLOBAL_LIMITER: OnceLock<RateLimiter> = OnceLock::new();

pub fn init_global_limiter(max_requests: u64, window_secs: u64) {
    let _ = GLOBAL_LIMITER.set(RateLimiter::new(max_requests, window_secs));
}

pub async fn rate_limit_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let limiter = GLOBAL_LIMITER.get_or_init(|| RateLimiter::new(120, 60));
    let ip = req.headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    if !limiter.check(ip) {
        return Ok(StatusCode::TOO_MANY_REQUESTS.into_response());
    }
    Ok(next.run(req).await)
}