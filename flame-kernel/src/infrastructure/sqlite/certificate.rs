use super::*;

// ─── SSL 证书记录仓储（M-B 主线三 §6.1 · Stage 1 数据层）──────────────────

/// 证书记录行模型。
#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct CertificateRow {
    id: i64,
    domains: String,
    source: String,
    dns_provider: Option<String>,
    auto_renew: bool,
    cert_path: Option<String>,
    key_path: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    last_renewed_at: Option<DateTime<Utc>>,
    website_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CertificateRow> for Certificate {
    fn from(r: CertificateRow) -> Self {
        Certificate {
            id: r.id,
            domains: r.domains,
            source: r.source,
            dns_provider: r.dns_provider,
            auto_renew: r.auto_renew,
            cert_path: r.cert_path,
            key_path: r.key_path,
            expires_at: r.expires_at,
            last_renewed_at: r.last_renewed_at,
            website_id: r.website_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// SQLite 证书记录仓储（F-14 读写双池）。
pub struct SqliteCertificateRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

const CERTIFICATE_COLS: &str = "id, domains, source, dns_provider, auto_renew, cert_path, key_path, expires_at, last_renewed_at, website_id, created_at, updated_at";

impl SqliteCertificateRepository {
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
impl CertificateRepository for SqliteCertificateRepository {
    async fn list_all(&self) -> Result<Vec<Certificate>, AppError> {
        let sql: String = format!("SELECT {CERTIFICATE_COLS} FROM certificates ORDER BY id");
        let rows = sqlx::query_as::<_, CertificateRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(Certificate::from).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Certificate>, AppError> {
        let sql: String = format!("SELECT {CERTIFICATE_COLS} FROM certificates WHERE id = ?");
        let row = sqlx::query_as::<_, CertificateRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(Certificate::from);
        Ok(row)
    }

    async fn find_by_domains(&self, domains: &str) -> Result<Option<Certificate>, AppError> {
        let sql: String = format!("SELECT {CERTIFICATE_COLS} FROM certificates WHERE domains = ?");
        let row = sqlx::query_as::<_, CertificateRow>(sqlx::AssertSqlSafe(sql))
            .bind(domains)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(Certificate::from);
        Ok(row)
    }

    async fn find_by_website(&self, website_id: i64) -> Result<Option<Certificate>, AppError> {
        let sql: String =
            format!("SELECT {CERTIFICATE_COLS} FROM certificates WHERE website_id = ? ORDER BY id DESC LIMIT 1");
        let row = sqlx::query_as::<_, CertificateRow>(sqlx::AssertSqlSafe(sql))
            .bind(website_id)
            .fetch_optional(&self.read_pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .map(Certificate::from);
        Ok(row)
    }

    async fn create(&self, cert: &Certificate) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO certificates (domains, source, dns_provider, auto_renew, cert_path, key_path, expires_at, last_renewed_at, website_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&cert.domains)
        .bind(&cert.source)
        .bind(&cert.dns_provider)
        .bind(cert.auto_renew)
        .bind(&cert.cert_path)
        .bind(&cert.key_path)
        .bind(cert.expires_at)
        .bind(cert.last_renewed_at)
        .bind(cert.website_id)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db) if db.code().as_deref() == Some("2067") => {
                AppError::Conflict("相同域名集合的证书已存在".to_string())
            }
            _ => AppError::internal(format!("DB error: {}", e)),
        })?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, cert: &Certificate) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE certificates SET source = ?, dns_provider = ?, auto_renew = ?, cert_path = ?, key_path = ?, expires_at = ?, last_renewed_at = ?, website_id = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&cert.source)
        .bind(&cert.dns_provider)
        .bind(cert.auto_renew)
        .bind(&cert.cert_path)
        .bind(&cert.key_path)
        .bind(cert.expires_at)
        .bind(cert.last_renewed_at)
        .bind(cert.website_id)
        .bind(cert.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("证书记录不存在".to_string()));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM certificates WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod runtime_repo_tests {
    use super::*;

    async fn setup_runtime() -> SqliteRuntimeRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：对同一库重复执行必须安全（SOP 自测要求）
        run_migrations(&pool).await.unwrap();
        SqliteRuntimeRepository::new(pool)
    }

    fn sample(kind: &str, version: &str) -> Runtime {
        Runtime {
            id: 0,
            kind: kind.to_string(),
            version: version.to_string(),
            install_path: format!("/opt/flamepanel/runtimes/{}/{}", kind, version),
            status: "not_installed".to_string(),
            is_default: false,
            metadata_json: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn runtime_crud_roundtrip() {
        let repo = setup_runtime().await;
        let id = repo.create(&sample("php", "8.2")).await.unwrap();

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.kind, "php");
        assert_eq!(found.version, "8.2");
        assert_eq!(found.status, "not_installed");
        assert!(!found.is_default);

        let mut r = found;
        r.status = "installed".to_string();
        r.metadata_json = Some(r#"{"extensions":["curl","mbstring"]}"#.to_string());
        repo.update(&r).await.unwrap();
        let updated = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(updated.status, "installed");
        assert!(updated.metadata_json.is_some());

        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn runtime_unique_kind_version_rejected() {
        let repo = setup_runtime().await;
        repo.create(&sample("node", "20")).await.unwrap();
        assert!(
            repo.create(&sample("node", "20")).await.is_err(),
            "UNIQUE(kind, version) 应拒绝同语言同版本重复"
        );
        // 同版本不同 kind 允许共存
        assert!(repo.create(&sample("python", "3.12")).await.is_ok());
    }

    #[tokio::test]
    async fn runtime_set_default_scoped_to_kind() {
        let repo = setup_runtime().await;
        let php82 = repo.create(&sample("php", "8.2")).await.unwrap();
        let php83 = repo.create(&sample("php", "8.3")).await.unwrap();
        let node20 = repo.create(&sample("node", "20")).await.unwrap();

        repo.set_default(php82).await.unwrap();
        let r = repo.find_by_id(php82).await.unwrap().unwrap();
        assert!(r.is_default);

        // 切换默认：同 kind 旧默认被清，跨 kind 不受影响
        repo.set_default(php83).await.unwrap();
        assert!(
            !repo.find_by_id(php82).await.unwrap().unwrap().is_default,
            "同 kind 旧默认应被清除"
        );
        assert!(repo.find_by_id(php83).await.unwrap().unwrap().is_default);

        repo.set_default(node20).await.unwrap();
        assert!(repo.find_by_id(php83).await.unwrap().unwrap().is_default);
        assert!(repo.find_by_id(node20).await.unwrap().unwrap().is_default);

        // 不存在的 id → NotFound
        assert!(repo.set_default(999).await.is_err());
    }

    async fn setup_project() -> SqliteProjectRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
        SqliteProjectRepository::new(pool)
    }

    fn sample_project(name: &str) -> Project {
        Project {
            id: 0,
            name: name.to_string(),
            runtime_id: Some(1),
            kind: "node".to_string(),
            source: "git".to_string(),
            source_url: Some("https://example.com/repo.git".to_string()),
            deploy_path: format!("/opt/flamepanel/projects/{}", name),
            run_command: Some("npm run start".to_string()),
            port: Some(3000),
            domain: None,
            status: "stopped".to_string(),
            process_unit: None,
            deploy_type: "manual".to_string(),
            auto_deploy: false,
            webhook_token: String::new(),
            procfile_json: None,
            build_type: "runtime".into(),
            health_check_path: String::new(),
            deploy_strategy: "restart".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn project_crud_roundtrip() {
        let repo = setup_project().await;
        let id = repo.create(&sample_project("blog")).await.unwrap();

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.name, "blog");
        assert_eq!(found.kind, "node");
        assert_eq!(found.source, "git");
        assert_eq!(found.port, Some(3000));
        assert_eq!(found.status, "stopped");

        let mut p = found;
        p.status = "running".to_string();
        p.process_unit = Some(format!("flame-proj-{}.service", id));
        p.domain = Some("blog.example.com".to_string());
        repo.update(&p).await.unwrap();
        let updated = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(updated.status, "running");
        assert_eq!(updated.domain.as_deref(), Some("blog.example.com"));

        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn project_unique_name_rejected() {
        let repo = setup_project().await;
        repo.create(&sample_project("demo")).await.unwrap();
        assert!(
            repo.create(&sample_project("demo")).await.is_err(),
            "UNIQUE(name) 应拒绝重名项目"
        );
    }

    /// Stage A-7（Doc/20 §5.8）：projects 表 webhook 部署三列——写入回读 + 老库升级。
    #[tokio::test]
    async fn project_webhook_columns_roundtrip() {
        let repo = setup_project().await;
        let mut p = sample_project("wh-proj");
        p.source = "git".into();
        p.deploy_type = "webhook".into();
        p.auto_deploy = true;
        p.webhook_token = "tok-abc123".into();
        let id = repo.create(&p).await.unwrap();
        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.deploy_type, "webhook");
        assert!(found.auto_deploy);
        assert_eq!(found.webhook_token, "tok-abc123");

        // 默认值：manual / false / 空
        let q = repo.create(&sample_project("plain")).await.unwrap();
        let q = repo.find_by_id(q).await.unwrap().unwrap();
        assert_eq!(q.deploy_type, "manual");
        assert!(!q.auto_deploy);
        assert!(q.webhook_token.is_empty());
    }

    /// Stage A-9（Doc/20 §5.8 ④）：projects 表零停机切换两列——
    /// health_check_path / deploy_strategy 写入回读 + 老库升级默认值。
    #[tokio::test]
    async fn project_zero_downtime_columns_roundtrip() {
        let repo = setup_project().await;
        let mut p = sample_project("zd-proj");
        p.health_check_path = "/health".into();
        p.deploy_strategy = "rolling".into();
        let id = repo.create(&p).await.unwrap();
        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.health_check_path, "/health");
        assert_eq!(found.deploy_strategy, "rolling");
        assert_eq!(
            found.health_check_url().as_deref(),
            Some("http://127.0.0.1:3000/health")
        );

        // 老库升级默认值：restart / 空
        let q = repo.create(&sample_project("legacy")).await.unwrap();
        let q = repo.find_by_id(q).await.unwrap().unwrap();
        assert_eq!(q.deploy_strategy, "restart");
        assert!(q.health_check_path.is_empty());
        assert!(q.health_check_url().is_none());

        // 更新回读
        let mut upd = found;
        upd.health_check_path = "/healthz".into();
        upd.deploy_strategy = "restart".into();
        repo.update(&upd).await.unwrap();
        let upd = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(upd.health_check_path, "/healthz");
        assert_eq!(upd.deploy_strategy, "restart");
    }

    /// Stage A-8（Doc/20 §5.8 ③）：projects 表 procfile_json 列——写入回读 + 老库升级默认 NULL。
    /// Stage A-8（Doc/20 §5.8 ②）：projects 表 build_type 列——写入回读 + 老库升级默认 runtime。
    #[tokio::test]
    async fn project_build_type_column_roundtrip() {
        let repo = setup_project().await;
        let mut p = sample_project("builder-proj");
        p.build_type = "dockerfile".into();
        let id = repo.create(&p).await.unwrap();
        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.build_type, "dockerfile");

        // 老库默认 runtime
        let q = repo.create(&sample_project("default-bt")).await.unwrap();
        let q = repo.find_by_id(q).await.unwrap().unwrap();
        assert_eq!(q.build_type, "runtime");

        // 更新回读
        let mut upd = found;
        upd.build_type = "runtime".into();
        repo.update(&upd).await.unwrap();
        let upd = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(upd.build_type, "runtime");
    }

    #[tokio::test]
    async fn project_procfile_column_roundtrip() {
        let repo = setup_project().await;
        let mut p = sample_project("pf-proj");
        p.procfile_json = Some(r#"{"web":"node server.js"}"#.into());
        let id = repo.create(&p).await.unwrap();
        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(
            found.procfile_json.as_deref(),
            Some(r#"{"web":"node server.js"}"#)
        );

        // 默认 NULL
        let q = repo.create(&sample_project("no-pf")).await.unwrap();
        let q = repo.find_by_id(q).await.unwrap().unwrap();
        assert!(q.procfile_json.is_none());

        // 更新回读（清空声明）
        let mut upd = found;
        upd.procfile_json = None;
        repo.update(&upd).await.unwrap();
        let upd = repo.find_by_id(id).await.unwrap().unwrap();
        assert!(upd.procfile_json.is_none());
    }
}

#[cfg(test)]
mod rw_pool_tests {
    use super::*;

    /// F-14: 读写双池——读方法必须走 read_pool，写方法必须走 write_pool。
    /// write 池写入的数据经 WAL 立即可被 read 池读到（同库共享）。
    #[tokio::test]
    async fn rw_pools_share_one_database() {
        use std::str::FromStr;
        // 内存库跨连接不共享：改用独立临时文件库验证双池同库语义
        let dir = std::env::temp_dir().join(format!("flame_rw_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("rw_test.db");
        let url = format!("sqlite://{}", db_path.display());
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&url)
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let write = sqlx::SqlitePool::connect_with(opts.clone()).await.unwrap();
        let read = sqlx::SqlitePool::connect_with(opts).await.unwrap();
        run_migrations(&write).await.unwrap();

        let repo = SqliteUserRepository::new_rw(write, read);
        let user = repo
            .create("rw_user", "$2y$05$test", "admin")
            .await
            .unwrap();
        // 读池立即读到写池刚写入的数据（WAL 语义）
        let found = repo.find_by_username("rw_user").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);
        repo.delete(user.id).await.unwrap();
        assert!(repo.find_by_username("rw_user").await.unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn rw_pools_new_keeps_same_pool_for_both() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // new() 单池模式下读写同池，行为向后兼容
        let repo = SqliteUserRepository::new(pool);
        let user = repo.create("single", "$2y$05$test", "admin").await.unwrap();
        assert!(repo.find_by_username("single").await.unwrap().is_some());
        repo.delete(user.id).await.unwrap();
    }
}

#[cfg(test)]
mod website_repo_tests {
    use super::*;

    /// 伪静态套用列（Doc/20 §5.5）：迁移幂等（重复 run）+ 读写回读 + NULL 语义。
    #[tokio::test]
    async fn rewrite_preset_column_roundtrip_and_idempotent_migration() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 幂等：重复执行不报错（老库升级路径）
        run_migrations(&pool).await.unwrap();

        let repo = SqliteWebsiteRepository::new(pool.clone());
        let now = chrono::Utc::now();
        let site = Website {
            id: 0,
            name: "rw".into(),
            domain: "rw.migrate.test".into(),
            root_path: "/var/www/rw".into(),
            status: "active".into(),
            node_id: 1,
            engine: "nginx".into(),
            ssl_enabled: false,
            proxy_enabled: false,
            proxy_pass: None,
            site_type: "runtime_php".into(),
            runtime_id: None,
            project_id: None,
            rewrite_preset: None,
            created_at: now,
            resource_version: 0,
        };
        let id = repo.create(&site).await.unwrap();
        assert_eq!(
            repo.find_by_id(id).await.unwrap().unwrap().rewrite_preset,
            None
        );

        // 套用 → 回读
        let mut stored = repo.find_by_id(id).await.unwrap().unwrap();
        stored.rewrite_preset = Some("wordpress".into());
        repo.update(&stored).await.unwrap();
        let fetched = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(fetched.rewrite_preset.as_deref(), Some("wordpress"));
        assert_eq!(
            repo.find_by_domain("rw.migrate.test")
                .await
                .unwrap()
                .unwrap()
                .rewrite_preset
                .as_deref(),
            Some("wordpress")
        );
        assert_eq!(
            repo.list_all().await.unwrap()[0].rewrite_preset.as_deref(),
            Some("wordpress")
        );

        // 清除 → 回落 NULL
        let mut cleared = repo.find_by_id(id).await.unwrap().unwrap();
        cleared.rewrite_preset = None;
        repo.update(&cleared).await.unwrap();
        assert_eq!(
            repo.find_by_id(id).await.unwrap().unwrap().rewrite_preset,
            None
        );
    }
}

#[cfg(test)]
mod app_source_tests {
    use super::*;

    async fn setup() -> SqliteAppSourceRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：对同一库重复执行必须安全（SOP 自测要求）
        run_migrations(&pool).await.unwrap();
        SqliteAppSourceRepository::new(pool)
    }

    fn sample(name: &str, url: &str, source_type: &str) -> AppSource {
        AppSource {
            id: 0,
            name: name.to_string(),
            source_type: source_type.to_string(),
            url: url.to_string(),
            enabled: true,
            public_key: Some("ed25519-pubkey-placeholder".to_string()),
            last_synced_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_find_and_list() {
        let repo = setup().await;
        let id = repo
            .create(&sample(
                "1Panel 官方源",
                "https://github.com/1Panel-dev/appstore",
                "git",
            ))
            .await
            .unwrap();
        assert!(id > 0);

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.name, "1Panel 官方源");
        assert_eq!(found.source_type, "git");
        assert!(found.enabled);
        assert_eq!(
            found.public_key.as_deref(),
            Some("ed25519-pubkey-placeholder")
        );
        assert!(found.last_synced_at.is_none());

        let by_url = repo
            .find_by_url("https://github.com/1Panel-dev/appstore")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_url.id, id);

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn create_duplicate_url_rejected() {
        let repo = setup().await;
        let url = "https://example.com/appstore.git";
        repo.create(&sample("a", url, "git")).await.unwrap();
        let dup = repo.create(&sample("b", url, "git")).await;
        assert!(dup.is_err(), "UNIQUE(url) 约束应拒绝重复添加");
    }

    #[tokio::test]
    async fn update_and_touch_synced() {
        let repo = setup().await;
        let id = repo
            .create(&sample("s", "https://s.example.com", "http"))
            .await
            .unwrap();

        let mut s = repo.find_by_id(id).await.unwrap().unwrap();
        s.enabled = false;
        s.name = "renamed".to_string();
        s.public_key = None;
        repo.update(&s).await.unwrap();

        let after = repo.find_by_id(id).await.unwrap().unwrap();
        assert!(!after.enabled);
        assert_eq!(after.name, "renamed");
        assert!(after.public_key.is_none());
        assert!(after.last_synced_at.is_none());

        repo.touch_synced(id).await.unwrap();
        let synced = repo.find_by_id(id).await.unwrap().unwrap();
        assert!(
            synced.last_synced_at.is_some(),
            "touch_synced 应写入 last_synced_at"
        );
    }

    #[tokio::test]
    async fn delete_and_missing_find() {
        let repo = setup().await;
        let id = repo
            .create(&sample("d", "https://d.example.com", "git"))
            .await
            .unwrap();
        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
        assert!(repo
            .find_by_url("https://d.example.com")
            .await
            .unwrap()
            .is_none());
        repo.delete(id).await.unwrap(); // 删除不存在 id 不报错（幂等）
    }

    /// app_packages 新三列：默认 local，可写入 git 来源与同步时间并回读。
    #[tokio::test]
    async fn app_package_source_columns_roundtrip() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        let repo = SqliteAppPackageRepository::new(pool.clone());

        let mut pkg = AppPackage {
            id: 0,
            key: "wordpress".into(),
            name: "WordPress".into(),
            category: "blog".into(),
            format: "1panel".into(),
            description: "".into(),
            logo: None,
            metadata_json: "{}".into(),
            source_path: None,
            source: "git".into(),
            source_url: Some("https://github.com/1Panel-dev/appstore".into()),
            synced_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        repo.create(&pkg).await.unwrap();

        let got = repo.find_by_key("wordpress").await.unwrap().unwrap();
        assert_eq!(got.source, "git");
        assert_eq!(
            got.source_url.as_deref(),
            Some("https://github.com/1Panel-dev/appstore")
        );
        assert!(got.synced_at.is_some());

        // 本地包默认 source = local（服务层构造时的缺省语义）
        pkg.key = "local-app".into();
        pkg.source = "local".into();
        pkg.source_url = None;
        pkg.synced_at = None;
        repo.create(&pkg).await.unwrap();
        let local = repo.find_by_key("local-app").await.unwrap().unwrap();
        assert_eq!(local.source, "local");
        assert!(local.source_url.is_none() && local.synced_at.is_none());
    }
}

#[cfg(test)]
mod backup_account_tests {
    use super::*;

    async fn setup() -> SqliteBackupAccountRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：对同一库重复执行必须安全（SOP 自测要求）
        run_migrations(&pool).await.unwrap();
        SqliteBackupAccountRepository::new(pool)
    }

    fn sample(name: &str, account_type: &str) -> BackupAccount {
        BackupAccount {
            id: 0,
            name: name.to_string(),
            account_type: account_type.to_string(),
            config_json: r#"{"endpoint":"http://minio.local:9000","bucket":"flame"}"#.to_string(),
            enabled: true,
            last_test_status: None,
            last_test_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn backup_account_create_find_and_list() {
        let repo = setup().await;
        let id = repo.create(&sample("MinIO 本地", "s3")).await.unwrap();
        assert!(id > 0);

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.name, "MinIO 本地");
        assert_eq!(found.account_type, "s3");
        assert!(found.enabled);
        assert!(found.last_test_status.is_none());
        assert!(found.last_test_at.is_none());
        assert!(found.config_json.contains("minio.local"));

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn backup_account_duplicate_name_rejected() {
        let repo = setup().await;
        repo.create(&sample("云备份", "s3")).await.unwrap();
        let dup = repo.create(&sample("云备份", "sftp")).await;
        assert!(dup.is_err(), "UNIQUE(name) 约束应拒绝重复命名");
    }

    #[tokio::test]
    async fn backup_account_update_and_record_test() {
        let repo = setup().await;
        let id = repo.create(&sample("nas", "sftp")).await.unwrap();

        let mut a = repo.find_by_id(id).await.unwrap().unwrap();
        a.enabled = false;
        a.name = "renamed".to_string();
        repo.update(&a).await.unwrap();

        let after = repo.find_by_id(id).await.unwrap().unwrap();
        assert!(!after.enabled);
        assert_eq!(after.name, "renamed");
        assert!(after.last_test_status.is_none(), "编辑不应重置测试记录");

        repo.record_test(id, true).await.unwrap();
        let ok = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(ok.last_test_status.as_deref(), Some("ok"));
        assert!(ok.last_test_at.is_some(), "record_test 应写入 last_test_at");

        repo.record_test(id, false).await.unwrap();
        let failed = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(failed.last_test_status.as_deref(), Some("failed"));
    }

    #[tokio::test]
    async fn backup_account_delete_and_missing_update() {
        let repo = setup().await;
        let id = repo.create(&sample("tmp", "local")).await.unwrap();
        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
        repo.delete(id).await.unwrap(); // 删除不存在 id 不报错（幂等）

        let mut missing = sample("ghost", "local");
        missing.id = 999;
        assert!(repo.update(&missing).await.is_err());
        assert!(repo.record_test(999, true).await.is_err());
    }
}

#[cfg(test)]
mod backup_job_tests {
    use super::*;

    async fn setup() -> SqliteBackupJobRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：对同一库重复执行必须安全（SOP 自测要求）
        run_migrations(&pool).await.unwrap();
        SqliteBackupJobRepository::new(pool)
    }

    fn sample(name: &str) -> BackupJob {
        BackupJob {
            id: 0,
            name: name.to_string(),
            scope: "panel".to_string(),
            scope_key: None,
            schedule: "0 3 * * *".to_string(),
            target_account_id: 1,
            retention: 7,
            enabled: true,
            last_status: "never".to_string(),
            last_message: None,
            last_remote_key: None,
            last_run_at: None,
            next_run_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let repo = setup().await;
        let id = repo.create(&sample("nightly")).await.unwrap();

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.name, "nightly");
        assert_eq!(found.scope, "panel");
        assert_eq!(found.retention, 7);
        assert!(found.enabled);
        assert_eq!(found.last_status, "never");

        let mut j = found;
        j.retention = 3;
        j.enabled = false;
        repo.update(&j).await.unwrap();
        let updated = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(updated.retention, 3);
        assert!(!updated.enabled);

        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn create_duplicate_name_rejected() {
        let repo = setup().await;
        repo.create(&sample("a")).await.unwrap();
        assert!(
            repo.create(&sample("a")).await.is_err(),
            "UNIQUE(name) 应拒绝重名"
        );
    }

    #[tokio::test]
    async fn record_run_writes_history_and_next_run() {
        let repo = setup().await;
        let id = repo.create(&sample("r")).await.unwrap();
        let next = chrono::Utc::now() + chrono::Duration::hours(1);
        repo.record_run(id, true, None, Some("panel/x.db".into()), Some(next))
            .await
            .unwrap();

        let j = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(j.last_status, "success");
        assert_eq!(j.last_remote_key.as_deref(), Some("panel/x.db"));
        assert!(j.last_run_at.is_some());
        assert_eq!(j.next_run_at, Some(next));

        // 失败路径
        repo.record_run(id, false, Some("boom".into()), None, None)
            .await
            .unwrap();
        let j = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(j.last_status, "failed");
        assert_eq!(j.last_message.as_deref(), Some("boom"));
        assert!(j.last_remote_key.is_none());

        // 不存在的任务 → NotFound
        assert!(repo.record_run(999, true, None, None, None).await.is_err());
    }
}

#[cfg(test)]
mod project_db_link_tests {
    use super::*;

    async fn setup() -> SqliteProjectDbLinkRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：对同一库重复执行必须安全（SOP 自测要求）
        run_migrations(&pool).await.unwrap();
        SqliteProjectDbLinkRepository::new(pool)
    }

    fn sample(project_id: i64, db_instance_id: i64, env: &str) -> ProjectDbLink {
        ProjectDbLink {
            id: 0,
            project_id,
            db_instance_id,
            env_name: env.into(),
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn crud_and_list_by_project() {
        let repo = setup().await;
        let l1 = repo.create(&sample(1, 10, "DATABASE_URL")).await.unwrap();
        let l2 = repo.create(&sample(1, 11, "REDIS_URL")).await.unwrap();
        repo.create(&sample(2, 10, "DATABASE_URL")).await.unwrap();

        let links = repo.list_by_project(1).await.unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].id, l1);
        assert_eq!(links[0].env_name, "DATABASE_URL");
        assert_eq!(links[1].id, l2);
        // 项目 2 不受项目 1 影响
        assert_eq!(repo.list_by_project(2).await.unwrap().len(), 1);
        // 详情
        assert_eq!(
            repo.find_by_id(l1).await.unwrap().unwrap().env_name,
            "DATABASE_URL"
        );
        assert!(repo.find_by_id(999).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn unique_constraints_reject_duplicates() {
        let repo = setup().await;
        repo.create(&sample(1, 10, "DATABASE_URL")).await.unwrap();
        // 同项目同 env
        assert!(repo.create(&sample(1, 11, "DATABASE_URL")).await.is_err());
        // 同项目同实例
        assert!(repo.create(&sample(1, 10, "MYSQL_URL")).await.is_err());
        // 不同项目同实例同 env 允许
        repo.create(&sample(2, 10, "DATABASE_URL")).await.unwrap();
    }

    #[tokio::test]
    async fn delete_and_cascade_by_project() {
        let repo = setup().await;
        let l1 = repo.create(&sample(1, 10, "DATABASE_URL")).await.unwrap();
        repo.create(&sample(1, 11, "REDIS_URL")).await.unwrap();
        repo.delete(l1).await.unwrap();
        assert_eq!(repo.list_by_project(1).await.unwrap().len(), 1);
        repo.delete_by_project(1).await.unwrap();
        assert!(repo.list_by_project(1).await.unwrap().is_empty());
        // 幂等
        repo.delete_by_project(1).await.unwrap();
    }
}

#[cfg(test)]
mod certificate_repo_tests {
    use super::*;

    async fn setup() -> SqliteCertificateRepository {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：对同一库重复执行必须安全（SOP 自测要求）
        run_migrations(&pool).await.unwrap();
        SqliteCertificateRepository::new(pool)
    }

    fn sample(domains: &str) -> Certificate {
        Certificate {
            domains: domains.to_string(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let repo = setup().await;
        let id = repo
            .create(&sample(r#"["example.com","www.example.com"]"#))
            .await
            .unwrap();

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.source, "letsencrypt");
        assert!(found.auto_renew);
        assert!(found.cert_path.is_none());
        assert_eq!(found.primary_domain().as_deref(), Some("example.com"));

        // 签发回写
        let mut c = found;
        c.source = "letsencrypt".to_string();
        c.cert_path = Some("/opt/flamepanel/data/certs/example.com/fullchain.pem".to_string());
        c.key_path = Some("/opt/flamepanel/data/certs/example.com/privkey.pem".to_string());
        c.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(90));
        c.last_renewed_at = Some(chrono::Utc::now());
        repo.update(&c).await.unwrap();

        let updated = repo.find_by_id(id).await.unwrap().unwrap();
        assert!(updated.cert_path.is_some());
        assert!(!updated.needs_renewal(chrono::Utc::now()));

        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());
        // 删除幂等
        repo.delete(id).await.unwrap();
    }

    #[tokio::test]
    async fn unique_domains_rejected() {
        let repo = setup().await;
        repo.create(&sample(r#"["a.com"]"#)).await.unwrap();
        assert!(repo.create(&sample(r#"["a.com"]"#)).await.is_err());
        // 不同域名集合允许
        repo.create(&sample(r#"["b.com"]"#)).await.unwrap();
    }

    #[tokio::test]
    async fn find_by_domains_and_website() {
        let repo = setup().await;
        let id = repo.create(&sample(r#"["a.com"]"#)).await.unwrap();
        assert!(repo
            .find_by_domains(r#"["a.com"]"#)
            .await
            .unwrap()
            .is_some());
        assert!(repo
            .find_by_domains(r#"["x.com"]"#)
            .await
            .unwrap()
            .is_none());

        let mut c = repo.find_by_id(id).await.unwrap().unwrap();
        c.website_id = Some(7);
        repo.update(&c).await.unwrap();
        assert_eq!(repo.find_by_website(7).await.unwrap().unwrap().id, id);
        assert!(repo.find_by_website(8).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn update_missing_returns_not_found() {
        let repo = setup().await;
        let mut c = sample(r#"["a.com"]"#);
        c.id = 999;
        assert!(repo.update(&c).await.is_err());
    }
}

// ── DNS 厂商账号仓储（主线三 §6.1 · Stage 4）──────────────────────────
