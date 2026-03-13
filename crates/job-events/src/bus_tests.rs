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
    async fn test_buffer_overflow() {
        let bus = InMemoryEventBus::new(2);
        let mut rx = bus.subscribe();

        for i in 0..5 {
            bus.publish(Event::JobCreated(JobCreated {
                job_id: format!("job_{}", i),
                job_type: "test".to_string(),
                created_at: Utc::now(),
            }));
        }

        // Should receive some events, but may miss some due to overflow
        let result = rx.recv().await;
        assert!(result.is_ok() || matches!(result, Err(tokio::sync::broadcast::error::RecvError::Lagged(_))));
    }
}
