//! WAF 下发实现：规则 → 各 Web 引擎原生配置（`webserver::waf_engines`：Nginx / OpenResty /
//! Apache / Caddy / OpenLiteSpeed）→ `WebServerManager::deploy_waf`（原子写 + 校验 + reload +
//! 回滚；非 root 经特权助手）。

use crate::application::site_config_ports::SharedSiteConfigPort;
use crate::application::waf_ports::{
    WafApplyReport, WafEnforcementPort, WafEngineApply, WafSkippedRule,
};
use crate::core::error::AppError;
use crate::domain::entity::WafRule;
use crate::domain::repository::WebsiteRepository;
use crate::webserver::waf_engines::{self, WafSite};
use crate::webserver::{WebServerEngine, WebServerManager};
use async_trait::async_trait;
use std::sync::Arc;

pub struct NativeWafEnforcer {
    manager: WebServerManager,
    website_repo: Arc<dyn WebsiteRepository>,
    /// 站点重部署（接入 WAF include）；未注入时只报告不重部署。
    site_writer: Option<SharedSiteConfigPort>,
}

impl NativeWafEnforcer {
    pub fn new(manager: WebServerManager, website_repo: Arc<dyn WebsiteRepository>) -> Self {
        Self {
            manager,
            website_repo,
            site_writer: None,
        }
    }

    pub fn with_site_writer(mut self, writer: SharedSiteConfigPort) -> Self {
        self.site_writer = Some(writer);
        self
    }
}

const ENGINES: [WebServerEngine; 5] = [
    WebServerEngine::Nginx,
    WebServerEngine::OpenResty,
    WebServerEngine::Apache,
    WebServerEngine::Caddy,
    WebServerEngine::OpenLiteSpeed,
];

#[async_trait]
impl WafEnforcementPort for NativeWafEnforcer {
    async fn apply(&self, rules: &[WafRule]) -> Result<WafApplyReport, AppError> {
        let websites = self.website_repo.list_all().await?;
        let mut engines = Vec::new();
        for engine in ENGINES {
            let mut report = WafEngineApply {
                engine: engine.as_str().to_string(),
                applied: false,
                reloaded: false,
                enforced: Vec::new(),
                skipped: Vec::new(),
                redeployed_sites: Vec::new(),
                reason: None,
                error: None,
            };
            if !self.manager.can_deploy(&engine).await {
                report.reason = Some(format!(
                    "{} is not installed or its config directory is not writable by the panel",
                    engine.as_str()
                ));
                engines.push(report);
                continue;
            }
            let on_engine: Vec<_> = websites
                .iter()
                .filter(|w| WebServerEngine::from_name(&w.engine).as_ref() == Some(&engine))
                .collect();
            let sites: Vec<WafSite> = on_engine
                .iter()
                .map(|w| WafSite {
                    website_id: w.id,
                    domains: vec![w.domain.clone()],
                })
                .collect();
            let layout = self.manager.layout(&engine).await;
            let compiled = waf_engines::compile(rules, &sites, &layout);
            report.enforced = compiled.enforced.clone();
            report.skipped = compiled
                .skipped
                .iter()
                .map(|(id, r)| WafSkippedRule {
                    rule_id: *id,
                    reason: r.clone(),
                })
                .collect();
            match self.manager.deploy_waf(&engine, &compiled).await {
                Ok(reloaded) => {
                    report.applied = true;
                    report.reloaded = reloaded;
                }
                Err(e) => {
                    report.error = Some(e.to_string());
                    engines.push(report);
                    continue;
                }
            }
            // 旧版本渲染的站点没有站点级 include：重新部署一次（尽力而为；OLS 由 deploy_waf 直接拼接）
            if let Some(writer) = &self.site_writer {
                for w in on_engine {
                    if self.manager.site_has_waf_include(&engine, &w.domain).await == Some(false) {
                        match writer.deploy_site(w).await {
                            Ok(_) => report.redeployed_sites.push(w.domain.clone()),
                            Err(e) => tracing::warn!(
                                domain = %w.domain,
                                "WAF: redeploying site to add the WAF include failed: {e}"
                            ),
                        }
                    }
                }
            }
            engines.push(report);
        }
        Ok(WafApplyReport {
            ok: engines.iter().all(|e| e.error.is_none()),
            engines,
        })
    }
}
