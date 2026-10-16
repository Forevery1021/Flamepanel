//! 项目健康探测器（M-B 主线二 Stage A-9 · Doc/20 §5.8 ④ 零停机切换）。
//!
//! 对新启动实例的 `http://127.0.0.1:{port}{health_check_path}` 发起轮询探测：
//! - HTTP 状态码 2xx/3xx 即视为健康（对齐 Dokku app.json checks 宽松语义）；
//! - 单次请求超时 [`HealthProbe::request_timeout`] 秒，总轮询窗口
//!   [`HealthProbe::deadline`] 秒，间隔 [`HealthProbe::interval_ms`] 毫秒；
//! - 仅回环地址、端口经项目白名单校验，探测结果只作流量切换依据，不落库。
//!
//! 端口 trait [`HealthProbePort`](crate::application::project_ports::HealthProbePort)
//! 声明于 application 层，本实现注入 [`ProjectService`](crate::application::project_service::ProjectService)。

use crate::application::project_ports::HealthProbePort;
use crate::core::error::AppError;
use crate::domain::entity::Project;
use async_trait::async_trait;
use std::time::Duration;

/// HTTP 健康探测器。
pub struct HttpHealthProbe {
    /// 单次请求超时（秒）
    pub request_timeout: u64,
    /// 总轮询窗口（秒）
    pub deadline: u64,
    /// 轮询间隔（毫秒）
    pub interval_ms: u64,
}

impl Default for HttpHealthProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpHealthProbe {
    pub fn new() -> Self {
        Self {
            request_timeout: 5,
            deadline: 60,
            interval_ms: 500,
        }
    }

    /// 覆盖轮询参数（测试可缩短窗口）。
    pub fn with_timeouts(mut self, deadline: u64, interval_ms: u64, request_timeout: u64) -> Self {
        self.deadline = deadline;
        self.interval_ms = interval_ms;
        self.request_timeout = request_timeout;
        self
    }

    /// 单次探测：2xx/3xx 通过。
    async fn probe_once(&self, url: &str) -> bool {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.request_timeout))
            .build();
        let client = match client {
            Ok(c) => c,
            Err(_) => return false,
        };
        match client.get(url).send().await {
            Ok(resp) => resp.status().is_success() || resp.status().is_redirection(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl HealthProbePort for HttpHealthProbe {
    /// 轮询探测直到健康或窗口耗尽。
    /// 返回 `Ok(true)` 健康 / `Ok(false)` 窗口内始终不健康（不报错——
    /// 超时不健康是零停机切换的预期分支，由服务层决定回滚）。
    async fn wait_healthy(&self, project: &Project) -> Result<bool, AppError> {
        let url = project
            .health_check_url()
            .ok_or_else(|| AppError::ValidationError("项目未配置健康检查路径或端口".into()))?;
        let deadline = std::time::Instant::now() + Duration::from_secs(self.deadline);
        loop {
            if self.probe_once(&url).await {
                return Ok(true);
            }
            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(self.interval_ms)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(port: i64, path: &str) -> Project {
        Project {
            id: 1,
            name: "blog".into(),
            kind: "node".into(),
            port: Some(port),
            health_check_path: path.into(),
            ..Project::default()
        }
    }

    #[tokio::test]
    async fn wait_healthy_requires_url() {
        let probe = HttpHealthProbe::new();
        // 无路径
        let err = probe.wait_healthy(&sample(3000, "")).await.unwrap_err();
        assert!(err.to_string().contains("健康检查"));
        // 无端口（port 为 None）
        let mut p = sample(0, "/health");
        p.port = None;
        let err = probe.wait_healthy(&p).await.unwrap_err();
        assert!(err.to_string().contains("健康检查"));
    }

    #[tokio::test]
    async fn wait_healthy_returns_false_when_unreachable() {
        // 回环保留端口，连接必失败 → 窗口耗尽返回 Ok(false)
        let probe = HttpHealthProbe::new().with_timeouts(1, 100, 1);
        let ok = probe.wait_healthy(&sample(1, "/health")).await.unwrap();
        assert!(!ok);
    }

    #[test]
    fn health_check_url_normalizes_path() {
        let p = sample(3000, "health");
        assert_eq!(
            p.health_check_url().as_deref(),
            Some("http://127.0.0.1:3000/health")
        );
        let p = sample(3000, "/healthz?probe=1");
        assert_eq!(
            p.health_check_url().as_deref(),
            Some("http://127.0.0.1:3000/healthz?probe=1")
        );
    }
}
