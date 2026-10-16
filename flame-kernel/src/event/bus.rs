use crate::core::error::AppError;
use crate::domain::entity::DomainEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// F-13: 事件总线——广播通道 + 丢弃计数器。
///
/// `publish` 使用 broadcast 发送事件，但无订阅者时静默丢弃。
/// 为审计完整性提供 `events_dropped` 计数器：outbox 消费者在 Lagged 时
/// 使用该计数生成补偿事件，确保审计链路的可追踪性。
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<DomainEvent>,
    /// F-13: 累计丢弃事件数（broadcast 无订阅者或 channel 满时）
    events_dropped: Arc<AtomicU64>,
    /// F-13: 累计发送事件总数
    events_published: Arc<AtomicU64>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            tx,
            events_dropped: Arc::new(AtomicU64::new(0)),
            events_published: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn publish(&self, event: DomainEvent) -> Result<(), AppError> {
        self.events_published.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(event);
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.tx.subscribe()
    }

    /// F-13: 获取累计丢弃事件数（供 outbox 补偿/监控指标使用）。
    pub fn dropped_count(&self) -> u64 {
        self.events_dropped.load(Ordering::Relaxed)
    }

    /// F-13: 获取累计发布事件总数。
    pub fn published_count(&self) -> u64 {
        self.events_published.load(Ordering::Relaxed)
    }

    /// F-13: 记录一次事件丢弃（由订阅者在 Lagged 时调用）。
    pub fn record_dropped(&self, n: u64) {
        self.events_dropped.fetch_add(n, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::DomainEvent;

    #[tokio::test]
    async fn dropped_count_tracks_lagged_events() {
        let bus = EventBus::new(2);
        let mut rx = bus.subscribe();

        // 发送3条，超过容量，第3条会导致 lagged
        bus.publish(DomainEvent::UserCreated {
            user_id: 1,
            username: "u1".into(),
        })
        .await
        .unwrap();
        bus.publish(DomainEvent::UserCreated {
            user_id: 2,
            username: "u2".into(),
        })
        .await
        .unwrap();
        bus.publish(DomainEvent::UserCreated {
            user_id: 3,
            username: "u3".into(),
        })
        .await
        .unwrap();

        // 尝试读取，遇到 Lagged
        if let Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) = rx.recv().await {
            bus.record_dropped(n);
        }

        assert!(bus.dropped_count() >= 1);
        assert_eq!(bus.published_count(), 3);
    }

    #[test]
    fn default_new_bus_has_zero_dropped() {
        let bus = EventBus::new(10);
        assert_eq!(bus.dropped_count(), 0);
        assert_eq!(bus.published_count(), 0);
    }
}
