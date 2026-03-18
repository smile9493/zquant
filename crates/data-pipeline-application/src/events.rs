use async_trait::async_trait;
use job_events::bus::{Event, EventBus};
use job_events::types;
use std::sync::Arc;
use tracing::info;

// Re-export bus event types as the single source of truth.
// Callers construct `types::DatasetFetchedEvent` etc. directly.
pub use types::{
    DatasetFetchedEvent, DatasetGateCompletedEvent, DatasetIngestedEvent, DqDegradedEvent,
    DqIssue, DqRejectionEvent,
};

#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit_dataset_fetched(&self, event: DatasetFetchedEvent) -> anyhow::Result<()>;
    async fn emit_dataset_gate_completed(
        &self,
        event: DatasetGateCompletedEvent,
    ) -> anyhow::Result<()>;
    async fn emit_dataset_ingested(&self, event: DatasetIngestedEvent) -> anyhow::Result<()>;
    async fn emit_dq_rejection(&self, event: DqRejectionEvent) -> anyhow::Result<()>;
    async fn emit_dq_degraded(&self, event: DqDegradedEvent) -> anyhow::Result<()>;
}

pub struct PipelineEventEmitter {
    bus: Option<Arc<dyn EventBus>>,
}

impl PipelineEventEmitter {
    pub fn new(bus: Arc<dyn EventBus>) -> Self {
        Self { bus: Some(bus) }
    }

    pub fn new_noop() -> Self {
        Self { bus: None }
    }
}

impl Default for PipelineEventEmitter {
    fn default() -> Self {
        Self::new_noop()
    }
}

#[async_trait]
impl EventEmitter for PipelineEventEmitter {
    async fn emit_dataset_fetched(&self, event: DatasetFetchedEvent) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            provider = %event.provider,
            row_count = event.row_count,
            "dataset.fetched"
        );
        if let Some(bus) = &self.bus {
            bus.publish(Event::DatasetFetched(event));
        }
        Ok(())
    }

    async fn emit_dataset_gate_completed(
        &self,
        event: DatasetGateCompletedEvent,
    ) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            decision = %event.decision,
            quality_score = event.quality_score,
            issue_count = event.issue_count,
            "dataset.gate.completed"
        );
        if let Some(bus) = &self.bus {
            bus.publish(Event::DatasetGateCompleted(event));
        }
        Ok(())
    }

    async fn emit_dataset_ingested(&self, event: DatasetIngestedEvent) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            decision = %event.decision,
            storage_path = %event.storage_path,
            catalog_id = %event.catalog_id,
            "dataset.ingested"
        );
        if let Some(bus) = &self.bus {
            bus.publish(Event::DatasetIngested(event));
        }
        Ok(())
    }

    async fn emit_dq_rejection(&self, event: DqRejectionEvent) -> anyhow::Result<()> {
        info!(
            quarantine_id = %event.quarantine_id,
            dataset_id = %event.dataset_id,
            reasons = ?event.reasons,
            "dq.rejection"
        );
        if let Some(bus) = &self.bus {
            bus.publish(Event::DqRejection(event));
        }
        Ok(())
    }

    async fn emit_dq_degraded(&self, event: DqDegradedEvent) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            quality_score = event.quality_score,
            issue_count = event.issues.len(),
            "dq.degraded"
        );
        if let Some(bus) = &self.bus {
            bus.publish(Event::DqDegraded(event));
        }
        Ok(())
    }
}
