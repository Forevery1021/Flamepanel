use super::*;

pub struct SqlitePermissionRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqlitePermissionRepository {
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
impl PermissionRepository for SqlitePermissionRepository {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError> {
        let perms = sqlx::query_as::<_, PermissionRow>(
            "SELECT id, resource, action, description FROM permissions ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(Permission::from)
        .collect();
        Ok(perms)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError> {
        let perm = sqlx::query_as::<_, PermissionRow>(
            "SELECT id, resource, action, description FROM permissions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(Permission::from);
        Ok(perm)
    }

    async fn find_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<Permission>, AppError> {
        let perm = sqlx::query_as::<_, PermissionRow>(
            "SELECT id, resource, action, description FROM permissions WHERE resource = ? AND action = ?",
        )
        .bind(resource)
        .bind(action)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(Permission::from);
        Ok(perm)
    }

    async fn create(&self, permission: &Permission) -> Result<i64, AppError> {
        let id =
            sqlx::query("INSERT INTO permissions (resource, action, description) VALUES (?, ?, ?)")
                .bind(&permission.resource)
                .bind(&permission.action)
                .bind(&permission.description)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .last_insert_rowid();
        Ok(id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM permissions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
