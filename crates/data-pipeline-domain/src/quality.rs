use serde::{Deserialize, Serialize};

use crate::types::NormalizedData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DqDecision {
    Accept,
    Degraded,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqIssue {
    pub severity: IssueSeverity,
    pub field: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityResult {
    pub decision: DqDecision,
    pub quality_score: f64,
    pub issues: Vec<DqIssue>,
    pub cleaned_data: NormalizedData,
}
