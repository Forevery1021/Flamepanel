//! 用户领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::event::EventBus;
use std::sync::Arc;

pub struct UserService {
    pub user_repo: Arc<dyn UserRepository>,
    pub event_bus: EventBus,
    /// 鉴权短缓存（Stage 2 / A4）：`find_by_id` 走 cache-aside，写路径显式失效
    pub auth_cache: Arc<crate::utils::auth_cache::AuthCache>,
}

impl UserService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        event_bus: EventBus,
        auth_cache: Arc<crate::utils::auth_cache::AuthCache>,
    ) -> Self {
        Self {
            user_repo,
            event_bus,
            auth_cache,
        }
    }

    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError> {
        // 领域规则：用户名格式校验上移到 User::validate_username
        let probe = User {
            id: 0,
            username: username.to_string(),
            password_hash: String::new(),
            role: role.to_string(),
            created_at: chrono::Utc::now(),
            must_change_password: false,
        };
        probe.validate_username()?;
        let user = self.user_repo.create(username, password_hash, role).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::UserCreated {
                user_id: user.id,
                username: username.to_string(),
            })
            .await;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        // Stage 2（A4）：cache-aside 短缓存，命中即返回，避免每请求查仓储
        if let Some(user) = self.auth_cache.users.get(&id).await {
            return Ok(Some(user));
        }
        let user = self.user_repo.find_by_id(id).await?;
        if let Some(ref u) = user {
            self.auth_cache.users.insert(id, u.clone()).await;
        }
        Ok(user)
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        self.user_repo.find_by_username(username).await
    }

    pub async fn update_password(&self, id: i64, new_hash: &str) -> Result<(), AppError> {
        self.user_repo.update_password(id, new_hash).await?;
        // 密码变更后旧缓存立即失效，避免旧哈希残留
        self.auth_cache.users.invalidate(&id).await;
        Ok(())
    }

    /// 设置/清除强制改密标志
    pub async fn set_must_change_password(&self, id: i64, must: bool) -> Result<(), AppError> {
        let mut user = self.get_user(id).await?;
        if must {
            user.must_change_password = true;
        } else {
            user.mark_password_changed();
        }
        self.user_repo.update(&user).await?;
        // 强制改密标志变化后旧缓存立即失效
        self.auth_cache.users.invalidate(&id).await;
        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<User>, AppError> {
        self.user_repo.list().await
    }

    pub async fn list_users_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<User>, AppError> {
        // 分页下沉（Stage1）：直接 LIMIT/OFFSET，避免全表加载 + 内存切片
        let total = self.user_repo.count().await?;
        let data = self
            .user_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_user(&self, id: i64) -> Result<User, AppError> {
        self.user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
    }

    pub async fn update_user(&self, user: &User) -> Result<(), AppError> {
        self.user_repo
            .find_by_id(user.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user.id)))?;
        self.user_repo.update(user).await?;
        // 角色/用户名等变更后旧缓存立即失效，保证下一次鉴权读到最新
        self.auth_cache.users.invalidate(&user.id).await;
        Ok(())
    }

    pub async fn delete_user(&self, id: i64) -> Result<(), AppError> {
        self.user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
        self.user_repo.delete(id).await?;
        // 删除后移除缓存，避免残留用户可被后续请求命中
        self.auth_cache.users.invalidate(&id).await;
        Ok(())
    }
}
