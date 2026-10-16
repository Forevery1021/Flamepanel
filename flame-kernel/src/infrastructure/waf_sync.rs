//! WAF 后台同步（Doc/20 §12.3 · Doc/32 W3-d）：
//!
//! 1. **命中日志回灌**：增量读取 Nginx / OpenResty / Apache / Caddy 的 `flamepanel-waf.log`
//!    （OLS 无原生条件日志，不回灌；非 root 经特权助手
//!    `read_log`），解析为拦截日志写库并触发拦截通知；读取位置（inode + 偏移）持久化到面板设置，
//!    重启不重复导入、轮转自动从头读。
//! 2. **网站变化重新下发**：网站集合（引擎 × ID × 域名）变化且存在网站级规则时重新下发，
//!    覆盖建站 / 删站 / 改域名 / 切换引擎（此前需要手动「下发到 Web 引擎」）。

use crate::application::waf_ports::{EngineHit, WafIngestStatus};
use crate::application::waf_service::WafService;
use crate::core::error::AppError;
use crate::domain::repository::{SettingsRepository, WebsiteRepository};
use crate::webserver::host::MAX_LOG_CHUNK;
use crate::webserver::waf_engines;
use crate::webserver::{WebServerEngine, WebServerManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 每次同步每个引擎最多读取的块数（防止积压时单次占用过久；剩余下次继续）。
const MAX_CHUNKS_PER_TICK: usize = 8;
/// 有命中日志的引擎（OLS 除外，见 `waf_engines::has_hit_log`）。
const ENGINES: [WebServerEngine; 4] = [
    WebServerEngine::Nginx,
    WebServerEngine::OpenResty,
    WebServerEngine::Apache,
    WebServerEngine::Caddy,
];

fn offset_key(engine: &WebServerEngine) -> String {
    format!("waf.ingest.{}", engine.as_str())
}

#[derive(Default)]
struct State {
    /// 引擎 → (inode, offset)
    cursors: HashMap<String, (u64, u64)>,
    imported: HashMap<String, u64>,
    /// 上次观察到的网站集合指纹（首次只建立基线）
    fingerprint: Option<String>,
}

pub struct WafSyncTask {
    manager: WebServerManager,
    website_repo: Arc<dyn WebsiteRepository>,
    settings: Arc<dyn SettingsRepository>,
    waf: Arc<WafService>,
    state: Mutex<State>,
}

impl WafSyncTask {
    pub fn new(
        manager: WebServerManager,
        website_repo: Arc<dyn WebsiteRepository>,
        settings: Arc<dyn SettingsRepository>,
        waf: Arc<WafService>,
    ) -> Self {
        Self {
            manager,
            website_repo,
            settings,
            waf,
            state: Mutex::new(State::default()),
        }
    }

    /// 一次同步：先回灌命中日志，再检查网站集合变化。返回本次导入条数。
    pub async fn tick(&self) -> usize {
        let websites = match self.website_repo.list_all().await {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("WAF sync: list websites failed: {e}");
                return 0;
            }
        };
        let mut st = self.state.lock().await;
        let mut total = 0;
        for engine in ENGINES {
            // 引擎未安装：没有日志可读，也不展示状态
            if !self.manager.engine_installed(&engine).await {
                continue;
            }
            let hosts: HashMap<String, i64> = websites
                .iter()
                .filter(|w| WebServerEngine::from_name(&w.engine).as_ref() == Some(&engine))
                .map(|w| (w.domain.to_ascii_lowercase(), w.id))
                .collect();
            let name = engine.as_str().to_string();
            let result = self.ingest_engine(&mut st, &engine, &hosts).await;
            let (last_run_at, last_error) = match result {
                Ok(n) => {
                    total += n;
                    *st.imported.entry(name.clone()).or_default() += n as u64;
                    (Some(chrono::Utc::now()), None)
                }
                Err(e) => {
                    tracing::debug!(engine = %name, "WAF sync: ingest failed: {e}");
                    (None, Some(e.to_string()))
                }
            };
            let status = WafIngestStatus {
                engine: name.clone(),
                offset: st.cursors.get(&name).map(|c| c.1).unwrap_or(0),
                imported: st.imported.get(&name).copied().unwrap_or(0),
                last_run_at,
                last_error,
            };
            self.waf.set_ingest_status(status);
        }
        self.reconcile_sites(&mut st, &websites).await;
        total
    }

    async fn cursor(&self, st: &mut State, engine: &WebServerEngine) -> (Option<u64>, u64) {
        let name = engine.as_str().to_string();
        if let Some((ino, off)) = st.cursors.get(&name) {
            return (Some(*ino), *off);
        }
        let saved = self
            .settings
            .get(&offset_key(engine))
            .await
            .ok()
            .flatten()
            .and_then(|v| {
                let (i, o) = v.split_once(':')?;
                Some((i.parse::<u64>().ok()?, o.parse::<u64>().ok()?))
            });
        match saved {
            Some((i, o)) => {
                st.cursors.insert(name, (i, o));
                (Some(i), o)
            }
            None => (None, 0),
        }
    }

    async fn ingest_engine(
        &self,
        st: &mut State,
        engine: &WebServerEngine,
        hosts: &HashMap<String, i64>,
    ) -> Result<usize, AppError> {
        let (mut inode, mut offset) = self.cursor(st, engine).await;
        let mut written = 0;
        for _ in 0..MAX_CHUNKS_PER_TICK {
            let Some(chunk) = self
                .manager
                .read_waf_log(engine, offset, inode, MAX_LOG_CHUNK)
                .await?
            else {
                // 尚未下发过 WAF（日志未创建）
                return Ok(written);
            };
            let hits: Vec<EngineHit> = chunk
                .data
                .lines()
                .filter_map(|l| waf_engines::parse_hit_line(engine, l))
                .map(|h| EngineHit {
                    website_id: hosts.get(&h.host).copied().unwrap_or(0),
                    time: h.time,
                    client_ip: h.client_ip,
                    method: h.method,
                    uri: h.uri,
                    user_agent: h.user_agent,
                    blocked: h.blocked,
                    rules: h.rules,
                    cc_rules: h.cc_rules,
                })
                .collect();
            // 先写库再推进游标：失败时下次重读（至少一次）
            written += self.waf.ingest_engine_hits(&hits).await?;
            let moved = chunk.next_offset != offset || Some(chunk.inode) != inode;
            inode = Some(chunk.inode);
            offset = chunk.next_offset;
            st.cursors
                .insert(engine.as_str().to_string(), (chunk.inode, offset));
            if moved {
                self.settings
                    .set(&offset_key(engine), &format!("{}:{offset}", chunk.inode))
                    .await?;
            }
            if chunk.next_offset >= chunk.size || !moved {
                break;
            }
        }
        Ok(written)
    }

    async fn reconcile_sites(&self, st: &mut State, websites: &[crate::domain::entity::Website]) {
        let mut items: Vec<String> = websites
            .iter()
            .filter(|w| {
                WebServerEngine::from_name(&w.engine)
                    .as_ref()
                    .is_some_and(waf_engines::supports)
            })
            .map(|w| format!("{}|{}|{}", w.engine, w.id, w.domain.to_ascii_lowercase()))
            .collect();
        items.sort();
        let fp = items.join("\n");
        let changed = st.fingerprint.as_ref().is_some_and(|prev| *prev != fp);
        st.fingerprint = Some(fp);
        if !changed || !self.waf.enforcement_configured() {
            return;
        }
        match self.waf.has_site_scoped_rules().await {
            Ok(true) => {
                tracing::info!("WAF: websites changed; re-applying site-scoped rules");
                self.waf.reapply_best_effort().await;
            }
            Ok(false) => {}
            Err(e) => tracing::warn!("WAF sync: load rules failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::{WafRule, Website};
    use crate::infrastructure::db::{
        InMemorySettingsRepository, InMemoryWafLogRepository, InMemoryWafRuleRepository,
        InMemoryWebsiteRepository,
    };
    use crate::webserver::host::LocalEngineHost;

    const LOG: &str = "var/log/nginx/flamepanel-waf.log";

    fn line(host: &str, rules: &str) -> String {
        format!(
            r#"{{"time":"2026-09-27T11:25:48+08:00","host":"{host}","ip":"1.2.3.4","method":"GET","uri":"/x","ua":"sqlmap","status":"403","block":"1","rules":"{rules}","cc":""}}"#
        ) + "\n"
    }

    async fn setup() -> (std::path::PathBuf, WafSyncTask, Arc<WafService>, i64) {
        let root = std::env::temp_dir().join(format!(
            "fp-wafsync-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        std::fs::create_dir_all(root.join("etc/nginx")).unwrap();
        std::fs::write(root.join("etc/nginx/nginx.conf"), "http {\n}\n").unwrap();
        std::fs::create_dir_all(root.join("var/log/nginx")).unwrap();
        let manager = WebServerManager::new().with_host(Arc::new(LocalEngineHost::new(&root)));
        let websites = Arc::new(InMemoryWebsiteRepository::new());
        let waf = Arc::new(WafService::new(
            Arc::new(InMemoryWafRuleRepository::new()),
            Arc::new(InMemoryWafLogRepository::new()),
        ));
        let rule = waf
            .create_rule(WafRule {
                name: "ua".into(),
                rule_type: "ua_blacklist".into(),
                pattern: "sqlmap".into(),
                action: "block".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let settings = Arc::new(InMemorySettingsRepository::new());
        let task = WafSyncTask::new(manager, websites, settings, waf.clone());
        (root, task, waf, rule.id)
    }

    fn rand_suffix() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        N.fetch_add(1, Ordering::Relaxed)
    }

    #[tokio::test]
    async fn ingests_incrementally_and_survives_restart() {
        let (root, task, waf, rid) = setup().await;
        // 日志尚不存在：无导入、无错误
        assert_eq!(task.tick().await, 0);
        let log = root.join(LOG);
        std::fs::write(&log, line("a.test", &format!("r{rid}:1"))).unwrap();
        assert_eq!(task.tick().await, 1);
        assert_eq!(task.tick().await, 0, "不重复导入");
        // 追加 + 半行
        let mut more = line("a.test", &format!("r{rid}:1"));
        more.push_str("{\"partial");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&log)
            .and_then(|mut f| std::io::Write::write_all(&mut f, more.as_bytes()))
            .unwrap();
        assert_eq!(task.tick().await, 1);
        let st = waf.ingest_status();
        assert_eq!(st.len(), 1);
        assert_eq!((st[0].engine.as_str(), st[0].imported), ("nginx", 2));
        assert!(st[0].last_error.is_none());

        // 「重启」：新任务共享同一设置仓储 → 从持久化偏移继续
        let task2 = WafSyncTask::new(
            task.manager.clone(),
            task.website_repo.clone(),
            task.settings.clone(),
            waf.clone(),
        );
        assert_eq!(task2.tick().await, 0);
        assert_eq!(waf.list_logs_paginated(1, 20, None).await.unwrap().1, 2);

        // 轮转：新文件从头读
        std::fs::rename(&log, root.join("var/log/nginx/flamepanel-waf.log.1")).unwrap();
        std::fs::write(&log, line("a.test", &format!("r{rid}:1"))).unwrap();
        assert_eq!(task2.tick().await, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolves_website_by_host_and_reports_errors() {
        let (root, task, waf, rid) = setup().await;
        let id = task
            .website_repo
            .create(&Website {
                id: 0,
                name: "a".into(),
                domain: "A.test".into(),
                root_path: "/var/www/a".into(),
                status: "running".into(),
                node_id: 0,
                engine: "nginx".into(),
                ssl_enabled: false,
                proxy_enabled: false,
                proxy_pass: None,
                site_type: "static".into(),
                runtime_id: None,
                project_id: None,
                rewrite_preset: None,
                created_at: chrono::Utc::now(),
                resource_version: 0,
            })
            .await
            .unwrap();
        std::fs::write(root.join(LOG), line("a.test", &format!("r{rid}:1"))).unwrap();
        assert_eq!(task.tick().await, 1);
        let (logs, _) = waf.list_logs_paginated(1, 20, None).await.unwrap();
        assert_eq!(logs[0].website_id, id);

        // 日志被替换为符号链接：记录错误而不导入
        let log = root.join(LOG);
        std::fs::remove_file(&log).unwrap();
        std::os::unix::fs::symlink(root.join("etc/nginx/nginx.conf"), &log).unwrap();
        assert_eq!(task.tick().await, 0);
        assert!(waf.ingest_status()[0].last_error.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }
}
