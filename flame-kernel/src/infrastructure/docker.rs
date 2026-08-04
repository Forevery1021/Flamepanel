use crate::core::error::AppError;
use crate::domain::entity::DockerContainer;
use crate::domain::repository::DockerRepository;
use async_trait::async_trait;
use bollard::container::{ListContainersOptions, StartContainerOptions, StopContainerOptions};
use bollard::Docker;
use chrono::Utc;
use std::fs;
use std::process::Command;

pub struct BollardDockerRepository {
    docker: Docker,
}

impl BollardDockerRepository {
    pub fn new() -> Result<Self, AppError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| AppError::internal(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { docker })
    }

    pub fn connect_with_env() -> Result<Self, AppError> {
        let docker = Docker::connect_with_defaults()
            .map_err(|e| AppError::internal(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { docker })
    }

    pub fn new_with_connection(docker: Docker) -> Result<Self, AppError> {
        Ok(Self { docker })
    }
}

#[async_trait]
impl DockerRepository for BollardDockerRepository {
    async fn list_containers(&self, _node_id: i64) -> Result<Vec<DockerContainer>, AppError> {
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker list error: {}", e)))?;

        let result = containers
            .into_iter()
            .map(|c| {
                let names = c.names.unwrap_or_default();
                let name = names
                    .first()
                    .map(|n| n.trim_start_matches('/').to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                DockerContainer {
                    id: c.id.unwrap_or_default(),
                    image: c.image.unwrap_or_default(),
                    name,
                    status: c.state.unwrap_or_default(),
                    node_id: _node_id,
                    created_at: Utc::now(),
                }
            })
            .collect();
        Ok(result)
    }

    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError> {
        use bollard::container::InspectContainerOptions;
        let result = self
            .docker
            .inspect_container(id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| AppError::internal(format!("Docker inspect error: {}", e)))?;

        let container = DockerContainer {
            id: id.to_string(),
            image: result.image.unwrap_or_default(),
            name: result
                .name
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string(),
            status: result
                .state
                .as_ref()
                .and_then(|s| s.status.as_ref().map(|s| s.to_string()))
                .unwrap_or_default(),
            node_id: 0,
            created_at: Utc::now(),
        };
        Ok(Some(container))
    }

    async fn start_container(&self, id: &str) -> Result<(), AppError> {
        self.docker
            .start_container::<String>(id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker start error: {}", e)))?;
        Ok(())
    }

    async fn stop_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        let options = StopContainerOptions { t: timeout as i64 };
        self.docker
            .stop_container(id, Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker stop error: {}", e)))?;
        Ok(())
    }

    async fn restart_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        self.docker
            .stop_container(id, Some(StopContainerOptions { t: timeout as i64 }))
            .await
            .map_err(|e| AppError::internal(format!("Docker stop error: {}", e)))?;
        self.docker
            .start_container::<String>(id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker start error: {}", e)))?;
        Ok(())
    }

    async fn remove_container(&self, id: &str, force: bool) -> Result<(), AppError> {
        use bollard::container::RemoveContainerOptions;
        let options = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        self.docker
            .remove_container(id, Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker remove error: {}", e)))?;
        Ok(())
    }

    async fn get_container_logs(&self, id: &str, tail: usize) -> Result<String, AppError> {
        use bollard::container::LogsOptions;
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            ..Default::default()
        };
        let logs = self.docker.logs(id, Some(options));

        let mut output = String::new();
        use futures_util::StreamExt;
        let mut stream = logs;
        while let Some(log_result) = stream.next().await {
            if let Ok(log) = log_result {
                output.push_str(&log.to_string());
            }
        }
        Ok(output)
    }

    async fn get_container_stats(&self, id: &str) -> Result<serde_json::Value, AppError> {
        use bollard::container::StatsOptions;
        use futures_util::StreamExt;
        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };
        let mut stats_stream = self.docker.stats(id, Some(options));

        if let Some(stats_result) = stats_stream.next().await {
            let stats = stats_result
                .map_err(|e| AppError::internal(format!("Docker stats error: {}", e)))?;
            Ok(serde_json::json!({
                "cpu_stats": stats.cpu_stats,
                "memory_stats": stats.memory_stats,
                "networks": stats.networks,
                "pids_stats": stats.pids_stats,
            }))
        } else {
            Err(AppError::internal("No stats available"))
        }
    }

    async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError> {
        use bollard::image::ListImagesOptions;
        let options = ListImagesOptions::<String> {
            all: true,
            ..Default::default()
        };
        let images = self
            .docker
            .list_images(Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker images error: {}", e)))?;

        Ok(images
            .into_iter()
            .map(|img| {
                serde_json::json!({
                    "id": img.id,
                    "tags": img.repo_tags,
                    "size": img.size,
                    "created": img.created,
                })
            })
            .collect())
    }

    async fn remove_image(&self, id: &str) -> Result<(), AppError> {
        use bollard::image::RemoveImageOptions;
        let options = RemoveImageOptions {
            force: true,
            ..Default::default()
        };
        self.docker
            .remove_image(id, Some(options), None)
            .await
            .map_err(|e| AppError::internal(format!("Docker remove image error: {}", e)))?;
        Ok(())
    }

    async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError> {
        let tmp_dir = std::env::temp_dir().join(format!("flame_compose_{}", project_name));
        fs::create_dir_all(&tmp_dir)
            .map_err(|e| AppError::internal(format!("Failed to create temp dir: {}", e)))?;
        let compose_path = tmp_dir.join("docker-compose.yml");
        fs::write(&compose_path, compose_yaml)
            .map_err(|e| AppError::internal(format!("Failed to write compose file: {}", e)))?;
        let output = Command::new("docker")
            .args([
                "compose",
                "-p",
                project_name,
                "-f",
                &compose_path.to_string_lossy(),
                "up",
                "-d",
            ])
            .output()
            .map_err(|e| AppError::internal(format!("Failed to execute docker compose: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::internal(format!(
                "Docker compose up failed: {}",
                stderr
            )));
        }
        Ok(serde_json::json!({
            "project_name": project_name,
            "path": compose_path.to_string_lossy().to_string(),
            "status": "deployed"
        }))
    }

    async fn compose_up(&self, project_name: &str) -> Result<(), AppError> {
        let output = Command::new("docker")
            .args(["compose", "-p", project_name, "up", "-d"])
            .output()
            .map_err(|e| AppError::internal(format!("Failed to execute docker compose: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::internal(format!(
                "Docker compose up failed: {}",
                stderr
            )));
        }
        Ok(())
    }

    async fn compose_down(&self, project_name: &str) -> Result<(), AppError> {
        let output = Command::new("docker")
            .args(["compose", "-p", project_name, "down"])
            .output()
            .map_err(|e| AppError::internal(format!("Failed to execute docker compose: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::internal(format!(
                "Docker compose down failed: {}",
                stderr
            )));
        }
        Ok(())
    }

    // ── 容器高级操作 ─────────────────────────────────────────────

    async fn inspect_container(&self, id: &str) -> Result<serde_json::Value, AppError> {
        use bollard::container::InspectContainerOptions;
        let info = self
            .docker
            .inspect_container(id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| AppError::internal(format!("Docker inspect error: {}", e)))?;
        serde_json::to_value(&info)
            .map_err(|e| AppError::internal(format!("Failed to serialize inspect result: {}", e)))
    }

    async fn rename_container(&self, id: &str, new_name: &str) -> Result<(), AppError> {
        use bollard::container::RenameContainerOptions;
        let options = RenameContainerOptions { name: new_name };
        self.docker
            .rename_container(id, options)
            .await
            .map_err(|e| AppError::internal(format!("Docker rename error: {}", e)))?;
        Ok(())
    }

    async fn pause_container(&self, id: &str) -> Result<(), AppError> {
        self.docker
            .pause_container(id)
            .await
            .map_err(|e| AppError::internal(format!("Docker pause error: {}", e)))?;
        Ok(())
    }

    async fn unpause_container(&self, id: &str) -> Result<(), AppError> {
        self.docker
            .unpause_container(id)
            .await
            .map_err(|e| AppError::internal(format!("Docker unpause error: {}", e)))?;
        Ok(())
    }

    async fn kill_container(&self, id: &str) -> Result<(), AppError> {
        use bollard::container::KillContainerOptions;
        let options = KillContainerOptions::<String> {
            signal: "SIGKILL".into(),
        };
        self.docker
            .kill_container(id, Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker kill error: {}", e)))?;
        Ok(())
    }

    async fn prune_containers(&self) -> Result<serde_json::Value, AppError> {
        use bollard::container::PruneContainersOptions;
        let result = self
            .docker
            .prune_containers(None::<PruneContainersOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker prune containers error: {}", e)))?;
        Ok(serde_json::json!({
            "containers_deleted": result.containers_deleted,
            "space_reclaimed": result.space_reclaimed,
        }))
    }

    // ── 网络管理 ─────────────────────────────────────────────────

    async fn list_networks(&self) -> Result<Vec<serde_json::Value>, AppError> {
        use bollard::network::ListNetworksOptions;
        let networks = self
            .docker
            .list_networks(None::<ListNetworksOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker list networks error: {}", e)))?;
        Ok(networks
            .into_iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "name": n.name,
                    "driver": n.driver,
                    "scope": n.scope,
                    "internal": n.internal,
                    "attachable": n.attachable,
                    "ipam": n.ipam,
                    "containers": n.containers.map(|c| {
                        c.into_iter().map(|(k, v)| {
                            serde_json::json!({ "name": k, "ipv4_address": v.ipv4_address, "ipv6_address": v.ipv6_address })
                        }).collect::<Vec<_>>()
                    }),
                    "created": n.created,
                })
            })
            .collect())
    }

    async fn create_network(
        &self,
        name: &str,
        driver: &str,
        subnet: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        use bollard::models::{Ipam, IpamConfig};
        use bollard::network::CreateNetworkOptions;
        let mut ipam = Ipam {
            driver: Some(driver.to_string()),
            ..Default::default()
        };
        if let Some(sub) = subnet {
            if !sub.is_empty() {
                ipam.config = Some(vec![IpamConfig {
                    subnet: Some(sub.to_string()),
                    ..Default::default()
                }]);
            }
        }
        let options = CreateNetworkOptions::<String> {
            name: name.into(),
            driver: driver.into(),
            check_duplicate: true,
            ipam,
            internal: false,
            attachable: false,
            ingress: false,
            enable_ipv6: false,
            options: Default::default(),
            labels: Default::default(),
        };
        let net = self
            .docker
            .create_network(options)
            .await
            .map_err(|e| AppError::internal(format!("Docker create network error: {}", e)))?;
        Ok(serde_json::json!({
            "id": net.id,
            "name": name,
            "warning": net.warning,
        }))
    }

    async fn remove_network(&self, id: &str) -> Result<(), AppError> {
        self.docker
            .remove_network(id)
            .await
            .map_err(|e| AppError::internal(format!("Docker remove network error: {}", e)))?;
        Ok(())
    }

    async fn connect_network(&self, network_id: &str, container_id: &str) -> Result<(), AppError> {
        use bollard::models::EndpointSettings;
        use bollard::network::ConnectNetworkOptions;
        let options = ConnectNetworkOptions::<String> {
            container: container_id.into(),
            endpoint_config: EndpointSettings::default(),
        };
        self.docker
            .connect_network(network_id, options)
            .await
            .map_err(|e| AppError::internal(format!("Docker connect network error: {}", e)))?;
        Ok(())
    }

    async fn disconnect_network(
        &self,
        network_id: &str,
        container_id: &str,
        force: bool,
    ) -> Result<(), AppError> {
        use bollard::network::DisconnectNetworkOptions;
        let options = DisconnectNetworkOptions::<String> {
            container: container_id.into(),
            force,
        };
        self.docker
            .disconnect_network(network_id, options)
            .await
            .map_err(|e| AppError::internal(format!("Docker disconnect network error: {}", e)))?;
        Ok(())
    }

    async fn prune_networks(&self) -> Result<serde_json::Value, AppError> {
        use bollard::network::PruneNetworksOptions;
        let result = self
            .docker
            .prune_networks(None::<PruneNetworksOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker prune networks error: {}", e)))?;
        Ok(serde_json::json!({
            "networks_deleted": result.networks_deleted,
        }))
    }

    // ── 卷管理 ──────────────────────────────────────────────────

    async fn list_volumes(&self) -> Result<Vec<serde_json::Value>, AppError> {
        use bollard::volume::ListVolumesOptions;
        let response = self
            .docker
            .list_volumes(None::<ListVolumesOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker list volumes error: {}", e)))?;
        Ok(response
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| {
                serde_json::json!({
                    "name": v.name,
                    "driver": v.driver,
                    "mountpoint": v.mountpoint,
                    "created_at": v.created_at,
                    "scope": v.scope,
                    "labels": v.labels,
                    "options": v.options,
                })
            })
            .collect())
    }

    async fn create_volume(&self, name: &str, driver: &str) -> Result<serde_json::Value, AppError> {
        use bollard::volume::CreateVolumeOptions;
        let options = CreateVolumeOptions::<String> {
            name: name.into(),
            driver: driver.into(),
            ..Default::default()
        };
        let vol = self
            .docker
            .create_volume(options)
            .await
            .map_err(|e| AppError::internal(format!("Docker create volume error: {}", e)))?;
        Ok(serde_json::json!({
            "name": vol.name,
            "driver": vol.driver,
            "mountpoint": vol.mountpoint,
        }))
    }

    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), AppError> {
        use bollard::volume::RemoveVolumeOptions;
        let options = RemoveVolumeOptions { force };
        self.docker
            .remove_volume(name, Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker remove volume error: {}", e)))?;
        Ok(())
    }

    async fn prune_volumes(&self) -> Result<serde_json::Value, AppError> {
        use bollard::volume::PruneVolumesOptions;
        let result = self
            .docker
            .prune_volumes(None::<PruneVolumesOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker prune volumes error: {}", e)))?;
        Ok(serde_json::json!({
            "volumes_deleted": result.volumes_deleted,
            "space_reclaimed": result.space_reclaimed,
        }))
    }

    // ── 镜像管理 ────────────────────────────────────────────────

    async fn pull_image(&self, image: &str) -> Result<String, AppError> {
        use bollard::image::CreateImageOptions;
        use futures_util::StreamExt;

        let (from_image, tag) = split_image_tag(image);
        let options = CreateImageOptions {
            from_image,
            tag,
            ..Default::default()
        };
        let mut stream = self.docker.create_image(Some(options), None, None);
        let mut last_status = String::new();
        let mut last_error: Option<String> = None;
        while let Some(item) = stream.next().await {
            match item {
                Ok(info) => {
                    if let Some(status) = info.status {
                        last_status = status;
                    }
                    if let Some(err) = info.error {
                        last_error = Some(err);
                    }
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }
        }
        if let Some(err) = last_error {
            return Err(AppError::internal(format!(
                "Docker pull '{}' failed: {}",
                image, err
            )));
        }
        Ok(if last_status.is_empty() {
            format!("Image {} pulled", image)
        } else {
            last_status
        })
    }

    async fn tag_image(&self, image_id: &str, repo: &str, tag: &str) -> Result<(), AppError> {
        use bollard::image::TagImageOptions;
        let options = TagImageOptions::<String> {
            repo: repo.into(),
            tag: tag.into(),
        };
        self.docker
            .tag_image(image_id, Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker tag image error: {}", e)))?;
        Ok(())
    }

    async fn prune_images(&self) -> Result<serde_json::Value, AppError> {
        use bollard::image::PruneImagesOptions;
        let result = self
            .docker
            .prune_images(None::<PruneImagesOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker prune images error: {}", e)))?;
        Ok(serde_json::json!({
            "images_deleted": result.images_deleted,
            "space_reclaimed": result.space_reclaimed,
        }))
    }

    // ── Compose 项目列表 ────────────────────────────────────────

    async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError> {
        let output = Command::new("docker")
            .args(["compose", "ls", "--format", "json"])
            .output()
            .map_err(|e| {
                AppError::internal(format!("Failed to execute docker compose ls: {}", e))
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::internal(format!(
                "Docker compose ls failed: {}",
                stderr
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut projects = Vec::new();
        for line in stdout.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                projects.push(serde_json::json!({
                    "name": v.get("Name").or_else(|| v.get("name")),
                    "status": v.get("Status").or_else(|| v.get("status")),
                    "config_files": v.get("ConfigFiles").or_else(|| v.get("configFiles")),
                }));
            }
        }
        Ok(projects)
    }
}

fn split_image_tag(image: &str) -> (String, String) {
    // 支持 name:tag / registry/name:tag；digest(@) 不在本次范围
    match image.rfind(':') {
        Some(idx) => {
            let tag_part = &image[idx + 1..];
            if tag_part.contains('/') {
                (image.to_string(), String::new())
            } else {
                (image[..idx].to_string(), tag_part.to_string())
            }
        }
        None => (image.to_string(), String::new()),
    }
}
