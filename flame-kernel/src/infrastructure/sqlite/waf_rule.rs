use super::*;

pub struct SqliteWafRuleRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteWafRuleRepository {
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

const WAF_RULE_COLS: &str =
    "id, name, rule_type, website_id, pattern, config_json, action, enabled, created_at, updated_at";

#[async_trait]
impl WafRuleRepository for SqliteWafRuleRepository {
    async fn list_all(&self, website_id: Option<i64>) -> Result<Vec<WafRule>, AppError> {
        let rows = match website_id {
            Some(wid) => {
                let sql: String = format!(
                    "SELECT {WAF_RULE_COLS} FROM waf_rules WHERE website_id = ? ORDER BY id"
                );
                sqlx::query_as::<_, WafRuleRow>(sqlx::AssertSqlSafe(sql))
                    .bind(wid)
                    .fetch_all(&self.read_pool)
                    .await
            }
            None => {
                let sql: String = format!("SELECT {WAF_RULE_COLS} FROM waf_rules ORDER BY id");
                sqlx::query_as::<_, WafRuleRow>(sqlx::AssertSqlSafe(sql))
                    .fetch_all(&self.read_pool)
                    .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(WafRule::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WafRule>, AppError> {
        let sql: String = format!("SELECT {WAF_RULE_COLS} FROM waf_rules WHERE id = ?");
        let row = sqlx::query_as::<_, WafRuleRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(WafRule::from);
        Ok(row)
    }

    async fn create(&self, rule: &WafRule) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO waf_rules (name, rule_type, website_id, pattern, config_json, action, enabled) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&rule.name)
        .bind(&rule.rule_type)
        .bind(rule.website_id)
        .bind(&rule.pattern)
        .bind(&rule.config_json)
        .bind(&rule.action)
        .bind(rule.enabled)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, rule: &WafRule) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE waf_rules SET name = ?, rule_type = ?, website_id = ?, pattern = ?, config_json = ?, action = ?, enabled = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&rule.name)
        .bind(&rule.rule_type)
        .bind(rule.website_id)
        .bind(&rule.pattern)
        .bind(&rule.config_json)
        .bind(&rule.action)
        .bind(rule.enabled)
        .bind(rule.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("WAF 规则不存在: {}", rule.id)));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM waf_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_enabled(&self) -> Result<Vec<WafRule>, AppError> {
        let sql: String =
            format!("SELECT {WAF_RULE_COLS} FROM waf_rules WHERE enabled = 1 ORDER BY id");
        let rows = sqlx::query_as::<_, WafRuleRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(WafRule::from).collect())
    }
}
