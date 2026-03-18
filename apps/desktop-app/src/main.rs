use anyhow::Result;
use tracing::{info, error, warn};

fn main() -> Result<()> {
    // Initialize structured logging with JSON support
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting zquant desktop application"
    );

    // Load database URL from environment (optional for M1)
    let database_url = match dotenvy::dotenv() {
        Ok(_) => {
            info!("Loaded .env file");
            std::env::var("DATABASE_URL").ok()
        }
        Err(_) => {
            warn!("No .env file found, running in UI-only mode");
            std::env::var("DATABASE_URL").ok()
        }
    };

    if database_url.is_some() {
        info!("Database URL configured, will initialize application core");
    } else {
        info!("No DATABASE_URL set, running in UI-only mode");
    }

    // --- Startup self-check orchestration ---
    // Run health checks early so we can log structured results before UI starts.
    // The actual checks + strategy + notification wiring happens inside app_shell,
    // but we perform a pre-flight path resolution check here at the binary level
    // to catch catastrophic env issues before any heavy initialization.
    match app_shell::health::resolve_runtime_paths() {
        Ok(paths) => {
            info!(
                config = %paths.config_dir.display(),
                logs = %paths.log_dir.display(),
                parquet = %paths.parquet_dir.display(),
                tmp = %paths.tmp_dir.display(),
                "Runtime paths resolved"
            );
        }
        Err(e) => {
            // Log but do NOT exit — app_shell will handle degraded startup.
            error!(error = %e, "Runtime path resolution failed at pre-flight check");
        }
    }

    // Run the app shell (health checks, recovery, notifications all handled inside)
    match app_shell::run_with_config(database_url) {
        Ok(()) => {
            info!("Desktop application shutdown complete");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Desktop application failed");
            Err(e)
        }
    }
}
