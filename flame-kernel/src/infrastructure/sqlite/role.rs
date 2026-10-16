use super::*;

pub struct SqliteRoleRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteRoleRepository {
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
impl RoleRepository for SqliteRoleRepository {
    async fn list_all(&self) -> Result<Vec<Role>, AppError> {
        let roles =
            sqlx::query_as::<_, RoleRow>("SELECT id, name, description FROM roles ORDER BY id")
                .fetch_all(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .into_iter()
                .map(Role::from)
                .collect();
        Ok(roles)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        let role =
            sqlx::query_as::<_, RoleRow>("SELECT id, name, description FROM roles WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .map(Role::from);
        Ok(role)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError> {
        let role =
            sqlx::query_as::<_, RoleRow>("SELECT id, name, description FROM roles WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .map(Role::from);
        Ok(role)
    }

    async fn create(&self, role: &Role) -> Result<i64, AppError> {
        let id = sqlx::query("INSERT INTO roles (name, description) VALUES (?, ?)")
            .bind(&role.name)
            .bind(&role.description)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, role: &Role) -> Result<(), AppError> {
        sqlx::query("UPDATE roles SET name = ?, description = ? WHERE id = ?")
            .bind(&role.name)
            .bind(&role.description)
            .bind(role.id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT permission_id FROM role_permissions WHERE role_id = ?")
                .bind(role_id)
                .fetch_all(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError> {
        // T7：DELETE + INSERT 循环放入同一事务，避免半截状态；DELETE 错误不再吞掉。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        for pid in permission_ids {
            sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                .bind(role_id)
                .bind(pid)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }
}
