use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    job_observability::init_tracing();
    info!("job-ws-bridge starting");
    Ok(())
}
