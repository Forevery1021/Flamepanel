/// 远程应用源 SQLite 仓储（M-A Issue 1.1）
use super::*;

pub struct SqliteAppSourceRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteAppSourceRepository {
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

const APP_SOURCE_COLS: &str =
    "id, name, source_type, url, enabled, public_key, last_synced_at, created_at, updated_at";

#[async_trait]
impl AppSourceRepository for SqliteAppSourceRepository {
    async fn list_all(&self) -> Result<Vec<AppSource>, AppError> {
        let sql: String = format!("SELECT {APP_SOURCE_COLS} FROM app_sources ORDER BY id");
        let rows = sqlx::query_as::<_, AppSourceRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(AppSource::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<AppSource>, AppError> {
        let sql: String = format!("SELECT {APP_SOURCE_COLS} FROM app_sources WHERE id = ?");
        let row = sqlx::query_as::<_, AppSourceRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(AppSource::from);
        Ok(row)
    }

    async fn find_by_url(&self, url: &str) -> Result<Option<AppSource>, AppError> {
        let sql: String = format!("SELECT {APP_SOURCE_COLS} FROM app_sources WHERE url = ?");
        let row = sqlx::query_as::<_, AppSourceRow>(sqlx::AssertSqlSafe(sql))
            .bind(url)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(AppSource::from);
        Ok(row)
    }

    async fn create(&self, source: &AppSource) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO app_sources (name, source_type, url, enabled, public_key) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&source.name)
        .bind(&source.source_type)
        .bind(&source.url)
        .bind(source.enabled)
        .bind(&source.public_key)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, source: &AppSource) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE app_sources SET name = ?, source_type = ?, url = ?, enabled = ?, public_key = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&source.name)
        .bind(&source.source_type)
        .bind(&source.url)
        .bind(source.enabled)
        .bind(&source.public_key)
        .bind(source.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM app_sources WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn touch_synced(&self, id: i64) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE app_sources SET last_synced_at = datetime('now'), updated_at = datetime('now') WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
