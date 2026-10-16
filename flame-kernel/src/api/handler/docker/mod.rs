use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::entity::DockerContainer;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 容器日志尾行上限（输出另有 1 MiB 截断）
pub const MAX_LOG_TAIL: usize = 10_000;
/// compose YAML 上限
pub const MAX_COMPOSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Deserialize)]
pub struct ContainerQuery {
    node_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    tail: Option<usize>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ComposeDeployRequest {
    /// `[A-Za-z0-9][A-Za-z0-9_.-]*`，≤ 63
    pub project_name: String,
    /// ≤ 256 KiB；文件持久化到 `data/compose/<project>/docker-compose.yml`
    pub compose_yaml: String,
}

/// 重命名容器（schema 加前缀，避免与文件管理 `RenameRequest` 同名覆盖）
#[derive(Debug, Deserialize, ToSchema)]
#[schema(as = DockerRenameRequest)]
pub struct RenameRequest {
    pub new_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNetworkRequest {
    pub name: String,
    #[serde(default = "default_network_driver")]
    pub driver: String,
    pub subnet: Option<String>,
}

fn default_network_driver() -> String {
    "bridge".into()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectNetworkRequest {
    pub container_id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateVolumeRequest {
    pub name: String,
    #[serde(default = "default_volume_driver")]
    pub driver: String,
}

fn default_volume_driver() -> String {
    "local".into()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PullImageRequest {
    pub image: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TagImageRequest {
    /// 仓库名（不含 tag / digest），如 `registry.local:5000/team/app`
    pub repo: String,
    /// `[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}`
    pub tag: String,
}

#[derive(Debug, Deserialize)]
pub struct ForceQuery {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ComposeDeployResponse {
    pub project_name: String,
    pub status: String,
    pub message: String,
}

/// 容器列表
#[utoipa::path(
    get,
    path = "/api/docker/containers",
    tag = "docker",
    params(("node_id" = Option<i64>, Query, description = "节点 ID，缺省 0 = 本机")),
    responses(
        (status = 200, description = "容器", body = [DockerContainer]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ContainerQuery>,
) -> Result<Json<Vec<DockerContainer>>, AppError> {
    let node_id = query.node_id.unwrap_or(0);
    let containers = state.docker_service.list_containers(node_id).await?;
    Ok(Json(containers))
}

/// 单个容器（按 ID 或名称匹配）
#[utoipa::path(
    get,
    path = "/api/docker/containers/{id}",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "容器", body = DockerContainer),
        (status = 404, description = "容器不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DockerContainer>, AppError> {
    let containers = state.docker_service.list_containers(0).await?;
    let container = containers
        .into_iter()
        .find(|c| c.id == id || c.name == id)
        .ok_or_else(|| AppError::NotFound(format!("Container {} not found", id)))?;
    Ok(Json(container))
}

/// 启动容器
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/start",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.start_container(&id).await?;
    Ok(Json(()))
}

/// 停止容器（10 s 超时）
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/stop",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:stop 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn stop(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.stop_container(&id, 10).await?;
    Ok(Json(()))
}

/// 重启容器（10 s 超时）
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/restart",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn restart(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.restart_container(&id, 10).await?;
    Ok(Json(()))
}

/// 删除容器（非强制）
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/remove",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.remove_container(&id, false).await?;
    Ok(Json(()))
}

/// 容器日志（stdout + stderr，输出上限 1 MiB）
#[utoipa::path(
    get,
    path = "/api/docker/containers/{id}/logs",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）"),
        ("tail" = Option<usize>, Query, description = "尾行数，缺省 100，钳制到 1–10000")),
    responses(
        (status = 200, description = "日志文本", body = String),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<String>, AppError> {
    let tail = query.tail.unwrap_or(100).clamp(1, MAX_LOG_TAIL);
    let logs = state.docker_service.get_container_logs(&id, tail).await?;
    Ok(Json(logs))
}

/// 容器资源统计（Engine API 原样）
#[utoipa::path(
    get,
    path = "/api/docker/containers/{id}/stats",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "统计", body = serde_json::Value),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn stats(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = state.docker_service.get_container_stats(&id).await?;
    Ok(Json(stats))
}

/// 镜像列表
#[utoipa::path(
    get,
    path = "/api/docker/images",
    tag = "docker",
    responses(
        (status = 200, description = "镜像", body = [serde_json::Value]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_images(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let images = state.docker_service.list_images().await?;
    Ok(Json(images))
}

/// 删除镜像
#[utoipa::path(
    post,
    path = "/api/docker/images/{id}/remove",
    tag = "docker",
    params(("id" = String, Path, description = "镜像 ID 或引用")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remove_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.remove_image(&id).await?;
    Ok(Json(()))
}

/// `docker run` 转换请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConvertRunRequest {
    /// 完整的 `docker run …` 命令（可含 `\` 续行，≤ 16 KiB）
    pub command: String,
}

/// `docker run` 命令 → Compose YAML（纯文本转换，不执行命令、不接触 Docker）
#[utoipa::path(
    post,
    path = "/api/docker/compose/convert-run",
    tag = "docker",
    request_body = ConvertRunRequest,
    responses(
        (status = 200, description = "转换结果；`warnings` 列出无法等价转换的参数", body = crate::domain::compose_convert::ComposeConversion),
        (status = 400, description = "不是 docker run 命令 / 含管道等 shell 结构 / 不支持的参数 / 超过 16 KiB"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn compose_convert_run(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ConvertRunRequest>,
) -> Result<Json<crate::domain::compose_convert::ComposeConversion>, AppError> {
    Ok(Json(
        state.docker_service.convert_run_command(&req.command)?,
    ))
}

/// 部署 Compose 项目（等价于宿主机 root 能力，故归 `docker:create`）
#[utoipa::path(
    post,
    path = "/api/docker/compose/deploy",
    tag = "docker",
    request_body = ComposeDeployRequest,
    responses(
        (status = 200, description = "`{project_name, path, status}`", body = serde_json::Value),
        (status = 400, description = "项目名非法 / YAML 为空或超过 256 KiB"),
        (status = 500, description = "docker compose up 失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn compose_deploy(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ComposeDeployRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.compose_yaml.trim().is_empty() || req.compose_yaml.len() > MAX_COMPOSE_BYTES {
        return Err(AppError::ValidationError(format!(
            "compose_yaml: must be non-empty and at most {MAX_COMPOSE_BYTES} bytes"
        )));
    }
    let result = state
        .docker_service
        .compose_deploy(&req.project_name, &req.compose_yaml)
        .await?;
    Ok(Json(result))
}

/// 启动 Compose 项目（使用项目记录的配置文件）
#[utoipa::path(
    post,
    path = "/api/docker/compose/{project_name}/up",
    tag = "docker",
    params(("project_name" = String, Path, description = "Compose 项目名")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "项目名非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn compose_up(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.compose_up(&project_name).await?;
    Ok(Json(()))
}

/// 停止并移除 Compose 项目容器与网络（保留配置文件与数据）
#[utoipa::path(
    post,
    path = "/api/docker/compose/{project_name}/down",
    tag = "docker",
    params(("project_name" = String, Path, description = "Compose 项目名")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "项目名非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:stop 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn compose_down(
    State(state): State<AppState>,
    Path(project_name): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.compose_down(&project_name).await?;
    Ok(Json(()))
}

// ── 容器高级操作 ─────────────────────────────────────────────

/// 容器详情（Engine API inspect 原样）
#[utoipa::path(
    get,
    path = "/api/docker/containers/{id}/inspect",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "详情", body = serde_json::Value),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn inspect(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let detail = state.docker_service.inspect_container(&id).await?;
    Ok(Json(detail))
}

/// 既有容器 → Compose YAML（收编：导出后停止原容器，再以该 YAML 部署）
#[utoipa::path(
    get,
    path = "/api/docker/containers/{id}/compose",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "转换结果（YAML 仅供审阅，不会部署）", body = crate::domain::compose_convert::ComposeConversion),
        (status = 400, description = "引用含非法字符 / 容器信息缺少镜像"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn container_compose(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::domain::compose_convert::ComposeConversion>, AppError> {
    Ok(Json(state.docker_service.container_to_compose(&id).await?))
}

/// 重命名容器
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/rename",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    request_body = RenameRequest,
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "ID 或新名称非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn rename(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .rename_container(&id, &req.new_name)
        .await?;
    Ok(Json(()))
}

/// 暂停容器
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/pause",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn pause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.pause_container(&id).await?;
    Ok(Json(()))
}

/// 恢复容器
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/unpause",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn unpause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.unpause_container(&id).await?;
    Ok(Json(()))
}

/// 强制终止容器
#[utoipa::path(
    post,
    path = "/api/docker/containers/{id}/kill",
    tag = "docker",
    params(("id" = String, Path, description = "容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`）")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn kill(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.kill_container(&id).await?;
    Ok(Json(()))
}

/// 清理已停止容器
#[utoipa::path(
    post,
    path = "/api/docker/containers/prune",
    tag = "docker",
    responses(
        (status = 200, description = "清理结果", body = serde_json::Value),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn prune_containers(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_containers().await?;
    Ok(Json(result))
}

// ── 网络管理 ────────────────────────────────────────────────

/// 网络列表
#[utoipa::path(
    get,
    path = "/api/docker/networks",
    tag = "docker",
    responses(
        (status = 200, description = "网络", body = [serde_json::Value]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_networks(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let networks = state.docker_service.list_networks().await?;
    Ok(Json(networks))
}

/// 创建网络
#[utoipa::path(
    post,
    path = "/api/docker/networks",
    tag = "docker",
    request_body = CreateNetworkRequest,
    responses(
        (status = 200, description = "创建结果", body = serde_json::Value),
        (status = 400, description = "名称 / 驱动 / 子网（CIDR）非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_network(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateNetworkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .docker_service
        .create_network(&req.name, &req.driver, req.subnet.as_deref())
        .await?;
    Ok(Json(result))
}

/// 删除网络
#[utoipa::path(
    delete,
    path = "/api/docker/networks/{id}",
    tag = "docker",
    params(("id" = String, Path, description = "网络 ID 或名称")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remove_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, AppError> {
    state.docker_service.remove_network(&id).await?;
    Ok(Json(()))
}

/// 把容器接入网络
#[utoipa::path(
    post,
    path = "/api/docker/networks/{id}/connect",
    tag = "docker",
    params(("id" = String, Path, description = "网络 ID 或名称")),
    request_body = ConnectNetworkRequest,
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn connect_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ConnectNetworkRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .connect_network(&id, &req.container_id)
        .await?;
    Ok(Json(()))
}

/// 把容器移出网络（`force` 可强制）
#[utoipa::path(
    post,
    path = "/api/docker/networks/{id}/disconnect",
    tag = "docker",
    params(("id" = String, Path, description = "网络 ID 或名称")),
    request_body = ConnectNetworkRequest,
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn disconnect_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ConnectNetworkRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .disconnect_network(&id, &req.container_id, req.force)
        .await?;
    Ok(Json(()))
}

/// 清理未使用网络
#[utoipa::path(
    post,
    path = "/api/docker/networks/prune",
    tag = "docker",
    responses(
        (status = 200, description = "清理结果", body = serde_json::Value),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn prune_networks(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_networks().await?;
    Ok(Json(result))
}

// ── 卷管理 ──────────────────────────────────────────────────

/// 卷列表
#[utoipa::path(
    get,
    path = "/api/docker/volumes",
    tag = "docker",
    responses(
        (status = 200, description = "卷", body = [serde_json::Value]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_volumes(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let volumes = state.docker_service.list_volumes().await?;
    Ok(Json(volumes))
}

/// 创建卷
#[utoipa::path(
    post,
    path = "/api/docker/volumes",
    tag = "docker",
    request_body = CreateVolumeRequest,
    responses(
        (status = 200, description = "创建结果", body = serde_json::Value),
        (status = 400, description = "名称 / 驱动非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_volume(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateVolumeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .docker_service
        .create_volume(&req.name, &req.driver)
        .await?;
    Ok(Json(result))
}

/// 删除卷
#[utoipa::path(
    delete,
    path = "/api/docker/volumes/{name}",
    tag = "docker",
    params(("name" = String, Path, description = "卷名"),
        ("force" = Option<bool>, Query, description = "强制删除")),
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remove_volume(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<ForceQuery>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .remove_volume(&name, query.force)
        .await?;
    Ok(Json(()))
}

/// 清理未使用卷
#[utoipa::path(
    post,
    path = "/api/docker/volumes/prune",
    tag = "docker",
    responses(
        (status = 200, description = "清理结果", body = serde_json::Value),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn prune_volumes(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_volumes().await?;
    Ok(Json(result))
}

// ── 镜像管理 ────────────────────────────────────────────────

/// 拉取镜像（`{image, message}`）
#[utoipa::path(
    post,
    path = "/api/docker/images/pull",
    tag = "docker",
    request_body = PullImageRequest,
    responses(
        (status = 200, description = "拉取结果", body = serde_json::Value),
        (status = 400, description = "镜像引用非法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn pull_image(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<PullImageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let message = state.docker_service.pull_image(&req.image).await?;
    Ok(Json(
        serde_json::json!({ "image": req.image, "message": message }),
    ))
}

/// 为镜像打标签
#[utoipa::path(
    post,
    path = "/api/docker/images/{id}/tag",
    tag = "docker",
    params(("id" = String, Path, description = "镜像 ID 或引用")),
    request_body = TagImageRequest,
    responses(
        (status = 200, description = "成功（`null`）"),
        (status = 400, description = "镜像 / 仓库 / tag 非法；repo 含 tag 或 digest"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn tag_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<TagImageRequest>,
) -> Result<Json<()>, AppError> {
    state
        .docker_service
        .tag_image(&id, &req.repo, &req.tag)
        .await?;
    Ok(Json(()))
}

/// 清理悬空镜像
#[utoipa::path(
    post,
    path = "/api/docker/images/prune",
    tag = "docker",
    responses(
        (status = 200, description = "清理结果", body = serde_json::Value),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn prune_images(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.docker_service.prune_images().await?;
    Ok(Json(result))
}

// ── Compose 项目 ────────────────────────────────────────────

/// Compose 项目列表（含已停止项目；`{name, status, config_files}`）
#[utoipa::path(
    get,
    path = "/api/docker/compose",
    tag = "docker",
    responses(
        (status = 200, description = "项目", body = [serde_json::Value]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 docker:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn compose_ls(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let projects = state.docker_service.compose_ls().await?;
    Ok(Json(projects))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/docker/containers", axum::routing::get(list))
        .route("/api/docker/containers/{id}", axum::routing::get(get))
        .route(
            "/api/docker/containers/{id}/start",
            axum::routing::post(start),
        )
        .route(
            "/api/docker/containers/{id}/stop",
            axum::routing::post(stop),
        )
        .route(
            "/api/docker/containers/{id}/restart",
            axum::routing::post(restart),
        )
        .route(
            "/api/docker/containers/{id}/remove",
            axum::routing::post(remove),
        )
        .route("/api/docker/containers/{id}/logs", axum::routing::get(logs))
        .route(
            "/api/docker/containers/{id}/stats",
            axum::routing::get(stats),
        )
        .route("/api/docker/images", axum::routing::get(list_images))
        .route("/api/docker/images/pull", axum::routing::post(pull_image))
        .route(
            "/api/docker/images/{id}/remove",
            axum::routing::post(remove_image),
        )
        .route(
            "/api/docker/images/{id}/tag",
            axum::routing::post(tag_image),
        )
        .route(
            "/api/docker/images/prune",
            axum::routing::post(prune_images),
        )
        .route(
            "/api/docker/containers/{id}/inspect",
            axum::routing::get(inspect),
        )
        .route(
            "/api/docker/containers/{id}/compose",
            axum::routing::get(container_compose),
        )
        .route(
            "/api/docker/containers/{id}/rename",
            axum::routing::post(rename),
        )
        .route(
            "/api/docker/containers/{id}/pause",
            axum::routing::post(pause),
        )
        .route(
            "/api/docker/containers/{id}/unpause",
            axum::routing::post(unpause),
        )
        .route(
            "/api/docker/containers/{id}/kill",
            axum::routing::post(kill),
        )
        .route(
            "/api/docker/containers/prune",
            axum::routing::post(prune_containers),
        )
        .route("/api/docker/networks", axum::routing::get(list_networks))
        .route("/api/docker/networks", axum::routing::post(create_network))
        .route(
            "/api/docker/networks/prune",
            axum::routing::post(prune_networks),
        )
        .route(
            "/api/docker/networks/{id}",
            axum::routing::delete(remove_network),
        )
        .route(
            "/api/docker/networks/{id}/connect",
            axum::routing::post(connect_network),
        )
        .route(
            "/api/docker/networks/{id}/disconnect",
            axum::routing::post(disconnect_network),
        )
        .route("/api/docker/volumes", axum::routing::get(list_volumes))
        .route("/api/docker/volumes", axum::routing::post(create_volume))
        .route(
            "/api/docker/volumes/prune",
            axum::routing::post(prune_volumes),
        )
        .route(
            "/api/docker/volumes/{name}",
            axum::routing::delete(remove_volume),
        )
        .route("/api/docker/compose", axum::routing::get(compose_ls))
        .route(
            "/api/docker/compose/convert-run",
            axum::routing::post(compose_convert_run),
        )
        .route(
            "/api/docker/compose/deploy",
            axum::routing::post(compose_deploy),
        )
        .route(
            "/api/docker/compose/{project_name}/up",
            axum::routing::post(compose_up),
        )
        .route(
            "/api/docker/compose/{project_name}/down",
            axum::routing::post(compose_down),
        )
}
