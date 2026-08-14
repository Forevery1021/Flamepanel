//! 备忘录 / 操作日志 / Outbox 事件落库 / 系统日志 等小型领域服务
//! （T8 拆分自原 `application/service.rs` 上帝文件，按「小服务合为 misc」策略）。
use crate::api::types::{PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct MemoService {
    pub repo: Arc<dyn MemoRepository>,
}

impl MemoService {
    pub fn new(repo: Arc<dyn MemoRepository>) -> Self {
        Self { repo }
    }

    pub async fn list(
        &self,
        kind: Option<&str>,
        done: Option<bool>,
    ) -> Result<Vec<Memo>, AppError> {
        self.repo.list(kind, done).await
    }

    pub async fn create(&self, content: &str, kind: &str) -> Result<Memo, AppError> {
        let kind = if kind == "todo" { "todo" } else { "memo" };
        if content.trim().is_empty() {
            return Err(AppError::BadRequest("内容不能为空".into()));
        }
        let id = self.repo.create(content, kind).await?;
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("Memo created but not found"))
    }

    pub async fn update(
        &self,
        id: i64,
        content: Option<&str>,
        done: Option<bool>,
    ) -> Result<Memo, AppError> {
        let mut memo = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Memo {} not found", id)))?;
        if let Some(c) = content {
            if c.trim().is_empty() {
                return Err(AppError::BadRequest("内容不能为空".into()));
            }
            memo.content = c.to_string();
        }
        if let Some(d) = done {
            memo.done = d;
        }
        self.repo.update(&memo).await?;
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("Memo updated but not found"))
    }

    pub async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.repo.delete(id).await
    }
}

pub struct OperationLogService {
    pub log_repo: Arc<dyn OperationLogRepository>,
}

impl OperationLogService {
    pub fn new(log_repo: Arc<dyn OperationLogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn log(
        &self,
        username: &str,
        action: &str,
        target: Option<&str>,
        ip: Option<&str>,
    ) -> Result<OperationLog, AppError> {
        self.log_repo.create(username, action, target, ip).await
    }

    pub async fn list(&self) -> Result<Vec<OperationLog>, AppError> {
        self.log_repo.list().await
    }

    pub async fn list_paginated(
        &self,
        params: &PaginationParams,
        action_filter: Option<&str>,
    ) -> Result<PaginatedResponse<OperationLog>, AppError> {
        // 分页下沉（Stage2）：数据库层直接 LIMIT/OFFSET，避免 operation_logs 全表加载
        let prefix = action_filter.filter(|s| !s.is_empty());
        let total = self.log_repo.count(prefix).await?;
        let data = self
            .log_repo
            .list_page(params.page_size(), params.offset(), prefix)
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        self.log_repo.find_by_id(id).await
    }

    pub async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        self.log_repo.list_by_username(username).await
    }

    pub async fn delete_log(&self, id: i64) -> Result<(), AppError> {
        self.log_repo.delete(id).await
    }
}

/// 事件落库（Outbox）服务：负责把领域事件持久化存档，并提供审计查询（Stage6）。
pub struct OutboxService {
    pub outbox_repo: Arc<dyn OutboxRepository>,
}

impl OutboxService {
    pub fn new(outbox_repo: Arc<dyn OutboxRepository>) -> Self {
        Self { outbox_repo }
    }

    /// 把一条领域事件落库：`event_type` 取变体名，`payload` 为 JSON 载荷。
    ///
    /// 落库失败时自动重试（小指数退避，最多 3 次），保证「不丢关键审计」（Stage 9）。
    pub async fn record_event(&self, event: &DomainEvent) -> Result<OutboxEvent, AppError> {
        let event_type = format!("{:?}", event)
            .split(' ')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        let payload = self.event_payload(event);
        let max_attempts = 3;
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.outbox_repo.append(&event_type, &payload, true).await {
                Ok(ev) => return Ok(ev),
                Err(e) if attempt < max_attempts => {
                    // 小指数退避后重试，避免短时 I/O/锁瞬时抖动丢审计
                    tokio::time::sleep(std::time::Duration::from_millis(50 * (1 << (attempt - 1))))
                        .await;
                    tracing::warn!(
                        "Outbox append retry {}/{} for event {:?}: {}",
                        attempt,
                        max_attempts,
                        event,
                        e
                    );
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 事件载荷：用 debug 形式序列化为紧凑 JSON（事件各字段均为简单标量）。
    fn event_payload(&self, event: &DomainEvent) -> String {
        serde_json::to_string(event).unwrap_or_else(|_| format!("{:?}", event))
    }

    pub async fn list_paginated(
        &self,
        params: &PaginationParams,
        event_type: Option<&str>,
    ) -> Result<PaginatedResponse<OutboxEvent>, AppError> {
        let filter = event_type.filter(|s| !s.is_empty());
        let total = self.outbox_repo.count(filter).await?;
        let data = self
            .outbox_repo
            .list_page(params.page_size(), params.offset(), filter)
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }
}

pub struct LogService {
    pub log_repo: Arc<dyn LogRepository>,
}

impl LogService {
    pub fn new(log_repo: Arc<dyn LogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn log(
        &self,
        source: &str,
        level: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<LogEntry, AppError> {
        self.log_repo.create(source, level, message, metadata).await
    }

    pub async fn list(&self) -> Result<Vec<LogEntry>, AppError> {
        self.log_repo.list().await
    }

    pub async fn list_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<LogEntry>, AppError> {
        // 分页下沉（Stage2）：数据库层直接 LIMIT/OFFSET，避免 logs 全表加载
        let total = self.log_repo.count().await?;
        let data = self
            .log_repo
            .list_page(params.page_size(), params.offset())
            .await?;
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        self.log_repo.find_by_id(id).await
    }

    pub async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        self.log_repo.list_by_source(source).await
    }

    pub async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        self.log_repo.list_by_level(level).await
    }

    pub async fn delete_log(&self, id: i64) -> Result<(), AppError> {
        self.log_repo.delete(id).await
    }
}
