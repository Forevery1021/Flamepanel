use super::*;

pub struct SqliteScheduledTaskRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteScheduledTaskRepository {
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
impl ScheduledTaskRepository for SqliteScheduledTaskRepository {
    async fn list_all(&self) -> Result<Vec<ScheduledTask>, AppError> {
        let rows = sqlx::query_as::<_, ScheduledTaskRow>(
            "SELECT id, name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at, created_at, updated_at FROM scheduled_tasks ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ScheduledTask::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ScheduledTask>, AppError> {
        let row = sqlx::query_as::<_, ScheduledTaskRow>(
            "SELECT id, name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at, created_at, updated_at FROM scheduled_tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(ScheduledTask::from);
        Ok(row)
    }

    async fn create(&self, task: &ScheduledTask) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO scheduled_tasks (name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&task.name)
        .bind(&task.command)
        .bind(&task.schedule)
        .bind(task.enabled)
        .bind(&task.last_status)
        .bind(&task.last_output)
        .bind(task.last_run_at)
        .bind(task.next_run_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, task: &ScheduledTask) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE scheduled_tasks SET name=?, command=?, schedule=?, enabled=?, last_status=?, last_output=?, last_run_at=?, next_run_at=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&task.name)
        .bind(&task.command)
        .bind(&task.schedule)
        .bind(task.enabled)
        .bind(&task.last_status)
        .bind(&task.last_output)
        .bind(task.last_run_at)
        .bind(task.next_run_at)
        .bind(task.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM scheduled_tasks WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ScheduledTask>, AppError> {
        let rows = sqlx::query_as::<_, ScheduledTaskRow>(
            "SELECT id, name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at, created_at, updated_at FROM scheduled_tasks \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(ScheduledTask::from)
        .collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scheduled_tasks")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }
}
