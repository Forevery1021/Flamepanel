use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::infrastructure::db_models::*;
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

/// SQLite 运行时加固：WAL 日志 + busy_timeout 5s + synchronous=NORMAL
/// 减少并发写锁冲突（database is locked），并提升崩溃恢复能力。
pub async fn configure_sqlite_pragmas(pool: &SqlitePool) -> Result<(), AppError> {
    // journal_mode 返回结果行，使用 execute 需要 query 消费
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Failed to set journal_mode=WAL: {}", e)))?;
    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Failed to set busy_timeout: {}", e)))?;
    sqlx::query("PRAGMA synchronous=NORMAL")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Failed to set synchronous=NORMAL: {}", e)))?;
    tracing::info!("SQLite runtime hardening applied: WAL, busy_timeout=5000, synchronous=NORMAL");
    Ok(())
}

/// 幂等迁移：若表中不存在指定列则 ALTER TABLE 添加（SQLite 无 ADD COLUMN IF NOT EXISTS）
async fn add_column_if_missing(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), AppError> {
    let exists: bool = sqlx::query("PRAGMA table_info(? )")
        .bind(table)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration check error: {}", e)))?
        .iter()
        .any(|row| row.get::<String, _>("name") == column);
    if exists {
        return Ok(());
    }
    // 列名与定义均为硬编码常量（非用户输入），经 AssertSqlSafe 审计包装
    let sql: String = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    Ok(())
}

pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, created_at, must_change_password FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(User::from);
        Ok(user)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, created_at, must_change_password FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
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
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(User::from).collect();
        Ok(users)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}

pub struct SqliteNodeRepository {
    pool: SqlitePool,
}

impl SqliteNodeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NodeRepository for SqliteNodeRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<ServerNode>, AppError> {
        let node = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(ServerNode::from);
        Ok(node)
    }

    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError> {
        let node = sqlx::query_as::<_, ServerNodeRow>(
            "SELECT id, name, hostname, ip_address, status, created_at, last_heartbeat_at, metrics_json, auth_token, agent_port FROM nodes WHERE hostname = ?",
        )
        .bind(hostname)
        .fetch_optional(&self.pool)
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
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ServerNode::from).collect();
        Ok(nodes)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM nodes")
            .fetch_one(&self.pool)
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ServerNode::from).collect();
        Ok(nodes)
    }
}

pub struct SqliteWebsiteRepository {
    pool: SqlitePool,
}

impl SqliteWebsiteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebsiteRepository for SqliteWebsiteRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError> {
        let site = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, created_at, resource_version FROM websites WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(Website::from);
        Ok(site)
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError> {
        let site = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, created_at, resource_version FROM websites WHERE domain = ?",
        )
        .bind(domain)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(Website::from);
        Ok(site)
    }

    async fn create(&self, website: &Website) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO websites (name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, resource_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
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
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, website: &Website) -> Result<(), AppError> {
        // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
        let result = sqlx::query(
            "UPDATE websites SET name=?, domain=?, root_path=?, node_id=?, engine=?, ssl_enabled=?, proxy_enabled=?, proxy_pass=?, resource_version=resource_version+1 WHERE id=? AND resource_version=?",
        )
        .bind(&website.name)
        .bind(&website.domain)
        .bind(&website.root_path)
        .bind(website.node_id)
        .bind(&website.engine)
        .bind(website.ssl_enabled)
        .bind(website.proxy_enabled)
        .bind(&website.proxy_pass)
        .bind(website.id)
        .bind(website.resource_version)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        if result.rows_affected() == 0 {
            // 区分“不存在”与“版本冲突”
            let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM websites WHERE id = ?")
                .bind(website.id)
                .fetch_one(&self.pool)
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
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, created_at, resource_version FROM websites ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(Website::from).collect();
        Ok(sites)
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Website>, AppError> {
        let limit = limit.clamp(1, 200);
        let sites = sqlx::query_as::<_, WebsiteRow>(
            "SELECT id, name, domain, root_path, status, node_id, engine, ssl_enabled, proxy_enabled, proxy_pass, created_at, resource_version FROM websites ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(Website::from).collect();
        Ok(sites)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM websites")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}

pub struct SqliteWebServerRepository {
    pool: SqlitePool,
}

impl SqliteWebServerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebServerRepository for SqliteWebServerRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<WebServerInstance>, AppError> {
        let instance = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(WebServerInstance::from);
        Ok(instance)
    }

    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers WHERE engine = ? ORDER BY id",
        )
        .bind(engine)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(WebServerInstance::from).collect();
        Ok(instances)
    }

    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO web_servers (engine, version, status, config_path, binary_path, port, resource_version) VALUES (?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&instance.engine)
        .bind(&instance.version)
        .bind("stopped")
        .bind(&instance.config_path)
        .bind(&instance.binary_path)
        .bind(instance.port)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
        let result = sqlx::query(
            "UPDATE web_servers SET engine = ?, version = ?, status = ?, config_path = ?, binary_path = ?, port = ?, resource_version = resource_version + 1 WHERE id = ? AND resource_version = ?",
        )
        .bind(&instance.engine)
        .bind(&instance.version)
        .bind(&instance.status)
        .bind(&instance.config_path)
        .bind(&instance.binary_path)
        .bind(instance.port)
        .bind(instance.id)
        .bind(instance.resource_version)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        if result.rows_affected() == 0 {
            // 区分“不存在”与“版本冲突”
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM web_servers WHERE id = ?")
                    .bind(instance.id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            if exists == 0 {
                return Err(AppError::NotFound(format!(
                    "Web server {} not found",
                    instance.id
                )));
            }
            return Err(AppError::Conflict(format!(
                "Web server {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                instance.id, instance.resource_version
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM web_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(WebServerInstance::from).collect();
        Ok(instances)
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<WebServerInstance>, AppError> {
        let limit = limit.clamp(1, 200);
        let instances = sqlx::query_as::<_, WebServerInstanceRow>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at, resource_version FROM web_servers ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(WebServerInstance::from).collect();
        Ok(instances)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM web_servers")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 幂等迁移：users 表补充强制改密列（旧库升级）
    add_column_if_missing(
        pool,
        "users",
        "must_change_password",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS nodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            hostname TEXT NOT NULL,
            ip_address TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'unknown',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 幂等迁移：为 nodes 表补充心跳/指标/令牌列（旧库升级）
    add_column_if_missing(pool, "nodes", "last_heartbeat_at", "TEXT").await?;
    add_column_if_missing(pool, "nodes", "metrics_json", "TEXT").await?;
    add_column_if_missing(pool, "nodes", "auth_token", "TEXT").await?;
    add_column_if_missing(pool, "nodes", "agent_port", "INTEGER NOT NULL DEFAULT 9527").await?;

    // T13：节点离线扫描按 last_heartbeat_at 过滤，补索引避免全表扫描。
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_heartbeat ON nodes(last_heartbeat_at)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS websites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            domain TEXT NOT NULL UNIQUE,
            root_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            node_id INTEGER NOT NULL DEFAULT 0,
            engine TEXT NOT NULL DEFAULT 'nginx',
            ssl_enabled INTEGER NOT NULL DEFAULT 0,
            proxy_enabled INTEGER NOT NULL DEFAULT 0,
            proxy_pass TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 幂等迁移：为 websites 表补充 resource_version 乐观并发版本列（旧库升级）
    add_column_if_missing(
        pool,
        "websites",
        "resource_version",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS web_servers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            engine TEXT NOT NULL,
            version TEXT,
            status TEXT NOT NULL DEFAULT 'stopped',
            config_path TEXT NOT NULL DEFAULT '',
            binary_path TEXT,
            port INTEGER NOT NULL DEFAULT 80,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 幂等迁移：为 web_servers 表补充 resource_version 乐观并发版本列（旧库升级）
    add_column_if_missing(
        pool,
        "web_servers",
        "resource_version",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            action TEXT NOT NULL,
            target TEXT,
            ip TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // T13：operation_logs 按 username/action 过滤、按 id 倒序分页，补索引避免全表扫描。
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_username ON operation_logs(username)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_operation_logs_action ON operation_logs(action)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_created ON operation_logs(created_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS outbox_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL DEFAULT '{}',
            published INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_outbox_events_type ON outbox_events(event_type)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS permissions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            resource TEXT NOT NULL,
            action TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT ''
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT ''
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS role_permissions (
            role_id INTEGER NOT NULL,
            permission_id INTEGER NOT NULL,
            PRIMARY KEY (role_id, permission_id),
            FOREIGN KEY (role_id) REFERENCES roles(id),
            FOREIGN KEY (permission_id) REFERENCES permissions(id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            level TEXT NOT NULL,
            message TEXT NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // T13：logs 按 source/level 过滤、按 id 倒序分页，补索引避免全表扫描。
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_source ON logs(source)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_created ON logs(created_at)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS databases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            db_type TEXT NOT NULL,
            name TEXT NOT NULL,
            version TEXT NOT NULL DEFAULT '',
            port INTEGER NOT NULL DEFAULT 3306,
            status TEXT NOT NULL DEFAULT 'stopped',
            install_path TEXT NOT NULL DEFAULT '',
            data_dir TEXT NOT NULL DEFAULT '',
            config_file TEXT NOT NULL DEFAULT '',
            root_user TEXT NOT NULL DEFAULT 'root',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 幂等迁移：为 databases 表补充 resource_version 乐观并发版本列（旧库升级）
    add_column_if_missing(
        pool,
        "databases",
        "resource_version",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS panel_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS firewall_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            protocol TEXT NOT NULL DEFAULT 'any',
            port TEXT,
            source TEXT DEFAULT '0.0.0.0/0',
            destination TEXT,
            action TEXT NOT NULL DEFAULT 'allow',
            enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 50,
            direction TEXT NOT NULL DEFAULT 'in',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_packages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT '',
            format TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            logo TEXT,
            metadata_json TEXT NOT NULL,
            source_path TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS installed_apps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            package_key TEXT NOT NULL,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            mode TEXT NOT NULL DEFAULT 'container',
            status TEXT NOT NULL DEFAULT 'stopped',
            access_url TEXT,
            install_path TEXT NOT NULL DEFAULT '',
            container_name TEXT,
            port INTEGER,
            params_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS plugins (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL DEFAULT '1.0.0',
            author TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            wasm_base64 TEXT NOT NULL,
            wasm_hash TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            homepage TEXT,
            license TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            config_schema TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scheduled_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            schedule TEXT NOT NULL DEFAULT '* * * * *',
            enabled INTEGER NOT NULL DEFAULT 1,
            last_status TEXT NOT NULL DEFAULT 'never',
            last_output TEXT NOT NULL DEFAULT '',
            last_run_at TEXT,
            next_run_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 备忘录 / TODO（v0.7.0）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'memo',
            done INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_enabled ON scheduled_tasks(enabled)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 幂等迁移：installed_apps 补 launch_count（v0.7.0 常用应用）
    add_column_if_missing(
        pool,
        "installed_apps",
        "launch_count",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_installed_apps_key ON installed_apps(package_key)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 统一 Task 状态机持久化（Phase B1 扩展：TaskTracker 任务记录落库）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            state TEXT NOT NULL,
            progress INTEGER NOT NULL DEFAULT 0,
            message TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    Ok(())
}

pub struct SqliteDatabaseRepository {
    pool: SqlitePool,
}

impl SqliteDatabaseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatabaseRepository for SqliteDatabaseRepository {
    async fn list_all(&self) -> Result<Vec<DatabaseInstance>, AppError> {
        let rows = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(DatabaseInstance::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError> {
        let row = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(DatabaseInstance::from);
        Ok(row)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<DatabaseInstance>, AppError> {
        let row = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(DatabaseInstance::from);
        Ok(row)
    }

    async fn find_by_type(&self, db_type: &str) -> Result<Vec<DatabaseInstance>, AppError> {
        let rows = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases WHERE db_type = ? ORDER BY id",
        )
        .bind(db_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(DatabaseInstance::from).collect();
        Ok(rows)
    }

    async fn create(&self, instance: &DatabaseInstance) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO databases (db_type, name, version, port, status, install_path, data_dir, config_file, root_user, resource_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&instance.db_type)
        .bind(&instance.name)
        .bind(&instance.version)
        .bind(instance.port)
        .bind(&instance.status)
        .bind(&instance.install_path)
        .bind(&instance.data_dir)
        .bind(&instance.config_file)
        .bind(&instance.root_user)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, instance: &DatabaseInstance) -> Result<(), AppError> {
        // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
        let result = sqlx::query(
            "UPDATE databases SET version=?, port=?, status=?, install_path=?, data_dir=?, config_file=?, root_user=?, updated_at=datetime('now'), resource_version=resource_version+1 WHERE id=? AND resource_version=?",
        )
        .bind(&instance.version)
        .bind(instance.port)
        .bind(&instance.status)
        .bind(&instance.install_path)
        .bind(&instance.data_dir)
        .bind(&instance.config_file)
        .bind(&instance.root_user)
        .bind(instance.id)
        .bind(instance.resource_version)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        if result.rows_affected() == 0 {
            // 区分“不存在”与“版本冲突”
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM databases WHERE id = ?")
                    .bind(instance.id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            if exists == 0 {
                return Err(AppError::NotFound(format!(
                    "Database instance {} not found",
                    instance.id
                )));
            }
            return Err(AppError::Conflict(format!(
                "Database instance {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                instance.id, instance.resource_version
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM databases WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE databases SET status=?, updated_at=datetime('now') WHERE id=?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_status_batch(&self, updates: &[(i64, String)]) -> Result<(), AppError> {
        if updates.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        for (id, status) in updates {
            sqlx::query("UPDATE databases SET status=?, updated_at=datetime('now') WHERE id=?")
                .bind(status)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<DatabaseInstance>, AppError> {
        let limit = limit.clamp(1, 200);
        let rows = sqlx::query_as::<_, DatabaseInstanceRow>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at, resource_version FROM databases ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(DatabaseInstance::from).collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM databases")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 启动时 upsert 默认设置，保证 SQLite 模式与 InMemory 模式行为一致。
    pub async fn ensure_defaults(&self) -> Result<(), AppError> {
        for setting in crate::domain::entity::default_settings() {
            sqlx::query(
                "INSERT INTO panel_settings (key, value, description, updated_at) VALUES (?, ?, ?, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET description = excluded.description",
            )
            .bind(&setting.key)
            .bind(&setting.value)
            .bind(&setting.description)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        Ok(())
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let row = sqlx::query_scalar::<_, String>("SELECT value FROM panel_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO panel_settings (key, value, description, updated_at) VALUES (?, ?, '', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn set_many(&self, entries: &[(String, String)]) -> Result<(), AppError> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        for (key, value) in entries {
            sqlx::query(
                "INSERT INTO panel_settings (key, value, description, updated_at) VALUES (?, ?, '', datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            )
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        let rows = sqlx::query_as::<_, PanelSettingRow>(
            "SELECT key, value, description, updated_at FROM panel_settings ORDER BY key",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(PanelSetting::from)
        .collect();
        Ok(rows)
    }

    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let rows = sqlx::query("SELECT key, value FROM panel_settings")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows
            .iter()
            .map(|r| (r.get::<String, _>("key"), r.get::<String, _>("value")))
            .collect())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<PanelSetting>, AppError> {
        let limit = limit.clamp(1, 200);
        let rows = sqlx::query_as::<_, PanelSettingRow>(
            "SELECT key, value, description, updated_at FROM panel_settings ORDER BY key DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(PanelSetting::from).collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM panel_settings")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}

pub struct SqlitePermissionRepository {
    pool: SqlitePool,
}

impl SqlitePermissionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PermissionRepository for SqlitePermissionRepository {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError> {
        let perms = sqlx::query_as::<_, PermissionRow>(
            "SELECT id, resource, action, description FROM permissions ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(Permission::from)
        .collect();
        Ok(perms)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError> {
        let perm = sqlx::query_as::<_, PermissionRow>(
            "SELECT id, resource, action, description FROM permissions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(Permission::from);
        Ok(perm)
    }

    async fn find_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<Permission>, AppError> {
        let perm = sqlx::query_as::<_, PermissionRow>(
            "SELECT id, resource, action, description FROM permissions WHERE resource = ? AND action = ?",
        )
        .bind(resource)
        .bind(action)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(Permission::from);
        Ok(perm)
    }

    async fn create(&self, permission: &Permission) -> Result<i64, AppError> {
        let id =
            sqlx::query("INSERT INTO permissions (resource, action, description) VALUES (?, ?, ?)")
                .bind(&permission.resource)
                .bind(&permission.action)
                .bind(&permission.description)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .last_insert_rowid();
        Ok(id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM permissions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}

pub struct SqliteRoleRepository {
    pool: SqlitePool,
}

impl SqliteRoleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoleRepository for SqliteRoleRepository {
    async fn list_all(&self) -> Result<Vec<Role>, AppError> {
        let roles =
            sqlx::query_as::<_, RoleRow>("SELECT id, name, description FROM roles ORDER BY id")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .into_iter()
                .map(Role::from)
                .collect();
        Ok(roles)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        let role =
            sqlx::query_as::<_, RoleRow>("SELECT id, name, description FROM roles WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .map(Role::from);
        Ok(role)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError> {
        let role =
            sqlx::query_as::<_, RoleRow>("SELECT id, name, description FROM roles WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .map(Role::from);
        Ok(role)
    }

    async fn create(&self, role: &Role) -> Result<i64, AppError> {
        let id = sqlx::query("INSERT INTO roles (name, description) VALUES (?, ?)")
            .bind(&role.name)
            .bind(&role.description)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
            .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, role: &Role) -> Result<(), AppError> {
        sqlx::query("UPDATE roles SET name = ?, description = ? WHERE id = ?")
            .bind(&role.name)
            .bind(&role.description)
            .bind(role.id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT permission_id FROM role_permissions WHERE role_id = ?")
                .bind(role_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError> {
        // T7：DELETE + INSERT 循环放入同一事务，避免半截状态；DELETE 错误不再吞掉。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        for pid in permission_ids {
            sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                .bind(role_id)
                .bind(pid)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(())
    }
}

pub struct SqliteOperationLogRepository {
    pool: SqlitePool,
}

impl SqliteOperationLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OperationLogRepository for SqliteOperationLogRepository {
    async fn create(
        &self,
        username: &str,
        action: &str,
        target: Option<&str>,
        ip: Option<&str>,
    ) -> Result<OperationLog, AppError> {
        let result = sqlx::query(
            "INSERT INTO operation_logs (username, action, target, ip, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(username)
        .bind(action)
        .bind(target)
        .bind(ip)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let id = result.last_insert_rowid();
        Ok(OperationLog {
            id: id as i64,
            username: username.into(),
            action: action.into(),
            target: target.map(|s| s.into()),
            ip: ip.map(|s| s.into()),
            created_at: chrono::Utc::now(),
        })
    }

    async fn list(&self) -> Result<Vec<OperationLog>, AppError> {
        let rows = sqlx::query_as::<_, OperationLogRow>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs ORDER BY id DESC LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(OperationLog::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        let row = sqlx::query_as::<_, OperationLogRow>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(OperationLog::from);
        Ok(row)
    }

    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        let rows = sqlx::query_as::<_, OperationLogRow>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs WHERE username = ? ORDER BY id DESC LIMIT 100",
        )
        .bind(username)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(OperationLog::from).collect();
        Ok(rows)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM operation_logs WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        action_prefix: Option<&str>,
    ) -> Result<Vec<OperationLog>, AppError> {
        let rows = match action_prefix {
            Some(prefix) if !prefix.is_empty() => {
                sqlx::query_as::<_, OperationLogRow>(
                    "SELECT id, username, action, target, ip, created_at FROM operation_logs \
                     WHERE action LIKE ? ESCAPE '\\' ORDER BY id DESC LIMIT ? OFFSET ?",
                )
                .bind(escape_like(prefix))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as::<_, OperationLogRow>(
                    "SELECT id, username, action, target, ip, created_at FROM operation_logs \
                     ORDER BY id DESC LIMIT ? OFFSET ?",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(OperationLog::from)
        .collect();
        Ok(rows)
    }

    async fn count(&self, action_prefix: Option<&str>) -> Result<i64, AppError> {
        let count: i64 = match action_prefix {
            Some(prefix) if !prefix.is_empty() => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM operation_logs WHERE action LIKE ? ESCAPE '\\'",
                )
                .bind(escape_like(prefix))
                .fetch_one(&self.pool)
                .await
            }
            _ => {
                sqlx::query_scalar("SELECT COUNT(*) FROM operation_logs")
                    .fetch_one(&self.pool)
                    .await
            }
        }
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }
}

pub struct SqliteOutboxRepository {
    pool: SqlitePool,
}

impl SqliteOutboxRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboxRepository for SqliteOutboxRepository {
    async fn append(
        &self,
        event_type: &str,
        payload: &str,
        published: bool,
    ) -> Result<OutboxEvent, AppError> {
        let result = sqlx::query(
            "INSERT INTO outbox_events (event_type, payload, published, created_at) VALUES (?, ?, ?, datetime('now'))",
        )
        .bind(event_type)
        .bind(payload)
        .bind(published as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let id = result.last_insert_rowid();
        Ok(OutboxEvent {
            id: id as i64,
            event_type: event_type.into(),
            payload: payload.into(),
            published,
            created_at: chrono::Utc::now(),
        })
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        event_type: Option<&str>,
    ) -> Result<Vec<OutboxEvent>, AppError> {
        let sql = match event_type {
            Some(_) => {
                "SELECT id, event_type, payload, published, created_at FROM outbox_events \
                 WHERE event_type = ? ORDER BY id DESC LIMIT ? OFFSET ?"
            }
            None => {
                "SELECT id, event_type, payload, published, created_at FROM outbox_events \
                 ORDER BY id DESC LIMIT ? OFFSET ?"
            }
        };
        let rows = if let Some(et) = event_type {
            sqlx::query_as::<_, OutboxEventRow>(sql)
                .bind(et)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        } else {
            sqlx::query_as::<_, OutboxEventRow>(sql)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        };
        Ok(rows.into_iter().map(OutboxEvent::from).collect())
    }

    async fn count(&self, event_type: Option<&str>) -> Result<i64, AppError> {
        let sql = match event_type {
            Some(_) => "SELECT COUNT(*) FROM outbox_events WHERE event_type = ?",
            None => "SELECT COUNT(*) FROM outbox_events",
        };
        let count: (i64,) = if let Some(et) = event_type {
            sqlx::query_as(sql)
                .bind(et)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        } else {
            sqlx::query_as(sql)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        };
        Ok(count.0)
    }
}

/// 转义 LIKE 通配符（`%` / `_` / 转义符 `\`），使前缀过滤退化为字面前缀匹配。
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub struct SqliteLogRepository {
    pool: SqlitePool,
}

impl SqliteLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LogRepository for SqliteLogRepository {
    async fn create(
        &self,
        source: &str,
        level: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<LogEntry, AppError> {
        let result = sqlx::query(
            "INSERT INTO logs (source, level, message, metadata, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(source)
        .bind(level)
        .bind(message)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let id = result.last_insert_rowid();
        Ok(LogEntry {
            id: id as i64,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            metadata: metadata.map(|s| s.into()),
            created_at: chrono::Utc::now(),
        })
    }

    async fn list(&self) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs ORDER BY id DESC LIMIT 200",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(LogEntry::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        let row = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .map(LogEntry::from);
        Ok(row)
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE source = ? ORDER BY id DESC LIMIT 200",
        )
        .bind(source)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(LogEntry::from).collect();
        Ok(rows)
    }

    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE level = ? ORDER BY id DESC LIMIT 200",
        )
        .bind(level)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(LogEntry::from).collect();
        Ok(rows)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM logs WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT id, source, level, message, metadata, created_at FROM logs \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(LogEntry::from)
        .collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM logs")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }
}

pub struct SqliteFirewallRepository {
    pool: SqlitePool,
}

impl SqliteFirewallRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FirewallRepository for SqliteFirewallRepository {
    async fn list_all(&self) -> Result<Vec<FirewallRule>, AppError> {
        let rows = sqlx::query_as::<_, FirewallRuleRow>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules ORDER BY priority, id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(FirewallRule::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<FirewallRule>, AppError> {
        let row = sqlx::query_as::<_, FirewallRuleRow>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(FirewallRule::from).collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM firewall_rules")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(c)
    }
}
// ─── 应用商店仓储 ────────────────────────────────────────────────────────────

pub struct SqliteAppPackageRepository {
    pool: SqlitePool,
}

impl SqliteAppPackageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AppPackageRepository for SqliteAppPackageRepository {
    async fn list_all(&self) -> Result<Vec<AppPackage>, AppError> {
        let rows = sqlx::query_as::<_, AppPackageRow>(
            "SELECT id, key, name, category, format, description, logo, metadata_json, source_path, created_at, updated_at FROM app_packages ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(AppPackage::from).collect();
        Ok(rows)
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<AppPackage>, AppError> {
        let row = sqlx::query_as::<_, AppPackageRow>(
            "SELECT id, key, name, category, format, description, logo, metadata_json, source_path, created_at, updated_at FROM app_packages WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(AppPackage::from);
        Ok(row)
    }

    async fn create(&self, pkg: &AppPackage) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO app_packages (key, name, category, format, description, logo, metadata_json, source_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&pkg.key)
        .bind(&pkg.name)
        .bind(&pkg.category)
        .bind(&pkg.format)
        .bind(&pkg.description)
        .bind(&pkg.logo)
        .bind(&pkg.metadata_json)
        .bind(&pkg.source_path)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM app_packages WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn create_many(&self, pkgs: &[AppPackage]) -> Result<usize, AppError> {
        if pkgs.is_empty() {
            return Ok(0);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;
        let mut count = 0usize;
        for pkg in pkgs {
            // 幂等：已存在则跳过，不视为失败
            let exists = sqlx::query("SELECT 1 FROM app_packages WHERE key=?")
                .bind(&pkg.key)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
                .is_some();
            if exists {
                continue;
            }
            sqlx::query(
                "INSERT INTO app_packages (key, name, category, format, description, logo, metadata_json, source_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&pkg.key)
            .bind(&pkg.name)
            .bind(&pkg.category)
            .bind(&pkg.format)
            .bind(&pkg.description)
            .bind(&pkg.logo)
            .bind(&pkg.metadata_json)
            .bind(&pkg.source_path)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
            count += 1;
        }
        tx.commit()
            .await
            .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
        Ok(count)
    }
}

pub struct SqliteInstalledAppRepository {
    pool: SqlitePool,
}

impl SqliteInstalledAppRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InstalledAppRepository for SqliteInstalledAppRepository {
    async fn list_all(&self) -> Result<Vec<InstalledApp>, AppError> {
        let rows = sqlx::query_as::<_, InstalledAppRow>(
            "SELECT id, package_key, name, version, mode, status, access_url, install_path, container_name, port, params_json, created_at, updated_at, launch_count FROM installed_apps ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(InstalledApp::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError> {
        let row = sqlx::query_as::<_, InstalledAppRow>(
            "SELECT id, package_key, name, version, mode, status, access_url, install_path, container_name, port, params_json, created_at, updated_at, launch_count FROM installed_apps WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(InstalledApp::from);
        Ok(row)
    }

    async fn create(&self, app: &InstalledApp) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO installed_apps (package_key, name, version, mode, status, access_url, install_path, container_name, port, params_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&app.package_key)
        .bind(&app.name)
        .bind(&app.version)
        .bind(&app.mode)
        .bind(&app.status)
        .bind(&app.access_url)
        .bind(&app.install_path)
        .bind(&app.container_name)
        .bind(app.port)
        .bind(&app.params_json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, app: &InstalledApp) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE installed_apps SET version=?, status=?, access_url=?, install_path=?, container_name=?, port=?, params_json=?, launch_count=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&app.version)
        .bind(&app.status)
        .bind(&app.access_url)
        .bind(&app.install_path)
        .bind(&app.container_name)
        .bind(app.port)
        .bind(&app.params_json)
        .bind(app.launch_count)
        .bind(app.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM installed_apps WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}

pub struct SqlitePluginRepository {
    pool: SqlitePool,
}

impl SqlitePluginRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PluginRepository for SqlitePluginRepository {
    async fn save(&self, plugin: &Plugin) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO plugins (id, name, version, author, description, wasm_base64, wasm_hash, enabled, homepage, license, tags, config_schema) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, version=excluded.version, author=excluded.author, description=excluded.description, wasm_base64=excluded.wasm_base64, wasm_hash=excluded.wasm_hash, enabled=excluded.enabled, homepage=excluded.homepage, license=excluded.license, tags=excluded.tags, config_schema=excluded.config_schema, updated_at=datetime('now')",
        )
        .bind(&plugin.id)
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(&plugin.author)
        .bind(&plugin.description)
        .bind(&plugin.wasm_base64)
        .bind(&plugin.wasm_hash)
        .bind(plugin.enabled)
        .bind(&plugin.homepage)
        .bind(&plugin.license)
        .bind(serde_json::to_string(&plugin.tags).unwrap_or_default())
        .bind(&plugin.config_schema)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Plugin>, AppError> {
        let rows = sqlx::query_as::<_, PluginDbRow>(
            "SELECT id, name, version, author, description, wasm_base64, wasm_hash, enabled, homepage, license, tags, config_schema, created_at, updated_at FROM plugins ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        rows.into_iter().map(PluginDbRow::into_entity).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Plugin>, AppError> {
        let row = sqlx::query_as::<_, PluginDbRow>(
            "SELECT id, name, version, author, description, wasm_base64, wasm_hash, enabled, homepage, license, tags, config_schema, created_at, updated_at FROM plugins WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        row.map(|r| r.into_entity()).transpose()
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM plugins WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct PluginDbRow {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    wasm_base64: String,
    wasm_hash: String,
    enabled: bool,
    homepage: Option<String>,
    license: Option<String>,
    tags: String,
    config_schema: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl PluginDbRow {
    fn into_entity(self) -> Result<Plugin, AppError> {
        Ok(Plugin {
            id: self.id,
            name: self.name,
            version: self.version,
            author: self.author,
            description: self.description,
            wasm_hash: self.wasm_hash,
            wasm_base64: self.wasm_base64,
            enabled: self.enabled,
            homepage: self.homepage,
            license: self.license,
            tags: serde_json::from_str(&self.tags).unwrap_or_default(),
            config_schema: self.config_schema,
            dependencies: Vec::new(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// ─── 定时任务 SQLite 仓储 ────────────────────────────────────────────────

pub struct SqliteScheduledTaskRepository {
    pool: SqlitePool,
}

impl SqliteScheduledTaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScheduledTaskRepository for SqliteScheduledTaskRepository {
    async fn list_all(&self) -> Result<Vec<ScheduledTask>, AppError> {
        let rows = sqlx::query_as::<_, ScheduledTaskRow>(
            "SELECT id, name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at, created_at, updated_at FROM scheduled_tasks ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.into_iter().map(ScheduledTask::from).collect();
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ScheduledTask>, AppError> {
        let row = sqlx::query_as::<_, ScheduledTaskRow>(
            "SELECT id, name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at, created_at, updated_at FROM scheduled_tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?.map(ScheduledTask::from);
        Ok(row)
    }

    async fn create(&self, task: &ScheduledTask) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO scheduled_tasks (name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&task.name)
        .bind(&task.command)
        .bind(&task.schedule)
        .bind(task.enabled)
        .bind(&task.last_status)
        .bind(&task.last_output)
        .bind(task.last_run_at)
        .bind(task.next_run_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, task: &ScheduledTask) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE scheduled_tasks SET name=?, command=?, schedule=?, enabled=?, last_status=?, last_output=?, last_run_at=?, next_run_at=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&task.name)
        .bind(&task.command)
        .bind(&task.schedule)
        .bind(task.enabled)
        .bind(&task.last_status)
        .bind(&task.last_output)
        .bind(task.last_run_at)
        .bind(task.next_run_at)
        .bind(task.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM scheduled_tasks WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ScheduledTask>, AppError> {
        let rows = sqlx::query_as::<_, ScheduledTaskRow>(
            "SELECT id, name, command, schedule, enabled, last_status, last_output, last_run_at, next_run_at, created_at, updated_at FROM scheduled_tasks \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?
        .into_iter()
        .map(ScheduledTask::from)
        .collect();
        Ok(rows)
    }

    async fn count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scheduled_tasks")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(count)
    }
}

pub struct SqliteMemoRepository {
    pool: SqlitePool,
}

impl SqliteMemoRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
            .fetch_all(&self.pool)
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
        .fetch_optional(&self.pool)
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

// ─── 统一 Task 状态机持久化（Phase B1 扩展）────────────────────────────────

/// SQLite TaskStore 的行模型。
#[derive(sqlx::FromRow)]
struct TaskDbRow {
    id: i64,
    kind: String,
    name: String,
    state: String,
    progress: i32,
    message: String,
    created_at: String,
    updated_at: String,
}

/// SQLite TaskStore：将统一 Task 记录落库（进程重启可恢复）。
pub struct SqliteTaskStore {
    pool: SqlitePool,
}

impl SqliteTaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn task_state_str(s: &crate::runtime::task_state::TaskState) -> &'static str {
    match s {
        crate::runtime::task_state::TaskState::Pending => "pending",
        crate::runtime::task_state::TaskState::Running => "running",
        crate::runtime::task_state::TaskState::Success => "success",
        crate::runtime::task_state::TaskState::Failed => "failed",
        crate::runtime::task_state::TaskState::Cancelled => "cancelled",
    }
}

fn task_kind_str(k: &crate::runtime::task_state::TaskKind) -> &'static str {
    k.as_str()
}

fn parse_task_state(s: &str) -> crate::runtime::task_state::TaskState {
    match s {
        "running" => crate::runtime::task_state::TaskState::Running,
        "success" => crate::runtime::task_state::TaskState::Success,
        "failed" => crate::runtime::task_state::TaskState::Failed,
        "cancelled" => crate::runtime::task_state::TaskState::Cancelled,
        _ => crate::runtime::task_state::TaskState::Pending,
    }
}

fn parse_task_kind(s: &str) -> crate::runtime::task_state::TaskKind {
    match s {
        "install" => crate::runtime::task_state::TaskKind::Install,
        "engine_switch" => crate::runtime::task_state::TaskKind::EngineSwitch,
        "batch_node" => crate::runtime::task_state::TaskKind::BatchNode,
        _ => crate::runtime::task_state::TaskKind::Generic,
    }
}

#[async_trait::async_trait]
impl crate::runtime::task_state::TaskStore for SqliteTaskStore {
    async fn insert(&self, record: &crate::runtime::task_state::TaskRecord) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO tasks (id, kind, name, state, progress, message, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               kind=excluded.kind, name=excluded.name, state=excluded.state,
               progress=excluded.progress, message=excluded.message,
               created_at=excluded.created_at, updated_at=excluded.updated_at",
        )
        .bind(record.id as i64)
        .bind(task_kind_str(&record.kind))
        .bind(&record.name)
        .bind(task_state_str(&record.state))
        .bind(record.progress as i32)
        .bind(&record.message)
        .bind(record.created_at.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("insert task: {}", e))?;
        Ok(())
    }

    async fn update(&self, record: &crate::runtime::task_state::TaskRecord) -> Result<(), String> {
        sqlx::query(
            "UPDATE tasks SET kind=?, name=?, state=?, progress=?, message=?, updated_at=?
             WHERE id=?",
        )
        .bind(task_kind_str(&record.kind))
        .bind(&record.name)
        .bind(task_state_str(&record.state))
        .bind(record.progress as i32)
        .bind(&record.message)
        .bind(record.updated_at.to_rfc3339())
        .bind(record.id as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("update task: {}", e))?;
        Ok(())
    }

    async fn load_all(&self) -> Result<Vec<crate::runtime::task_state::TaskRecord>, String> {
        let rows = sqlx::query_as::<_, TaskDbRow>(
            "SELECT id, kind, name, state, progress, message, created_at, updated_at FROM tasks ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("load tasks: {}", e))?;
        Ok(rows
            .into_iter()
            .map(|r| crate::runtime::task_state::TaskRecord {
                id: r.id as u64,
                kind: parse_task_kind(&r.kind),
                name: r.name,
                state: parse_task_state(&r.state),
                progress: r.progress.clamp(0, 100) as u8,
                message: r.message,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
            .collect())
    }

    async fn remove(&self, id: u64) -> Result<(), String> {
        sqlx::query("DELETE FROM tasks WHERE id=?")
            .bind(id as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("delete task: {}", e))?;
        Ok(())
    }
}
