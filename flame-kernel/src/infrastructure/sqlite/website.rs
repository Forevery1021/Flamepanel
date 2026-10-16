use super::*;

pub struct SqliteWebsiteRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteWebsiteRepository {
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
impl WebsiteRepository for SqliteWebsiteRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError> {
        let site = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, site_type, runtime_id, project_id, rewrite_preset, created_at, resource_version FROM websites WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(Website::from);
        Ok(site)
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError> {
        let site = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, site_type, runtime_id, project_id, rewrite_preset, created_at, resource_version FROM websites WHERE domain = ?",
        )
        .bind(domain)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(Website::from);
        Ok(site)
    }

    async fn create(&self, website: &Website) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO websites (name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, site_type, runtime_id, project_id, rewrite_preset, resource_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&website.name)
        .bind(&website.domain)
        .bind(&website.root_path)
        .bind(&website.status)
        .bind(website.node_id)
        .bind(&website.engine)
        .bind(website.ssl_enabled)
        .bind(website.proxy_enabled)
        .bind(&website.proxy_pass)
        .bind(&website.site_type)
        .bind(website.runtime_id)
        .bind(website.project_id)
        .bind(&website.rewrite_preset)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, website: &Website) -> Result<(), AppError> {
        // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
        let result = sqlx::query(
            "UPDATE websites SET name=?, domain=?, root_path=?, status=?, node_id=?, engine=?, ssl_enabled=?, proxy_enabled=?, proxy_pass=?, site_type=?, runtime_id=?, project_id=?, rewrite_preset=?, resource_version=resource_version+1 WHERE id=? AND resource_version=?",
        )
        .bind(&website.name)
        .bind(&website.domain)
        .bind(&website.root_path)
        // 状态随更新持久化（此前遗漏：停用站点在刷新后又变回运行中）
        .bind(&website.status)
        .bind(website.node_id)
        .bind(&website.engine)
        .bind(website.ssl_enabled)
        .bind(website.proxy_enabled)
        .bind(&website.proxy_pass)
        .bind(&website.site_type)
        .bind(website.runtime_id)
        .bind(website.project_id)
        .bind(&website.rewrite_preset)
        .bind(website.id)
        .bind(website.resource_version)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        if result.rows_affected() == 0 {
            // 区分“不存在”与“版本冲突”
            let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM websites WHERE id = ?")
                .bind(website.id)
                .fetch_one(&self.read_pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            if exists == 0 {
                return Err(AppError::NotFound(format!(
                    "Website {} not found",
                    website.id
                )));
            }
            return Err(AppError::Conflict(format!(
                "Website {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                website.id, website.resource_version
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM websites WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<Website>, AppError> {
        let sites = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, site_type, runtime_id, project_id, rewrite_preset, created_at, resource_version FROM websites ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(Website::from).collect();
        Ok(sites)
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Website>, AppError> {
        let limit = limit.clamp(1, 200);
        let sites = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, site_type, runtime_id, project_id, rewrite_preset, created_at, resource_version FROM websites ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(Website::from).collect();
        Ok(sites)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM websites")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::run_migrations;

    /// 回归：站点状态（running / stopped）必须随更新持久化——它驱动 vhost 启用 / 停用。
    #[tokio::test]
    async fn update_persists_status() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        let repo = SqliteWebsiteRepository::new(pool);
        let id = repo
            .create(&Website {
                id: 0,
                name: "s".into(),
                domain: "status.example.invalid".into(),
                root_path: "/var/www/s".into(),
                status: "running".into(),
                node_id: 1,
                engine: "nginx".into(),
                ssl_enabled: false,
                proxy_enabled: false,
                proxy_pass: None,
                site_type: "static".into(),
                runtime_id: None,
                project_id: None,
                rewrite_preset: None,
                created_at: chrono::Utc::now(),
                resource_version: 0,
            })
            .await
            .unwrap();
        let mut site = repo.find_by_id(id).await.unwrap().unwrap();
        site.status = "stopped".into();
        repo.update(&site).await.unwrap();
        let stored = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(stored.status, "stopped");
        assert_eq!(stored.resource_version, site.resource_version + 1);
    }
}
