//! 防火墙应用编排服务（T8 拆分自原 `application/service.rs` 上帝文件）。
//! `FirewallManager`（OS 适配）已移至 `infrastructure/firewall.rs`。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::domain::entity::*;
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
        let id = self.firewall_repo.create(&rule).await?;
        rule.id = id;
        // Apply to OS if enabled
        if rule.enabled {
            self.manager.apply_rule(&rule).await.ok();
        }
        Ok(rule)
    }

    pub async fn update_rule(&self, rule: FirewallRule) -> Result<FirewallRule, AppError> {
        // Get old rule to remove OS rule if changed
        let old = self
            .firewall_repo
            .find_by_id(rule.id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))?;

        // T7：先应用 OS 规则（失败即返回错误、不写 DB），避免库表与系统状态不一致。
        if old.enabled {
            self.manager.remove_rule(&old).await?;
        }
        if rule.enabled {
            self.manager.apply_rule(&rule).await?;
        }

        self.firewall_repo.update(&rule).await?;
        Ok(rule)
    }

    pub async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        if let Some(rule) = self.firewall_repo.find_by_id(id).await? {
            self.manager.remove_rule(&rule).await?;
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
