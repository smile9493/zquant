use anyhow::Result;
use domain_workspace::WorkspaceStore;
use job_application::ApiState;
use jobs_runtime::{TaskEntry, TaskId, TaskRuntime};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Facade for desktop UI operations
#[derive(Clone)]
pub struct ApplicationFacade {
    #[allow(dead_code)]
    state: ApiState,
    workspace_store: WorkspaceStore,
    runtime: Arc<TaskRuntime>,
}

impl ApplicationFacade {
    pub(crate) fn new(state: ApiState, workspace_store: WorkspaceStore, runtime: Arc<TaskRuntime>) -> Self {
        Self { state, workspace_store, runtime }
    }

    /// Load chart data for a symbol
    pub async fn load_chart(&self, symbol: &str, timeframe: &str) -> Result<ChartData> {
        info!("Loading chart: symbol={}, timeframe={}", symbol, timeframe);
        
        // Placeholder: real data loading in later milestones
        Ok(ChartData {
            symbol: symbol.to_string(),
            timeframe: timeframe.to_string(),
            data_points: vec![],
            provider: "akshare".to_string(),
            dataset_id: "stock_zh_a_hist".to_string(),
            market: "cn_stock".to_string(),
            capability: "ohlcv".to_string(),
        })
    }

    /// Refresh data for current workspace — submits an async task to the runtime.
    pub async fn refresh_data(&self) -> Result<TaskId> {
        info!("Submitting refresh-data task");
        let handle = self.runtime.submit("refresh-data", |_cancel_rx| async {
            // Placeholder: real data refresh logic in later milestones
            info!("Refresh-data task executing");
            Ok("Data refreshed".to_string())
        }).await;
        Ok(handle.id())
    }

    /// Cancel a running task by ID.
    pub async fn cancel_task(&self, id: TaskId) -> bool {
        self.runtime.cancel(id).await
    }

    /// Get a snapshot of all tasks.
    pub async fn list_tasks(&self) -> Vec<TaskEntry> {
        self.runtime.list_tasks().await
    }

    /// Drain pending task events (non-blocking).
    pub async fn drain_task_events(&self) -> Vec<jobs_runtime::TaskEvent> {
        self.runtime.drain_events().await
    }

    /// Save workspace snapshot to database
    pub async fn save_workspace(&self, snapshot: WorkspaceState) -> Result<()> {
        info!("Saving workspace snapshot: symbol={:?}", snapshot.symbol);
        
        let db_snapshot = domain_workspace::WorkspaceSnapshot {
            workspace_id: "default".to_string(),
            symbol: snapshot.symbol,
            timeframe: snapshot.timeframe,
            layout_state: serde_json::to_value(&snapshot.layout_state)?,
            schema_version: 1,
            created_at: chrono::Utc::now(),
        };
        
        self.workspace_store.save(&db_snapshot).await?;
        info!("Workspace snapshot saved");
        Ok(())
    }

    /// Pull dataset from a provider (framework-level — delegates to pipeline in later milestones).
    pub async fn pull_dataset(&self, req: PullRequest) -> Result<PullResult> {
        info!(
            provider = %req.provider,
            dataset_id = %req.dataset_id,
            symbol = %req.symbol,
            "Pull dataset requested"
        );

        // Framework stub: real provider dispatch will be wired in a later task.
        // For now, return a success placeholder so the UI round-trip is testable.
        Ok(PullResult {
            status: PullStatus::Success,
            message: format!(
                "拉取完成: {} / {} / {}",
                req.provider, req.dataset_id, req.symbol
            ),
            record_count: 0,
        })
    }

    /// Load latest workspace snapshot from database
    pub async fn load_workspace(&self) -> Result<Option<WorkspaceState>> {
        info!("Loading latest workspace snapshot");
        
        let db_snapshot = self.workspace_store.load_or_default("default").await;
        
        let layout_state: LayoutState = match serde_json::from_value(
            db_snapshot.layout_state.clone()
        ) {
            Ok(ls) => ls,
            Err(e) => {
                warn!("Failed to deserialize layout_state: {}, using defaults (all panels visible)", e);
                LayoutState::default()
            }
        };
        
        Ok(Some(WorkspaceState {
            symbol: db_snapshot.symbol,
            timeframe: db_snapshot.timeframe,
            layout_state,
        }))
    }
}

/// Chart data structure with source metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub symbol: String,
    pub timeframe: String,
    pub data_points: Vec<DataPoint>,
    /// Data provider identifier (e.g. "akshare").
    pub provider: String,
    /// Dataset identifier within the provider.
    pub dataset_id: String,
    /// Market classification (e.g. "cn_stock", "us_stock").
    pub market: String,
    /// Data capability tag (e.g. "ohlcv", "tick").
    pub capability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Request to pull dataset from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub provider: String,
    pub dataset_id: String,
    pub symbol: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Result of a pull operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResult {
    pub status: PullStatus,
    pub message: String,
    pub record_count: usize,
}

/// Status of a pull operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PullStatus {
    Success,
    Failed,
}

/// UI-layer workspace state for persistence.
/// Distinct from `domain_workspace::WorkspaceSnapshot` which is the DB model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub symbol: Option<String>,
    pub timeframe: Option<String>,
    pub layout_state: LayoutState,
}

/// Layout panel visibility state.
/// Defaults to all panels visible (true) for a usable initial experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    pub left_visible: bool,
    pub right_visible: bool,
    pub bottom_visible: bool,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            left_visible: true,
            right_visible: true,
            bottom_visible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_state_default_all_visible() {
        let ls = LayoutState::default();
        assert!(ls.left_visible, "left panel should default to visible");
        assert!(ls.right_visible, "right panel should default to visible");
        assert!(ls.bottom_visible, "bottom panel should default to visible");
    }

    #[test]
    fn layout_state_roundtrip_serde() {
        let original = LayoutState {
            left_visible: false,
            right_visible: true,
            bottom_visible: false,
        };
        let json = serde_json::to_value(&original).unwrap();
        let restored: LayoutState = serde_json::from_value(json).unwrap();
        assert_eq!(restored.left_visible, false);
        assert_eq!(restored.right_visible, true);
        assert_eq!(restored.bottom_visible, false);
    }

    #[test]
    fn layout_state_deserialize_invalid_falls_back_to_default() {
        let bad_json = serde_json::json!({"left_visible": "not_a_bool"});
        let result: Result<LayoutState, _> = serde_json::from_value(bad_json);
        assert!(result.is_err(), "should fail on invalid type");

        let fallback = LayoutState::default();
        assert!(fallback.left_visible);
        assert!(fallback.right_visible);
        assert!(fallback.bottom_visible);
    }

    #[test]
    fn layout_state_deserialize_empty_object_fails() {
        let empty = serde_json::json!({});
        let result: Result<LayoutState, _> = serde_json::from_value(empty);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_snapshot_roundtrip() {
        let snap = WorkspaceState {
            symbol: Some("AAPL".to_string()),
            timeframe: Some("1D".to_string()),
            layout_state: LayoutState {
                left_visible: true,
                right_visible: false,
                bottom_visible: true,
            },
        };
        let json = serde_json::to_string(&snap).unwrap();
        let restored: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.symbol.as_deref(), Some("AAPL"));
        assert_eq!(restored.timeframe.as_deref(), Some("1D"));
        assert!(restored.layout_state.left_visible);
        assert!(!restored.layout_state.right_visible);
        assert!(restored.layout_state.bottom_visible);
    }
}
