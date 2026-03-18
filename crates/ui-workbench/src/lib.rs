use egui::{Context, CentralPanel, TopBottomPanel, SidePanel, Color32};
use egui_plot::{Plot, Bar, BarChart, Line, PlotPoints};
use jobs_runtime::{TaskEntry, TaskStatus};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use application_core::WorkspaceState;
use std::collections::VecDeque;

/// Panel visibility state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelState {
    pub left_visible: bool,
    pub right_visible: bool,
    pub bottom_visible: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            left_visible: true,
            right_visible: true,
            bottom_visible: true,
        }
    }
}

/// Single OHLC candle for chart rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Render snapshot: data contract between UI state and chart rendering.
/// Kept inside ui-workbench to avoid leaking plot types into application-core.
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub symbol: String,
    pub timeframe: String,
    pub candles: Vec<Candle>,
    /// Data provider identifier (e.g. "akshare").
    pub provider: String,
    /// Dataset identifier within the provider.
    pub dataset_id: String,
    /// Market classification (e.g. "cn_stock", "us_stock").
    pub market: String,
    /// Data capability tag (e.g. "ohlcv", "tick").
    pub capability: String,
}

/// Commands that workbench can send to application layer
#[derive(Debug, Clone)]
pub enum WorkbenchCommand {
    LoadChart { symbol: String, timeframe: String },
    RefreshData,
    SaveWorkspace(WorkspaceState),
    CancelTask(jobs_runtime::TaskId),
    PullDataset {
        provider: String,
        dataset_id: String,
        symbol: String,
        start_date: Option<String>,
        end_date: Option<String>,
    },
}

/// Provider-dataset mapping entry for the pull dialog.
struct DatasetEntry {
    provider: &'static str,
    dataset_id: &'static str,
    label: &'static str,
}

/// Static registry of available provider-dataset combinations.
fn available_datasets() -> &'static [DatasetEntry] {
    &[
        DatasetEntry { provider: "akshare", dataset_id: "cn_equity.ohlcv.daily", label: "A股日线 (AkShare)" },
        DatasetEntry { provider: "pytdx",   dataset_id: "cn_equity.ohlcv.daily", label: "A股日线 (PyTDX)" },
        DatasetEntry { provider: "mock",    dataset_id: "mock_ohlcv",            label: "模拟数据 (Mock)" },
    ]
}

/// Pull dialog form state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PullDialogPhase {
    /// Dialog is closed.
    Closed,
    /// User is filling the form.
    Editing,
    /// Request submitted, waiting for result.
    Submitting,
    /// Pull completed (success or failure).
    Done { success: bool, message: String },
}

/// State for the data pull dialog.
struct PullDialogState {
    phase: PullDialogPhase,
    selected_idx: usize,
    symbol: String,
    start_date: String,
    end_date: String,
    validation_error: Option<String>,
}

/// Workbench manages the main UI layout and chart rendering
pub struct Workbench {
    panel_state: PanelState,
    command_queue: VecDeque<WorkbenchCommand>,
    current_symbol: String,
    current_timeframe: String,
    render_snapshot: Option<RenderSnapshot>,
    task_entries: Vec<TaskEntry>,
    pull_dialog: PullDialogState,
}

impl Workbench {
    pub fn new() -> Self {
        debug!("Creating new Workbench");
        let demo_candles = Self::generate_demo_candles();
        let snapshot = RenderSnapshot {
            symbol: "AAPL".to_string(),
            timeframe: "1D".to_string(),
            candles: demo_candles,
            provider: "demo".to_string(),
            dataset_id: "demo_ohlc".to_string(),
            market: "us_stock".to_string(),
            capability: "ohlcv".to_string(),
        };
        info!("Workbench initialized with {} demo candles", snapshot.candles.len());
        Self {
            panel_state: PanelState::default(),
            command_queue: VecDeque::new(),
            current_symbol: "AAPL".to_string(),
            current_timeframe: "1D".to_string(),
            render_snapshot: Some(snapshot),
            task_entries: Vec::new(),
            pull_dialog: PullDialogState {
                phase: PullDialogPhase::Closed,
                selected_idx: 0,
                symbol: String::new(),
                start_date: String::new(),
                end_date: String::new(),
                validation_error: None,
            },
        }
    }

    /// Generate demo OHLC data for initial display
    fn generate_demo_candles() -> Vec<Candle> {
        let mut candles = Vec::with_capacity(60);
        let mut price = 150.0_f64;
        for i in 0..60 {
            let change = ((i as f64 * 0.7).sin() * 3.0) + ((i as f64 * 0.3).cos() * 1.5);
            let open = price;
            let close = price + change;
            let high = open.max(close) + (1.0 + (i as f64 * 0.5).sin().abs() * 2.0);
            let low = open.min(close) - (1.0 + (i as f64 * 0.3).cos().abs() * 2.0);
            candles.push(Candle {
                timestamp: i as f64,
                open,
                high,
                low,
                close,
            });
            price = close;
        }
        candles
    }

    /// Poll for pending commands
    pub fn poll_command(&mut self) -> Option<WorkbenchCommand> {
        self.command_queue.pop_front()
    }

    /// Update render snapshot with new candle data
    pub fn update_render_snapshot(&mut self, snapshot: RenderSnapshot) {
        info!(
            symbol = %snapshot.symbol,
            candles = snapshot.candles.len(),
            "Render snapshot updated"
        );
        self.render_snapshot = Some(snapshot);
    }

    /// Update task entries for bottom panel display.
    pub fn update_tasks(&mut self, entries: Vec<TaskEntry>) {
        self.task_entries = entries;
    }

    /// Restore workbench state from a workspace snapshot
    pub fn restore_from_snapshot(&mut self, snapshot: &WorkspaceState) {
        if let Some(ref s) = snapshot.symbol {
            self.current_symbol = s.clone();
        }
        if let Some(ref t) = snapshot.timeframe {
            self.current_timeframe = t.clone();
        }
        self.panel_state.left_visible = snapshot.layout_state.left_visible;
        self.panel_state.right_visible = snapshot.layout_state.right_visible;
        self.panel_state.bottom_visible = snapshot.layout_state.bottom_visible;
        debug!("Workbench state restored from snapshot");
    }

    /// Create workspace snapshot
    pub fn create_snapshot(&self) -> WorkspaceState {
        WorkspaceState {
            symbol: Some(self.current_symbol.clone()),
            timeframe: Some(self.current_timeframe.clone()),
            layout_state: application_core::LayoutState {
                left_visible: self.panel_state.left_visible,
                right_visible: self.panel_state.right_visible,
                bottom_visible: self.panel_state.bottom_visible,
            },
        }
    }

    fn enqueue_command(&mut self, cmd: WorkbenchCommand) {
        debug!("Enqueuing command: {:?}", cmd);
        self.command_queue.push_back(cmd);
    }

    /// Notify the pull dialog of a completed pull result.
    pub fn notify_pull_result(&mut self, success: bool, message: String) {
        self.pull_dialog.phase = PullDialogPhase::Done { success, message };
    }

    pub fn show(&mut self, ctx: &Context) {
        // Top bar
        TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("zquant");
                ui.separator();

                if ui.button(if self.panel_state.left_visible { "◀ 隐藏左侧" } else { "▶ 显示左侧" }).clicked() {
                    self.panel_state.left_visible = !self.panel_state.left_visible;
                }
                if ui.button(if self.panel_state.right_visible { "隐藏右侧 ▶" } else { "◀ 显示右侧" }).clicked() {
                    self.panel_state.right_visible = !self.panel_state.right_visible;
                }
                if ui.button(if self.panel_state.bottom_visible { "▼ 隐藏底部" } else { "▲ 显示底部" }).clicked() {
                    self.panel_state.bottom_visible = !self.panel_state.bottom_visible;
                }

                ui.separator();

                if ui.button("🔄 刷新数据").clicked() {
                    self.enqueue_command(WorkbenchCommand::RefreshData);
                }
                if ui.button("📊 加载图表").clicked() {
                    self.enqueue_command(WorkbenchCommand::LoadChart {
                        symbol: self.current_symbol.clone(),
                        timeframe: self.current_timeframe.clone(),
                    });
                }

                ui.separator();

                if ui.button("📥 拉取数据").clicked() {
                    self.pull_dialog.phase = PullDialogPhase::Editing;
                    self.pull_dialog.validation_error = None;
                }
            });
        });

        // Bottom dock — task panel
        if self.panel_state.bottom_visible {
            TopBottomPanel::bottom("bottom_dock")
                .min_height(80.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("任务面板");
                        ui.separator();
                        ui.label(format!("共 {} 个任务", self.task_entries.len()));
                    });
                    ui.separator();

                    if self.task_entries.is_empty() {
                        ui.label("暂无任务");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                let mut cancel_ids = Vec::new();
                                for entry in &self.task_entries {
                                    ui.horizontal(|ui| {
                                        let status_icon = match entry.status {
                                            TaskStatus::Pending => "⏳",
                                            TaskStatus::Running => "▶",
                                            TaskStatus::Success => "✅",
                                            TaskStatus::Failed => "❌",
                                            TaskStatus::Cancelled => "🚫",
                                        };
                                        ui.label(format!(
                                            "{} [{}] {}",
                                            status_icon, entry.id, entry.name
                                        ));
                                        if let Some(ref msg) = entry.message {
                                            ui.label(format!("— {msg}"));
                                        }
                                        if !entry.status.is_terminal() {
                                            if ui.small_button("取消").clicked() {
                                                cancel_ids.push(entry.id);
                                            }
                                        }
                                    });
                                }
                                for id in cancel_ids {
                                    self.enqueue_command(WorkbenchCommand::CancelTask(id));
                                }
                            });
                    }
                });
        }

        // Left sidebar — data source info
        if self.panel_state.left_visible {
            SidePanel::left("left_sidebar")
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("导航");
                    ui.separator();

                    ui.label("数据源");
                    let provider = self.render_snapshot.as_ref()
                        .map(|s| s.provider.as_str())
                        .unwrap_or("unknown");
                    ui.indent("provider_indent", |ui| {
                        ui.label(format!("Provider: {provider}"));
                    });

                    ui.separator();
                    ui.label("• 策略");
                    ui.label("• 回测");
                });
        }

        // Right dock — properties with dataset metadata
        if self.panel_state.right_visible {
            SidePanel::right("right_dock")
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.heading("属性");
                    ui.separator();
                    ui.label(format!("Symbol: {}", self.current_symbol));
                    ui.label(format!("Timeframe: {}", self.current_timeframe));
                    if let Some(ref snap) = self.render_snapshot {
                        ui.label(format!("K线数量: {}", snap.candles.len()));
                        ui.separator();
                        ui.label("数据集分类");
                        ui.indent("dataset_indent", |ui| {
                            ui.label(format!("dataset_id: {}", snap.dataset_id));
                            ui.label(format!("market: {}", snap.market));
                            ui.label(format!("capability: {}", snap.capability));
                        });
                    }
                    ui.separator();
                    ui.label("Status: Ready");
                });
        }

        // Pull data dialog (modal window)
        if self.pull_dialog.phase != PullDialogPhase::Closed {
            let mut open = true;
            egui::Window::new("拉取数据")
                .collapsible(false)
                .resizable(false)
                .default_width(360.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    self.render_pull_dialog(ui);
                });
            if !open {
                self.pull_dialog.phase = PullDialogPhase::Closed;
            }
        }

        // Center canvas with K-line chart
        CentralPanel::default().show(ctx, |ui| {
            self.render_chart(ui);
        });
    }

    /// Render the pull data dialog contents.
    fn render_pull_dialog(&mut self, ui: &mut egui::Ui) {
        let datasets = available_datasets();

        match &self.pull_dialog.phase {
            PullDialogPhase::Submitting => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("正在拉取...");
                });
                return;
            }
            PullDialogPhase::Done { success, message } => {
                let color = if *success {
                    Color32::from_rgb(0, 180, 80)
                } else {
                    Color32::from_rgb(220, 50, 50)
                };
                ui.colored_label(color, message.as_str());
                ui.add_space(8.0);
                if ui.button("关闭").clicked() {
                    self.pull_dialog.phase = PullDialogPhase::Closed;
                }
                return;
            }
            _ => {}
        }

        // Dataset combo
        ui.horizontal(|ui| {
            ui.label("数据集:");
            egui::ComboBox::from_id_source("pull_dataset_combo")
                .selected_text(datasets.get(self.pull_dialog.selected_idx).map_or("选择...", |d| d.label))
                .show_ui(ui, |ui: &mut egui::Ui| {
                    for (i, ds) in datasets.iter().enumerate() {
                        ui.selectable_value(&mut self.pull_dialog.selected_idx, i, ds.label);
                    }
                });
        });

        // Symbol input
        ui.horizontal(|ui| {
            ui.label("代码:    ");
            ui.text_edit_singleline(&mut self.pull_dialog.symbol);
        });

        // Date range
        ui.horizontal(|ui| {
            ui.label("起始日期:");
            ui.add(egui::TextEdit::singleline(&mut self.pull_dialog.start_date).hint_text("YYYYMMDD"));
        });
        ui.horizontal(|ui| {
            ui.label("结束日期:");
            ui.add(egui::TextEdit::singleline(&mut self.pull_dialog.end_date).hint_text("YYYYMMDD"));
        });

        // Validation error
        if let Some(ref err) = self.pull_dialog.validation_error {
            ui.colored_label(Color32::from_rgb(220, 50, 50), err.as_str());
        }

        ui.add_space(8.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("拉取并加载").clicked() {
                if let Some(err) = self.validate_pull_form(datasets) {
                    self.pull_dialog.validation_error = Some(err);
                } else {
                    self.pull_dialog.validation_error = None;
                    let ds = &datasets[self.pull_dialog.selected_idx];
                    self.pull_dialog.phase = PullDialogPhase::Submitting;
                    self.enqueue_command(WorkbenchCommand::PullDataset {
                        provider: ds.provider.to_string(),
                        dataset_id: ds.dataset_id.to_string(),
                        symbol: self.pull_dialog.symbol.trim().to_string(),
                        start_date: if self.pull_dialog.start_date.trim().is_empty() { None } else { Some(self.pull_dialog.start_date.trim().to_string()) },
                        end_date: if self.pull_dialog.end_date.trim().is_empty() { None } else { Some(self.pull_dialog.end_date.trim().to_string()) },
                    });
                }
            }
            if ui.button("取消").clicked() {
                self.pull_dialog.phase = PullDialogPhase::Closed;
            }
        });
    }

    /// Validate pull form fields. Returns Some(error) if invalid.
    fn validate_pull_form(&self, datasets: &[DatasetEntry]) -> Option<String> {
        if self.pull_dialog.selected_idx >= datasets.len() {
            return Some("请选择数据集".to_string());
        }
        let sym = self.pull_dialog.symbol.trim();
        if sym.is_empty() {
            return Some("请输入代码".to_string());
        }
        let sd = self.pull_dialog.start_date.trim();
        if !sd.is_empty() && (sd.len() != 8 || !sd.chars().all(|c| c.is_ascii_digit())) {
            return Some("起始日期格式应为 YYYYMMDD".to_string());
        }
        let ed = self.pull_dialog.end_date.trim();
        if !ed.is_empty() && (ed.len() != 8 || !ed.chars().all(|c| c.is_ascii_digit())) {
            return Some("结束日期格式应为 YYYYMMDD".to_string());
        }
        None
    }

    /// Render K-line chart in center canvas using egui_plot.
    /// Degrades to placeholder text if no data or on error.
    fn render_chart(&self, ui: &mut egui::Ui) {
        let snapshot = match &self.render_snapshot {
            Some(s) if !s.candles.is_empty() => s,
            _ => {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("中心画布区域");
                    ui.label("暂无K线数据，点击「加载图表」获取数据");
                });
                return;
            }
        };

        ui.heading(format!("{} - {} K线图", snapshot.symbol, snapshot.timeframe));

        // Build candle bodies (bars) and wicks (lines)
        let mut bull_bars = Vec::new();
        let mut bear_bars = Vec::new();
        let mut wick_points: Vec<[f64; 2]> = Vec::new();

        let bar_width = 0.6;

        for candle in &snapshot.candles {
            let x = candle.timestamp;
            let is_bull = candle.close >= candle.open;
            let body_bottom = candle.open.min(candle.close);
            let body_height = (candle.close - candle.open).abs().max(0.1);

            let bar = Bar::new(x, body_height).base_offset(body_bottom).width(bar_width);

            if is_bull {
                bull_bars.push(bar);
            } else {
                bear_bars.push(bar);
            }

            // Wick: high-low vertical line segments
            // We draw wicks as line segments by adding NaN-separated points
            wick_points.push([x, candle.low]);
            wick_points.push([x, candle.high]);
            wick_points.push([f64::NAN, f64::NAN]); // separator
        }

        let bull_chart = BarChart::new(bull_bars)
            .color(Color32::from_rgb(0, 180, 80))
            .name("阳线");

        let bear_chart = BarChart::new(bear_bars)
            .color(Color32::from_rgb(220, 50, 50))
            .name("阴线");

        let wick_line = Line::new(PlotPoints::new(wick_points))
            .color(Color32::from_rgb(150, 150, 150))
            .name("影线");

        Plot::new("kline_chart")
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .x_axis_label("时间")
            .y_axis_label("价格")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(bull_chart);
                plot_ui.bar_chart(bear_chart);
                plot_ui.line(wick_line);
            });
    }
}

impl Default for Workbench {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_candles_generated() {
        let candles = Workbench::generate_demo_candles();
        assert_eq!(candles.len(), 60);
        for c in &candles {
            assert!(c.high >= c.open.max(c.close), "high must be >= max(open,close)");
            assert!(c.low <= c.open.min(c.close), "low must be <= min(open,close)");
        }
    }

    #[test]
    fn render_snapshot_creation() {
        let snap = RenderSnapshot {
            symbol: "AAPL".into(),
            timeframe: "1D".into(),
            candles: vec![Candle {
                timestamp: 0.0,
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 102.0,
            }],
            provider: "akshare".into(),
            dataset_id: "stock_zh_a_hist".into(),
            market: "cn_stock".into(),
            capability: "ohlcv".into(),
        };
        assert_eq!(snap.candles.len(), 1);
        assert_eq!(snap.symbol, "AAPL");
        assert_eq!(snap.provider, "akshare");
        assert_eq!(snap.dataset_id, "stock_zh_a_hist");
        assert_eq!(snap.market, "cn_stock");
        assert_eq!(snap.capability, "ohlcv");
    }

    #[test]
    fn workbench_command_queue() {
        let mut wb = Workbench::new();
        assert!(wb.poll_command().is_none());
        wb.enqueue_command(WorkbenchCommand::RefreshData);
        assert!(matches!(wb.poll_command(), Some(WorkbenchCommand::RefreshData)));
        assert!(wb.poll_command().is_none());
    }

    // T5: Pull dialog state machine tests
    #[test]
    fn pull_dialog_initial_state_is_closed() {
        let wb = Workbench::new();
        assert_eq!(wb.pull_dialog.phase, PullDialogPhase::Closed);
    }

    #[test]
    fn pull_dialog_transitions_to_editing() {
        let mut wb = Workbench::new();
        wb.pull_dialog.phase = PullDialogPhase::Editing;
        assert_eq!(wb.pull_dialog.phase, PullDialogPhase::Editing);
    }

    #[test]
    fn pull_dialog_transitions_to_submitting() {
        let mut wb = Workbench::new();
        wb.pull_dialog.phase = PullDialogPhase::Submitting;
        assert_eq!(wb.pull_dialog.phase, PullDialogPhase::Submitting);
    }

    #[test]
    fn pull_dialog_transitions_to_done_success() {
        let mut wb = Workbench::new();
        wb.notify_pull_result(true, "成功".to_string());
        assert!(matches!(
            wb.pull_dialog.phase,
            PullDialogPhase::Done { success: true, .. }
        ));
    }

    #[test]
    fn pull_dialog_transitions_to_done_failure() {
        let mut wb = Workbench::new();
        wb.notify_pull_result(false, "失败".to_string());
        assert!(matches!(
            wb.pull_dialog.phase,
            PullDialogPhase::Done { success: false, .. }
        ));
    }

    // T5: Form validation tests
    #[test]
    fn validate_pull_form_empty_symbol() {
        let wb = Workbench::new();
        let datasets = available_datasets();
        let err = wb.validate_pull_form(datasets);
        assert!(err.is_some());
        assert!(err.unwrap().contains("请输入代码"));
    }

    #[test]
    fn validate_pull_form_invalid_start_date_format() {
        let mut wb = Workbench::new();
        wb.pull_dialog.symbol = "600000".to_string();
        wb.pull_dialog.start_date = "2024-01-01".to_string(); // wrong format
        let datasets = available_datasets();
        let err = wb.validate_pull_form(datasets);
        assert!(err.is_some());
        assert!(err.unwrap().contains("起始日期格式"));
    }

    #[test]
    fn validate_pull_form_invalid_end_date_format() {
        let mut wb = Workbench::new();
        wb.pull_dialog.symbol = "600000".to_string();
        wb.pull_dialog.end_date = "20240101X".to_string(); // contains non-digit
        let datasets = available_datasets();
        let err = wb.validate_pull_form(datasets);
        assert!(err.is_some());
        assert!(err.unwrap().contains("结束日期格式"));
    }

    #[test]
    fn validate_pull_form_valid_minimal() {
        let mut wb = Workbench::new();
        wb.pull_dialog.symbol = "600000".to_string();
        let datasets = available_datasets();
        let err = wb.validate_pull_form(datasets);
        assert!(err.is_none());
    }

    #[test]
    fn validate_pull_form_valid_with_dates() {
        let mut wb = Workbench::new();
        wb.pull_dialog.symbol = "600000".to_string();
        wb.pull_dialog.start_date = "20240101".to_string();
        wb.pull_dialog.end_date = "20241231".to_string();
        let datasets = available_datasets();
        let err = wb.validate_pull_form(datasets);
        assert!(err.is_none());
    }

    // T5: Command mapping tests
    #[test]
    fn pull_dataset_command_enqueued() {
        let mut wb = Workbench::new();
        wb.enqueue_command(WorkbenchCommand::PullDataset {
            provider: "akshare".to_string(),
            dataset_id: "cn_equity.ohlcv.daily".to_string(),
            symbol: "600000".to_string(),
            start_date: Some("20240101".to_string()),
            end_date: Some("20241231".to_string()),
        });
        let cmd = wb.poll_command();
        assert!(matches!(cmd, Some(WorkbenchCommand::PullDataset { .. })));
    }

    #[test]
    fn available_datasets_registry_not_empty() {
        let datasets = available_datasets();
        assert!(!datasets.is_empty());
        assert!(datasets.len() >= 3); // akshare, pytdx, mock
    }

    #[test]
    fn available_datasets_contains_akshare() {
        let datasets = available_datasets();
        let has_akshare = datasets.iter().any(|d| d.provider == "akshare");
        assert!(has_akshare);
    }

    #[test]
    fn available_datasets_contains_pytdx() {
        let datasets = available_datasets();
        let has_pytdx = datasets.iter().any(|d| d.provider == "pytdx");
        assert!(has_pytdx);
    }
}
