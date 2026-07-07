use async_trait::async_trait;
use sqlx::{SqlitePool, Row};
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::core::error::AppError;

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
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(user)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(user)
    }

    async fn create(&self, username: &str, password_hash: &str, role: &str) -> Result<User, AppError> {
        let id = sqlx::query("INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)")
            .bind(username)
            .bind(password_hash)
            .bind(role)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
            .last_insert_rowid();
        self.find_by_id(id).await.map(|u| u.unwrap())
    }

    async fn list(&self) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at FROM users ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(users)
    }

    async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(new_password_hash)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
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
        let node = sqlx::query_as::<_, ServerNode>(
            "SELECT id, name, hostname, ip_address, status, created_at FROM nodes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(node)
    }

    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError> {
        let node = sqlx::query_as::<_, ServerNode>(
            "SELECT id, name, hostname, ip_address, status, created_at FROM nodes WHERE hostname = ?",
        )
        .bind(hostname)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(node)
    }

    async fn create(&self, node: &ServerNode) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO nodes (name, hostname, ip_address, status) VALUES (?, ?, ?, ?)",
        )
        .bind(&node.name)
        .bind(&node.hostname)
        .bind(&node.ip_address)
        .bind(&node.status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn list_all(&self) -> Result<Vec<ServerNode>, AppError> {
        let nodes = sqlx::query_as::<_, ServerNode>(
            "SELECT id, name, hostname, ip_address, status, created_at FROM nodes ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
        let site = sqlx::query_as::<_, Website>(
            "SELECT id, name, domain, root_path, status, node_id, created_at FROM websites WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(site)
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError> {
        let site = sqlx::query_as::<_, Website>(
            "SELECT id, name, domain, root_path, status, node_id, created_at FROM websites WHERE domain = ?",
        )
        .bind(domain)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(site)
    }

    async fn create(&self, website: &Website) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO websites (name, domain, root_path, status, node_id) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&website.name)
        .bind(&website.domain)
        .bind(&website.root_path)
        .bind(&website.status)
        .bind(website.node_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn list_all(&self) -> Result<Vec<Website>, AppError> {
        let sites = sqlx::query_as::<_, Website>(
            "SELECT id, name, domain, root_path, status, node_id, created_at FROM websites ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(sites)
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
        let instance = sqlx::query_as::<_, WebServerInstance>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at FROM web_servers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(instance)
    }

    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = sqlx::query_as::<_, WebServerInstance>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at FROM web_servers WHERE engine = ? ORDER BY id",
        )
        .bind(engine)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(instances)
    }

    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO web_servers (engine, version, status, config_path, binary_path, port) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&instance.engine)
        .bind(&instance.version)
        .bind("stopped")
        .bind(&instance.config_path)
        .bind(&instance.binary_path)
        .bind(instance.port)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(result.last_insert_rowid())
    }

    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE web_servers SET engine = ?, version = ?, status = ?, config_path = ?, binary_path = ?, port = ? WHERE id = ?",
        )
        .bind(&instance.engine)
        .bind(&instance.version)
        .bind(&instance.status)
        .bind(&instance.config_path)
        .bind(&instance.binary_path)
        .bind(instance.port)
        .bind(instance.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM web_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = sqlx::query_as::<_, WebServerInstance>(
            "SELECT id, engine, version, status, config_path, binary_path, port, created_at FROM web_servers ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(instances)
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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT ''
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
    .map_err(|e| AppError::Internal(format!("Migration error: {}", e)))?;

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
        let rows = sqlx::query_as::<_, DatabaseInstance>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at FROM databases ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError> {
        let row = sqlx::query_as::<_, DatabaseInstance>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at FROM databases WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<DatabaseInstance>, AppError> {
        let row = sqlx::query_as::<_, DatabaseInstance>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at FROM databases WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_by_type(&self, db_type: &str) -> Result<Vec<DatabaseInstance>, AppError> {
        let rows = sqlx::query_as::<_, DatabaseInstance>(
            "SELECT id, db_type, name, version, port, status, install_path, data_dir, config_file, root_user, created_at, updated_at FROM databases WHERE db_type = ? ORDER BY id",
        )
        .bind(db_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn create(&self, instance: &DatabaseInstance) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO databases (db_type, name, version, port, status, install_path, data_dir, config_file, root_user) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn update(&self, instance: &DatabaseInstance) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE databases SET version=?, port=?, status=?, install_path=?, data_dir=?, config_file=?, root_user=?, updated_at=datetime('now') WHERE id=?",
        )
        .bind(&instance.version)
        .bind(instance.port)
        .bind(&instance.status)
        .bind(&instance.install_path)
        .bind(&instance.data_dir)
        .bind(&instance.config_file)
        .bind(&instance.root_user)
        .bind(instance.id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM databases WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE databases SET status=?, updated_at=datetime('now') WHERE id=?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }
}

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let row = sqlx::query_scalar::<_, String>(
            "SELECT value FROM panel_settings WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        let rows = sqlx::query_as::<_, PanelSetting>(
            "SELECT key, value, description, updated_at FROM panel_settings ORDER BY key",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let rows = sqlx::query("SELECT key, value FROM panel_settings")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows.iter().map(|r| {
            (r.get::<String, _>("key"), r.get::<String, _>("value"))
        }).collect())
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
        let perms = sqlx::query_as::<_, Permission>(
            "SELECT id, resource, action, description FROM permissions ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(perms)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError> {
        let perm = sqlx::query_as::<_, Permission>(
            "SELECT id, resource, action, description FROM permissions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(perm)
    }

    async fn find_by_resource_action(&self, resource: &str, action: &str) -> Result<Option<Permission>, AppError> {
        let perm = sqlx::query_as::<_, Permission>(
            "SELECT id, resource, action, description FROM permissions WHERE resource = ? AND action = ?",
        )
        .bind(resource)
        .bind(action)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(perm)
    }

    async fn create(&self, permission: &Permission) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO permissions (resource, action, description) VALUES (?, ?, ?)",
        )
        .bind(&permission.resource)
        .bind(&permission.action)
        .bind(&permission.description)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
        .last_insert_rowid();
        Ok(id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM permissions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
        let roles = sqlx::query_as::<_, Role>(
            "SELECT id, name, description FROM roles ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(roles)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT id, name, description FROM roles WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(role)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT id, name, description FROM roles WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(role)
    }

    async fn create(&self, role: &Role) -> Result<i64, AppError> {
        let id = sqlx::query(
            "INSERT INTO roles (name, description) VALUES (?, ?)",
        )
        .bind(&role.name)
        .bind(&role.description)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
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
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT permission_id FROM role_permissions WHERE role_id = ?",
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn set_role_permissions(&self, role_id: i64, permission_ids: &[i64]) -> Result<(), AppError> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(role_id)
            .execute(&self.pool)
            .await
            .ok();
        for pid in permission_ids {
            sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                .bind(role_id)
                .bind(pid)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        }
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
    async fn create(&self, username: &str, action: &str, target: Option<&str>, ip: Option<&str>) -> Result<OperationLog, AppError> {
        let result = sqlx::query(
            "INSERT INTO operation_logs (username, action, target, ip, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(username)
        .bind(action)
        .bind(target)
        .bind(ip)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
        let rows = sqlx::query_as::<_, OperationLog>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs ORDER BY id DESC LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        let row = sqlx::query_as::<_, OperationLog>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        let rows = sqlx::query_as::<_, OperationLog>(
            "SELECT id, username, action, target, ip, created_at FROM operation_logs WHERE username = ? ORDER BY id DESC LIMIT 100",
        )
        .bind(username)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }
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
    async fn create(&self, source: &str, level: &str, message: &str, metadata: Option<&str>) -> Result<LogEntry, AppError> {
        let result = sqlx::query(
            "INSERT INTO logs (source, level, message, metadata, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(source)
        .bind(level)
        .bind(message)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
        let rows = sqlx::query_as::<_, LogEntry>(
            "SELECT id, source, level, message, metadata, created_at FROM logs ORDER BY id DESC LIMIT 200",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        let row = sqlx::query_as::<_, LogEntry>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntry>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE source = ? ORDER BY id DESC LIMIT 200",
        )
        .bind(source)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        let rows = sqlx::query_as::<_, LogEntry>(
            "SELECT id, source, level, message, metadata, created_at FROM logs WHERE level = ? ORDER BY id DESC LIMIT 200",
        )
        .bind(level)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
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
        let rows = sqlx::query_as::<_, FirewallRule>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules ORDER BY priority, id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(rows)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<FirewallRule>, AppError> {
        let row = sqlx::query_as::<_, FirewallRule>(
            "SELECT id, name, description, protocol, port, source, destination, action, enabled, priority, direction, created_at, updated_at FROM firewall_rules WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
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
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?
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
        .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM firewall_rules WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError> {
        sqlx::query("UPDATE firewall_rules SET enabled=?, updated_at=datetime('now') WHERE id=?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError> {
        let mut priority = 10i32;
        for id in ids {
            sqlx::query("UPDATE firewall_rules SET priority=?, updated_at=datetime('now') WHERE id=?")
                .bind(priority)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(format!("DB error: {}", e)))?;
            priority += 10;
        }
        Ok(())
    }
}