//! 系统体检服务（S-17 · Doc/30 M-C）：采集 → 判定 → 报告，带 TTL 缓存与单飞。
//!
//! - `GET /api/inspection` 读缓存（缺省 10 min），无缓存时同步执行一次——安装后首次打开
//!   仪表盘 / 体检页即完成"安装后检测"；
//! - `POST /api/inspection/run` 强制重跑；
//! - 缓存锁在整个采集期间持有：并发请求等待同一次采集结果，不会各自 spawn 一轮探测命令。
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::Mutex;

use crate::application::settings_service::SettingsService;
use crate::domain::inspection::{build_report, FactsSource, InspectionReport, PanelFacts};
use crate::infrastructure::metrics::MetricsHub;

/// 缺省缓存时长
pub const DEFAULT_INSPECTION_TTL: Duration = Duration::from_secs(600);

struct CachedReport {
    report: Arc<InspectionReport>,
    at: Instant,
}

pub struct InspectionService {
    /// 事实来源（可在测试中替换为固定事实）
    source: RwLock<Arc<dyn FactsSource>>,
    settings: Arc<SettingsService>,
    metrics: Arc<MetricsHub>,
    metrics_token_set: bool,
    ttl: Duration,
    cache: Mutex<Option<CachedReport>>,
}

impl InspectionService {
    pub fn new(
        source: Arc<dyn FactsSource>,
        settings: Arc<SettingsService>,
        metrics: Arc<MetricsHub>,
        metrics_token_set: bool,
    ) -> Self {
        Self {
            source: RwLock::new(source),
            settings,
            metrics,
            metrics_token_set,
            ttl: DEFAULT_INSPECTION_TTL,
            cache: Mutex::new(None),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// 替换事实来源（集成测试注入 `StaticFactsSource`），同时清空缓存。
    pub fn set_source(&self, source: Arc<dyn FactsSource>) {
        if let Ok(mut guard) = self.source.write() {
            *guard = source;
        }
        if let Ok(mut cache) = self.cache.try_lock() {
            *cache = None;
        }
    }

    fn current_source(&self) -> Arc<dyn FactsSource> {
        match self.source.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// 取报告：`force = false` 时命中 TTL 内缓存（`cache_age_secs` 为缓存年龄），否则重新采集。
    pub async fn report(&self, force: bool) -> InspectionReport {
        let mut guard = self.cache.lock().await;
        if !force {
            if let Some(cached) = guard.as_ref() {
                let age = cached.at.elapsed();
                if age < self.ttl {
                    let mut report = (*cached.report).clone();
                    report.cache_age_secs = age.as_secs();
                    return report;
                }
            }
        }
        let report = Arc::new(self.run().await);
        *guard = Some(CachedReport {
            report: report.clone(),
            at: Instant::now(),
        });
        (*report).clone()
    }

    /// 仅读缓存（不触发采集）；仪表盘横幅等"有则显示"的场景使用。
    pub async fn cached(&self) -> Option<InspectionReport> {
        let guard = self.cache.lock().await;
        guard.as_ref().map(|c| {
            let mut report = (*c.report).clone();
            report.cache_age_secs = c.at.elapsed().as_secs();
            report
        })
    }

    async fn run(&self) -> InspectionReport {
        let started = Instant::now();
        let panel = self.panel_facts().await;
        let source = self.current_source();
        let mut facts = source.collect(panel).await;
        // 根分区使用率复用指标中枢的最新快照（与仪表盘同口径，不再 statvfs 一次）
        facts.root_disk_usage_percent = self.metrics.latest().map(|s| s.disk_usage_percent);
        let duration_ms = started.elapsed().as_millis() as u64;
        let report = build_report(&facts, Utc::now(), duration_ms);
        tracing::info!(
            score = report.summary.score,
            failed = report.summary.failed,
            unknown = report.summary.unknown,
            duration_ms,
            "system inspection completed"
        );
        report
    }

    async fn panel_facts(&self) -> PanelFacts {
        let map = self.settings.get_all_map().await.unwrap_or_default();
        PanelFacts {
            session_timeout_minutes: map
                .get("session_timeout_minutes")
                .and_then(|v| v.trim().parse().ok()),
            metrics_token_set: self.metrics_token_set,
            running_as_root: Some(crate::infrastructure::os::is_root_process()),
            panel_port: map.get("panel_port").and_then(|v| v.trim().parse().ok()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::inspection::{CheckStatus, StaticFactsSource, SystemFacts};
    use crate::infrastructure::db::InMemorySettingsRepository;

    fn service(ttl: Duration) -> InspectionService {
        let settings = Arc::new(SettingsService::new(Arc::new(
            InMemorySettingsRepository::new(),
        )));
        InspectionService::new(
            Arc::new(StaticFactsSource(SystemFacts {
                arch: "x86_64".into(),
                osrelease: Some("6.1.0-25-amd64".into()),
                ..Default::default()
            })),
            settings,
            Arc::new(MetricsHub::new(8)),
            true,
        )
        .with_ttl(ttl)
    }

    #[tokio::test]
    async fn report_is_cached_within_ttl_and_force_reruns() {
        let svc = service(Duration::from_secs(60));
        assert!(svc.cached().await.is_none());

        let first = svc.report(false).await;
        assert_eq!(first.cache_age_secs, 0);
        assert_eq!(first.kernel.release, "6.1.0-25-amd64");
        assert!(first.summary.total > 10);

        let second = svc.report(false).await;
        assert_eq!(second.generated_at, first.generated_at, "TTL 内命中缓存");
        assert!(svc.cached().await.is_some());

        let forced = svc.report(true).await;
        assert!(forced.generated_at >= first.generated_at);
        assert_eq!(forced.cache_age_secs, 0);
    }

    #[tokio::test]
    async fn expired_ttl_triggers_recollection() {
        let svc = service(Duration::ZERO);
        let first = svc.report(false).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        let second = svc.report(false).await;
        assert!(second.generated_at > first.generated_at);
    }

    #[tokio::test]
    async fn panel_facts_come_from_settings_and_source_can_be_swapped() {
        let svc = service(Duration::from_secs(60));
        svc.settings
            .set("session_timeout_minutes", "10080")
            .await
            .unwrap();
        svc.settings.set("panel_port", "9090").await.unwrap();
        let report = svc.report(false).await;
        let timeout = report
            .findings
            .iter()
            .find(|f| f.code == "panel_session_timeout")
            .unwrap();
        assert_eq!(timeout.status, CheckStatus::Fail);
        assert!(timeout.evidence.contains("10080"));

        svc.set_source(Arc::new(StaticFactsSource(SystemFacts {
            osrelease: Some("5.4.0-150-custom".into()),
            ..Default::default()
        })));
        let swapped = svc.report(false).await;
        assert_eq!(swapped.kernel.release, "5.4.0-150-custom");
        assert_eq!(swapped.cache_age_secs, 0, "换源后缓存已清空");
    }
}
