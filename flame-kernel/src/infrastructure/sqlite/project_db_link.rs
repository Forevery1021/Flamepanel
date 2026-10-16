use super::*;

// ─── 项目数据库联链仓储（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）──────────

/// 项目数据库联链行模型。
#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ProjectDbLinkRow {
    id: i64,
    project_id: i64,
    db_instance_id: i64,
    env_name: String,
    created_at: DateTime<Utc>,
}

impl From<ProjectDbLinkRow> for ProjectDbLink {
    fn from(r: ProjectDbLinkRow) -> Self {
        ProjectDbLink {
            id: r.id,
            project_id: r.project_id,
            db_instance_id: r.db_instance_id,
            env_name: r.env_name,
            created_at: r.created_at,
        }
    }
}

/// SQLite 项目数据库联链仓储（F-14 读写双池）。
pub struct SqliteProjectDbLinkRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

const PROJECT_DB_LINK_COLS: &str = "id, project_id, db_instance_id, env_name, created_at";

impl SqliteProjectDbLinkRepository {
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

#[async_trait]
impl ProjectDbLinkRepository for SqliteProjectDbLinkRepository {
    async fn list_by_project(&self, project_id: i64) -> Result<Vec<ProjectDbLink>, AppError> {
        let sql: String = format!(
            "SELECT {PROJECT_DB_LINK_COLS} FROM project_db_links WHERE project_id = ? ORDER BY id"
        );
        let rows = sqlx::query_as::<_, ProjectDbLinkRow>(sqlx::AssertSqlSafe(sql))
            .bind(project_id)
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(ProjectDbLink::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ProjectDbLink>, AppError> {
        let sql: String =
            format!("SELECT {PROJECT_DB_LINK_COLS} FROM project_db_links WHERE id = ?");
        let row = sqlx::query_as::<_, ProjectDbLinkRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(ProjectDbLink::from);
        Ok(row)
    }

    async fn create(&self, link: &ProjectDbLink) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO project_db_links (project_id, db_instance_id, env_name) VALUES (?, ?, ?)",
        )
        .bind(link.project_id)
        .bind(link.db_instance_id)
        .bind(&link.env_name)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM project_db_links WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete_by_project(&self, project_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM project_db_links WHERE project_id = ?")
            .bind(project_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
