//! Parquet-based archive storage for market data.
//!
//! This module provides:
//! - Partition-based file organization (provider/exchange/symbol/timeframe/date)
//! - Atomic write operations (tmp -> flush -> rename)
//! - Manifest-driven read operations
//! - Integration with PostgreSQL manifest table

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod partition;
mod reader;
mod writer;

pub use partition::{PartitionKey, PartitionPath};
pub use reader::ParquetReader;
pub use writer::ParquetWriter;

/// Market data point for Parquet storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataPoint {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Partition metadata for manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMetadata {
    pub partition_key: PartitionKey,
    pub file_path: PathBuf,
    pub row_count: i64,
    pub min_timestamp: DateTime<Utc>,
    pub max_timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Archive storage configuration
#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    /// Root directory for Parquet files
    pub root_path: PathBuf,
    /// Temporary directory for atomic writes
    pub tmp_path: PathBuf,
}

impl ArchiveConfig {
    pub fn new(root_path: PathBuf) -> Self {
        let tmp_path = root_path.join(".tmp");
        Self { root_path, tmp_path }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_config_creates_tmp_path() {
        let config = ArchiveConfig::new(PathBuf::from("/data/archive"));
        assert_eq!(config.root_path, PathBuf::from("/data/archive"));
        assert_eq!(config.tmp_path, PathBuf::from("/data/archive/.tmp"));
    }
}
