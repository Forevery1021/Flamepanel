use std::sync::Arc;
use sqlx::SqlitePool;
use bollard::Docker;
use crate::domain::repository::*;
use crate::infrastructure::db::*;
use crate::infrastructure::sqlite::*;
use crate::infrastructure::docker::BollardDockerRepository;
use crate::core::error::AppError;

pub enum BackendKind {
    InMemory,
    Sqlite(SqlitePool),
}

pub struct RepoFactory {
    kind: BackendKind,
    docker: Option<Docker>,
}

impl RepoFactory {
    pub fn new_in_memory() -> Self {
        Self { kind: BackendKind::InMemory, docker: None }
    }

    pub async fn new_sqlite(database_url: &str) -> Result<Self, AppError> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to database: {}", e)))?;
        run_migrations(&pool).await?;
        Ok(Self { kind: BackendKind::Sqlite(pool), docker: None })
    }

    pub fn with_docker_connection(mut self, docker: Docker) -> Self {
        self.docker = Some(docker);
        self
    }

    pub fn connect_docker(mut self) -> Self {
        if let Ok(docker) = Docker::connect_with_local_defaults() {
            self.docker = Some(docker);
        }
        self
    }

    pub fn create_user_repo(&self) -> Arc<dyn UserRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryUserRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteUserRepository::new(pool.clone())),
        }
    }

    pub fn create_node_repo(&self) -> Arc<dyn NodeRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryNodeRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteNodeRepository::new(pool.clone())),
        }
    }

    pub fn create_website_repo(&self) -> Arc<dyn WebsiteRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWebsiteRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteWebsiteRepository::new(pool.clone())),
        }
    }

    pub fn create_docker_repo(&self) -> Arc<dyn DockerRepository> {
        if let Some(docker) = &self.docker {
            match BollardDockerRepository::new_with_connection(docker.clone()) {
                Ok(repo) => return Arc::new(repo),
                Err(_) => { /* fall through to in-memory */ }
            }
        }
        Arc::new(InMemoryDockerRepository::new())
    }

    pub fn create_permission_repo(&self) -> Arc<dyn PermissionRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryPermissionRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqlitePermissionRepository::new(pool.clone())),
        }
    }

    pub fn create_role_repo(&self) -> Arc<dyn RoleRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryRoleRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteRoleRepository::new(pool.clone())),
        }
    }

    pub fn create_operation_log_repo(&self) -> Arc<dyn OperationLogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryOperationLogRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteOperationLogRepository::new(pool.clone())),
        }
    }

    pub fn create_log_repo(&self) -> Arc<dyn LogRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryLogRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteLogRepository::new(pool.clone())),
        }
    }

    pub fn create_settings_repo(&self) -> Arc<dyn SettingsRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemorySettingsRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteSettingsRepository::new(pool.clone())),
        }
    }

    pub fn create_web_server_repo(&self) -> Arc<dyn WebServerRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryWebServerRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteWebServerRepository::new(pool.clone())),
        }
    }

    pub fn create_database_repo(&self) -> Arc<dyn DatabaseRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryDatabaseRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteDatabaseRepository::new(pool.clone())),
        }
    }

    pub fn create_firewall_repo(&self) -> Arc<dyn FirewallRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryFirewallRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteFirewallRepository::new(pool.clone())),
        }
    }

    pub fn create_app_package_repo(&self) -> Arc<dyn AppPackageRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryAppPackageRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteAppPackageRepository::new(pool.clone())),
        }
    }

    pub fn create_installed_app_repo(&self) -> Arc<dyn InstalledAppRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryInstalledAppRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqliteInstalledAppRepository::new(pool.clone())),
        }
    }

    pub fn create_plugin_repo(&self) -> Arc<dyn PluginRepository> {
        match &self.kind {
            BackendKind::InMemory => Arc::new(InMemoryPluginRepository::new()),
            BackendKind::Sqlite(pool) => Arc::new(SqlitePluginRepository::new(pool.clone())),
        }
    }
}