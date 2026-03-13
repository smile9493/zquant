use crate::types::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum Event {
    JobCreated(JobCreated),
    JobStarted(JobStarted),
    JobCompleted(JobCompleted),
    AgentSpawnRequested(AgentSpawnRequested),
    AgentTaskScheduled(AgentTaskScheduled),
    AgentMessageProduced(AgentMessageProduced),
}

pub trait EventBus: Send + Sync {
    fn publish(&self, event: Event);
    fn subscribe(&self) -> broadcast::Receiver<Event>;
}

#[derive(Clone)]
pub struct InMemoryEventBus {
    sender: broadcast::Sender<Event>,
    publish_total: Arc<AtomicU64>,
    publish_no_subscribers_total: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EventBusStats {
    pub publish_total: u64,
    pub publish_no_subscribers_total: u64,
}

impl InMemoryEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            publish_total: Arc::new(AtomicU64::new(0)),
            publish_no_subscribers_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn stats(&self) -> EventBusStats {
        EventBusStats {
            publish_total: self.publish_total.load(Ordering::Relaxed),
            publish_no_subscribers_total: self.publish_no_subscribers_total.load(Ordering::Relaxed),
        }
    }
}

impl EventBus for InMemoryEventBus {
    fn publish(&self, event: Event) {
        self.publish_total.fetch_add(1, Ordering::Relaxed);
        if let Err(e) = self.sender.send(event) {
            self.publish_no_subscribers_total
                .fetch_add(1, Ordering::Relaxed);
            tracing::warn!("EventBus publish failed (no subscribers): {:?}", e);
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::JobCreated;
    use chrono::Utc;

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = InMemoryEventBus::new(10);
        let mut rx = bus.subscribe();

        let event = Event::JobCreated(JobCreated {
            job_id: "job_1".to_string(),
            job_type: "test".to_string(),
            created_at: Utc::now(),
        });

        bus.publish(event.clone());

        let received = rx.recv().await.unwrap();
        match received {
            Event::JobCreated(e) => assert_eq!(e.job_id, "job_1"),
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = InMemoryEventBus::new(10);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = Event::JobCreated(JobCreated {
            job_id: "job_1".to_string(),
            job_type: "test".to_string(),
            created_at: Utc::now(),
        });

        bus.publish(event);

        let r1 = rx1.recv().await.unwrap();
        let r2 = rx2.recv().await.unwrap();

        match (r1, r2) {
            (Event::JobCreated(e1), Event::JobCreated(e2)) => {
                assert_eq!(e1.job_id, "job_1");
                assert_eq!(e2.job_id, "job_1");
            }
            _ => panic!("Wrong event types"),
        }
    }

    #[tokio::test]
    async fn test_stats_count_no_subscribers() {
        let bus = InMemoryEventBus::new(10);
        let event = Event::JobCreated(JobCreated {
            job_id: "job_1".to_string(),
            job_type: "test".to_string(),
            created_at: Utc::now(),
        });

        bus.publish(event);
        let stats = bus.stats();
        assert_eq!(stats.publish_total, 1);
        assert_eq!(stats.publish_no_subscribers_total, 1);
    }
}
