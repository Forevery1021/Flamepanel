use crate::application::execution_mode::{CommandOutput, PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use crate::domain::entity::DockerContainer;
use crate::domain::repository::*;
use async_trait::async_trait;
use bollard::container::{ListContainersOptions, StartContainerOptions, StopContainerOptions};
use bollard::Docker;
use chrono::Utc;
use tokio::fs;

pub struct BollardDockerRepository {
    docker: Docker,
    runner: SharedCommandRunner,
}

impl BollardDockerRepository {
    /// Docker API 请求超时：避免 bollard 默认 120s 长超时导致 handler 长时间占用（S6）。
    const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
    /// F-15: 容器日志返回大小上限（1MB），防止失控容器日志打爆面板内存。
    const MAX_LOG_BYTES: usize = 1024 * 1024;

    pub fn new() -> Result<Self, AppError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| AppError::internal(format!("Failed to connect to Docker: {}", e)))?
            .with_timeout(Self::REQUEST_TIMEOUT);
        Ok(Self {
            docker,
            runner: Self::embedded_runner(),
        })
    }

    fn embedded_runner() -> SharedCommandRunner {
        std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner)
    }

    pub fn connect_with_env() -> Result<Self, AppError> {
        let docker = Docker::connect_with_defaults()
            .map_err(|e| AppError::internal(format!("Failed to connect to Docker: {}", e)))?
            .with_timeout(Self::REQUEST_TIMEOUT);
        Ok(Self {
            docker,
            runner: Self::embedded_runner(),
        })
    }

    pub fn new_with_connection(docker: Docker) -> Result<Self, AppError> {
        Ok(Self {
            docker,
            runner: Self::embedded_runner(),
        })
    }

    /// 注入统一特权命令执行端口（`execution_mode=embedded|agent` 分离模式）。
    pub fn new_with_connection_and_runner(
        docker: Docker,
        runner: SharedCommandRunner,
    ) -> Result<Self, AppError> {
        Ok(Self { docker, runner })
    }
}

#[async_trait]
impl ContainerRepository for BollardDockerRepository {
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

    /// F-15: 容器日志限制 1MB 上限 + 30s 超时 + 错误不静默丢弃。
    async fn get_container_logs(&self, id: &str, tail: usize) -> Result<String, AppError> {
        use bollard::container::LogsOptions;
        use futures_util::StreamExt;
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            ..Default::default()
        };
        let logs = self.docker.logs(id, Some(options));

        let mut output = String::new();
        let mut stream = logs;
        // F-15: 包裹 30s 超时，防止 bollard 默认长超时导致 handler 长期占用
        let timeout = std::time::Duration::from_secs(30);
        let result = tokio::time::timeout(timeout, async {
            while let Some(log_result) = stream.next().await {
                match log_result {
                    Ok(log) => {
                        let s = log.to_string();
                        if output.len() + s.len() > Self::MAX_LOG_BYTES {
                            // 截断：只保留前 MAX_LOG_BYTES
                            let remaining = Self::MAX_LOG_BYTES - output.len();
                            if remaining > 0 {
                                output.push_str(&s[..s.len().min(remaining)]);
                            }
                            output.push_str("\n... (log truncated at 1MB)");
                            break;
                        }
                        output.push_str(&s);
                    }
                    Err(e) => {
                        // F-15: 不再静默丢弃错误——记录并继续
                        tracing::warn!("Container {} log stream error: {}", id, e);
                    }
                }
            }
        })
        .await;

        match result {
            Ok(_) => Ok(output),
            Err(_) => {
                // 超时：返回已收集的部分日志
                tracing::warn!("Container {} log fetch timed out after 30s", id);
                output.push_str("\n... (log fetch timed out)");
                Ok(output)
            }
        }
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
}

#[async_trait]
impl NetworkRepository for BollardDockerRepository {
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
}

#[async_trait]
impl VolumeRepository for BollardDockerRepository {
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
}

#[async_trait]
impl ImageRepository for BollardDockerRepository {
    async fn inspect_image(&self, image: &str) -> Result<serde_json::Value, AppError> {
        let info = self
            .docker
            .inspect_image(image)
            .await
            .map_err(|e| AppError::internal(format!("Docker image inspect error: {}", e)))?;
        serde_json::to_value(&info)
            .map_err(|e| AppError::internal(format!("Failed to serialize image inspect: {}", e)))
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
}

/// F-05: 项目名校验（实现已移至 feature 无关的 `infrastructure::naming`，此处保留再导出，
/// 令 `docker` 特性关闭时 `db.rs` / `project_builder.rs` 仍可编译）。
pub use crate::infrastructure::naming::sanitize_project_name;

#[async_trait]
impl ComposeRepository for BollardDockerRepository {
    /// 经统一特权命令执行端口运行 `docker compose ...`（`execution_mode=embedded|agent` 分离模式）。
    /// F-09: compose 操作（up/deploy）可能拉取数百 MB~数 GB 镜像，远超默认 60s。
    /// 显式放宽到 1800s（30min），保证大镜像在正常生产网络下能完成。
    async fn run_compose(&self, args: Vec<String>) -> Result<CommandOutput, AppError> {
        self.runner
            .run(&PrivilegedCommand::new("docker", args).timeout(1800))
            .await
    }

    async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError> {
        // F-05: 校验 project_name 防路径穿越（拒绝 ".." 等危险片段）。
        sanitize_project_name(project_name)?;
        let dir = compose_root().join(project_name);
        self.compose_deploy_at(project_name, compose_yaml, &dir)
            .await
    }

    /// 旧实现写入 `tempfile::TempDir` 并在 `up -d` 返回后立即删除：① 相对挂载的宿主目录
    /// 随之被删（容器仍在运行）；② `compose up -p <name>` 找不到配置文件，面板里对已部署
    /// 项目点「启动」必失败。改为持久项目目录（0700，compose 文件 0600——可能含渲染后的口令）。
    async fn compose_deploy_at(
        &self,
        project_name: &str,
        compose_yaml: &str,
        project_dir: &std::path::Path,
    ) -> Result<serde_json::Value, AppError> {
        sanitize_project_name(project_name)?;
        let dir = std::path::absolute(project_dir)
            .map_err(|e| AppError::internal(format!("Failed to resolve compose dir: {}", e)))?;
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create compose dir: {}", e)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
        let compose_path = dir.join("docker-compose.yml");
        let tmp_path = dir.join(".docker-compose.yml.tmp");
        fs::write(&tmp_path, compose_yaml)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write compose file: {}", e)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600));
        }
        fs::rename(&tmp_path, &compose_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write compose file: {}", e)))?;
        let output = self
            .run_compose(vec![
                "compose".into(),
                "-p".into(),
                project_name.into(),
                "--project-directory".into(),
                dir.to_string_lossy().to_string(),
                "-f".into(),
                compose_path.to_string_lossy().to_string(),
                "up".into(),
                "-d".into(),
            ])
            .await?;
        if !output.success() {
            let stderr = output.stderr;
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
        sanitize_project_name(project_name)?;
        let mut args: Vec<String> = vec!["compose".into(), "-p".into(), project_name.into()];
        // compose 在容器标签里记录了配置文件路径（`compose ls -a` 的 ConfigFiles）；
        // 仅 `-p` 时 compose 会在进程工作目录找配置文件，对面板部署的项目必失败。
        for file in self.project_config_files(project_name).await {
            args.push("-f".into());
            args.push(file);
        }
        args.push("up".into());
        args.push("-d".into());
        let output = self.run_compose(args).await?;
        if !output.success() {
            let stderr = output.stderr;
            return Err(AppError::internal(format!(
                "Docker compose up failed: {}",
                stderr
            )));
        }
        Ok(())
    }

    async fn compose_down(&self, project_name: &str) -> Result<(), AppError> {
        sanitize_project_name(project_name)?;
        let output = self
            .run_compose(vec![
                "compose".into(),
                "-p".into(),
                project_name.into(),
                "down".into(),
            ])
            .await?;
        if !output.success() {
            let stderr = output.stderr;
            return Err(AppError::internal(format!(
                "Docker compose down failed: {}",
                stderr
            )));
        }
        Ok(())
    }

    async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError> {
        let output = self
            .run_compose(vec![
                "compose".into(),
                "ls".into(),
                "-a".into(),
                "--format".into(),
                "json".into(),
            ])
            .await?;
        if !output.success() {
            let stderr = output.stderr;
            return Err(AppError::internal(format!(
                "Docker compose ls failed: {}",
                stderr
            )));
        }
        let projects = parse_compose_ls(&output.stdout);
        Ok(projects)
    }
}

#[async_trait]
impl DockerRepository for BollardDockerRepository {}

impl BollardDockerRepository {
    /// 项目记录的 compose 配置文件（仅返回仍存在的文件）；无记录时回落到面板默认项目目录。
    async fn project_config_files(&self, project_name: &str) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(projects) = self.compose_ls().await {
            if let Some(p) = projects
                .iter()
                .find(|p| p.get("name").and_then(|v| v.as_str()) == Some(project_name))
            {
                if let Some(list) = p.get("config_files").and_then(|v| v.as_str()) {
                    files.extend(
                        list.split(',')
                            .map(str::trim)
                            .filter(|f| !f.is_empty() && std::path::Path::new(f).is_file())
                            .map(String::from),
                    );
                }
            }
        }
        if files.is_empty() {
            let fallback = compose_root().join(project_name).join("docker-compose.yml");
            if fallback.is_file() {
                if let Ok(abs) = std::path::absolute(&fallback) {
                    files.push(abs.to_string_lossy().into_owned());
                }
            }
        }
        files
    }
}

/// 面板直接部署（`POST /api/docker/compose/deploy`）的项目目录根：`$FLAME_COMPOSE_DIR`，缺省
/// `data/compose`（与 `data/apps` 同级，随面板数据目录备份）。
pub fn compose_root() -> std::path::PathBuf {
    std::env::var_os("FLAME_COMPOSE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("data/compose"))
}

/// 解析 `docker compose ls --format json`：新版输出单个 JSON 数组，旧版逐行一个对象；两者都接受。
/// 旧实现逐行按对象取字段，遇到数组输出时得到一条全 null 的记录。
pub fn parse_compose_ls(stdout: &str) -> Vec<serde_json::Value> {
    let pick = |v: &serde_json::Value| {
        serde_json::json!({
            "name": v.get("Name").or_else(|| v.get("name")),
            "status": v.get("Status").or_else(|| v.get("status")),
            "config_files": v.get("ConfigFiles").or_else(|| v.get("configFiles")),
        })
    };
    let mut projects = Vec::new();
    for line in stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(serde_json::Value::Array(items)) => projects.extend(items.iter().map(pick)),
            Ok(v @ serde_json::Value::Object(_)) => projects.push(pick(&v)),
            _ => {}
        }
    }
    projects
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compose_ls_array_and_ndjson() {
        let arr = r#"[{"Name":"a","Status":"running(1)","ConfigFiles":"/x/a.yml"},{"Name":"b","Status":"exited(1)","ConfigFiles":"/y/b.yml"}]"#;
        let p = parse_compose_ls(arr);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0]["name"], "a");
        assert_eq!(p[1]["config_files"], "/y/b.yml");
        let nd = "{\"Name\":\"a\",\"Status\":\"running\"}\n{\"Name\":\"b\"}\n";
        let p = parse_compose_ls(nd);
        assert_eq!(p.len(), 2);
        assert_eq!(p[1]["name"], "b");
        assert!(parse_compose_ls("").is_empty());
        assert!(parse_compose_ls("[]").is_empty());
    }

    #[test]
    fn sanitize_project_name_valid_cases() {
        assert!(sanitize_project_name("my-app").is_ok());
        assert!(sanitize_project_name("my_app").is_ok());
        assert!(sanitize_project_name("my-app-1").is_ok());
        assert!(sanitize_project_name("App123").is_ok());
        assert!(sanitize_project_name("a").is_ok());
    }

    #[test]
    fn sanitize_project_name_rejects_path_traversal() {
        assert!(sanitize_project_name("../etc").is_err());
        assert!(sanitize_project_name("../../etc/cron.d").is_err());
        assert!(sanitize_project_name("a/../b").is_err());
        assert!(sanitize_project_name("a/../b").is_err());
        assert!(sanitize_project_name("...").is_err());
    }

    #[test]
    fn sanitize_project_name_rejects_invalid_chars() {
        assert!(sanitize_project_name("").is_err());
        assert!(sanitize_project_name("my app").is_err());
        assert!(sanitize_project_name("my/app").is_err());
        assert!(sanitize_project_name("my/app").is_err());
        assert!(sanitize_project_name("1starts-with-digit").is_ok()); // digit is ok for first char
        assert!(sanitize_project_name("!bad").is_err());
        assert!(sanitize_project_name("my$bad").is_err());
        assert!(sanitize_project_name("a".repeat(64).as_str()).is_err());
    }
}
