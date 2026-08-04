use crate::domain::entity::DomainEvent;
use crate::notification::EmailNotifier;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct EventHandler {
    notifier: Option<Arc<EmailNotifier>>,
}

impl EventHandler {
    pub fn new() -> Self {
        Self { notifier: None }
    }

    pub fn with_email(mut self, notifier: Arc<EmailNotifier>) -> Self {
        self.notifier = Some(notifier);
        self
    }

    pub fn spawn(self, mut rx: broadcast::Receiver<DomainEvent>) {
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                tracing::info!("Event received: {:?}", event);
                if let Some(ref notifier) = self.notifier {
                    if let Err(e) = Self::handle_notification(notifier, &event).await {
                        tracing::error!("Notification failed for event {:?}: {}", event, e);
                    }
                }
            }
        });
    }

    async fn handle_notification(
        notifier: &EmailNotifier,
        event: &DomainEvent,
    ) -> Result<(), crate::core::error::AppError> {
        match event {
            DomainEvent::NodeRegistered { node_id, node_name } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        &format!("Node Registered: {}", node_name),
                        &format!("Node {} (ID: {}) has been registered.", node_name, node_id),
                    )
                    .await?;
            }
            DomainEvent::UserCreated { user_id, username } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        &format!("User Created: {}", username),
                        &format!("User {} (ID: {}) has been created.", username, user_id),
                    )
                    .await?;
            }
            DomainEvent::AppInstalled { app_name, version, .. } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        &format!("App Installed: {}", app_name),
                        &format!("{} v{} installed successfully.", app_name, version),
                    )
                    .await?;
            }
            DomainEvent::AppUninstalled { app_name, .. } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        &format!("App Uninstalled: {}", app_name),
                        &format!("{} has been uninstalled.", app_name),
                    )
                    .await?;
            }
            DomainEvent::AppUpgraded { app_name, from, to, .. } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        &format!("App Upgraded: {}", app_name),
                        &format!("{} upgraded from {} to {}.", app_name, from, to),
                    )
                    .await?;
            }
            DomainEvent::BackupCreated { filename } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        "Backup Created",
                        &format!("Database backup created: {}", filename),
                    )
                    .await?;
            }
            DomainEvent::FirewallRulesApplied { rule_count } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        "Firewall Rules Applied",
                        &format!("{} firewall rules applied.", rule_count),
                    )
                    .await?;
            }
            DomainEvent::NodeOffline { node_name, .. } => {
                notifier
                    .send(
                        "admin@flamepanel.local",
                        &format!("Node Offline: {}", node_name),
                        &format!("Node {} has gone offline (heartbeat timeout).", node_name),
                    )
                    .await?;
            }
            _ => {} // quiet other events for now
        }
        Ok(())
    }
}
impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
