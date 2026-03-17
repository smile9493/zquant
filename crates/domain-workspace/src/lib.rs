use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::{info, warn, debug};

/// Workspace snapshot persisted to PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub workspace_id: String,
    pub symbol: Option<String>,
    pub timeframe: Option<String>,
    pub layout_state: serde_json::Value,
    pub schema_version: i32,
    pub created_at: DateTime<Utc>,
}

/// Default layout state for fallback
fn default_layout_state() -> serde_json::Value {
    serde_json::json!({
        "left_visible": true,
        "right_visible": true,
        "bottom_visible": true
    })
}

/// Workspace state store backed by PostgreSQL
#[derive(Clone)]
pub struct WorkspaceStore {
    pool: PgPool,
}

impl WorkspaceStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load the latest snapshot for a workspace
    pub async fn load_latest(&self, workspace_id: &str) -> Result<Option<WorkspaceSnapshot>> {
        info!(workspace_id, "Loading latest workspace snapshot");

        let row = sqlx::query(
            r#"
            SELECT workspace_id, symbol, timeframe,
                   layout_state, schema_version, created_at
            FROM workspace_snapshots
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let created_at: DateTime<Utc> = r.get("created_at");
                debug!(workspace_id, "Snapshot found, created_at={}", created_at);
                Ok(Some(WorkspaceSnapshot {
                    workspace_id: r.get("workspace_id"),
                    symbol: r.get("symbol"),
                    timeframe: r.get("timeframe"),
                    layout_state: r.get("layout_state"),
                    schema_version: r.get("schema_version"),
                    created_at,
                }))
            }
            None => {
                info!(workspace_id, "No snapshot found, will use defaults");
                Ok(None)
            }
        }
    }

    /// Save a new snapshot (append-only)
    pub async fn save(&self, snapshot: &WorkspaceSnapshot) -> Result<()> {
        info!(
            workspace_id = snapshot.workspace_id,
            symbol = ?snapshot.symbol,
            timeframe = ?snapshot.timeframe,
            "Saving workspace snapshot"
        );

        sqlx::query(
            r#"
            INSERT INTO workspace_snapshots
                (workspace_id, symbol, timeframe, layout_state, schema_version)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&snapshot.workspace_id)
        .bind(&snapshot.symbol)
        .bind(&snapshot.timeframe)
        .bind(&snapshot.layout_state)
        .bind(snapshot.schema_version)
        .execute(&self.pool)
        .await?;

        debug!(workspace_id = snapshot.workspace_id, "Snapshot saved");
        Ok(())
    }

    /// Load latest with fallback to defaults on error
    pub async fn load_or_default(&self, workspace_id: &str) -> WorkspaceSnapshot {
        match self.load_latest(workspace_id).await {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                info!(workspace_id, "Using default workspace state");
                WorkspaceSnapshot {
                    workspace_id: workspace_id.to_string(),
                    symbol: None,
                    timeframe: None,
                    layout_state: default_layout_state(),
                    schema_version: 1,
                    created_at: Utc::now(),
                }
            }
            Err(e) => {
                warn!(
                    workspace_id,
                    error = %e,
                    "Failed to load snapshot, falling back to defaults"
                );
                WorkspaceSnapshot {
                    workspace_id: workspace_id.to_string(),
                    symbol: None,
                    timeframe: None,
                    layout_state: default_layout_state(),
                    schema_version: 1,
                    created_at: Utc::now(),
                }
            }
        }
    }
}
