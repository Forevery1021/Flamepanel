use super::*;

pub struct SqliteLogRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteLogRepository {
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
impl LogRepository for SqliteLogRepository {
    async fn create(
        &self,
        source: &str,
        level: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<LogEntry, AppError> {
        let result = sqlx::query(
            "INSERT INTO logs (source, level, message, metadata, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(source)
        .bind(level)
        .bind(message)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let id = result.last_insert_rowid();
        Ok(LogEntry {
            id: id as i64,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            metadata: metadata.map(|s| s.into()),
            created_at: chrono::Utc::now(),
        })
    }

    async fn list(&self) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs ORDER BY id DESC LIMIT 200",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(LogEntry::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        let row = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(LogEntry::from);
        Ok(row)
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE source = ? ORDER BY id DESC LIMIT 200",
        )
        .bind(source)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(LogEntry::from).collect();
        Ok(rows)
    }

    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE level = ? ORDER BY id DESC LIMIT 200",
        )
        .bind(level)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(LogEntry::from).collect();
        Ok(rows)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM logs WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    /// F-12: 删除早于 N 天的系统日志
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM logs WHERE created_at < datetime('now', ?)")
            .bind(format!("-{} days", days))
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.rows_affected())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(LogEntry::from)
        .collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM logs")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }
}
