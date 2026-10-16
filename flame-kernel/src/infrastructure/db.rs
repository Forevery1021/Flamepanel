use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::utils::sync::lock_or_recover;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

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
        let users = lock_or_recover(&self.users);
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let users = lock_or_recover(&self.users);
        Ok(users.iter().find(|u| u.username == username).cloned())
    }

    async fn create(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError> {
        let mut users = lock_or_recover(&self.users);
        let mut next_id = lock_or_recover(&self.next_id);
        let user = User {
            id: *next_id,
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            role: role.to_string(),
            created_at: chrono::Utc::now(),
            must_change_password: false,
        };
        users.push(user.clone());
        *next_id += 1;
        Ok(user)
    }

    async fn update(&self, user: &User) -> Result<(), AppError> {
        let mut users = lock_or_recover(&self.users);
        if let Some(existing) = users.iter_mut().find(|u| u.id == user.id) {
            existing.username = user.username.clone();
            existing.password_hash = user.password_hash.clone();
            existing.role = user.role.clone();
            existing.must_change_password = user.must_change_password;
            Ok(())
        } else {
            Err(AppError::NotFound("User not found".into()))
        }
    }

    async fn list(&self) -> Result<Vec<User>, AppError> {
        let users = lock_or_recover(&self.users);
        Ok(users.clone())
    }

    async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError> {
        let mut users = lock_or_recover(&self.users);
        if let Some(user) = users.iter_mut().find(|u| u.id == id) {
            user.password_hash = new_password_hash.to_string();
            user.must_change_password = false;
            Ok(())
        } else {
            Err(AppError::NotFound("User not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut users = lock_or_recover(&self.users);
        users.retain(|u| u.id != id);
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let users = lock_or_recover(&self.users);
        let mut sorted: Vec<User> = users.clone();
        sorted.sort_by_key(|x| std::cmp::Reverse(x.id));
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.users).len() as i64)
    }

    async fn get_password_changed_at(&self, id: i64) -> Result<Option<i64>, AppError> {
        // InMemory 后端不做持久化，保持与旧行为一致（返回 None）
        let _ = id;
        Ok(None)
    }

    async fn update_password_changed_at(&self, _id: i64, _ts: i64) -> Result<(), AppError> {
        // InMemory 后端不做持久化
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
        let nodes = lock_or_recover(&self.nodes);
        Ok(nodes.iter().find(|n| n.id == id).cloned())
    }

    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError> {
        let nodes = lock_or_recover(&self.nodes);
        Ok(nodes.iter().find(|n| n.hostname == hostname).cloned())
    }

    async fn create(&self, node: &ServerNode) -> Result<i64, AppError> {
        let mut nodes = lock_or_recover(&self.nodes);
        let mut next_id = lock_or_recover(&self.next_id);
        let node_with_id = ServerNode {
            id: *next_id,
            name: node.name.clone(),
            hostname: node.hostname.clone(),
            ip_address: node.ip_address.clone(),
            status: node.status.clone(),
            created_at: node.created_at,
            last_heartbeat_at: None,
            metrics_json: None,
            auth_token: node.auth_token.clone(),
            agent_port: node.agent_port,
        };
        let id = node_with_id.id;
        nodes.push(node_with_id);
        *next_id += 1;
        Ok(id)
    }

    async fn update(&self, node: &ServerNode) -> Result<(), AppError> {
        let mut nodes = lock_or_recover(&self.nodes);
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
        let nodes = lock_or_recover(&self.nodes);
        Ok(nodes.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut nodes = lock_or_recover(&self.nodes);
        nodes.retain(|n| n.id != id);
        Ok(())
    }

    async fn update_heartbeat(&self, id: i64, metrics_json: &str) -> Result<(), AppError> {
        let mut nodes = lock_or_recover(&self.nodes);
        if let Some(existing) = nodes.iter_mut().find(|n| n.id == id) {
            existing.last_heartbeat_at = Some(chrono::Utc::now());
            existing.metrics_json = Some(metrics_json.to_string());
            Ok(())
        } else {
            Err(AppError::NotFound("Node not found".into()))
        }
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ServerNode>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let nodes = lock_or_recover(&self.nodes);
        let mut sorted: Vec<ServerNode> = nodes.clone();
        sorted.sort_by_key(|x| std::cmp::Reverse(x.id));
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.nodes).len() as i64)
    }

    /// 离线扫描条件化：筛选心跳早于阈值的节点。
    async fn list_stale_heartbeats(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ServerNode>, AppError> {
        let nodes = lock_or_recover(&self.nodes);
        Ok(nodes
            .iter()
            .filter(|n| match n.last_heartbeat_at {
                Some(t) => t < before,
                None => true,
            })
            .cloned()
            .collect())
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
        let websites = lock_or_recover(&self.websites);
        Ok(websites.iter().find(|w| w.id == id).cloned())
    }

    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError> {
        let websites = lock_or_recover(&self.websites);
        Ok(websites.iter().find(|w| w.domain == domain).cloned())
    }

    async fn create(&self, website: &Website) -> Result<i64, AppError> {
        let mut websites = lock_or_recover(&self.websites);
        let mut next_id = lock_or_recover(&self.next_id);
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
            site_type: website.site_type.clone(),
            runtime_id: website.runtime_id,
            project_id: website.project_id,
            rewrite_preset: website.rewrite_preset.clone(),
            created_at: chrono::Utc::now(),
            resource_version: 0,
        };
        let id = ws.id;
        websites.push(ws);
        *next_id += 1;
        Ok(id)
    }

    async fn update(&self, website: &Website) -> Result<(), AppError> {
        let mut websites = lock_or_recover(&self.websites);
        if let Some(existing) = websites.iter_mut().find(|w| w.id == website.id) {
            // 乐观并发控制（OCC）：版本不匹配则冲突
            if existing.resource_version != website.resource_version {
                return Err(AppError::Conflict(format!(
                    "Website {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                    website.id, website.resource_version
                )));
            }
            existing.name = website.name.clone();
            existing.domain = website.domain.clone();
            existing.root_path = website.root_path.clone();
            existing.status = website.status.clone();
            existing.engine = website.engine.clone();
            existing.ssl_enabled = website.ssl_enabled;
            existing.proxy_enabled = website.proxy_enabled;
            existing.proxy_pass = website.proxy_pass.clone();
            existing.site_type = website.site_type.clone();
            existing.runtime_id = website.runtime_id;
            existing.project_id = website.project_id;
            existing.rewrite_preset = website.rewrite_preset.clone();
            existing.resource_version += 1;
            Ok(())
        } else {
            Err(AppError::NotFound("Website not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut websites = lock_or_recover(&self.websites);
        websites.retain(|w| w.id != id);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<Website>, AppError> {
        let websites = lock_or_recover(&self.websites);
        Ok(websites.clone())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Website>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let websites = lock_or_recover(&self.websites);
        let mut sorted: Vec<Website> = websites.clone();
        sorted.sort_by_key(|x| std::cmp::Reverse(x.id));
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.websites).len() as i64)
    }
}

/// F-16: InMemory Docker 五仓储共享状态（容器/网络/卷/镜像/Compose 互通）。
/// 原先五个子仓储各自 new 独立实例，create_container 后 list 不可见。
#[derive(Default)]
pub struct InMemoryDockerState {
    pub containers: Mutex<Vec<DockerContainer>>,
    pub networks: Mutex<Vec<serde_json::Value>>,
    pub volumes: Mutex<Vec<serde_json::Value>>,
    pub images: Mutex<Vec<serde_json::Value>>,
    pub compose_projects: Mutex<Vec<serde_json::Value>>,
}

pub struct InMemoryDockerRepository {
    state: Arc<InMemoryDockerState>,
}

impl InMemoryDockerRepository {
    pub fn new() -> Self {
        Self {
            state: Arc::new(InMemoryDockerState::default()),
        }
    }

    /// F-16: 使用共享状态创建仓储实例（所有子仓储共用同一 state）。
    pub fn new_shared(state: Arc<InMemoryDockerState>) -> Self {
        Self { state }
    }
}

#[async_trait]
#[async_trait]
impl ContainerRepository for InMemoryDockerRepository {
    async fn list_containers(&self, _node_id: i64) -> Result<Vec<DockerContainer>, AppError> {
        let containers = lock_or_recover(&self.state.containers);
        Ok(containers.clone())
    }

    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError> {
        let containers = lock_or_recover(&self.state.containers);
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

    async fn inspect_container(&self, id: &str) -> Result<serde_json::Value, AppError> {
        // 最小的 Engine inspect 形状：让「容器 → Compose」等依赖 inspect 的流程可在内存后端走通
        Ok(serde_json::json!({
            "mode": "memory",
            "Id": id,
            "Name": format!("/{id}"),
            "Config": { "Image": "memory/placeholder:latest" },
            "HostConfig": { "RestartPolicy": { "Name": "unless-stopped" } }
        }))
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
        Ok(
            serde_json::json!({ "mode": "memory", "containers_deleted": null, "space_reclaimed": 0 }),
        )
    }
}

#[async_trait]
impl NetworkRepository for InMemoryDockerRepository {
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

    async fn connect_network(
        &self,
        _network_id: &str,
        _container_id: &str,
    ) -> Result<(), AppError> {
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
}

#[async_trait]
impl VolumeRepository for InMemoryDockerRepository {
    async fn list_volumes(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }

    async fn create_volume(
        &self,
        _name: &str,
        _driver: &str,
    ) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "name": _name }))
    }

    async fn remove_volume(&self, _name: &str, _force: bool) -> Result<(), AppError> {
        Ok(())
    }

    async fn prune_volumes(&self) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({ "mode": "memory", "volumes_deleted": null }))
    }
}

#[async_trait]
impl ImageRepository for InMemoryDockerRepository {
    async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }

    async fn remove_image(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
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
}

#[async_trait]
impl ComposeRepository for InMemoryDockerRepository {
    async fn run_compose(
        &self,
        _args: Vec<String>,
    ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
        Ok(crate::application::execution_mode::CommandOutput::default())
    }

    async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError> {
        // F-05: 与 Bollard 实现保持一致，拒绝路径穿越等危险输入。
        // 直接调用 docker.rs 中的 sanitize 函数（同 crate 内）。
        crate::infrastructure::naming::sanitize_project_name(project_name)?;
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

    async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError> {
        Ok(vec![])
    }
}

#[async_trait]
impl DockerRepository for InMemoryDockerRepository {}

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
        Ok(lock_or_recover(&self.permissions).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError> {
        Ok(lock_or_recover(&self.permissions)
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn find_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<Permission>, AppError> {
        Ok(lock_or_recover(&self.permissions)
            .iter()
            .find(|p| p.resource == resource && p.action == action)
            .cloned())
    }

    async fn create(&self, permission: &Permission) -> Result<i64, AppError> {
        let mut perms = lock_or_recover(&self.permissions);
        let mut next_id = lock_or_recover(&self.next_id);
        let mut p = permission.clone();
        p.id = *next_id;
        *next_id += 1;
        perms.push(p.clone());
        Ok(p.id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut perms = lock_or_recover(&self.permissions);
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
        Ok(lock_or_recover(&self.roles).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Role>, AppError> {
        Ok(lock_or_recover(&self.roles)
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AppError> {
        Ok(lock_or_recover(&self.roles)
            .iter()
            .find(|r| r.name == name)
            .cloned())
    }

    async fn create(&self, role: &Role) -> Result<i64, AppError> {
        let mut roles = lock_or_recover(&self.roles);
        let mut next_id = lock_or_recover(&self.next_id);
        let mut r = role.clone();
        r.id = *next_id;
        *next_id += 1;
        roles.push(r.clone());
        Ok(r.id)
    }

    async fn update(&self, role: &Role) -> Result<(), AppError> {
        let mut roles = lock_or_recover(&self.roles);
        if let Some(existing) = roles.iter_mut().find(|r| r.id == role.id) {
            existing.name = role.name.clone();
            existing.description = role.description.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.roles).retain(|r| r.id != id);
        lock_or_recover(&self.role_perms).remove(&id);
        Ok(())
    }

    async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        Ok(lock_or_recover(&self.role_perms)
            .get(&role_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError> {
        lock_or_recover(&self.role_perms).insert(role_id, permission_ids.to_vec());
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
        let mut logs = lock_or_recover(&self.logs);
        let mut next_id = lock_or_recover(&self.next_id);
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
        Ok(lock_or_recover(&self.logs).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        Ok(lock_or_recover(&self.logs)
            .iter()
            .find(|l| l.id == id)
            .cloned())
    }

    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        Ok(lock_or_recover(&self.logs)
            .iter()
            .filter(|l| l.username == username)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut logs = lock_or_recover(&self.logs);
        logs.retain(|l| l.id != id);
        Ok(())
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        action_prefix: Option<&str>,
    ) -> Result<Vec<OperationLog>, AppError> {
        let all = lock_or_recover(&self.logs);
        let filtered: Vec<&OperationLog> = match action_prefix {
            Some(prefix) if !prefix.is_empty() => all
                .iter()
                .filter(|l| l.action.starts_with(prefix))
                .collect(),
            _ => all.iter().collect(),
        };
        let page: Vec<OperationLog> = filtered
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect();
        Ok(page)
    }

    async fn count(&self, action_prefix: Option<&str>) -> Result<i64, AppError> {
        let all = lock_or_recover(&self.logs);
        let n = match action_prefix {
            Some(prefix) if !prefix.is_empty() => {
                all.iter().filter(|l| l.action.starts_with(prefix)).count()
            }
            _ => all.len(),
        };
        Ok(n as i64)
    }

    /// F-12: 删除早于 N 天的操作日志
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let mut logs = lock_or_recover(&self.logs);
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let before = logs.len();
        logs.retain(|l| l.created_at >= cutoff);
        Ok((before - logs.len()) as u64)
    }
}

pub struct InMemoryOutboxRepository {
    events: Mutex<Vec<OutboxEvent>>,
    next_id: Mutex<i64>,
}

impl InMemoryOutboxRepository {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl Default for InMemoryOutboxRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutboxRepository for InMemoryOutboxRepository {
    async fn append(
        &self,
        event_type: &str,
        payload: &str,
        published: bool,
    ) -> Result<OutboxEvent, AppError> {
        let mut events = lock_or_recover(&self.events);
        let mut next_id = lock_or_recover(&self.next_id);
        let id = *next_id;
        *next_id += 1;
        let ev = OutboxEvent {
            id,
            event_type: event_type.into(),
            payload: payload.into(),
            published,
            created_at: Utc::now(),
        };
        events.push(ev.clone());
        Ok(ev)
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        event_type: Option<&str>,
    ) -> Result<Vec<OutboxEvent>, AppError> {
        let all = lock_or_recover(&self.events);
        let mut filtered: Vec<&OutboxEvent> = match event_type {
            Some(et) if !et.is_empty() => all.iter().filter(|e| e.event_type == et).collect(),
            _ => all.iter().collect(),
        };
        // 按 id 倒序
        filtered.sort_by_key(|e| std::cmp::Reverse(e.id));
        let page: Vec<OutboxEvent> = filtered
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect();
        Ok(page)
    }

    async fn count(&self, event_type: Option<&str>) -> Result<i64, AppError> {
        let all = lock_or_recover(&self.events);
        let n = match event_type {
            Some(et) if !et.is_empty() => all.iter().filter(|e| e.event_type == et).count(),
            _ => all.len(),
        };
        Ok(n as i64)
    }

    /// F-12: 删除早于 N 天的已发布 outbox 事件
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let mut events = lock_or_recover(&self.events);
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let before = events.len();
        events.retain(|e| !e.published || e.created_at >= cutoff);
        Ok((before - events.len()) as u64)
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
        let instances = lock_or_recover(&self.instances);
        Ok(instances.iter().find(|i| i.id == id).cloned())
    }

    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError> {
        let instances = lock_or_recover(&self.instances);
        Ok(instances
            .iter()
            .filter(|i| i.engine == engine)
            .cloned()
            .collect())
    }

    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        let mut instances = lock_or_recover(&self.instances);
        let mut next_id = lock_or_recover(&self.next_id);
        let new = WebServerInstance {
            id: *next_id,
            engine: instance.engine.clone(),
            version: instance.version.clone(),
            status: "stopped".to_string(),
            config_path: instance.config_path.clone(),
            binary_path: instance.binary_path.clone(),
            port: instance.port,
            created_at: chrono::Utc::now(),
            resource_version: 0,
        };
        let id = new.id;
        instances.push(new);
        *next_id += 1;
        Ok(id)
    }

    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        let mut instances = lock_or_recover(&self.instances);
        if let Some(existing) = instances.iter_mut().find(|i| i.id == instance.id) {
            // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
            if existing.resource_version != instance.resource_version {
                return Err(AppError::Conflict(format!(
                    "Web server {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                    instance.id, instance.resource_version
                )));
            }
            existing.status = instance.status.clone();
            existing.config_path = instance.config_path.clone();
            existing.binary_path = instance.binary_path.clone();
            existing.port = instance.port;
            existing.version = instance.version.clone();
            existing.engine = instance.engine.clone();
            existing.resource_version += 1;
        } else {
            return Err(AppError::NotFound(format!(
                "Web server {} not found",
                instance.id
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.instances).retain(|i| i.id != id);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError> {
        Ok(lock_or_recover(&self.instances).clone())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<WebServerInstance>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let instances = lock_or_recover(&self.instances);
        let mut sorted: Vec<WebServerInstance> = instances.clone();
        sorted.sort_by_key(|x| std::cmp::Reverse(x.id));
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.instances).len() as i64)
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
        let mut logs = lock_or_recover(&self.logs);
        let mut next_id = lock_or_recover(&self.next_id);
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
        Ok(lock_or_recover(&self.logs).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        Ok(lock_or_recover(&self.logs)
            .iter()
            .find(|l| l.id == id)
            .cloned())
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        Ok(lock_or_recover(&self.logs)
            .iter()
            .filter(|l| l.source == source)
            .cloned()
            .collect())
    }

    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        Ok(lock_or_recover(&self.logs)
            .iter()
            .filter(|l| l.level == level)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut logs = lock_or_recover(&self.logs);
        logs.retain(|l| l.id != id);
        Ok(())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<LogEntry>, AppError> {
        let all = lock_or_recover(&self.logs);
        let page: Vec<LogEntry> = all
            .iter()
            .rev()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect();
        Ok(page)
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.logs).len() as i64)
    }

    /// F-12: 删除早于 N 天的系统日志
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let mut logs = lock_or_recover(&self.logs);
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let before = logs.len();
        logs.retain(|l| l.created_at >= cutoff);
        Ok((before - logs.len()) as u64)
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
        Ok(lock_or_recover(&self.instances).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DatabaseInstance>, AppError> {
        Ok(lock_or_recover(&self.instances)
            .iter()
            .find(|i| i.id == id)
            .cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<DatabaseInstance>, AppError> {
        Ok(lock_or_recover(&self.instances)
            .iter()
            .find(|i| i.name == name)
            .cloned())
    }

    async fn find_by_type(&self, db_type: &str) -> Result<Vec<DatabaseInstance>, AppError> {
        Ok(lock_or_recover(&self.instances)
            .iter()
            .filter(|i| i.db_type == db_type)
            .cloned()
            .collect())
    }

    async fn create(&self, instance: &DatabaseInstance) -> Result<i64, AppError> {
        let mut instances = lock_or_recover(&self.instances);
        let mut next_id = lock_or_recover(&self.next_id);
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
            resource_version: 0,
        };
        instances.push(new);
        Ok(id)
    }

    async fn update(&self, instance: &DatabaseInstance) -> Result<(), AppError> {
        let mut instances = lock_or_recover(&self.instances);
        if let Some(existing) = instances.iter_mut().find(|i| i.id == instance.id) {
            // 乐观并发控制（OCC）：仅当 resource_version 匹配时才更新，并将版本号自增。
            if existing.resource_version != instance.resource_version {
                return Err(AppError::Conflict(format!(
                    "Database instance {} 已被其他会话修改，resource_version 冲突（期望 {}）",
                    instance.id, instance.resource_version
                )));
            }
            existing.port = instance.port;
            existing.status = instance.status.clone();
            existing.version = instance.version.clone();
            existing.install_path = instance.install_path.clone();
            existing.data_dir = instance.data_dir.clone();
            existing.config_file = instance.config_file.clone();
            existing.root_user = instance.root_user.clone();
            existing.updated_at = chrono::Utc::now();
            existing.resource_version += 1;
        } else {
            return Err(AppError::NotFound(format!(
                "Database instance {} not found",
                instance.id
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.instances).retain(|i| i.id != id);
        Ok(())
    }

    async fn update_status(&self, id: i64, status: &str) -> Result<(), AppError> {
        let mut instances = lock_or_recover(&self.instances);
        if let Some(existing) = instances.iter_mut().find(|i| i.id == id) {
            existing.status = status.to_string();
            existing.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_status_batch(&self, updates: &[(i64, String)]) -> Result<(), AppError> {
        // 单锁内批量写入：原子（要么全部应用要么全部不应用）
        let mut instances = lock_or_recover(&self.instances);
        let now = chrono::Utc::now();
        for (id, status) in updates {
            if let Some(existing) = instances.iter_mut().find(|i| i.id == *id) {
                existing.status = status.clone();
                existing.updated_at = now;
            }
        }
        Ok(())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<DatabaseInstance>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let instances = lock_or_recover(&self.instances);
        let mut sorted: Vec<DatabaseInstance> = instances.clone();
        sorted.sort_by_key(|x| std::cmp::Reverse(x.id));
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.instances).len() as i64)
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
        let settings = lock_or_recover(&self.settings);
        Ok(settings
            .iter()
            .find(|s| s.key == key)
            .map(|s| s.value.clone()))
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut settings = lock_or_recover(&self.settings);
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

    async fn set_many(&self, entries: &[(String, String)]) -> Result<(), AppError> {
        if entries.is_empty() {
            return Ok(());
        }
        // 单锁内批量写入（原子性：要么全部生效，要么维持原状）
        let mut settings = lock_or_recover(&self.settings);
        for (key, value) in entries {
            if let Some(existing) = settings.iter_mut().find(|s| s.key == *key) {
                existing.value = value.clone();
                existing.updated_at = Utc::now();
            } else {
                settings.push(PanelSetting {
                    key: key.clone(),
                    value: value.clone(),
                    description: String::new(),
                    updated_at: Utc::now(),
                });
            }
        }
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        Ok(lock_or_recover(&self.settings).clone())
    }

    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let settings = lock_or_recover(&self.settings);
        Ok(settings
            .iter()
            .map(|s| (s.key.clone(), s.value.clone()))
            .collect())
    }

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<PanelSetting>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let settings = lock_or_recover(&self.settings);
        let mut sorted: Vec<PanelSetting> = settings.clone();
        sorted.sort_by(|a, b| b.key.cmp(&a.key).reverse());
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.settings).len() as i64)
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
        Ok(lock_or_recover(&self.rules).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<FirewallRule>, AppError> {
        Ok(lock_or_recover(&self.rules)
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn create(&self, rule: &FirewallRule) -> Result<i64, AppError> {
        let mut rules = lock_or_recover(&self.rules);
        let mut next = lock_or_recover(&self.next_id);
        let id = *next;
        *next += 1;
        let mut r = rule.clone();
        r.id = id;
        rules.push(r);
        Ok(id)
    }

    async fn update(&self, rule: &FirewallRule) -> Result<(), AppError> {
        let mut rules = lock_or_recover(&self.rules);
        if let Some(existing) = rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("Firewall rule not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut rules = lock_or_recover(&self.rules);
        rules.retain(|r| r.id != id);
        Ok(())
    }

    async fn update_enabled(&self, id: i64, enabled: bool) -> Result<(), AppError> {
        let mut rules = lock_or_recover(&self.rules);
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = enabled;
            rule.updated_at = Utc::now();
            Ok(())
        } else {
            Err(AppError::NotFound("Firewall rule not found".into()))
        }
    }

    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError> {
        let mut rules = lock_or_recover(&self.rules);
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

    // ── 分页下沉（Stage1）──
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<FirewallRule>, AppError> {
        let limit = limit.clamp(1, 200) as usize;
        let rules = lock_or_recover(&self.rules);
        let mut sorted: Vec<FirewallRule> = rules.clone();
        sorted.sort_by_key(|x| std::cmp::Reverse(x.id));
        Ok(sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect())
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.rules).len() as i64)
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
        Ok(lock_or_recover(&self.tasks).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ScheduledTask>, AppError> {
        Ok(lock_or_recover(&self.tasks)
            .iter()
            .find(|t| t.id == id)
            .cloned())
    }

    async fn create(&self, task: &ScheduledTask) -> Result<i64, AppError> {
        let mut tasks = lock_or_recover(&self.tasks);
        let mut next = lock_or_recover(&self.next_id);
        let id = *next;
        *next += 1;
        let mut t = task.clone();
        t.id = id;
        tasks.push(t);
        Ok(id)
    }

    async fn update(&self, task: &ScheduledTask) -> Result<(), AppError> {
        let mut tasks = lock_or_recover(&self.tasks);
        if let Some(existing) = tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task.clone();
            Ok(())
        } else {
            Err(AppError::NotFound("Scheduled task not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut tasks = lock_or_recover(&self.tasks);
        tasks.retain(|t| t.id != id);
        Ok(())
    }

    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ScheduledTask>, AppError> {
        let all = lock_or_recover(&self.tasks);
        let page: Vec<ScheduledTask> = all
            .iter()
            .rev()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect();
        Ok(page)
    }

    async fn count(&self) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.tasks).len() as i64)
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
        Ok(lock_or_recover(&self.packages).clone())
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<AppPackage>, AppError> {
        Ok(lock_or_recover(&self.packages)
            .iter()
            .find(|p| p.key == key)
            .cloned())
    }

    async fn create(&self, pkg: &AppPackage) -> Result<i64, AppError> {
        let mut packages = lock_or_recover(&self.packages);
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
        lock_or_recover(&self.packages).retain(|p| p.id != id);
        Ok(())
    }

    async fn create_many(&self, pkgs: &[AppPackage]) -> Result<usize, AppError> {
        // 单锁内批量写入：原子（要么全部成功要么全部不写）
        let mut packages = lock_or_recover(&self.packages);
        let mut new_ids: Vec<AppPackage> = Vec::new();
        for pkg in pkgs {
            if packages.iter().any(|p| p.key == pkg.key) {
                continue; // 幂等：已存在则跳过
            }
            let mut p = pkg.clone();
            p.id = (packages.len() as i64 + new_ids.len() as i64) + 1;
            new_ids.push(p);
        }
        let count = new_ids.len();
        packages.extend(new_ids);
        Ok(count)
    }

    async fn update_source_path(&self, id: i64, source_path: &str) -> Result<(), AppError> {
        let mut packages = lock_or_recover(&self.packages);
        match packages.iter_mut().find(|p| p.id == id) {
            Some(p) => {
                p.source_path = Some(source_path.to_string());
                p.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("应用包不存在: id={}", id))),
        }
    }
}

/// 远程应用源内存仓储（M-A Issue 1.1，InMemory 后端 / 测试用）
pub struct InMemoryAppSourceRepository {
    sources: std::sync::Arc<Mutex<Vec<AppSource>>>,
}

impl InMemoryAppSourceRepository {
    pub fn new() -> Self {
        Self {
            sources: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryAppSourceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppSourceRepository for InMemoryAppSourceRepository {
    async fn list_all(&self) -> Result<Vec<AppSource>, AppError> {
        Ok(lock_or_recover(&self.sources).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<AppSource>, AppError> {
        Ok(lock_or_recover(&self.sources)
            .iter()
            .find(|s| s.id == id)
            .cloned())
    }

    async fn find_by_url(&self, url: &str) -> Result<Option<AppSource>, AppError> {
        Ok(lock_or_recover(&self.sources)
            .iter()
            .find(|s| s.url == url)
            .cloned())
    }

    async fn create(&self, source: &AppSource) -> Result<i64, AppError> {
        let mut sources = lock_or_recover(&self.sources);
        if sources.iter().any(|s| s.url == source.url) {
            return Err(AppError::BadRequest(format!(
                "应用源已存在: {}",
                source.url
            )));
        }
        let mut s = source.clone();
        s.id = (sources.len() as i64) + 1;
        let id = s.id;
        sources.push(s);
        Ok(id)
    }

    async fn update(&self, source: &AppSource) -> Result<(), AppError> {
        let mut sources = lock_or_recover(&self.sources);
        match sources.iter_mut().find(|s| s.id == source.id) {
            Some(s) => {
                s.name = source.name.clone();
                s.source_type = source.source_type.clone();
                s.url = source.url.clone();
                s.enabled = source.enabled;
                s.public_key = source.public_key.clone();
                s.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("应用源不存在: {}", source.id))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.sources).retain(|s| s.id != id);
        Ok(())
    }

    async fn touch_synced(&self, id: i64) -> Result<(), AppError> {
        let mut sources = lock_or_recover(&self.sources);
        match sources.iter_mut().find(|s| s.id == id) {
            Some(s) => {
                s.last_synced_at = Some(Utc::now());
                Ok(())
            }
            None => Err(AppError::NotFound(format!("应用源不存在: {}", id))),
        }
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
        Ok(lock_or_recover(&self.apps).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError> {
        Ok(lock_or_recover(&self.apps)
            .iter()
            .find(|a| a.id == id)
            .cloned())
    }

    async fn create(&self, app: &InstalledApp) -> Result<i64, AppError> {
        let mut apps = lock_or_recover(&self.apps);
        let mut app = app.clone();
        app.id = (apps.len() as i64) + 1;
        let id = app.id;
        apps.push(app);
        Ok(id)
    }

    async fn update(&self, app: &InstalledApp) -> Result<(), AppError> {
        let mut apps = lock_or_recover(&self.apps);
        if let Some(existing) = apps.iter_mut().find(|a| a.id == app.id) {
            *existing = app.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.apps).retain(|a| a.id != id);
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
        let mut plugins = lock_or_recover(&self.plugins);
        if let Some(existing) = plugins.iter_mut().find(|p| p.id == plugin.id) {
            *existing = plugin.clone();
        } else {
            plugins.push(plugin.clone());
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Plugin>, AppError> {
        Ok(lock_or_recover(&self.plugins).clone())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Plugin>, AppError> {
        Ok(lock_or_recover(&self.plugins)
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        lock_or_recover(&self.plugins).retain(|p| p.id != id);
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

pub struct InMemoryMemoRepository {
    memos: Mutex<Vec<Memo>>,
    next_id: Mutex<i64>,
}

impl Default for InMemoryMemoRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMemoRepository {
    pub fn new() -> Self {
        Self {
            memos: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl MemoRepository for InMemoryMemoRepository {
    async fn list(&self, kind: Option<&str>, done: Option<bool>) -> Result<Vec<Memo>, AppError> {
        let memos = lock_or_recover(&self.memos);
        let mut result: Vec<Memo> = memos
            .iter()
            .filter(|m| {
                let k_ok = kind.map(|k| !k.is_empty() && m.kind == k).unwrap_or(true);
                let d_ok = done.map(|d| m.done == d).unwrap_or(true);
                k_ok && d_ok
            })
            .cloned()
            .collect();
        result.sort_by_key(|m| std::cmp::Reverse(m.id));
        Ok(result)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Memo>, AppError> {
        let memos = lock_or_recover(&self.memos);
        Ok(memos.iter().find(|m| m.id == id).cloned())
    }

    async fn create(&self, content: &str, kind: &str) -> Result<i64, AppError> {
        let mut memos = lock_or_recover(&self.memos);
        let mut next_id = lock_or_recover(&self.next_id);
        let memo = Memo {
            id: *next_id,
            content: content.to_string(),
            kind: kind.to_string(),
            done: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        memos.push(memo);
        *next_id += 1;
        Ok(*next_id - 1)
    }

    async fn update(&self, memo: &Memo) -> Result<(), AppError> {
        let mut memos = lock_or_recover(&self.memos);
        if let Some(existing) = memos.iter_mut().find(|m| m.id == memo.id) {
            existing.content = memo.content.clone();
            existing.kind = memo.kind.clone();
            existing.done = memo.done;
            existing.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(AppError::NotFound("Memo not found".into()))
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let mut memos = lock_or_recover(&self.memos);
        let before = memos.len();
        memos.retain(|m| m.id != id);
        if memos.len() == before {
            return Err(AppError::NotFound("Memo not found".into()));
        }
        Ok(())
    }
}

// ─── 统一 Task 状态机持久化（Phase B1 扩展）────────────────────────────────

/// 进程内 TaskStore：仅内存，供默认/测试路径使用（不落盘）。
#[derive(Default, Clone)]
pub struct InMemoryTaskStore {
    records: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<u64, crate::runtime::task_state::TaskRecord>>,
    >,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl crate::runtime::task_state::TaskStore for InMemoryTaskStore {
    async fn insert(&self, record: &crate::runtime::task_state::TaskRecord) -> Result<(), String> {
        if record.id == 0 || record.id > i64::MAX as u64 {
            return Err("invalid task ID".into());
        }
        let mut records = self.records.lock().map_err(|e| e.to_string())?;
        match records.entry(record.id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(record.clone());
                Ok(())
            }
            std::collections::hash_map::Entry::Occupied(_) => Err("task ID already exists".into()),
        }
    }
    async fn update(&self, record: &crate::runtime::task_state::TaskRecord) -> Result<(), String> {
        let mut records = self.records.lock().map_err(|e| e.to_string())?;
        let existing = records
            .get_mut(&record.id)
            .ok_or_else(|| "update task: task not found".to_string())?;
        *existing = record.clone();
        Ok(())
    }
    async fn load_all(&self) -> Result<Vec<crate::runtime::task_state::TaskRecord>, String> {
        Ok(self
            .records
            .lock()
            .map_err(|e| e.to_string())?
            .values()
            .cloned()
            .collect())
    }
    async fn remove(&self, id: u64) -> Result<(), String> {
        self.records.lock().map_err(|e| e.to_string())?.remove(&id);
        Ok(())
    }
}

/// F-04: InMemory refresh token 黑名单仓储（用于测试/内存模式）。
pub struct InMemoryRevokedTokenRepository {
    revoked: Mutex<std::collections::HashMap<String, i64>>,
}

impl Default for InMemoryRevokedTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRevokedTokenRepository {
    pub fn new() -> Self {
        Self {
            revoked: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl RevokedTokenRepository for InMemoryRevokedTokenRepository {
    async fn insert(&self, token_hash: &str, expires_at: i64) -> Result<(), AppError> {
        lock_or_recover(&self.revoked).insert(token_hash.to_string(), expires_at);
        Ok(())
    }

    async fn contains(&self, token_hash: &str) -> Result<bool, AppError> {
        let revoked = lock_or_recover(&self.revoked);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(revoked.get(token_hash).copied().unwrap_or(0) > now)
    }

    async fn cleanup_expired(&self) -> Result<(), AppError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut revoked = lock_or_recover(&self.revoked);
        revoked.retain(|_, exp| *exp > now);
        Ok(())
    }
}

/// S-07: 监控历史内存仓储（测试 / InMemory 后端）
pub struct InMemoryMetricsHistoryRepository {
    points: Mutex<std::collections::BTreeMap<(MetricsResolution, i64), MetricsHistoryPoint>>,
}

impl Default for InMemoryMetricsHistoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMetricsHistoryRepository {
    pub fn new() -> Self {
        Self {
            points: Mutex::new(std::collections::BTreeMap::new()),
        }
    }
}

#[async_trait]
impl MetricsHistoryRepository for InMemoryMetricsHistoryRepository {
    async fn upsert(
        &self,
        resolution: MetricsResolution,
        point: &MetricsHistoryPoint,
    ) -> Result<(), AppError> {
        lock_or_recover(&self.points).insert((resolution, point.bucket_start), point.clone());
        Ok(())
    }

    async fn query(
        &self,
        resolution: MetricsResolution,
        from: i64,
        to: i64,
        limit: usize,
    ) -> Result<Vec<MetricsHistoryPoint>, AppError> {
        let points = lock_or_recover(&self.points);
        Ok(points
            .range((resolution, from)..(resolution, to))
            .take(limit)
            .map(|(_, p)| p.clone())
            .collect())
    }

    async fn prune(&self, resolution: MetricsResolution, before: i64) -> Result<u64, AppError> {
        let mut points = lock_or_recover(&self.points);
        let before_len = points.len();
        points.retain(|(res, start), _| !(*res == resolution && *start < before));
        Ok((before_len - points.len()) as u64)
    }
}

pub struct InMemoryBackupAccountRepository {
    accounts: std::sync::Arc<Mutex<Vec<BackupAccount>>>,
}

impl InMemoryBackupAccountRepository {
    pub fn new() -> Self {
        Self {
            accounts: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryBackupAccountRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BackupAccountRepository for InMemoryBackupAccountRepository {
    async fn list_all(&self) -> Result<Vec<BackupAccount>, AppError> {
        Ok(lock_or_recover(&self.accounts).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<BackupAccount>, AppError> {
        Ok(lock_or_recover(&self.accounts)
            .iter()
            .find(|a| a.id == id)
            .cloned())
    }

    async fn create(&self, account: &BackupAccount) -> Result<i64, AppError> {
        let mut accounts = lock_or_recover(&self.accounts);
        if accounts.iter().any(|a| a.name == account.name) {
            return Err(AppError::BadRequest(format!(
                "备份账号已存在: {}",
                account.name
            )));
        }
        let mut a = account.clone();
        a.id = (accounts.len() as i64) + 1;
        let id = a.id;
        accounts.push(a);
        Ok(id)
    }

    async fn update(&self, account: &BackupAccount) -> Result<(), AppError> {
        let mut accounts = lock_or_recover(&self.accounts);
        match accounts.iter_mut().find(|a| a.id == account.id) {
            Some(a) => {
                a.name = account.name.clone();
                a.account_type = account.account_type.clone();
                a.config_json = account.config_json.clone();
                a.enabled = account.enabled;
                a.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!(
                "备份账号不存在: {}",
                account.id
            ))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.accounts).retain(|a| a.id != id);
        Ok(())
    }

    async fn record_test(&self, id: i64, ok: bool) -> Result<(), AppError> {
        let mut accounts = lock_or_recover(&self.accounts);
        match accounts.iter_mut().find(|a| a.id == id) {
            Some(a) => {
                a.last_test_status = Some(if ok { "ok" } else { "failed" }.to_string());
                a.last_test_at = Some(Utc::now());
                Ok(())
            }
            None => Err(AppError::NotFound(format!("备份账号不存在: {}", id))),
        }
    }
}

/// 备份任务内存仓储（M-A 收尾 Stage 3 · Doc/20 §6.5）
pub struct InMemoryBackupJobRepository {
    jobs: std::sync::Arc<Mutex<Vec<BackupJob>>>,
}

impl InMemoryBackupJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryBackupJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BackupJobRepository for InMemoryBackupJobRepository {
    async fn list_all(&self) -> Result<Vec<BackupJob>, AppError> {
        Ok(lock_or_recover(&self.jobs).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<BackupJob>, AppError> {
        Ok(lock_or_recover(&self.jobs)
            .iter()
            .find(|j| j.id == id)
            .cloned())
    }

    async fn create(&self, job: &BackupJob) -> Result<i64, AppError> {
        let mut jobs = lock_or_recover(&self.jobs);
        if jobs.iter().any(|j| j.name == job.name) {
            return Err(AppError::BadRequest(format!(
                "备份任务已存在: {}",
                job.name
            )));
        }
        let mut j = job.clone();
        j.id = jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;
        let id = j.id;
        jobs.push(j);
        Ok(id)
    }

    async fn update(&self, job: &BackupJob) -> Result<(), AppError> {
        let mut jobs = lock_or_recover(&self.jobs);
        match jobs.iter_mut().find(|j| j.id == job.id) {
            Some(j) => {
                j.name = job.name.clone();
                j.scope = job.scope.clone();
                j.scope_key = job.scope_key.clone();
                j.schedule = job.schedule.clone();
                j.target_account_id = job.target_account_id;
                j.retention = job.retention;
                j.enabled = job.enabled;
                j.next_run_at = job.next_run_at;
                j.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("备份任务不存在: {}", job.id))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.jobs).retain(|j| j.id != id);
        Ok(())
    }

    async fn record_run(
        &self,
        id: i64,
        ok: bool,
        message: Option<String>,
        remote_key: Option<String>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        let mut jobs = lock_or_recover(&self.jobs);
        match jobs.iter_mut().find(|j| j.id == id) {
            Some(j) => {
                j.last_status = if ok { "success" } else { "failed" }.to_string();
                j.last_message = message;
                j.last_remote_key = remote_key;
                j.last_run_at = Some(Utc::now());
                j.next_run_at = next_run_at;
                j.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("备份任务不存在: {}", id))),
        }
    }
}

/// 运行时内存仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
pub struct InMemoryRuntimeRepository {
    runtimes: std::sync::Arc<Mutex<Vec<Runtime>>>,
}

impl InMemoryRuntimeRepository {
    pub fn new() -> Self {
        Self {
            runtimes: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryRuntimeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RuntimeRepository for InMemoryRuntimeRepository {
    async fn list_all(&self) -> Result<Vec<Runtime>, AppError> {
        Ok(lock_or_recover(&self.runtimes).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Runtime>, AppError> {
        Ok(lock_or_recover(&self.runtimes)
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn create(&self, runtime: &Runtime) -> Result<i64, AppError> {
        let mut runtimes = lock_or_recover(&self.runtimes);
        if runtimes
            .iter()
            .any(|r| r.kind == runtime.kind && r.version == runtime.version)
        {
            return Err(AppError::BadRequest(format!(
                "运行时已存在: {} {}",
                runtime.kind, runtime.version
            )));
        }
        let mut r = runtime.clone();
        r.id = runtimes.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        let id = r.id;
        runtimes.push(r);
        Ok(id)
    }

    async fn update(&self, runtime: &Runtime) -> Result<(), AppError> {
        let mut runtimes = lock_or_recover(&self.runtimes);
        match runtimes.iter_mut().find(|r| r.id == runtime.id) {
            Some(r) => {
                r.install_path = runtime.install_path.clone();
                r.status = runtime.status.clone();
                r.is_default = runtime.is_default;
                r.metadata_json = runtime.metadata_json.clone();
                r.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("运行时不存在: {}", runtime.id))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.runtimes).retain(|r| r.id != id);
        Ok(())
    }

    async fn set_default(&self, id: i64) -> Result<(), AppError> {
        let mut runtimes = lock_or_recover(&self.runtimes);
        let kind = runtimes
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.kind.clone())
            .ok_or_else(|| AppError::NotFound(format!("运行时不存在: {}", id)))?;
        for r in runtimes.iter_mut() {
            if r.kind == kind {
                r.is_default = r.id == id;
            }
        }
        Ok(())
    }
}

/// 用户项目内存仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
pub struct InMemoryProjectRepository {
    projects: std::sync::Arc<Mutex<Vec<Project>>>,
}

impl InMemoryProjectRepository {
    pub fn new() -> Self {
        Self {
            projects: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryProjectRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProjectRepository for InMemoryProjectRepository {
    async fn list_all(&self) -> Result<Vec<Project>, AppError> {
        Ok(lock_or_recover(&self.projects).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Project>, AppError> {
        Ok(lock_or_recover(&self.projects)
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn create(&self, project: &Project) -> Result<i64, AppError> {
        let mut projects = lock_or_recover(&self.projects);
        if projects.iter().any(|p| p.name == project.name) {
            return Err(AppError::BadRequest(format!(
                "项目已存在: {}",
                project.name
            )));
        }
        let mut p = project.clone();
        p.id = projects.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let id = p.id;
        projects.push(p);
        Ok(id)
    }

    async fn update(&self, project: &Project) -> Result<(), AppError> {
        let mut projects = lock_or_recover(&self.projects);
        match projects.iter_mut().find(|p| p.id == project.id) {
            Some(p) => {
                p.runtime_id = project.runtime_id;
                p.kind = project.kind.clone();
                p.source = project.source.clone();
                p.source_url = project.source_url.clone();
                p.deploy_path = project.deploy_path.clone();
                p.run_command = project.run_command.clone();
                p.port = project.port;
                p.domain = project.domain.clone();
                p.status = project.status.clone();
                p.process_unit = project.process_unit.clone();
                p.deploy_type = project.deploy_type.clone();
                p.auto_deploy = project.auto_deploy;
                p.webhook_token = project.webhook_token.clone();
                p.procfile_json = project.procfile_json.clone();
                p.build_type = project.build_type.clone();
                p.health_check_path = project.health_check_path.clone();
                p.deploy_strategy = project.deploy_strategy.clone();
                p.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("项目不存在: {}", project.id))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.projects).retain(|p| p.id != id);
        Ok(())
    }
}

// ─── 项目数据库联链仓储（内存实现，M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）──

/// 项目数据库联链内存仓储（测试 / 无 SQLite 后端）。
pub struct InMemoryProjectDbLinkRepository {
    links: std::sync::Arc<Mutex<Vec<ProjectDbLink>>>,
}

impl InMemoryProjectDbLinkRepository {
    pub fn new() -> Self {
        Self {
            links: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryProjectDbLinkRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProjectDbLinkRepository for InMemoryProjectDbLinkRepository {
    async fn list_by_project(&self, project_id: i64) -> Result<Vec<ProjectDbLink>, AppError> {
        Ok(lock_or_recover(&self.links)
            .iter()
            .filter(|l| l.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ProjectDbLink>, AppError> {
        Ok(lock_or_recover(&self.links)
            .iter()
            .find(|l| l.id == id)
            .cloned())
    }

    async fn create(&self, link: &ProjectDbLink) -> Result<i64, AppError> {
        let mut links = lock_or_recover(&self.links);
        if links
            .iter()
            .any(|l| l.project_id == link.project_id && l.env_name == link.env_name)
        {
            return Err(AppError::Conflict(format!(
                "环境变量 {} 已存在联链",
                link.env_name
            )));
        }
        if links
            .iter()
            .any(|l| l.project_id == link.project_id && l.db_instance_id == link.db_instance_id)
        {
            return Err(AppError::Conflict("该项目已关联此数据库实例".to_string()));
        }
        let mut l = link.clone();
        l.id = links.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let id = l.id;
        links.push(l);
        Ok(id)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.links).retain(|l| l.id != id);
        Ok(())
    }

    async fn delete_by_project(&self, project_id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.links).retain(|l| l.project_id != project_id);
        Ok(())
    }
}

/// InMemory SSL 证书记录仓储（测试与无 SQLite 降级模式）。
pub struct InMemoryCertificateRepository {
    certs: std::sync::Arc<Mutex<Vec<Certificate>>>,
}

impl InMemoryCertificateRepository {
    pub fn new() -> Self {
        Self {
            certs: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryCertificateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CertificateRepository for InMemoryCertificateRepository {
    async fn list_all(&self) -> Result<Vec<Certificate>, AppError> {
        Ok(lock_or_recover(&self.certs).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Certificate>, AppError> {
        Ok(lock_or_recover(&self.certs)
            .iter()
            .find(|c| c.id == id)
            .cloned())
    }

    async fn find_by_domains(&self, domains: &str) -> Result<Option<Certificate>, AppError> {
        Ok(lock_or_recover(&self.certs)
            .iter()
            .find(|c| c.domains == domains)
            .cloned())
    }

    async fn find_by_website(&self, website_id: i64) -> Result<Option<Certificate>, AppError> {
        Ok(lock_or_recover(&self.certs)
            .iter()
            .filter(|c| c.website_id == Some(website_id))
            .max_by_key(|c| c.id)
            .cloned())
    }

    async fn create(&self, cert: &Certificate) -> Result<i64, AppError> {
        let mut certs = lock_or_recover(&self.certs);
        if certs.iter().any(|c| c.domains == cert.domains) {
            return Err(AppError::Conflict("相同域名集合的证书已存在".to_string()));
        }
        let mut c = cert.clone();
        c.id = certs.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        let id = c.id;
        certs.push(c);
        Ok(id)
    }

    async fn update(&self, cert: &Certificate) -> Result<(), AppError> {
        let mut certs = lock_or_recover(&self.certs);
        let existing = certs
            .iter_mut()
            .find(|c| c.id == cert.id)
            .ok_or_else(|| AppError::NotFound("证书记录不存在".to_string()))?;
        *existing = cert.clone();
        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.certs).retain(|c| c.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod runtime_project_in_memory_tests {
    use super::*;

    fn sample_runtime(kind: &str, version: &str) -> Runtime {
        Runtime {
            id: 0,
            kind: kind.to_string(),
            version: version.to_string(),
            install_path: format!("/opt/flamepanel/runtimes/{}/{}", kind, version),
            status: "not_installed".to_string(),
            is_default: false,
            metadata_json: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn in_memory_runtime_crud_and_unique() {
        let repo = InMemoryRuntimeRepository::new();
        let id = repo.create(&sample_runtime("php", "8.2")).await.unwrap();
        assert_eq!(id, 1);
        assert!(repo.create(&sample_runtime("php", "8.2")).await.is_err());

        let found = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.version, "8.2");

        let mut r = found;
        r.status = "installed".to_string();
        repo.update(&r).await.unwrap();
        assert_eq!(
            repo.find_by_id(id).await.unwrap().unwrap().status,
            "installed"
        );

        assert_eq!(repo.list_all().await.unwrap().len(), 1);
        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());

        let mut missing = sample_runtime("go", "1.22");
        missing.id = 999;
        assert!(repo.update(&missing).await.is_err());
    }

    #[tokio::test]
    async fn in_memory_runtime_set_default() {
        let repo = InMemoryRuntimeRepository::new();
        let a = repo.create(&sample_runtime("php", "8.2")).await.unwrap();
        let b = repo.create(&sample_runtime("php", "8.3")).await.unwrap();

        repo.set_default(a).await.unwrap();
        assert!(repo.find_by_id(a).await.unwrap().unwrap().is_default);

        repo.set_default(b).await.unwrap();
        assert!(!repo.find_by_id(a).await.unwrap().unwrap().is_default);
        assert!(repo.find_by_id(b).await.unwrap().unwrap().is_default);
        assert!(repo.set_default(999).await.is_err());
    }

    fn sample_project(name: &str) -> Project {
        Project {
            id: 0,
            name: name.to_string(),
            runtime_id: None,
            kind: "static".to_string(),
            source: "local_path".to_string(),
            source_url: None,
            deploy_path: format!("/opt/flamepanel/projects/{}", name),
            run_command: None,
            port: None,
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn in_memory_project_crud_and_unique() {
        let repo = InMemoryProjectRepository::new();
        let id = repo.create(&sample_project("demo")).await.unwrap();
        assert_eq!(id, 1);
        assert!(repo.create(&sample_project("demo")).await.is_err());

        let mut p = repo.find_by_id(id).await.unwrap().unwrap();
        p.status = "running".to_string();
        p.port = Some(8080);
        repo.update(&p).await.unwrap();
        let updated = repo.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(updated.status, "running");
        assert_eq!(updated.port, Some(8080));

        repo.delete(id).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_none());

        let mut missing = sample_project("ghost");
        missing.id = 999;
        assert!(repo.update(&missing).await.is_err());
    }
}

#[cfg(test)]
mod p1_fix_tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_docker_shared_state_across_repos() {
        // F-16: 验证共享状态——create 后立即在 list 中可见
        let state = Arc::new(InMemoryDockerState::default());
        let repo1 = InMemoryDockerRepository::new_shared(state.clone());
        let repo2 = InMemoryDockerRepository::new_shared(state.clone());

        let container = DockerContainer {
            id: "test-container".into(),
            image: "nginx:latest".into(),
            name: "test-nginx".into(),
            status: "running".into(),
            node_id: 0,
            created_at: Utc::now(),
        };
        lock_or_recover(&repo1.state.containers).push(container);

        // repo2 应能看到 repo1 创建的容器
        let containers = repo2.list_containers(0).await.unwrap();
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "test-nginx");
    }

    #[tokio::test]
    async fn in_memory_operation_log_delete_older_than() {
        let repo = InMemoryOperationLogRepository::new();
        repo.create("user1", "POST /api/test", Some("/api/test"), None)
            .await
            .unwrap();
        // 创建一条很早的记录
        {
            let mut logs = lock_or_recover(&repo.logs);
            let mut old_log = logs[0].clone();
            old_log.created_at = Utc::now() - chrono::Duration::days(100);
            logs[0] = old_log;
        } // drop logs guard

        let deleted = repo.delete_older_than(90).await.unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn in_memory_log_delete_older_than() {
        let repo = InMemoryLogRepository::new();
        repo.create("system", "INFO", "test message", None)
            .await
            .unwrap();
        // 把记录改早
        {
            let mut logs = lock_or_recover(&repo.logs);
            let mut old_log = logs[0].clone();
            old_log.created_at = Utc::now() - chrono::Duration::days(40);
            logs[0] = old_log;
        } // drop logs guard

        let deleted = repo.delete_older_than(30).await.unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(repo.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn in_memory_outbox_delete_older_than_only_published() {
        let repo = InMemoryOutboxRepository::new();
        // 已发布的旧事件
        repo.append("TestEvent", "{}", true).await.unwrap();
        // 修改为旧时间
        {
            let mut events = lock_or_recover(&repo.events);
            let mut old = events[0].clone();
            old.created_at = Utc::now() - chrono::Duration::days(40);
            old.published = true;
            events[0] = old;
        } // drop events guard

        let deleted = repo.delete_older_than(30).await.unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(repo.count(None).await.unwrap(), 0);
    }
}

#[cfg(test)]
mod certificate_in_memory_tests {
    use super::*;

    fn sample(domains: &str) -> Certificate {
        Certificate {
            domains: domains.to_string(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn in_memory_crud_and_unique() {
        let repo = InMemoryCertificateRepository::new();
        let id = repo.create(&sample(r#"["a.com"]"#)).await.unwrap();
        assert!(repo.create(&sample(r#"["a.com"]"#)).await.is_err());
        let id2 = repo.create(&sample(r#"["b.com"]"#)).await.unwrap();
        assert_eq!(repo.list_all().await.unwrap().len(), 2);
        assert!(repo
            .find_by_domains(r#"["a.com"]"#)
            .await
            .unwrap()
            .is_some());

        let mut c = repo.find_by_id(id).await.unwrap().unwrap();
        c.website_id = Some(3);
        repo.update(&c).await.unwrap();
        assert_eq!(repo.find_by_website(3).await.unwrap().unwrap().id, id);

        repo.delete(id2).await.unwrap();
        assert_eq!(repo.list_all().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn in_memory_update_missing_not_found() {
        let repo = InMemoryCertificateRepository::new();
        let mut c = sample(r#"["a.com"]"#);
        c.id = 42;
        assert!(repo.update(&c).await.is_err());
    }
}

/// DNS 厂商账号内存仓储（主线三 §6.1 · Stage 4）
pub struct InMemoryDnsProviderAccountRepository {
    accounts: std::sync::Arc<Mutex<Vec<DnsProviderAccount>>>,
}

impl InMemoryDnsProviderAccountRepository {
    pub fn new() -> Self {
        Self {
            accounts: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryDnsProviderAccountRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DnsProviderAccountRepository for InMemoryDnsProviderAccountRepository {
    async fn list_all(&self) -> Result<Vec<DnsProviderAccount>, AppError> {
        Ok(lock_or_recover(&self.accounts).clone())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<DnsProviderAccount>, AppError> {
        Ok(lock_or_recover(&self.accounts)
            .iter()
            .find(|a| a.id == id)
            .cloned())
    }

    async fn create(&self, account: &DnsProviderAccount) -> Result<i64, AppError> {
        let mut accounts = lock_or_recover(&self.accounts);
        if accounts.iter().any(|a| a.name == account.name) {
            return Err(AppError::BadRequest(format!(
                "DNS 厂商账号已存在: {}",
                account.name
            )));
        }
        let mut a = account.clone();
        a.id = (accounts.iter().map(|x| x.id).max().unwrap_or(0)) + 1;
        let id = a.id;
        accounts.push(a);
        Ok(id)
    }

    async fn update(&self, account: &DnsProviderAccount) -> Result<(), AppError> {
        let mut accounts = lock_or_recover(&self.accounts);
        match accounts.iter_mut().find(|a| a.id == account.id) {
            Some(a) => {
                a.name = account.name.clone();
                a.provider_type = account.provider_type.clone();
                a.credentials_json = account.credentials_json.clone();
                a.enabled = account.enabled;
                a.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!(
                "DNS 厂商账号不存在: {}",
                account.id
            ))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.accounts).retain(|a| a.id != id);
        Ok(())
    }

    async fn record_test(&self, id: i64, ok: bool) -> Result<(), AppError> {
        let mut accounts = lock_or_recover(&self.accounts);
        match accounts.iter_mut().find(|a| a.id == id) {
            Some(a) => {
                a.last_test_status = Some(if ok { "ok" } else { "failed" }.to_string());
                a.last_test_at = Some(Utc::now());
                Ok(())
            }
            None => Err(AppError::NotFound(format!("DNS 厂商账号不存在: {}", id))),
        }
    }
}

// ── WAF 基础防护（主线三 §6.2）：InMemory 仓储 ─────────────────────────

pub struct InMemoryWafRuleRepository {
    rules: Arc<Mutex<Vec<WafRule>>>,
}

impl InMemoryWafRuleRepository {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryWafRuleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WafRuleRepository for InMemoryWafRuleRepository {
    async fn list_all(&self, website_id: Option<i64>) -> Result<Vec<WafRule>, AppError> {
        let rules = lock_or_recover(&self.rules);
        Ok(rules
            .iter()
            .filter(|r| website_id.is_none_or(|w| r.website_id == w))
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WafRule>, AppError> {
        Ok(lock_or_recover(&self.rules)
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn create(&self, rule: &WafRule) -> Result<i64, AppError> {
        let mut rules = lock_or_recover(&self.rules);
        let mut r = rule.clone();
        r.id = (rules.iter().map(|x| x.id).max().unwrap_or(0)) + 1;
        let id = r.id;
        rules.push(r);
        Ok(id)
    }

    async fn update(&self, rule: &WafRule) -> Result<(), AppError> {
        let mut rules = lock_or_recover(&self.rules);
        match rules.iter_mut().find(|r| r.id == rule.id) {
            Some(r) => {
                r.name = rule.name.clone();
                r.rule_type = rule.rule_type.clone();
                r.website_id = rule.website_id;
                r.pattern = rule.pattern.clone();
                r.config_json = rule.config_json.clone();
                r.action = rule.action.clone();
                r.enabled = rule.enabled;
                r.updated_at = Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("WAF 规则不存在: {}", rule.id))),
        }
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.rules).retain(|r| r.id != id);
        Ok(())
    }

    async fn list_enabled(&self) -> Result<Vec<WafRule>, AppError> {
        let rules = lock_or_recover(&self.rules);
        Ok(rules.iter().filter(|r| r.enabled).cloned().collect())
    }
}

pub struct InMemoryWafLogRepository {
    logs: Arc<Mutex<Vec<WafLog>>>,
}

impl InMemoryWafLogRepository {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryWafLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WafLogRepository for InMemoryWafLogRepository {
    async fn create(&self, log: &WafLog) -> Result<i64, AppError> {
        let mut logs = lock_or_recover(&self.logs);
        let mut l = log.clone();
        l.id = (logs.iter().map(|x| x.id).max().unwrap_or(0)) + 1;
        let id = l.id;
        logs.push(l);
        Ok(id)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WafLog>, AppError> {
        Ok(lock_or_recover(&self.logs)
            .iter()
            .find(|l| l.id == id)
            .cloned())
    }

    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        website_id: Option<i64>,
    ) -> Result<Vec<WafLog>, AppError> {
        let logs = lock_or_recover(&self.logs);
        let mut filtered: Vec<WafLog> = logs
            .iter()
            .filter(|l| {
                website_id
                    .filter(|w| *w > 0)
                    .is_none_or(|w| l.website_id == w)
            })
            .cloned()
            .collect();
        filtered.sort_by_key(|l| std::cmp::Reverse(l.id));
        Ok(filtered
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .collect())
    }

    async fn count(&self, website_id: Option<i64>) -> Result<i64, AppError> {
        let logs = lock_or_recover(&self.logs);
        Ok(logs
            .iter()
            .filter(|l| {
                website_id
                    .filter(|w| *w > 0)
                    .is_none_or(|w| l.website_id == w)
            })
            .count() as i64)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.logs).retain(|l| l.id != id);
        Ok(())
    }

    async fn clear(&self, website_id: Option<i64>) -> Result<u64, AppError> {
        let mut logs = lock_or_recover(&self.logs);
        let before = logs.len();
        logs.retain(|l| {
            website_id
                .filter(|w| *w > 0)
                .is_some_and(|w| l.website_id != w)
        });
        Ok((before - logs.len()) as u64)
    }

    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let mut logs = lock_or_recover(&self.logs);
        let before = logs.len();
        logs.retain(|l| l.created_at >= cutoff);
        Ok((before - logs.len()) as u64)
    }
}

/// M-AI-1：MCP 凭证 / 会话 / 调用审计的内存仓储（测试与 InMemory 后端）。
#[derive(Default)]
pub struct InMemoryMcpRepository {
    principals: Mutex<Vec<crate::domain::mcp::AgentPrincipal>>,
    sessions: Mutex<Vec<crate::domain::mcp::AgentSession>>,
    calls: Mutex<Vec<crate::domain::mcp::ToolCallRecord>>,
    approvals: Mutex<Vec<crate::domain::mcp::ToolApproval>>,
}

impl InMemoryMcpRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

fn mcp_call_matches(
    c: &crate::domain::mcp::ToolCallRecord,
    f: &crate::domain::mcp::ToolCallFilter,
) -> bool {
    f.principal_id.map(|p| c.principal_id == p).unwrap_or(true)
        && f.session_id
            .as_deref()
            .map(|s| c.session_id.as_deref() == Some(s))
            .unwrap_or(true)
        && f.tool.as_deref().map(|t| c.tool == t).unwrap_or(true)
}

#[async_trait]
impl McpRepository for InMemoryMcpRepository {
    async fn create_principal(
        &self,
        p: &crate::domain::mcp::AgentPrincipal,
    ) -> Result<i64, AppError> {
        let mut all = lock_or_recover(&self.principals);
        let id = all.iter().map(|x| x.id).max().unwrap_or(0) + 1;
        let mut p = p.clone();
        p.id = id;
        all.push(p);
        Ok(id)
    }

    async fn list_principals(&self) -> Result<Vec<crate::domain::mcp::AgentPrincipal>, AppError> {
        let mut v = lock_or_recover(&self.principals).clone();
        v.sort_by_key(|a| std::cmp::Reverse(a.id));
        Ok(v)
    }

    async fn find_principal(
        &self,
        id: i64,
    ) -> Result<Option<crate::domain::mcp::AgentPrincipal>, AppError> {
        Ok(lock_or_recover(&self.principals)
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn find_principal_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<crate::domain::mcp::AgentPrincipal>, AppError> {
        Ok(lock_or_recover(&self.principals)
            .iter()
            .find(|p| p.token_hash == token_hash)
            .cloned())
    }

    async fn touch_principal(&self, id: i64, at: DateTime<Utc>) -> Result<(), AppError> {
        if let Some(p) = lock_or_recover(&self.principals)
            .iter_mut()
            .find(|p| p.id == id)
        {
            p.last_used_at = Some(at);
        }
        Ok(())
    }

    async fn revoke_principal(&self, id: i64, at: DateTime<Utc>) -> Result<(), AppError> {
        if let Some(p) = lock_or_recover(&self.principals)
            .iter_mut()
            .find(|p| p.id == id)
        {
            p.revoked_at.get_or_insert(at);
        }
        Ok(())
    }

    async fn create_session(&self, s: &crate::domain::mcp::AgentSession) -> Result<(), AppError> {
        lock_or_recover(&self.sessions).push(s.clone());
        Ok(())
    }

    async fn find_session(
        &self,
        id: &str,
    ) -> Result<Option<crate::domain::mcp::AgentSession>, AppError> {
        Ok(lock_or_recover(&self.sessions)
            .iter()
            .find(|s| s.id == id)
            .cloned())
    }

    async fn touch_session(&self, id: &str, at: DateTime<Utc>) -> Result<(), AppError> {
        if let Some(s) = lock_or_recover(&self.sessions)
            .iter_mut()
            .find(|s| s.id == id)
        {
            s.last_seen_at = at;
        }
        Ok(())
    }

    async fn close_session(&self, id: &str, at: DateTime<Utc>) -> Result<(), AppError> {
        if let Some(s) = lock_or_recover(&self.sessions)
            .iter_mut()
            .find(|s| s.id == id)
        {
            s.closed_at.get_or_insert(at);
        }
        Ok(())
    }

    async fn list_sessions(
        &self,
        principal_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<crate::domain::mcp::AgentSession>, AppError> {
        let mut v: Vec<_> = lock_or_recover(&self.sessions)
            .iter()
            .filter(|s| principal_id.map(|p| s.principal_id == p).unwrap_or(true))
            .cloned()
            .collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.last_seen_at));
        v.truncate(limit.max(0) as usize);
        Ok(v)
    }

    async fn record_call(&self, c: &crate::domain::mcp::ToolCallRecord) -> Result<i64, AppError> {
        let mut all = lock_or_recover(&self.calls);
        let id = all.iter().map(|x| x.id).max().unwrap_or(0) + 1;
        let mut c = c.clone();
        c.id = id;
        all.push(c);
        Ok(id)
    }

    async fn list_calls(
        &self,
        filter: &crate::domain::mcp::ToolCallFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::domain::mcp::ToolCallRecord>, AppError> {
        let all = lock_or_recover(&self.calls);
        Ok(all
            .iter()
            .rev()
            .filter(|c| mcp_call_matches(c, filter))
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }

    async fn count_calls(
        &self,
        filter: &crate::domain::mcp::ToolCallFilter,
    ) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.calls)
            .iter()
            .filter(|c| mcp_call_matches(c, filter))
            .count() as i64)
    }

    async fn prune_calls(&self, keep: i64) -> Result<u64, AppError> {
        let mut all = lock_or_recover(&self.calls);
        let keep = keep.max(0) as usize;
        if all.len() <= keep {
            return Ok(0);
        }
        let n = all.len() - keep;
        all.drain(0..n);
        Ok(n as u64)
    }

    async fn create_approval(&self, a: &crate::domain::mcp::ToolApproval) -> Result<i64, AppError> {
        let mut all = lock_or_recover(&self.approvals);
        let id = all.iter().map(|x| x.id).max().unwrap_or(0) + 1;
        let mut a = a.clone();
        a.id = id;
        all.push(a);
        Ok(id)
    }

    async fn find_approval(
        &self,
        id: i64,
    ) -> Result<Option<crate::domain::mcp::ToolApproval>, AppError> {
        Ok(lock_or_recover(&self.approvals)
            .iter()
            .find(|a| a.id == id)
            .cloned())
    }

    async fn list_approvals(
        &self,
        filter: &crate::domain::mcp::ApprovalFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::domain::mcp::ToolApproval>, AppError> {
        let mut v: Vec<_> = lock_or_recover(&self.approvals)
            .iter()
            .filter(|a| mcp_approval_matches(a, filter))
            .cloned()
            .collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.id));
        Ok(v.into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .collect())
    }

    async fn count_approvals(
        &self,
        filter: &crate::domain::mcp::ApprovalFilter,
    ) -> Result<i64, AppError> {
        Ok(lock_or_recover(&self.approvals)
            .iter()
            .filter(|a| mcp_approval_matches(a, filter))
            .count() as i64)
    }

    async fn transition_approval(
        &self,
        id: i64,
        from: crate::domain::mcp::ApprovalStatus,
        to: crate::domain::mcp::ApprovalStatus,
        decided_by: Option<i64>,
        at: DateTime<Utc>,
        reason: &str,
    ) -> Result<bool, AppError> {
        let mut all = lock_or_recover(&self.approvals);
        match all.iter_mut().find(|a| a.id == id && a.status == from) {
            Some(a) => {
                a.status = to;
                a.decided_by = decided_by;
                a.decided_at = Some(at);
                a.reason = reason.to_string();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn finish_approval(
        &self,
        id: i64,
        status: crate::domain::mcp::ApprovalStatus,
        result: &str,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if let Some(a) = lock_or_recover(&self.approvals)
            .iter_mut()
            .find(|a| a.id == id && a.status == crate::domain::mcp::ApprovalStatus::Executing)
        {
            a.status = status;
            a.result = result.to_string();
            a.finished_at = Some(at);
        }
        Ok(())
    }

    async fn expire_approvals(&self, now: DateTime<Utc>) -> Result<u64, AppError> {
        let mut n = 0;
        for a in lock_or_recover(&self.approvals).iter_mut() {
            if a.status == crate::domain::mcp::ApprovalStatus::Pending && a.expires_at < now {
                a.status = crate::domain::mcp::ApprovalStatus::Expired;
                a.decided_at = Some(now);
                n += 1;
            }
        }
        Ok(n)
    }
}

fn mcp_approval_matches(
    a: &crate::domain::mcp::ToolApproval,
    f: &crate::domain::mcp::ApprovalFilter,
) -> bool {
    f.principal_id.map(|p| a.principal_id == p).unwrap_or(true)
        && f.status.map(|s| a.status == s).unwrap_or(true)
}

/// 两步验证（TOTP）内存仓储（测试与 InMemory 后端）。
#[derive(Default)]
pub struct InMemoryTwoFactorRepository {
    rows: Mutex<std::collections::HashMap<i64, crate::domain::two_factor::UserTwoFactor>>,
}

impl InMemoryTwoFactorRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl crate::domain::two_factor::TwoFactorRepository for InMemoryTwoFactorRepository {
    async fn get(
        &self,
        user_id: i64,
    ) -> Result<Option<crate::domain::two_factor::UserTwoFactor>, AppError> {
        Ok(lock_or_recover(&self.rows).get(&user_id).cloned())
    }

    async fn upsert(&self, rec: &crate::domain::two_factor::UserTwoFactor) -> Result<(), AppError> {
        lock_or_recover(&self.rows).insert(rec.user_id, rec.clone());
        Ok(())
    }

    async fn delete(&self, user_id: i64) -> Result<bool, AppError> {
        Ok(lock_or_recover(&self.rows).remove(&user_id).is_some())
    }

    async fn list_enabled_user_ids(&self) -> Result<Vec<i64>, AppError> {
        Ok(lock_or_recover(&self.rows)
            .values()
            .filter(|r| r.enabled)
            .map(|r| r.user_id)
            .collect())
    }

    async fn advance_step(&self, user_id: i64, step: i64) -> Result<bool, AppError> {
        let mut rows = lock_or_recover(&self.rows);
        match rows.get_mut(&user_id) {
            Some(r) if r.last_step.is_none_or(|l| l < step) => {
                r.last_step = Some(step);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn consume_recovery(&self, user_id: i64, hash: &str) -> Result<bool, AppError> {
        let mut rows = lock_or_recover(&self.rows);
        let Some(r) = rows.get_mut(&user_id) else {
            return Ok(false);
        };
        let before = r.recovery_hashes.len();
        r.recovery_hashes.retain(|h| h != hash);
        Ok(r.recovery_hashes.len() < before)
    }
}
