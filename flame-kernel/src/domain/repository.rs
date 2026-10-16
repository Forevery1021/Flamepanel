use crate::core::error::AppError;
use crate::domain::entity::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

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
    /// F-04: 读取用户最近一次改密时间（Unix 秒）
    async fn get_password_changed_at(&self, id: i64) -> Result<Option<i64>, AppError>;
    /// F-04: 更新用户最近一次改密时间（Unix 秒）
    async fn update_password_changed_at(&self, id: i64, ts: i64) -> Result<(), AppError>;

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
    /// `docker image inspect`：用于「容器收编」时剔除镜像自带的默认配置。
    /// 默认返回 Null（内存实现 / 不支持时退化为「保留全部并提示」）。
    async fn inspect_image(&self, _image: &str) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::Value::Null)
    }
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
    /// 在指定项目目录部署：compose 文件落盘为 `project_dir/docker-compose.yml`，
    /// 相对挂载（`./data:/data`，1Panel 包惯例）以该目录为基准，且目录在部署后保留，
    /// 之后 `compose_up` 可凭 compose 记录的配置文件重新拉起。缺省实现退化为 `compose_deploy`。
    async fn compose_deploy_at(
        &self,
        project_name: &str,
        compose_yaml: &str,
        project_dir: &std::path::Path,
    ) -> Result<serde_json::Value, AppError> {
        let _ = project_dir;
        self.compose_deploy(project_name, compose_yaml).await
    }
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

    /// F-12: 删除早于指定天数的操作日志（保留策略）。
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError>;
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

    /// F-12: 删除早于指定天数的 outbox 事件（保留策略，仅删除已发布且超期的）。
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError>;
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

    /// F-12: 删除早于指定天数的系统日志（保留策略）。
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError>;
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
    /// 回写包体本地缓存路径（M-A Issue 1.4：http 源懒加载拉取后落库）。
    async fn update_source_path(&self, id: i64, source_path: &str) -> Result<(), AppError>;
}

/// 远程应用源仓储（M-A Issue 1.1，Doc/20 §4.1）
#[async_trait]
pub trait AppSourceRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<AppSource>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<AppSource>, AppError>;
    async fn find_by_url(&self, url: &str) -> Result<Option<AppSource>, AppError>;
    /// 新建源，返回自增 id。url 重复时返回错误（UNIQUE 约束）。
    async fn create(&self, source: &AppSource) -> Result<i64, AppError>;
    /// 更新可编辑字段（name/url/enabled/public_key）与 updated_at。
    async fn update(&self, source: &AppSource) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 记录一次成功同步（写 last_synced_at = now）。
    async fn touch_synced(&self, id: i64) -> Result<(), AppError>;
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

/// F-04: 已吊销 refresh token 的持久化黑名单仓储（SQLite 落库，重启不丢失）。
#[async_trait]
pub trait RevokedTokenRepository: Send + Sync {
    /// 将 token 哈希加入黑名单（带过期时间，Unix 秒）
    async fn insert(&self, token_hash: &str, expires_at: i64) -> Result<(), AppError>;
    /// 检查 token 哈希是否在黑名单中
    async fn contains(&self, token_hash: &str) -> Result<bool, AppError>;
    /// 清理已过期条目
    async fn cleanup_expired(&self) -> Result<(), AppError>;
}

/// 监控历史仓储（S-07）：按分辨率存放降采样点，`(resolution, bucket_start)` 唯一。
#[async_trait]
pub trait MetricsHistoryRepository: Send + Sync {
    /// 写入 / 覆盖一个桶（同一桶重复写入取最新值）
    async fn upsert(
        &self,
        resolution: MetricsResolution,
        point: &MetricsHistoryPoint,
    ) -> Result<(), AppError>;
    /// 查询 `[from, to)` 区间内的点，按 `bucket_start` 升序，最多 `limit` 条
    async fn query(
        &self,
        resolution: MetricsResolution,
        from: i64,
        to: i64,
        limit: usize,
    ) -> Result<Vec<MetricsHistoryPoint>, AppError>;
    /// 删除 `bucket_start < before` 的点，返回删除行数
    async fn prune(&self, resolution: MetricsResolution, before: i64) -> Result<u64, AppError>;
}

/// 备份账号仓储（M-A 收尾 · Doc/20 §6.5 备份账号体系 Stage 1 数据层）
#[async_trait]
pub trait BackupAccountRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<BackupAccount>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<BackupAccount>, AppError>;
    /// 新建账号，返回自增 id。
    async fn create(&self, account: &BackupAccount) -> Result<i64, AppError>;
    /// 更新可编辑字段（name/account_type/config_json/enabled）与 updated_at。
    async fn update(&self, account: &BackupAccount) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 记录一次连通性测试结果（写 last_test_status / last_test_at / updated_at）。
    async fn record_test(&self, id: i64, ok: bool) -> Result<(), AppError>;
}

/// 备份任务仓储（M-A 收尾 Stage 3 · Doc/20 §6.5）
#[async_trait]
pub trait BackupJobRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<BackupJob>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<BackupJob>, AppError>;
    /// 新建任务，返回自增 id。
    async fn create(&self, job: &BackupJob) -> Result<i64, AppError>;
    /// 更新可编辑字段（name/scope/scope_key/schedule/target_account_id/retention/enabled
    /// /next_run_at）与 updated_at，执行历史字段不在此更新。
    async fn update(&self, job: &BackupJob) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 记录一次执行结果（写 last_status/last_message/last_remote_key/last_run_at
    /// /next_run_at/updated_at）。
    async fn record_run(
        &self,
        id: i64,
        ok: bool,
        message: Option<String>,
        remote_key: Option<String>,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<(), AppError>;
}

/// 运行时仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
#[async_trait]
pub trait RuntimeRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Runtime>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Runtime>, AppError>;
    /// 同 kind + version 唯一（UNIQUE 约束），返回自增 id。
    async fn create(&self, runtime: &Runtime) -> Result<i64, AppError>;
    /// 更新可编辑字段（install_path/status/is_default/metadata_json）与 updated_at。
    async fn update(&self, runtime: &Runtime) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 将指定运行时设为同 kind 唯一默认（事务内先清后设）。
    async fn set_default(&self, id: i64) -> Result<(), AppError>;
}

/// 用户项目仓储（M-B 主线二 Stage A-1 · Doc/20 §5.2）
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Project>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Project>, AppError>;
    /// 项目名唯一（UNIQUE 约束），返回自增 id。
    async fn create(&self, project: &Project) -> Result<i64, AppError>;
    /// 更新可编辑字段（runtime_id/source/source_url/deploy_path/run_command
    /// /port/domain/status/process_unit）与 updated_at。
    async fn update(&self, project: &Project) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

/// 项目数据库联链仓储（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）
#[async_trait]
pub trait ProjectDbLinkRepository: Send + Sync {
    /// 项目的全部联链（按 id 升序）。
    async fn list_by_project(&self, project_id: i64) -> Result<Vec<ProjectDbLink>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<ProjectDbLink>, AppError>;
    /// 新建联链，返回自增 id。
    async fn create(&self, link: &ProjectDbLink) -> Result<i64, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 删除项目的全部联链（项目删除时级联清理，幂等）。
    async fn delete_by_project(&self, project_id: i64) -> Result<(), AppError>;
}

/// SSL 证书记录仓储（M-B 主线三 §6.1 · Stage 1 数据层）
#[async_trait]
pub trait CertificateRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Certificate>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Certificate>, AppError>;
    /// 按 SAN 列表原文（JSON 字符串）查找——同域名集合视为同一证书。
    async fn find_by_domains(&self, domains: &str) -> Result<Option<Certificate>, AppError>;
    /// 列出关联网站的证书。
    async fn find_by_website(&self, website_id: i64) -> Result<Option<Certificate>, AppError>;
    /// 新建，返回自增 id。
    async fn create(&self, cert: &Certificate) -> Result<i64, AppError>;
    /// 更新可编辑/签发回写字段（cert_path/key_path/expires_at/last_renewed_at
    /// /auto_renew/website_id/dns_provider）与 updated_at。
    async fn update(&self, cert: &Certificate) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

/// DNS 厂商账号仓储（主线三 §6.1 · Stage 4 · DNS-01 泛域名续签前置）
#[async_trait]
pub trait DnsProviderAccountRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<DnsProviderAccount>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<DnsProviderAccount>, AppError>;
    /// 新建账号，返回自增 id。
    async fn create(&self, account: &DnsProviderAccount) -> Result<i64, AppError>;
    /// 更新可编辑字段（name/provider_type/credentials_json/enabled）与 updated_at。
    async fn update(&self, account: &DnsProviderAccount) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 记录一次连通性测试结果（写 last_test_status / last_test_at / updated_at）。
    async fn record_test(&self, id: i64, ok: bool) -> Result<(), AppError>;
}

/// WAF 规则仓储（主线三 §6.2 · WAF 基础防护）
#[async_trait]
pub trait WafRuleRepository: Send + Sync {
    /// 规则列表（`website_id = Some(id)` 过滤站点维度，`None` 返回全部含全局规则）。
    async fn list_all(&self, website_id: Option<i64>) -> Result<Vec<WafRule>, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<WafRule>, AppError>;
    /// 新建规则，返回自增 id。
    async fn create(&self, rule: &WafRule) -> Result<i64, AppError>;
    /// 更新可编辑字段（name/rule_type/website_id/pattern/config_json/action/enabled）。
    async fn update(&self, rule: &WafRule) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 所有启用规则（引擎每次请求评估用，禁止返回停用规则）。
    async fn list_enabled(&self) -> Result<Vec<WafRule>, AppError>;
}

/// WAF 拦截日志仓储（§6.2：复用 operation_logs 查询/分页/保留清理模式）。
#[async_trait]
pub trait WafLogRepository: Send + Sync {
    async fn create(&self, log: &WafLog) -> Result<i64, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<WafLog>, AppError>;
    /// 分页查询（按 id 倒序）；`website_id` 可选过滤（0 值不过滤）。
    async fn list_page(
        &self,
        limit: i64,
        offset: i64,
        website_id: Option<i64>,
    ) -> Result<Vec<WafLog>, AppError>;
    /// 总数（`website_id` 可选过滤，0 值不过滤）。
    async fn count(&self, website_id: Option<i64>) -> Result<i64, AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
    /// 清空指定网站（0 = 全部）的拦截日志。
    async fn clear(&self, website_id: Option<i64>) -> Result<u64, AppError>;
    /// F-12 对齐：删除早于 N 天的拦截日志（保留策略，后台每日清理）。
    async fn delete_older_than(&self, days: i64) -> Result<u64, AppError>;
}

/// M-AI-1：AI 接入凭证 / MCP 会话 / 工具调用审计仓储（三表同属一个聚合，统一仓储）。
#[async_trait]
pub trait McpRepository: Send + Sync {
    async fn create_principal(
        &self,
        p: &crate::domain::mcp::AgentPrincipal,
    ) -> Result<i64, AppError>;
    async fn list_principals(&self) -> Result<Vec<crate::domain::mcp::AgentPrincipal>, AppError>;
    async fn find_principal(
        &self,
        id: i64,
    ) -> Result<Option<crate::domain::mcp::AgentPrincipal>, AppError>;
    async fn find_principal_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<crate::domain::mcp::AgentPrincipal>, AppError>;
    async fn touch_principal(&self, id: i64, at: DateTime<Utc>) -> Result<(), AppError>;
    async fn revoke_principal(&self, id: i64, at: DateTime<Utc>) -> Result<(), AppError>;

    async fn create_session(&self, s: &crate::domain::mcp::AgentSession) -> Result<(), AppError>;
    async fn find_session(
        &self,
        id: &str,
    ) -> Result<Option<crate::domain::mcp::AgentSession>, AppError>;
    async fn touch_session(&self, id: &str, at: DateTime<Utc>) -> Result<(), AppError>;
    async fn close_session(&self, id: &str, at: DateTime<Utc>) -> Result<(), AppError>;
    /// 按最近活跃倒序；`principal_id` 可选过滤。
    async fn list_sessions(
        &self,
        principal_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<crate::domain::mcp::AgentSession>, AppError>;

    async fn record_call(&self, c: &crate::domain::mcp::ToolCallRecord) -> Result<i64, AppError>;
    /// 按 id 倒序分页。
    async fn list_calls(
        &self,
        filter: &crate::domain::mcp::ToolCallFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::domain::mcp::ToolCallRecord>, AppError>;
    async fn count_calls(
        &self,
        filter: &crate::domain::mcp::ToolCallFilter,
    ) -> Result<i64, AppError>;
    /// 保留策略：只保留最近 `keep` 条调用记录，返回删除条数。
    async fn prune_calls(&self, keep: i64) -> Result<u64, AppError>;

    // ── M-AI-2 审批单 ──
    async fn create_approval(&self, a: &crate::domain::mcp::ToolApproval) -> Result<i64, AppError>;
    async fn find_approval(
        &self,
        id: i64,
    ) -> Result<Option<crate::domain::mcp::ToolApproval>, AppError>;
    /// 按 id 倒序分页。
    async fn list_approvals(
        &self,
        filter: &crate::domain::mcp::ApprovalFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::domain::mcp::ToolApproval>, AppError>;
    async fn count_approvals(
        &self,
        filter: &crate::domain::mcp::ApprovalFilter,
    ) -> Result<i64, AppError>;
    /// 比较并设置：仅当当前状态为 `from` 时改为 `to` 并记录决定人 / 时间 / 理由；返回是否成功。
    /// 保证同一审批单不会被并发批准两次。
    async fn transition_approval(
        &self,
        id: i64,
        from: crate::domain::mcp::ApprovalStatus,
        to: crate::domain::mcp::ApprovalStatus,
        decided_by: Option<i64>,
        at: DateTime<Utc>,
        reason: &str,
    ) -> Result<bool, AppError>;
    /// 执行结束：`executing` → `succeeded` / `failed`，写入结果。
    async fn finish_approval(
        &self,
        id: i64,
        status: crate::domain::mcp::ApprovalStatus,
        result: &str,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    /// 把 `expires_at < now` 的待审批单标为过期，返回条数。
    async fn expire_approvals(&self, now: DateTime<Utc>) -> Result<u64, AppError>;
}
