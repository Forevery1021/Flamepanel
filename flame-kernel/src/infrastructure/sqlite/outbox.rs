use super::*;

pub struct SqliteOutboxRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteOutboxRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self::new_rw(pool.clone(), pool)
    }

    /// F-14: 读写双池构造——write 池串行写（max=1），read 池并发读（WAL）。
    pub fn new_rw(write_pool: SqlitePool, read_pool: SqlitePool) -> Self {
        Self {
            pool: write_pool,
            read_pool,
        }
    }
}

#[async_trait]
impl OutboxRepository for SqliteOutboxRepository {
    async fn append(
        &self,
        event_type: &str,
        payload: &str,
        published: bool,
    ) -> Result<OutboxEvent, AppError> {
        let result = sqlx::query(
            "INSERT INTO outbox_events (event_type, payload, published, created_at) VALUES (?, ?, ?, datetime('now'))",
        )
        .bind(event_type)
        .bind(payload)
        .bind(published as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let id = result.last_insert_rowid();
        Ok(OutboxEvent {
            id: id as i64,
            event_type: event_type.into(),
            payload: payload.into(),
            published,
            created_at: chrono::Utc::now(),
        })
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        event_type: Option<&str>,
    ) -> Result<Vec<OutboxEvent>, AppError> {
        let sql = match event_type {
            Some(_) => {
                "SELECT id, event_type, payload, published, created_at FROM outbox_events \
                 WHERE event_type = ? ORDER BY id DESC LIMIT ? OFFSET ?"
            }
            None => {
                "SELECT id, event_type, payload, published, created_at FROM outbox_events \
                 ORDER BY id DESC LIMIT ? OFFSET ?"
            }
        };
        let rows = if let Some(et) = event_type {
            sqlx::query_as::<_, OutboxEventRow>(sql)
                .bind(et)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        } else {
            sqlx::query_as::<_, OutboxEventRow>(sql)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        };
        Ok(rows.into_iter().map(OutboxEvent::from).collect())
    }

    async fn count(&self, event_type: Option<&str>) -> Result<i64, AppError> {
        let sql = match event_type {
            Some(_) => "SELECT COUNT(*) FROM outbox_events WHERE event_type = ?",
            None => "SELECT COUNT(*) FROM outbox_events",
        };
        let count: (i64,) = if let Some(et) = event_type {
            sqlx::query_as(sql)
                .bind(et)
                .fetch_one(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        } else {
            sqlx::query_as(sql)
                .fetch_one(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        };
        Ok(count.0)
    }

    /// F-12: 删除早于 N 天的已发布 outbox 事件
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let result = sqlx::query(
            "DELETE FROM outbox_events WHERE published = 1 AND created_at < datetime('now', ?)",
        )
        .bind(format!("-{} days", days))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.rows_affected())
    }
}
