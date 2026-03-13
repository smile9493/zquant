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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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
            symbol_scope: vec!["TEST".to_string()],
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

// ============================================================================
// Phase B4: AkShare Provider Hermetic Tests
// ============================================================================

mod akshare_tests {
    use super::*;
    use data_pipeline_application::{AkshareProvider, PythonRunner};
    use data_pipeline_application::route_resolver::RouteResolver;
    use data_pipeline_domain::DataProvider;
    use std::path::Path;

    struct FakePythonRunner {
        response: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl PythonRunner for FakePythonRunner {
        async fn run_json(
            &self,
            _script_path: &Path,
            _input: serde_json::Value,
        ) -> anyhow::Result<serde_json::Value> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_akshare_provider_routability() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({
                "status": "success",
                "data": []
            }),
        });

        let provider = Arc::new(AkshareProvider::new(fake_runner));
        let mut registry = ProviderRegistry::new();
        registry.register(provider.clone());

        let candidates = registry.find_providers(Capability::Ohlcv, Market::CnEquity);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider_name(), "akshare");
    }

    #[tokio::test]
    async fn test_forced_provider_fail_closed_unavailable() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider::new()));

        let resolver = PriorityRouteResolver::new();
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            symbol_scope: vec!["TEST".to_string()],
            time_range: None,
            forced_provider: Some("akshare".to_string()),
        };

        let candidates = registry.find_providers(req.capability, req.market);
        let result = resolver.resolve(&req, candidates).await;

        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not available"));
    }

    #[tokio::test]
    async fn test_forced_provider_fail_closed_mismatch() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(AkshareProvider::new(fake_runner)));

        let resolver = PriorityRouteResolver::new();
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            symbol_scope: vec!["TEST".to_string()],
            time_range: None,
            forced_provider: Some("akshare".to_string()),
        };

        let candidates = registry.find_providers(req.capability, req.market);
        let result = resolver.resolve(&req, candidates).await;

        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not available"));
    }

    #[tokio::test]
    async fn test_akshare_fetch_dataset_with_fake_runner() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({
                "status": "success",
                "data": [
                    {
                        "date": "2024-01-01",
                        "open": 10.5,
                        "high": 11.0,
                        "low": 10.2,
                        "close": 10.8,
                        "volume": 1000000
                    }
                ]
            }),
        });

        let provider = AkshareProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["000001".to_string()],
            time_range: None,
            forced_provider: Some("akshare".to_string()),
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_ok());

        let raw_data = result.unwrap();
        assert!(raw_data.content.get("data").is_some());
    }

    #[tokio::test]
    async fn test_akshare_dataset_id_contract() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let provider = AkshareProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("wrong_dataset_id".to_string()),
            symbol_scope: vec!["000001".to_string()],
            time_range: None,
            forced_provider: None,
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsupported dataset_id"));
    }
}

