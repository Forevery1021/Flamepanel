use crate::core::error::AppError;
use crate::domain::repository::*;
use crate::infrastructure::db::*;
#[cfg(feature = "docker")]
use crate::infrastructure::docker::BollardDockerRepository;
#[cfg(feature = "sqlite")]
use crate::infrastructure::sqlite::*;
#[cfg(feature = "docker")]
use bollard::Docker;
use std::sync::Arc;

pub enum BackendKind {
    InMemory,
    #[cfg(feature = "sqlite")]
    Sqlite(crate::infrastructure::sqlite::SqliteRwPools),
}

pub struct RepoFactory {
    kind: BackendKind,
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
    /// F-16: InMemory 模式下 Docker 五仓储共享状态（仅 InMemory 模式使用）
    in_memory_docker_state: std::sync::Arc<crate::infrastructure::db::InMemoryDockerState>,
}

impl RepoFactory {
    pub fn new_in_memory() -> Self {
        Self {
            kind: BackendKind::InMemory,
            #[cfg(feature = "docker")]
            docker: None,
            in_memory_docker_state: std::sync::Arc::new(
                crate::infrastructure::db::InMemoryDockerState::default(),
            ),
        }
    }

    /// 创建 SQLite 后端（feature `sqlite` 开启时可用）。
    ///
    /// F-01: PRAGMA 加固（WAL / busy_timeout / synchronous / foreign_keys）通过
    /// `SqliteConnectOptions` 在 **每个连接建立时** 自动应用，而非仅作用于池中单个连接。
    #[cfg(feature = "sqlite")]
    pub async fn new_sqlite(database_url: &str) -> Result<Self, AppError> {
        use sqlx::sqlite::{
            SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
        };
        use std::str::FromStr;

        let opts = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| AppError::internal(format!("Invalid SQLite URL: {}", e)))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5))
            .foreign_keys(true); // F-19: 开启外键约束

        // F-14: 读写双池——写池 max=1 串行写（写-写不再互相等锁），读池 max=8 并发读
        let write_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(opts.clone())
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect write pool: {}", e)))?;
        let read_pool = SqlitePoolOptions::new()
            .max_connections(8)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect_with(opts)
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect read pool: {}", e)))?;

        run_migrations(&write_pool).await?;
        // RBAC 种子（Doc/07 §5）：权限/角色/角色权限在此成表——SQLite 模式没有任何
        // 其它写入路径，缺种子即「admin 全 403」整体不可用，故同步执行、失败即启动失败。
        seed_rbac(&write_pool).await?;
        Ok(Self {
            kind: BackendKind::Sqlite(crate::infrastructure::sqlite::SqliteRwPools {
                write: write_pool,
                read: read_pool,
            }),
            #[cfg(feature = "docker")]
            docker: None,
            in_memory_docker_state: std::sync::Arc::new(
                crate::infrastructure::db::InMemoryDockerState::default(),
            ),
        })
    }

    /// 注入 Docker 连接（feature `docker` 开启时可用）。
    #[cfg(feature = "docker")]
    pub fn with_docker_connection(mut self, docker: Docker) -> Self {
        self.docker = Some(docker);
        self
    }

    /// 尝试自动连接本机 Docker（feature `docker` 开启时可用）。
    #[cfg(feature = "docker")]
    pub fn connect_docker(mut self) -> Self {
        if let Ok(docker) = Docker::connect_with_local_defaults() {
            self.docker = Some(docker);
        }
        self
    }

    pub fn create_user_repo(&self) -> Arc<dyn UserRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryUserRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteUserRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_node_repo(&self) -> Arc<dyn NodeRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryNodeRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteNodeRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_website_repo(&self) -> Arc<dyn WebsiteRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWebsiteRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteWebsiteRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// 创建 Docker 仓库（feature `docker` 开启时优先使用 bollard 真实实现；
    /// 否则回落为 InMemory 模拟实现，便于无 Docker 环境降级运行）。
    pub fn create_docker_repo(&self) -> Arc<dyn DockerRepository> {
        #[cfg(feature = "docker")]
        {
            if let Some(docker) = &self.docker {
                match BollardDockerRepository::new_with_connection(docker.clone()) {
                    Ok(repo) => return Arc::new(repo),
                    Err(_) => { /* fall through to in-memory */ }
                }
            }
        }
        // F-16: 使用共享状态，保证容器/网络/卷/镜像/Compose 状态互通
        Arc::new(InMemoryDockerRepository::new_shared(
            self.in_memory_docker_state.clone(),
        ))
    }

    /// 按职责拆分的 Docker 端口：容器
    pub fn create_container_repo(&self) -> Arc<dyn ContainerRepository> {
        self.create_docker_repo()
    }

    /// 按职责拆分的 Docker 端口：网络
    pub fn create_network_repo(&self) -> Arc<dyn NetworkRepository> {
        self.create_docker_repo()
    }

    /// 按职责拆分的 Docker 端口：卷
    pub fn create_volume_repo(&self) -> Arc<dyn VolumeRepository> {
        self.create_docker_repo()
    }

    /// 按职责拆分的 Docker 端口：镜像
    pub fn create_image_repo(&self) -> Arc<dyn ImageRepository> {
        self.create_docker_repo()
    }

    /// 按职责拆分的 Docker 端口：Compose 编排
    pub fn create_compose_repo(&self) -> Arc<dyn ComposeRepository> {
        self.create_docker_repo()
    }

    pub fn create_permission_repo(&self) -> Arc<dyn PermissionRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryPermissionRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqlitePermissionRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_role_repo(&self) -> Arc<dyn RoleRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryRoleRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteRoleRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_operation_log_repo(&self) -> Arc<dyn OperationLogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryOperationLogRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteOperationLogRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_outbox_repo(&self) -> Arc<dyn OutboxRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryOutboxRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteOutboxRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_log_repo(&self) -> Arc<dyn LogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryLogRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteLogRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_settings_repo(&self) -> Arc<dyn SettingsRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemorySettingsRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteSettingsRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// SQLite 模式启动时补齐默认设置，保证与 InMemory 模式行为一致。
    pub async fn seed_default_settings(&self) -> Result<(), AppError> {
        #[cfg(feature = "sqlite")]
        if let BackendKind::Sqlite(pools) = &self.kind {
            SqliteSettingsRepository::new_rw(pools.write.clone(), pools.read.clone())
                .ensure_defaults()
                .await?;
        }
        Ok(())
    }

    pub fn create_web_server_repo(&self) -> Arc<dyn WebServerRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWebServerRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteWebServerRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_database_repo(&self) -> Arc<dyn DatabaseRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryDatabaseRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteDatabaseRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// 项目数据库联链仓储（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）
    pub fn create_project_db_link_repo(&self) -> Arc<dyn ProjectDbLinkRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryProjectDbLinkRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteProjectDbLinkRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// SSL 证书记录仓储（M-B 主线三 §6.1 · Stage 1 数据层）
    pub fn create_certificate_repo(&self) -> Arc<dyn CertificateRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryCertificateRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteCertificateRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_firewall_repo(&self) -> Arc<dyn FirewallRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryFirewallRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteFirewallRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_scheduled_task_repo(&self) -> Arc<dyn ScheduledTaskRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryScheduledTaskRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteScheduledTaskRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_app_package_repo(&self) -> Arc<dyn AppPackageRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryAppPackageRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteAppPackageRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_app_source_repo(&self) -> Arc<dyn AppSourceRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryAppSourceRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteAppSourceRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_backup_account_repo(&self) -> Arc<dyn BackupAccountRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryBackupAccountRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteBackupAccountRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// DNS 厂商账号仓储（主线三 §6.1 · Stage 4）
    pub fn create_dns_provider_account_repo(&self) -> Arc<dyn DnsProviderAccountRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryDnsProviderAccountRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteDnsProviderAccountRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// WAF 规则仓储（主线三 §6.2）
    pub fn create_waf_rule_repo(&self) -> Arc<dyn WafRuleRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWafRuleRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteWafRuleRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// Doc/33：TOTP 两步验证仓储
    pub fn create_two_factor_repo(
        &self,
    ) -> Arc<dyn crate::domain::two_factor::TwoFactorRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryTwoFactorRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteTwoFactorRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// M-AI-1：MCP 凭证 / 会话 / 调用审计仓储
    pub fn create_mcp_repo(&self) -> Arc<dyn McpRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryMcpRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteMcpRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// WAF 拦截日志仓储（主线三 §6.2）
    pub fn create_waf_log_repo(&self) -> Arc<dyn WafLogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWafLogRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteWafLogRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_installed_app_repo(&self) -> Arc<dyn InstalledAppRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryInstalledAppRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteInstalledAppRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_plugin_repo(&self) -> Arc<dyn PluginRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryPluginRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqlitePluginRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_memo_repo(&self) -> Arc<dyn MemoRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryMemoRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteMemoRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// F-04: 创建 refresh token 黑名单仓储
    pub fn create_revoked_token_repo(&self) -> Arc<dyn RevokedTokenRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryRevokedTokenRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteRevokedTokenRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// S-07: 监控历史降采样仓储
    pub fn create_metrics_history_repo(&self) -> Arc<dyn MetricsHistoryRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryMetricsHistoryRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteMetricsHistoryRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// 统一 Task 状态机持久化存储（Phase B1 扩展：SQLite 落库 / InMemory 默认）。
    pub fn create_task_store(&self) -> Arc<dyn crate::runtime::task_state::TaskStore> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryTaskStore::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteTaskStore::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    pub fn create_backup_job_repo(&self) -> Arc<dyn BackupJobRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryBackupJobRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteBackupJobRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// 运行时仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
    pub fn create_runtime_repo(&self) -> Arc<dyn RuntimeRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryRuntimeRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteRuntimeRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }

    /// 用户项目仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
    pub fn create_project_repo(&self) -> Arc<dyn ProjectRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryProjectRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pools) => Arc::new(SqliteProjectRepository::new_rw(
                pools.write.clone(),
                pools.read.clone(),
            )),
        }
    }
}
