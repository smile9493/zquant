use anyhow::Result;
use job_application::{HandlerRegistry, Runner};
use job_domain::{JobContext, JobHandler, JobResult, JobStatus};
use job_events::bus::{Event, EventBus, InMemoryEventBus};
use job_events::types::JobCreated;
use job_store_pg::JobStore;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};

struct TestHandler;

#[async_trait::async_trait]
impl JobHandler for TestHandler {
    fn job_types(&self) -> &'static [&'static str] {
        &["test"]
    }

    async fn handle(&self, _ctx: JobContext) -> Result<JobResult> {
        sleep(Duration::from_millis(100)).await;
        Ok(JobResult {
            artifacts: Some(serde_json::json!({"result": "success"})),
        })
    }
}

async fn setup_test_db() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres_e2e".to_string());

    let pool = PgPool::connect(&database_url).await?;

    sqlx::query("TRUNCATE TABLE jobs, jobs_idempotency CASCADE")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[tokio::test]
async fn test_e2e_job_lifecycle() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = Arc::new(JobStore::new(pool));
    let bus = Arc::new(InMemoryEventBus::new(100)) as Arc<dyn EventBus>;
    let mut events_rx = bus.subscribe();

    let mut registry = HandlerRegistry::new();
    registry.register(Arc::new(TestHandler))?;
    let registry = Arc::new(registry);

    let runner = Runner::new(
        store.clone(),
        bus.clone(),
        registry,
        "test-executor".to_string(),
    );

    let runner_clone = runner.clone();
    tokio::spawn(async move {
        runner_clone.run_claim_loop().await;
    });

    // Give runner time to start before creating job
    sleep(Duration::from_millis(200)).await;

    let job = store
        .create_job("test".to_string(), serde_json::json!({}), 0, None)
        .await?;

    bus.publish(Event::JobCreated(JobCreated {
        job_id: job.job_id.clone(),
        job_type: job.job_type.clone(),
        created_at: job.created_at,
    }));

    // Observe lifecycle events for the created job.
    let started_ok = wait_for_event(&mut events_rx, &job.job_id, |e| {
        matches!(e, Event::JobStarted(_))
    })
    .await?;
    assert!(started_ok);
    let completed_ok = wait_for_event(&mut events_rx, &job.job_id, |e| {
        matches!(e, Event::JobCompleted(_))
    })
    .await?;
    assert!(completed_ok);

    for _ in 0..50 {
        sleep(Duration::from_millis(100)).await;
        let current = store.get_job(&job.job_id).await?.unwrap();
        if current.status == JobStatus::Done {
            assert!(current.artifacts.is_some());
            return Ok(());
        }
    }

    panic!("Job did not complete in time");
}

async fn wait_for_event<F>(
    rx: &mut tokio::sync::broadcast::Receiver<Event>,
    job_id: &str,
    predicate: F,
) -> Result<bool>
where
    F: Fn(&Event) -> bool,
{
    let deadline = Duration::from_secs(5);
    let started = tokio::time::Instant::now();

    loop {
        if started.elapsed() > deadline {
            return Ok(false);
        }

        match timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Ok(event)) => {
                let matches_job = match &event {
                    Event::JobCreated(e) => e.job_id == job_id,
                    Event::JobStarted(e) => e.job_id == job_id,
                    Event::JobCompleted(e) => e.job_id == job_id,
                    Event::AgentSpawnRequested(e) => e.job_id == job_id,
                    Event::AgentTaskScheduled(_) => false,
                    Event::AgentMessageProduced(e) => e.job_id == job_id,
                    _ => false,
                };

                if matches_job && predicate(&event) {
                    return Ok(true);
                }
            }
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => return Ok(false),
            Err(_) => continue,
        }
    }
}
