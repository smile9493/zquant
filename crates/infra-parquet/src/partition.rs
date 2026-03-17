//! Partition key and path management for Parquet files.
//!
//! Partition structure: {provider}/{exchange}/{symbol}/{timeframe}/{date}.parquet
//! Example: akshare/SSE/600000/1d/2024-03-17.parquet

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Partition key uniquely identifies a data partition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartitionKey {
    pub provider: String,
    pub exchange: String,
    pub symbol: String,
    pub timeframe: String,
    pub date: NaiveDate,
}

impl PartitionKey {
    pub fn new(
        provider: impl Into<String>,
        exchange: impl Into<String>,
        symbol: impl Into<String>,
        timeframe: impl Into<String>,
        date: NaiveDate,
    ) -> Self {
        Self {
            provider: provider.into(),
            exchange: exchange.into(),
            symbol: symbol.into(),
            timeframe: timeframe.into(),
            date,
        }
    }

    /// Convert timestamp to partition key
    pub fn from_timestamp(
        provider: impl Into<String>,
        exchange: impl Into<String>,
        symbol: impl Into<String>,
        timeframe: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        let date = timestamp.date_naive();
        Self::new(provider, exchange, symbol, timeframe, date)
    }
}

/// Partition path builder
pub struct PartitionPath;

impl PartitionPath {
    /// Build relative path from partition key
    /// Format: {provider}/{exchange}/{symbol}/{timeframe}/{date}.parquet
    pub fn build(key: &PartitionKey) -> PathBuf {
        PathBuf::from(&key.provider)
            .join(&key.exchange)
            .join(&key.symbol)
            .join(&key.timeframe)
            .join(format!("{}.parquet", key.date.format("%Y-%m-%d")))
    }

    /// Build absolute path from root and partition key
    pub fn build_absolute(root: &Path, key: &PartitionKey) -> PathBuf {
        root.join(Self::build(key))
    }

    /// Parse partition key from relative path
    pub fn parse(path: &Path) -> Result<PartitionKey> {
        let components: Vec<_> = path.components().collect();
        
        if components.len() != 5 {
            anyhow::bail!("Invalid partition path: expected 5 components, got {}", components.len());
        }

        let provider = components[0]
            .as_os_str()
            .to_str()
            .context("Invalid provider")?
            .to_string();
        
        let exchange = components[1]
            .as_os_str()
            .to_str()
            .context("Invalid exchange")?
            .to_string();
        
        let symbol = components[2]
            .as_os_str()
            .to_str()
            .context("Invalid symbol")?
            .to_string();
        
        let timeframe = components[3]
            .as_os_str()
            .to_str()
            .context("Invalid timeframe")?
            .to_string();
        
        let filename = components[4]
            .as_os_str()
            .to_str()
            .context("Invalid filename")?;
        
        let date_str = filename
            .strip_suffix(".parquet")
            .context("Filename must end with .parquet")?;
        
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context("Invalid date format")?;

        Ok(PartitionKey {
            provider,
            exchange,
            symbol,
            timeframe,
            date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn partition_key_creation() {
        let key = PartitionKey::new("akshare", "SSE", "600000", "1d", NaiveDate::from_ymd_opt(2024, 3, 17).unwrap());
        assert_eq!(key.provider, "akshare");
        assert_eq!(key.exchange, "SSE");
        assert_eq!(key.symbol, "600000");
        assert_eq!(key.timeframe, "1d");
        assert_eq!(key.date, NaiveDate::from_ymd_opt(2024, 3, 17).unwrap());
    }

    #[test]
    fn partition_path_build() {
        let key = PartitionKey::new("akshare", "SSE", "600000", "1d", NaiveDate::from_ymd_opt(2024, 3, 17).unwrap());
        let path = PartitionPath::build(&key);
        assert_eq!(path, PathBuf::from("akshare/SSE/600000/1d/2024-03-17.parquet"));
    }

    #[test]
    fn partition_path_roundtrip() {
        let key = PartitionKey::new("akshare", "SSE", "600000", "1d", NaiveDate::from_ymd_opt(2024, 3, 17).unwrap());
        let path = PartitionPath::build(&key);
        let parsed = PartitionPath::parse(&path).unwrap();
        assert_eq!(parsed, key);
    }

    #[test]
    fn partition_path_parse_invalid() {
        let path = PathBuf::from("invalid/path");
        assert!(PartitionPath::parse(&path).is_err());
    }
}
