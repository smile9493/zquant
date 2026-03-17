CREATE TABLE IF NOT EXISTS workspace_snapshots (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(255) NOT NULL,
    symbol VARCHAR(64),
    timeframe VARCHAR(32),
    layout_state JSONB NOT NULL DEFAULT '{}'::jsonb,
    schema_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Latest snapshot lookup: WHERE workspace_id = ? ORDER BY created_at DESC LIMIT 1
CREATE INDEX IF NOT EXISTS idx_workspace_snapshots_workspace_created
    ON workspace_snapshots (workspace_id, created_at DESC);
