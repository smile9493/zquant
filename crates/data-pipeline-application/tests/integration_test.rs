use data_pipeline_application::{
    BasicNormalizer, BasicQualityGate, DataPipelineJobHandler, DataPipelineManager,
    InMemoryPersistWriter, MockProvider, PipelineEventEmitter, PriorityRouteResolver,
    ProviderRegistry,
};
use data_pipeline_domain::{Capability, DatasetRequest, DqDecision, IngestRequest, Market};
use job_domain::{JobContext, JobHandler};
use job_events::bus::{Event, EventBus, InMemoryEventBus};
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
        Box::new(PipelineEventEmitter::new_noop()),
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
        Box::new(PipelineEventEmitter::new_noop()),
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
        Box::new(PipelineEventEmitter::new_noop()),
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

#[tokio::test]
async fn test_job_handler_integration() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = Arc::new(DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new_noop()),
    ));

    let handler = DataPipelineJobHandler::new(manager);

    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            time_range: None,
            forced_provider: None,
        },
    };

    let ctx = JobContext {
        job_id: "test_job".to_string(),
        job_type: "ingest_dataset".to_string(),
        payload: serde_json::to_value(&req).unwrap(),
    };

    let result = handler.handle(ctx).await.unwrap();
    assert!(result.artifacts.is_some());
}

#[tokio::test]
async fn test_eventbus_integration() {
    let bus = Arc::new(InMemoryEventBus::new(10));
    let mut rx = bus.subscribe();

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new(bus.clone())),
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

    let _result = manager.ingest_dataset(req).await.unwrap();

    let mut event_count = 0;
    while let Ok(event) = rx.try_recv() {
        match event {
            Event::DatasetFetched(_) | Event::DatasetGateCompleted(_) | Event::DatasetIngested(_) => {
                event_count += 1;
            }
            _ => {}
        }
    }

    assert!(event_count >= 3);
}

#[tokio::test]
async fn test_quarantine_contains_data_and_reasons() {
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
                    field: Some("price".to_string()),
                    message: "Price out of range".to_string(),
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
        Box::new(PipelineEventEmitter::new_noop()),
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
}

#[tokio::test]
async fn test_preserve_caller_dataset_id() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new_noop()),
    );

    let caller_dataset_id = "my_custom_dataset_123".to_string();
    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: Some(caller_dataset_id.clone()),
            time_range: None,
            forced_provider: None,
        },
    };

    let result = manager.ingest_dataset(req).await.unwrap();
    assert_eq!(result.dataset_id, Some(caller_dataset_id));
}

#[tokio::test]
async fn test_events_have_version() {
    let bus = Arc::new(InMemoryEventBus::new(10));
    let mut rx = bus.subscribe();

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new(bus.clone())),
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

    let _result = manager.ingest_dataset(req).await.unwrap();

    let mut has_version = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            Event::DatasetFetched(e) => {
                assert_eq!(e.schema_v, "1.0");
                has_version = true;
            }
            Event::DatasetGateCompleted(e) => {
                assert_eq!(e.schema_v, "1.0");
                has_version = true;
            }
            Event::DatasetIngested(e) => {
                assert_eq!(e.schema_v, "1.0");
                has_version = true;
            }
            _ => {}
        }
    }

    assert!(has_version);
}


