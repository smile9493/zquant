//! Parquet reader for market data.

use crate::{ArchiveConfig, MarketDataPoint, PartitionKey, PartitionPath};
use anyhow::{Context, Result};
use arrow::array::{Array, Float64Array, TimestampMillisecondArray};
use chrono::{DateTime, TimeZone, Utc};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
use tracing::{debug, info};

/// Parquet reader for market data
pub struct ParquetReader {
    config: ArchiveConfig,
}

impl ParquetReader {
    pub fn new(config: ArchiveConfig) -> Self {
        Self { config }
    }

    /// Read market data from Parquet file
    pub async fn read(&self, key: &PartitionKey) -> Result<Vec<MarketDataPoint>> {
        let file_path = PartitionPath::build_absolute(&self.config.root_path, key);

        if !file_path.exists() {
            anyhow::bail!("Partition file not found: {:?}", file_path);
        }

        debug!(
            partition = ?key,
            file_path = ?file_path,
            "Reading Parquet file"
        );

        // Read Parquet file (blocking I/O)
        let file_path_clone = file_path.clone();
        let data = tokio::task::spawn_blocking(move || Self::read_parquet_file(&file_path_clone))
            .await
            .context("Parquet read task panicked")??;

        info!(
            partition = ?key,
            row_count = data.len(),
            "Parquet file read successfully"
        );

        Ok(data)
    }

    /// Read Parquet file (blocking operation)
    fn read_parquet_file(path: &std::path::Path) -> Result<Vec<MarketDataPoint>> {
        let file = File::open(path).context("Failed to open Parquet file")?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .context("Failed to create ParquetRecordBatchReaderBuilder")?;

        let mut reader = builder.build().context("Failed to build reader")?;

        let mut data = Vec::new();

        while let Some(batch) = reader.next() {
            let batch = batch.context("Failed to read record batch")?;

            let timestamps = batch
                .column(0)
                .as_any()
                .downcast_ref::<TimestampMillisecondArray>()
                .context("Invalid timestamp column")?;

            let opens = batch
                .column(1)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid open column")?;

            let highs = batch
                .column(2)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid high column")?;

            let lows = batch
                .column(3)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid low column")?;

            let closes = batch
                .column(4)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid close column")?;

            let volumes = batch
                .column(5)
                .as_any()
                .downcast_ref::<Float64Array>()
                .context("Invalid volume column")?;

            for i in 0..batch.num_rows() {
                let timestamp_ms = timestamps.value(i);
                let timestamp = Utc
                    .timestamp_millis_opt(timestamp_ms)
                    .single()
                    .context("Invalid timestamp")?;

                data.push(MarketDataPoint {
                    timestamp,
                    open: opens.value(i),
                    high: highs.value(i),
                    low: lows.value(i),
                    close: closes.value(i),
                    volume: volumes.value(i),
                });
            }
        }

        Ok(data)
    }

    /// Read market data within time range
    pub async fn read_range(
        &self,
        key: &PartitionKey,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketDataPoint>> {
        let all_data = self.read(key).await?;

        let filtered: Vec<_> = all_data
            .into_iter()
            .filter(|d| d.timestamp >= start && d.timestamp < end)
            .collect();

        debug!(
            partition = ?key,
            start = %start,
            end = %end,
            filtered_count = filtered.len(),
            "Filtered data by time range"
        );

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::ParquetWriter;
    use chrono::{NaiveDate, TimeZone};
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
    async fn read_parquet_file_success() {
        let tmp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let key = PartitionKey::new(
            "akshare",
            "SSE",
            "600000",
            "1d",
            NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
        );
        let data = create_test_data();

        // Write first
        let writer = ParquetWriter::new(config.clone());
        writer.write(&key, &data).await.unwrap();

        // Then read
        let reader = ParquetReader::new(config);
        let read_data = reader.read(&key).await.unwrap();

        assert_eq!(read_data.len(), 2);
        assert_eq!(read_data[0].open, 100.0);
        assert_eq!(read_data[1].close, 106.0);
    }

    #[tokio::test]
    async fn read_range_filters_correctly() {
        let tmp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let key = PartitionKey::new(
            "akshare",
            "SSE",
            "600000",
            "1d",
            NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
        );
        let data = create_test_data();

        // Write
        let writer = ParquetWriter::new(config.clone());
        writer.write(&key, &data).await.unwrap();

        // Read with range
        let reader = ParquetReader::new(config);
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 11, 0, 0).unwrap();
        
        let filtered = reader.read_range(&key, start, end).await.unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].open, 103.0);
    }
}
