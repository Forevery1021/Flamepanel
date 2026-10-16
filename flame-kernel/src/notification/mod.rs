use crate::core::error::AppError;
use crate::domain::entity::DomainEvent;

#[cfg(feature = "email")]
use std::sync::Arc;

/// SMTP 通知配置（feature `email` 开启时使用，关闭时仅保留纯数据字段）。
#[cfg(feature = "email")]
use lettre::message::header::ContentType;
#[cfg(feature = "email")]
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

/// 通知渠道抽象端口（Stage6 事件驱动深化）。
///
/// 任何实现方（邮件 / Webhook / 站内信）只需消费 `DomainEvent` 并决定如何处置。
/// `EventHandler` 不再依赖具体通知器，改为持有 `Vec<Arc<dyn AsyncNotificationChannel>>`，
/// 可组合多个渠道并行下发，并便于后续扩展站内信 / Webhook。
#[async_trait::async_trait]
pub trait AsyncNotificationChannel: Send + Sync {
    fn name(&self) -> &str;
    /// 处理一条领域事件；返回 Err 仅表示该渠道自身失败（不影响其他渠道）。
    async fn notify(&self, event: &DomainEvent) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub use_tls: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 25,
            username: String::new(),
            password: String::new(),
            from: "noreply@flamepanel.local".into(),
            use_tls: false,
        }
    }
}

/// 邮件通知器（feature `email` 开启时可用）。
/// 关闭 `email` feature 时该类型不参与编译，事件通知退化为仅记录日志。
#[cfg(feature = "email")]
pub struct EmailNotifier {
    config: SmtpConfig,
}

#[cfg(feature = "email")]
impl EmailNotifier {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// SMTP 是否已配置：默认值（localhost:25 且无账号密码）视为未配置，
    /// 事件通知应静默跳过而不是反复报连接失败。
    pub fn is_configured(&self) -> bool {
        !(self.config.host == "localhost"
            && self.config.username.is_empty()
            && self.config.password.is_empty())
    }

    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(
                self.config
                    .from
                    .parse()
                    .map_err(|e| AppError::internal(format!("Invalid from: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AppError::internal(format!("Invalid to: {}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| AppError::internal(format!("Email build: {}", e)))?;

        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)
            .map_err(|e| AppError::internal(format!("SMTP relay: {}", e)))?
            .port(self.config.port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| AppError::internal(format!("Send email: {}", e)))?;
        Ok(())
    }
}

/// 邮件通知渠道：把领域事件映射为邮件主题/正文后经 `EmailNotifier` 下发。
///
/// 仅对需要通知的事件产生邮件；其余事件静默返回 `Ok`（避免空邮件）。
#[cfg(feature = "email")]
pub struct EmailChannel {
    recipient: String,
    notifier: Arc<EmailNotifier>,
}

#[cfg(feature = "email")]
impl EmailChannel {
    pub fn new(recipient: &str, notifier: Arc<EmailNotifier>) -> Self {
        Self {
            recipient: recipient.to_string(),
            notifier,
        }
    }

    fn render(&self, event: &DomainEvent) -> Option<(String, String)> {
        match event {
            DomainEvent::NodeRegistered { node_id, node_name } => Some((
                format!("Node Registered: {}", node_name),
                format!("Node {} (ID: {}) has been registered.", node_name, node_id),
            )),
            DomainEvent::NodeHeartbeat { node_name, .. } => Some((
                format!("Node Heartbeat: {}", node_name),
                format!("Node {} reported a heartbeat.", node_name),
            )),
            DomainEvent::NodeOffline { node_name, .. } => Some((
                format!("Node Offline: {}", node_name),
                format!("Node {} has gone offline (heartbeat timeout).", node_name),
            )),
            DomainEvent::UserCreated { user_id, username } => Some((
                format!("User Created: {}", username),
                format!("User {} (ID: {}) has been created.", username, user_id),
            )),
            DomainEvent::UserLoggedIn { username } => Some((
                format!("User Logged In: {}", username),
                format!("User {} has logged in.", username),
            )),
            DomainEvent::PasswordChanged { username } => Some((
                format!("Password Changed: {}", username),
                format!("The password for user {} was changed.", username),
            )),
            DomainEvent::WebsiteCreated { website_id, domain } => Some((
                format!("Website Created: {}", domain),
                format!("Website {} (ID: {}) has been created.", domain, website_id),
            )),
            DomainEvent::AppInstalled {
                app_name, version, ..
            } => Some((
                format!("App Installed: {}", app_name),
                format!("{} v{} installed successfully.", app_name, version),
            )),
            DomainEvent::AppUninstalled { app_name, .. } => Some((
                format!("App Uninstalled: {}", app_name),
                format!("{} has been uninstalled.", app_name),
            )),
            DomainEvent::AppUpgraded {
                app_name, from, to, ..
            } => Some((
                format!("App Upgraded: {}", app_name),
                format!("{} upgraded from {} to {}.", app_name, from, to),
            )),
            DomainEvent::BackupCreated { filename } => Some((
                "Backup Created".to_string(),
                format!("Database backup created: {}", filename),
            )),
            DomainEvent::BackupSucceeded {
                job_name,
                remote_key,
                ..
            } => Some((
                "Backup Succeeded".to_string(),
                format!("Backup job \"{}\" succeeded: {}", job_name, remote_key),
            )),
            DomainEvent::BackupFailed {
                job_name, reason, ..
            } => Some((
                "Backup Failed".to_string(),
                format!("Backup job \"{}\" failed: {}", job_name, reason),
            )),
            DomainEvent::FirewallRulesApplied { rule_count } => Some((
                "Firewall Rules Applied".to_string(),
                format!("{} firewall rules applied.", rule_count),
            )),
            DomainEvent::SetupCompleted { username } => Some((
                "Setup Completed".to_string(),
                format!("Panel initialized by {}.", username),
            )),
            DomainEvent::ProjectZeroDowntimeFailed {
                project_name,
                reason,
                ..
            } => Some((
                "Zero-Downtime Deploy Failed".to_string(),
                format!(
                    "Project \"{}\" zero-downtime switch failed, rolled back to old instance: {}",
                    project_name, reason
                ),
            )),
            DomainEvent::CertificateIssued {
                domains, source, ..
            } => Some((
                "Certificate Issued".to_string(),
                format!(
                    "Certificate for {} (source: {}) issued/renewed successfully.",
                    domains, source
                ),
            )),
            DomainEvent::CertificateRenewed { id, domain } => Some((
                "Certificate Renewed".to_string(),
                format!(
                    "Certificate #{} for {} has been renewed successfully.",
                    id, domain
                ),
            )),
            DomainEvent::CertificateRenewalFailed { id, domain, error } => Some((
                "Certificate Renewal Failed".to_string(),
                format!(
                    "Failed to renew certificate #{} for {}: {}.",
                    id, domain, error
                ),
            )),
            DomainEvent::CertificateExpiring {
                domains,
                expires_at,
                ..
            } => Some((
                "Certificate Expiring".to_string(),
                format!("Certificate for {} expires at {}.", domains, expires_at),
            )),
            DomainEvent::WafBlocked {
                rule_name,
                rule_type,
                client_ip,
                request_summary,
                hit_count,
                ..
            } => Some((
                format!("WAF Blocked: {}", rule_name),
                format!(
                    "WAF rule \"{}\" (type: {}) blocked a request from {}: {}\n\
                     Blocked hits in this notification window: {}.",
                    rule_name, rule_type, client_ip, request_summary, hit_count
                ),
            )),
            // F-13: 审计补偿事件不触发通知
            DomainEvent::AuditEvent { .. } => None,
        }
    }
}

#[cfg(feature = "email")]
#[async_trait::async_trait]
impl AsyncNotificationChannel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn notify(&self, event: &DomainEvent) -> Result<(), AppError> {
        if !self.notifier.is_configured() {
            tracing::debug!(
                "SMTP not configured; skipping email notification for {:?}",
                event
            );
            return Ok(());
        }
        if let Some((subject, body)) = self.render(event) {
            self.notifier.send(&self.recipient, &subject, &body).await
        } else {
            Ok(())
        }
    }
}

#[cfg(all(test, feature = "email"))]
mod email_render_tests {
    use super::*;
    use crate::domain::entity::DomainEvent;

    fn channel() -> EmailChannel {
        EmailChannel::new(
            "admin@flamepanel.local",
            Arc::new(EmailNotifier::new(SmtpConfig::default())),
        )
    }

    /// §6.2 拦截通知：`WafBlocked` 渲染为主题 + 正文（含规则类型 / 来源 IP /
    /// 请求行摘要 / 窗口累计次数），且正文不含敏感头信息（只有请求行摘要）。
    #[test]
    fn renders_waf_blocked_notification() {
        let (subject, body) = channel()
            .render(&DomainEvent::WafBlocked {
                website_id: 3,
                rule_id: 7,
                rule_name: "sqli-guard".into(),
                rule_type: "sql_injection".into(),
                client_ip: "9.9.9.9".into(),
                request_summary: "GET /login?id=1' OR '1'='1".into(),
                hit_count: 12,
            })
            .expect("WafBlocked 应有邮件映射");
        assert_eq!(subject, "WAF Blocked: sqli-guard");
        assert!(body.contains("sql_injection"));
        assert!(body.contains("9.9.9.9"));
        assert!(body.contains("GET /login"));
        assert!(body.contains("12"), "窗口累计次数应透出: {body}");
    }

    /// 静默事件（含审计补偿）不得产生空邮件。
    #[test]
    fn skips_silent_events() {
        let c = channel();
        assert!(c
            .render(&DomainEvent::AuditEvent {
                message: "OUTBOX_LAGGED:3".into()
            })
            .is_none());
        assert!(
            c.render(&DomainEvent::CertificateExpiring {
                cert_id: 1,
                domains: "example.com".into(),
                expires_at: "2026-12-31T00:00:00Z".into(),
            })
            .is_some(),
            "到期预警应有邮件映射"
        );
        assert!(
            c.render(&DomainEvent::CertificateRenewed {
                id: 1,
                domain: "example.com".into(),
            })
            .is_some(),
            "续签成功应有邮件映射"
        );
        assert!(
            c.render(&DomainEvent::CertificateRenewalFailed {
                id: 1,
                domain: "example.com".into(),
                error: "timeout".into(),
            })
            .is_some(),
            "续签失败应有邮件映射"
        );
    }

    /// 未配置 SMTP 时静默跳过（不发信也不报错）。
    #[tokio::test]
    async fn notify_skips_when_smtp_unconfigured() {
        let c = channel();
        c.notify(&DomainEvent::WafBlocked {
            website_id: 0,
            rule_id: 1,
            rule_name: "bl".into(),
            rule_type: "ip_blacklist".into(),
            client_ip: "1.2.3.4".into(),
            request_summary: "GET /".into(),
            hit_count: 1,
        })
        .await
        .expect("未配置 SMTP 应静默成功");
    }
}
