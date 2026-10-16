//! 领域事件（EventBus 载荷）。

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DomainEvent {
    UserCreated {
        user_id: i64,
        username: String,
    },
    NodeRegistered {
        node_id: i64,
        node_name: String,
    },
    WebsiteCreated {
        website_id: i64,
        domain: String,
    },
    NodeHeartbeat {
        node_id: i64,
        node_name: String,
    },
    AppInstalled {
        app_key: String,
        app_name: String,
        version: String,
    },
    AppUninstalled {
        app_key: String,
        app_name: String,
    },
    AppUpgraded {
        app_key: String,
        app_name: String,
        from: String,
        to: String,
    },
    FirewallRulesApplied {
        rule_count: usize,
    },
    BackupCreated {
        filename: String,
    },
    /// 计划备份任务执行成功（M-A 收尾 Stage 3 · Doc/20 §6.5）
    BackupSucceeded {
        job_id: i64,
        job_name: String,
        remote_key: String,
    },
    /// 计划备份任务执行失败
    BackupFailed {
        job_id: i64,
        job_name: String,
        reason: String,
    },
    UserLoggedIn {
        username: String,
    },
    PasswordChanged {
        username: String,
    },
    NodeOffline {
        node_id: i64,
        node_name: String,
    },
    /// 首次部署向导完成（初始化流程终态）
    SetupCompleted {
        username: String,
    },
    /// F-13: 审计补偿事件（outbox 滞后时记录丢失事件数，保证审计链路完整）
    AuditEvent {
        message: String,
    },
    /// 证书签发/续签成功（主线三 §6.1 · Stage 2）：审计与通知
    CertificateIssued {
        cert_id: i64,
        domains: String,
        source: String,
    },
    /// 证书自动续签成功（主线三 §6.1 · Stage 3 · T-08）
    CertificateRenewed {
        id: i64,
        domain: String,
    },
    /// 证书自动续签失败（主线三 §6.1 · Stage 3 · T-08）
    CertificateRenewalFailed {
        id: i64,
        domain: String,
        error: String,
    },
    /// 证书即将到期（主线三 §6.1 Stage 3 续签扫描预警预留）
    CertificateExpiring {
        cert_id: i64,
        domains: String,
        expires_at: String,
    },
    /// 零停机切换失败（Stage A-9 · Doc/20 §5.8 ④）：
    /// 新实例健康检查未通过，已回滚旧实例继续服务，需要人工介入排查。
    ProjectZeroDowntimeFailed {
        project_id: i64,
        project_name: String,
        reason: String,
    },
    /// WAF 命中阻断动作（主线三 §6.2 · 拦截通知）：
    /// 按规则节流聚合后发布，`hit_count` 为本次通知覆盖的命中次数
    /// （窗口内后续命中只累计、不重复通知，防攻击期间告警风暴）。
    WafBlocked {
        /// 请求所属站点 ID（0 = 全局规则命中）
        website_id: i64,
        rule_id: i64,
        rule_name: String,
        rule_type: String,
        client_ip: String,
        /// 请求行摘要（`GET /path?query`，截断后）
        request_summary: String,
        /// 本次通知覆盖的命中次数（节流窗口内累计）
        hit_count: u32,
    },
}
