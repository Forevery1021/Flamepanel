use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::infrastructure::db_models::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// F-14: SQLite 读写双池——写池 max_connections=1 串行化写入（消除写-写锁等待），
/// 读池 max_connections=8 承担并发读（WAL 模式下读写互不阻塞）。
pub struct SqliteRwPools {
    pub write: SqlitePool,
    pub read: SqlitePool,
}

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
    // PRAGMA 不支持参数绑定，表名为硬编码常量（调用点字面量），经 AssertSqlSafe 包装
    let pragma_sql: String = format!("PRAGMA table_info({})", table);
    let exists: bool = sqlx::query(sqlx::AssertSqlSafe(pragma_sql))
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

mod app_package;
mod app_source;
mod backup_account;
mod backup_job;
mod certificate;
mod database;
mod dns_provider_account;
mod firewall;
mod installed_app;
mod log;
mod mcp;
mod memo;
mod metrics_history;
mod node;
mod operation_log;
mod outbox;
mod permission;
mod plugin;
mod project;
mod project_db_link;
mod revoked_token;
mod role;
mod runtime;
mod scheduled_task;
mod settings;
mod task_store;
mod two_factor;
mod user;
mod waf_log;
mod waf_rule;
mod web_server;
mod website;

pub use app_package::SqliteAppPackageRepository;
pub use app_source::SqliteAppSourceRepository;
pub use backup_account::SqliteBackupAccountRepository;
pub use backup_job::SqliteBackupJobRepository;
pub use certificate::SqliteCertificateRepository;
pub use database::SqliteDatabaseRepository;
pub use dns_provider_account::SqliteDnsProviderAccountRepository;
pub use firewall::SqliteFirewallRepository;
pub use installed_app::SqliteInstalledAppRepository;
pub use log::SqliteLogRepository;
pub use mcp::SqliteMcpRepository;
pub use memo::SqliteMemoRepository;
pub use metrics_history::SqliteMetricsHistoryRepository;
pub use node::SqliteNodeRepository;
pub use operation_log::SqliteOperationLogRepository;
pub use outbox::SqliteOutboxRepository;
pub use permission::SqlitePermissionRepository;
pub use plugin::SqlitePluginRepository;
pub use project::SqliteProjectRepository;
pub use project_db_link::SqliteProjectDbLinkRepository;
pub use revoked_token::SqliteRevokedTokenRepository;
pub use role::SqliteRoleRepository;
pub use runtime::SqliteRuntimeRepository;
pub use scheduled_task::SqliteScheduledTaskRepository;
pub use settings::SqliteSettingsRepository;
pub use task_store::SqliteTaskStore;
pub use two_factor::SqliteTwoFactorRepository;
pub use user::SqliteUserRepository;
pub use waf_log::SqliteWafLogRepository;
pub use waf_rule::SqliteWafRuleRepository;
pub use web_server::SqliteWebServerRepository;
pub use website::SqliteWebsiteRepository;

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

    // F-04: users 表补充 password_changed_at 列（改密时间 Unix 秒，用于持久化吊销 refresh token）
    add_column_if_missing(
        pool,
        "users",
        "password_changed_at",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

    // F-04: 已吊销 refresh token 黑名单表（持久化，重启不丢失）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS revoked_refresh_tokens (
            token_hash TEXT PRIMARY KEY,
            expires_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires ON revoked_refresh_tokens(expires_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // S-07: 监控历史降采样表（1m / 1h 两级点同表，主键 = 分辨率 + 桶起点，区间查询走主键前缀）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS metrics_history (
            resolution TEXT NOT NULL,
            bucket_start INTEGER NOT NULL,
            samples INTEGER NOT NULL DEFAULT 0,
            cpu_avg REAL NOT NULL DEFAULT 0,
            cpu_max REAL NOT NULL DEFAULT 0,
            memory_avg REAL NOT NULL DEFAULT 0,
            memory_max REAL NOT NULL DEFAULT 0,
            disk_avg REAL NOT NULL DEFAULT 0,
            load_one_avg REAL NOT NULL DEFAULT 0,
            net_rx_avg REAL NOT NULL DEFAULT 0,
            net_tx_avg REAL NOT NULL DEFAULT 0,
            PRIMARY KEY (resolution, bucket_start)
        ) WITHOUT ROWID",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

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
            site_type TEXT NOT NULL DEFAULT 'static',
            runtime_id INTEGER,
            project_id INTEGER,
            rewrite_preset TEXT,
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

    // M-B 主线二 Stage A-6 · Doc/20 §5.5：网站 ⇄ 运行时 ⇄ 项目绑定（旧库升级，幂等）
    add_column_if_missing(
        pool,
        "websites",
        "site_type",
        "TEXT NOT NULL DEFAULT 'static'",
    )
    .await?;
    add_column_if_missing(pool, "websites", "runtime_id", "INTEGER").await?;
    add_column_if_missing(pool, "websites", "project_id", "INTEGER").await?;
    // 伪静态模板持久化（Doc/20 §5.5 伪静态规则库「套用即生效」，旧库升级幂等）
    add_column_if_missing(pool, "websites", "rewrite_preset", "TEXT").await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_websites_site_type ON websites(site_type)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

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
    // F-14: operation_logs 组合索引（username + created_at）
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_user_created ON operation_logs(username, created_at)",
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
    // RBAC 种子（Doc/07 §5）：权限以 (resource, action) 为业务主键——
    // 既让种子 upsert 有冲突目标，也把「同资源同动作重复定义」从约定升级为约束。
    //
    // 历史说明：permissions 此前**没有任何写入路径**，存量库该表必然为空，故补建唯一
    // 索引对历史数据零风险。仍保留一次幂等去重（保留最小 id 行）以覆盖「手工改过库」
    // 的实例——先丢弃指向被裁掉行的映射（含孤儿映射），再裁权限行，避免撞主键/外键。
    sqlx::query(
        "DELETE FROM role_permissions WHERE permission_id NOT IN (
             SELECT MIN(id) FROM permissions GROUP BY resource, action
         )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query(
        "DELETE FROM permissions WHERE id NOT IN (SELECT MIN(id) FROM permissions GROUP BY resource, action)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_permissions_resource_action ON permissions(resource, action)",
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
    // F-14: logs 高频过滤组合索引（source + created_at）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_source_created ON logs(source, created_at)")
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

    // 幂等迁移（S-04）：多页签栏已移除，前端不再消费 `open_menu_tabs`，清掉旧库残留行
    sqlx::query("DELETE FROM panel_settings WHERE key = 'open_menu_tabs'")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    // 幂等迁移（W4-a）：签名密钥不得留在 settings 表——旧版「轮换密钥」把随机串写进
    // `jwt_secret`，`rotate-secret` 又冗余写 `jwt_secret_persisted`，两者都会被
    // `GET /api/settings` 返回给只读账号。密钥只保存在 data/jwt_secret.key。
    sqlx::query("DELETE FROM panel_settings WHERE key IN ('jwt_secret', 'jwt_secret_persisted')")
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

    // M-A Issue 1.1: 远程应用源——app_packages 补充来源三列（旧库升级，幂等加列）
    add_column_if_missing(
        pool,
        "app_packages",
        "source",
        "TEXT NOT NULL DEFAULT 'local'",
    )
    .await?;
    add_column_if_missing(pool, "app_packages", "source_url", "TEXT").await?;
    add_column_if_missing(pool, "app_packages", "synced_at", "DATETIME").await?;

    // M-A Issue 1.1: 可配置多远程源表（git | http），UNIQUE(url) 防重复添加
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            public_key TEXT,
            last_synced_at DATETIME,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(url)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // app_sources 启用状态高频过滤（源列表开关 + 同步调度筛选）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_app_sources_enabled ON app_sources(enabled)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    // app_packages 按源过滤（GET /api/app-store/packages?source=:id 列表查询）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_app_packages_source ON app_packages(source)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // M-A 收尾 Stage 1: 备份账号表（Doc/20 §6.5）——s3 | sftp | local，UNIQUE(name) 防重复命名
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS backup_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            account_type TEXT NOT NULL,
            config_json TEXT NOT NULL DEFAULT '{}',
            enabled INTEGER NOT NULL DEFAULT 1,
            last_test_status TEXT,
            last_test_at DATETIME,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(name)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 备份账号启用状态过滤（账号列表开关 + 备份任务调度筛选）
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_backup_accounts_enabled ON backup_accounts(enabled)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // M-A 收尾 Stage 3: 备份任务表（Doc/20 §6.5）——计划备份：scope → schedule → target_account
    // target_account_id 引用 backup_accounts(id)，账号删除由服务层校验拒绝（SQLite 未开外键，保持与既有表一致）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS backup_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            scope TEXT NOT NULL DEFAULT 'panel',
            scope_key TEXT,
            schedule TEXT NOT NULL DEFAULT '0 3 * * *',
            target_account_id INTEGER NOT NULL DEFAULT 0,
            retention INTEGER NOT NULL DEFAULT 7,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_status TEXT NOT NULL DEFAULT 'never',
            last_message TEXT,
            last_remote_key TEXT,
            last_run_at DATETIME,
            next_run_at DATETIME,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(name)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 备份任务启用状态过滤（任务列表开关 + 调度筛选）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_backup_jobs_enabled ON backup_jobs(enabled)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // M-B 主线二 Stage A-1: 运行时表（Doc/20 §5.2）——多版本共存，同 (kind, version) 唯一
    // status: not_installed | installing | installed | running | error
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS runtimes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            version TEXT NOT NULL,
            install_path TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'not_installed',
            is_default INTEGER NOT NULL DEFAULT 0,
            metadata_json TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(kind, version)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 运行时按语言种类过滤（RuntimeView 语言 Tab + 可用版本清单）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_runtimes_kind ON runtimes(kind)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // M-B 主线二 Stage A-1: 用户项目表（Doc/20 §5.2）——Runtime + Project 双层模型的 Project 层
    // runtime_id 引用 runtimes(id)，运行时删除由服务层校验（SQLite 未开外键，与既有表一致）
    // status: building | running | stopped | error
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            runtime_id INTEGER,
            kind TEXT NOT NULL DEFAULT 'static',
            source TEXT NOT NULL DEFAULT 'local_path',
            source_url TEXT,
            deploy_path TEXT NOT NULL DEFAULT '',
            run_command TEXT,
            port INTEGER,
            domain TEXT,
            status TEXT NOT NULL DEFAULT 'stopped',
            process_unit TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(name)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 项目按状态过滤（项目列表 + 部署任务筛选）与运行时关联查询
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_projects_runtime_id ON projects(runtime_id)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // M-B 主线二 Stage A-7（Doc/20 §5.8）：git push 即部署——webhook 部署通道
    // 幂等加列：deploy_type（manual | webhook）/ auto_deploy / webhook_token
    add_column_if_missing(
        pool,
        "projects",
        "deploy_type",
        "TEXT NOT NULL DEFAULT 'manual'",
    )
    .await?;
    add_column_if_missing(
        pool,
        "projects",
        "auto_deploy",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column_if_missing(
        pool,
        "projects",
        "webhook_token",
        "TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    // M-B 主线二 Stage A-8（Doc/20 §5.8 ③）：Procfile 进程类型声明 JSON
    add_column_if_missing(pool, "projects", "procfile_json", "TEXT").await?;
    // M-B 主线二 Stage A-8（Doc/20 §5.8 ②）：构建器协商结果（dockerfile | buildpacks | runtime）
    add_column_if_missing(
        pool,
        "projects",
        "build_type",
        "TEXT NOT NULL DEFAULT 'runtime'",
    )
    .await?;
    // M-B 主线二 Stage A-9（Doc/20 §5.8 ④）：零停机切换——健康检查路径与切换策略
    add_column_if_missing(
        pool,
        "projects",
        "health_check_path",
        "TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column_if_missing(
        pool,
        "projects",
        "deploy_strategy",
        "TEXT NOT NULL DEFAULT 'restart'",
    )
    .await?;

    // M-B 主线二 Stage A-10（Doc/20 §5.8 ⑤）：项目数据库联链——Dokku link 模式
    // project_id 引用 projects(id)，db_instance_id 引用 database_instances(id)，
    // 引用完整性由服务层校验（SQLite 未开外键，与既有表一致）。
    // 同项目内 env_name 唯一（同一变量只能有一个来源），同项目+同实例不重复关联。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_db_links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            db_instance_id INTEGER NOT NULL,
            env_name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(project_id, env_name),
            UNIQUE(project_id, db_instance_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 联链按项目过滤（启动编排注入 env 时的高频查询）
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_project_db_links_project ON project_db_links(project_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // ── SSL 证书记录表（M-B 主线三 §6.1 · Stage 1 数据层）──────────────────
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS certificates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domains TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'letsencrypt',
            dns_provider TEXT,
            auto_renew INTEGER NOT NULL DEFAULT 1,
            cert_path TEXT,
            key_path TEXT,
            expires_at DATETIME,
            last_renewed_at DATETIME,
            website_id INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(domains)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // 续签扫描：按到期时间过滤即将过期证书（renewer tick 高频查询）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_certificates_expires ON certificates(expires_at)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    // 网站维度查询：绑定/解绑/展示站点证书
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_certificates_website ON certificates(website_id)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // ── DNS 厂商账号表（主线三 §6.1 · Stage 4）──────────────────────────
    // DNS-01 验证的厂商 API 凭据托管；credentials_json 落库前经 SecretCipher 加密。
    // UNIQUE(name) 防重复命名（对齐 backup_accounts）。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS dns_provider_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            provider_type TEXT NOT NULL,
            credentials_json TEXT NOT NULL DEFAULT '{}',
            enabled INTEGER NOT NULL DEFAULT 1,
            last_test_status TEXT,
            last_test_at DATETIME,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(name)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // DNS 厂商账号启用状态过滤（账号列表开关 + 续签调度筛选）
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dns_provider_accounts_enabled ON dns_provider_accounts(enabled)",
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
    // batch-14：WASM 插件宿主能力与持久化设置
    add_column_if_missing(
        pool,
        "plugins",
        "capabilities",
        "TEXT NOT NULL DEFAULT '[]'",
    )
    .await?;
    add_column_if_missing(pool, "plugins", "settings", "TEXT NOT NULL DEFAULT '{}'").await?;

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

    // WAF 基础防护（主线三 §6.2 · Doc/20）：规则表 + 拦截日志表（幂等）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS waf_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            rule_type TEXT NOT NULL,
            website_id INTEGER NOT NULL DEFAULT 0,
            pattern TEXT NOT NULL DEFAULT '',
            config_json TEXT NOT NULL DEFAULT '{}',
            action TEXT NOT NULL DEFAULT 'log',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // waf_rules 查询覆盖：按站点维度过滤 / 引擎评估拉取启用规则
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_waf_rules_website ON waf_rules(website_id)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_waf_rules_enabled ON waf_rules(enabled)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS waf_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            website_id INTEGER NOT NULL DEFAULT 0,
            rule_id INTEGER NOT NULL DEFAULT 0,
            rule_name TEXT NOT NULL DEFAULT '',
            rule_type TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL DEFAULT 'log',
            client_ip TEXT NOT NULL DEFAULT '',
            request_summary TEXT NOT NULL DEFAULT '',
            user_agent TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // waf_logs 查询覆盖：站点维度过滤 + 按时间倒序分页 + 保留清理（created_at）
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_waf_logs_website ON waf_logs(website_id)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_waf_logs_created ON waf_logs(created_at)")
        .execute(pool)
        .await
        .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;

    // M-AI-1：AI 接入凭证 / MCP 会话 / 工具调用审计
    for ddl in [
        "CREATE TABLE IF NOT EXISTS agent_principals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            user_id INTEGER NOT NULL,
            token_prefix TEXT NOT NULL,
            token_hash TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            last_used_at TEXT,
            expires_at TEXT,
            revoked_at TEXT
        )",
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id TEXT PRIMARY KEY,
            principal_id INTEGER NOT NULL,
            client_name TEXT NOT NULL DEFAULT '',
            client_version TEXT NOT NULL DEFAULT '',
            protocol_version TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            closed_at TEXT
        )",
        "CREATE INDEX IF NOT EXISTS idx_agent_sessions_seen ON agent_sessions(last_seen_at)",
        "CREATE TABLE IF NOT EXISTS tool_calls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            principal_id INTEGER NOT NULL,
            session_id TEXT,
            tool TEXT NOT NULL,
            arguments TEXT NOT NULL DEFAULT '',
            ok INTEGER NOT NULL DEFAULT 1,
            error TEXT NOT NULL DEFAULT '',
            duration_ms INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_tool_calls_principal ON tool_calls(principal_id)",
        "CREATE INDEX IF NOT EXISTS idx_tool_calls_session ON tool_calls(session_id)",
        // M-AI-2：写操作工具审批单
        "CREATE TABLE IF NOT EXISTS tool_approvals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            principal_id INTEGER NOT NULL,
            session_id TEXT,
            tool TEXT NOT NULL,
            risk TEXT NOT NULL,
            arguments TEXT NOT NULL DEFAULT '{}',
            raw_arguments TEXT NOT NULL DEFAULT '{}',
            preview TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            requested_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            decided_by INTEGER,
            decided_at TEXT,
            reason TEXT NOT NULL DEFAULT '',
            result TEXT NOT NULL DEFAULT '',
            finished_at TEXT
        )",
        "CREATE INDEX IF NOT EXISTS idx_tool_approvals_status ON tool_approvals(status)",
        "CREATE INDEX IF NOT EXISTS idx_tool_approvals_principal ON tool_approvals(principal_id)",
        // Doc/33：TOTP 两步验证（每用户一行；密钥加密存储，恢复码只存哈希）
        "CREATE TABLE IF NOT EXISTS user_two_factor (
            user_id INTEGER PRIMARY KEY,
            secret TEXT NOT NULL DEFAULT '',
            enabled INTEGER NOT NULL DEFAULT 0,
            pending_secret TEXT,
            last_step INTEGER,
            recovery_hashes TEXT NOT NULL DEFAULT '[]',
            enabled_at TEXT,
            updated_at TEXT NOT NULL
        )",
    ] {
        sqlx::query(ddl)
            .execute(pool)
            .await
            .map_err(|e| AppError::internal(format!("Migration error: {}", e)))?;
    }
    // M-AI-2：凭证级写操作开关（老凭证默认只读）
    add_column_if_missing(
        pool,
        "agent_principals",
        "allow_write",
        "INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

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

/// RBAC 种子（Doc/07 §5）：SQLite 模式与 InMemory 模式行为对齐。
///
/// **背景（P0 缺陷，本函数即修复）**：`run_migrations()` 只建表，
/// `permissions` / `roles` / `role_permissions` 三张表在 SQLite 模式下**没有任何写入
/// 路径**——InMemory 模式靠仓储构造器种子，SQLite 模式则完全依赖这三张表里的数据。
/// 结果是生产（SQLite）模式下三表恒空：`RoleService::check_permission()` 对任何角色都
/// 命中空集合，admin 登录后所有业务接口一律
/// `403 Forbidden: Missing permission: <resource>:<action>`，面板除 `/api/auth/*`
/// 外整体不可用（向导模式与无人值守模式同样命中）。
///
/// **幂等与不覆盖原则**：
/// - `permissions`：以 `UNIQUE(resource, action)` 为业务主键 upsert，描述随版本演进更新；
/// - `roles`：以 `UNIQUE(name)` upsert，同样只更新描述；
/// - `role_permissions`：**仅在角色尚无任何映射时**写入默认集合——已存在的映射
///   （后续经角色管理能力定制的授权）一律不动，避免每次启动重置管理员的自定义授权。
///
/// **权限 id 不假设与枚举下标一致**：旧库或部分种子场景下 AUTOINCREMENT 取值可能与
/// `default_permissions()` 的下标错位，故 `role_permissions()` 返回的「下标 id」先经
/// 枚举表翻译为 `(resource, action)` 业务键，再回查库内真实 id 后落库。
///
/// 调用点：`RepoFactory::new_sqlite()`（迁移之后、任何服务构造之前）——RBAC 未就绪时
/// 面板整体 403，因此必须**同步**完成，不能像默认设置那样丢给后台任务。
pub async fn seed_rbac(pool: &SqlitePool) -> Result<(), AppError> {
    let builtin_perms = crate::domain::entity::default_permissions();
    // 枚举下标 → 业务键：翻译 role_permissions() 返回的下标 id
    let id_to_key: std::collections::HashMap<i64, (String, String)> = builtin_perms
        .iter()
        .map(|p| (p.id, (p.resource.clone(), p.action.clone())))
        .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::internal(format!("DB begin tx error: {}", e)))?;

    // 0) 记录种子前已存在的权限键：本次新增的内置权限（版本升级）需要对内置角色"补授"，
    //    而既有权限的映射一律不动（管理员可能已显式收回）。
    let pre_existing: std::collections::HashSet<(String, String)> = {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT resource, action FROM permissions")
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        rows.into_iter().collect()
    };

    // 1) permissions：upsert（冲突目标 = 上文的唯一索引）
    for perm in &builtin_perms {
        sqlx::query(
            "INSERT INTO permissions (resource, action, description) VALUES (?, ?, ?)
             ON CONFLICT(resource, action) DO UPDATE SET description = excluded.description",
        )
        .bind(&perm.resource)
        .bind(&perm.action)
        .bind(&perm.description)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
    }

    // 2) 回查库内真实 id，建立 业务键 → id 映射
    let rows: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id, resource, action FROM permissions")
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
    let key_to_id: std::collections::HashMap<(String, String), i64> = rows
        .into_iter()
        .map(|(id, resource, action)| ((resource, action), id))
        .collect();

    // 3) roles + role_permissions
    for role in crate::domain::entity::default_roles() {
        // 刻意不写显式 id：自有 id 可能与既有行撞主键（而 ON CONFLICT(name) 只覆盖
        // 重名冲突），交给 AUTOINCREMENT 分配；新库得 1/2/3，与 default_roles() 一致。
        sqlx::query(
            "INSERT INTO roles (name, description) VALUES (?, ?)
             ON CONFLICT(name) DO UPDATE SET description = excluded.description",
        )
        .bind(&role.name)
        .bind(&role.description)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        // 名称 → 真实 id（冲突分支可能保留既有 id，不能假设等于 role.id）
        let role_id: i64 = sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
            .bind(&role.name)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;

        // 非覆盖：已有映射说明该角色被显式配置过（或已被种子填充），既有权限保持现状；
        // 仅对本次升级新增的内置权限按角色默认集补授（否则老库升级后新功能对所有人 403）。
        let (existing,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM role_permissions WHERE role_id = ?")
                .bind(role_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        let only_new = existing > 0;

        for pid in crate::domain::entity::role_permissions(&role.name) {
            let Some((resource, action)) = id_to_key.get(&pid) else {
                continue;
            };
            let key = (resource.clone(), action.clone());
            if only_new && pre_existing.contains(&key) {
                continue;
            }
            let Some(real_id) = key_to_id.get(&key) else {
                continue;
            };
            sqlx::query(
                "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
            )
            .bind(role_id)
            .bind(real_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("DB error: {}", e)))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::internal(format!("DB commit error: {}", e)))?;
    tracing::info!(
        "RBAC seed ensured: {} builtin permissions / {} builtin roles",
        builtin_perms.len(),
        crate::domain::entity::default_roles().len()
    );
    Ok(())
}

/// 转义 LIKE 通配符（`%` / `_` / 转义符 `\`），使前缀过滤退化为字面前缀匹配。
pub(super) fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod waf_repo_tests {
    use super::*;

    async fn setup() -> (SqliteWafRuleRepository, SqliteWafLogRepository) {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("in-memory pool");
        run_migrations(&pool).await.expect("migrations");
        (
            SqliteWafRuleRepository::new(pool.clone()),
            SqliteWafLogRepository::new(pool),
        )
    }

    /// 测试辅助：取仓储内的写池（时间回拨用）。
    fn pool_of(repo: &SqliteWafLogRepository) -> &SqlitePool {
        &repo.pool
    }

    fn sample_rule(name: &str, rule_type: &str, website_id: i64) -> WafRule {
        WafRule {
            id: 0,
            name: name.into(),
            rule_type: rule_type.into(),
            website_id,
            pattern: "1.2.3.4".into(),
            config_json: "{}".into(),
            action: "log".into(),
            enabled: true,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn rule_crud_and_filters() {
        let (rules, _) = setup().await;
        let g = rules
            .create(&sample_rule("global-bl", "ip_blacklist", 0))
            .await
            .unwrap();
        let s = rules
            .create(&sample_rule("site-bl", "path_block", 5))
            .await
            .unwrap();

        let all = rules.list_all(None).await.unwrap();
        assert_eq!(all.len(), 2);
        let site = rules.list_all(Some(5)).await.unwrap();
        assert_eq!(site.len(), 1);
        assert_eq!(site[0].id, s);

        let mut r = rules.find_by_id(g).await.unwrap().unwrap();
        r.action = "block".into();
        r.enabled = false;
        rules.update(&r).await.unwrap();
        let updated = rules.find_by_id(g).await.unwrap().unwrap();
        assert_eq!(updated.action, "block");
        assert!(!updated.enabled);
        // 启用过滤：停用规则不出现在 list_enabled
        assert_eq!(rules.list_enabled().await.unwrap().len(), 1);

        rules.delete(g).await.unwrap();
        assert!(rules.find_by_id(g).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn update_missing_rule_returns_not_found() {
        let (rules, _) = setup().await;
        let mut r = sample_rule("ghost", "ip_blacklist", 0);
        r.id = 999;
        assert!(rules.update(&r).await.is_err());
    }

    #[tokio::test]
    async fn log_crud_pagination_and_retention() {
        let (_, logs) = setup().await;
        let mk = |wid: i64, i: i64| WafLog {
            id: 0,
            website_id: wid,
            rule_id: 1,
            rule_name: format!("r{i}"),
            rule_type: "sql_injection".into(),
            action: "block".into(),
            client_ip: "10.0.0.1".into(),
            request_summary: "GET /a?id=1'".into(),
            user_agent: "sqlmap".into(),
            created_at: chrono::Utc::now(),
        };
        for i in 0..3 {
            logs.create(&mk(0, i)).await.unwrap();
        }
        logs.create(&mk(7, 99)).await.unwrap();

        assert_eq!(logs.count(None).await.unwrap(), 4);
        assert_eq!(logs.count(Some(7)).await.unwrap(), 1);
        // website_id = 0 → 不过滤
        assert_eq!(logs.count(Some(0)).await.unwrap(), 4);

        let page = logs.list_page(2, 0, None).await.unwrap();
        assert_eq!(page.len(), 2);
        // 倒序：最新在前
        assert_eq!(page[0].rule_name, "r99");

        let site_page = logs.list_page(10, 0, Some(7)).await.unwrap();
        assert_eq!(site_page.len(), 1);

        // 保留清理：把日志时间回拨 2 天后，delete_older_than(1) 应清空全部
        sqlx::query("UPDATE waf_logs SET created_at = datetime('now', '-2 days')")
            .execute(pool_of(&logs))
            .await
            .unwrap();
        let removed = logs.delete_older_than(1).await.unwrap();
        assert_eq!(removed, 4);
        assert_eq!(logs.count(None).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn log_clear_by_site_and_find() {
        let (_, logs) = setup().await;
        let mk = |wid: i64| WafLog {
            id: 0,
            website_id: wid,
            rule_id: 1,
            rule_name: "r".into(),
            rule_type: "cc".into(),
            action: "log".into(),
            client_ip: "10.0.0.2".into(),
            request_summary: "GET /".into(),
            user_agent: "ua".into(),
            created_at: chrono::Utc::now(),
        };
        let id1 = logs.create(&mk(1)).await.unwrap();
        let _ = logs.create(&mk(2)).await.unwrap();
        let found = logs.find_by_id(id1).await.unwrap().unwrap();
        assert_eq!(found.website_id, 1);

        let cleared = logs.clear(Some(1)).await.unwrap();
        assert_eq!(cleared, 1);
        assert_eq!(logs.count(None).await.unwrap(), 1);
        let cleared_all = logs.clear(None).await.unwrap();
        assert_eq!(cleared_all, 1);
    }
}

#[cfg(test)]
mod settings_migration_tests {
    //! S-04：`open_menu_tabs` 设置项下线——旧库残留行由幂等迁移清理，
    //! 且 `ensure_defaults` 不会再把它写回。

    use super::*;

    #[tokio::test]
    async fn legacy_open_menu_tabs_row_is_removed_and_not_reseeded() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 模拟旧版本落库的残留行
        sqlx::query(
            "INSERT INTO panel_settings (key, value, description) VALUES ('open_menu_tabs', 'true', '是否开启多页签')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // 再跑一次迁移（升级路径）：残留行被清掉，且重复执行安全
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let repo = SqliteSettingsRepository::new(pool.clone());
        repo.ensure_defaults().await.unwrap();

        let legacy: Option<String> =
            sqlx::query_scalar("SELECT value FROM panel_settings WHERE key = 'open_menu_tabs'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(legacy.is_none(), "legacy open_menu_tabs row must be purged");
        assert!(
            !crate::domain::entity::default_settings()
                .iter()
                .any(|s| s.key == "open_menu_tabs"),
            "default_settings must not reseed open_menu_tabs"
        );
        // 其余默认项仍完整（menu_accordion 是同一批 UI 设置中保留的一项）
        let accordion: Option<String> =
            sqlx::query_scalar("SELECT value FROM panel_settings WHERE key = 'menu_accordion'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert_eq!(accordion.as_deref(), Some("false"));
    }

    #[tokio::test]
    async fn leaked_jwt_secret_rows_are_purged() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        for key in ["jwt_secret", "jwt_secret_persisted"] {
            sqlx::query(
                "INSERT INTO panel_settings (key, value, description) VALUES (?, 'leak', '')",
            )
            .bind(key)
            .execute(&pool)
            .await
            .unwrap();
        }
        run_migrations(&pool).await.unwrap();
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM panel_settings WHERE key IN ('jwt_secret', 'jwt_secret_persisted')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(n, 0);
    }
}

#[cfg(test)]
mod rbac_seed_tests {
    //! RBAC 种子（`seed_rbac`）回归：SQLite 模式下权限/角色/角色权限三表的唯一
    //! 写入路径。修复前该函数不存在，三表恒空 → admin 登录后所有业务接口 403。
    //!
    //! 覆盖四类不变量：
    //! 1. 全量落库 + 语义正确（admin=全部 / operator=无 delete / viewer=仅 read）；
    //! 2. 重复执行幂等（不重复插入）；
    //! 3. 不覆盖既有授权（管理员自定义收紧后重启不得被重置）；
    //! 4. 旧库升级（RBAC 表为空的老库 / 权限表有重复行的脏库）不阻断启动。

    use super::*;

    async fn fresh_pool() -> SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        // 迁移幂等性：同一库重复执行必须安全
        run_migrations(&pool).await.unwrap();
        pool
    }

    async fn grant_pairs(
        pool: &SqlitePool,
        role: &str,
    ) -> std::collections::HashSet<(String, String)> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT p.resource, p.action FROM role_permissions rp
             JOIN roles r ON r.id = rp.role_id
             JOIN permissions p ON p.id = rp.permission_id
             WHERE r.name = ?",
        )
        .bind(role)
        .fetch_all(pool)
        .await
        .unwrap();
        rows.into_iter().collect()
    }

    #[tokio::test]
    async fn seeds_permissions_roles_and_grants() {
        let pool = fresh_pool().await;
        let builtin = crate::domain::entity::default_permissions();
        seed_rbac(&pool).await.unwrap();

        let (perm_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM permissions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(perm_count as usize, builtin.len(), "权限应全量落库");

        let (role_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(role_count, 3, "内置三角色应全量落库");

        // 语义正确性：与 domain::role_permissions 的角色矩阵逐项一致
        let all: std::collections::HashSet<(String, String)> = builtin
            .iter()
            .map(|p| (p.resource.clone(), p.action.clone()))
            .collect();
        assert_eq!(grant_pairs(&pool, "admin").await, all, "admin = 全部权限");

        let operator: std::collections::HashSet<(String, String)> =
            all.iter().filter(|(_, a)| a != "delete").cloned().collect();
        assert_eq!(
            grant_pairs(&pool, "operator").await,
            operator,
            "operator = 除 delete"
        );

        let viewer: std::collections::HashSet<(String, String)> =
            all.iter().filter(|(_, a)| a == "read").cloned().collect();
        assert_eq!(
            grant_pairs(&pool, "viewer").await,
            viewer,
            "viewer = 仅 read"
        );
    }

    #[tokio::test]
    async fn repeated_seed_is_idempotent() {
        let pool = fresh_pool().await;
        seed_rbac(&pool).await.unwrap();
        let first = grant_pairs(&pool, "admin").await;
        seed_rbac(&pool).await.unwrap();
        seed_rbac(&pool).await.unwrap();

        let (perm_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM permissions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            perm_count as usize,
            crate::domain::entity::default_permissions().len(),
            "重复种子不得产生重复权限行"
        );
        let (role_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(role_count, 3, "重复种子不得产生重复角色");
        assert_eq!(grant_pairs(&pool, "admin").await, first, "授权集合保持稳定");
    }

    #[tokio::test]
    async fn does_not_overwrite_customized_grants() {
        let pool = fresh_pool().await;
        seed_rbac(&pool).await.unwrap();

        // 管理员经角色管理收紧：收回 admin 的 user:delete
        let (admin_id,): (i64,) = sqlx::query_as("SELECT id FROM roles WHERE name = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "DELETE FROM role_permissions WHERE role_id = ? AND permission_id = (
                 SELECT id FROM permissions WHERE resource = 'user' AND action = 'delete'
             )",
        )
        .bind(admin_id)
        .execute(&pool)
        .await
        .unwrap();

        let before = grant_pairs(&pool, "admin").await;
        assert!(!before.contains(&("user".into(), "delete".into())));

        // 重启（再次种子）必须保留自定义授权，不得整表重置
        seed_rbac(&pool).await.unwrap();
        assert_eq!(
            grant_pairs(&pool, "admin").await,
            before,
            "已有映射的角色不得被种子重置"
        );
    }

    #[tokio::test]
    async fn fills_role_that_has_no_mapping_yet() {
        let pool = fresh_pool().await;
        seed_rbac(&pool).await.unwrap();

        // 模拟「角色已存在但从未授权」：清空 viewer 映射后再次种子应补齐
        let (viewer_id,): (i64,) = sqlx::query_as("SELECT id FROM roles WHERE name = 'viewer'")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(viewer_id)
            .execute(&pool)
            .await
            .unwrap();
        seed_rbac(&pool).await.unwrap();

        let viewer = grant_pairs(&pool, "viewer").await;
        assert!(!viewer.is_empty(), "空映射角色应被种子补齐");
        assert!(viewer.contains(&("user".into(), "read".into())));
        assert!(
            !viewer.contains(&("user".into(), "create".into())),
            "viewer 不得获得写权限"
        );
    }

    #[tokio::test]
    async fn grants_newly_added_builtin_permissions_on_upgrade() {
        // 模拟旧版本库：已种子并由管理员收回 admin 的 user:delete，随后删除一条"新版本才有"的
        // 内置权限（toolbox:read）及其映射 → 再次种子应补授 toolbox:read，且不恢复 user:delete。
        let pool = fresh_pool().await;
        seed_rbac(&pool).await.unwrap();
        let (admin_id,): (i64,) = sqlx::query_as("SELECT id FROM roles WHERE name = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "DELETE FROM role_permissions WHERE role_id = ? AND permission_id = (
                 SELECT id FROM permissions WHERE resource = 'user' AND action = 'delete'
             )",
        )
        .bind(admin_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "DELETE FROM role_permissions WHERE permission_id IN (
                 SELECT id FROM permissions WHERE resource = 'toolbox' AND action = 'read'
             )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("DELETE FROM permissions WHERE resource = 'toolbox' AND action = 'read'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(!grant_pairs(&pool, "viewer")
            .await
            .contains(&("toolbox".into(), "read".into())));

        seed_rbac(&pool).await.unwrap();

        let admin = grant_pairs(&pool, "admin").await;
        let viewer = grant_pairs(&pool, "viewer").await;
        assert!(
            admin.contains(&("toolbox".into(), "read".into())),
            "升级新增的内置权限应补授给 admin"
        );
        assert!(
            viewer.contains(&("toolbox".into(), "read".into())),
            "viewer 默认集含 read → 同样补授"
        );
        assert!(
            !viewer.contains(&("toolbox".into(), "execute".into())),
            "补授仍遵循角色默认集"
        );
        assert!(
            !admin.contains(&("user".into(), "delete".into())),
            "既有权限的显式收回不得被升级补授恢复"
        );
    }

    #[tokio::test]
    async fn legacy_empty_rbac_tables_upgraded() {
        // 旧库形态：三表存在但为空（修复前 SQLite 模式的真实状态）
        let pool = fresh_pool().await;
        seed_rbac(&pool).await.unwrap();
        let admin = grant_pairs(&pool, "admin").await;
        assert!(
            admin.contains(&("user".into(), "read".into())),
            "升级后 admin 应真正持有 user:read（修复前此处为空集 → 全站 403）"
        );
    }

    #[tokio::test]
    async fn dedupes_pre_existing_duplicate_permissions() {
        // 脏库：手工插入过重复 (resource, action)，补建唯一索引不得失败，
        // 且指向被裁行的映射先被清理（否则撞主键/外键）。
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        for ddl in [
            "CREATE TABLE permissions (id INTEGER PRIMARY KEY AUTOINCREMENT, resource TEXT NOT NULL, action TEXT NOT NULL, description TEXT NOT NULL DEFAULT '')",
            "CREATE TABLE roles (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, description TEXT NOT NULL DEFAULT '')",
            "CREATE TABLE role_permissions (role_id INTEGER NOT NULL, permission_id INTEGER NOT NULL, PRIMARY KEY (role_id, permission_id))",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO permissions (id, resource, action, description) VALUES (7,'user','read','a'),(9,'user','read','b'),(11,'node','read','n')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO roles (id, name, description) VALUES (1,'admin','a'),(2,'operator','o'),(3,'viewer','v')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO role_permissions (role_id, permission_id) VALUES (1,7),(1,9),(1,11)",
        )
        .execute(&pool)
        .await
        .unwrap();

        run_migrations(&pool).await.unwrap();
        let (dup,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM permissions WHERE resource='user' AND action='read'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(dup, 1, "重复权限应被去重为一行（保留最小 id）");

        let (orphan,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM role_permissions rp WHERE NOT EXISTS (SELECT 1 FROM permissions p WHERE p.id = rp.permission_id)",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(orphan, 0, "去重后不得残留孤儿映射");

        // 迁移后种子仍可正常执行（索引已就位，upsert 走冲突分支）
        seed_rbac(&pool).await.unwrap();
    }
}

#[cfg(test)]
mod mcp_repo_tests {
    use super::*;
    use crate::domain::mcp::{AgentPrincipal, AgentSession, ToolCallFilter, ToolCallRecord};

    async fn repo() -> SqliteMcpRepository {
        // 单连接：`:memory:` 每条连接是独立数据库
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .expect("in-memory pool");
        run_migrations(&pool).await.expect("migrations");
        // 幂等：二次迁移不报错
        run_migrations(&pool).await.expect("migrations again");
        SqliteMcpRepository::new(pool)
    }

    fn call(pid: i64, sid: Option<&str>, tool: &str) -> ToolCallRecord {
        ToolCallRecord {
            id: 0,
            principal_id: pid,
            session_id: sid.map(str::to_string),
            tool: tool.into(),
            arguments: "{}".into(),
            ok: tool != "bad",
            error: String::new(),
            duration_ms: 3,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn principal_session_call_roundtrip() {
        let r = repo().await;
        let now = Utc::now();
        let p = AgentPrincipal {
            id: 0,
            name: "cursor".into(),
            user_id: 1,
            token_prefix: "fpk_abcd1234".into(),
            token_hash: "h1".into(),
            created_at: now,
            last_used_at: None,
            expires_at: Some(now + chrono::Duration::days(1)),
            revoked_at: None,
            allow_write: true,
        };
        let id = r.create_principal(&p).await.unwrap();
        let got = r.find_principal_by_hash("h1").await.unwrap().unwrap();
        assert_eq!(got.id, id);
        assert!(got.allow_write);
        assert_eq!(got.name, "cursor");
        assert!(got.expires_at.is_some() && got.last_used_at.is_none());
        // token_hash 唯一
        assert!(r.create_principal(&p).await.is_err());
        r.touch_principal(id, now).await.unwrap();
        r.revoke_principal(id, now).await.unwrap();
        let got = r.find_principal(id).await.unwrap().unwrap();
        assert!(got.last_used_at.is_some() && got.revoked_at.is_some());
        assert_eq!(r.list_principals().await.unwrap().len(), 1);

        let s = AgentSession {
            id: "s-1".into(),
            principal_id: id,
            client_name: "claude".into(),
            client_version: "1".into(),
            protocol_version: "2025-06-18".into(),
            created_at: now,
            last_seen_at: now,
            closed_at: None,
        };
        r.create_session(&s).await.unwrap();
        r.close_session("s-1", now).await.unwrap();
        let got = r.find_session("s-1").await.unwrap().unwrap();
        assert!(got.closed_at.is_some());
        assert_eq!(r.list_sessions(Some(id), 10).await.unwrap().len(), 1);
        assert!(r.list_sessions(Some(id + 1), 10).await.unwrap().is_empty());

        for t in ["a", "bad", "a", "b"] {
            r.record_call(&call(id, Some("s-1"), t)).await.unwrap();
        }
        r.record_call(&call(id + 1, None, "a")).await.unwrap();
        let f = ToolCallFilter {
            principal_id: Some(id),
            session_id: Some("s-1".into()),
            tool: Some("a".into()),
        };
        assert_eq!(r.count_calls(&f).await.unwrap(), 2);
        let all = r
            .list_calls(&ToolCallFilter::default(), 10, 0)
            .await
            .unwrap();
        assert_eq!(all.len(), 5);
        assert!(all[0].id > all[1].id);
        assert!(all.iter().any(|c| !c.ok));
        assert_eq!(all[0].session_id, None);
        // 保留最近 2 条
        assert_eq!(r.prune_calls(2).await.unwrap(), 3);
        assert_eq!(r.count_calls(&ToolCallFilter::default()).await.unwrap(), 2);
        assert_eq!(r.prune_calls(10).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn approval_state_machine_roundtrip() {
        use crate::domain::mcp::{ApprovalFilter, ApprovalStatus, RiskLevel, ToolApproval};
        let r = repo().await;
        let now = Utc::now();
        let mk = |pid: i64, expires: chrono::DateTime<Utc>| ToolApproval {
            id: 0,
            principal_id: pid,
            session_id: Some("s".into()),
            tool: "docker_container_stop".into(),
            risk: RiskLevel::Write,
            arguments: r#"{"container":"web"}"#.into(),
            raw_arguments: r#"{"container":"web","token":"x"}"#.into(),
            preview: "将停止容器 web".into(),
            status: ApprovalStatus::Pending,
            requested_at: now,
            expires_at: expires,
            decided_by: None,
            decided_at: None,
            reason: String::new(),
            result: String::new(),
            finished_at: None,
        };
        let a = r
            .create_approval(&mk(1, now + chrono::Duration::minutes(30)))
            .await
            .unwrap();
        let b = r
            .create_approval(&mk(2, now - chrono::Duration::seconds(1)))
            .await
            .unwrap();
        let got = r.find_approval(a).await.unwrap().unwrap();
        assert_eq!(got.risk, RiskLevel::Write);
        assert_eq!(got.status, ApprovalStatus::Pending);
        assert!(got.raw_arguments.contains("token"));
        // 过期只影响超时的待审批单
        assert_eq!(r.expire_approvals(now).await.unwrap(), 1);
        assert_eq!(
            r.find_approval(b).await.unwrap().unwrap().status,
            ApprovalStatus::Expired
        );
        // 比较并设置：第二次批准失败
        assert!(r
            .transition_approval(
                a,
                ApprovalStatus::Pending,
                ApprovalStatus::Executing,
                Some(9),
                now,
                ""
            )
            .await
            .unwrap());
        assert!(!r
            .transition_approval(
                a,
                ApprovalStatus::Pending,
                ApprovalStatus::Executing,
                Some(9),
                now,
                ""
            )
            .await
            .unwrap());
        r.finish_approval(a, ApprovalStatus::Succeeded, "ok", now)
            .await
            .unwrap();
        let got = r.find_approval(a).await.unwrap().unwrap();
        assert_eq!(got.status, ApprovalStatus::Succeeded);
        assert_eq!(got.decided_by, Some(9));
        assert_eq!(got.result, "ok");
        assert!(got.finished_at.is_some());
        // 过滤与计数
        let f = ApprovalFilter {
            principal_id: Some(1),
            status: Some(ApprovalStatus::Succeeded),
        };
        assert_eq!(r.count_approvals(&f).await.unwrap(), 1);
        assert_eq!(
            r.list_approvals(&ApprovalFilter::default(), 10, 0)
                .await
                .unwrap()[0]
                .id,
            b
        );
    }
}
