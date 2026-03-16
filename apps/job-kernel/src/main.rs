use anyhow::Result;
use job_application::{AgentSupervisor, ApiState, HandlerRegistry, Runner};
use job_domain::{JobContext, JobHandler, JobResult};
use job_events::bus::{EventBus, InMemoryEventBus};
use job_store_pg::JobStore;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::signal;

struct TestHandler;

#[async_trait::async_trait]
impl JobHandler for TestHandler {
    fn job_types(&self) -> &'static [&'static str] {
        &["test"]
    }

    async fn handle(&self, ctx: JobContext) -> Result<JobResult> {
        tracing::info!("Executing test job: {}", ctx.job_id);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        Ok(JobResult {
            artifacts: Some(serde_json::json!({"result": "ok"})),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/zquant".to_string());
    let api_host = std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let api_port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("{}:{}", api_host, api_port);

    let pool = PgPool::connect(&database_url).await?;
    let store = Arc::new(JobStore::new(pool));
    let bus = Arc::new(InMemoryEventBus::new(1000)) as Arc<dyn EventBus>;

    let mut registry = HandlerRegistry::new();
    registry.register(Arc::new(TestHandler))?;
    let registry = Arc::new(registry);

    let api_state = ApiState {
        store: store.clone(),
        bus: bus.clone(),
    };

    let app = job_application::router(api_state);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("Starting API server on {}", bind_addr);
    let server = axum::serve(listener, app);

    let runner = Runner::new(
        store.clone(),
        bus.clone(),
        registry.clone(),
        "kernel-1".to_string(),
    );

    let supervisor = AgentSupervisor::new(bus.clone());
    let supervisor_task = tokio::spawn(supervisor.run());

    let claim_runner = runner.clone();
    let claim_task = tokio::spawn(async move {
        claim_runner.run_claim_loop().await;
    });

    let sweep_runner = runner.clone();
    let sweep_task = tokio::spawn(async move {
        sweep_runner.run_sweep_loop().await;
    });

    tokio::select! {
        _ = server => {},
        _ = supervisor_task => {},
        _ = claim_task => {},
        _ = sweep_task => {},
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down gracefully");
        }
    }

    Ok(())
}
