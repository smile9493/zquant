use anyhow::Result;
use job_application::ApiState;
use job_events::bus::{EventBus, InMemoryEventBus};
use job_store_pg::JobStore;
use domain_workspace::WorkspaceStore;
use std::sync::Arc;
use tracing::info;
use sqlx::PgPool;

mod facade;

pub use facade::{ApplicationFacade, ChartData, WorkspaceSnapshot, LayoutState};

/// Application core initialization
pub struct ApplicationCore {
    state: ApiState,
    workspace_store: WorkspaceStore,
}

impl ApplicationCore {
    /// Initialize application core with database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Initializing application core");

        // Create database pool
        let pool = PgPool::connect(database_url).await?;
        
        // Initialize store
        let store = Arc::new(JobStore::new(pool.clone()));
        
        // Initialize workspace store
        let workspace_store = WorkspaceStore::new(pool);
        
        // Initialize event bus with capacity 100
        let bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new(100));

        let state = ApiState { store, bus };

        info!("Application core initialized");
        Ok(Self { state, workspace_store })
    }

    /// Get facade for UI operations
    pub fn facade(&self) -> ApplicationFacade {
        ApplicationFacade::new(self.state.clone(), self.workspace_store.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_core_exports_expected_types() {
        // Verify public API surface is accessible
        let _: fn() -> LayoutState = LayoutState::default;
        let _: fn() -> WorkspaceSnapshot = || WorkspaceSnapshot {
            symbol: None,
            timeframe: None,
            layout_state: LayoutState::default(),
        };
        // ChartData is constructible
        let cd = ChartData {
            symbol: "TEST".into(),
            timeframe: "1D".into(),
            data_points: vec![],
        };
        assert_eq!(cd.symbol, "TEST");
        assert!(cd.data_points.is_empty());
    }
}
