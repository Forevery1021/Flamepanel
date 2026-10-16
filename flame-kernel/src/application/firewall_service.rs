//! 防火墙应用编排服务（T8 拆分自原 `application/service.rs` 上帝文件）。
//! `FirewallManager`（OS 适配）已移至 `infrastructure/firewall.rs`。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::firewall::{describe_errors, normalize_rule};
use crate::domain::repository::*;
use crate::infrastructure::firewall::FirewallManager;
use std::collections::HashMap;
use std::sync::Arc;

pub struct FirewallService {
    pub firewall_repo: Arc<dyn FirewallRepository>,
    pub manager: FirewallManager,
}

impl FirewallService {
    pub fn new(firewall_repo: Arc<dyn FirewallRepository>, runner: SharedCommandRunner) -> Self {
        Self {
            firewall_repo,
            manager: FirewallManager::new(runner),
        }
    }

    /// 便捷：嵌入式执行器构造（行为与重构前一致）。
    pub fn new_embedded(firewall_repo: Arc<dyn FirewallRepository>) -> Self {
        Self {
            firewall_repo,
            manager: FirewallManager::embedded(),
        }
    }

    pub async fn list_rules(&self) -> Result<Vec<FirewallRule>, AppError> {
        self.firewall_repo.list_all().await
    }

    pub async fn list_rules_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<FirewallRule>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.firewall_repo.count().await?;
        let data = self
            .firewall_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_rule(&self, id: i64) -> Result<FirewallRule, AppError> {
        self.firewall_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))
    }

    pub async fn create_rule(&self, mut rule: FirewallRule) -> Result<FirewallRule, AppError> {
        validate(&mut rule)?;
        let id = self.firewall_repo.create(&rule).await?;
        rule.id = id;
        // Apply to OS if enabled
        if rule.enabled {
            self.manager.apply_rule(&rule).await.ok();
        }
        Ok(rule)
    }

    pub async fn update_rule(&self, mut rule: FirewallRule) -> Result<FirewallRule, AppError> {
        validate(&mut rule)?;
        // Get old rule to remove OS rule if changed
        let old = self
            .firewall_repo
            .find_by_id(rule.id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))?;

        // T7：先应用 OS 规则（失败即返回错误、不写 DB），避免库表与系统状态不一致。
        if old.enabled {
            match self.manager.remove_rule(&old).await {
                // 旧规则是历史遗留的非法数据（无法渲染 → 不可能被成功写入过）：跳过移除，允许借编辑修复
                Err(AppError::BadRequest(reason)) => {
                    tracing::warn!(id = old.id, %reason, "old firewall rule not renderable; skip removal");
                }
                other => other?,
            }
        }
        if rule.enabled {
            self.manager.apply_rule(&rule).await?;
        }

        self.firewall_repo.update(&rule).await?;
        Ok(rule)
    }

    pub async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        if let Some(rule) = self.firewall_repo.find_by_id(id).await? {
            match self.manager.remove_rule(&rule).await {
                // 历史遗留的非法规则无法渲染成系统命令（也就不可能被成功写入过）：允许直接删库
                Err(AppError::BadRequest(reason)) => {
                    tracing::warn!(id, %reason, "firewall rule not renderable; deleting record only");
                }
                other => other?,
            }
        }
        self.firewall_repo.delete(id).await
    }

    pub async fn toggle_rule(&self, id: i64, enabled: bool) -> Result<FirewallRule, AppError> {
        let rule = self
            .firewall_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))?;
        // T7：先应用 OS 规则，失败即返回错误、不写 DB，避免状态不一致。
        if enabled {
            self.manager.apply_rule(&rule).await?;
        } else {
            self.manager.remove_rule(&rule).await?;
        }
        self.firewall_repo.update_enabled(id, enabled).await?;
        self.firewall_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))
    }

    pub async fn apply_all_rules(&self) -> Result<(), AppError> {
        let rules = self.firewall_repo.list_all().await?;
        for rule in &rules {
            if rule.enabled {
                self.manager.apply_rule(rule).await.ok();
            }
        }
        Ok(())
    }

    pub async fn get_backend_status(&self) -> Result<HashMap<String, String>, AppError> {
        let backend = self.manager.detect_backend().await;
        let mut info = HashMap::new();
        info.insert("backend".to_string(), format!("{:?}", backend));
        match self.manager.get_status().await {
            Ok(s) => {
                info.insert("status".to_string(), s);
            }
            Err(_) => {
                info.insert("status".to_string(), "unknown".into());
            }
        }
        info.insert(
            "backend_name".to_string(),
            match backend {
                FirewallBackend::Ufw => "ufw".into(),
                FirewallBackend::Firewalld => "firewalld".into(),
                FirewallBackend::Iptables => "iptables".into(),
                FirewallBackend::Unsupported(m) => m,
            },
        );
        Ok(info)
    }

    pub async fn enable_firewall(&self) -> Result<(), AppError> {
        self.manager.enable_firewall().await
    }

    pub async fn disable_firewall(&self) -> Result<(), AppError> {
        // Remove all enabled OS rules first
        let rules = self.firewall_repo.list_all().await?;
        for rule in &rules {
            if rule.enabled {
                self.manager.remove_rule(rule).await.ok();
            }
        }
        self.manager.disable_firewall().await
    }

    pub async fn reorder_rules(&self, ids: &[i64]) -> Result<(), AppError> {
        self.firewall_repo.reorder(ids).await
    }
}

/// S-13：写入前统一校验 / 规范化；字段错误拼成一条 400 message（`field: reason; …`）。
fn validate(rule: &mut FirewallRule) -> Result<(), AppError> {
    normalize_rule(rule).map_err(|errors| AppError::ValidationError(describe_errors(&errors)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution_mode::{
        CommandOutput, PrivilegedCommand, PrivilegedCommandRunner,
    };
    use crate::infrastructure::db::InMemoryFirewallRepository;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Mutex;

    /// 只有 iptables 可用的假主机：记录每条非探测命令。
    #[derive(Default)]
    struct IptablesHost {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl PrivilegedCommandRunner for IptablesHost {
        async fn run(&self, cmd: &PrivilegedCommand) -> Result<CommandOutput, AppError> {
            let probe = cmd.args.is_empty();
            let ok = !(probe && (cmd.program == "ufw" || cmd.program == "firewall-cmd"));
            if !probe {
                self.calls
                    .lock()
                    .unwrap()
                    .push(format!("{} {}", cmd.program, cmd.args.join(" ")));
            }
            Ok(CommandOutput {
                stdout: String::new(),
                stderr: if ok {
                    String::new()
                } else {
                    "not found".into()
                },
                exit_code: if ok { 0 } else { 127 },
            })
        }
    }

    fn setup() -> (
        FirewallService,
        Arc<IptablesHost>,
        Arc<InMemoryFirewallRepository>,
    ) {
        let host = Arc::new(IptablesHost::default());
        let repo = Arc::new(InMemoryFirewallRepository::new());
        (FirewallService::new(repo.clone(), host.clone()), host, repo)
    }

    fn rule(port: &str) -> FirewallRule {
        FirewallRule {
            id: 0,
            name: "web".into(),
            description: None,
            protocol: "tcp".into(),
            port: Some(port.into()),
            source: Some("0.0.0.0/0".into()),
            destination: None,
            action: "allow".into(),
            enabled: true,
            priority: 50,
            direction: "in".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_validates_normalizes_and_renders_ranges() {
        let (svc, host, repo) = setup();
        let seeded = repo.list_all().await.unwrap().len();
        let err = svc.create_rule(rule("0")).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(ref m) if m.starts_with("port:")),
            "{err:?}"
        );
        assert_eq!(
            repo.list_all().await.unwrap().len(),
            seeded,
            "invalid rule not stored"
        );
        assert!(
            host.calls.lock().unwrap().is_empty(),
            "no command for invalid rule"
        );

        let created = svc.create_rule(rule("8000:9000,22")).await.unwrap();
        assert_eq!(created.port.as_deref(), Some("8000-9000,22"));
        assert_eq!(
            host.calls.lock().unwrap().as_slice(),
            ["iptables -A INPUT -p tcp -m multiport --dports 8000:9000,22 -j ACCEPT"]
        );
    }

    #[tokio::test]
    async fn legacy_invalid_rule_can_be_fixed_by_edit_or_deleted() {
        let (svc, host, repo) = setup();
        // 绕过校验直接写入一条历史遗留的脏规则
        let id = repo.create(&rule("80; reboot")).await.unwrap();

        let mut fixed = svc.get_rule(id).await.unwrap();
        fixed.port = Some("80".into());
        let saved = svc.update_rule(fixed).await.unwrap();
        assert_eq!(saved.port.as_deref(), Some("80"));
        // 旧规则无法渲染 → 跳过移除；只下发新规则
        assert_eq!(
            host.calls.lock().unwrap().as_slice(),
            ["iptables -A INPUT -p tcp --dport 80 -j ACCEPT"]
        );

        let dirty = repo.create(&rule("x")).await.unwrap();
        svc.delete_rule(dirty).await.unwrap();
        assert!(repo.find_by_id(dirty).await.unwrap().is_none());
        assert!(
            host.calls.lock().unwrap().iter().all(|c| !c.contains('x')),
            "dirty value never reaches a command"
        );
    }
}
