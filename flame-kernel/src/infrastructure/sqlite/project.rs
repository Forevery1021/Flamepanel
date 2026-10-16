/// 用户项目仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
use super::*;

pub struct SqliteProjectRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteProjectRepository {
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

const PROJECT_COLS: &str = "id, name, runtime_id, kind, source, source_url, deploy_path, run_command, port, domain, status, process_unit, deploy_type, auto_deploy, webhook_token, procfile_json, build_type, health_check_path, deploy_strategy, created_at, updated_at";

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn list_all(&self) -> Result<Vec<Project>, AppError> {
        let sql: String = format!("SELECT {PROJECT_COLS} FROM projects ORDER BY id");
        let rows = sqlx::query_as::<_, ProjectRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(Project::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Project>, AppError> {
        let sql: String = format!("SELECT {PROJECT_COLS} FROM projects WHERE id = ?");
        let row = sqlx::query_as::<_, ProjectRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(Project::from);
        Ok(row)
    }

    async fn create(&self, project: &Project) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO projects (name, runtime_id, kind, source, source_url, deploy_path, run_command, port, domain, status, process_unit, deploy_type, auto_deploy, webhook_token, procfile_json, build_type, health_check_path, deploy_strategy) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&project.name)
        .bind(project.runtime_id)
        .bind(&project.kind)
        .bind(&project.source)
        .bind(&project.source_url)
        .bind(&project.deploy_path)
        .bind(&project.run_command)
        .bind(project.port)
        .bind(&project.domain)
        .bind(&project.status)
        .bind(&project.process_unit)
        .bind(&project.deploy_type)
        .bind(project.auto_deploy)
        .bind(&project.webhook_token)
        .bind(&project.procfile_json)
        .bind(&project.build_type)
        .bind(&project.health_check_path)
        .bind(&project.deploy_strategy)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, project: &Project) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE projects SET runtime_id = ?, kind = ?, source = ?, source_url = ?, deploy_path = ?, run_command = ?, port = ?, domain = ?, status = ?, process_unit = ?, deploy_type = ?, auto_deploy = ?, webhook_token = ?, procfile_json = ?, build_type = ?, health_check_path = ?, deploy_strategy = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(project.runtime_id)
        .bind(&project.kind)
        .bind(&project.source)
        .bind(&project.source_url)
        .bind(&project.deploy_path)
        .bind(&project.run_command)
        .bind(project.port)
        .bind(&project.domain)
        .bind(&project.status)
        .bind(&project.process_unit)
        .bind(&project.deploy_type)
        .bind(project.auto_deploy)
        .bind(&project.webhook_token)
        .bind(&project.procfile_json)
        .bind(&project.build_type)
        .bind(&project.health_check_path)
        .bind(&project.deploy_strategy)
        .bind(project.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("项目不存在: {}", project.id)));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}
