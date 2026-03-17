use egui::{Context, CentralPanel, TopBottomPanel, SidePanel};
use serde::{Deserialize, Serialize};
use tracing::debug;
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

/// Commands that workbench can send to application layer
#[derive(Debug, Clone)]
pub enum WorkbenchCommand {
    LoadChart { symbol: String, timeframe: String },
    RefreshData,
    SaveWorkspace(WorkspaceSnapshot),
}

/// Workbench manages the main UI layout
pub struct Workbench {
    panel_state: PanelState,
    command_queue: VecDeque<WorkbenchCommand>,
    current_symbol: String,
    current_timeframe: String,
}

impl Workbench {
    pub fn new() -> Self {
        debug!("Creating new Workbench");
        Self {
            panel_state: PanelState::default(),
            command_queue: VecDeque::new(),
            current_symbol: "AAPL".to_string(),
            current_timeframe: "1D".to_string(),
        }
    }

    /// Poll for pending commands
    pub fn poll_command(&mut self) -> Option<WorkbenchCommand> {
        self.command_queue.pop_front()
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
                
                // Panel toggle buttons
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
                
                // Action buttons (trigger facade calls)
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

        // Bottom dock
        if self.panel_state.bottom_visible {
            TopBottomPanel::bottom("bottom_dock").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("底部面板");
                    ui.separator();
                    ui.label("日志 | 任务 | 终端");
                });
            });
        }

        // Left sidebar
        if self.panel_state.left_visible {
            SidePanel::left("left_sidebar")
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("导航");
                    ui.separator();
                    ui.label("• 数据源");
                    ui.label("• 策略");
                    ui.label("• 回测");
                });
        }

        // Right dock
        if self.panel_state.right_visible {
            SidePanel::right("right_dock")
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.heading("属性");
                    ui.separator();
                    ui.label(format!("Symbol: {}", self.current_symbol));
                    ui.label(format!("Timeframe: {}", self.current_timeframe));
                    ui.label("Status: Ready");
                });
        }

        // Center canvas (main area)
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading("中心画布区域");
                ui.add_space(20.0);
                ui.label("工作台五区布局已就绪");
                ui.add_space(10.0);
                ui.label("Top: 顶部工具栏（含操作按钮）");
                ui.label("Left: 左侧导航");
                ui.label("Center: 中心画布（当前区域）");
                ui.label("Right: 右侧属性面板");
                ui.label("Bottom: 底部日志/任务面板");
                ui.add_space(20.0);
                ui.label("点击顶部按钮可触发进程内 Facade 调用");
            });
        });
    }
}

impl Default for Workbench {
    fn default() -> Self {
        Self::new()
    }
}
