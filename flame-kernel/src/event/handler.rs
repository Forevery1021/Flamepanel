use tokio::sync::broadcast;
use crate::domain::entity::DomainEvent;
use crate::notification::EmailNotifier;
use std::sync::Arc;

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

    async fn handle_notification(notifier: &EmailNotifier, event: &DomainEvent) -> Result<(), crate::core::error::AppError> {
        match event {
            DomainEvent::NodeRegistered { node_id, node_name } => {
                notifier.send(
                    "admin@flamepanel.local",
                    &format!("Node Registered: {}", node_name),
                    &format!("Node {} (ID: {}) has been registered.", node_name, node_id),
                ).await?;
            }
            DomainEvent::UserCreated { user_id, username } => {
                notifier.send(
                    "admin@flamepanel.local",
                    &format!("User Created: {}", username),
                    &format!("User {} (ID: {}) has been created.", username, user_id),
                ).await?;
            }
            _ => {} // quiet other events for now
        }
        Ok(())
    }
}
