//! 网站领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::event::EventBus;
use crate::webserver::WebServerEngine;
use std::sync::Arc;

pub struct WebsiteService {
    pub website_repo: Arc<dyn WebsiteRepository>,
    pub event_bus: EventBus,
}

impl WebsiteService {
    pub fn new(website_repo: Arc<dyn WebsiteRepository>, event_bus: EventBus) -> Self {
        Self {
            website_repo,
            event_bus,
        }
    }

    pub async fn create_website(&self, website: &Website) -> Result<i64, AppError> {
        let id = self.website_repo.create(website).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::WebsiteCreated {
                website_id: id,
                domain: website.domain.clone(),
            })
            .await;
        Ok(id)
    }

    pub async fn list_websites(&self) -> Result<Vec<Website>, AppError> {
        self.website_repo.list_all().await
    }

    pub async fn list_websites_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<Website>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.website_repo.count().await?;
        let data = self
            .website_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_website(&self, id: i64) -> Result<Website, AppError> {
        self.website_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Website {} not found", id)))
    }

    pub async fn update_website(&self, website: &Website) -> Result<(), AppError> {
        self.website_repo.update(website).await
    }

    pub async fn delete_website(&self, id: i64) -> Result<(), AppError> {
        self.website_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Website {} not found", id)))?;
        self.website_repo.delete(id).await
    }

    /// 切换网站 Web 服务器引擎，并重新生成站点配置
    pub async fn switch_engine(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<Website, AppError> {
        let mut site = self.get_website(id).await?;
        let old_engine = site.engine.clone();
        if old_engine.eq_ignore_ascii_case(new_engine.as_str()) {
            return Ok(site);
        }
        site.engine = new_engine.as_str().into();
        let generated = crate::webserver::get_config_generator(new_engine)
            .generate_site_config(&site, None, None);
        let _ = generated;
        self.website_repo.update(&site).await?;
        Ok(site)
    }
}
