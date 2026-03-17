-- Partition manifest table for Parquet archive tracking
-- Tracks metadata for each Parquet partition file

CREATE TABLE IF NOT EXISTS partition_manifest (
    id BIGSERIAL PRIMARY KEY,
    
    -- Partition key components
    provider VARCHAR(64) NOT NULL,
    exchange VARCHAR(64) NOT NULL,
    symbol VARCHAR(64) NOT NULL,
    timeframe VARCHAR(16) NOT NULL,
    partition_date DATE NOT NULL,
    
    -- File metadata
    file_path TEXT NOT NULL,
    row_count BIGINT NOT NULL,
    min_timestamp TIMESTAMPTZ NOT NULL,
    max_timestamp TIMESTAMPTZ NOT NULL,
    
    -- Tracking
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Unique constraint on partition key
    CONSTRAINT partition_manifest_unique_key UNIQUE (provider, exchange, symbol, timeframe, partition_date)
);

-- Index for common query patterns
CREATE INDEX idx_partition_manifest_symbol_timeframe 
    ON partition_manifest (symbol, timeframe, partition_date DESC);

CREATE INDEX idx_partition_manifest_provider_exchange 
    ON partition_manifest (provider, exchange, symbol);

CREATE INDEX idx_partition_manifest_timestamp_range 
    ON partition_manifest (min_timestamp, max_timestamp);

-- Comments
COMMENT ON TABLE partition_manifest IS 'Tracks Parquet partition files for market data archive';
COMMENT ON COLUMN partition_manifest.provider IS 'Data provider (e.g., akshare, yahoo)';
COMMENT ON COLUMN partition_manifest.exchange IS 'Exchange code (e.g., SSE, NASDAQ)';
COMMENT ON COLUMN partition_manifest.symbol IS 'Trading symbol';
COMMENT ON COLUMN partition_manifest.timeframe IS 'Data timeframe (e.g., 1d, 1h)';
COMMENT ON COLUMN partition_manifest.partition_date IS 'Partition date (YYYY-MM-DD)';
COMMENT ON COLUMN partition_manifest.file_path IS 'Relative path to Parquet file';
COMMENT ON COLUMN partition_manifest.row_count IS 'Number of rows in partition';
COMMENT ON COLUMN partition_manifest.min_timestamp IS 'Earliest timestamp in partition';
COMMENT ON COLUMN partition_manifest.max_timestamp IS 'Latest timestamp in partition';
