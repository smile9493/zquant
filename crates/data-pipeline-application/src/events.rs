use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_pipeline_domain::{Capability, DqDecision, DqIssue, Market};
use job_events::bus::{Event, EventBus};
use std::sync::Arc;
use tracing::info;

pub struct DatasetFetchedEvent {
    pub dataset_id: String,
    pub provider: String,
    pub capability: Capability,
    pub market: Market,
    pub timestamp: DateTime<Utc>,
    pub row_count: usize,
}

pub struct DatasetGateCompletedEvent {
    pub dataset_id: String,
    pub decision: DqDecision,
    pub quality_score: f64,
    pub issue_count: usize,
    pub timestamp: DateTime<Utc>,
}

pub struct DatasetIngestedEvent {
    pub dataset_id: String,
    pub decision: DqDecision,
    pub storage_path: String,
    pub catalog_id: String,
    pub timestamp: DateTime<Utc>,
}

pub struct DqRejectionEvent {
    pub quarantine_id: String,
    pub dataset_id: String,
    pub reasons: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

pub struct DqDegradedEvent {
    pub dataset_id: String,
    pub quality_score: f64,
    pub issues: Vec<DqIssue>,
    pub timestamp: DateTime<Utc>,
}

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
            let bus_event = job_events::types::DatasetFetchedEvent {
                dataset_id: event.dataset_id,
                provider: event.provider,
                capability: format!("{:?}", event.capability),
                market: format!("{:?}", event.market),
                timestamp: event.timestamp,
                row_count: event.row_count,
            };
            bus.publish(Event::DatasetFetched(bus_event));
        }
        Ok(())
    }

    async fn emit_dataset_gate_completed(
        &self,
        event: DatasetGateCompletedEvent,
    ) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            decision = ?event.decision,
            quality_score = event.quality_score,
            issue_count = event.issue_count,
            "dataset.gate.completed"
        );

        if let Some(bus) = &self.bus {
            let bus_event = job_events::types::DatasetGateCompletedEvent {
                dataset_id: event.dataset_id,
                decision: format!("{:?}", event.decision),
                quality_score: event.quality_score,
                issue_count: event.issue_count,
                timestamp: event.timestamp,
            };
            bus.publish(Event::DatasetGateCompleted(bus_event));
        }
        Ok(())
    }

    async fn emit_dataset_ingested(&self, event: DatasetIngestedEvent) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            decision = ?event.decision,
            storage_path = %event.storage_path,
            catalog_id = %event.catalog_id,
            "dataset.ingested"
        );

        if let Some(bus) = &self.bus {
            let bus_event = job_events::types::DatasetIngestedEvent {
                dataset_id: event.dataset_id,
                decision: format!("{:?}", event.decision),
                storage_path: event.storage_path,
                catalog_id: event.catalog_id,
                timestamp: event.timestamp,
            };
            bus.publish(Event::DatasetIngested(bus_event));
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
            let bus_event = job_events::types::DqRejectionEvent {
                quarantine_id: event.quarantine_id,
                dataset_id: event.dataset_id,
                reasons: event.reasons,
                timestamp: event.timestamp,
            };
            bus.publish(Event::DqRejection(bus_event));
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
            let bus_event = job_events::types::DqDegradedEvent {
                dataset_id: event.dataset_id,
                quality_score: event.quality_score,
                issues: event.issues.iter().map(|i| job_events::types::DqIssue {
                    severity: format!("{:?}", i.severity),
                    field: i.field.clone(),
                    message: i.message.clone(),
                }).collect(),
                timestamp: event.timestamp,
            };
            bus.publish(Event::DqDegraded(bus_event));
        }
        Ok(())
    }
}
