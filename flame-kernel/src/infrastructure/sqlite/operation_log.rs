use super::*;

pub struct SqliteOperationLogRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteOperationLogRepository {
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
impl OperationLogRepository for SqliteOperationLogRepository {
    async fn create(
        &self,
        username: &str,
        action: &str,
        target: Option<&str>,
        ip: Option<&str>,
    ) -> Result<OperationLog, AppError> {
        let result = sqlx::query(
            "INSERT INTO operation_logs (username, action, target, ip, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(username)
        .bind(action)
        .bind(target)
        .bind(ip)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let id = result.last_insert_rowid();
        Ok(OperationLog {
            id: id as i64,
            username: username.into(),
            action: action.into(),
            target: target.map(|s| s.into()),
            ip: ip.map(|s| s.into()),
            created_at: chrono::Utc::now(),
        })
    }

    async fn list(&self) -> Result<Vec<OperationLog>, AppError> {
        let rows = sqlx::query_as::<_, OperationLogRow>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs ORDER BY id DESC LIMIT 100",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(OperationLog::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        let row = sqlx::query_as::<_, OperationLogRow>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(OperationLog::from);
        Ok(row)
    }

    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        let rows = sqlx::query_as::<_, OperationLogRow>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs WHERE username = ? ORDER BY id DESC LIMIT 100",
        )
        .bind(username)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(OperationLog::from).collect();
        Ok(rows)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM operation_logs WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    /// F-12: 删除早于 N 天的操作日志
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let result =
            sqlx::query("DELETE FROM operation_logs WHERE created_at < datetime('now', ?)")
                .bind(format!("-{} days", days))
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.rows_affected())
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        action_prefix: Option<&str>,
    ) -> Result<Vec<OperationLog>, AppError> {
        let rows = match action_prefix {
            Some(prefix) if !prefix.is_empty() => {
                sqlx::query_as::<_, OperationLogRow>(
                    "SELECT id, username, action, target, ip, created_at FROM operation_logs \
                     WHERE action LIKE ? ESCAPE '\\' ORDER BY id DESC LIMIT ? OFFSET ?",
                )
                .bind(escape_like(prefix))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.read_pool)
                .await
            }
            _ => {
                sqlx::query_as::<_, OperationLogRow>(
                    "SELECT id, username, action, target, ip, created_at FROM operation_logs \
                     ORDER BY id DESC LIMIT ? OFFSET ?",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.read_pool)
                .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(OperationLog::from)
        .collect();
        Ok(rows)
    }

    async fn count(&self, action_prefix: Option<&str>) -> Result<i64, AppError> {
        let count: i64 = match action_prefix {
            Some(prefix) if !prefix.is_empty() => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM operation_logs WHERE action LIKE ? ESCAPE '\\'",
                )
                .bind(escape_like(prefix))
                .fetch_one(&self.read_pool)
                .await
            }
            _ => {
                sqlx::query_scalar("SELECT COUNT(*) FROM operation_logs")
                    .fetch_one(&self.read_pool)
                    .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }
}
