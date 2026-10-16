use super::*;

pub struct SqliteBackupAccountRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteBackupAccountRepository {
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

const BACKUP_ACCOUNT_COLS: &str = "id, name, account_type, config_json, enabled, last_test_status, last_test_at, created_at, updated_at";

#[async_trait]
impl BackupAccountRepository for SqliteBackupAccountRepository {
    async fn list_all(&self) -> Result<Vec<BackupAccount>, AppError> {
        let sql: String = format!("SELECT {BACKUP_ACCOUNT_COLS} FROM backup_accounts ORDER BY id");
        let rows = sqlx::query_as::<_, BackupAccountRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(BackupAccount::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<BackupAccount>, AppError> {
        let sql: String = format!("SELECT {BACKUP_ACCOUNT_COLS} FROM backup_accounts WHERE id = ?");
        let row = sqlx::query_as::<_, BackupAccountRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(BackupAccount::from);
        Ok(row)
    }

    async fn create(&self, account: &BackupAccount) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO backup_accounts (name, account_type, config_json, enabled) VALUES (?, ?, ?, ?)",
        )
        .bind(&account.name)
        .bind(&account.account_type)
        .bind(&account.config_json)
        .bind(account.enabled)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, account: &BackupAccount) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE backup_accounts SET name = ?, account_type = ?, config_json = ?, enabled = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&account.name)
        .bind(&account.account_type)
        .bind(&account.config_json)
        .bind(account.enabled)
        .bind(account.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "备份账号不存在: {}",
                account.id
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM backup_accounts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn record_test(&self, id: i64, ok: bool) -> Result<(), AppError> {
        let status = if ok { "ok" } else { "failed" };
        let result = sqlx::query(
            "UPDATE backup_accounts SET last_test_status = ?, last_test_at = datetime('now'), updated_at = datetime('now') WHERE id = ?",
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("备份账号不存在: {}", id)));
        }
        Ok(())
    }
}
