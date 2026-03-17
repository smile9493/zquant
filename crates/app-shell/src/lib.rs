use anyhow::Result;
use eframe::egui;
use tracing::{info, error, warn};
use application_core::ApplicationCore;
use std::sync::Arc;

mod app;

pub use app::ZQuantApp;

/// Run the desktop application shell
pub fn run() -> Result<()> {
    run_with_config(None)
}

/// Run with optional database configuration
pub fn run_with_config(database_url: Option<String>) -> Result<()> {
    info!("Initializing egui native options");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("zquant - 量化研究工作台"),
        ..Default::default()
    };

    // Initialize application core if database URL provided
    let facade = if let Some(url) = database_url {
        info!("Initializing application core with database");
        
        // Create a temporary runtime for initialization
        let rt = tokio::runtime::Runtime::new()?;
        match rt.block_on(ApplicationCore::new(&url)) {
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

    info!("Starting eframe application");

    eframe::run_native(
        "zquant",
        native_options,
        Box::new(move |cc| {
            info!("Creating ZQuantApp instance");
            Box::new(ZQuantApp::new(cc, facade.clone()))
        }),
    )
    .map_err(|e| {
        error!("Application error: {}", e);
        anyhow::anyhow!("Failed to run application: {}", e)
    })
}
