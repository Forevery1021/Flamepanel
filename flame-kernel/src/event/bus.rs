use tokio::sync::broadcast;
use crate::domain::entity::DomainEvent;
use crate::core::error::AppError;

pub struct EventBus {
    tx: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }
    
    pub async fn publish(&self, event: DomainEvent) -> Result<(), AppError> {
        let _ = self.tx.send(event);
        Ok(())
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.tx.subscribe()
    }
}
