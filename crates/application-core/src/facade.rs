use anyhow::Result;
use domain_workspace::WorkspaceStore;
use job_application::ApiState;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Facade for desktop UI operations
#[derive(Clone)]
pub struct ApplicationFacade {
    #[allow(dead_code)]
    state: ApiState,
    workspace_store: WorkspaceStore,
}

impl ApplicationFacade {
    pub(crate) fn new(state: ApiState, workspace_store: WorkspaceStore) -> Self {
        Self { state, workspace_store }
    }

    /// Load chart data for a symbol
    pub async fn load_chart(&self, symbol: &str, timeframe: &str) -> Result<ChartData> {
        info!("Loading chart: symbol={}, timeframe={}", symbol, timeframe);
        
        // Placeholder: real data loading in later milestones
        Ok(ChartData {
            symbol: symbol.to_string(),
            timeframe: timeframe.to_string(),
            data_points: vec![],
        })
    }

    /// Refresh data for current workspace
    pub async fn refresh_data(&self) -> Result<()> {
        info!("Refreshing workspace data");
        Ok(())
    }

    /// Save workspace snapshot to database
    pub async fn save_workspace(&self, snapshot: WorkspaceSnapshot) -> Result<()> {
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

    /// Load latest workspace snapshot from database
    pub async fn load_workspace(&self) -> Result<Option<WorkspaceSnapshot>> {
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
        
        Ok(Some(WorkspaceSnapshot {
            symbol: db_snapshot.symbol,
            timeframe: db_snapshot.timeframe,
            layout_state,
        }))
    }
}

/// Chart data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub symbol: String,
    pub timeframe: String,
    pub data_points: Vec<DataPoint>,
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

/// Workspace snapshot for state persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
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
        let snap = WorkspaceSnapshot {
            symbol: Some("AAPL".to_string()),
            timeframe: Some("1D".to_string()),
            layout_state: LayoutState {
                left_visible: true,
                right_visible: false,
                bottom_visible: true,
            },
        };
        let json = serde_json::to_string(&snap).unwrap();
        let restored: WorkspaceSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.symbol.as_deref(), Some("AAPL"));
        assert_eq!(restored.timeframe.as_deref(), Some("1D"));
        assert!(restored.layout_state.left_visible);
        assert!(!restored.layout_state.right_visible);
        assert!(restored.layout_state.bottom_visible);
    }
}
