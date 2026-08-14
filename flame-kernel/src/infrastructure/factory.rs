use crate::core::error::AppError;
use crate::domain::repository::*;
use crate::infrastructure::db::*;
#[cfg(feature = "docker")]
use crate::infrastructure::docker::BollardDockerRepository;
#[cfg(feature = "sqlite")]
use crate::infrastructure::sqlite::*;
#[cfg(feature = "docker")]
use bollard::Docker;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;
use std::sync::Arc;

pub enum BackendKind {
    InMemory,
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
}

pub struct RepoFactory {
    kind: BackendKind,
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
}

impl RepoFactory {
    pub fn new_in_memory() -> Self {
        Self {
            kind: BackendKind::InMemory,
            #[cfg(feature = "docker")]
            docker: None,
        }
    }

    /// 创建 SQLite 后端（feature `sqlite` 开启时可用）。
    #[cfg(feature = "sqlite")]
    pub async fn new_sqlite(database_url: &str) -> Result<Self, AppError> {
        // 连接前先为 SQLite 追加运行时加固参数：WAL 日志 + 忙等待 5s + 同步 NORMAL
        // （journal_mode/synchronous 也通过 PRAGMA 强制设置，覆盖未显式配置的情况）
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect to database: {}", e)))?;
        configure_sqlite_pragmas(&pool).await?;
        run_migrations(&pool).await?;
        Ok(Self {
            kind: BackendKind::Sqlite(pool),
            #[cfg(feature = "docker")]
            docker: None,
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
            BackendKind::Sqlite(pool) => Arc::new(SqliteUserRepository::new(pool.clone())),
        }
    }

    pub fn create_node_repo(&self) -> Arc<dyn NodeRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryNodeRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteNodeRepository::new(pool.clone())),
        }
    }

    pub fn create_website_repo(&self) -> Arc<dyn WebsiteRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWebsiteRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteWebsiteRepository::new(pool.clone())),
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
        Arc::new(InMemoryDockerRepository::new())
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
            BackendKind::Sqlite(pool) => Arc::new(SqlitePermissionRepository::new(pool.clone())),
        }
    }

    pub fn create_role_repo(&self) -> Arc<dyn RoleRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryRoleRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteRoleRepository::new(pool.clone())),
        }
    }

    pub fn create_operation_log_repo(&self) -> Arc<dyn OperationLogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryOperationLogRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteOperationLogRepository::new(pool.clone())),
        }
    }

    pub fn create_outbox_repo(&self) -> Arc<dyn OutboxRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryOutboxRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteOutboxRepository::new(pool.clone())),
        }
    }

    pub fn create_log_repo(&self) -> Arc<dyn LogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryLogRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteLogRepository::new(pool.clone())),
        }
    }

    pub fn create_settings_repo(&self) -> Arc<dyn SettingsRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemorySettingsRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteSettingsRepository::new(pool.clone())),
        }
    }

    /// SQLite 模式启动时补齐默认设置，保证与 InMemory 模式行为一致。
    pub async fn seed_default_settings(&self) -> Result<(), AppError> {
        #[cfg(feature = "sqlite")]
        if let BackendKind::Sqlite(pool) = &self.kind {
            SqliteSettingsRepository::new(pool.clone())
                .ensure_defaults()
                .await?;
        }
        Ok(())
    }

    pub fn create_web_server_repo(&self) -> Arc<dyn WebServerRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWebServerRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteWebServerRepository::new(pool.clone())),
        }
    }

    pub fn create_database_repo(&self) -> Arc<dyn DatabaseRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryDatabaseRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteDatabaseRepository::new(pool.clone())),
        }
    }

    pub fn create_firewall_repo(&self) -> Arc<dyn FirewallRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryFirewallRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteFirewallRepository::new(pool.clone())),
        }
    }

    pub fn create_scheduled_task_repo(&self) -> Arc<dyn ScheduledTaskRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryScheduledTaskRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteScheduledTaskRepository::new(pool.clone())),
        }
    }

    pub fn create_app_package_repo(&self) -> Arc<dyn AppPackageRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryAppPackageRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteAppPackageRepository::new(pool.clone())),
        }
    }

    pub fn create_installed_app_repo(&self) -> Arc<dyn InstalledAppRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryInstalledAppRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteInstalledAppRepository::new(pool.clone())),
        }
    }

    pub fn create_plugin_repo(&self) -> Arc<dyn PluginRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryPluginRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqlitePluginRepository::new(pool.clone())),
        }
    }

    pub fn create_memo_repo(&self) -> Arc<dyn MemoRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryMemoRepository::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteMemoRepository::new(pool.clone())),
        }
    }

    /// 统一 Task 状态机持久化存储（Phase B1 扩展：SQLite 落库 / InMemory 默认）。
    pub fn create_task_store(&self) -> Arc<dyn crate::runtime::task_state::TaskStore> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryTaskStore::new()),
            #[cfg(feature = "sqlite")]
            BackendKind::Sqlite(pool) => Arc::new(SqliteTaskStore::new(pool.clone())),
        }
    }
}
