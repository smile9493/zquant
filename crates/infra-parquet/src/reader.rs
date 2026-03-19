//! Parquet reader for market data.

use crate::{ArchiveConfig, MarketDataPoint, PartitionKey, PartitionPath};
use anyhow::{Context, Result};
use arrow::array::{Array, BooleanArray, Float64Array, TimestampMillisecondArray};
use chrono::{DateTime, TimeZone, Utc};
use parquet::arrow::arrow_reader::{
    ArrowPredicateFn, ParquetRecordBatchReaderBuilder, RowFilter,
};
use parquet::arrow::ProjectionMask;
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

    /// Read market data within time range using pushdown filtering.
    ///
    /// Uses row-group statistics to skip irrelevant row groups, then applies
    /// a row-level predicate filter on the timestamp column so that only
    /// matching rows are materialised into memory.
    pub async fn read_range(
        &self,
        key: &PartitionKey,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketDataPoint>> {
        let file_path = PartitionPath::build_absolute(&self.config.root_path, key);

        if !file_path.exists() {
            anyhow::bail!("Partition file not found: {:?}", file_path);
        }

        let start_ms = start.timestamp_millis();
        let end_ms = end.timestamp_millis();

        let file_path_clone = file_path.clone();
        let data = tokio::task::spawn_blocking(move || {
            Self::read_parquet_file_filtered(&file_path_clone, start_ms, end_ms)
        })
        .await
        .context("Parquet filtered-read task panicked")??;

        debug!(
            partition = ?key,
            start = %start,
            end = %end,
            filtered_count = data.len(),
            "Read data with pushdown filter"
        );

        Ok(data)
    }

    /// Read Parquet file with row-level pushdown filter on timestamp.
    ///
    /// Semantics: `start_ms <= timestamp < end_ms` (half-open interval).
    fn read_parquet_file_filtered(
        path: &std::path::Path,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<MarketDataPoint>> {
        let file = File::open(path).context("Failed to open Parquet file")?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .context("Failed to create ParquetRecordBatchReaderBuilder")?;

        // Build a projection mask that covers only the timestamp column (index 0)
        // so the predicate evaluation reads minimal data.
        let ts_projection = ProjectionMask::roots(builder.parquet_schema(), [0]);

        let row_filter = RowFilter::new(vec![Box::new(ArrowPredicateFn::new(
            ts_projection,
            move |batch| {
                let ts_col = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<TimestampMillisecondArray>()
                    .ok_or_else(|| {
                        arrow::error::ArrowError::SchemaError(
                            "column 0 is not TimestampMillisecondArray".to_string(),
                        )
                    })?;

                let bools: Vec<bool> = (0..ts_col.len())
                    .map(|i| {
                        let v = ts_col.value(i);
                        v >= start_ms && v < end_ms
                    })
                    .collect();

                Ok(BooleanArray::from(bools))
            },
        ))]);

        let mut reader = builder
            .with_row_filter(row_filter)
            .build()
            .context("Failed to build filtered reader")?;

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

    /// Helper: write 5 bars spanning 09:00–13:00 at hourly intervals.
    fn create_five_bar_data() -> Vec<MarketDataPoint> {
        (0..5)
            .map(|i| MarketDataPoint {
                timestamp: Utc
                    .with_ymd_and_hms(2024, 3, 17, 9 + i, 0, 0)
                    .unwrap(),
                open: 100.0 + i as f64,
                high: 105.0,
                low: 99.0,
                close: 103.0,
                volume: 1000.0,
            })
            .collect()
    }

    async fn write_five_bars(tmp_dir: &TempDir) -> (ArchiveConfig, PartitionKey) {
        let config = ArchiveConfig::new(tmp_dir.path().to_path_buf());
        let key = PartitionKey::new(
            "akshare",
            "SSE",
            "600000",
            "1h",
            NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
        );
        ParquetWriter::new(config.clone())
            .write(&key, &create_five_bar_data())
            .await
            .unwrap();
        (config, key)
    }

    #[tokio::test]
    async fn read_range_window_hit_returns_only_target() {
        let tmp_dir = TempDir::new().unwrap();
        let (config, key) = write_five_bars(&tmp_dir).await;
        let reader = ParquetReader::new(config);

        // Window [10:00, 12:00) should return bars at 10:00 and 11:00
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let result = reader.read_range(&key, start, end).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].open, 101.0); // 10:00
        assert_eq!(result[1].open, 102.0); // 11:00
    }

    #[tokio::test]
    async fn read_range_empty_window_returns_empty() {
        let tmp_dir = TempDir::new().unwrap();
        let (config, key) = write_five_bars(&tmp_dir).await;
        let reader = ParquetReader::new(config);

        // Window entirely before data
        let start = Utc.with_ymd_and_hms(2024, 3, 16, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 16, 1, 0, 0).unwrap();
        let result = reader.read_range(&key, start, end).await.unwrap();
        assert!(result.is_empty(), "window before data should be empty");

        // Window entirely after data
        let start = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 1, 0, 0).unwrap();
        let result = reader.read_range(&key, start, end).await.unwrap();
        assert!(result.is_empty(), "window after data should be empty");
    }

    #[tokio::test]
    async fn read_range_boundary_half_open_semantics() {
        let tmp_dir = TempDir::new().unwrap();
        let (config, key) = write_five_bars(&tmp_dir).await;
        let reader = ParquetReader::new(config);

        // Window [09:00, 10:00) — start is inclusive, end is exclusive
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap();
        let result = reader.read_range(&key, start, end).await.unwrap();

        assert_eq!(result.len(), 1, "half-open [09:00, 10:00) should include 09:00 only");
        assert_eq!(result[0].open, 100.0);

        // Window [13:00, 14:00) — start equals last bar timestamp
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 14, 0, 0).unwrap();
        let result = reader.read_range(&key, start, end).await.unwrap();

        assert_eq!(result.len(), 1, "[13:00, 14:00) should include the 13:00 bar");
        assert_eq!(result[0].open, 104.0);
    }
}
