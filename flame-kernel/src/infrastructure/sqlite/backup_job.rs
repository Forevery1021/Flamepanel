use super::*;

pub struct SqliteBackupJobRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteBackupJobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self::new_rw(pool.clone(), pool)
    }

    /// F-14: 读写双池构造——写串行、读并发（WAL 同库共享语义）。
    pub fn new_rw(write_pool: SqlitePool, read_pool: SqlitePool) -> Self {
        Self {
            pool: write_pool,
            read_pool,
        }
    }
}

const BACKUP_JOB_COLS: &str = "id, name, scope, scope_key, schedule, target_account_id, retention, enabled, last_status, last_message, last_remote_key, last_run_at, next_run_at, created_at, updated_at";

#[async_trait]
impl BackupJobRepository for SqliteBackupJobRepository {
    async fn list_all(&self) -> Result<Vec<BackupJob>, AppError> {
        let sql: String = format!("SELECT {BACKUP_JOB_COLS} FROM backup_jobs ORDER BY id");
        let rows = sqlx::query_as::<_, BackupJobRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(BackupJob::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<BackupJob>, AppError> {
        let sql: String = format!("SELECT {BACKUP_JOB_COLS} FROM backup_jobs WHERE id = ?");
        let row = sqlx::query_as::<_, BackupJobRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(BackupJob::from);
        Ok(row)
    }

    async fn create(&self, job: &BackupJob) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO backup_jobs (name, scope, scope_key, schedule, target_account_id, retention, enabled, next_run_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&job.name)
        .bind(&job.scope)
        .bind(&job.scope_key)
        .bind(&job.schedule)
        .bind(job.target_account_id)
        .bind(job.retention)
        .bind(job.enabled)
        .bind(job.next_run_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, job: &BackupJob) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE backup_jobs SET name = ?, scope = ?, scope_key = ?, schedule = ?, target_account_id = ?, retention = ?, enabled = ?, next_run_at = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&job.name)
        .bind(&job.scope)
        .bind(&job.scope_key)
        .bind(&job.schedule)
        .bind(job.target_account_id)
        .bind(job.retention)
        .bind(job.enabled)
        .bind(job.next_run_at)
        .bind(job.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("备份任务不存在: {}", job.id)));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM backup_jobs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn record_run(
        &self,
        id: i64,
        ok: bool,
        message: Option<String>,
        remote_key: Option<String>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        let status = if ok { "success" } else { "failed" };
        let result = sqlx::query(
            "UPDATE backup_jobs SET last_status = ?, last_message = ?, last_remote_key = ?, last_run_at = datetime('now'), next_run_at = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(status)
        .bind(message)
        .bind(remote_key)
        .bind(next_run_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("备份任务不存在: {}", id)));
        }
        Ok(())
    }
}
