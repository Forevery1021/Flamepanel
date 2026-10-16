/// F-04: SQLite 持久化 refresh token 黑名单仓储（重启后已注销凭证保持失效）。
use super::*;

pub struct SqliteRevokedTokenRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteRevokedTokenRepository {
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
impl RevokedTokenRepository for SqliteRevokedTokenRepository {
    async fn insert(&self, token_hash: &str, expires_at: i64) -> Result<(), AppError> {
        sqlx::query(
            "INSERT OR REPLACE INTO revoked_refresh_tokens (token_hash, expires_at) VALUES (?, ?)",
        )
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn contains(&self, token_hash: &str) -> Result<bool, AppError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM revoked_refresh_tokens WHERE token_hash = ? AND expires_at > ?",
        )
        .bind(token_hash)
        .bind(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        )
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(row.is_some())
    }

    async fn cleanup_expired(&self) -> Result<(), AppError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        sqlx::query("DELETE FROM revoked_refresh_tokens WHERE expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
