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

    struct FakeErrorPythonRunner {
        error_message: String,
    }

    #[async_trait::async_trait]
    impl PythonRunner for FakeErrorPythonRunner {
        async fn run_json(
            &self,
            _script_path: &Path,
            _input: serde_json::Value,
        ) -> anyhow::Result<serde_json::Value> {
            Err(anyhow::anyhow!("{}", self.error_message))
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

    #[tokio::test]
    async fn test_akshare_chinese_column_mapping() {
        use data_pipeline_application::python_runner::SubprocessPythonRunner;
        use std::path::PathBuf;

        let script_path = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/python/test_chinese_column_mapping.py"
        ));

        let runner = SubprocessPythonRunner::new();
        let input = serde_json::json!({
            "symbol": "000001",
            "start_date": "20240101",
            "end_date": "20241231",
            "adjust": ""
        });

        let result = runner.run_json(&script_path, input).await;
        assert!(result.is_ok(), "Script execution failed: {:?}", result.err());

        let output = result.unwrap();
        let data_array = output.get("data").unwrap().as_array().unwrap();
        assert!(!data_array.is_empty(), "Data array should not be empty");

        let first_record = &data_array[0];

        // Verify English column names are present
        assert!(first_record.get("date").is_some(), "Missing 'date' field");
        assert!(first_record.get("open").is_some(), "Missing 'open' field");
        assert!(first_record.get("close").is_some(), "Missing 'close' field");
        assert!(first_record.get("high").is_some(), "Missing 'high' field");
        assert!(first_record.get("low").is_some(), "Missing 'low' field");
        assert!(first_record.get("volume").is_some(), "Missing 'volume' field");

        // Verify Chinese column names are NOT present
        assert!(first_record.get("日期").is_none(), "Chinese column '日期' should be mapped");
        assert!(first_record.get("开盘").is_none(), "Chinese column '开盘' should be mapped");
    }

    #[tokio::test]
    async fn test_akshare_error_message_propagation() {
        let fake_runner = Arc::new(FakeErrorPythonRunner {
            error_message: "Failed to fetch data: symbol not found".to_string(),
        });

        let provider = AkshareProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["INVALID".to_string()],
            time_range: None,
            forced_provider: None,
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("symbol not found"));
    }

    #[tokio::test]
    async fn test_akshare_file_persist_success() {
        use data_pipeline_application::FilePersistWriter;
        use tempfile::TempDir;

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

        let temp_dir = TempDir::new().unwrap();
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(AkshareProvider::new(fake_runner)));

        let manager = DataPipelineManager::new(
            registry,
            Box::new(PriorityRouteResolver::new()),
            Box::new(BasicNormalizer::new()),
            Box::new(BasicQualityGate::new()),
            Box::new(FilePersistWriter::new(temp_dir.path())),
            Box::new(PipelineEventEmitter::new_noop()),
        );

        let req = IngestRequest {
            dataset_request: DatasetRequest {
                capability: Capability::Ohlcv,
                market: Market::CnEquity,
                dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
                symbol_scope: vec!["000001".to_string()],
                time_range: None,
                forced_provider: Some("akshare".to_string()),
            },
        };

        let result = manager.ingest_dataset(req).await.unwrap();

        assert_eq!(result.decision, DqDecision::Accept);
        assert!(result.persist_receipt.is_some());

        let receipt = result.persist_receipt.unwrap();
        assert!(std::path::Path::new(&receipt.storage_path).exists());

        let catalog_path = temp_dir.path().join("catalogs").join("cn_equity.ohlcv.daily.json");
        assert!(catalog_path.exists());
    }

    #[tokio::test]
    async fn test_akshare_file_persist_reject() {
        use data_pipeline_application::FilePersistWriter;
        use data_pipeline_domain::{DataQualityResult, DqIssue, IssueSeverity, NormalizedData};
        use tempfile::TempDir;

        struct RejectQualityGate;

        #[async_trait::async_trait]
        impl data_pipeline_application::quality_gate::QualityGate for RejectQualityGate {
            async fn check(&self, data: &NormalizedData) -> anyhow::Result<DataQualityResult> {
                Ok(DataQualityResult {
                    decision: DqDecision::Reject,
                    quality_score: 0.0,
                    issues: vec![DqIssue {
                        severity: IssueSeverity::Error,
                        field: Some("volume".to_string()),
                        message: "Volume out of range".to_string(),
                    }],
                    cleaned_data: data.clone(),
                })
            }
        }

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

        let temp_dir = TempDir::new().unwrap();
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(AkshareProvider::new(fake_runner)));

        let manager = DataPipelineManager::new(
            registry,
            Box::new(PriorityRouteResolver::new()),
            Box::new(BasicNormalizer::new()),
            Box::new(RejectQualityGate),
            Box::new(FilePersistWriter::new(temp_dir.path())),
            Box::new(PipelineEventEmitter::new_noop()),
        );

        let req = IngestRequest {
            dataset_request: DatasetRequest {
                capability: Capability::Ohlcv,
                market: Market::CnEquity,
                dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
                symbol_scope: vec!["000001".to_string()],
                time_range: None,
                forced_provider: Some("akshare".to_string()),
            },
        };

        let result = manager.ingest_dataset(req).await.unwrap();

        assert_eq!(result.decision, DqDecision::Reject);
        assert!(result.quarantine_id.is_some());

        let quarantine_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("quarantine"))
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(quarantine_files.len(), 1);
    }
}

// ============================================================================
// Phase C: Filesystem Persistence Tests
// ============================================================================

#[tokio::test]
async fn test_file_persist_accept_scenario() {
    use data_pipeline_application::FilePersistWriter;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(FilePersistWriter::new(temp_dir.path())),
        Box::new(PipelineEventEmitter::new_noop()),
    );

    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: Some("test_dataset".to_string()),
            symbol_scope: vec!["TEST".to_string()],
            time_range: None,
            forced_provider: None,
        },
    };

    let result = manager.ingest_dataset(req).await.unwrap();

    assert_eq!(result.decision, DqDecision::Accept);
    assert!(result.persist_receipt.is_some());

    let receipt = result.persist_receipt.unwrap();
    assert!(std::path::Path::new(&receipt.storage_path).exists());

    let catalog_path = temp_dir.path().join("catalogs").join("test_dataset.json");
    assert!(catalog_path.exists());
}

#[tokio::test]
async fn test_file_persist_reject_scenario() {
    use data_pipeline_application::FilePersistWriter;
    use data_pipeline_domain::{DataQualityResult, DqIssue, IssueSeverity, NormalizedData};
    use tempfile::TempDir;

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

    let temp_dir = TempDir::new().unwrap();
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(RejectQualityGate),
        Box::new(FilePersistWriter::new(temp_dir.path())),
        Box::new(PipelineEventEmitter::new_noop()),
    );

    let req = IngestRequest {
        dataset_request: DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: Some("test_dataset".to_string()),
            symbol_scope: vec!["TEST".to_string()],
            time_range: None,
            forced_provider: None,
        },
    };

    let result = manager.ingest_dataset(req).await.unwrap();

    assert_eq!(result.decision, DqDecision::Reject);
    assert!(result.quarantine_id.is_some());

    let quarantine_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("quarantine"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(quarantine_files.len(), 1);
}

#[tokio::test]
async fn test_observability_metrics_no_panic() {
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

    let result = manager.ingest_dataset(req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_observability_emit_events_in_degraded_path() {
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

    let bus = Arc::new(InMemoryEventBus::new(10));
    let mut rx = bus.subscribe();

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(DegradedQualityGate),
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

    let result = manager.ingest_dataset(req).await.unwrap();
    assert_eq!(result.decision, DqDecision::Degraded);

    let mut fetched_count = 0;
    let mut gate_count = 0;
    let mut ingested_count = 0;
    let mut degraded_count = 0;

    while let Ok(event) = rx.try_recv() {
        match event {
            Event::DatasetFetched(_) => fetched_count += 1,
            Event::DatasetGateCompleted(_) => gate_count += 1,
            Event::DatasetIngested(_) => ingested_count += 1,
            Event::DqDegraded(_) => degraded_count += 1,
            _ => {}
        }
    }

    assert_eq!(fetched_count, 1, "Should emit dataset_fetched event");
    assert_eq!(gate_count, 1, "Should emit gate_completed event");
    assert_eq!(ingested_count, 1, "Should emit dataset_ingested event");
    assert_eq!(degraded_count, 1, "Should emit dq_degraded event");
}

#[tokio::test]
async fn test_observability_metrics_recorded() {
    use metrics::{Counter, Histogram, Key, KeyName, Metadata, Recorder, SharedString, Unit};
    use std::sync::Mutex;

    struct TestCounter {
        name: String,
        labels: String,
        data: Arc<Mutex<Vec<(String, String, u64)>>>,
    }

    impl metrics::CounterFn for TestCounter {
        fn increment(&self, value: u64) {
            self.data.lock().unwrap().push((self.name.clone(), self.labels.clone(), value));
        }

        fn absolute(&self, _value: u64) {}
    }

    struct TestHistogram {
        name: String,
        labels: String,
        data: Arc<Mutex<Vec<(String, String, f64)>>>,
    }

    impl metrics::HistogramFn for TestHistogram {
        fn record(&self, value: f64) {
            self.data.lock().unwrap().push((self.name.clone(), self.labels.clone(), value));
        }
    }

    struct TestRecorder {
        counters: Arc<Mutex<Vec<(String, String, u64)>>>,
        histograms: Arc<Mutex<Vec<(String, String, f64)>>>,
    }

    impl Recorder for TestRecorder {
        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}

        fn register_counter(&self, key: &Key, _: &Metadata<'_>) -> Counter {
            let name = key.name().to_string();
            let labels = format!("{:?}", key.labels().collect::<Vec<_>>());
            Counter::from_arc(Arc::new(TestCounter {
                name,
                labels,
                data: self.counters.clone(),
            }))
        }

        fn register_gauge(&self, _: &Key, _: &Metadata<'_>) -> metrics::Gauge {
            metrics::Gauge::noop()
        }

        fn register_histogram(&self, key: &Key, _: &Metadata<'_>) -> Histogram {
            let name = key.name().to_string();
            let labels = format!("{:?}", key.labels().collect::<Vec<_>>());
            Histogram::from_arc(Arc::new(TestHistogram {
                name,
                labels,
                data: self.histograms.clone(),
            }))
        }
    }

    let counters = Arc::new(Mutex::new(Vec::new()));
    let histograms = Arc::new(Mutex::new(Vec::new()));

    let recorder = TestRecorder {
        counters: counters.clone(),
        histograms: histograms.clone(),
    };

    metrics::set_global_recorder(recorder).ok();

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

    let _result = manager.ingest_dataset(req).await.unwrap();

    let counters_data = counters.lock().unwrap();
    let histograms_data = histograms.lock().unwrap();

    let has_ingest_counter = counters_data.iter().any(|(name, labels, _)| {
        name == "pipeline_ingest_total" && labels.contains("decision")
    });
    assert!(has_ingest_counter, "Should record pipeline_ingest_total with decision label");

    let has_stage_duration = histograms_data.iter().any(|(name, labels, _)| {
        name == "pipeline_stage_duration_seconds" && labels.contains("stage")
    });
    assert!(has_stage_duration, "Should record pipeline_stage_duration_seconds with stage label");
}


