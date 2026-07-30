use async_trait::async_trait;
use crate::domain::entity::*;
use crate::core::error::AppError;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn create(&self, username: &str, password_hash: &str, role: &str) -> Result<User, AppError>;
    async fn list(&self) -> Result<Vec<User>, AppError>;
    async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<ServerNode>, AppError>;
    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError>;
    async fn create(&self, node: &ServerNode) -> Result<i64, AppError>;
    async fn list_all(&self) -> Result<Vec<ServerNode>, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

#[async_trait]
pub trait WebsiteRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError>;
    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError>;
    async fn create(&self, website: &Website) -> Result<i64, AppError>;
    async fn list_all(&self) -> Result<Vec<Website>, AppError>;
}

#[async_trait]
pub trait DockerRepository: Send + Sync {
    async fn list_containers(&self, node_id: i64) -> Result<Vec<DockerContainer>, AppError>;
    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError>;
    async fn start_container(&self, id: &str) -> Result<(), AppError>;
    async fn stop_container(&self, id: &str, timeout: u64) -> Result<(), AppError>;
    async fn restart_container(&self, id: &str, timeout: u64) -> Result<(), AppError>;
    async fn remove_container(&self, id: &str, force: bool) -> Result<(), AppError>;
    async fn get_container_logs(&self, id: &str, tail: usize) -> Result<String, AppError>;
    async fn get_container_stats(&self, id: &str) -> Result<serde_json::Value, AppError>;
    async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError>;
    async fn remove_image(&self, id: &str) -> Result<(), AppError>;
    async fn compose_deploy(&self, project_name: &str, compose_yaml: &str) -> Result<serde_json::Value, AppError>;
    async fn compose_up(&self, project_name: &str) -> Result<(), AppError>;
    async fn compose_down(&self, project_name: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait OperationLogRepository: Send + Sync {
    async fn create(&self, username: &str, action: &str, target: Option<&str>, ip: Option<&str>) -> Result<OperationLog, AppError>;
    async fn list(&self) -> Result<Vec<OperationLog>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError>;
    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError>;
}

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn create(&self, source: &str, level: &str, message: &str, metadata: Option<&str>) -> Result<LogEntry, AppError>;
    async fn list(&self) -> Result<Vec<LogEntry>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError>;
    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError>;
    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError>;
}

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError>;
    async fn find_by_resource_action(&self, resource: &str, action: &str) -> Result<Option<Permission>, AppError>;
    async fn create(&self, permission: &Permission) -> Result<i64, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Role>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError>;
    async fn create(&self, role: &Role) -> Result<i64, AppError>;
    async fn update(&self, role: &Role) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError>;
    async fn set_role_permissions(&self, role_id: i64, permission_ids: &[i64]) -> Result<(), AppError>;
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError>;
    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError>;
}

#[async_trait]
pub trait DatabaseRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<DatabaseInstance>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<DatabaseInstance>, AppError>;
    async fn find_by_type(&self, db_type: &str) -> Result<Vec<DatabaseInstance>, AppError>;
    async fn create(&self, instance: &DatabaseInstance) -> Result<i64, AppError>;
    async fn update(&self, instance: &DatabaseInstance) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait FirewallRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<FirewallRule>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<FirewallRule>, AppError>;
    async fn create(&self, rule: &FirewallRule) -> Result<i64, AppError>;
    async fn update(&self, rule: &FirewallRule) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn update_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError>;
    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError>;
}

#[async_trait]
pub trait WebServerRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<WebServerInstance>, AppError>;
    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError>;
    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError>;
    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError>;
}