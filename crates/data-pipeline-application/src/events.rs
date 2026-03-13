use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_pipeline_domain::{Capability, DqDecision, DqIssue, Market};
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

pub struct PipelineEventEmitter;

impl Default for PipelineEventEmitter {
    fn default() -> Self {
        Self
    }
}

impl PipelineEventEmitter {
    pub fn new() -> Self {
        Self
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
        Ok(())
    }

    async fn emit_dq_rejection(&self, event: DqRejectionEvent) -> anyhow::Result<()> {
        info!(
            quarantine_id = %event.quarantine_id,
            dataset_id = %event.dataset_id,
            reasons = ?event.reasons,
            "dq.rejection"
        );
        Ok(())
    }

    async fn emit_dq_degraded(&self, event: DqDegradedEvent) -> anyhow::Result<()> {
        info!(
            dataset_id = %event.dataset_id,
            quality_score = event.quality_score,
            issue_count = event.issues.len(),
            "dq.degraded"
        );
        Ok(())
    }
}
