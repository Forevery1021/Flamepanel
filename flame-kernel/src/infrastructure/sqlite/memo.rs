use super::*;

pub struct SqliteMemoRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteMemoRepository {
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
impl MemoRepository for SqliteMemoRepository {
    async fn list(&self, kind: Option<&str>, done: Option<bool>) -> Result<Vec<Memo>, AppError> {
        let mut sql =
            "SELECT id, content, kind, done, created_at, updated_at FROM memos".to_string();
        let mut conds: Vec<String> = Vec::new();
        if let Some(k) = kind {
            if !k.is_empty() {
                conds.push(format!("kind = '{}'", k.replace('\'', "''")));
            }
        }
        if let Some(d) = done {
            conds.push(format!("done = {}", if d { 1 } else { 0 }));
        }
        if !conds.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conds.join(" AND "));
        }
        sql.push_str(" ORDER BY done ASC, id DESC");
        let memos = sqlx::query_as::<_, MemoRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .into_iter()
            .map(Memo::from)
            .collect();
        Ok(memos)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Memo>, AppError> {
        let memo = sqlx::query_as::<_, MemoRow>(
            "SELECT id, content, kind, done, created_at, updated_at FROM memos WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(Memo::from);
        Ok(memo)
    }

    async fn create(&self, content: &str, kind: &str) -> Result<i64, AppError> {
        let id = sqlx::query("INSERT INTO memos (content, kind) VALUES (?, ?)")
            .bind(content)
            .bind(kind)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, memo: &Memo) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE memos SET content = ?, kind = ?, done = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&memo.content)
        .bind(&memo.kind)
        .bind(memo.done)
        .bind(memo.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Memo not found".into()));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM memos WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Memo not found".into()));
        }
        Ok(())
    }
}
