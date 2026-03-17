//! Parquet writer with atomic write operations.
//!
//! Write flow: tmp -> flush -> rename
//! Ensures no partial files are visible to readers.

use crate::{ArchiveConfig, MarketDataPoint, PartitionKey, PartitionMetadata, PartitionPath};
use anyhow::{Context, Result};
use arrow::array::{Float64Array, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info, warn};

/// Parquet writer for market data
pub struct ParquetWriter {
    config: ArchiveConfig,
}

impl ParquetWriter {
    pub fn new(config: ArchiveConfig) -> Self {
        Self { config }
    }

    /// Write market data points to Parquet file atomically
    pub async fn write(
        &self,
        key: &PartitionKey,
        data: &[MarketDataPoint],
    ) -> Result<PartitionMetadata> {
        if data.is_empty() {
            anyhow::bail!("Cannot write empty data");
        }

        // Ensure tmp directory exists
        fs::create_dir_all(&self.config.tmp_path)
            .await
            .context("Failed to create tmp directory")?;

        // Generate tmp file path
        let tmp_filename = format!(
            "{}_{}_{}_{}_{}.parquet.tmp",
            key.provider, key.exchange, key.symbol, key.timeframe, key.date
        );
        let tmp_path = self.config.tmp_path.join(&tmp_filename);

        debug!(
            partition = ?key,
            tmp_path = ?tmp_path,
            row_count = data.len(),
            "Writing Parquet file to tmp"
        );

        // Write to tmp file (blocking I/O)
        let tmp_path_clone = tmp_path.clone();
        let data_clone = data.to_vec();
        let row_count = data.len() as i64;
        
        tokio::task::spawn_blocking(move || {
            Self::write_parquet_file(&tmp_path_clone, &data_clone)
        })
        .await
        .context("Parquet write task panicked")??;

        // Build final path
        let final_relative_path = PartitionPath::build(key);
        let final_path = self.config.root_path.join(&final_relative_path);

        // Ensure parent directory exists
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create partition directory")?;
        }

        // Atomic rename: tmp -> final
        fs::rename(&tmp_path, &final_path)
            .await
            .context("Failed to rename tmp file to final path")?;

        info!(
            partition = ?key,
            final_path = ?final_path,
            row_count,
            "Parquet file written successfully"
        );

        // Build metadata
        let min_timestamp = data
            .iter()
            .map(|d| d.timestamp)
            .min()
            .ok_or_else(|| anyhow::anyhow!("Cannot compute min_timestamp from empty data"))?;
        
        let max_timestamp = data
            .iter()
            .map(|d| d.timestamp)
            .max()
            .ok_or_else(|| anyhow::anyhow!("Cannot compute max_timestamp from empty data"))?;

        Ok(PartitionMetadata {
            partition_key: key.clone(),
            file_path: final_relative_path,
            row_count,
            min_timestamp,
            max_timestamp,
            created_at: chrono::Utc::now(),
        })
    }

    /// Write Parquet file (blocking operation)
    fn write_parquet_file(path: &PathBuf, data: &[MarketDataPoint]) -> Result<()> {
        let schema = Self::build_schema();
        let batch = Self::build_record_batch(&schema, data)?;

        let file = File::create(path).context("Failed to create Parquet file")?;
        
        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))
            .context("Failed to create ArrowWriter")?;

        writer
            .write(&batch)
            .context("Failed to write record batch")?;

        writer.close().context("Failed to close ArrowWriter")?;

        Ok(())
    }

    /// Build Arrow schema for market data
    fn build_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Millisecond, None),
                false,
            ),
            Field::new("open", DataType::Float64, false),
            Field::new("high", DataType::Float64, false),
            Field::new("low", DataType::Float64, false),
            Field::new("close", DataType::Float64, false),
            Field::new("volume", DataType::Float64, false),
        ]))
    }

    /// Build Arrow record batch from market data
    fn build_record_batch(
        schema: &Arc<Schema>,
        data: &[MarketDataPoint],
    ) -> Result<RecordBatch> {
        let timestamps: Vec<i64> = data.iter().map(|d| d.timestamp.timestamp_millis()).collect();
        let opens: Vec<f64> = data.iter().map(|d| d.open).collect();
        let highs: Vec<f64> = data.iter().map(|d| d.high).collect();
        let lows: Vec<f64> = data.iter().map(|d| d.low).collect();
        let closes: Vec<f64> = data.iter().map(|d| d.close).collect();
        let volumes: Vec<f64> = data.iter().map(|d| d.volume).collect();

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(TimestampMillisecondArray::from(timestamps)),
                Arc::new(Float64Array::from(opens)),
                Arc::new(Float64Array::from(highs)),
                Arc::new(Float64Array::from(lows)),
                Arc::new(Float64Array::from(closes)),
                Arc::new(Float64Array::from(volumes)),
            ],
        )
        .context("Failed to create record batch")?;

        Ok(batch)
    }

    /// Cleanup tmp files (for error recovery)
    pub async fn cleanup_tmp(&self) -> Result<()> {
        if !self.config.tmp_path.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&self.config.tmp_path)
            .await
            .context("Failed to read tmp directory")?;

        let mut cleaned = 0;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("tmp") {
                if let Err(e) = fs::remove_file(&path).await {
                    warn!(path = ?path, error = %e, "Failed to remove tmp file");
                } else {
                    cleaned += 1;
                }
            }
        }

        if cleaned > 0 {
            info!(cleaned, "Cleaned up tmp files");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use tempfile::TempDir;

    fn create_test_data() -> Vec<MarketDataPoint> {
        vec![
            MarketDataPoint {
                timestamp: Utc.with_ymd_and_hms(2024, 3, 17, 9, 30, 0).unwrap(),
                open: 100.0,
                high: 105.0,
                low: 99.0,
                close: 103.0,
                volume: 1000.0,
            },
            MarketDataPoint {
                timestamp: Utc.with_ymd_and_hms(2024, 3, 17, 10, 30, 0).unwrap(),
                open: 103.0,
                high: 107.0,
                low: 102.0,
                close: 106.0,
                volume: 1500.0,
            },
        ]
    }

    #[tokio::test]
    async fn write_parquet_file_success() {
        let tmp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig::new(tmp_dir.path().to_path_buf());
        let writer = ParquetWriter::new(config);

        let key = PartitionKey::new(
            "akshare",
            "SSE",
            "600000",
            "1d",
            NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
        );
        let data = create_test_data();

        let metadata = writer.write(&key, &data).await.unwrap();

        assert_eq!(metadata.row_count, 2);
        assert_eq!(metadata.partition_key, key);
        
        let final_path = tmp_dir.path().join(metadata.file_path);
        assert!(final_path.exists());
    }

    #[tokio::test]
    async fn write_empty_data_fails() {
        let tmp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig::new(tmp_dir.path().to_path_buf());
        let writer = ParquetWriter::new(config);

        let key = PartitionKey::new(
            "akshare",
            "SSE",
            "600000",
            "1d",
            NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
        );

        let result = writer.write(&key, &[]).await;
        assert!(result.is_err());
    }
}
