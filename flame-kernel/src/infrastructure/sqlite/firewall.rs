use super::*;

pub struct SqliteFirewallRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteFirewallRepository {
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
impl FirewallRepository for SqliteFirewallRepository {
    async fn list_all(&self) -> Result<Vec<FirewallRule>, AppError> {
        let rows = sqlx::query_as::<_, FirewallRuleRow>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules ORDER BY priority, id",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(FirewallRule::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<FirewallRule>, AppError> {
        let row = sqlx::query_as::<_, FirewallRuleRow>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(FirewallRule::from);
        Ok(row)
    }

    async fn create(&self, rule: &FirewallRule) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO firewall_rules (name, description, protocol, port, source, destination, action, enabled, priority, direction) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.protocol)
        .bind(&rule.port)
        .bind(&rule.source)
        .bind(&rule.destination)
        .bind(&rule.action)
        .bind(rule.enabled)
        .bind(rule.priority)
        .bind(&rule.direction)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, rule: &FirewallRule) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE firewall_rules SET name=?, description=?, protocol=?, port=?, source=?, destination=?, action=?, enabled=?, priority=?, direction=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.protocol)
        .bind(&rule.port)
        .bind(&rule.source)
        .bind(&rule.destination)
        .bind(&rule.action)
        .bind(rule.enabled)
        .bind(rule.priority)
        .bind(&rule.direction)
        .bind(rule.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM firewall_rules WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError> {
        sqlx::query("UPDATE firewall_rules SET enabled=?, updated_at=datetime('now') WHERE id=?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError> {
        // T7：循环写放入单事务，避免批量重排中途失败留下半截优先级。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        let mut priority = 10i32;
        for id in ids {
            sqlx::query(
                "UPDATE firewall_rules SET priority=?, updated_at=datetime('now') WHERE id=?",
            )
            .bind(priority)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            priority += 10;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<FirewallRule>, AppError> {
        let limit = limit.clamp(1, 200);
        let rows = sqlx::query_as::<_, FirewallRuleRow>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(FirewallRule::from).collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM firewall_rules")
            .fetch_one(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}
// ─── 应用商店仓储 ────────────────────────────────────────────────────────────
