use super::*;

pub struct SqliteDnsProviderAccountRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqliteDnsProviderAccountRepository {
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

const DNS_PROVIDER_ACCOUNT_COLS: &str = "id, name, provider_type, credentials_json, enabled, last_test_status, last_test_at, created_at, updated_at";

#[async_trait]
impl DnsProviderAccountRepository for SqliteDnsProviderAccountRepository {
    async fn list_all(&self) -> Result<Vec<DnsProviderAccount>, AppError> {
        let sql: String =
            format!("SELECT {DNS_PROVIDER_ACCOUNT_COLS} FROM dns_provider_accounts ORDER BY id");
        let rows = sqlx::query_as::<_, DnsProviderAccountRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(DnsProviderAccount::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DnsProviderAccount>, AppError> {
        let sql: String =
            format!("SELECT {DNS_PROVIDER_ACCOUNT_COLS} FROM dns_provider_accounts WHERE id = ?");
        let row = sqlx::query_as::<_, DnsProviderAccountRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(DnsProviderAccount::from);
        Ok(row)
    }

    async fn create(&self, account: &DnsProviderAccount) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO dns_provider_accounts (name, provider_type, credentials_json, enabled) VALUES (?, ?, ?, ?)",
        )
        .bind(&account.name)
        .bind(&account.provider_type)
        .bind(&account.credentials_json)
        .bind(account.enabled)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, account: &DnsProviderAccount) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE dns_provider_accounts SET name = ?, provider_type = ?, credentials_json = ?, enabled = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&account.name)
        .bind(&account.provider_type)
        .bind(&account.credentials_json)
        .bind(account.enabled)
        .bind(account.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "DNS 厂商账号不存在: {}",
                account.id
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM dns_provider_accounts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn record_test(&self, id: i64, ok: bool) -> Result<(), AppError> {
        let status = if ok { "ok" } else { "failed" };
        let result = sqlx::query(
            "UPDATE dns_provider_accounts SET last_test_status = ?, last_test_at = datetime('now'), updated_at = datetime('now') WHERE id = ?",
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("DNS 厂商账号不存在: {}", id)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod dns_provider_repo_tests {
    use super::*;

    async fn setup() -> SqliteDnsProviderAccountRepository {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("in-memory pool");
        run_migrations(&pool).await.expect("migrations");
        SqliteDnsProviderAccountRepository::new(pool)
    }

    fn sample(name: &str, provider_type: &str) -> DnsProviderAccount {
        DnsProviderAccount {
            id: 0,
            name: name.into(),
            provider_type: provider_type.into(),
            credentials_json: r#"{"api_token":"tok"}"#.into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let repo = setup().await;
        let id = repo.create(&sample("cf", "cloudflare")).await.unwrap();
        let loaded = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "cf");
        assert_eq!(loaded.provider_type, "cloudflare");
        assert!(loaded.enabled);
        assert!(loaded.last_test_status.is_none());

        let mut a = loaded.clone();
        a.name = "cf2".into();
        a.credentials_json = r#"{"api_token":"tok2"}"#.into();
        repo.update(&a).await.unwrap();
        let updated = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(updated.name, "cf2");
        assert!(updated.updated_at >= updated.created_at);

        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
        // 删除幂等
        repo.delete(id).await.unwrap();
    }

    #[tokio::test]
    async fn unique_name_conflict() {
        let repo = setup().await;
        repo.create(&sample("dup", "aliyun")).await.unwrap();
        let err = repo.create(&sample("dup", "aliyun")).await.unwrap_err();
        assert!(
            err.to_string().contains("DB error"),
            "UNIQUE 冲突应报错: {err}"
        );
    }

    #[tokio::test]
    async fn update_missing_not_found() {
        let repo = setup().await;
        let mut a = sample("ghost", "tencent");
        a.id = 999;
        let err = repo.update(&a).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
        let err2 = repo.record_test(999, true).await.unwrap_err();
        assert!(matches!(err2, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn record_test_writes_status() {
        let repo = setup().await;
        let id = repo.create(&sample("t", "huawei")).await.unwrap();
        repo.record_test(id, true).await.unwrap();
        let a = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(a.last_test_status.as_deref(), Some("ok"));
        assert!(a.last_test_at.is_some());
        assert!(a.is_test_ok());
        repo.record_test(id, false).await.unwrap();
        let a2 = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(a2.last_test_status.as_deref(), Some("failed"));
    }

    #[tokio::test]
    async fn migration_idempotent_and_list_empty() {
        // 双跑迁移不报错（幂等），且空表 list 正常
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let repo = SqliteDnsProviderAccountRepository::new_rw(pool.clone(), pool);
        assert!(repo.list_all().await.unwrap().is_empty());
    }
}

// ── WAF 基础防护仓储（主线三 §6.2）──────────────────────────────────────
