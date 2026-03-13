use data_pipeline_application::{
    BasicNormalizer, BasicQualityGate, DataPipelineManager, InMemoryPersistWriter,
    MockProvider, PipelineEventEmitter, PriorityRouteResolver, ProviderRegistry,
};
use data_pipeline_domain::{Capability, DatasetRequest, DqDecision, IngestRequest, Market};
use std::sync::Arc;

#[tokio::test]
async fn test_ingest_accept_scenario() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new()),
    );

    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            time_range: None,
            forced_provider: None,
        },
    };

    let result = manager.ingest_dataset(req).await.unwrap();

    assert_eq!(result.decision, DqDecision::Accept);
    assert!(result.dataset_id.is_some());
    assert!(result.persist_receipt.is_some());
    assert!(result.quarantine_id.is_none());
}

#[tokio::test]
async fn test_ingest_reject_scenario() {
    use data_pipeline_domain::{DataQualityResult, DqIssue, IssueSeverity, NormalizedData};

    struct RejectQualityGate;

    #[async_trait::async_trait]
    impl data_pipeline_application::quality_gate::QualityGate for RejectQualityGate {
        async fn check(&self, data: &NormalizedData) -> anyhow::Result<DataQualityResult> {
            Ok(DataQualityResult {
                decision: DqDecision::Reject,
                quality_score: 0.0,
                issues: vec![DqIssue {
                    severity: IssueSeverity::Error,
                    field: None,
                    message: "Critical error".to_string(),
                }],
                cleaned_data: data.clone(),
            })
        }
    }

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(RejectQualityGate),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new()),
    );

    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            time_range: None,
            forced_provider: None,
        },
    };

    let result = manager.ingest_dataset(req).await.unwrap();

    assert_eq!(result.decision, DqDecision::Reject);
    assert!(result.quarantine_id.is_some());
    assert!(result.persist_receipt.is_none());
}

#[tokio::test]
async fn test_ingest_degraded_scenario() {
    use data_pipeline_domain::{DataQualityResult, DqIssue, IssueSeverity, NormalizedData};

    struct DegradedQualityGate;

    #[async_trait::async_trait]
    impl data_pipeline_application::quality_gate::QualityGate for DegradedQualityGate {
        async fn check(&self, data: &NormalizedData) -> anyhow::Result<DataQualityResult> {
            Ok(DataQualityResult {
                decision: DqDecision::Degraded,
                quality_score: 0.7,
                issues: vec![DqIssue {
                    severity: IssueSeverity::Warning,
                    field: Some("price".to_string()),
                    message: "Minor issue".to_string(),
                }],
                cleaned_data: data.clone(),
            })
        }
    }

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(DegradedQualityGate),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new()),
    );

    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            time_range: None,
            forced_provider: None,
        },
    };

    let result = manager.ingest_dataset(req).await.unwrap();

    assert_eq!(result.decision, DqDecision::Degraded);
    assert!(result.dataset_id.is_some());
    assert!(result.persist_receipt.is_some());
    assert!(result.quarantine_id.is_none());
}
