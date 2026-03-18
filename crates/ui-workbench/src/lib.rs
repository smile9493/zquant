use egui::{Context, CentralPanel, TopBottomPanel, SidePanel, Color32};
use egui_plot::{Plot, Bar, BarChart, Line, PlotPoints};
use jobs_runtime::{TaskEntry, TaskStatus};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use application_core::WorkspaceSnapshot;
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
    SaveWorkspace(WorkspaceSnapshot),
    CancelTask(jobs_runtime::TaskId),
}

/// Workbench manages the main UI layout and chart rendering
pub struct Workbench {
    panel_state: PanelState,
    command_queue: VecDeque<WorkbenchCommand>,
    current_symbol: String,
    current_timeframe: String,
    render_snapshot: Option<RenderSnapshot>,
    task_entries: Vec<TaskEntry>,
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
    pub fn restore_from_snapshot(&mut self, snapshot: &WorkspaceSnapshot) {
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
    pub fn create_snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
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

        // Center canvas with K-line chart
        CentralPanel::default().show(ctx, |ui| {
            self.render_chart(ui);
        });
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
}
