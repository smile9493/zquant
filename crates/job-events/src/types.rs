use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordsEnvelope<T> {
    pub event_id: String,
    pub r#type: String,
    pub ts: DateTime<Utc>,
    pub data: T,
    pub producer_id: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobLifecycleEvent {
    pub event_id: String,
    pub event_type: String,
    pub schema_v: i32,
    pub event_ts: DateTime<Utc>,
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub executor_id: Option<String>,
    pub progress: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub duration_ms: Option<i64>,
}

// Phase 1 Event Contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreated {
    pub job_id: String,
    pub job_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStarted {
    pub job_id: String,
    pub executor_id: String,
    pub lease_until_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCompleted {
    pub job_id: String,
    pub status: String,
    pub duration_ms: i64,
    pub error: Option<serde_json::Value>,
    pub artifacts: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnRequested {
    pub agent_id: String,
    pub job_id: String,
    pub agent_kind: String,
    pub init_payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskScheduled {
    pub agent_id: String,
    pub task_id: String,
    pub task_payload: serde_json::Value,
    pub deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageProduced {
    pub agent_id: String,
    pub job_id: String,
    pub message_type: String,
    pub content: serde_json::Value,
    pub ts: DateTime<Utc>,
}

// Data Pipeline Event Contracts
// Note: These mirror data-pipeline-application event types for bus serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetFetchedEvent {
    pub schema_v: String,
    pub dataset_id: String,
    pub provider: String,
    pub capability: String,
    pub market: String,
    pub timestamp: DateTime<Utc>,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetGateCompletedEvent {
    pub schema_v: String,
    pub dataset_id: String,
    pub decision: String,
    pub quality_score: f64,
    pub issue_count: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetIngestedEvent {
    pub schema_v: String,
    pub dataset_id: String,
    pub decision: String,
    pub storage_path: String,
    pub catalog_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqRejectionEvent {
    pub schema_v: String,
    pub quarantine_id: String,
    pub dataset_id: String,
    pub reasons: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqDegradedEvent {
    pub schema_v: String,
    pub dataset_id: String,
    pub quality_score: f64,
    pub issues: Vec<DqIssue>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqIssue {
    pub severity: String,
    pub field: Option<String>,
    pub message: String,
}
