use eframe::egui;
use tracing::{info, debug, warn, error};
use ui_workbench::{Workbench, WorkbenchCommand, RenderSnapshot};
use application_core::{ApplicationFacade, PullRequest};
use crate::health::{HealthReport, CheckSeverity};
use crate::notification::StartupNotification;
use std::sync::Arc;

/// Main application state
pub struct ZQuantApp {
    workbench: Workbench,
    facade: Option<Arc<ApplicationFacade>>,
    /// None when runtime creation failed; app degrades to UI-only mode.
    runtime: Option<tokio::runtime::Runtime>,
    health_report: HealthReport,
    /// User-facing startup notifications (non-Ok checks).
    startup_notifications: Vec<StartupNotification>,
    /// Whether the notification panel is visible.
    show_notifications: bool,
    /// Channel for receiving render snapshots from async chart loads.
    snapshot_rx: std::sync::mpsc::Receiver<RenderSnapshot>,
    snapshot_tx: std::sync::mpsc::Sender<RenderSnapshot>,
    /// Channel for receiving pull results from async pull operations.
    pull_result_rx: std::sync::mpsc::Receiver<(bool, String)>,
    pull_result_tx: std::sync::mpsc::Sender<(bool, String)>,
}

impl ZQuantApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        facade: Option<Arc<ApplicationFacade>>,
        health_report: HealthReport,
        startup_notifications: Vec<StartupNotification>,
    ) -> Self {
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

        let show_notifications = !startup_notifications.is_empty();

        let (snapshot_tx, snapshot_rx) = std::sync::mpsc::channel();
        let (pull_result_tx, pull_result_rx) = std::sync::mpsc::channel();

        Self {
            workbench,
            facade,
            runtime,
            health_report,
            startup_notifications,
            show_notifications,
            snapshot_rx,
            snapshot_tx,
            pull_result_rx,
            pull_result_tx,
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
                let tx = self.snapshot_tx.clone();
                runtime.spawn(async move {
                    match facade.load_chart(&symbol, &timeframe).await {
                        Ok(data) => {
                            info!("Chart loaded: {} points", data.data_points.len());
                            let snap = RenderSnapshot {
                                symbol: data.symbol,
                                timeframe: data.timeframe,
                                candles: vec![], // placeholder — real candle conversion in later milestones
                                provider: data.provider,
                                dataset_id: data.dataset_id,
                                market: data.market,
                                capability: data.capability,
                            };
                            if let Err(e) = tx.send(snap) {
                                warn!("Failed to send render snapshot to UI: {}", e);
                            }
                        }
                        Err(e) => warn!("Failed to load chart: {}", e),
                    }
                });
            }
            WorkbenchCommand::RefreshData => {
                runtime.spawn(async move {
                    match facade.refresh_data().await {
                        Ok(task_id) => info!(task_id, "Refresh-data task submitted"),
                        Err(e) => warn!("Failed to submit refresh-data task: {}", e),
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
            WorkbenchCommand::CancelTask(id) => {
                runtime.spawn(async move {
                    let cancelled = facade.cancel_task(id).await;
                    if cancelled {
                        info!(task_id = id, "Task cancelled via UI");
                    } else {
                        debug!(task_id = id, "Task cancel request ignored (already terminal)");
                    }
                });
            }
            WorkbenchCommand::PullDataset { provider, dataset_id, symbol, start_date, end_date } => {
                let tx = self.pull_result_tx.clone();
                runtime.spawn(async move {
                    let req = PullRequest {
                        provider: provider.clone(),
                        dataset_id: dataset_id.clone(),
                        symbol: symbol.clone(),
                        start_date,
                        end_date,
                    };
                    match facade.pull_dataset(req).await {
                        Ok(result) => {
                            let success = result.status == application_core::PullStatus::Success;
                            info!(
                                provider = %provider,
                                symbol = %symbol,
                                records = result.record_count,
                                "Pull completed"
                            );
                            let _ = tx.send((success, result.message));
                        }
                        Err(e) => {
                            warn!("Pull failed: {}", e);
                            let _ = tx.send((false, format!("拉取失败: {e}")));
                        }
                    }
                });
            }
        }
    }
}

impl eframe::App for ZQuantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Sync task list from runtime to workbench each frame (in-memory, fast)
        if let (Some(rt), Some(facade)) = (&self.runtime, &self.facade) {
            let facade = facade.clone();
            let tasks = rt.block_on(facade.list_tasks());
            self.workbench.update_tasks(tasks);
        }

        // Drain any pending render snapshots from async chart loads
        while let Ok(snap) = self.snapshot_rx.try_recv() {
            self.workbench.update_render_snapshot(snap);
        }

        // Drain any pending pull results
        while let Ok((success, message)) = self.pull_result_rx.try_recv() {
            self.workbench.notify_pull_result(success, message);
        }

        if let Some(cmd) = self.workbench.poll_command() {
            self.handle_command(cmd);
        }
        self.workbench.show(ctx);

        // Notification panel — shows actionable fix suggestions from startup checks
        if self.show_notifications && !self.startup_notifications.is_empty() {
            egui::TopBottomPanel::top("startup_notifications")
                .max_height(150.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("启动检查提示");
                        if ui.small_button("关闭").clicked() {
                            self.show_notifications = false;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for n in &self.startup_notifications {
                            let color = match n.severity {
                                CheckSeverity::Error => egui::Color32::from_rgb(220, 50, 50),
                                CheckSeverity::Warn => egui::Color32::from_rgb(220, 180, 0),
                                CheckSeverity::Ok => egui::Color32::from_rgb(80, 180, 80),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(color, &n.title);
                                ui.label(&n.suggestion);
                            });
                        }
                    });
                });
        }

        // Status bar at the very bottom — health summary
        egui::TopBottomPanel::bottom("status_bar")
            .max_height(20.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let err_count = self.health_report.count_by_severity(CheckSeverity::Error);
                    let warn_count = self.health_report.count_by_severity(CheckSeverity::Warn);
                    let ok_count = self.health_report.count_by_severity(CheckSeverity::Ok);

                    if err_count > 0 {
                        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), format!("❌ {err_count} 错误"));
                        ui.separator();
                    }
                    if warn_count > 0 {
                        ui.colored_label(egui::Color32::from_rgb(220, 180, 0), format!("⚠ {warn_count} 警告"));
                        ui.separator();
                    }
                    ui.label(format!("✅ {ok_count} 正常"));

                    ui.separator();
                    let db_status = if self.facade.is_some() { "DB: 已连接" } else { "DB: 离线" };
                    ui.label(db_status);

                    ui.separator();
                    let task_count = if let (Some(rt), Some(facade)) = (&self.runtime, &self.facade) {
                        let facade = facade.clone();
                        let tasks = rt.block_on(facade.list_tasks());
                        tasks.iter().filter(|t| !t.status.is_terminal()).count()
                    } else {
                        0
                    };
                    ui.label(format!("运行中任务: {task_count}"));
                });
            });
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
