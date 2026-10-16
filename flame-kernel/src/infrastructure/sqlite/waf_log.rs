use super::*;

pub struct SqliteWafLogRepository {
    pub(super) pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteWafLogRepository {
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

const WAF_LOG_COLS: &str =
    "id, website_id, rule_id, rule_name, rule_type, action, client_ip, request_summary, user_agent, created_at";

#[async_trait]
impl WafLogRepository for SqliteWafLogRepository {
    async fn create(&self, log: &WafLog) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO waf_logs (website_id, rule_id, rule_name, rule_type, action, client_ip, request_summary, user_agent) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(log.website_id)
        .bind(log.rule_id)
        .bind(&log.rule_name)
        .bind(&log.rule_type)
        .bind(&log.action)
        .bind(&log.client_ip)
        .bind(&log.request_summary)
        .bind(&log.user_agent)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WafLog>, AppError> {
        let sql: String = format!("SELECT {WAF_LOG_COLS} FROM waf_logs WHERE id = ?");
        let row = sqlx::query_as::<_, WafLogRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(WafLog::from);
        Ok(row)
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        website_id: Option<i64>,
    ) -> Result<Vec<WafLog>, AppError> {
        let rows = match website_id.filter(|w| *w > 0) {
            Some(wid) => {
                let sql: String = format!(
                    "SELECT {WAF_LOG_COLS} FROM waf_logs WHERE website_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
                );
                sqlx::query_as::<_, WafLogRow>(sqlx::AssertSqlSafe(sql))
                    .bind(wid)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.read_pool)
                    .await
            }
            None => {
                let sql: String = format!(
                    "SELECT {WAF_LOG_COLS} FROM waf_logs ORDER BY id DESC LIMIT ? OFFSET ?"
                );
                sqlx::query_as::<_, WafLogRow>(sqlx::AssertSqlSafe(sql))
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.read_pool)
                    .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(WafLog::from).collect())
    }

    async fn count(&self, website_id: Option<i64>) -> Result<i64, AppError> {
        let count: i64 = match website_id.filter(|w| *w > 0) {
            Some(wid) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM waf_logs WHERE website_id = ?")
                    .bind(wid)
                    .fetch_one(&self.read_pool)
                    .await
            }
            None => {
                sqlx::query_scalar("SELECT COUNT(*) FROM waf_logs")
                    .fetch_one(&self.read_pool)
                    .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM waf_logs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn clear(&self, website_id: Option<i64>) -> Result<u64, AppError> {
        let result = match website_id.filter(|w| *w > 0) {
            Some(wid) => {
                sqlx::query("DELETE FROM waf_logs WHERE website_id = ?")
                    .bind(wid)
                    .execute(&self.pool)
                    .await
            }
            None => {
                sqlx::query("DELETE FROM waf_logs")
                    .execute(&self.pool)
                    .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.rows_affected())
    }

    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM waf_logs WHERE created_at < datetime('now', ?)")
            .bind(format!("-{} days", days))
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.rows_affected())
    }
}
