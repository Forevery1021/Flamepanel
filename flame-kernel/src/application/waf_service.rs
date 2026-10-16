//! WAF 服务编排（主线三 §6.2 · WAF 基础防护）。
//!
//! 职责：规则 CRUD 校验 + 引擎实例管理（规则变更后重建）+ 拦截日志登记与查询，
//! 以及**拦截通知**——阻断命中发布 `WafBlocked` 事件（按规则节流聚合，
//! 经 Outbox 存档与邮件渠道下发）。
//!
//! 评估引擎（`webserver::waf::WafEngine`）为纯逻辑，此处只做生命周期编排，
//! 站点流量的实际拦截由 Web 引擎完成：注入 `WafEnforcementPort` 后规则变更自动编译下发
//! 为各 Web 引擎原生配置（Nginx / OpenResty / Apache / Caddy / OpenLiteSpeed，
//! `webserver::waf_engines`；Doc/32 W3 · Doc/30 v1.1）。
use crate::application::waf_ports::{EngineHit, WafIngestStatus};
use crate::core::error::AppError;
use crate::domain::entity::{DomainEvent, WafLog, WafRule, WAF_ACTIONS, WAF_RULE_TYPES};
use crate::domain::repository::{WafLogRepository, WafRuleRepository};
use crate::event::EventBus;
use crate::utils::sync::lock_or_recover;
use crate::webserver::waf::{WafEngine, WafRequest, WafVerdict};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 拦截通知默认节流窗口（秒）：同一规则窗口内重复命中只累计不通知，
/// 命中次数并在下一次通知中透出（防攻击期间告警风暴淹没收件箱）。
pub const DEFAULT_BLOCK_NOTIFY_WINDOW_SECS: u64 = 300;

/// 拦截通知节流器：按规则维度限频 + 命中聚合。
///
/// `record` 语义：
/// - `Some(n)`：应发布通知，`n` 为**自上次通知以来**该规则的累计命中次数；
/// - `None`：窗口内重复命中，仅累计不通知。
///
/// 首次命中立即通知；窗口期满后的首次命中把窗口内累计次数一并透出，
/// 保证「首个异常请求可达」与「持续攻击不刷屏」两者兼顾。
struct BlockThrottle {
    window: Duration,
    entries: Mutex<HashMap<i64, ThrottleEntry>>,
}

#[derive(Debug, Clone, Copy)]
struct ThrottleEntry {
    /// 最近一次发布通知的时刻（条目创建时回拨一个窗口，使首次命中立即通知）
    last_notified: Instant,
    /// 自上次通知以来的累计命中次数（含触发当次）
    pending: u32,
}

impl BlockThrottle {
    fn new(window: Duration) -> Self {
        Self {
            window,
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn record(&self, rule_id: i64, now: Instant) -> Option<u32> {
        let mut entries = lock_or_recover(&self.entries);
        // 简单自愈：按规则维度建条目，异常膨胀时整体清空（节流为尽力而为语义，清空安全）
        if entries.len() > 10_000 {
            entries.clear();
        }
        let entry = entries.entry(rule_id).or_insert_with(|| ThrottleEntry {
            last_notified: now.checked_sub(self.window).unwrap_or(now),
            pending: 0,
        });
        entry.pending = entry.pending.saturating_add(1);
        if now.duration_since(entry.last_notified) >= self.window {
            let count = entry.pending;
            entry.last_notified = now;
            entry.pending = 0;
            Some(count)
        } else {
            None
        }
    }
}

pub struct WafService {
    rule_repo: Arc<dyn WafRuleRepository>,
    log_repo: Arc<dyn WafLogRepository>,
    /// 进程内引擎实例：规则变更（创建/更新/删除）后整体重建。
    engine: RwLock<Arc<WafEngine>>,
    /// 事件总线（组合根注入）：阻断命中经此发布 `WafBlocked`
    /// （Outbox 存档 + 邮件渠道下发）；未注入时仅记日志不发通知。
    event_bus: Option<EventBus>,
    /// 拦截通知节流（按规则维度；窗口可注入，默认 300s）
    block_notify: BlockThrottle,
    /// Web 引擎下发端口（Doc/32 W3）：规则变更后编译为原生 Nginx 配置；未注入时仅进程内生效
    enforcer: Option<crate::application::waf_ports::SharedWafEnforcementPort>,
    /// Web 引擎命中日志回灌状态（按引擎；由后台同步任务更新）
    ingest: std::sync::Mutex<std::collections::BTreeMap<String, WafIngestStatus>>,
}

impl WafService {
    pub fn new(rule_repo: Arc<dyn WafRuleRepository>, log_repo: Arc<dyn WafLogRepository>) -> Self {
        Self {
            rule_repo,
            log_repo,
            engine: RwLock::new(Arc::new(WafEngine::new(Vec::new()))),
            event_bus: None,
            block_notify: BlockThrottle::new(Duration::from_secs(DEFAULT_BLOCK_NOTIFY_WINDOW_SECS)),
            enforcer: None,
            ingest: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }

    /// 注入 Web 引擎下发端口（规则变更后自动下发；`apply` 手动下发）。
    pub fn with_enforcer(
        mut self,
        enforcer: crate::application::waf_ports::SharedWafEnforcementPort,
    ) -> Self {
        self.enforcer = Some(enforcer);
        self
    }

    /// 手动下发（`POST /api/waf/apply`）：未注入端口返回 503；单引擎失败记入报告。
    pub async fn apply(&self) -> Result<crate::application::waf_ports::WafApplyReport, AppError> {
        let enforcer = self.enforcer.as_ref().ok_or_else(|| {
            AppError::ServiceUnavailable("WAF web engine enforcement is not configured".into())
        })?;
        let rules = self.rule_repo.list_enabled().await?;
        enforcer.apply(&rules).await
    }

    /// 是否注入了 Web 引擎下发端口。
    pub fn enforcement_configured(&self) -> bool {
        self.enforcer.is_some()
    }

    /// 是否存在启用的网站级规则（网站集合变化时才需要重新下发）。
    pub async fn has_site_scoped_rules(&self) -> Result<bool, AppError> {
        Ok(self
            .rule_repo
            .list_enabled()
            .await?
            .iter()
            .any(|r| r.website_id != 0))
    }

    /// 网站集合变化后的重新下发（后台同步任务调用；尽力而为）。
    pub async fn reapply_best_effort(&self) {
        self.enforce_best_effort().await;
    }

    /// 回灌 Web 引擎命中：每条命中规则写一条拦截日志，阻断命中走拦截通知（节流）。
    ///
    /// 规则按当前仓储快照解析名称 / 类型 / 动作；白名单命中不记录（与进程内引擎一致）；
    /// 已删除的规则以 `#<id>` 记名。返回写入条数。
    pub async fn ingest_engine_hits(&self, hits: &[EngineHit]) -> Result<usize, AppError> {
        use crate::domain::entity::truncate_summary;
        if hits.is_empty() {
            return Ok(0);
        }
        let rules: HashMap<i64, WafRule> = self
            .rule_repo
            .list_all(None)
            .await?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        let mut written = 0;
        for hit in hits {
            let entries = hit
                .rules
                .iter()
                .map(|id| (*id, false))
                .chain(hit.cc_rules.iter().map(|id| (*id, true)));
            for (rule_id, is_cc) in entries {
                let rule = rules.get(&rule_id);
                if rule.is_some_and(|r| r.rule_type.ends_with("_whitelist")) {
                    continue;
                }
                let (name, rule_type, action) = match (rule, is_cc) {
                    (Some(r), true) => (r.name.clone(), "cc".to_string(), "block".to_string()),
                    (Some(r), false) => (r.name.clone(), r.rule_type.clone(), r.action.clone()),
                    (None, true) => (format!("#{rule_id}"), "cc".into(), "block".into()),
                    (None, false) => (
                        format!("#{rule_id}"),
                        "unknown".into(),
                        if hit.blocked { "block" } else { "log" }.into(),
                    ),
                };
                let log = WafLog {
                    id: 0,
                    website_id: rule
                        .map(|r| r.website_id)
                        .filter(|w| *w != 0)
                        .unwrap_or(hit.website_id),
                    rule_id,
                    rule_name: truncate_summary(&name, 100),
                    rule_type,
                    action,
                    client_ip: truncate_summary(&hit.client_ip, 64),
                    request_summary: truncate_summary(&format!("{} {}", hit.method, hit.uri), 200),
                    user_agent: truncate_summary(&hit.user_agent, 200),
                    created_at: hit.time.unwrap_or_else(chrono::Utc::now),
                };
                self.log_repo.create(&log).await?;
                written += 1;
                if log.action == "block" {
                    self.notify_blocked(&log).await;
                }
            }
        }
        Ok(written)
    }

    /// 更新某引擎的回灌状态（后台同步任务调用）。
    pub fn set_ingest_status(&self, status: WafIngestStatus) {
        if let Ok(mut m) = self.ingest.lock() {
            m.insert(status.engine.clone(), status);
        }
    }

    /// 各引擎回灌状态（按引擎名排序）。
    pub fn ingest_status(&self) -> Vec<WafIngestStatus> {
        self.ingest
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// 规则变更后的自动下发：尽力而为（失败只记日志，规则已落库；可经 `apply` 查看原因重试）。
    async fn enforce_best_effort(&self) {
        if self.enforcer.is_none() {
            return;
        }
        match self.apply().await {
            Ok(report) => {
                for e in report.engines.iter().filter(|e| e.error.is_some()) {
                    tracing::warn!(
                        engine = %e.engine,
                        "WAF rules saved but enforcement failed: {}",
                        e.error.as_deref().unwrap_or("")
                    );
                }
            }
            Err(e) => tracing::warn!("WAF enforcement failed: {e}"),
        }
    }

    /// 注入事件总线：阻断命中发布 `WafBlocked` 事件（§6.2 拦截通知）。
    pub fn with_event_bus(mut self, bus: EventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// 覆盖拦截通知节流窗口（测试用；生产默认 `DEFAULT_BLOCK_NOTIFY_WINDOW_SECS`）。
    pub fn with_block_notify_window(mut self, secs: u64) -> Self {
        self.block_notify = BlockThrottle::new(Duration::from_secs(secs));
        self
    }

    /// 启动时从仓储加载启用规则构建引擎（组合根后台调用）。
    pub async fn reload_engine(&self) -> Result<usize, AppError> {
        let rules = self.rule_repo.list_enabled().await?;
        let count = rules.len();
        *self.engine.write().await = Arc::new(WafEngine::new(rules));
        Ok(count)
    }

    pub async fn list_rules(&self, website_id: Option<i64>) -> Result<Vec<WafRule>, AppError> {
        self.rule_repo.list_all(website_id).await
    }

    pub async fn get_rule(&self, id: i64) -> Result<WafRule, AppError> {
        self.rule_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("WAF 规则不存在: {}", id)))
    }

    pub async fn create_rule(&self, mut rule: WafRule) -> Result<WafRule, AppError> {
        validate_rule(&rule)?;
        rule.id = 0;
        let id = self.rule_repo.create(&rule).await?;
        rule.id = id;
        self.reload_engine().await?;
        self.enforce_best_effort().await;
        Ok(rule)
    }

    pub async fn update_rule(&self, rule: &WafRule) -> Result<WafRule, AppError> {
        validate_rule(rule)?;
        let existing = self.get_rule(rule.id).await?;
        let mut updated = rule.clone();
        updated.created_at = existing.created_at;
        self.rule_repo.update(&updated).await?;
        self.reload_engine().await?;
        self.enforce_best_effort().await;
        Ok(updated)
    }

    pub async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        self.get_rule(id).await?;
        self.rule_repo.delete(id).await?;
        // 拦截日志保留（规则名快照仍在），引擎重建 + 重新下发
        self.reload_engine().await?;
        self.enforce_best_effort().await;
        Ok(())
    }

    /// 评估请求并登记日志（log 动作记 Warn 日志，block 动作记 Block 日志）。
    /// 返回是否放行（false = 应阻断）。
    pub async fn inspect(&self, website_id: i64, req: &WafRequest) -> Result<bool, AppError> {
        let engine = self.engine.read().await.clone();
        let verdict = engine.evaluate_for_site(website_id, req);
        let (rule_id, rule_name, rule_type, action) = match verdict {
            WafVerdict::Allow => return Ok(true),
            WafVerdict::Warn {
                rule_id,
                rule_name,
                rule_type,
            } => (rule_id, rule_name, rule_type, "log".to_string()),
            WafVerdict::Block {
                rule_id,
                rule_name,
                rule_type,
            } => (rule_id, rule_name, rule_type, "block".to_string()),
        };
        let log = WafLog {
            id: 0,
            website_id,
            rule_id,
            rule_name: crate::domain::entity::truncate_summary(&rule_name, 100),
            rule_type,
            action,
            client_ip: req.client_ip.clone(),
            request_summary: crate::domain::entity::truncate_summary(
                &format!("{} {}", req.method, req.path),
                200,
            ),
            user_agent: crate::domain::entity::truncate_summary(&req.user_agent, 200),
            created_at: chrono::Utc::now(),
        };
        // 日志写入失败不阻断主流程（评估结论优先返回）
        if let Err(e) = self.log_repo.create(&log).await {
            tracing::warn!("WAF log write failed: {e}");
        }
        // log 动作放行（仅告警），block 动作阻断
        let should_block = log.action == "block";
        if should_block {
            // 拦截通知：仅针对真实阻断命中（log 动作量大且无阻断语义，不进通知）
            self.notify_blocked(&log).await;
        }
        Ok(!should_block)
    }

    /// 发布 `WafBlocked` 拦截通知（按规则节流聚合）。
    ///
    /// 未注入事件总线（裁剪装配/单测）时静默跳过；发布失败仅告警，
    /// 不影响评估结论与请求处理（通知为旁路副作用）。
    async fn notify_blocked(&self, log: &WafLog) {
        let Some(bus) = &self.event_bus else {
            return;
        };
        let Some(hit_count) = self.block_notify.record(log.rule_id, Instant::now()) else {
            return;
        };
        if let Err(e) = bus
            .publish(DomainEvent::WafBlocked {
                website_id: log.website_id,
                rule_id: log.rule_id,
                rule_name: log.rule_name.clone(),
                rule_type: log.rule_type.clone(),
                client_ip: log.client_ip.clone(),
                request_summary: log.request_summary.clone(),
                hit_count,
            })
            .await
        {
            tracing::warn!("WAF block notification publish failed: {e}");
        }
    }

    pub async fn list_logs_paginated(
        &self,
        page: i64,
        page_size: i64,
        website_id: Option<i64>,
    ) -> Result<(Vec<WafLog>, i64), AppError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let total = self.log_repo.count(website_id).await?;
        let data = self
            .log_repo
            .list_page(page_size, (page - 1) * page_size, website_id)
            .await?;
        Ok((data, total))
    }

    pub async fn clear_logs(&self, website_id: Option<i64>) -> Result<u64, AppError> {
        self.log_repo.clear(website_id).await
    }

    /// 保留清理：删除早于 N 天的拦截日志（F-12 对齐——由组合根每日
    /// `data-retention-cleanup` 任务调用，防止 WAF 日志表无限膨胀）。
    pub async fn cleanup_logs_older_than(&self, days: i64) -> Result<u64, AppError> {
        self.log_repo.delete_older_than(days).await
    }

    /// 当前引擎内规则数（诊断用）。
    pub async fn engine_rule_count(&self) -> usize {
        self.engine.read().await.rule_count()
    }
}

fn validate_rule(rule: &WafRule) -> Result<(), AppError> {
    if rule.name.trim().is_empty() {
        return Err(AppError::BadRequest("WAF 规则名称不能为空".into()));
    }
    if rule.name.len() > 100 {
        return Err(AppError::BadRequest("WAF 规则名称过长（>100）".into()));
    }
    if !WafRule::is_valid_rule_type(&rule.rule_type) {
        return Err(AppError::BadRequest(format!(
            "不支持的 WAF 规则类型: {}（可选 {}）",
            rule.rule_type,
            WAF_RULE_TYPES.join("/")
        )));
    }
    if !WafRule::is_valid_action(&rule.action) {
        return Err(AppError::BadRequest(format!(
            "不支持的 WAF 动作: {}（可选 {}）",
            rule.action,
            WAF_ACTIONS.join("/")
        )));
    }
    if rule.website_id < 0 {
        return Err(AppError::BadRequest("website_id 不能为负".into()));
    }
    // pattern：除特征类（sql_injection/xss/cc，pattern 可空，走内置特征/参数）外必填
    if !matches!(rule.rule_type.as_str(), "sql_injection" | "xss" | "cc")
        && rule.pattern.trim().is_empty()
    {
        return Err(AppError::BadRequest(format!(
            "规则类型 {} 需要 pattern 匹配值",
            rule.rule_type
        )));
    }
    if rule.pattern.len() > 500 {
        return Err(AppError::BadRequest("pattern 过长（>500）".into()));
    }
    // config_json 必须是合法 JSON（cc 规则解析 cc_params，其他类型自解析）
    if serde_json::from_str::<serde_json::Value>(&rule.config_json).is_err() {
        return Err(AppError::BadRequest("config_json 不是合法 JSON".into()));
    }
    Ok(())
}

#[cfg(test)]
mod waf_service_tests {
    use super::*;
    use crate::infrastructure::db::{InMemoryWafLogRepository, InMemoryWafRuleRepository};

    fn svc() -> WafService {
        WafService::new(
            Arc::new(InMemoryWafRuleRepository::new()),
            Arc::new(InMemoryWafLogRepository::new()),
        )
    }

    fn req(ip: &str) -> WafRequest {
        WafRequest {
            method: "GET".into(),
            path: "/".into(),
            client_ip: ip.into(),
            user_agent: "curl".into(),
            ..Default::default()
        }
    }

    fn sample(name: &str, rule_type: &str, pattern: &str) -> WafRule {
        WafRule {
            name: name.into(),
            rule_type: rule_type.into(),
            website_id: 0,
            pattern: pattern.into(),
            config_json: "{}".into(),
            action: "block".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn ingest_engine_hits_maps_rules_and_skips_whitelist() {
        let s = svc();
        let ua = s
            .create_rule(sample("bad ua", "ua_blacklist", "sqlmap"))
            .await
            .unwrap();
        let mut wl = sample("ok bot", "ua_whitelist", "Good");
        wl.action = "log".into();
        let wl = s.create_rule(wl).await.unwrap();
        let mut path = sample("admin", "path_block", "/wp-admin");
        path.website_id = 7;
        path.action = "log".into();
        let path = s.create_rule(path).await.unwrap();
        let hit = |rules: Vec<i64>, cc: Vec<i64>, blocked: bool| EngineHit {
            website_id: 3,
            time: None,
            client_ip: "1.2.3.4".into(),
            method: "GET".into(),
            uri: "/wp-admin/x".into(),
            user_agent: "sqlmap".into(),
            blocked,
            rules,
            cc_rules: cc,
        };
        let n = s
            .ingest_engine_hits(&[
                hit(vec![ua.id, wl.id], vec![], true),
                hit(vec![path.id], vec![], false),
                hit(vec![], vec![ua.id + 100], false),
                hit(vec![999], vec![], true),
            ])
            .await
            .unwrap();
        assert_eq!(n, 4, "白名单命中不记录");
        let (logs, total) = s.list_logs_paginated(1, 20, None).await.unwrap();
        assert_eq!(total, 4);
        let by_rule = |id: i64| logs.iter().find(|l| l.rule_id == id).unwrap().clone();
        let l = by_rule(ua.id);
        assert_eq!(
            (l.action.as_str(), l.website_id, l.rule_type.as_str()),
            ("block", 3, "ua_blacklist")
        );
        assert_eq!(l.request_summary, "GET /wp-admin/x");
        // 网站级规则记在规则所属网站
        let l = by_rule(path.id);
        assert_eq!((l.action.as_str(), l.website_id), ("log", 7));
        // CC 拒绝与已删除规则
        let l = by_rule(ua.id + 100);
        assert_eq!((l.rule_type.as_str(), l.action.as_str()), ("cc", "block"));
        assert_eq!(by_rule(999).rule_name, "#999");
        assert_eq!(s.ingest_engine_hits(&[]).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn rule_crud_reloads_engine() {
        let s = svc();
        let created = s
            .create_rule(sample("bl", "ip_blacklist", "1.2.3.4"))
            .await
            .unwrap();
        assert_eq!(s.engine_rule_count().await, 1);

        let mut r = s.get_rule(created.id).await.unwrap();
        r.enabled = false;
        s.update_rule(&r).await.unwrap();
        // 停用规则退出引擎
        assert_eq!(s.engine_rule_count().await, 0);

        s.delete_rule(created.id).await.unwrap();
        assert!(s.get_rule(created.id).await.is_err());
    }

    #[tokio::test]
    async fn validation_rejects_bad_input() {
        let s = svc();
        assert!(s
            .create_rule(sample("", "ip_blacklist", "1.1.1.1"))
            .await
            .is_err());
        assert!(s.create_rule(sample("x", "rce", "y")).await.is_err());
        assert!(s
            .create_rule(sample("x", "ip_blacklist", ""))
            .await
            .is_err());
        // 特征类 pattern 可空
        let ok = s
            .create_rule(sample("sqli", "sql_injection", ""))
            .await
            .unwrap();
        assert!(ok.id > 0);
        // 非法 action / config_json
        let mut bad = sample("x", "ip_blacklist", "1.1.1.1");
        bad.action = "deny".into();
        assert!(s.create_rule(bad).await.is_err());
        let mut bad2 = sample("x", "ip_blacklist", "1.1.1.1");
        bad2.config_json = "not-json".into();
        assert!(s.create_rule(bad2).await.is_err());
    }

    #[tokio::test]
    async fn inspect_blocks_and_logs() {
        let s = svc();
        let r = s
            .create_rule(sample("bl", "ip_blacklist", "9.9.9.9"))
            .await
            .unwrap();

        let blocked = WafRequest {
            method: "GET".into(),
            path: "/".into(),
            query: String::new(),
            client_ip: "9.9.9.9".into(),
            user_agent: "curl".into(),
        };
        assert!(!s.inspect(0, &blocked).await.unwrap(), "黑名单 IP 应被阻断");
        let (logs, total) = s.list_logs_paginated(1, 20, None).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(logs[0].rule_id, r.id);
        assert_eq!(logs[0].action, "block");
        assert_eq!(logs[0].client_ip, "9.9.9.9");

        let normal = WafRequest {
            client_ip: "8.8.8.8".into(),
            ..Default::default()
        };
        assert!(s.inspect(0, &normal).await.unwrap());
        assert_eq!(s.list_logs_paginated(1, 20, None).await.unwrap().1, 1);
    }

    #[tokio::test]
    async fn inspect_log_action_records_warn() {
        let s = svc();
        let mut r = sample("obs", "path_block", "/admin-secret");
        r.action = "log".into();
        s.create_rule(r).await.unwrap();

        let req = WafRequest {
            method: "GET".into(),
            path: "/admin-secret/config".into(),
            ..Default::default()
        };
        // log 动作：返回放行 true（不阻断），但记录告警日志
        assert!(s.inspect(3, &req).await.unwrap());
        let (logs, total) = s.list_logs_paginated(1, 20, Some(3)).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(logs[0].action, "log");
        assert_eq!(logs[0].website_id, 3);
    }

    /// F-12 对齐：保留清理——超期日志删除，未超期保留（过期行由仓储语义决定）。
    #[tokio::test]
    async fn cleanup_logs_older_than_removes_expired() {
        let logs = Arc::new(InMemoryWafLogRepository::new());
        let s = WafService::new(Arc::new(InMemoryWafRuleRepository::new()), logs.clone());
        let r = s
            .create_rule(sample("bl", "ip_blacklist", "9.9.9.9"))
            .await
            .unwrap();
        // 一条「超期」日志（时间回拨 40 天）
        logs.create(&WafLog {
            id: 0,
            website_id: 0,
            rule_id: r.id,
            rule_name: "bl".into(),
            rule_type: "ip_blacklist".into(),
            action: "block".into(),
            client_ip: "9.9.9.9".into(),
            request_summary: "GET /".into(),
            user_agent: "curl".into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(40),
        })
        .await
        .unwrap();
        // 一条「新鲜」日志
        let _ = s
            .inspect(
                0,
                &WafRequest {
                    client_ip: "9.9.9.9".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let removed = s.cleanup_logs_older_than(30).await.unwrap();
        assert_eq!(removed, 1, "仅超期日志被清理");
        let (left, total) = s.list_logs_paginated(1, 20, None).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(left.len(), 1);
    }

    #[tokio::test]
    async fn clear_logs_and_pagination() {
        let s = svc();
        let mut r = sample("cc", "cc", "");
        r.config_json = r#"{"threshold":1,"window_secs":60}"#.into();
        r.action = "block".into();
        s.create_rule(r).await.unwrap();
        for _ in 0..3 {
            let _ = s
                .inspect(
                    0,
                    &WafRequest {
                        client_ip: "7.7.7.7".into(),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }
        let (_, total) = s.list_logs_paginated(1, 20, None).await.unwrap();
        assert!(total >= 2, "超阈值后应产生日志: {total}");

        // 分页窗口合法
        let (page, _) = s.list_logs_paginated(1, 2, None).await.unwrap();
        assert_eq!(page.len(), 2);

        let cleared = s.clear_logs(None).await.unwrap();
        assert!(cleared >= 2);
        assert_eq!(s.list_logs_paginated(1, 20, None).await.unwrap().1, 0);
    }

    /// §6.2 拦截通知：阻断命中发布 `WafBlocked` 事件；log 动作命中不通知。
    #[tokio::test]
    async fn block_hit_publishes_waf_blocked_event() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let s = WafService::new(
            Arc::new(InMemoryWafRuleRepository::new()),
            Arc::new(InMemoryWafLogRepository::new()),
        )
        .with_event_bus(bus);
        let r = s
            .create_rule(sample("bl", "ip_blacklist", "9.9.9.9"))
            .await
            .unwrap();

        assert!(!s.inspect(7, &req("9.9.9.9")).await.unwrap());
        let event = rx.try_recv().expect("阻断命中应发布 WafBlocked 事件");
        match event {
            DomainEvent::WafBlocked {
                website_id,
                rule_id,
                rule_name,
                rule_type,
                client_ip,
                request_summary,
                hit_count,
            } => {
                assert_eq!(website_id, 7);
                assert_eq!(rule_id, r.id);
                assert_eq!(rule_name, "bl");
                assert_eq!(rule_type, "ip_blacklist");
                assert_eq!(client_ip, "9.9.9.9");
                assert!(request_summary.starts_with("GET /"));
                assert_eq!(hit_count, 1, "首次命中立即通知");
            }
            other => panic!("非预期事件: {other:?}"),
        }
    }

    /// 拦截通知节流：同规则窗口内重复命中只累计不通知（防告警风暴）。
    #[tokio::test]
    async fn block_notify_throttles_repeated_hits() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let s = WafService::new(
            Arc::new(InMemoryWafRuleRepository::new()),
            Arc::new(InMemoryWafLogRepository::new()),
        )
        .with_event_bus(bus)
        // 大窗口：测试期间不可能过期 → 仅首次通知
        .with_block_notify_window(3600);
        let r = s
            .create_rule(sample("bl", "ip_blacklist", "9.9.9.9"))
            .await
            .unwrap();

        for _ in 0..5 {
            assert!(!s.inspect(0, &req("9.9.9.9")).await.unwrap());
        }
        assert!(matches!(
            rx.try_recv(),
            Ok(DomainEvent::WafBlocked { hit_count: 1, .. })
        ));
        assert!(
            rx.try_recv().is_err(),
            "同规则窗口内重复命中不应重复通知（规则 {}）",
            r.id
        );
        // 拦截日志仍逐条登记（节流只作用于通知，不影响审计）
        assert_eq!(s.list_logs_paginated(1, 20, None).await.unwrap().1, 5);
    }

    /// 节流窗口过期后的首次命中：把窗口内累计次数一并透出。
    #[tokio::test]
    async fn block_notify_aggregates_after_window_expires() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let s = WafService::new(
            Arc::new(InMemoryWafRuleRepository::new()),
            Arc::new(InMemoryWafLogRepository::new()),
        )
        .with_event_bus(bus)
        // 零窗口：每次命中都视为过期 → 每次通知且计数复位
        .with_block_notify_window(0);
        s.create_rule(sample("bl", "ip_blacklist", "9.9.9.9"))
            .await
            .unwrap();
        assert!(!s.inspect(0, &req("9.9.9.9")).await.unwrap());
        assert!(matches!(
            rx.try_recv(),
            Ok(DomainEvent::WafBlocked { hit_count: 1, .. })
        ));

        // 手工构造「窗口内累计」场景：大窗口下先累计 3 次（仅首次通知，
        // 后续 2 次 pending=2），随后直接断言节流器计数语义
        let throttle = BlockThrottle::new(Duration::from_secs(300));
        let now = Instant::now();
        assert_eq!(throttle.record(1, now), Some(1), "首次命中立即通知");
        assert_eq!(throttle.record(1, now), None, "窗口内重复命中不通知");
        assert_eq!(throttle.record(1, now), None, "窗口内重复命中不通知");
        let later = now + Duration::from_secs(301);
        assert_eq!(
            throttle.record(1, later),
            Some(3),
            "窗口过期后首次命中透出累计 3 次"
        );
        // 不同规则独立窗口，互不影响
        assert_eq!(throttle.record(2, later), Some(1), "规则维度独立节流");
    }

    /// 未注入事件总线（裁剪装配）：阻断命中不 panic，评估结论与日志不变。
    #[tokio::test]
    async fn block_without_event_bus_stays_silent() {
        let s = svc();
        s.create_rule(sample("bl", "ip_blacklist", "9.9.9.9"))
            .await
            .unwrap();
        assert!(!s.inspect(0, &req("9.9.9.9")).await.unwrap());
        assert_eq!(s.list_logs_paginated(1, 20, None).await.unwrap().1, 1);
    }

    /// 通知仅覆盖阻断命中：log（仅告警）动作不发布事件。
    #[tokio::test]
    async fn log_action_does_not_notify() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let s = WafService::new(
            Arc::new(InMemoryWafRuleRepository::new()),
            Arc::new(InMemoryWafLogRepository::new()),
        )
        .with_event_bus(bus);
        let mut r = sample("obs", "ip_blacklist", "9.9.9.9");
        r.action = "log".into();
        s.create_rule(r).await.unwrap();
        // log 动作放行且不通知
        assert!(s.inspect(0, &req("9.9.9.9")).await.unwrap());
        assert!(rx.try_recv().is_err(), "log 动作不应触发拦截通知");
    }
}
