//! HTTP 服务公共层：鉴权令牌、限流（F-20）、共享 HTTP Client（F-29）、路由装配。

use super::*;

// 启动时由 main 初始化一次；各 handler 经常量时间比较校验 token。
pub(crate) static AUTH_TOKEN_CELL: OnceLock<String> = OnceLock::new();

// 文件读写白名单根目录。
pub(crate) static FILE_ROOT_CELL: OnceLock<PathBuf> = OnceLock::new();

// ─── F-20: Agent 限流 ─────────────────────────────────────────────
// 简单令牌桶：每个客户端 IP 每分钟最多 120 个请求（与面板限流对齐）。
pub(crate) const AGENT_RATE_LIMIT_PER_MIN: u64 = 120;

pub(crate) struct AgentRateLimiter {
    /// IP → (计数, 窗口起始时间)
    pub(crate) entries: Mutex<HashMap<String, (u64, u64)>>,
}

impl AgentRateLimiter {
    pub(crate) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// 检查并记录请求。返回 true 表示允许，false 表示被限流。
    pub(crate) fn check_and_count(&self, ip: &str) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // 锁中毒时取回内部数据继续（限流表丢一次更新无害，不能让 Agent panic）
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let entry = entries.entry(ip.to_string()).or_insert((0, now));

        // 清理过期的窗口
        if now >= entry.1 + 60 {
            entry.0 = 0;
            entry.1 = now;
        }
        // 检查限流
        if entry.0 >= AGENT_RATE_LIMIT_PER_MIN {
            return false;
        }
        entry.0 += 1;
        true
    }
}

pub(crate) static RATE_LIMITER: OnceLock<AgentRateLimiter> = OnceLock::new();

pub(crate) fn rate_limiter() -> &'static AgentRateLimiter {
    RATE_LIMITER.get_or_init(AgentRateLimiter::new)
}

// ─── 共享 HTTP Client（F-29）─────────────────────────────────────────────
// 进程级单例 reqwest::Client：连接池复用，避免每 10s 心跳/注册新建 Client 导致
// 每次新开 TCP/TLS 连接（无 keep-alive 复用）。
pub(crate) static SHARED_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub(crate) fn http_client() -> &'static reqwest::Client {
    SHARED_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            // 长连接池上限 4，心跳/注册/请求复用
            .pool_max_idle_per_host(4)
            // 默认 30s 连接超时（单个请求仍可覆盖）
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest::Client build failed")
    })
}

/// 常量时间字符串比较，避免基于比较时长的时序侧信道。
pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut diff: u8 = 0;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// 校验 `Authorization` 头。面板 `AgentClient` 发送标准的 `Bearer <token>`；
/// 历史上这里直接拿整个头值与 token 比较，导致面板 → Agent 的所有调用（远程命令 / 文件 /
/// 特权助手）在真实 Agent 上一律 401（内核单测用的是期望 `Bearer` 的 mock，未暴露）。
/// 现接受 `Bearer <token>`（scheme 大小写不敏感），并兼容裸 token 的旧调用方。
pub(crate) fn check_auth(header_value: &str) -> bool {
    let expected = AUTH_TOKEN_CELL.get().map(|s| s.as_str()).unwrap_or("");
    check_auth_against(header_value, expected)
}

fn check_auth_against(header_value: &str, expected: &str) -> bool {
    if expected.is_empty() {
        return false;
    }
    let v = header_value.trim();
    let token = match v.split_once(' ') {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("bearer") => rest.trim(),
        _ => v,
    };
    constant_time_eq(token, expected)
}

pub(crate) fn auth_error() -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "Unauthorized"})),
    )
}

/// F-20: Agent 全局限流中间件——基于客户端 IP 的简单限流与失败锁定。
pub(crate) async fn rate_limit_middleware(
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<SocketAddr>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let ip = peer.ip().to_string();
    if !rate_limiter().check_and_count(&ip) {
        return Err(axum::http::StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(request).await)
}

pub(crate) fn agent_routes() -> Router {
    // S4：统一走 `/action` 动作枚举，原始 `/exec` 任意命令端点已删除（不再提供）。
    Router::new()
        .route("/action", post(action_endpoint))
        .route("/files/list", get(list_files_endpoint))
        .route("/files/download", get(download_file_endpoint))
        .route("/files/upload", post(upload_file_endpoint))
        .layer(middleware::from_fn(rate_limit_middleware))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shared data only: this module is absent from non-test builds.
    mod constant_time_eq_vectors {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test-fixtures/security/constant_time_eq.rs"
        ));
    }

    #[test]
    fn constant_time_eq_shared_utf8_vectors() {
        for &(name, left, right, equal) in constant_time_eq_vectors::UTF8_CASES {
            assert_eq!(constant_time_eq(left, right), equal, "case: {name}");
            assert_eq!(constant_time_eq(right, left), equal, "reverse case: {name}");
            assert!(constant_time_eq(left, left), "reflexive case: {name}");
        }
    }

    #[test]
    fn constant_time_eq_every_ascii_byte_at_every_position() {
        let expected = "A".repeat(64);
        for position in 0..expected.len() {
            let mut candidate = expected.as_bytes().to_vec();
            for replacement in 0u8..=127 {
                candidate[position] = replacement;
                let candidate = std::str::from_utf8(&candidate).expect("ASCII is valid UTF-8");
                let equal = replacement == b'A';
                assert_eq!(
                    constant_time_eq(&expected, candidate),
                    equal,
                    "position: {position}, byte: {replacement}"
                );
                assert_eq!(
                    constant_time_eq(candidate, &expected),
                    equal,
                    "reverse position: {position}, byte: {replacement}"
                );
            }
        }
    }

    #[test]
    fn constant_time_eq_empty_inputs_do_not_authorize_unconfigured_agent() {
        // Comparator equality and the authentication policy are separate contracts.
        assert!(constant_time_eq("", ""));
        for header in ["", " ", "Bearer ", "bearer", "synthetic-key"] {
            assert!(!check_auth_against(header, ""));
        }
    }

    #[test]
    fn constant_time_eq_matches_only_identical_strings() {
        assert!(constant_time_eq("s3cret-token", "s3cret-token"));
        assert!(!constant_time_eq("s3cret-token", "s3cret-tokeN"));
        assert!(!constant_time_eq("short", "shorter"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn check_auth_accepts_bearer_and_legacy_bare_token() {
        assert!(check_auth_against("Bearer s3cret", "s3cret"));
        assert!(check_auth_against("bearer  s3cret ", "s3cret"));
        assert!(check_auth_against("s3cret", "s3cret"), "兼容裸 token");
        assert!(!check_auth_against("Bearer wrong", "s3cret"));
        assert!(!check_auth_against("Basic s3cret", "s3cret"));
        assert!(!check_auth_against("", "s3cret"));
        assert!(!check_auth_against("Bearer ", ""), "未配置 token 一律拒绝");
    }

    #[test]
    fn rate_limiter_caps_each_ip_independently() {
        let limiter = AgentRateLimiter::new();
        for _ in 0..AGENT_RATE_LIMIT_PER_MIN {
            assert!(limiter.check_and_count("10.0.0.1"));
        }
        assert!(!limiter.check_and_count("10.0.0.1"), "第 121 次应被限流");
        assert!(limiter.check_and_count("10.0.0.2"), "其他 IP 不受影响");
    }
}
