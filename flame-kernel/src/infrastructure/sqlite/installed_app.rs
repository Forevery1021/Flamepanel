use super::*;

pub struct SqliteInstalledAppRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteInstalledAppRepository {
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
impl InstalledAppRepository for SqliteInstalledAppRepository {
    async fn list_all(&self) -> Result<Vec<InstalledApp>, AppError> {
        let rows = sqlx::query_as::<_, InstalledAppRow>(
            "SELECT id, package_key, name, version, mode, status, access_url, install_path, container_name, port, params_json, created_at, updated_at, launch_count FROM installed_apps ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(InstalledApp::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError> {
        let row = sqlx::query_as::<_, InstalledAppRow>(
            "SELECT id, package_key, name, version, mode, status, access_url, install_path, container_name, port, params_json, created_at, updated_at, launch_count FROM installed_apps WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(InstalledApp::from);
        Ok(row)
    }

    async fn create(&self, app: &InstalledApp) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO installed_apps (package_key, name, version, mode, status, access_url, install_path, container_name, port, params_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&app.package_key)
        .bind(&app.name)
        .bind(&app.version)
        .bind(&app.mode)
        .bind(&app.status)
        .bind(&app.access_url)
        .bind(&app.install_path)
        .bind(&app.container_name)
        .bind(app.port)
        .bind(&app.params_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, app: &InstalledApp) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE installed_apps SET version=?, status=?, access_url=?, install_path=?, container_name=?, port=?, params_json=?, launch_count=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&app.version)
        .bind(&app.status)
        .bind(&app.access_url)
        .bind(&app.install_path)
        .bind(&app.container_name)
        .bind(app.port)
        .bind(&app.params_json)
        .bind(app.launch_count)
        .bind(app.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM installed_apps WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
