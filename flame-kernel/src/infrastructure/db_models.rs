//! SQLite 持久化行模型（DB Row）。
//!
//! 六边形架构要求：`domain` 不依赖 sqlx，因此所有 `#[derive(sqlx::FromRow)]`
//! 的持久化结构体统一放在 infrastructure 层，与领域实体之间通过
//! `From<XxxRow>` 做显式映射（字段一一对应）。
//!
//! 命名约定：`XxxRow` 表示数据库行；`Xxx` 表示领域实体（`domain::entity`）。

use crate::domain::entity::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub must_change_password: bool,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id,
            username: r.username,
            password_hash: r.password_hash,
            role: r.role,
            created_at: r.created_at,
            must_change_password: r.must_change_password,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerNodeRow {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub metrics_json: Option<String>,
    pub auth_token: Option<String>,
    pub agent_port: i64,
}

impl From<ServerNodeRow> for ServerNode {
    fn from(r: ServerNodeRow) -> Self {
        ServerNode {
            id: r.id,
            name: r.name,
            hostname: r.hostname,
            ip_address: r.ip_address,
            status: r.status,
            created_at: r.created_at,
            last_heartbeat_at: r.last_heartbeat_at,
            metrics_json: r.metrics_json,
            auth_token: r.auth_token,
            agent_port: r.agent_port.max(0) as u16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebsiteRow {
    pub id: i64,
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub status: String,
    pub node_id: i64,
    pub engine: String,
    pub ssl_enabled: bool,
    pub proxy_enabled: bool,
    pub proxy_pass: Option<String>,
    /// 站点类型（Doc/20 §5.5 绑定闭环；旧库迁移默认 static）
    pub site_type: String,
    pub runtime_id: Option<i64>,
    pub project_id: Option<i64>,
    /// 已套用的伪静态模板名（Doc/20 §5.5；旧库迁移默认 NULL = 未套用）
    pub rewrite_preset: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resource_version: i64,
}

impl From<WebsiteRow> for Website {
    fn from(r: WebsiteRow) -> Self {
        Website {
            id: r.id,
            name: r.name,
            domain: r.domain,
            root_path: r.root_path,
            status: r.status,
            node_id: r.node_id,
            engine: r.engine,
            ssl_enabled: r.ssl_enabled,
            proxy_enabled: r.proxy_enabled,
            proxy_pass: r.proxy_pass,
            site_type: r.site_type,
            runtime_id: r.runtime_id,
            project_id: r.project_id,
            rewrite_preset: r.rewrite_preset,
            created_at: r.created_at,
            resource_version: r.resource_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebServerInstanceRow {
    pub id: i64,
    pub engine: String,
    pub version: Option<String>,
    pub status: String,
    pub config_path: String,
    pub binary_path: Option<String>,
    pub port: i32,
    pub created_at: DateTime<Utc>,
    pub resource_version: i64,
}

impl From<WebServerInstanceRow> for WebServerInstance {
    fn from(r: WebServerInstanceRow) -> Self {
        WebServerInstance {
            id: r.id,
            engine: r.engine,
            version: r.version,
            status: r.status,
            config_path: r.config_path,
            binary_path: r.binary_path,
            port: r.port,
            created_at: r.created_at,
            resource_version: r.resource_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DatabaseInstanceRow {
    pub id: i64,
    pub db_type: String,
    pub name: String,
    pub version: String,
    pub port: i32,
    pub status: String,
    pub install_path: String,
    pub data_dir: String,
    pub config_file: String,
    pub root_user: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resource_version: i64,
}

impl From<DatabaseInstanceRow> for DatabaseInstance {
    fn from(r: DatabaseInstanceRow) -> Self {
        DatabaseInstance {
            id: r.id,
            db_type: r.db_type,
            name: r.name,
            version: r.version,
            port: r.port,
            status: r.status,
            install_path: r.install_path,
            data_dir: r.data_dir,
            config_file: r.config_file,
            root_user: r.root_user,
            created_at: r.created_at,
            updated_at: r.updated_at,
            resource_version: r.resource_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PanelSettingRow {
    pub key: String,
    pub value: String,
    pub description: String,
    pub updated_at: DateTime<Utc>,
}

impl From<PanelSettingRow> for PanelSetting {
    fn from(r: PanelSettingRow) -> Self {
        PanelSetting {
            key: r.key,
            value: r.value,
            description: r.description,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct PermissionRow {
    pub id: i64,
    pub resource: String,
    pub action: String,
    pub description: String,
}

impl From<PermissionRow> for Permission {
    fn from(r: PermissionRow) -> Self {
        Permission {
            id: r.id,
            resource: r.resource,
            action: r.action,
            description: r.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RoleRow {
    pub id: i64,
    pub name: String,
    pub description: String,
}

impl From<RoleRow> for Role {
    fn from(r: RoleRow) -> Self {
        Role {
            id: r.id,
            name: r.name,
            description: r.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OperationLogRow {
    pub id: i64,
    pub username: String,
    pub action: String,
    pub target: Option<String>,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<OperationLogRow> for OperationLog {
    fn from(r: OperationLogRow) -> Self {
        OperationLog {
            id: r.id,
            username: r.username,
            action: r.action,
            target: r.target,
            ip: r.ip,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OutboxEventRow {
    pub id: i64,
    pub event_type: String,
    pub payload: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
}

impl From<OutboxEventRow> for OutboxEvent {
    fn from(r: OutboxEventRow) -> Self {
        OutboxEvent {
            id: r.id,
            event_type: r.event_type,
            payload: r.payload,
            published: r.published,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LogEntryRow {
    pub id: i64,
    pub source: String,
    pub level: String,
    pub message: String,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<LogEntryRow> for LogEntry {
    fn from(r: LogEntryRow) -> Self {
        LogEntry {
            id: r.id,
            source: r.source,
            level: r.level,
            message: r.message,
            metadata: r.metadata,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FirewallRuleRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub protocol: String,
    pub port: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub action: String,
    pub enabled: bool,
    pub priority: i32,
    pub direction: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<FirewallRuleRow> for FirewallRule {
    fn from(r: FirewallRuleRow) -> Self {
        FirewallRule {
            id: r.id,
            name: r.name,
            description: r.description,
            protocol: r.protocol,
            port: r.port,
            source: r.source,
            destination: r.destination,
            action: r.action,
            enabled: r.enabled,
            priority: r.priority,
            direction: r.direction,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppPackageRow {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub category: String,
    pub format: String,
    pub description: String,
    pub logo: Option<String>,
    pub metadata_json: String,
    pub source_path: Option<String>,
    pub source: String,
    pub source_url: Option<String>,
    pub synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AppPackageRow> for AppPackage {
    fn from(r: AppPackageRow) -> Self {
        AppPackage {
            id: r.id,
            key: r.key,
            name: r.name,
            category: r.category,
            format: r.format,
            description: r.description,
            logo: r.logo,
            metadata_json: r.metadata_json,
            source_path: r.source_path,
            source: r.source,
            source_url: r.source_url,
            synced_at: r.synced_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// 远程应用源行（M-A Issue 1.1）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppSourceRow {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub enabled: bool,
    pub public_key: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AppSourceRow> for AppSource {
    fn from(r: AppSourceRow) -> Self {
        AppSource {
            id: r.id,
            name: r.name,
            source_type: r.source_type,
            url: r.url,
            enabled: r.enabled,
            public_key: r.public_key,
            last_synced_at: r.last_synced_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstalledAppRow {
    pub id: i64,
    pub package_key: String,
    pub name: String,
    pub version: String,
    pub mode: String,
    pub status: String,
    pub access_url: Option<String>,
    pub install_path: String,
    pub container_name: Option<String>,
    pub port: Option<i32>,
    pub params_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub launch_count: i64,
}

impl From<InstalledAppRow> for InstalledApp {
    fn from(r: InstalledAppRow) -> Self {
        InstalledApp {
            id: r.id,
            package_key: r.package_key,
            name: r.name,
            version: r.version,
            mode: r.mode,
            status: r.status,
            access_url: r.access_url,
            install_path: r.install_path,
            container_name: r.container_name,
            port: r.port,
            params_json: r.params_json,
            created_at: r.created_at,
            updated_at: r.updated_at,
            launch_count: r.launch_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemoRow {
    pub id: i64,
    pub content: String,
    pub kind: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MemoRow> for Memo {
    fn from(r: MemoRow) -> Self {
        Memo {
            id: r.id,
            content: r.content,
            kind: r.kind,
            done: r.done,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledTaskRow {
    pub id: i64,
    pub name: String,
    pub command: String,
    pub schedule: String,
    pub enabled: bool,
    pub last_status: String,
    pub last_output: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ScheduledTaskRow> for ScheduledTask {
    fn from(r: ScheduledTaskRow) -> Self {
        ScheduledTask {
            id: r.id,
            name: r.name,
            command: r.command,
            schedule: r.schedule,
            enabled: r.enabled,
            last_status: r.last_status,
            last_output: r.last_output,
            last_run_at: r.last_run_at,
            next_run_at: r.next_run_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// 备份账号行（M-A 收尾 Stage 1 · Doc/20 §6.5）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupAccountRow {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub config_json: String,
    pub enabled: bool,
    pub last_test_status: Option<String>,
    pub last_test_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BackupAccountRow> for BackupAccount {
    fn from(r: BackupAccountRow) -> Self {
        BackupAccount {
            id: r.id,
            name: r.name,
            account_type: r.account_type,
            config_json: r.config_json,
            enabled: r.enabled,
            last_test_status: r.last_test_status,
            last_test_at: r.last_test_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// 备份任务行（M-A 收尾 Stage 3 · Doc/20 §6.5）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupJobRow {
    pub id: i64,
    pub name: String,
    pub scope: String,
    pub scope_key: Option<String>,
    pub schedule: String,
    pub target_account_id: i64,
    pub retention: i64,
    pub enabled: bool,
    pub last_status: String,
    pub last_message: Option<String>,
    pub last_remote_key: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BackupJobRow> for BackupJob {
    fn from(r: BackupJobRow) -> Self {
        BackupJob {
            id: r.id,
            name: r.name,
            scope: r.scope,
            scope_key: r.scope_key,
            schedule: r.schedule,
            target_account_id: r.target_account_id,
            retention: r.retention,
            enabled: r.enabled,
            last_status: r.last_status,
            last_message: r.last_message,
            last_remote_key: r.last_remote_key,
            last_run_at: r.last_run_at,
            next_run_at: r.next_run_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// 运行时行（M-B 主线二 Stage A-1 · Doc/20 §5.2）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RuntimeRow {
    pub id: i64,
    pub kind: String,
    pub version: String,
    pub install_path: String,
    pub status: String,
    pub is_default: bool,
    pub metadata_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RuntimeRow> for Runtime {
    fn from(r: RuntimeRow) -> Self {
        Runtime {
            id: r.id,
            kind: r.kind,
            version: r.version,
            install_path: r.install_path,
            status: r.status,
            is_default: r.is_default,
            metadata_json: r.metadata_json,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// 用户项目行（M-B 主线二 Stage A-1 · Doc/20 §5.2）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectRow {
    pub id: i64,
    pub name: String,
    pub runtime_id: Option<i64>,
    pub kind: String,
    pub source: String,
    pub source_url: Option<String>,
    pub deploy_path: String,
    pub run_command: Option<String>,
    pub port: Option<i64>,
    pub domain: Option<String>,
    pub status: String,
    pub process_unit: Option<String>,
    pub deploy_type: String,
    pub auto_deploy: bool,
    pub webhook_token: String,
    pub procfile_json: Option<String>,
    pub build_type: String,
    pub health_check_path: String,
    pub deploy_strategy: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProjectRow> for Project {
    fn from(r: ProjectRow) -> Self {
        Project {
            id: r.id,
            name: r.name,
            runtime_id: r.runtime_id,
            kind: r.kind,
            source: r.source,
            source_url: r.source_url,
            deploy_path: r.deploy_path,
            run_command: r.run_command,
            port: r.port,
            domain: r.domain,
            status: r.status,
            process_unit: r.process_unit,
            deploy_type: r.deploy_type,
            auto_deploy: r.auto_deploy,
            webhook_token: r.webhook_token,
            procfile_json: r.procfile_json,
            build_type: r.build_type,
            health_check_path: r.health_check_path,
            deploy_strategy: r.deploy_strategy,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// DNS 厂商账号行（主线三 §6.1 · Stage 4）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DnsProviderAccountRow {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub credentials_json: String,
    pub enabled: bool,
    pub last_test_status: Option<String>,
    pub last_test_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DnsProviderAccountRow> for DnsProviderAccount {
    fn from(r: DnsProviderAccountRow) -> Self {
        DnsProviderAccount {
            id: r.id,
            name: r.name,
            provider_type: r.provider_type,
            credentials_json: r.credentials_json,
            enabled: r.enabled,
            last_test_status: r.last_test_status,
            last_test_at: r.last_test_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// WAF 规则行（主线三 §6.2）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WafRuleRow {
    pub id: i64,
    pub name: String,
    pub rule_type: String,
    pub website_id: i64,
    pub pattern: String,
    pub config_json: String,
    pub action: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<WafRuleRow> for WafRule {
    fn from(r: WafRuleRow) -> Self {
        WafRule {
            id: r.id,
            name: r.name,
            rule_type: r.rule_type,
            website_id: r.website_id,
            pattern: r.pattern,
            config_json: r.config_json,
            action: r.action,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// WAF 拦截日志行（主线三 §6.2）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WafLogRow {
    pub id: i64,
    pub website_id: i64,
    pub rule_id: i64,
    pub rule_name: String,
    pub rule_type: String,
    pub action: String,
    pub client_ip: String,
    pub request_summary: String,
    pub user_agent: String,
    pub created_at: DateTime<Utc>,
}

impl From<WafLogRow> for WafLog {
    fn from(r: WafLogRow) -> Self {
        WafLog {
            id: r.id,
            website_id: r.website_id,
            rule_id: r.rule_id,
            rule_name: r.rule_name,
            rule_type: r.rule_type,
            action: r.action,
            client_ip: r.client_ip,
            request_summary: r.request_summary,
            user_agent: r.user_agent,
            created_at: r.created_at,
        }
    }
}
