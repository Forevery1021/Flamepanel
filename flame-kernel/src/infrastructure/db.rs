use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Mutex;

pub struct InMemoryUserRepository {
    users: Mutex<Vec<User>>,
    next_id: Mutex<i64>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.username == username).cloned())
    }

    async fn create(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError> {
        let mut users = self.users.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let user = User {
            id: *next_id,
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            role: role.to_string(),
            created_at: chrono::Utc::now(),
        };
        users.push(user.clone());
        *next_id += 1;
        Ok(user)
    }

    async fn update(&self, user: &User) -> Result<(), AppError> {
        let mut users = self.users.lock().unwrap();
        if let Some(existing) = users.iter_mut().find(|u| u.id == user.id) {
            existing.username = user.username.clone();
            existing.password_hash = user.password_hash.clone();
            existing.role = user.role.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("User not found".into()))
        }
    }

    async fn list(&self) -> Result<Vec<User>, AppError> {
        let users = self.users.lock().unwrap();
        Ok(users.clone())
    }

    async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == id) {
            user.password_hash = new_password_hash.to_string();
            Ok(())
        } else {
            Err(AppError::NotFound("User not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut users = self.users.lock().unwrap();
        users.retain(|u| u.id != id);
        Ok(())
    }
}

pub struct InMemoryNodeRepository {
    nodes: Mutex<Vec<ServerNode>>,
    next_id: Mutex<i64>,
}

impl InMemoryNodeRepository {
    pub fn new() -> Self {
        Self {
            nodes: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl NodeRepository for InMemoryNodeRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<ServerNode>, AppError> {
        let nodes = self.nodes.lock().unwrap();
        Ok(nodes.iter().find(|n| n.id == id).cloned())
    }

    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError> {
        let nodes = self.nodes.lock().unwrap();
        Ok(nodes.iter().find(|n| n.hostname == hostname).cloned())
    }

    async fn create(&self, node: &ServerNode) -> Result<i64, AppError> {
        let mut nodes = self.nodes.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let node_with_id = ServerNode {
            id: *next_id,
            name: node.name.clone(),
            hostname: node.hostname.clone(),
            ip_address: node.ip_address.clone(),
            status: node.status.clone(),
            created_at: node.created_at,
        };
        let id = node_with_id.id;
        nodes.push(node_with_id);
        *next_id += 1;
        Ok(id)
    }

    async fn update(&self, node: &ServerNode) -> Result<(), AppError> {
        let mut nodes = self.nodes.lock().unwrap();
        if let Some(existing) = nodes.iter_mut().find(|n| n.id == node.id) {
            existing.name = node.name.clone();
            existing.hostname = node.hostname.clone();
            existing.ip_address = node.ip_address.clone();
            existing.status = node.status.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("Node not found".into()))
        }
    }

    async fn list_all(&self) -> Result<Vec<ServerNode>, AppError> {
        let nodes = self.nodes.lock().unwrap();
        Ok(nodes.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.retain(|n| n.id != id);
        Ok(())
    }
}

pub struct InMemoryWebsiteRepository {
    websites: Mutex<Vec<Website>>,
    next_id: Mutex<i64>,
}

impl InMemoryWebsiteRepository {
    pub fn new() -> Self {
        Self {
            websites: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl WebsiteRepository for InMemoryWebsiteRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError> {
        let websites = self.websites.lock().unwrap();
        Ok(websites.iter().find(|w| w.id == id).cloned())
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError> {
        let websites = self.websites.lock().unwrap();
        Ok(websites.iter().find(|w| w.domain == domain).cloned())
    }

    async fn create(&self, website: &Website) -> Result<i64, AppError> {
        let mut websites = self.websites.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let ws = Website {
            id: *next_id,
            name: website.name.clone(),
            domain: website.domain.clone(),
            root_path: website.root_path.clone(),
            status: "active".to_string(),
            node_id: website.node_id,
            engine: website.engine.clone(),
            ssl_enabled: website.ssl_enabled,
            proxy_enabled: website.proxy_enabled,
            proxy_pass: website.proxy_pass.clone(),
            created_at: chrono::Utc::now(),
        };
        let id = ws.id;
        websites.push(ws);
        *next_id += 1;
        Ok(id)
    }

    async fn update(&self, website: &Website) -> Result<(), AppError> {
        let mut websites = self.websites.lock().unwrap();
        if let Some(existing) = websites.iter_mut().find(|w| w.id == website.id) {
            existing.name = website.name.clone();
            existing.domain = website.domain.clone();
            existing.root_path = website.root_path.clone();
            existing.engine = website.engine.clone();
            existing.ssl_enabled = website.ssl_enabled;
            existing.proxy_enabled = website.proxy_enabled;
            existing.proxy_pass = website.proxy_pass.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("Website not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut websites = self.websites.lock().unwrap();
        websites.retain(|w| w.id != id);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<Website>, AppError> {
        let websites = self.websites.lock().unwrap();
        Ok(websites.clone())
    }
}

pub struct InMemoryDockerRepository {
    containers: Mutex<Vec<DockerContainer>>,
}

impl InMemoryDockerRepository {
    pub fn new() -> Self {
        Self {
            containers: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl DockerRepository for InMemoryDockerRepository {
    async fn list_containers(&self, _node_id: i64) -> Result<Vec<DockerContainer>, AppError> {
        let containers = self.containers.lock().unwrap();
        Ok(containers.clone())
    }

    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError> {
        let containers = self.containers.lock().unwrap();
        Ok(containers.iter().find(|c| c.id == id).cloned())
    }

    async fn start_container(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn stop_container(&self, _id: &str, _timeout: u64) -> Result<(), AppError> {
        Ok(())
    }

    async fn restart_container(&self, _id: &str, _timeout: u64) -> Result<(), AppError> {
        Ok(())
    }

    async fn remove_container(&self, _id: &str, _force: bool) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_container_logs(&self, _id: &str, _tail: usize) -> Result<String, AppError> {
        Ok("logs not available in memory mode".into())
    }

    async fn get_container_stats(&self, _id: &str) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({"mode": "memory"}))
    }

    async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }

    async fn remove_image(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({
            "mode": "memory",
            "project_name": project_name,
            "compose_yaml": compose_yaml,
            "status": "deployed"
        }))
    }

    async fn compose_up(&self, _project_name: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn compose_down(&self, _project_name: &str) -> Result<(), AppError> {
        Ok(())
    }

    // ── 容器高级操作（内存降级：返回空/成功，避免破坏单测环境） ──
    async fn inspect_container(&self, _id: &str) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory" }))
    }

    async fn rename_container(&self, _id: &str, _new_name: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn pause_container(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn unpause_container(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn kill_container(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn prune_containers(&self) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "containers_deleted": null, "space_reclaimed": 0 }))
    }

    async fn list_networks(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }

    async fn create_network(
        &self,
        _name: &str,
        _driver: &str,
        _subnet: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "id": "mem-net", "name": _name }))
    }

    async fn remove_network(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn connect_network(&self, _network_id: &str, _container_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn disconnect_network(
        &self,
        _network_id: &str,
        _container_id: &str,
        _force: bool,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn prune_networks(&self) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "networks_deleted": null }))
    }

    async fn list_volumes(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }

    async fn create_volume(&self, _name: &str, _driver: &str) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "name": _name }))
    }

    async fn remove_volume(&self, _name: &str, _force: bool) -> Result<(), AppError> {
        Ok(())
    }

    async fn prune_volumes(&self) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "volumes_deleted": null }))
    }

    async fn pull_image(&self, _image: &str) -> Result<String, AppError> {
        Ok(format!("Image {} pulled (memory mode)", _image))
    }

    async fn tag_image(&self, _image_id: &str, _repo: &str, _tag: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn prune_images(&self) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "images_deleted": null, "space_reclaimed": 0 }))
    }

    async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }
}

pub struct InMemoryPermissionRepository {
    permissions: Mutex<Vec<Permission>>,
    next_id: Mutex<i64>,
}

impl InMemoryPermissionRepository {
    pub fn new() -> Self {
        let seed = crate::domain::entity::default_permissions();
        Self {
            permissions: Mutex::new(seed),
            next_id: Mutex::new(100),
        }
    }
}

#[async_trait]
impl PermissionRepository for InMemoryPermissionRepository {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError> {
        Ok(self.permissions.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError> {
        Ok(self
            .permissions
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn find_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<Permission>, AppError> {
        Ok(self
            .permissions
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.resource == resource && p.action == action)
            .cloned())
    }

    async fn create(&self, permission: &Permission) -> Result<i64, AppError> {
        let mut perms = self.permissions.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let mut p = permission.clone();
        p.id = *next_id;
        *next_id += 1;
        perms.push(p.clone());
        Ok(p.id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut perms = self.permissions.lock().unwrap();
        perms.retain(|p| p.id != id);
        Ok(())
    }
}

pub struct InMemoryRoleRepository {
    roles: Mutex<Vec<Role>>,
    role_perms: Mutex<std::collections::HashMap<i64, Vec<i64>>>,
    next_id: Mutex<i64>,
}

impl InMemoryRoleRepository {
    pub fn new() -> Self {
        let seed = crate::domain::entity::default_roles();
        let mut rp = std::collections::HashMap::new();
        for role in &seed {
            let pids: Vec<i64> = crate::domain::entity::role_permissions(&role.name)
                .into_iter()
                .collect();
            rp.insert(role.id, pids);
        }
        Self {
            roles: Mutex::new(seed),
            role_perms: Mutex::new(rp),
            next_id: Mutex::new(100),
        }
    }
}

#[async_trait]
impl RoleRepository for InMemoryRoleRepository {
    async fn list_all(&self) -> Result<Vec<Role>, AppError> {
        Ok(self.roles.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.name == name)
            .cloned())
    }

    async fn create(&self, role: &Role) -> Result<i64, AppError> {
        let mut roles = self.roles.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let mut r = role.clone();
        r.id = *next_id;
        *next_id += 1;
        roles.push(r.clone());
        Ok(r.id)
    }

    async fn update(&self, role: &Role) -> Result<(), AppError> {
        let mut roles = self.roles.lock().unwrap();
        if let Some(existing) = roles.iter_mut().find(|r| r.id == role.id) {
            existing.name = role.name.clone();
            existing.description = role.description.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.roles.lock().unwrap().retain(|r| r.id != id);
        self.role_perms.lock().unwrap().remove(&id);
        Ok(())
    }

    async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        Ok(self
            .role_perms
            .lock()
            .unwrap()
            .get(&role_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError> {
        self.role_perms
            .lock()
            .unwrap()
            .insert(role_id, permission_ids.to_vec());
        Ok(())
    }
}

pub struct InMemoryOperationLogRepository {
    logs: Mutex<Vec<OperationLog>>,
    next_id: Mutex<i64>,
}

impl InMemoryOperationLogRepository {
    pub fn new() -> Self {
        Self {
            logs: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl OperationLogRepository for InMemoryOperationLogRepository {
    async fn create(
        &self,
        username: &str,
        action: &str,
        target: Option<&str>,
        ip: Option<&str>,
    ) -> Result<OperationLog, AppError> {
        let mut logs = self.logs.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let log = OperationLog {
            id,
            username: username.into(),
            action: action.into(),
            target: target.map(|s| s.into()),
            ip: ip.map(|s| s.into()),
            created_at: Utc::now(),
        };
        logs.push(log.clone());
        Ok(log)
    }

    async fn list(&self) -> Result<Vec<OperationLog>, AppError> {
        Ok(self.logs.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        Ok(self
            .logs
            .lock()
            .unwrap()
            .iter()
            .find(|l| l.id == id)
            .cloned())
    }

    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        Ok(self
            .logs
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.username == username)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut logs = self.logs.lock().unwrap();
        logs.retain(|l| l.id != id);
        Ok(())
    }
}

pub struct InMemoryWebServerRepository {
    instances: Mutex<Vec<WebServerInstance>>,
    next_id: Mutex<i64>,
}

impl InMemoryWebServerRepository {
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl WebServerRepository for InMemoryWebServerRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<WebServerInstance>, AppError> {
        let instances = self.instances.lock().unwrap();
        Ok(instances.iter().find(|i| i.id == id).cloned())
    }

    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = self.instances.lock().unwrap();
        Ok(instances
            .iter()
            .filter(|i| i.engine == engine)
            .cloned()
            .collect())
    }

    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        let mut instances = self.instances.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let new = WebServerInstance {
            id: *next_id,
            engine: instance.engine.clone(),
            version: instance.version.clone(),
            status: "stopped".to_string(),
            config_path: instance.config_path.clone(),
            binary_path: instance.binary_path.clone(),
            port: instance.port,
            created_at: chrono::Utc::now(),
        };
        let id = new.id;
        instances.push(new);
        *next_id += 1;
        Ok(id)
    }

    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(existing) = instances.iter_mut().find(|i| i.id == instance.id) {
            existing.status = instance.status.clone();
            existing.config_path = instance.config_path.clone();
            existing.binary_path = instance.binary_path.clone();
            existing.port = instance.port;
            existing.version = instance.version.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.instances.lock().unwrap().retain(|i| i.id != id);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError> {
        Ok(self.instances.lock().unwrap().clone())
    }
}

pub struct InMemoryLogRepository {
    logs: Mutex<Vec<LogEntry>>,
    next_id: Mutex<i64>,
}

impl InMemoryLogRepository {
    pub fn new() -> Self {
        Self {
            logs: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl LogRepository for InMemoryLogRepository {
    async fn create(
        &self,
        source: &str,
        level: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<LogEntry, AppError> {
        let mut logs = self.logs.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let entry = LogEntry {
            id,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            metadata: metadata.map(|s| s.into()),
            created_at: Utc::now(),
        };
        logs.push(entry.clone());
        Ok(entry)
    }

    async fn list(&self) -> Result<Vec<LogEntry>, AppError> {
        Ok(self.logs.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        Ok(self
            .logs
            .lock()
            .unwrap()
            .iter()
            .find(|l| l.id == id)
            .cloned())
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        Ok(self
            .logs
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.source == source)
            .cloned()
            .collect())
    }

    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        Ok(self
            .logs
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.level == level)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut logs = self.logs.lock().unwrap();
        logs.retain(|l| l.id != id);
        Ok(())
    }
}

pub struct InMemoryDatabaseRepository {
    instances: Mutex<Vec<DatabaseInstance>>,
    next_id: Mutex<i64>,
}

impl InMemoryDatabaseRepository {
    pub fn new() -> Self {
        Self {
            instances: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl DatabaseRepository for InMemoryDatabaseRepository {
    async fn list_all(&self) -> Result<Vec<DatabaseInstance>, AppError> {
        Ok(self.instances.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<DatabaseInstance>, AppError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.name == name)
            .cloned())
    }

    async fn find_by_type(&self, db_type: &str) -> Result<Vec<DatabaseInstance>, AppError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.db_type == db_type)
            .cloned()
            .collect())
    }

    async fn create(&self, instance: &DatabaseInstance) -> Result<i64, AppError> {
        let mut instances = self.instances.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let new = DatabaseInstance {
            id,
            db_type: instance.db_type.clone(),
            name: instance.name.clone(),
            version: instance.version.clone(),
            port: instance.port,
            status: instance.status.clone(),
            install_path: instance.install_path.clone(),
            data_dir: instance.data_dir.clone(),
            config_file: instance.config_file.clone(),
            root_user: instance.root_user.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        instances.push(new);
        Ok(id)
    }

    async fn update(&self, instance: &DatabaseInstance) -> Result<(), AppError> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(existing) = instances.iter_mut().find(|i| i.id == instance.id) {
            existing.port = instance.port;
            existing.status = instance.status.clone();
            existing.version = instance.version.clone();
            existing.install_path = instance.install_path.clone();
            existing.data_dir = instance.data_dir.clone();
            existing.config_file = instance.config_file.clone();
            existing.root_user = instance.root_user.clone();
            existing.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.instances.lock().unwrap().retain(|i| i.id != id);
        Ok(())
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(existing) = instances.iter_mut().find(|i| i.id == id) {
            existing.status = status.to_string();
            existing.updated_at = chrono::Utc::now();
        }
        Ok(())
    }
}

pub struct InMemorySettingsRepository {
    settings: Mutex<Vec<PanelSetting>>,
}

impl InMemorySettingsRepository {
    pub fn new() -> Self {
        Self {
            settings: Mutex::new(crate::domain::entity::default_settings()),
        }
    }
}

#[async_trait]
impl SettingsRepository for InMemorySettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let settings = self.settings.lock().unwrap();
        Ok(settings
            .iter()
            .find(|s| s.key == key)
            .map(|s| s.value.clone()))
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut settings = self.settings.lock().unwrap();
        if let Some(existing) = settings.iter_mut().find(|s| s.key == key) {
            existing.value = value.to_string();
            existing.updated_at = Utc::now();
        } else {
            settings.push(PanelSetting {
                key: key.to_string(),
                value: value.to_string(),
                description: String::new(),
                updated_at: Utc::now(),
            });
        }
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        Ok(self.settings.lock().unwrap().clone())
    }

    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let settings = self.settings.lock().unwrap();
        Ok(settings
            .iter()
            .map(|s| (s.key.clone(), s.value.clone()))
            .collect())
    }
}

pub struct InMemoryFirewallRepository {
    rules: Mutex<Vec<FirewallRule>>,
    next_id: Mutex<i64>,
}

impl InMemoryFirewallRepository {
    pub fn new() -> Self {
        Self {
            rules: Mutex::new(default_firewall_rules()),
            next_id: Mutex::new(100),
        }
    }
}

#[async_trait]
impl FirewallRepository for InMemoryFirewallRepository {
    async fn list_all(&self) -> Result<Vec<FirewallRule>, AppError> {
        Ok(self.rules.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<FirewallRule>, AppError> {
        Ok(self
            .rules
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn create(&self, rule: &FirewallRule) -> Result<i64, AppError> {
        let mut rules = self.rules.lock().unwrap();
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;
        let mut r = rule.clone();
        r.id = id;
        rules.push(r);
        Ok(id)
    }

    async fn update(&self, rule: &FirewallRule) -> Result<(), AppError> {
        let mut rules = self.rules.lock().unwrap();
        if let Some(existing) = rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("Firewall rule not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut rules = self.rules.lock().unwrap();
        rules.retain(|r| r.id != id);
        Ok(())
    }

    async fn update_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError> {
        let mut rules = self.rules.lock().unwrap();
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = enabled;
            rule.updated_at = Utc::now();
            Ok(())
        } else {
            Err(AppError::NotFound("Firewall rule not found".into()))
        }
    }

    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError> {
        let mut rules = self.rules.lock().unwrap();
        let mut priority = 10i32;
        // Update priority for specified ids in order
        for id in ids {
            if let Some(rule) = rules.iter_mut().find(|r| r.id == *id) {
                rule.priority = priority;
                priority += 10;
            }
        }
        // Ensure rules not in the list keep their priorities
        Ok(())
    }
}

// ─── 定时任务 InMemory 仓储 ────────────────────────────────────────────────

pub struct InMemoryScheduledTaskRepository {
    tasks: Mutex<Vec<ScheduledTask>>,
    next_id: Mutex<i64>,
}

impl Default for InMemoryScheduledTaskRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryScheduledTaskRepository {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            next_id: Mutex::new(100),
        }
    }
}

#[async_trait]
impl ScheduledTaskRepository for InMemoryScheduledTaskRepository {
    async fn list_all(&self) -> Result<Vec<ScheduledTask>, AppError> {
        Ok(self.tasks.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ScheduledTask>, AppError> {
        Ok(self
            .tasks
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned())
    }

    async fn create(&self, task: &ScheduledTask) -> Result<i64, AppError> {
        let mut tasks = self.tasks.lock().unwrap();
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;
        let mut t = task.clone();
        t.id = id;
        tasks.push(t);
        Ok(id)
    }

    async fn update(&self, task: &ScheduledTask) -> Result<(), AppError> {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(existing) = tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("Scheduled task not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|t| t.id != id);
        Ok(())
    }
}
// ─── 应用商店 InMemory 仓储 ──────────────────────────────────────────────────

pub struct InMemoryAppPackageRepository {
    packages: std::sync::Arc<Mutex<Vec<AppPackage>>>,
}

impl InMemoryAppPackageRepository {
    pub fn new() -> Self {
        Self {
            packages: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl AppPackageRepository for InMemoryAppPackageRepository {
    async fn list_all(&self) -> Result<Vec<AppPackage>, AppError> {
        Ok(self.packages.lock().unwrap().clone())
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<AppPackage>, AppError> {
        Ok(self
            .packages
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.key == key)
            .cloned())
    }

    async fn create(&self, pkg: &AppPackage) -> Result<i64, AppError> {
        let mut packages = self.packages.lock().unwrap();
        if packages.iter().any(|p| p.key == pkg.key) {
            return Err(AppError::BadRequest(format!("应用包已存在: {}", pkg.key)));
        }
        let mut pkg = pkg.clone();
        pkg.id = (packages.len() as i64) + 1;
        let id = pkg.id;
        packages.push(pkg);
        Ok(id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.packages.lock().unwrap().retain(|p| p.id != id);
        Ok(())
    }
}

pub struct InMemoryInstalledAppRepository {
    apps: std::sync::Arc<Mutex<Vec<InstalledApp>>>,
}

impl InMemoryInstalledAppRepository {
    pub fn new() -> Self {
        Self {
            apps: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl InstalledAppRepository for InMemoryInstalledAppRepository {
    async fn list_all(&self) -> Result<Vec<InstalledApp>, AppError> {
        Ok(self.apps.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError> {
        Ok(self
            .apps
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id == id)
            .cloned())
    }

    async fn create(&self, app: &InstalledApp) -> Result<i64, AppError> {
        let mut apps = self.apps.lock().unwrap();
        let mut app = app.clone();
        app.id = (apps.len() as i64) + 1;
        let id = app.id;
        apps.push(app);
        Ok(id)
    }

    async fn update(&self, app: &InstalledApp) -> Result<(), AppError> {
        let mut apps = self.apps.lock().unwrap();
        if let Some(existing) = apps.iter_mut().find(|a| a.id == app.id) {
            *existing = app.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.apps.lock().unwrap().retain(|a| a.id != id);
        Ok(())
    }
}

pub struct InMemoryPluginRepository {
    plugins: std::sync::Arc<Mutex<Vec<Plugin>>>,
}

impl InMemoryPluginRepository {
    pub fn new() -> Self {
        Self {
            plugins: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl PluginRepository for InMemoryPluginRepository {
    async fn save(&self, plugin: &Plugin) -> Result<(), AppError> {
        let mut plugins = self.plugins.lock().unwrap();
        if let Some(existing) = plugins.iter_mut().find(|p| p.id == plugin.id) {
            *existing = plugin.clone();
        } else {
            plugins.push(plugin.clone());
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Plugin>, AppError> {
        Ok(self.plugins.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Plugin>, AppError> {
        Ok(self
            .plugins
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.plugins.lock().unwrap().retain(|p| p.id != id);
        Ok(())
    }
}

// ── Default implementations (delegating to new()) ─────────
impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryNodeRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryWebsiteRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryDockerRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryPermissionRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryRoleRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryOperationLogRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryLogRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryWebServerRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemorySettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryDatabaseRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryFirewallRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryAppPackageRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryInstalledAppRepository {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for InMemoryPluginRepository {
    fn default() -> Self {
        Self::new()
    }
}
