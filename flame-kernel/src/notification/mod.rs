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
            DomainEvent::FirewallRulesApplied { rule_count } => Some((
                "Firewall Rules Applied".to_string(),
                format!("{} firewall rules applied.", rule_count),
            )),
            DomainEvent::SetupCompleted { username } => Some((
                "Setup Completed".to_string(),
                format!("Panel initialized by {}.", username),
            )),
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
