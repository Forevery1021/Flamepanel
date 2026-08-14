//! 系统设置领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct SettingsService {
    pub repo: Arc<dyn SettingsRepository>,
}

impl SettingsService {
    pub fn new(repo: Arc<dyn SettingsRepository>) -> Self {
        Self { repo }
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
        self.repo.set(key, value).await
    }

    /// 批量原子写：多键在一次事务内全部写入（要么全成功要么全回滚）。
    pub async fn set_many(&self, entries: &[(String, String)]) -> Result<(), AppError> {
        self.repo.set_many(entries).await
    }

    pub async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        self.repo.get_all_map().await
    }
}
