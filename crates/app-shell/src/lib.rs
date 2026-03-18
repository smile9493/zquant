use anyhow::Result;
use eframe::egui;
use tracing::{info, error, warn};
use application_core::ApplicationCore;
use std::sync::Arc;

mod app;
pub mod health;
pub mod notification;
pub mod recovery;

pub use app::ZQuantApp;
pub use health::HealthReport;
pub use notification::{StartupNotification, generate_notifications};
pub use recovery::{StartupStrategy, determine_startup_strategy};

/// Run the desktop application shell
pub fn run() -> Result<()> {
    run_with_config(None)
}

/// Run with optional database configuration
pub fn run_with_config(database_url: Option<String>) -> Result<()> {
    info!("Initializing egui native options");

    // Create a temporary runtime for initialization and health checks
    let init_rt = tokio::runtime::Runtime::new()?;

    // Run startup health checks
    let health_report = init_rt.block_on(
        health::run_startup_checks(database_url.as_deref())
    );

    // Determine startup strategy and generate user-facing notifications
    let strategy = recovery::determine_startup_strategy(&health_report);
    let notifications = notification::generate_notifications(&health_report);

    match strategy {
        recovery::StartupStrategy::Continue => {
            info!("Startup strategy: Continue (all checks passed)");
        }
        recovery::StartupStrategy::Degrade => {
            warn!(
                errors = health_report.count_by_severity(health::CheckSeverity::Error),
                warnings = health_report.count_by_severity(health::CheckSeverity::Warn),
                "Startup strategy: Degrade — some features may be unavailable"
            );
            for n in &notifications {
                warn!(title = %n.title, "{}", n.suggestion);
            }
        }
        recovery::StartupStrategy::Block => {
            error!("Startup strategy: Block — critical directory checks failed");
            for n in &notifications {
                error!(title = %n.title, "{}", n.suggestion);
            }
            // Even in Block mode we still launch the UI to show the error,
            // rather than hard-exiting (per acceptance criteria: no panic/process::exit).
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("zquant - 量化研究工作台"),
        ..Default::default()
    };

    // Initialize application core if database URL provided
    let facade = if let Some(url) = database_url {
        info!("Initializing application core with database");
        
        match init_rt.block_on(ApplicationCore::new(&url)) {
            Ok(core) => {
                info!("Application core initialized");
                Some(Arc::new(core.facade()))
            }
            Err(e) => {
                warn!("Failed to initialize application core: {}, running in UI-only mode", e);
                None
            }
        }
    } else {
        info!("Running in UI-only mode (no database connection)");
        None
    };

    // Drop init runtime before starting eframe (eframe creates its own event loop)
    drop(init_rt);

    info!("Starting eframe application");

    eframe::run_native(
        "zquant",
        native_options,
        Box::new(move |cc| {
            info!("Creating ZQuantApp instance");
            Box::new(ZQuantApp::new(cc, facade.clone(), health_report.clone(), notifications.clone()))
        }),
    )
    .map_err(|e| {
        error!("Application error: {}", e);
        anyhow::anyhow!("Failed to run application: {}", e)
    })
}
