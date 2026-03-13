use data_pipeline_application::{
    BasicNormalizer, BasicQualityGate, DataPipelineJobHandler, DataPipelineManager,
    InMemoryPersistWriter, MockProvider, PipelineEventEmitter, PriorityRouteResolver,
    ProviderRegistry,
};
use data_pipeline_domain::{Capability, DatasetRequest, IngestRequest, Market};
use job_domain::{JobContext, JobHandler};
use job_events::bus::{Event, EventBus, InMemoryEventBus};
use std::sync::Arc;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Data Pipeline Demo ===\n");

    let bus = Arc::new(InMemoryEventBus::new(100));
    let mut rx = bus.subscribe();

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new()));

    let manager = Arc::new(DataPipelineManager::new(
        registry,
        Box::new(PriorityRouteResolver::new()),
        Box::new(BasicNormalizer::new()),
        Box::new(BasicQualityGate::new()),
        Box::new(InMemoryPersistWriter::new()),
        Box::new(PipelineEventEmitter::new(bus.clone())),
    ));

    let handler = DataPipelineJobHandler::new(manager.clone());

    println!("Job types supported: {:?}\n", handler.job_types());

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
        job_id: "demo_job_001".to_string(),
        job_type: "ingest_dataset".to_string(),
        payload: serde_json::to_value(&req)?,
    };

    println!("Executing job: {}", ctx.job_id);
    let result = handler.handle(ctx).await?;
    println!("Job completed successfully\n");

    println!("Events received:");
    while let Ok(event) = rx.try_recv() {
        match event {
            Event::DatasetFetched(e) => {
                println!("  - DatasetFetched: {} from {}", e.dataset_id, e.provider);
            }
            Event::DatasetGateCompleted(e) => {
                println!("  - DatasetGateCompleted: {} decision={}", e.dataset_id, e.decision);
            }
            Event::DatasetIngested(e) => {
                println!("  - DatasetIngested: {} at {}", e.dataset_id, e.storage_path);
            }
            _ => {}
        }
    }

    println!("\nArtifacts: {}", serde_json::to_string_pretty(&result.artifacts)?);

    Ok(())
}
