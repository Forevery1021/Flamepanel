//! 系统设置领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::domain::settings_schema::PanelPolicy;
use std::sync::{Arc, RwLock};

pub struct SettingsService {
    pub repo: Arc<dyn SettingsRepository>,
    /// W4-a：由设置快照派生的运行时策略（访问控制 / 登录锁定 / 会话时长）。
    /// 每次经本服务写入后重算；热路径只克隆一个 `Arc`，不查库。
    policy: RwLock<Arc<PanelPolicy>>,
}

impl SettingsService {
    pub fn new(repo: Arc<dyn SettingsRepository>) -> Self {
        Self {
            repo,
            policy: RwLock::new(Arc::new(PanelPolicy::default())),
        }
    }

    /// 当前生效策略（无锁竞争时为一次读锁 + `Arc` 克隆）。
    pub fn policy(&self) -> Arc<PanelPolicy> {
        self.policy
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// 从库重算策略（启动时与每次写入后调用）。读库失败保留旧策略并返回错误。
    pub async fn reload_policy(&self) -> Result<Arc<PanelPolicy>, AppError> {
        let map = self.repo.get_all_map().await?;
        let next = Arc::new(PanelPolicy::from_map(&map));
        match self.policy.write() {
            Ok(mut g) => *g = next.clone(),
            Err(e) => *e.into_inner() = next.clone(),
        }
        Ok(next)
    }

    pub async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        self.repo.list_all().await
    }

    pub async fn list_all_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<PanelSetting>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.repo.count().await?;
        let data = self
            .repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        self.repo.get(key).await
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.repo.set(key, value).await?;
        self.refresh_policy_after_write(std::iter::once(key)).await;
        Ok(())
    }

    /// 批量原子写：多键在一次事务内全部写入（要么全成功要么全回滚）。
    pub async fn set_many(&self, entries: &[(String, String)]) -> Result<(), AppError> {
        self.repo.set_many(entries).await?;
        self.refresh_policy_after_write(entries.iter().map(|(k, _)| k.as_str()))
            .await;
        Ok(())
    }

    /// 写入涉及策略键时重算；失败只告警（写入本身已成功，下次写入 / 重启会再算）。
    async fn refresh_policy_after_write<'a>(&self, mut keys: impl Iterator<Item = &'a str>) {
        const POLICY_KEYS: &[&str] = &[
            "allowed_ips",
            "bind_domains",
            "session_timeout_minutes",
            "login_max_failures",
            "login_lockout_minutes",
            "two_factor_enabled",
        ];
        if keys.any(|k| POLICY_KEYS.contains(&k)) {
            if let Err(e) = self.reload_policy().await {
                tracing::warn!(error = %e, "failed to reload panel policy after settings write");
            }
        }
    }

    pub async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        self.repo.get_all_map().await
    }
}
