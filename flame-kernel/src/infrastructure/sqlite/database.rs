use super::*;

pub struct SqliteDatabaseRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteDatabaseRepository {
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
impl DatabaseRepository for SqliteDatabaseRepository {
    async fn list_all(&self) -> Result<Vec<DatabaseInstance>, AppError> {
        let rows = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(DatabaseInstance::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError> {
        let row = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(DatabaseInstance::from);
        Ok(row)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<DatabaseInstance>, AppError> {
        let row = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(DatabaseInstance::from);
        Ok(row)
    }

    async fn find_by_type(&self, db_type: &str) -> Result<Vec<DatabaseInstance>, AppError> {
        let rows = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases WHERE db_type = ? ORDER BY id",
        )
        .bind(db_type)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(DatabaseInstance::from).collect();
        Ok(rows)
    }

    async fn create(&self, instance: &DatabaseInstance) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO databases (db_type, name, version, port, status, install_path, data_dir, config_file, root_user, resource_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&instance.db_type)
        .bind(&instance.name)
        .bind(&instance.version)
        .bind(instance.port)
        .bind(&instance.status)
        .bind(&instance.install_path)
        .bind(&instance.data_dir)
        .bind(&instance.config_file)
        .bind(&instance.root_user)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, instance: &DatabaseInstance) -> Result<(), AppError> {
        // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
        let result = sqlx::query(
            "UPDATE databases SET version=?, port=?, status=?, install_path=?, data_dir=?, config_file=?, root_user=?, updated_at=datetime('now'), resource_version=resource_version+1 WHERE id=? AND resource_version=?",
        )
        .bind(&instance.version)
        .bind(instance.port)
        .bind(&instance.status)
        .bind(&instance.install_path)
        .bind(&instance.data_dir)
        .bind(&instance.config_file)
        .bind(&instance.root_user)
        .bind(instance.id)
        .bind(instance.resource_version)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        if result.rows_affected() == 0 {
            // 区分“不存在”与“版本冲突”
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM databases WHERE id = ?")
                    .bind(instance.id)
                    .fetch_one(&self.read_pool)
                    .await
                    .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            if exists == 0 {
                return Err(AppError::NotFound(format!(
                    "Database instance {} not found",
                    instance.id
                )));
            }
            return Err(AppError::Conflict(format!(
                "Database instance {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                instance.id, instance.resource_version
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM databases WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE databases SET status=?, updated_at=datetime('now') WHERE id=?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_status_batch(&self, updates: &[(i64, String)]) -> Result<(), AppError> {
        if updates.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        for (id, status) in updates {
            sqlx::query("UPDATE databases SET status=?, updated_at=datetime('now') WHERE id=?")
                .bind(status)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<DatabaseInstance>, AppError> {
        let limit = limit.clamp(1, 200);
        let rows = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(DatabaseInstance::from).collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM databases")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}
