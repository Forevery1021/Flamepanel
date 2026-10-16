use super::*;

pub struct SqliteAppPackageRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteAppPackageRepository {
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
impl AppPackageRepository for SqliteAppPackageRepository {
    async fn list_all(&self) -> Result<Vec<AppPackage>, AppError> {
        let rows = sqlx::query_as::<_, AppPackageRow>(
            "SELECT id, key, name, category, format, description, logo, metadata_json, source_path, source, source_url, synced_at, created_at, updated_at FROM app_packages ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(AppPackage::from).collect();
        Ok(rows)
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<AppPackage>, AppError> {
        let row = sqlx::query_as::<_, AppPackageRow>(
            "SELECT id, key, name, category, format, description, logo, metadata_json, source_path, source, source_url, synced_at, created_at, updated_at FROM app_packages WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(AppPackage::from);
        Ok(row)
    }

    async fn create(&self, pkg: &AppPackage) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO app_packages (key, name, category, format, description, logo, metadata_json, source_path, source, source_url, synced_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&pkg.key)
        .bind(&pkg.name)
        .bind(&pkg.category)
        .bind(&pkg.format)
        .bind(&pkg.description)
        .bind(&pkg.logo)
        .bind(&pkg.metadata_json)
        .bind(&pkg.source_path)
        .bind(&pkg.source)
        .bind(&pkg.source_url)
        .bind(pkg.synced_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM app_packages WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn create_many(&self, pkgs: &[AppPackage]) -> Result<usize, AppError> {
        if pkgs.is_empty() {
            return Ok(0);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        let mut count = 0usize;
        for pkg in pkgs {
            // 幂等：已存在则跳过，不视为失败
            let exists = sqlx::query("SELECT 1 FROM app_packages WHERE key=?")
                .bind(&pkg.key)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .is_some();
            if exists {
                continue;
            }
            sqlx::query(
                "INSERT INTO app_packages (key, name, category, format, description, logo, metadata_json, source_path, source, source_url, synced_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&pkg.key)
            .bind(&pkg.name)
            .bind(&pkg.category)
            .bind(&pkg.format)
            .bind(&pkg.description)
            .bind(&pkg.logo)
            .bind(&pkg.metadata_json)
            .bind(&pkg.source_path)
            .bind(&pkg.source)
            .bind(&pkg.source_url)
            .bind(pkg.synced_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            count += 1;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(count)
    }

    async fn update_source_path(&self, id: i64, source_path: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE app_packages SET source_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(source_path)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("应用包不存在: id={}", id)));
        }
        Ok(())
    }
}
