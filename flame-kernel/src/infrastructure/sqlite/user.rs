use super::*;

pub struct SqliteUserRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteUserRepository {
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
impl UserRepository for SqliteUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, created_at, must_change_password FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(User::from);
        Ok(user)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, created_at, must_change_password FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(User::from);
        Ok(user)
    }

    async fn create(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError> {
        let id = sqlx::query("INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)")
            .bind(username)
            .bind(password_hash)
            .bind(role)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .last_insert_rowid();
        // T14：消除 TOCTOU unwrap。新行创建后若查不到，返回错误而非 panic。
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("User created but not found"))
    }

    async fn update(&self, user: &User) -> Result<(), AppError> {
        let result =
            sqlx::query(
                "UPDATE users SET username = ?, password_hash = ?, role = ?, must_change_password = ? WHERE id = ?",
            )
                .bind(&user.username)
                .bind(&user.password_hash)
                .bind(&user.role)
                .bind(user.must_change_password)
                .bind(user.id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("User not found".into()));
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, created_at, must_change_password FROM users ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(User::from).collect();
        Ok(users)
    }

    async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0 WHERE id = ?")
            .bind(new_password_hash)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>, AppError> {
        let limit = limit.clamp(1, 200);
        let users = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, created_at, must_change_password FROM users ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(User::from).collect();
        Ok(users)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }

    async fn get_password_changed_at(&self, id: i64) -> Result<Option<i64>, AppError> {
        let row =
            sqlx::query_scalar::<_, i64>("SELECT password_changed_at FROM users WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn update_password_changed_at(&self, id: i64, ts: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET password_changed_at = ? WHERE id = ?")
            .bind(ts)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
