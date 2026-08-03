use crate::api::types::{paginate_slice, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::database::{mysql::MySqlManager, redis::RedisManager, NativeDbManager};
use crate::domain::entity::*;
use crate::domain::repository::*;
use crate::event::EventBus;
use crate::webserver::{get_config_generator, WebServerEngine, WebServerManager};
use std::collections::HashMap;
use std::sync::Arc;

pub struct UserService {
    pub user_repo: Arc<dyn UserRepository>,
    pub event_bus: EventBus,
}

impl UserService {
    pub fn new(user_repo: Arc<dyn UserRepository>, event_bus: EventBus) -> Self {
        Self {
            user_repo,
            event_bus,
        }
    }

    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError> {
        let user = self.user_repo.create(username, password_hash, role).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::UserCreated {
                user_id: user.id,
                username: username.to_string(),
            })
            .await;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
        self.user_repo.find_by_id(id).await
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        self.user_repo.find_by_username(username).await
    }

    pub async fn update_password(&self, id: i64, new_hash: &str) -> Result<(), AppError> {
        self.user_repo.update_password(id, new_hash).await
    }

    pub async fn list_users(&self) -> Result<Vec<User>, AppError> {
        self.user_repo.list().await
    }

    pub async fn list_users_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<User>, AppError> {
        let users = self.user_repo.list().await?;
        let total = users.len() as i64;
        let data = paginate_slice(&users, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_user(&self, id: i64) -> Result<User, AppError> {
        self.user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
    }

    pub async fn update_user(&self, user: &User) -> Result<(), AppError> {
        self.user_repo
            .find_by_id(user.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user.id)))?;
        self.user_repo.update(user).await
    }

    pub async fn delete_user(&self, id: i64) -> Result<(), AppError> {
        self.user_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
        self.user_repo.delete(id).await
    }
}

pub struct NodeService {
    pub node_repo: Arc<dyn NodeRepository>,
    pub event_bus: EventBus,
}

impl NodeService {
    pub fn new(node_repo: Arc<dyn NodeRepository>, event_bus: EventBus) -> Self {
        Self {
            node_repo,
            event_bus,
        }
    }

    pub async fn register_node(&self, node: &ServerNode) -> Result<i64, AppError> {
        let id = self.node_repo.create(node).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::NodeRegistered {
                node_id: id,
                node_name: node.name.clone(),
            })
            .await;
        Ok(id)
    }

    pub async fn list_nodes(&self) -> Result<Vec<ServerNode>, AppError> {
        self.node_repo.list_all().await
    }

    pub async fn list_nodes_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<ServerNode>, AppError> {
        let nodes = self.node_repo.list_all().await?;
        let total = nodes.len() as i64;
        let data = paginate_slice(&nodes, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_node(&self, id: i64) -> Result<ServerNode, AppError> {
        self.node_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node {} not found", id)))
    }

    pub async fn update_node(&self, node: &ServerNode) -> Result<(), AppError> {
        self.node_repo
            .find_by_id(node.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node {} not found", node.id)))?;
        self.node_repo.update(node).await
    }

    pub async fn delete_node(&self, id: i64) -> Result<(), AppError> {
        self.node_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Node {} not found", id)))?;
        self.node_repo.delete(id).await
    }
}

pub struct WebsiteService {
    pub website_repo: Arc<dyn WebsiteRepository>,
    pub event_bus: EventBus,
}

impl WebsiteService {
    pub fn new(website_repo: Arc<dyn WebsiteRepository>, event_bus: EventBus) -> Self {
        Self {
            website_repo,
            event_bus,
        }
    }

    pub async fn create_website(&self, website: &Website) -> Result<i64, AppError> {
        let id = self.website_repo.create(website).await?;
        let _ = self
            .event_bus
            .publish(DomainEvent::WebsiteCreated {
                website_id: id,
                domain: website.domain.clone(),
            })
            .await;
        Ok(id)
    }

    pub async fn list_websites(&self) -> Result<Vec<Website>, AppError> {
        self.website_repo.list_all().await
    }

    pub async fn list_websites_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<Website>, AppError> {
        let websites = self.website_repo.list_all().await?;
        let total = websites.len() as i64;
        let data = paginate_slice(&websites, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_website(&self, id: i64) -> Result<Website, AppError> {
        self.website_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Website {} not found", id)))
    }

    pub async fn update_website(&self, website: &Website) -> Result<(), AppError> {
        self.website_repo.update(website).await
    }

    pub async fn delete_website(&self, id: i64) -> Result<(), AppError> {
        self.website_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Website {} not found", id)))?;
        self.website_repo.delete(id).await
    }

    /// 切换网站 Web 服务器引擎，并重新生成站点配置
    pub async fn switch_engine(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<Website, AppError> {
        let mut site = self.get_website(id).await?;
        let old_engine = site.engine.clone();
        if old_engine.eq_ignore_ascii_case(new_engine.as_str()) {
            return Ok(site);
        }
        site.engine = new_engine.as_str().into();
        let generated = crate::webserver::get_config_generator(new_engine)
            .generate_site_config(&site, None, None);
        let _ = generated;
        self.website_repo.update(&site).await?;
        Ok(site)
    }
}

pub struct DockerService {
    pub docker_repo: Arc<dyn DockerRepository>,
}

impl DockerService {
    pub fn new(docker_repo: Arc<dyn DockerRepository>) -> Self {
        Self { docker_repo }
    }

    pub async fn list_containers(&self, node_id: i64) -> Result<Vec<DockerContainer>, AppError> {
        self.docker_repo.list_containers(node_id).await
    }

    pub async fn start_container(&self, id: &str) -> Result<(), AppError> {
        self.docker_repo.start_container(id).await
    }

    pub async fn stop_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        self.docker_repo.stop_container(id, timeout).await
    }

    pub async fn restart_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        self.docker_repo.restart_container(id, timeout).await
    }

    pub async fn remove_container(&self, id: &str, force: bool) -> Result<(), AppError> {
        self.docker_repo.remove_container(id, force).await
    }

    pub async fn get_container_logs(&self, id: &str, tail: usize) -> Result<String, AppError> {
        self.docker_repo.get_container_logs(id, tail).await
    }

    pub async fn get_container_stats(&self, id: &str) -> Result<serde_json::Value, AppError> {
        self.docker_repo.get_container_stats(id).await
    }

    pub async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.docker_repo.list_images().await
    }

    pub async fn remove_image(&self, id: &str) -> Result<(), AppError> {
        self.docker_repo.remove_image(id).await
    }

    pub async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError> {
        self.docker_repo
            .compose_deploy(project_name, compose_yaml)
            .await
    }

    pub async fn compose_up(&self, project_name: &str) -> Result<(), AppError> {
        self.docker_repo.compose_up(project_name).await
    }

    pub async fn compose_down(&self, project_name: &str) -> Result<(), AppError> {
        self.docker_repo.compose_down(project_name).await
    }

    pub async fn inspect_container(&self, id: &str) -> Result<serde_json::Value, AppError> {
        self.docker_repo.inspect_container(id).await
    }

    pub async fn rename_container(&self, id: &str, new_name: &str) -> Result<(), AppError> {
        self.docker_repo.rename_container(id, new_name).await
    }

    pub async fn pause_container(&self, id: &str) -> Result<(), AppError> {
        self.docker_repo.pause_container(id).await
    }

    pub async fn unpause_container(&self, id: &str) -> Result<(), AppError> {
        self.docker_repo.unpause_container(id).await
    }

    pub async fn kill_container(&self, id: &str) -> Result<(), AppError> {
        self.docker_repo.kill_container(id).await
    }

    pub async fn prune_containers(&self) -> Result<serde_json::Value, AppError> {
        self.docker_repo.prune_containers().await
    }

    pub async fn list_networks(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.docker_repo.list_networks().await
    }

    pub async fn create_network(
        &self,
        name: &str,
        driver: &str,
        subnet: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        self.docker_repo.create_network(name, driver, subnet).await
    }

    pub async fn remove_network(&self, id: &str) -> Result<(), AppError> {
        self.docker_repo.remove_network(id).await
    }

    pub async fn connect_network(&self, network_id: &str, container_id: &str) -> Result<(), AppError> {
        self.docker_repo
            .connect_network(network_id, container_id)
            .await
    }

    pub async fn disconnect_network(
        &self,
        network_id: &str,
        container_id: &str,
        force: bool,
    ) -> Result<(), AppError> {
        self.docker_repo
            .disconnect_network(network_id, container_id, force)
            .await
    }

    pub async fn prune_networks(&self) -> Result<serde_json::Value, AppError> {
        self.docker_repo.prune_networks().await
    }

    pub async fn list_volumes(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.docker_repo.list_volumes().await
    }

    pub async fn create_volume(&self, name: &str, driver: &str) -> Result<serde_json::Value, AppError> {
        self.docker_repo.create_volume(name, driver).await
    }

    pub async fn remove_volume(&self, name: &str, force: bool) -> Result<(), AppError> {
        self.docker_repo.remove_volume(name, force).await
    }

    pub async fn prune_volumes(&self) -> Result<serde_json::Value, AppError> {
        self.docker_repo.prune_volumes().await
    }

    pub async fn pull_image(&self, image: &str) -> Result<String, AppError> {
        self.docker_repo.pull_image(image).await
    }

    pub async fn tag_image(&self, image_id: &str, repo: &str, tag: &str) -> Result<(), AppError> {
        self.docker_repo.tag_image(image_id, repo, tag).await
    }

    pub async fn prune_images(&self) -> Result<serde_json::Value, AppError> {
        self.docker_repo.prune_images().await
    }

    pub async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.docker_repo.compose_ls().await
    }
}

pub struct RoleService {
    pub role_repo: Arc<dyn RoleRepository>,
    pub perm_repo: Arc<dyn PermissionRepository>,
}

impl RoleService {
    pub fn new(
        role_repo: Arc<dyn RoleRepository>,
        perm_repo: Arc<dyn PermissionRepository>,
    ) -> Self {
        Self {
            role_repo,
            perm_repo,
        }
    }

    pub async fn list_roles(&self) -> Result<Vec<Role>, AppError> {
        self.role_repo.list_all().await
    }

    pub async fn create_role(&self, role: &Role) -> Result<i64, AppError> {
        self.role_repo.create(role).await
    }

    pub async fn update_role(&self, role: &Role) -> Result<(), AppError> {
        self.role_repo.update(role).await
    }

    pub async fn delete_role(&self, id: i64) -> Result<(), AppError> {
        self.role_repo.delete(id).await
    }

    pub async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        self.role_repo.get_role_permissions(role_id).await
    }

    pub async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError> {
        self.role_repo
            .set_role_permissions(role_id, permission_ids)
            .await
    }

    pub async fn check_permission(
        &self,
        user_role: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AppError> {
        let role = self.role_repo.find_by_name(user_role).await?;
        match role {
            Some(r) => {
                let pids = self.role_repo.get_role_permissions(r.id).await?;
                let perm = self
                    .perm_repo
                    .find_by_resource_action(resource, action)
                    .await?;
                match perm {
                    Some(p) => Ok(pids.contains(&p.id)),
                    None => Ok(false),
                }
            }
            None => Ok(false),
        }
    }
}

pub struct PermissionService {
    pub perm_repo: Arc<dyn PermissionRepository>,
}

impl PermissionService {
    pub fn new(perm_repo: Arc<dyn PermissionRepository>) -> Self {
        Self { perm_repo }
    }

    pub async fn list_permissions(&self) -> Result<Vec<Permission>, AppError> {
        self.perm_repo.list_all().await
    }
}

pub struct WebServerService {
    pub server_repo: Arc<dyn WebServerRepository>,
    pub manager: WebServerManager,
    pub native_manager: crate::webserver::native::WebServerNativeManager,
}

impl WebServerService {
    pub fn new(server_repo: Arc<dyn WebServerRepository>) -> Self {
        Self {
            server_repo,
            manager: WebServerManager::new(),
            native_manager: crate::webserver::native::WebServerNativeManager::new(),
        }
    }

    pub async fn list_servers(&self) -> Result<Vec<WebServerInstance>, AppError> {
        self.server_repo.list_all().await
    }

    pub async fn list_servers_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<WebServerInstance>, AppError> {
        let servers = self.server_repo.list_all().await?;
        let total = servers.len() as i64;
        let data = paginate_slice(&servers, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_server(&self, id: i64) -> Result<WebServerInstance, AppError> {
        self.server_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Web server {} not found", id)))
    }

    pub async fn create_server(&self, instance: &WebServerInstance) -> Result<i64, AppError> {
        self.server_repo.create(instance).await
    }

    pub async fn update_server(&self, instance: &WebServerInstance) -> Result<(), AppError> {
        self.server_repo.update(instance).await
    }

    pub async fn delete_server(&self, id: i64) -> Result<(), AppError> {
        self.server_repo.delete(id).await
    }

    pub async fn start_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.start(&instance).await
    }

    pub async fn stop_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.stop(&instance).await
    }

    pub async fn restart_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.restart(&instance).await
    }

    pub async fn reload_server(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.reload(&instance).await
    }

    pub async fn test_server_config(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.config_test(&instance).await
    }

    pub async fn check_server_status(&self, id: i64) -> Result<String, AppError> {
        let instance = self.get_server(id).await?;
        self.manager.check_status(&instance).await
    }

    pub fn generate_site_config(
        &self,
        site: &Website,
        engine: &WebServerEngine,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String {
        let generator = get_config_generator(engine);
        generator.generate_site_config(site, ssl_cert, ssl_key)
    }

    pub fn generate_global_config(
        &self,
        engine: &WebServerEngine,
        port: u16,
        workers: u32,
    ) -> String {
        let generator = get_config_generator(engine);
        generator.generate_global_config(port, workers)
    }

    pub fn generate_reverse_proxy_config(
        &self,
        engine: &WebServerEngine,
        domain: &str,
        proxy_pass: &str,
        port: u16,
    ) -> String {
        let generator = get_config_generator(engine);
        generator.generate_reverse_proxy_config(domain, proxy_pass, port)
    }

    pub async fn write_site_config(
        &self,
        engine: &WebServerEngine,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> Result<(), AppError> {
        let config = self.generate_site_config(site, engine, ssl_cert, ssl_key);
        let config_path = format!("{}/{}", engine.sites_available_dir(), site.domain);
        self.manager.write_config_file(&config_path, &config).await
    }

    pub async fn enable_site(
        &self,
        engine: &WebServerEngine,
        site: &Website,
    ) -> Result<(), AppError> {
        let config = self.generate_site_config(site, engine, None, None);
        self.manager
            .enable_site(engine, &site.domain, &config)
            .await
    }

    pub async fn disable_site(
        &self,
        engine: &WebServerEngine,
        site: &Website,
    ) -> Result<(), AppError> {
        self.manager.disable_site(engine, &site.domain).await
    }

    /// 切换 Web 服务器实例引擎：更新实例信息并重新生成全局配置
    pub async fn switch_engine(
        &self,
        id: i64,
        new_engine: &WebServerEngine,
    ) -> Result<WebServerInstance, AppError> {
        let instance = self.get_server(id).await?;
        let new = WebServerInstance {
            id: instance.id,
            engine: new_engine.as_str().into(),
            version: instance.version,
            status: instance.status,
            config_path: new_engine.default_config_path().into(),
            binary_path: Some(new_engine.binary_name().into()),
            port: instance.port,
            created_at: instance.created_at,
        };
        self.server_repo.update(&new).await?;
        Ok(new)
    }

    /// 应用性能预设：根据引擎生成全局配置并写入
    pub async fn apply_preset(
        &self,
        id: i64,
        preset: &crate::webserver::preset::PerformancePreset,
    ) -> Result<WebServerInstance, AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        let config = self.generate_global_config(
            &engine,
            instance.port as u16,
            preset.worker_processes(&instance.engine),
        );
        self.manager
            .write_config_file(&instance.config_path, &config)
            .await?;
        Ok(instance)
    }

    // ── 原生安装 / 卸载 / 服务控制（对接系统包管理器与 systemd） ──

    /// 检测本机已安装的 Web 服务器（1Panel 风格）
    pub async fn native_detect(&self) -> Vec<crate::webserver::native::NativeWebServerInfo> {
        self.native_manager.detect_all().await
    }

    /// 原生安装指定引擎并注册实例
    pub async fn native_install(
        &self,
        engine: &WebServerEngine,
        version: Option<&str>,
    ) -> Result<WebServerInstance, AppError> {
        let message = self.native_manager.install(engine, version).await?;
        // 自动注册实例，便于统一管理
        let instance = WebServerInstance {
            id: 0,
            engine: engine.as_str().into(),
            version: Some(message),
            status: "running".into(),
            config_path: engine.default_config_path().into(),
            binary_path: Some(engine.binary_name().into()),
            port: engine.default_port() as i32,
            created_at: chrono::Utc::now(),
        };
        let id = self.server_repo.create(&instance).await?;
        let created = self.get_server(id).await?;
        Ok(created)
    }

    /// 原生卸载并删除注册实例
    pub async fn native_uninstall(&self, id: i64) -> Result<(), AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        self.native_manager.uninstall(&engine).await?;
        self.server_repo.delete(id).await
    }

    /// 按引擎名原生卸载（同时清理该引擎的注册实例）
    pub async fn native_uninstall_by_engine(&self, engine: &WebServerEngine) -> Result<(), AppError> {
        self.native_manager.uninstall(engine).await?;
        let instances = self.server_repo.find_by_engine(engine.as_str()).await?;
        for inst in instances {
            self.server_repo.delete(inst.id).await?;
        }
        Ok(())
    }

    /// systemd 开机自启开关（systemd 不可用时容错降级）
    pub async fn set_autostart(
        &self,
        id: i64,
        enabled: bool,
    ) -> Result<WebServerInstance, AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        self.set_autostart_by_engine(&engine, enabled).await?;
        Ok(instance)
    }

    /// 按引擎名设置开机自启
    pub async fn set_autostart_by_engine(
        &self,
        engine: &WebServerEngine,
        enabled: bool,
    ) -> Result<(), AppError> {
        let result = self.native_manager.set_autostart(engine, enabled).await;
        if let Err(e) = result {
            tracing::warn!(
                "systemd autostart {} for {} unavailable, ignored: {}",
                if enabled { "enable" } else { "disable" },
                engine.as_str(),
                e
            );
        }
        Ok(())
    }

    /// 实例的原生状态详情（安装/版本/服务/端口）
    pub async fn native_status(
        &self,
        id: i64,
    ) -> Result<crate::webserver::native::NativeWebServerInfo, AppError> {
        let instance = self.get_server(id).await?;
        let engine = WebServerEngine::from_name(&instance.engine)
            .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", instance.engine)))?;
        Ok(self.native_manager.detect(&engine).await)
    }
}

pub struct SettingsService {
    pub repo: Arc<dyn SettingsRepository>,
}

impl SettingsService {
    pub fn new(repo: Arc<dyn SettingsRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError> {
        self.repo.list_all().await
    }

    pub async fn list_all_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<PanelSetting>, AppError> {
        let settings = self.repo.list_all().await?;
        let total = settings.len() as i64;
        let data = paginate_slice(&settings, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        self.repo.get(key).await
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.repo.set(key, value).await
    }

    pub async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        self.repo.get_all_map().await
    }
}

pub struct DatabaseService {
    pub repo: Arc<dyn DatabaseRepository>,
    pub mysql_manager: MySqlManager,
    pub redis_manager: RedisManager,
}

impl DatabaseService {
    pub fn new(repo: Arc<dyn DatabaseRepository>) -> Self {
        Self {
            repo,
            mysql_manager: MySqlManager::new(),
            redis_manager: RedisManager::new(),
        }
    }

    pub async fn list_instances(&self) -> Result<Vec<DatabaseInstance>, AppError> {
        self.repo.list_all().await
    }

    pub async fn list_instances_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<DatabaseInstance>, AppError> {
        let instances = self.repo.list_all().await?;
        let total = instances.len() as i64;
        let data = paginate_slice(&instances, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_instance(&self, id: i64) -> Result<DatabaseInstance, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database instance {} not found", id)))
    }

    pub async fn delete_instance(&self, id: i64) -> Result<(), AppError> {
        self.repo.delete(id).await
    }

    pub async fn install_mysql(
        &self,
        version: Option<&str>,
        port: i32,
        password: &str,
        name: &str,
    ) -> Result<DatabaseInstance, AppError> {
        let db_type = DatabaseType::Mysql;
        self.mysql_manager.install(version, port, password).await?;
        let ver = self
            .mysql_manager
            .get_version()
            .await
            .unwrap_or_else(|_| "latest".into());
        let instance = DatabaseInstance {
            id: 0,
            db_type: db_type.as_str().into(),
            name: name.into(),
            version: ver,
            port,
            status: "running".into(),
            install_path: "/usr/bin/mysql".into(),
            data_dir: "/var/lib/mysql".into(),
            config_file: "/etc/mysql/mysql.conf.d/mysqld.cnf".into(),
            root_user: "root".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let id = self.repo.create(&instance).await?;
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to create database instance"))
    }

    pub async fn install_redis(
        &self,
        version: Option<&str>,
        port: i32,
        password: Option<&str>,
        name: &str,
    ) -> Result<DatabaseInstance, AppError> {
        self.redis_manager
            .install(version, port, password.unwrap_or(""))
            .await?;
        let ver = self
            .redis_manager
            .get_version()
            .await
            .unwrap_or_else(|_| "latest".into());
        let instance = DatabaseInstance {
            id: 0,
            db_type: "redis".into(),
            name: name.into(),
            version: ver,
            port,
            status: "running".into(),
            install_path: "/usr/bin/redis-server".into(),
            data_dir: "/var/lib/redis".into(),
            config_file: "/etc/redis/redis.conf".into(),
            root_user: "".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let id = self.repo.create(&instance).await?;
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to create database instance"))
    }

    pub async fn start(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.start().await,
            "redis" => self.redis_manager.start().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.update_status(id, "running").await
    }

    pub async fn stop(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.stop().await,
            "redis" => self.redis_manager.stop().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.update_status(id, "stopped").await
    }

    pub async fn restart(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.restart().await,
            "redis" => self.redis_manager.restart().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }
    }

    pub async fn status(&self, id: i64) -> Result<String, AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => {
                if self.mysql_manager.is_running().await? {
                    Ok("running".into())
                } else {
                    Ok("stopped".into())
                }
            }
            "redis" => {
                if self.redis_manager.is_running().await? {
                    Ok("running".into())
                } else {
                    Ok("stopped".into())
                }
            }
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }
    }

    pub async fn create_database(
        &self,
        instance_id: i64,
        db_name: &str,
        charset: &str,
    ) -> Result<(), AppError> {
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.create_database(db_name, charset).await,
            t => Err(AppError::BadRequest(format!(
                "Database creation not supported for: {}",
                t
            ))),
        }
    }

    pub async fn drop_database(&self, instance_id: i64, db_name: &str) -> Result<(), AppError> {
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.drop_database(db_name).await,
            t => Err(AppError::BadRequest(format!(
                "Database drop not supported for: {}",
                t
            ))),
        }
    }

    pub async fn list_databases(&self, instance_id: i64) -> Result<Vec<String>, AppError> {
        let inst = self.get_instance(instance_id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.list_databases().await,
            t => Err(AppError::BadRequest(format!(
                "Database listing not supported for: {}",
                t
            ))),
        }
    }

    pub async fn create_user(
        &self,
        instance_id: i64,
        username: &str,
        password: &str,
        host: &str,
    ) -> Result<(), AppError> {
        let _inst = self.get_instance(instance_id).await?;
        self.mysql_manager
            .create_user(username, password, host)
            .await
    }

    pub async fn drop_user(
        &self,
        instance_id: i64,
        username: &str,
        host: &str,
    ) -> Result<(), AppError> {
        let _inst = self.get_instance(instance_id).await?;
        self.mysql_manager.drop_user(username, host).await
    }

    pub async fn uninstall(&self, id: i64) -> Result<(), AppError> {
        let inst = self.get_instance(id).await?;
        match inst.db_type.as_str() {
            "mysql" | "mariadb" => self.mysql_manager.uninstall().await,
            "redis" => self.redis_manager.uninstall().await,
            t => Err(AppError::BadRequest(format!(
                "Unknown database type: {}",
                t
            ))),
        }?;
        self.repo.delete(id).await
    }
}

pub struct OperationLogService {
    pub log_repo: Arc<dyn OperationLogRepository>,
}

impl OperationLogService {
    pub fn new(log_repo: Arc<dyn OperationLogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn log(
        &self,
        username: &str,
        action: &str,
        target: Option<&str>,
        ip: Option<&str>,
    ) -> Result<OperationLog, AppError> {
        self.log_repo.create(username, action, target, ip).await
    }

    pub async fn list(&self) -> Result<Vec<OperationLog>, AppError> {
        self.log_repo.list().await
    }

    pub async fn list_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<OperationLog>, AppError> {
        let logs = self.log_repo.list().await?;
        let total = logs.len() as i64;
        let data = paginate_slice(&logs, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError> {
        self.log_repo.find_by_id(id).await
    }

    pub async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError> {
        self.log_repo.list_by_username(username).await
    }

    pub async fn delete_log(&self, id: i64) -> Result<(), AppError> {
        self.log_repo.delete(id).await
    }
}

pub struct LogService {
    pub log_repo: Arc<dyn LogRepository>,
}

impl LogService {
    pub fn new(log_repo: Arc<dyn LogRepository>) -> Self {
        Self { log_repo }
    }

    pub async fn log(
        &self,
        source: &str,
        level: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<LogEntry, AppError> {
        self.log_repo.create(source, level, message, metadata).await
    }

    pub async fn list(&self) -> Result<Vec<LogEntry>, AppError> {
        self.log_repo.list().await
    }

    pub async fn list_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<LogEntry>, AppError> {
        let logs = self.log_repo.list().await?;
        let total = logs.len() as i64;
        let data = paginate_slice(&logs, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError> {
        self.log_repo.find_by_id(id).await
    }

    pub async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError> {
        self.log_repo.list_by_source(source).await
    }

    pub async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError> {
        self.log_repo.list_by_level(level).await
    }

    pub async fn delete_log(&self, id: i64) -> Result<(), AppError> {
        self.log_repo.delete(id).await
    }
}

#[derive(Debug)]
pub enum FirewallBackend {
    Ufw,
    Firewalld,
    Iptables,
    Unsupported(String),
}

pub struct FirewallManager;

impl FirewallManager {
    pub async fn detect_backend() -> FirewallBackend {
        async fn check(cmd: &'static str) -> bool {
            tokio::process::Command::new("which")
                .arg(cmd)
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        if check("ufw").await {
            FirewallBackend::Ufw
        } else if check("firewall-cmd").await {
            FirewallBackend::Firewalld
        } else if check("iptables").await {
            FirewallBackend::Iptables
        } else {
            FirewallBackend::Unsupported("no firewall tool found".into())
        }
    }

    pub async fn get_status() -> Result<String, AppError> {
        let backend = Self::detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let out = tokio::process::Command::new("ufw")
                    .arg("status")
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("ufw status failed: {}", e)))?;
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            }
            FirewallBackend::Firewalld => {
                let out = tokio::process::Command::new("firewall-cmd")
                    .args(["--state"])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("firewall-cmd state failed: {}", e)))?;
                let running = out.status.success();
                Ok(if running {
                    "running".into()
                } else {
                    "stopped".into()
                })
            }
            FirewallBackend::Iptables => {
                let out = tokio::process::Command::new("iptables")
                    .args(["-L", "-n", "--line-numbers"])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("iptables failed: {}", e)))?;
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn apply_rule(rule: &FirewallRule) -> Result<(), AppError> {
        let backend = Self::detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let mut args = vec!["ufw".to_string()];
                if !rule.enabled {
                    // Delete equivalent
                    args.push("delete".into());
                }
                args.push(match rule.action.as_str() {
                    "allow" => "allow".into(),
                    "deny" => "deny".into(),
                    "reject" => "reject".into(),
                    _ => "allow".into(),
                });
                if let Some(ref port) = rule.port {
                    if rule.protocol != "any" && rule.protocol != "icmp" {
                        args.push(format!("{}/{}", port, rule.protocol));
                    } else {
                        args.push(port.clone());
                    }
                }
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        args.push("from".into());
                        args.push(src.clone());
                    }
                }
                let out = tokio::process::Command::new(&args[0])
                    .args(&args[1..])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("ufw apply failed: {}", e)))?;
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(AppError::internal(format!("ufw error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Firewalld => {
                let action = match rule.action.as_str() {
                    "allow" => "add",
                    "deny" | "reject" => "remove",
                    _ => "add",
                };
                let proto = if rule.protocol == "any" {
                    "tcp"
                } else {
                    &rule.protocol
                };
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        let rich = format!("rule family=\"ipv4\" source address=\"{}\" port port=\"{}\" protocol=\"{}\" {}", 
                            src, rule.port.as_deref().unwrap_or(""), proto, action);
                        let out = tokio::process::Command::new("firewall-cmd")
                            .args(["--permanent", &format!("--add-rich-rule={}", rich)])
                            .output()
                            .await
                            .map_err(|e| {
                                AppError::internal(format!("firewalld rich rule failed: {}", e))
                            })?;
                        if !out.status.success() {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            return Err(AppError::internal(format!("firewalld error: {}", stderr)));
                        }
                        tokio::process::Command::new("firewall-cmd")
                            .arg("--reload")
                            .output()
                            .await
                            .map_err(|e| {
                                AppError::internal(format!("firewalld reload failed: {}", e))
                            })?;
                        return Ok(());
                    }
                }
                let out = tokio::process::Command::new("firewall-cmd")
                    .args([
                        "--permanent",
                        &format!(
                            "--{}-port={}/{}",
                            action,
                            rule.port.as_deref().unwrap_or(""),
                            proto
                        ),
                    ])
                    .output()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("firewalld port rule failed: {}", e))
                    })?;
                if !out.status.success() {
                    // Try without protocol
                    let out2 = tokio::process::Command::new("firewall-cmd")
                        .args([
                            "--permanent",
                            &format!("--{}-port={}", action, rule.port.as_deref().unwrap_or("")),
                        ])
                        .output()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("firewalld port rule failed: {}", e))
                        })?;
                    if !out2.status.success() {
                        let stderr = String::from_utf8_lossy(&out2.stderr);
                        return Err(AppError::internal(format!("firewalld error: {}", stderr)));
                    }
                }
                tokio::process::Command::new("firewall-cmd")
                    .arg("--reload")
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("firewalld reload failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Iptables => {
                let chain = if rule.direction == "in" {
                    "INPUT"
                } else {
                    "OUTPUT"
                };
                let action = match rule.action.as_str() {
                    "allow" => "ACCEPT",
                    "deny" => "DROP",
                    "reject" => "REJECT",
                    _ => "ACCEPT",
                };
                let mut args = vec!["iptables".to_string(), "-A".into(), chain.into()];
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        args.push("-s".into());
                        args.push(src.clone());
                    }
                }
                if rule.protocol != "any" {
                    args.push("-p".into());
                    args.push(rule.protocol.clone());
                }
                if let Some(ref port) = rule.port {
                    args.push("--dport".into());
                    args.push(port.clone());
                }
                if !rule.enabled {
                    // For disabled rules, insert at the end with comment
                    args.push("-m".into());
                    args.push("comment".into());
                    args.push("--comment".into());
                    args.push(format!("disabled:{}", rule.name));
                }
                args.push("-j".into());
                args.push(action.into());

                let out = tokio::process::Command::new("iptables")
                    .args(args.clone())
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("iptables failed: {}", e)))?;
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(AppError::internal(format!("iptables error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn remove_rule(rule: &FirewallRule) -> Result<(), AppError> {
        let backend = Self::detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                let out = tokio::process::Command::new("ufw")
                    .args(["delete", &rule.action, rule.port.as_deref().unwrap_or("")])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("ufw delete failed: {}", e)))?;
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(AppError::internal(format!("ufw error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Firewalld => {
                let action = match rule.action.as_str() {
                    "allow" => "remove",
                    "deny" | "reject" => "add",
                    _ => "remove",
                };
                let proto = if rule.protocol == "any" {
                    "tcp"
                } else {
                    &rule.protocol
                };
                let out = tokio::process::Command::new("firewall-cmd")
                    .args([
                        "--permanent",
                        &format!(
                            "--{}-port={}/{}",
                            action,
                            rule.port.as_deref().unwrap_or(""),
                            proto
                        ),
                    ])
                    .output()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("firewalld remove port failed: {}", e))
                    })?;
                tokio::process::Command::new("firewall-cmd")
                    .arg("--reload")
                    .output()
                    .await
                    .ok();
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(AppError::internal(format!("firewalld error: {}", stderr)));
                }
                Ok(())
            }
            FirewallBackend::Iptables => {
                let chain = if rule.direction == "in" {
                    "INPUT"
                } else {
                    "OUTPUT"
                };
                let mut args = vec!["iptables".to_string(), "-D".into(), chain.into()];
                if let Some(ref src) = rule.source {
                    if src != "0.0.0.0/0" && src != "any" {
                        args.push("-s".into());
                        args.push(src.clone());
                    }
                }
                if rule.protocol != "any" {
                    args.push("-p".into());
                    args.push(rule.protocol.clone());
                }
                if let Some(ref port) = rule.port {
                    args.push("--dport".into());
                    args.push(port.clone());
                }
                let action = match rule.action.as_str() {
                    "allow" => "ACCEPT",
                    "deny" => "DROP",
                    "reject" => "REJECT",
                    _ => "ACCEPT",
                };
                args.push("-j".into());
                args.push(action.into());
                tokio::process::Command::new("iptables")
                    .args(&args)
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("iptables delete failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn enable_firewall() -> Result<(), AppError> {
        let backend = Self::detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                tokio::process::Command::new("ufw")
                    .arg("--force")
                    .arg("enable")
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("ufw enable failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Firewalld => {
                tokio::process::Command::new("systemctl")
                    .args(["start", "firewalld"])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("firewalld start failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Iptables => Ok(()), // iptables is always active
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }

    pub async fn disable_firewall() -> Result<(), AppError> {
        let backend = Self::detect_backend().await;
        match backend {
            FirewallBackend::Ufw => {
                tokio::process::Command::new("ufw")
                    .arg("disable")
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("ufw disable failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Firewalld => {
                tokio::process::Command::new("systemctl")
                    .args(["stop", "firewalld"])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("firewalld stop failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Iptables => {
                // Flush all rules
                tokio::process::Command::new("iptables")
                    .args(["-F"])
                    .output()
                    .await
                    .map_err(|e| AppError::internal(format!("iptables flush failed: {}", e)))?;
                Ok(())
            }
            FirewallBackend::Unsupported(ref msg) => {
                Err(AppError::internal(format!("Unsupported firewall: {}", msg)))
            }
        }
    }
}

pub struct FirewallService {
    pub firewall_repo: Arc<dyn FirewallRepository>,
}

impl FirewallService {
    pub fn new(firewall_repo: Arc<dyn FirewallRepository>) -> Self {
        Self { firewall_repo }
    }

    pub async fn list_rules(&self) -> Result<Vec<FirewallRule>, AppError> {
        self.firewall_repo.list_all().await
    }

    pub async fn list_rules_paginated(
        &self,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<FirewallRule>, AppError> {
        let rules = self.firewall_repo.list_all().await?;
        let total = rules.len() as i64;
        let data = paginate_slice(&rules, params);
        Ok(PaginatedResponse::new(data, total, params))
    }

    pub async fn get_rule(&self, id: i64) -> Result<FirewallRule, AppError> {
        self.firewall_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))
    }

    pub async fn create_rule(&self, mut rule: FirewallRule) -> Result<FirewallRule, AppError> {
        let id = self.firewall_repo.create(&rule).await?;
        rule.id = id;
        // Apply to OS if enabled
        if rule.enabled {
            FirewallManager::apply_rule(&rule).await.ok();
        }
        Ok(rule)
    }

    pub async fn update_rule(&self, rule: FirewallRule) -> Result<FirewallRule, AppError> {
        // Get old rule to remove OS rule if changed
        let old = self
            .firewall_repo
            .find_by_id(rule.id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))?;

        self.firewall_repo.update(&rule).await?;

        // Remove old OS rule and apply new one
        FirewallManager::remove_rule(&old).await.ok();
        if rule.enabled {
            FirewallManager::apply_rule(&rule).await.ok();
        }
        Ok(rule)
    }

    pub async fn delete_rule(&self, id: i64) -> Result<(), AppError> {
        if let Some(rule) = self.firewall_repo.find_by_id(id).await? {
            FirewallManager::remove_rule(&rule).await.ok();
        }
        self.firewall_repo.delete(id).await
    }

    pub async fn toggle_rule(&self, id: i64, enabled: bool) -> Result<FirewallRule, AppError> {
        let rule = self
            .firewall_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))?;
        self.firewall_repo.update_enabled(id, enabled).await?;
        if enabled {
            FirewallManager::apply_rule(&rule).await.ok();
        } else {
            FirewallManager::remove_rule(&rule).await.ok();
        }
        self.firewall_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Firewall rule not found".into()))
    }

    pub async fn apply_all_rules(&self) -> Result<(), AppError> {
        let rules = self.firewall_repo.list_all().await?;
        for rule in &rules {
            if rule.enabled {
                FirewallManager::apply_rule(rule).await.ok();
            }
        }
        Ok(())
    }

    pub async fn get_backend_status(&self) -> Result<HashMap<String, String>, AppError> {
        let backend = FirewallManager::detect_backend().await;
        let mut info = HashMap::new();
        info.insert("backend".to_string(), format!("{:?}", backend));
        match FirewallManager::get_status().await {
            Ok(s) => {
                info.insert("status".to_string(), s);
            }
            Err(_) => {
                info.insert("status".to_string(), "unknown".into());
            }
        }
        info.insert(
            "backend_name".to_string(),
            match backend {
                FirewallBackend::Ufw => "ufw".into(),
                FirewallBackend::Firewalld => "firewalld".into(),
                FirewallBackend::Iptables => "iptables".into(),
                FirewallBackend::Unsupported(m) => m,
            },
        );
        Ok(info)
    }

    pub async fn enable_firewall(&self) -> Result<(), AppError> {
        FirewallManager::enable_firewall().await
    }

    pub async fn disable_firewall(&self) -> Result<(), AppError> {
        // Remove all enabled OS rules first
        let rules = self.firewall_repo.list_all().await?;
        for rule in &rules {
            if rule.enabled {
                FirewallManager::remove_rule(rule).await.ok();
            }
        }
        FirewallManager::disable_firewall().await
    }

    pub async fn reorder_rules(&self, ids: &[i64]) -> Result<(), AppError> {
        self.firewall_repo.reorder(ids).await
    }
}
