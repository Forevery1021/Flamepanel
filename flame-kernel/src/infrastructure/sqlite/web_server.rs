use super::*;

pub struct SqliteWebServerRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteWebServerRepository {
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
impl WebServerRepository for SqliteWebServerRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<WebServerInstance>, AppError> {
        let instance = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(WebServerInstance::from);
        Ok(instance)
    }

    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers WHERE engine = ? ORDER BY id",
        )
        .bind(engine)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(WebServerInstance::from).collect();
        Ok(instances)
    }

    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO web_servers (engine, version, status, config_path, binary_path, port, resource_version) VALUES (?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&instance.engine)
        .bind(&instance.version)
        .bind("stopped")
        .bind(&instance.config_path)
        .bind(&instance.binary_path)
        .bind(instance.port)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
        let result = sqlx::query(
            "UPDATE web_servers SET engine = ?, version = ?, status = ?, config_path = ?, binary_path = ?, port = ?, resource_version = resource_version + 1 WHERE id = ? AND resource_version = ?",
        )
        .bind(&instance.engine)
        .bind(&instance.version)
        .bind(&instance.status)
        .bind(&instance.config_path)
        .bind(&instance.binary_path)
        .bind(instance.port)
        .bind(instance.id)
        .bind(instance.resource_version)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        if result.rows_affected() == 0 {
            // 区分“不存在”与“版本冲突”
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM web_servers WHERE id = ?")
                    .bind(instance.id)
                    .fetch_one(&self.read_pool)
                    .await
                    .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            if exists == 0 {
                return Err(AppError::NotFound(format!(
                    "Web server {} not found",
                    instance.id
                )));
            }
            return Err(AppError::Conflict(format!(
                "Web server {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                instance.id, instance.resource_version
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM web_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(WebServerInstance::from).collect();
        Ok(instances)
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<WebServerInstance>, AppError> {
        let limit = limit.clamp(1, 200);
        let instances = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(WebServerInstance::from).collect();
        Ok(instances)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM web_servers")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}
