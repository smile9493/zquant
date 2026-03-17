use eframe::egui;
use tracing::{info, debug, warn, error};
use ui_workbench::{Workbench, WorkbenchCommand};
use application_core::ApplicationFacade;
use std::sync::Arc;

/// Main application state
pub struct ZQuantApp {
    workbench: Workbench,
    facade: Option<Arc<ApplicationFacade>>,
    /// None when runtime creation failed; app degrades to UI-only mode.
    runtime: Option<tokio::runtime::Runtime>,
}

impl ZQuantApp {
    pub fn new(cc: &eframe::CreationContext<'_>, facade: Option<Arc<ApplicationFacade>>) -> Self {
        info!("Initializing ZQuantApp");

        // Load Chinese font from Windows system fonts
        Self::configure_fonts(&cc.egui_ctx);

        // Create tokio runtime for async operations.
        // If both multi-thread and current-thread fail, degrade to UI-only.
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => Some(rt),
            Err(e) => {
                warn!("Failed to create multi-thread runtime: {}, trying current-thread", e);
                match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => Some(rt),
                    Err(e2) => {
                        error!(
                            "Cannot create any tokio runtime: {}. \
                             Running in UI-only mode (no async I/O).",
                            e2
                        );
                        None
                    }
                }
            }
        };

        let mut workbench = Workbench::new();

        // Restore workspace state on startup (requires runtime + facade)
        if let (Some(rt), Some(ref f)) = (&runtime, &facade) {
            let f = f.clone();
            match rt.block_on(f.load_workspace()) {
                Ok(Some(snapshot)) => {
                    info!("Workspace snapshot restored on startup");
                    workbench.restore_from_snapshot(&snapshot);
                }
                Ok(None) => {
                    info!("No workspace snapshot found, using defaults");
                }
                Err(e) => {
                    warn!("Failed to restore workspace on startup: {}, using defaults", e);
                }
            }
        }

        Self {
            workbench,
            facade,
            runtime,
        }
    }

    fn configure_fonts(ctx: &egui::Context) {
        let font_path = std::path::Path::new("C:\\Windows\\Fonts\\msyh.ttc");
        if let Ok(font_data) = std::fs::read(font_path) {
            info!("Loaded Chinese font: msyh.ttc");

            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "msyh".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "msyh".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("msyh".to_owned());

            ctx.set_fonts(fonts);
        } else {
            warn!("Chinese font not found at {:?}, CJK characters may not render correctly", font_path);
        }
    }

    fn handle_command(&mut self, cmd: WorkbenchCommand) {
        debug!("Handling workbench command: {:?}", cmd);

        let (Some(runtime), Some(facade)) = (&self.runtime, &self.facade) else {
            warn!("Runtime or facade not available, command ignored: {:?}", cmd);
            return;
        };
        let facade = facade.clone();

        match cmd {
            WorkbenchCommand::LoadChart { symbol, timeframe } => {
                runtime.spawn(async move {
                    match facade.load_chart(&symbol, &timeframe).await {
                        Ok(data) => info!("Chart loaded: {} points", data.data_points.len()),
                        Err(e) => warn!("Failed to load chart: {}", e),
                    }
                });
            }
            WorkbenchCommand::RefreshData => {
                runtime.spawn(async move {
                    match facade.refresh_data().await {
                        Ok(_) => info!("Data refreshed"),
                        Err(e) => warn!("Failed to refresh data: {}", e),
                    }
                });
            }
            WorkbenchCommand::SaveWorkspace(snapshot) => {
                runtime.spawn(async move {
                    match facade.save_workspace(snapshot).await {
                        Ok(_) => info!("Workspace saved"),
                        Err(e) => warn!("Failed to save workspace: {}", e),
                    }
                });
            }
        }
    }
}

impl eframe::App for ZQuantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(cmd) = self.workbench.poll_command() {
            self.handle_command(cmd);
        }
        self.workbench.show(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Application exiting");

        if let (Some(rt), Some(facade)) = (&self.runtime, &self.facade) {
            let snapshot = self.workbench.create_snapshot();
            let facade = facade.clone();

            rt.block_on(async move {
                if let Err(e) = facade.save_workspace(snapshot).await {
                    warn!("Failed to save workspace on exit: {}", e);
                }
            });
        }
    }
}
