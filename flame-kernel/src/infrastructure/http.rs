//! 进程级共享 HTTP 客户端（连接池 / TLS 会话复用）。
//!
//! `reqwest::Client` 内部持有连接池与 TLS 配置，每次 `Client::new()` 都会
//! 重建两者；下载器 / 远程源等「偶发出站」调用方应复用本实例，避免
//! 连续请求时反复做 TLS 初始化与握手。需要专用配置（代理、统一超时、
//! 自定义 UA）的域（AgentClient / ACME / 健康探针）仍自持 Client，不收编。

use std::sync::OnceLock;

static SHARED: OnceLock<reqwest::Client> = OnceLock::new();

/// 全局共享 `reqwest::Client`（懒初始化，进程生命周期）。
///
/// 超时策略由调用方按请求粒度指定（`RequestBuilder::timeout`），
/// 本客户端不设全局超时，避免不同场景（60s 索引 / 600s 发行包）互相牵制。
pub fn shared_client() -> &'static reqwest::Client {
    SHARED.get_or_init(reqwest::Client::new)
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_client_returns_same_instance() {
        let a = super::shared_client() as *const reqwest::Client;
        let b = super::shared_client() as *const reqwest::Client;
        assert_eq!(a, b, "应为同一实例（连接池复用）");
    }
}
