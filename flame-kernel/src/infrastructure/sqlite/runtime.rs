/// 运行时仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
use super::*;

pub struct SqliteRuntimeRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteRuntimeRepository {
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

const RUNTIME_COLS: &str =
    "id, kind, version, install_path, status, is_default, metadata_json, created_at, updated_at";

#[async_trait]
impl RuntimeRepository for SqliteRuntimeRepository {
    async fn list_all(&self) -> Result<Vec<Runtime>, AppError> {
        let sql: String = format!("SELECT {RUNTIME_COLS} FROM runtimes ORDER BY kind, version");
        let rows = sqlx::query_as::<_, RuntimeRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(Runtime::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Runtime>, AppError> {
        let sql: String = format!("SELECT {RUNTIME_COLS} FROM runtimes WHERE id = ?");
        let row = sqlx::query_as::<_, RuntimeRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(Runtime::from);
        Ok(row)
    }

    async fn create(&self, runtime: &Runtime) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO runtimes (kind, version, install_path, status, is_default, metadata_json) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&runtime.kind)
        .bind(&runtime.version)
        .bind(&runtime.install_path)
        .bind(&runtime.status)
        .bind(runtime.is_default)
        .bind(&runtime.metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, runtime: &Runtime) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE runtimes SET install_path = ?, status = ?, is_default = ?, metadata_json = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&runtime.install_path)
        .bind(&runtime.status)
        .bind(runtime.is_default)
        .bind(&runtime.metadata_json)
        .bind(runtime.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("运行时不存在: {}", runtime.id)));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM runtimes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn set_default(&self, id: i64) -> Result<(), AppError> {
        let target = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("运行时不存在: {}", id)))?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        sqlx::query("UPDATE runtimes SET is_default = 0 WHERE kind = ?")
            .bind(&target.kind)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        sqlx::query(
            "UPDATE runtimes SET is_default = 1, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
