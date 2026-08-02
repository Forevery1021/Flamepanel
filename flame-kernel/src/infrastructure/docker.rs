use async_trait::async_trait;
use bollard::Docker;
use bollard::container::{
    ListContainersOptions, StartContainerOptions, StopContainerOptions,
};
use chrono::Utc;
use std::fs;
use std::process::Command;
use crate::domain::entity::DockerContainer;
use crate::domain::repository::DockerRepository;
use crate::core::error::AppError;

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
        let containers = self.docker.list_containers(Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker list error: {}", e)))?;

        let result = containers.into_iter().map(|c| {
            let names = c.names.unwrap_or_default();
            let name = names.first().map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| "unknown".to_string());
            DockerContainer {
                id: c.id.unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                name,
                status: c.state.unwrap_or_default(),
                node_id: _node_id,
                created_at: Utc::now(),
            }
        }).collect();
        Ok(result)
    }

    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError> {
        use bollard::container::InspectContainerOptions;
        let result = self.docker.inspect_container(id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| AppError::internal(format!("Docker inspect error: {}", e)))?;

        let container = DockerContainer {
            id: id.to_string(),
            image: result.image.unwrap_or_default(),
            name: result.name.unwrap_or_default().trim_start_matches('/').to_string(),
            status: result.state.as_ref()
                .and_then(|s| s.status.as_ref().map(|s| s.to_string()))
                .unwrap_or_default(),
            node_id: 0,
            created_at: Utc::now(),
        };
        Ok(Some(container))
    }

    async fn start_container(&self, id: &str) -> Result<(), AppError> {
        self.docker.start_container::<String>(id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker start error: {}", e)))?;
        Ok(())
    }

    async fn stop_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        let options = StopContainerOptions { t: timeout as i64 };
        self.docker.stop_container(id, Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker stop error: {}", e)))?;
        Ok(())
    }

    async fn restart_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        self.docker.stop_container(id, Some(StopContainerOptions { t: timeout as i64 }))
            .await
            .map_err(|e| AppError::internal(format!("Docker stop error: {}", e)))?;
        self.docker.start_container::<String>(id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::internal(format!("Docker start error: {}", e)))?;
        Ok(())
    }

    async fn remove_container(&self, id: &str, force: bool) -> Result<(), AppError> {
        use bollard::container::RemoveContainerOptions;
        let options = RemoveContainerOptions { force, ..Default::default() };
        self.docker.remove_container(id, Some(options))
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
        let options = StatsOptions { stream: false, one_shot: true };
        let mut stats_stream = self.docker.stats(id, Some(options));
        
        if let Some(stats_result) = stats_stream.next().await {
            let stats = stats_result.map_err(|e| AppError::internal(format!("Docker stats error: {}", e)))?;
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
        let images = self.docker.list_images(Some(options))
            .await
            .map_err(|e| AppError::internal(format!("Docker images error: {}", e)))?;
        
        Ok(images.into_iter().map(|img| {
            serde_json::json!({
                "id": img.id,
                "tags": img.repo_tags,
                "size": img.size,
                "created": img.created,
            })
        }).collect())
    }

    async fn remove_image(&self, id: &str) -> Result<(), AppError> {
        use bollard::image::RemoveImageOptions;
        let options = RemoveImageOptions { force: true, ..Default::default() };
        self.docker.remove_image(id, Some(options), None)
            .await
            .map_err(|e| AppError::internal(format!("Docker remove image error: {}", e)))?;
        Ok(())
    }

    async fn compose_deploy(&self, project_name: &str, compose_yaml: &str) -> Result<serde_json::Value, AppError> {
        let tmp_dir = std::env::temp_dir().join(format!("flame_compose_{}", project_name));
        fs::create_dir_all(&tmp_dir)
            .map_err(|e| AppError::internal(format!("Failed to create temp dir: {}", e)))?;
        let compose_path = tmp_dir.join("docker-compose.yml");
        fs::write(&compose_path, compose_yaml)
            .map_err(|e| AppError::internal(format!("Failed to write compose file: {}", e)))?;
        let output = Command::new("docker")
            .args(["compose", "-p", project_name, "-f", &compose_path.to_string_lossy(), "up", "-d"])
            .output()
            .map_err(|e| AppError::internal(format!("Failed to execute docker compose: {}", e)))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::internal(format!("Docker compose up failed: {}", stderr)));
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
            return Err(AppError::internal(format!("Docker compose up failed: {}", stderr)));
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
            return Err(AppError::internal(format!("Docker compose down failed: {}", stderr)));
        }
        Ok(())
    }
}