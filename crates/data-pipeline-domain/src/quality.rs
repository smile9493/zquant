use serde::{Deserialize, Serialize};

use crate::types::NormalizedData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DqDecision {
    Accept,
    Degraded,
    Reject,
}

impl std::fmt::Display for DqDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DqDecision::Accept => write!(f, "accept"),
            DqDecision::Degraded => write!(f, "degraded"),
            DqDecision::Reject => write!(f, "reject"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Error => write!(f, "error"),
            IssueSeverity::Warning => write!(f, "warning"),
            IssueSeverity::Info => write!(f, "info"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dq_decision_display_wire_values() {
        assert_eq!(DqDecision::Accept.to_string(), "accept");
        assert_eq!(DqDecision::Degraded.to_string(), "degraded");
        assert_eq!(DqDecision::Reject.to_string(), "reject");
    }

    #[test]
    fn issue_severity_display_wire_values() {
        assert_eq!(IssueSeverity::Error.to_string(), "error");
        assert_eq!(IssueSeverity::Warning.to_string(), "warning");
        assert_eq!(IssueSeverity::Info.to_string(), "info");
    }
}
