use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
    Stopped,
    Reaped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub job_type: String,
    pub status: JobStatus,
    pub payload: serde_json::Value,
    pub progress: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub artifacts: Option<serde_json::Value>,
    pub executor_id: Option<String>,
    pub stop_requested: bool,
    pub stop_reason: Option<String>,
    pub lease_until_ms: Option<i64>,
    pub lease_version: i64,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: String,
    pub job_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub artifacts: Option<serde_json::Value>,
}

#[async_trait]
pub trait JobHandler: Send + Sync {
    fn job_types(&self) -> &'static [&'static str];

    async fn handle(&self, ctx: JobContext) -> anyhow::Result<JobResult>;
}
