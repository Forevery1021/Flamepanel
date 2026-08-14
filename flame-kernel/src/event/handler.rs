use crate::domain::entity::DomainEvent;
use crate::notification::AsyncNotificationChannel;
#[cfg(feature = "email")]
use crate::notification::EmailChannel;
#[cfg(feature = "email")]
use crate::notification::EmailNotifier;
use std::sync::Arc;
use tokio::sync::broadcast;

/// 领域事件处理器：负责把事件分发到所有已注册的通知渠道。
///
/// 通过 `AsyncNotificationChannel` 端口与具体通知器解耦（Stage6），
/// 不再直接依赖 `EmailNotifier`，后续可注入站内信 / Webhook 等渠道。
pub struct EventHandler {
    /// 已注册的通知渠道（邮箱 / 站内信 / Webhook 等），可组合多个。
    channels: Vec<Arc<dyn AsyncNotificationChannel>>,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }

    /// 注册一个通知渠道。
    pub fn with_channel(mut self, channel: Arc<dyn AsyncNotificationChannel>) -> Self {
        self.channels.push(channel);
        self
    }

    #[cfg(feature = "email")]
    pub fn with_email(mut self, recipient: &str, notifier: Arc<EmailNotifier>) -> Self {
        self.channels
            .push(Arc::new(EmailChannel::new(recipient, notifier)));
        self
    }

    pub fn spawn(self, rx: broadcast::Receiver<DomainEvent>) {
        self.spawn_with_token(rx, tokio_util::sync::CancellationToken::new())
    }

    /// 带取消令牌启动（由 TaskSupervisor 统一管理生命周期）
    pub fn spawn_with_token(
        self,
        mut rx: broadcast::Receiver<DomainEvent>,
        token: tokio_util::sync::CancellationToken,
    ) {
        tokio::spawn(async move {
            let channels = self.channels;
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::debug!("event handler shutting down");
                        break;
                    }
                    received = rx.recv() => {
                        match received {
                            Ok(event) => {
                                tracing::info!("Event received: {:?}", event);
                                for channel in &channels {
                                    if let Err(e) = channel.notify(&event).await {
                                        tracing::error!(
                                            "Notification via {} failed for event {:?}: {}",
                                            channel.name(),
                                            event,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                // T4：慢消费者滞后时不中断事件流，仅告警并继续。
                                tracing::warn!("event consumer lagged by {n} messages; continuing");
                            }
                            Err(_closed) => {
                                tracing::debug!("event channel closed; event handler stopping");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}
impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
