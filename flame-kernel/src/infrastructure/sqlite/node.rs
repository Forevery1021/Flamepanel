use super::*;

pub struct SqliteNodeRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteNodeRepository {
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
impl NodeRepository for SqliteNodeRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<ServerNode>, AppError> {
        let node = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(ServerNode::from);
        Ok(node)
    }

    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError> {
        let node = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes WHERE hostname = ?",
        )
        .bind(hostname)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(ServerNode::from);
        Ok(node)
    }

    async fn create(&self, node: &ServerNode) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO nodes (name, hostname, ip_address, status, auth_token, agent_port) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&node.name)
        .bind(&node.hostname)
        .bind(&node.ip_address)
        .bind(&node.status)
        .bind(&node.auth_token)
        .bind(node.agent_port as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, node: &ServerNode) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE nodes SET name = ?, hostname = ?, ip_address = ?, status = ?, agent_port = ? WHERE id = ?",
        )
        .bind(&node.name)
        .bind(&node.hostname)
        .bind(&node.ip_address)
        .bind(&node.status)
        .bind(node.agent_port as i64)
        .bind(node.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Node not found".into()));
        }
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<ServerNode>, AppError> {
        let nodes = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes ORDER BY id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ServerNode::from).collect();
        Ok(nodes)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM nodes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_heartbeat(&self, id: i64, metrics_json: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE nodes SET last_heartbeat_at = datetime('now'), metrics_json = ? WHERE id = ?",
        )
        .bind(metrics_json)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Node not found".into()));
        }
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ServerNode>, AppError> {
        let limit = limit.clamp(1, 200);
        let nodes = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ServerNode::from).collect();
        Ok(nodes)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM nodes")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }

    /// 离线扫描条件化：直接按 last_heartbeat_at 早于阈值的条件查询，避免全量加载后过滤。
    async fn list_stale_heartbeats(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ServerNode>, AppError> {
        let nodes = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes WHERE last_heartbeat_at IS NULL OR last_heartbeat_at < ?",
        )
        .bind(before)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ServerNode::from).collect();
        Ok(nodes)
    }
}
