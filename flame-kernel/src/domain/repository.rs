use crate::core::error::AppError;
use crate::domain::entity::*;
use async_trait::async_trait;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError>;
    async fn update(&self, user: &User) -> Result<(), AppError>;
    async fn list(&self) -> Result<Vec<User>, AppError>;
    async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    // ── 分页下沉（Stage1）：数据库层直接 LIMIT/OFFSET，避免全表加载 + 内存切片 ──
    /// 分页查询用户（按 id 倒序）。`limit` 会被 clamp 到 `1..=200`。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>, AppError>;
    /// 用户总数。
    async fn count(&self) -> Result<i64, AppError>;
}

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<ServerNode>, AppError>;
    async fn find_by_hostname(&self, hostname: &str) -> Result<Option<ServerNode>, AppError>;
    async fn create(&self, node: &ServerNode) -> Result<i64, AppError>;
    async fn update(&self, node: &ServerNode) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<ServerNode>, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 记录心跳：更新 last_heartbeat_at 与指标快照
    async fn update_heartbeat(&self, id: i64, metrics_json: &str) -> Result<(), AppError>;

    // ── 分页下沉（Stage1）──
    /// 分页查询节点（按 id 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ServerNode>, AppError>;
    /// 节点总数。
    async fn count(&self) -> Result<i64, AppError>;
    /// 查询心跳早于指定时间的节点（离线扫描条件化，避免全量 list_all 后过滤）。
    async fn list_stale_heartbeats(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ServerNode>, AppError>;
}

#[async_trait]
pub trait WebsiteRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Website>, AppError>;
    async fn find_by_domain(&self, domain: &str) -> Result<Option<Website>, AppError>;
    async fn create(&self, website: &Website) -> Result<i64, AppError>;
    async fn update(&self, website: &Website) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<Website>, AppError>;

    // ── 分页下沉（Stage1）──
    /// 分页查询网站（按 id 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Website>, AppError>;
    /// 网站总数。
    async fn count(&self) -> Result<i64, AppError>;
}

#[async_trait]
pub trait ContainerRepository: Send + Sync {
    async fn list_containers(&self, node_id: i64) -> Result<Vec<DockerContainer>, AppError>;
    async fn get_container(&self, id: &str) -> Result<Option<DockerContainer>, AppError>;
    async fn start_container(&self, id: &str) -> Result<(), AppError>;
    async fn stop_container(&self, id: &str, timeout: u64) -> Result<(), AppError>;
    async fn restart_container(&self, id: &str, timeout: u64) -> Result<(), AppError>;
    async fn remove_container(&self, id: &str, force: bool) -> Result<(), AppError>;
    async fn get_container_logs(&self, id: &str, tail: usize) -> Result<String, AppError>;
    async fn get_container_stats(&self, id: &str) -> Result<serde_json::Value, AppError>;

    // ── 容器高级操作 (参考 1Panel: inspect/rename/pause/unpause/kill/prune) ──
    async fn inspect_container(&self, id: &str) -> Result<serde_json::Value, AppError>;
    async fn rename_container(&self, id: &str, new_name: &str) -> Result<(), AppError>;
    async fn pause_container(&self, id: &str) -> Result<(), AppError>;
    async fn unpause_container(&self, id: &str) -> Result<(), AppError>;
    async fn kill_container(&self, id: &str) -> Result<(), AppError>;
    async fn prune_containers(&self) -> Result<serde_json::Value, AppError>;
}

#[async_trait]
pub trait NetworkRepository: Send + Sync {
    async fn list_networks(&self) -> Result<Vec<serde_json::Value>, AppError>;
    async fn create_network(
        &self,
        name: &str,
        driver: &str,
        subnet: Option<&str>,
    ) -> Result<serde_json::Value, AppError>;
    async fn remove_network(&self, id: &str) -> Result<(), AppError>;
    async fn connect_network(&self, network_id: &str, container_id: &str) -> Result<(), AppError>;
    async fn disconnect_network(
        &self,
        network_id: &str,
        container_id: &str,
        force: bool,
    ) -> Result<(), AppError>;
    async fn prune_networks(&self) -> Result<serde_json::Value, AppError>;
}

#[async_trait]
pub trait VolumeRepository: Send + Sync {
    async fn list_volumes(&self) -> Result<Vec<serde_json::Value>, AppError>;
    async fn create_volume(&self, name: &str, driver: &str) -> Result<serde_json::Value, AppError>;
    async fn remove_volume(&self, name: &str, force: bool) -> Result<(), AppError>;
    async fn prune_volumes(&self) -> Result<serde_json::Value, AppError>;
}

#[async_trait]
pub trait ImageRepository: Send + Sync {
    async fn list_images(&self) -> Result<Vec<serde_json::Value>, AppError>;
    async fn remove_image(&self, id: &str) -> Result<(), AppError>;
    async fn pull_image(&self, image: &str) -> Result<String, AppError>;
    async fn tag_image(&self, image_id: &str, repo: &str, tag: &str) -> Result<(), AppError>;
    async fn prune_images(&self) -> Result<serde_json::Value, AppError>;
}

#[async_trait]
pub trait ComposeRepository: Send + Sync {
    /// 经统一特权命令执行端口运行 `docker compose ...`（`execution_mode=embedded|agent` 分离模式）。
    async fn run_compose(
        &self,
        args: Vec<String>,
    ) -> Result<crate::domain::execution_mode::CommandOutput, AppError>;
    async fn compose_deploy(
        &self,
        project_name: &str,
        compose_yaml: &str,
    ) -> Result<serde_json::Value, AppError>;
    async fn compose_up(&self, project_name: &str) -> Result<(), AppError>;
    async fn compose_down(&self, project_name: &str) -> Result<(), AppError>;
    async fn compose_ls(&self) -> Result<Vec<serde_json::Value>, AppError>;
}

/// 兼容门面：一次性提供全部 Docker 端口（旧调用方/组合根便利，避免拆 split 时改大量 handler）。
///
/// **已拆分为 5 个细分端口（Stage 8）**：请改用 `ContainerRepository` / `ImageRepository` /
/// `NetworkRepository` / `VolumeRepository` / `ComposeRepository`。此门面仅保留供旧调用方
/// 与组合根过渡使用，新代码应直接按职责依赖具体 trait，不再依赖单个聚合门面。
#[async_trait]
pub trait DockerRepository:
    ContainerRepository
    + NetworkRepository
    + VolumeRepository
    + ImageRepository
    + ComposeRepository
    + Send
    + Sync
{
}

#[async_trait]
pub trait OperationLogRepository: Send + Sync {
    async fn create(
        &self,
        username: &str,
        action: &str,
        target: Option<&str>,
        ip: Option<&str>,
    ) -> Result<OperationLog, AppError>;
    async fn list(&self) -> Result<Vec<OperationLog>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<OperationLog>, AppError>;
    async fn list_by_username(&self, username: &str) -> Result<Vec<OperationLog>, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    // ── 分页下沉（Stage2）：数据库层直接 LIMIT/OFFSET，避免全表加载 ──
    /// 分页查询操作日志（按 id 倒序）。`action_prefix` 为可选 action 前缀过滤。
    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        action_prefix: Option<&str>,
    ) -> Result<Vec<OperationLog>, AppError>;
    /// 操作日志总数（支持 action 前缀过滤）。
    async fn count(&self, action_prefix: Option<&str>) -> Result<i64, AppError>;
}

/// 事件落库（Outbox）端口：把领域事件持久化存档，保证审计不丢（Stage6）。
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    /// 追加一条事件落库记录，返回带 id 的持久化实体。
    async fn append(
        &self,
        event_type: &str,
        payload: &str,
        published: bool,
    ) -> Result<OutboxEvent, AppError>;
    /// 分页查询（按 id 倒序）。`event_type` 为可选事件类型过滤。
    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        event_type: Option<&str>,
    ) -> Result<Vec<OutboxEvent>, AppError>;
    /// 事件落库总数（支持事件类型过滤）。
    async fn count(&self, event_type: Option<&str>) -> Result<i64, AppError>;
}

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn create(
        &self,
        source: &str,
        level: &str,
        message: &str,
        metadata: Option<&str>,
    ) -> Result<LogEntry, AppError>;
    async fn list(&self) -> Result<Vec<LogEntry>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<LogEntry>, AppError>;
    async fn list_by_source(&self, source: &str) -> Result<Vec<LogEntry>, AppError>;
    async fn list_by_level(&self, level: &str) -> Result<Vec<LogEntry>, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    // ── 分页下沉（Stage2）：数据库层直接 LIMIT/OFFSET，避免全表加载 ──
    /// 分页查询系统日志（按 id 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<LogEntry>, AppError>;
    /// 系统日志总数。
    async fn count(&self) -> Result<i64, AppError>;
}

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Permission>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Permission>, AppError>;
    async fn find_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<Permission>, AppError>;
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
    async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), AppError>;
    /// 批量原子写：多键在一个事务内全部写入（要么全成功要么全回滚）。
    async fn set_many(&self, entries: &[(String, String)]) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<PanelSetting>, AppError>;
    async fn get_all_map(&self) -> Result<std::collections::HashMap<String, String>, AppError>;

    // ── 分页下沉（Stage1）──
    /// 分页查询设置（按 key 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<PanelSetting>, AppError>;
    /// 设置总数。
    async fn count(&self) -> Result<i64, AppError>;
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
    /// 批量更新实例状态（Phase A2 扩展：多键原子写，要么全部成功要么全部回滚）。
    async fn update_status_batch(&self, updates: &[(i64, String)]) -> Result<(), AppError>;

    // ── 分页下沉（Stage1）──
    /// 分页查询数据库实例（按 id 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<DatabaseInstance>, AppError>;
    /// 数据库实例总数。
    async fn count(&self) -> Result<i64, AppError>;
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

    // ── 分页下沉（Stage1）──
    /// 分页查询防火墙规则（按 id 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<FirewallRule>, AppError>;
    /// 防火墙规则总数。
    async fn count(&self) -> Result<i64, AppError>;
}

#[async_trait]
pub trait ScheduledTaskRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<ScheduledTask>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<ScheduledTask>, AppError>;
    async fn create(&self, task: &ScheduledTask) -> Result<i64, AppError>;
    async fn update(&self, task: &ScheduledTask) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    // ── 分页下沉（Stage2）──
    /// 分页查询定时任务。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<ScheduledTask>, AppError>;
    /// 定时任务总数。
    async fn count(&self) -> Result<i64, AppError>;
}

#[async_trait]
pub trait WebServerRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<WebServerInstance>, AppError>;
    async fn find_by_engine(&self, engine: &str) -> Result<Vec<WebServerInstance>, AppError>;
    async fn create(&self, instance: &WebServerInstance) -> Result<i64, AppError>;
    async fn update(&self, instance: &WebServerInstance) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    async fn list_all(&self) -> Result<Vec<WebServerInstance>, AppError>;

    // ── 分页下沉（Stage1）──
    /// 分页查询 Web 引擎实例（按 id 倒序）。
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<WebServerInstance>, AppError>;
    /// Web 引擎实例总数。
    async fn count(&self) -> Result<i64, AppError>;
}

#[async_trait]
pub trait AppPackageRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<AppPackage>, AppError>;
    async fn find_by_key(&self, key: &str) -> Result<Option<AppPackage>, AppError>;
    async fn create(&self, pkg: &AppPackage) -> Result<i64, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 批量写入应用包（Phase A2 扩展：统一接入 `set_many` 事务语义，要么全部成功要么全部回滚）。
    async fn create_many(&self, pkgs: &[AppPackage]) -> Result<usize, AppError>;
}

#[async_trait]
pub trait InstalledAppRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<InstalledApp>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<InstalledApp>, AppError>;
    async fn create(&self, app: &InstalledApp) -> Result<i64, AppError>;
    async fn update(&self, app: &InstalledApp) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

#[async_trait]
pub trait PluginRepository: Send + Sync {
    async fn save(&self, plugin: &Plugin) -> Result<(), AppError>;
    async fn list(&self) -> Result<Vec<Plugin>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Plugin>, AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait MemoRepository: Send + Sync {
    async fn list(&self, kind: Option<&str>, done: Option<bool>) -> Result<Vec<Memo>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Memo>, AppError>;
    async fn create(&self, content: &str, kind: &str) -> Result<i64, AppError>;
    async fn update(&self, memo: &Memo) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}
