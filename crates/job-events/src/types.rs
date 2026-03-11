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
