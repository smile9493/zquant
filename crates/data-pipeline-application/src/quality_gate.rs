use async_trait::async_trait;
use data_pipeline_domain::{DataQualityResult, DqDecision, DqIssue, IssueSeverity, NormalizedData};

#[async_trait]
pub trait QualityGate: Send + Sync {
    async fn check(&self, data: &NormalizedData) -> anyhow::Result<DataQualityResult>;
}

pub struct BasicQualityGate;

impl Default for BasicQualityGate {
    fn default() -> Self {
        Self
    }
}

impl BasicQualityGate {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl QualityGate for BasicQualityGate {
    #[tracing::instrument(skip(self, data), fields(record_count = data.records.len(), quality_score, decision))]
    async fn check(&self, data: &NormalizedData) -> anyhow::Result<DataQualityResult> {
        let mut issues = Vec::new();
        let mut quality_score: f64 = 1.0;

        if data.records.is_empty() {
            return Ok(DataQualityResult {
                decision: DqDecision::Reject,
                quality_score: 0.0,
                issues: vec![DqIssue {
                    severity: IssueSeverity::Error,
                    field: None,
                    message: "No records found".to_string(),
                }],
                cleaned_data: data.clone(),
            });
        }

        for (idx, record) in data.records.iter().enumerate() {
            if let Some(close) = record.get("close").and_then(|v| v.as_f64()) {
                if close < 0.0 {
                    issues.push(DqIssue {
                        severity: IssueSeverity::Warning,
                        field: Some(format!("records[{}].close", idx)),
                        message: "Negative price detected".to_string(),
                    });
                    quality_score -= 0.1;
                }
            }
        }

        let decision = if quality_score < 0.5 {
            DqDecision::Reject
        } else if !issues.is_empty() {
            DqDecision::Degraded
        } else {
            DqDecision::Accept
        };

        tracing::Span::current().record("quality_score", quality_score.max(0.0));
        tracing::Span::current().record("decision", format!("{:?}", decision).as_str());

        Ok(DataQualityResult {
            decision,
            quality_score: quality_score.max(0.0),
            issues,
            cleaned_data: data.clone(),
        })
    }
}
