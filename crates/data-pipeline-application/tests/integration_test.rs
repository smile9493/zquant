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

        let catalog_dir = temp_dir.path().join("catalogs");
        let catalog_files: Vec<_> = std::fs::read_dir(&catalog_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(catalog_files.len(), 1, "Should have exactly one catalog file");
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

    let catalog_dir = temp_dir.path().join("catalogs");
    let catalog_files: Vec<_> = std::fs::read_dir(&catalog_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(catalog_files.len(), 1, "Should have exactly one catalog file");
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

// ============================================================================
// File Persistence Robustness Tests
// ============================================================================

#[tokio::test]
async fn test_file_persist_no_collision_same_dataset_id() {
    use data_pipeline_application::{FilePersistWriter, persist::PersistWriter};
    use data_pipeline_domain::NormalizedData;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let writer = FilePersistWriter::new(temp_dir.path());

    let metadata = data_pipeline_application::persist::DatasetMetadata {
        dataset_id: "test_dataset".to_string(),
        provider: "test".to_string(),
        capability: Capability::Ohlcv,
        market: Market::UsEquity,
        available_at: Some(chrono::Utc::now()),
        point_in_time: None,
        version: 1,
    };

    let data = NormalizedData {
        records: vec![serde_json::json!({"test": "data1"})],
        metadata: serde_json::json!({}),
    };

    let receipt1 = writer.write_dataset(&data, &metadata).await.unwrap();
    let receipt2 = writer.write_dataset(&data, &metadata).await.unwrap();

    assert_ne!(receipt1.storage_path, receipt2.storage_path, "Two writes should produce different files");

    assert!(std::path::Path::new(&receipt1.storage_path).exists());
    assert!(std::path::Path::new(&receipt2.storage_path).exists());
}

#[tokio::test]
async fn test_file_persist_path_safety() {
    use data_pipeline_application::{FilePersistWriter, persist::PersistWriter};
    use data_pipeline_domain::NormalizedData;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let writer = FilePersistWriter::new(temp_dir.path());

    let unsafe_ids = vec![
        "..\\..\\evil",
        "../../../etc/passwd",
        "C:\\Windows\\System32\\evil",
        "/etc/evil",
        "test/../../../evil",
    ];

    let data = NormalizedData {
        records: vec![serde_json::json!({"test": "data"})],
        metadata: serde_json::json!({}),
    };

    for unsafe_id in unsafe_ids {
        let metadata = data_pipeline_application::persist::DatasetMetadata {
            dataset_id: unsafe_id.to_string(),
            provider: "test".to_string(),
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            available_at: Some(chrono::Utc::now()),
            point_in_time: None,
            version: 1,
        };

        let receipt = writer.write_dataset(&data, &metadata).await.unwrap();
        let path = std::path::Path::new(&receipt.storage_path);

        assert!(path.starts_with(temp_dir.path()),
            "Path {} should be within base_dir for dataset_id: {}",
            receipt.storage_path, unsafe_id);
    }
}

#[tokio::test]
async fn test_catalog_idempotency() {
    use data_pipeline_application::{FilePersistWriter, persist::PersistWriter};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let writer = FilePersistWriter::new(temp_dir.path());

    let catalog1 = data_pipeline_application::persist::CatalogEntry {
        dataset_id: "test_dataset".to_string(),
        metadata: data_pipeline_application::persist::DatasetMetadata {
            dataset_id: "test_dataset".to_string(),
            provider: "provider1".to_string(),
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            available_at: Some(chrono::Utc::now()),
            point_in_time: None,
            version: 1,
        },
    };

    let catalog2 = data_pipeline_application::persist::CatalogEntry {
        dataset_id: "test_dataset".to_string(),
        metadata: data_pipeline_application::persist::DatasetMetadata {
            dataset_id: "test_dataset".to_string(),
            provider: "provider2".to_string(),
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            available_at: Some(chrono::Utc::now()),
            point_in_time: None,
            version: 2,
        },
    };

    let id1 = writer.write_catalog(&catalog1).await.unwrap();
    let id2 = writer.write_catalog(&catalog2).await.unwrap();

    assert_eq!(id1, id2, "Catalog ID should be the same for same dataset_id");

    let catalog_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("catalogs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(catalog_files.len(), 1, "Should have exactly one catalog file (latest pointer)");
}



// ============================================================================
// Phase D: PyTDX Provider Hermetic Tests
// ============================================================================

mod pytdx_tests {
    use super::*;
    use data_pipeline_application::{PytdxProvider, PythonRunner};
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
    async fn test_pytdx_provider_routability() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let provider = Arc::new(PytdxProvider::new(fake_runner));
        let mut registry = ProviderRegistry::new();
        registry.register(provider.clone());

        let candidates = registry.find_providers(Capability::Ohlcv, Market::CnEquity);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider_name(), "pytdx");
    }

    #[tokio::test]
    async fn test_pytdx_provider_priority_below_akshare() {
        let fake_runner_ak = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });
        let fake_runner_tdx = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let ak = Arc::new(data_pipeline_application::AkshareProvider::new(fake_runner_ak));
        let tdx = Arc::new(PytdxProvider::new(fake_runner_tdx));

        let mut registry = ProviderRegistry::new();
        registry.register(ak.clone());
        registry.register(tdx.clone());

        let candidates = registry.find_providers(Capability::Ohlcv, Market::CnEquity);
        assert_eq!(candidates.len(), 2);
        // AkShare priority=50 > PyTDX priority=40, so AkShare should be first
        assert_eq!(candidates[0].provider_name(), "akshare");
        assert_eq!(candidates[1].provider_name(), "pytdx");
    }

    #[tokio::test]
    async fn test_pytdx_fetch_dataset_success() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({
                "status": "success",
                "data": [
                    {
                        "date": "2024-01-02",
                        "open": 9.50,
                        "high": 9.80,
                        "low": 9.40,
                        "close": 9.70,
                        "volume": 500000.0
                    }
                ]
            }),
        });

        let provider = PytdxProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["600000".to_string()],
            time_range: None,
            forced_provider: Some("pytdx".to_string()),
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_ok(), "fetch_dataset should succeed: {:?}", result.err());

        let raw = result.unwrap();
        let data = raw.content.get("data").expect("missing data field");
        assert!(data.is_array());
        assert_eq!(data.as_array().unwrap().len(), 1);

        let rec = &data.as_array().unwrap()[0];
        assert!(rec.get("date").is_some());
        assert!(rec.get("open").is_some());
        assert!(rec.get("high").is_some());
        assert!(rec.get("low").is_some());
        assert!(rec.get("close").is_some());
        assert!(rec.get("volume").is_some());
    }

    #[tokio::test]
    async fn test_pytdx_wrong_dataset_id_rejected() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let provider = PytdxProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("wrong_dataset_id".to_string()),
            symbol_scope: vec!["600000".to_string()],
            time_range: None,
            forced_provider: None,
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsupported dataset_id"));
    }

    #[tokio::test]
    async fn test_pytdx_missing_symbol_error() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let provider = PytdxProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec![],
            time_range: None,
            forced_provider: None,
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing required symbol_scope"));
    }

    #[tokio::test]
    async fn test_pytdx_forced_provider_fail_closed() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider::new()));

        let resolver = PriorityRouteResolver::new();
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            dataset_id: None,
            symbol_scope: vec!["TEST".to_string()],
            time_range: None,
            forced_provider: Some("pytdx".to_string()),
        };

        let candidates = registry.find_providers(req.capability, req.market);
        let result = resolver.resolve(&req, candidates).await;

        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("not available"));
    }

    #[tokio::test]
    async fn test_pytdx_error_message_propagation() {
        let fake_runner = Arc::new(FakeErrorPythonRunner {
            error_message: "no data returned for 430047 (BJ)".to_string(),
        });

        let provider = PytdxProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["430047".to_string()],
            time_range: None,
            forced_provider: None,
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("430047"), "error should contain symbol");
        assert!(err_msg.contains("BJ"), "error should contain exchange label");
    }

    #[tokio::test]
    async fn test_pytdx_multiple_symbols_rejected() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let provider = PytdxProvider::new(fake_runner);
        let req = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["600000".to_string(), "000001".to_string()],
            time_range: None,
            forced_provider: None,
        };

        let result = provider.fetch_dataset(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exactly 1 symbol"));
    }

    #[tokio::test]
    async fn test_pytdx_generic_fetch_not_supported() {
        let fake_runner = Arc::new(FakePythonRunner {
            response: serde_json::json!({"status": "success", "data": []}),
        });

        let provider = PytdxProvider::new(fake_runner);
        let req = data_pipeline_domain::FetchRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            params: serde_json::json!({"symbol": "600000"}),
        };

        let result = provider.fetch(req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not support generic fetch"));
    }
}

// ============================================================================
// Phase E: Shared Provider Abstraction Tests
// ============================================================================

mod abstraction_tests {
    use super::*;
    use data_pipeline_application::{AkshareProvider, PytdxProvider, PythonRunner};
    use data_pipeline_domain::DataProvider;
    use std::path::Path;
    use std::sync::Mutex;

    /// A spy runner that captures the JSON input sent to the Python script.
    struct SpyPythonRunner {
        captured_input: Mutex<Option<serde_json::Value>>,
    }

    impl SpyPythonRunner {
        fn new() -> Self {
            Self {
                captured_input: Mutex::new(None),
            }
        }

        fn captured(&self) -> serde_json::Value {
            self.captured_input.lock().unwrap().clone().unwrap()
        }
    }

    #[async_trait::async_trait]
    impl PythonRunner for SpyPythonRunner {
        async fn run_json(
            &self,
            _script_path: &Path,
            input: serde_json::Value,
        ) -> anyhow::Result<serde_json::Value> {
            *self.captured_input.lock().unwrap() = Some(input);
            Ok(serde_json::json!({"status": "success", "data": []}))
        }
    }

    /// Test 1: Shared template flow — both providers produce identical base
    /// fields (symbol, start_date, end_date) through the same code path.
    #[tokio::test]
    async fn test_shared_template_produces_consistent_base_fields() {
        let spy_ak = Arc::new(SpyPythonRunner::new());
        let spy_ptdx = Arc::new(SpyPythonRunner::new());

        let ak = AkshareProvider::new(spy_ak.clone());
        let ptdx = PytdxProvider::new(spy_ptdx.clone());

        let make_req = || DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["600000".to_string()],
            time_range: Some(data_pipeline_domain::TimeRange {
                start: chrono::Utc::now(),
                end: chrono::Utc::now(),
            }),
            forced_provider: None,
        };

        ak.fetch_dataset(make_req()).await.unwrap();
        ptdx.fetch_dataset(make_req()).await.unwrap();

        let ak_input = spy_ak.captured();
        let ptdx_input = spy_ptdx.captured();

        // Both must have the same base fields
        assert_eq!(ak_input["symbol"], ptdx_input["symbol"]);
        assert_eq!(ak_input["start_date"], ptdx_input["start_date"]);
        assert_eq!(ak_input["end_date"], ptdx_input["end_date"]);
    }

    /// Test 2: Differentiation branch — AkshareProvider injects "adjust" extra
    /// field while PytdxProvider does not.
    #[tokio::test]
    async fn test_extra_input_differentiation() {
        let spy_ak = Arc::new(SpyPythonRunner::new());
        let spy_ptdx = Arc::new(SpyPythonRunner::new());

        let ak = AkshareProvider::new(spy_ak.clone());
        let ptdx = PytdxProvider::new(spy_ptdx.clone());

        let make_req = || DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["000001".to_string()],
            time_range: None,
            forced_provider: None,
        };

        ak.fetch_dataset(make_req()).await.unwrap();
        ptdx.fetch_dataset(make_req()).await.unwrap();

        let ak_input = spy_ak.captured();
        let ptdx_input = spy_ptdx.captured();

        // AkshareProvider must inject "adjust" field
        assert!(
            ak_input.get("adjust").is_some(),
            "akshare input should contain 'adjust' field"
        );
        assert_eq!(ak_input["adjust"], "");

        // PytdxProvider must NOT have "adjust" field
        assert!(
            ptdx_input.get("adjust").is_none(),
            "pytdx input should not contain 'adjust' field"
        );
    }

    /// Test 3: Shared validation — both providers reject multi-symbol requests
    /// through the same template code path (not duplicated logic).
    #[tokio::test]
    async fn test_shared_validation_rejects_multi_symbol() {
        let spy = Arc::new(SpyPythonRunner::new());

        let ak = AkshareProvider::new(spy.clone());
        let ptdx = PytdxProvider::new(spy.clone());

        let make_req = || DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
            symbol_scope: vec!["600000".to_string(), "000001".to_string()],
            time_range: None,
            forced_provider: None,
        };

        let ak_err = ak.fetch_dataset(make_req()).await.unwrap_err();
        let ptdx_err = ptdx.fetch_dataset(make_req()).await.unwrap_err();

        // Both errors should mention "exactly 1 symbol" — same template message
        assert!(
            ak_err.to_string().contains("exactly 1 symbol"),
            "akshare error: {}",
            ak_err
        );
        assert!(
            ptdx_err.to_string().contains("exactly 1 symbol"),
            "pytdx error: {}",
            ptdx_err
        );

        // But each should mention its own provider name
        assert!(ak_err.to_string().contains("akshare"));
        assert!(ptdx_err.to_string().contains("pytdx"));
    }
}
