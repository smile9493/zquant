use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{Capability, Market};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRequest {
    pub capability: Capability,
    pub market: Market,
    pub dataset_id: Option<String>,
    #[serde(default)]
    pub symbol_scope: Vec<String>,
    pub time_range: Option<TimeRange>,
    pub forced_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub dataset_request: DatasetRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub capability: Capability,
    pub market: Market,
    pub params: serde_json::Value,
}
