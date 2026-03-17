use eframe::egui;
use tracing::{info, debug, warn};
use ui_workbench::{Workbench, WorkbenchCommand};
use application_core::ApplicationFacade;
use std::sync::Arc;

/// Main application state
pub struct ZQuantApp {
    workbench: Workbench,
    facade: Option<Arc<ApplicationFacade>>,
    runtime: tokio::runtime::Runtime,
}

impl ZQuantApp {
    pub fn new(cc: &eframe::CreationContext<'_>, facade: Option<Arc<ApplicationFacade>>) -> Self {
        info!("Initializing ZQuantApp");

        // Load Chinese font from Windows system fonts
        Self::configure_fonts(&cc.egui_ctx);
        
        // Create tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new()
            .expect("Failed to create tokio runtime");
        
        Self {
            workbench: Workbench::new(),
            facade,
            runtime,
        }
    }

    fn configure_fonts(ctx: &egui::Context) {
        // Try loading Microsoft YaHei from Windows system fonts
        let font_path = std::path::Path::new("C:\\Windows\\Fonts\\msyh.ttc");
        if let Ok(font_data) = std::fs::read(font_path) {
            info!("Loaded Chinese font: msyh.ttc");

            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "msyh".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            // Put Chinese font first for proportional text
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "msyh".to_owned());

            // Also for monospace fallback
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
        
        if let Some(facade) = &self.facade {
            let facade = facade.clone();
            
            match cmd {
                WorkbenchCommand::LoadChart { symbol, timeframe } => {
                    self.runtime.spawn(async move {
                        match facade.load_chart(&symbol, &timeframe).await {
                            Ok(data) => info!("Chart loaded: {} points", data.data_points.len()),
                            Err(e) => warn!("Failed to load chart: {}", e),
                        }
                    });
                }
                WorkbenchCommand::RefreshData => {
                    self.runtime.spawn(async move {
                        match facade.refresh_data().await {
                            Ok(_) => info!("Data refreshed"),
                            Err(e) => warn!("Failed to refresh data: {}", e),
                        }
                    });
                }
                WorkbenchCommand::SaveWorkspace(snapshot) => {
                    self.runtime.spawn(async move {
                        match facade.save_workspace(snapshot).await {
                            Ok(_) => info!("Workspace saved"),
                            Err(e) => warn!("Failed to save workspace: {}", e),
                        }
                    });
                }
            }
        } else {
            warn!("Facade not available, command ignored: {:?}", cmd);
        }
    }
}

impl eframe::App for ZQuantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process workbench commands
        if let Some(cmd) = self.workbench.poll_command() {
            self.handle_command(cmd);
        }

        // Render workbench
        self.workbench.show(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Application exiting");
        
        // Save workspace state on exit
        if let Some(facade) = &self.facade {
            let snapshot = self.workbench.create_snapshot();
            let facade = facade.clone();
            
            self.runtime.block_on(async move {
                if let Err(e) = facade.save_workspace(snapshot).await {
                    warn!("Failed to save workspace on exit: {}", e);
                }
            });
        }
    }
}
