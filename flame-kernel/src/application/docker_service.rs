//! Docker 领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use async_trait::async_trait;
use std::sync::Arc;

/// Docker 门面适配器：将按职责拆分的 Docker 子端口（容器/网络/卷/镜像/Compose）
/// 聚合为门面 `DockerRepository`，供 `DockerService` 使用。
///
/// 位于 application 层（只依赖 domain 端口，不依赖 infrastructure 具体类型）。
///
/// **已拆分为 5 个细分端口（Stage 8）**：`DockerService` 应改为按职责持有各细分端口，
/// 避免新代码依赖单个聚合门面。此适配器仅保留用于既有构造路径过渡。
pub struct DockerRepositoryFacade {
    pub container: Arc<dyn ContainerRepository>,
    pub network: Arc<dyn NetworkRepository>,
    pub volume: Arc<dyn VolumeRepository>,
    pub image: Arc<dyn ImageRepository>,
    pub compose: Arc<dyn ComposeRepository>,
}

#[async_trait]
impl ContainerRepository for DockerRepositoryFacade {
    async fn list_containers(&self, node_id: i64) -> Result<Vec<DockerContainer>, AppError> {
        self.container.list_containers(node_id).await
    }
    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError> {
        self.container.get_container(id).await
    }
    async fn start_container(&self, id: &str) -> Result<(), AppError> {
        self.container.start_container(id).await
    }
    async fn stop_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        self.container.stop_container(id, timeout).await
    }
    async fn restart_container(&self, id: &str, timeout: u64) -> Result<(), AppError> {
        self.container.restart_container(id, timeout).await
    }
    async fn remove_container(&self, id: &str, force: bool) -> Result<(), AppError> {
        self.container.remove_container(id, force).await
    }
    async fn get_container_logs(&self, id: &str, tail: usize) -> Result<String, AppError> {
        self.container.get_container_logs(id, tail).await
    }
    async fn get_container_stats(&self, id: &str) -> Result<serde_json::Value, AppError> {
        self.container.get_container_stats(id).await
    }
    async fn inspect_container(&self, id: &str) -> Result<serde_json::Value, AppError> {
        self.container.inspect_container(id).await
    }
    async fn rename_container(&self, id: &str, new_name: &str) -> Result<(), AppError> {
        self.container.rename_container(id, new_name).await
    }
    async fn pause_container(&self, id: &str) -> Result<(), AppError> {
        self.container.pause_container(id).await
    }
    async fn unpause_container(&self, id: &str) -> Result<(), AppError> {
        self.container.unpause_container(id).await
    }
    async fn kill_container(&self, id: &str) -> Result<(), AppError> {
        self.container.kill_container(id).await
    }
    async fn prune_containers(&self) -> Result<serde_json::Value, AppError> {
        self.container.prune_containers().await
    }
}

#[async_trait]
impl NetworkRepository for DockerRepositoryFacade {
    async fn list_networks(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.network.list_networks().await
    }
    async fn create_network(
        &self,
        name: &str,
        driver: &str,
        subnet: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        self.network.create_network(name, driver, subnet).await
    }
    async fn remove_network(&self, id: &str) -> Result<(), AppError> {
        self.network.remove_network(id).await
    }
    async fn connect_network(&self, network_id: &str, container_id: &str) -> Result<(), AppError> {
        self.network.connect_network(network_id, container_id).await
    }
    async fn disconnect_network(
        &self,
        network_id: &str,
        container_id: &str,
        force: bool,
    ) -> Result<(), AppError> {
        self.network
            .disconnect_network(network_id, container_id, force)
            .await
    }
    async fn prune_networks(&self) -> Result<serde_json::Value, AppError> {
        self.network.prune_networks().await
    }
}

#[async_trait]
impl VolumeRepository for DockerRepositoryFacade {
    async fn list_volumes(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.volume.list_volumes().await
    }
    async fn create_volume(&self, name: &str, driver: &str) -> Result<serde_json::Value, AppError> {
        self.volume.create_volume(name, driver).await
    }
    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), AppError> {
        self.volume.remove_volume(name, force).await
    }
    async fn prune_volumes(&self) -> Result<serde_json::Value, AppError> {
        self.volume.prune_volumes().await
    }
}

#[async_trait]
impl ImageRepository for DockerRepositoryFacade {
    async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.image.list_images().await
    }
    async fn remove_image(&self, id: &str) -> Result<(), AppError> {
        self.image.remove_image(id).await
    }
    async fn pull_image(&self, image: &str) -> Result<String, AppError> {
        self.image.pull_image(image).await
    }
    async fn tag_image(&self, image_id: &str, repo: &str, tag: &str) -> Result<(), AppError> {
        self.image.tag_image(image_id, repo, tag).await
    }
    async fn prune_images(&self) -> Result<serde_json::Value, AppError> {
        self.image.prune_images().await
    }
}

#[async_trait]
impl ComposeRepository for DockerRepositoryFacade {
    async fn run_compose(
        &self,
        args: Vec<String>,
    ) -> Result<crate::application::execution_mode::CommandOutput, AppError> {
        self.compose.run_compose(args).await
    }
    async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError> {
        self.compose
            .compose_deploy(project_name, compose_yaml)
            .await
    }
    async fn compose_up(&self, project_name: &str) -> Result<(), AppError> {
        self.compose.compose_up(project_name).await
    }
    async fn compose_down(&self, project_name: &str) -> Result<(), AppError> {
        self.compose.compose_down(project_name).await
    }
    async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError> {
        self.compose.compose_ls().await
    }
}

#[async_trait]
impl DockerRepository for DockerRepositoryFacade {}

pub struct DockerService {
    pub docker_repo: Arc<dyn DockerRepository>,
}

impl DockerService {
    pub fn new(docker_repo: Arc<dyn DockerRepository>) -> Self {
        Self { docker_repo }
    }

    /// 从按职责拆分的端口组装（容器/网络/卷/镜像/Compose）。
    ///
    /// 六边形重构后各端口由 `RepoFactory::create_*_repo` 提供；
    /// 该构造器通过门面适配器 `DockerRepositoryFacade` 委托各子端口。
    pub fn from_repos(
        container: Arc<dyn ContainerRepository>,
        network: Arc<dyn NetworkRepository>,
        volume: Arc<dyn VolumeRepository>,
        image: Arc<dyn ImageRepository>,
        compose: Arc<dyn ComposeRepository>,
    ) -> Self {
        Self::new(Arc::new(DockerRepositoryFacade {
            container,
            network,
            volume,
            image,
            compose,
        }))
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

    pub async fn connect_network(
        &self,
        network_id: &str,
        container_id: &str,
    ) -> Result<(), AppError> {
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

    pub async fn create_volume(
        &self,
        name: &str,
        driver: &str,
    ) -> Result<serde_json::Value, AppError> {
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
