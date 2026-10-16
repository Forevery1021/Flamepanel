use super::*;

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteSettingsRepository {
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

    /// 启动时 upsert 默认设置，保证 SQLite 模式与 InMemory 模式行为一致。
    pub async fn ensure_defaults(&self) -> Result<(), AppError> {
        for setting in crate::domain::entity::default_settings() {
            sqlx::query(
                "INSERT INTO panel_settings (key, value, description, updated_at) VALUES (?, ?, ?, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET description = excluded.description",
            )
            .bind(&setting.key)
            .bind(&setting.value)
            .bind(&setting.description)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        Ok(())
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let row = sqlx::query_scalar::<_, String>("SELECT value FROM panel_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO panel_settings (key, value, description, updated_at) VALUES (?, ?, '', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn set_many(&self, entries: &[(String, String)]) -> Result<(), AppError> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        for (key, value) in entries {
            sqlx::query(
                "INSERT INTO panel_settings (key, value, description, updated_at) VALUES (?, ?, '', datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            )
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        let rows = sqlx::query_as::<_, PanelSettingRow>(
            "SELECT key, value, description, updated_at FROM panel_settings ORDER BY key",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(PanelSetting::from)
        .collect();
        Ok(rows)
    }

    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let rows = sqlx::query("SELECT key, value FROM panel_settings")
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows
            .iter()
            .map(|r| (r.get::<String, _>("key"), r.get::<String, _>("value")))
            .collect())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<PanelSetting>, AppError> {
        let limit = limit.clamp(1, 200);
        let rows = sqlx::query_as::<_, PanelSettingRow>(
            "SELECT key, value, description, updated_at FROM panel_settings ORDER BY key DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(PanelSetting::from).collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM panel_settings")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}
