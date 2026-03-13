use anyhow::Result;
use chrono::Utc;
use job_domain::{JobContext, JobHandler, JobResult, JobStatus};
use job_events::bus::{Event, EventBus};
use job_events::types::{JobCompleted, JobStarted};
use job_store_pg::JobStore;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::{interval, timeout};

pub struct HandlerRegistry {
    handlers: HashMap<String, Arc<dyn JobHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn JobHandler>) -> Result<()> {
        for job_type in handler.job_types() {
            if self.handlers.contains_key(*job_type) {
                anyhow::bail!("Duplicate job_type: {}", job_type);
            }
            self.handlers.insert(job_type.to_string(), handler.clone());
        }
        Ok(())
    }

    pub fn get(&self, job_type: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(job_type).cloned()
    }

    pub fn job_types(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

pub struct Runner {
    store: Arc<JobStore>,
    bus: Arc<dyn EventBus>,
    registry: Arc<HandlerRegistry>,
    executor_id: String,
    lagged_event_total: Arc<AtomicU64>,
    claimed_total: Arc<AtomicU64>,
    completed_total: Arc<AtomicU64>,
    errored_total: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RunnerStats {
    pub lagged_event_total: u64,
    pub claimed_total: u64,
    pub completed_total: u64,
    pub errored_total: u64,
}

impl Runner {
    pub fn new(
        store: Arc<JobStore>,
        bus: Arc<dyn EventBus>,
        registry: Arc<HandlerRegistry>,
        executor_id: String,
    ) -> Self {
        Self {
            store,
            bus,
            registry,
            executor_id,
            lagged_event_total: Arc::new(AtomicU64::new(0)),
            claimed_total: Arc::new(AtomicU64::new(0)),
            completed_total: Arc::new(AtomicU64::new(0)),
            errored_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn stats(&self) -> RunnerStats {
        RunnerStats {
            lagged_event_total: self.lagged_event_total.load(Ordering::Relaxed),
            claimed_total: self.claimed_total.load(Ordering::Relaxed),
            completed_total: self.completed_total.load(Ordering::Relaxed),
            errored_total: self.errored_total.load(Ordering::Relaxed),
        }
    }

    pub async fn run_claim_loop(&self) {
        let mut rx = self.bus.subscribe();
        let mut poll_interval = interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                recv = rx.recv() => {
                    match recv {
                        Ok(Event::JobCreated(_)) => self.try_claim().await,
                        Ok(_) => {},
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            let total = self.lagged_event_total.fetch_add(1, Ordering::Relaxed) + 1;
                            tracing::warn!(
                                skipped,
                                lagged_event_total = total,
                                "Event bus lagged while receiving events"
                            );
                            // Best-effort bus: lag is expected under load; rely on polling to catch up.
                            self.try_claim().await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = poll_interval.tick() => {
                    self.try_claim().await;
                }
            }
        }
    }

    async fn try_claim(&self) {
        let job_types = self.registry.job_types();
        if job_types.is_empty() {
            return;
        }

        match self
            .store
            .claim_jobs(&self.executor_id, 30000, 10, &job_types)
            .await
        {
            Ok(jobs) => {
                let claimed = jobs.len();
                if claimed > 0 {
                    let total = self
                        .claimed_total
                        .fetch_add(claimed as u64, Ordering::Relaxed)
                        + claimed as u64;
                    tracing::info!(
                        executor_id = %self.executor_id,
                        claimed,
                        claimed_total = total,
                        "Claimed jobs"
                    );
                }

                for job in jobs {
                    tracing::info!(
                        job_id = %job.job_id,
                        job_type = %job.job_type,
                        executor_id = %self.executor_id,
                        lease_until_ms = job.lease_until_ms.unwrap_or(0),
                        "Job started (claimed)"
                    );

                    self.bus.publish(Event::JobStarted(JobStarted {
                        job_id: job.job_id.clone(),
                        executor_id: self.executor_id.clone(),
                        lease_until_ms: job.lease_until_ms.unwrap_or(0),
                    }));

                    let runner = self.clone();
                    tokio::spawn(async move {
                        runner.execute_job(job).await;
                    });
                }
            }
            Err(e) => tracing::warn!("Claim failed: {:?}", e),
        }
    }

    async fn execute_job(&self, job: job_domain::Job) {
        let start = Utc::now();
        tracing::info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            executor_id = %self.executor_id,
            "Executing job"
        );

        let handler = match self.registry.get(&job.job_type) {
            Some(h) => h,
            None => {
                tracing::error!(job_id = %job.job_id, job_type = %job.job_type, "No handler for job_type");
                return;
            }
        };

        let ctx = JobContext {
            job_id: job.job_id.clone(),
            job_type: job.job_type.clone(),
            payload: job.payload.clone(),
        };

        // Run handler in a task so panics are isolated and observable via JoinError.
        let handler_task = tokio::spawn(async move { handler.handle(ctx).await });
        let result = timeout(Duration::from_secs(300), handler_task).await;

        let (status, artifacts, error) = match result {
            Ok(Ok(Ok(JobResult { artifacts }))) => (JobStatus::Done, artifacts, None),
            Ok(Ok(Err(e))) => (
                JobStatus::Error,
                None,
                Some(serde_json::json!({"error": e.to_string()})),
            ),
            Ok(Err(join_err)) => {
                let reason = if join_err.is_panic() {
                    "panic"
                } else {
                    "cancelled"
                };
                (
                    JobStatus::Error,
                    None,
                    Some(serde_json::json!({"error": reason})),
                )
            }
            Err(_) => (
                JobStatus::Error,
                None,
                Some(serde_json::json!({"error": "timeout"})),
            ),
        };

        let duration_ms = (Utc::now() - start).num_milliseconds();

        let finalized = match self
            .store
            .finalize_job(
                &job.job_id,
                &self.executor_id,
                job.lease_version,
                status,
                artifacts.clone(),
                error.clone(),
            )
            .await
        {
            Ok(true) => true,
            Ok(false) => {
                tracing::warn!(
                    job_id = %job.job_id,
                    executor_id = %self.executor_id,
                    "Finalize skipped due to fencing/state mismatch"
                );
                return;
            }
            Err(e) => {
                tracing::error!(job_id = %job.job_id, executor_id = %self.executor_id, err = ?e, "Finalize failed");
                return;
            }
        };

        if finalized {
            let status_str = format!("{:?}", status).to_lowercase();

            if status_str == "done" {
                let total = self.completed_total.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::info!(
                    job_id = %job.job_id,
                    job_type = %job.job_type,
                    executor_id = %self.executor_id,
                    duration_ms,
                    completed_total = total,
                    "Job completed"
                );
            } else {
                let total = self.errored_total.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::warn!(
                    job_id = %job.job_id,
                    job_type = %job.job_type,
                    executor_id = %self.executor_id,
                    duration_ms,
                    errored_total = total,
                    err = ?error,
                    "Job completed with non-done status"
                );
            }

            self.bus.publish(Event::JobCompleted(JobCompleted {
                job_id: job.job_id,
                status: status_str,
                duration_ms,
                error,
                artifacts,
            }));
        }
    }

    pub async fn run_sweep_loop(&self) {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let now_ms = Utc::now().timestamp_millis();
            match self.store.reap_expired_jobs(now_ms, 100).await {
                Ok(jobs) => {
                    if !jobs.is_empty() {
                        tracing::info!("Reaped {} expired jobs", jobs.len());
                    }
                }
                Err(e) => tracing::warn!("Sweep failed: {:?}", e),
            }
        }
    }
}

impl Clone for Runner {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            bus: self.bus.clone(),
            registry: self.registry.clone(),
            executor_id: self.executor_id.clone(),
            lagged_event_total: self.lagged_event_total.clone(),
            claimed_total: self.claimed_total.clone(),
            completed_total: self.completed_total.clone(),
            errored_total: self.errored_total.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler1;
    #[async_trait::async_trait]
    impl JobHandler for TestHandler1 {
        fn job_types(&self) -> &'static [&'static str] {
            &["test1"]
        }
        async fn handle(&self, _ctx: JobContext) -> Result<JobResult> {
            Ok(JobResult { artifacts: None })
        }
    }

    struct TestHandler2;
    #[async_trait::async_trait]
    impl JobHandler for TestHandler2 {
        fn job_types(&self) -> &'static [&'static str] {
            &["test1"]
        }
        async fn handle(&self, _ctx: JobContext) -> Result<JobResult> {
            Ok(JobResult { artifacts: None })
        }
    }

    #[test]
    fn test_registry_duplicate_fails() {
        let mut registry = HandlerRegistry::new();
        registry.register(Arc::new(TestHandler1)).unwrap();
        let result = registry.register(Arc::new(TestHandler2));
        assert!(result.is_err());
    }
}
