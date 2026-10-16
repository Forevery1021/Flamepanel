use super::*;

pub struct SqlitePluginRepository {
    pool: SqlitePool,
    read_pool: SqlitePool,
}

impl SqlitePluginRepository {
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
impl PluginRepository for SqlitePluginRepository {
    async fn save(&self, plugin: &Plugin) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO plugins (id, name, version, author, description, wasm_base64, wasm_hash, enabled, homepage, license, tags, config_schema, capabilities, settings) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, version=excluded.version, author=excluded.author, description=excluded.description, wasm_base64=excluded.wasm_base64, wasm_hash=excluded.wasm_hash, enabled=excluded.enabled, homepage=excluded.homepage, license=excluded.license, tags=excluded.tags, config_schema=excluded.config_schema, capabilities=excluded.capabilities, settings=excluded.settings, updated_at=datetime('now')",
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
        .bind(serde_json::to_string(&plugin.capabilities).unwrap_or_else(|_| "[]".into()))
        .bind(serde_json::to_string(&plugin.settings).unwrap_or_else(|_| "{}".into()))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Plugin>, AppError> {
        let rows = sqlx::query_as::<_, PluginDbRow>(
            "SELECT id, name, version, author, description, wasm_base64, wasm_hash, enabled, homepage, license, tags, config_schema, capabilities, settings, created_at, updated_at FROM plugins ORDER BY created_at",
        )
        .fetch_all(&self.read_pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        rows.into_iter().map(PluginDbRow::into_entity).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Plugin>, AppError> {
        let row = sqlx::query_as::<_, PluginDbRow>(
            "SELECT id, name, version, author, description, wasm_base64, wasm_hash, enabled, homepage, license, tags, config_schema, capabilities, settings, created_at, updated_at FROM plugins WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.read_pool)
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
    capabilities: String,
    settings: String,
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
            capabilities: serde_json::from_str(&self.capabilities).unwrap_or_default(),
            settings: serde_json::from_str(&self.settings).unwrap_or_default(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// ─── 定时任务 SQLite 仓储 ────────────────────────────────────────────────
