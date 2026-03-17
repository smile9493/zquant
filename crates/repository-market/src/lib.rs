//! Market data repository with layered storage strategy.
//!
//! Read flow: Hot PostgreSQL → Parquet archive → Remote provider
//! Write flow: PostgreSQL + Parquet (atomic) + Manifest update

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use infra_parquet::{ArchiveConfig, MarketDataPoint, PartitionKey, ParquetReader};
use sqlx::PgPool;
use store_manifest::{ManifestStore, PartitionRecord};
use tracing::{debug, info, warn};

mod gap;
mod hot_store;

pub use gap::{Gap, GapCalculator};
pub use hot_store::HotStore;

/// Trait for hot store operations (enables testing)
#[async_trait]
pub trait HotStoreOps: Send + Sync {
    async fn load_bars(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>>;
}

#[async_trait]
impl HotStoreOps for HotStore {
    async fn load_bars(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>> {
        self.load_bars(provider, exchange, symbol, timeframe, start, end)
            .await
    }
}

/// Trait for manifest store operations (enables testing)
#[async_trait]
pub trait ManifestStoreOps: Send + Sync {
    async fn find_partitions(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PartitionRecord>>;
}

#[async_trait]
impl ManifestStoreOps for ManifestStore {
    async fn find_partitions(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PartitionRecord>> {
        self.find_partitions(provider, exchange, symbol, timeframe, start, end)
            .await
    }
}

/// Trait for Parquet reader operations (enables testing)
#[async_trait]
pub trait ParquetReaderOps: Send + Sync {
    async fn read_range(
        &self,
        key: &PartitionKey,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketDataPoint>>;
}

#[async_trait]
impl ParquetReaderOps for ParquetReader {
    async fn read_range(
        &self,
        key: &PartitionKey,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketDataPoint>> {
        self.read_range(key, start, end).await
    }
}

/// Market data bar (OHLCV)
#[derive(Debug, Clone)]
pub struct Bar {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl From<MarketDataPoint> for Bar {
    fn from(point: MarketDataPoint) -> Self {
        Self {
            timestamp: point.timestamp,
            open: point.open,
            high: point.high,
            low: point.low,
            close: point.close,
            volume: point.volume,
        }
    }
}

impl From<Bar> for MarketDataPoint {
    fn from(bar: Bar) -> Self {
        Self {
            timestamp: bar.timestamp,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
        }
    }
}

/// Market data repository with layered storage
pub struct MarketRepository {
    hot_store: Box<dyn HotStoreOps>,
    manifest_store: Box<dyn ManifestStoreOps>,
    parquet_reader: Box<dyn ParquetReaderOps>,
}

impl MarketRepository {
    pub fn new(pool: PgPool, archive_config: ArchiveConfig) -> Self {
        Self {
            hot_store: Box::new(HotStore::new(pool.clone())),
            manifest_store: Box::new(ManifestStore::new(pool)),
            parquet_reader: Box::new(ParquetReader::new(archive_config)),
        }
    }

    #[cfg(test)]
    fn new_with_deps(
        hot_store: Box<dyn HotStoreOps>,
        manifest_store: Box<dyn ManifestStoreOps>,
        parquet_reader: Box<dyn ParquetReaderOps>,
    ) -> Self {
        Self {
            hot_store,
            manifest_store,
            parquet_reader,
        }
    }

    /// Load bars for a time range with layered strategy
    /// 
    /// Strategy:
    /// 1. Query hot store (PostgreSQL)
    /// 2. Calculate gaps
    /// 3. Fill gaps from Parquet archive
    /// 4. Merge and deduplicate results
    pub async fn load_bars_range(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>> {
        info!(
            provider,
            exchange,
            symbol,
            timeframe,
            start = %start,
            end = %end,
            "Loading bars with layered strategy"
        );

        // Step 1: Query hot store
        let hot_bars = self
            .hot_store
            .load_bars(provider, exchange, symbol, timeframe, start, end)
            .await
            .context("Failed to load hot bars")?;

        debug!(hot_count = hot_bars.len(), "Loaded bars from hot store");

        // Step 2: Calculate gaps
        let gaps = GapCalculator::calculate_gaps(start, end, &hot_bars);

        if gaps.is_empty() {
            info!("No gaps found, returning hot data only");
            return Ok(hot_bars);
        }

        info!(gap_count = gaps.len(), "Found gaps, querying Parquet archive");

        // Step 3: Fill gaps from Parquet
        let mut archive_bars = Vec::new();
        for gap in &gaps {
            match self
                .load_from_parquet(provider, exchange, symbol, timeframe, gap.start, gap.end)
                .await
            {
                Ok(mut bars) => {
                    debug!(
                        gap_start = %gap.start,
                        gap_end = %gap.end,
                        filled = bars.len(),
                        "Filled gap from Parquet"
                    );
                    archive_bars.append(&mut bars);
                }
                Err(e) => {
                    warn!(
                        gap_start = %gap.start,
                        gap_end = %gap.end,
                        error = %e,
                        "Failed to fill gap from Parquet, gap remains"
                    );
                }
            }
        }

        // Step 4: Merge and deduplicate
        let merged = Self::merge_and_deduplicate(hot_bars, archive_bars);

        info!(
            total = merged.len(),
            "Merged hot and archive data"
        );

        Ok(merged)
    }

    /// Load bars from Parquet archive
    async fn load_from_parquet(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>> {
        // Find partitions covering the time range
        let partitions = self
            .manifest_store
            .find_partitions(provider, exchange, symbol, timeframe, start, end)
            .await
            .context("Failed to find partitions in manifest")?;

        if partitions.is_empty() {
            debug!("No partitions found in manifest for range");
            return Ok(Vec::new());
        }

        debug!(partition_count = partitions.len(), "Found partitions in manifest");

        // Read from each partition
        let mut all_bars = Vec::new();
        for partition in partitions {
            let key = partition.to_key();
            
            match self.parquet_reader.read_range(&key, start, end).await {
                Ok(points) => {
                    let bars: Vec<Bar> = points.into_iter().map(Bar::from).collect();
                    debug!(
                        partition = ?key,
                        bars = bars.len(),
                        "Read bars from Parquet partition"
                    );
                    all_bars.extend(bars);
                }
                Err(e) => {
                    warn!(
                        partition = ?key,
                        error = %e,
                        "Failed to read Parquet partition, skipping"
                    );
                }
            }
        }

        Ok(all_bars)
    }

    /// Merge hot and archive bars, deduplicate by timestamp
    fn merge_and_deduplicate(mut hot: Vec<Bar>, mut archive: Vec<Bar>) -> Vec<Bar> {
        // Combine all bars
        hot.append(&mut archive);

        // Sort by timestamp
        hot.sort_by_key(|b| b.timestamp);

        // Deduplicate by timestamp (keep first occurrence)
        let mut result = Vec::new();
        let mut last_ts: Option<DateTime<Utc>> = None;

        for bar in hot {
            if last_ts != Some(bar.timestamp) {
                last_ts = Some(bar.timestamp);
                result.push(bar);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::{Arc, Mutex};

    fn create_test_bar(timestamp: DateTime<Utc>) -> Bar {
        Bar {
            timestamp,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 103.0,
            volume: 1000.0,
        }
    }

    #[test]
    fn merge_and_deduplicate_removes_duplicates() {
        let ts1 = Utc.with_ymd_and_hms(2024, 3, 17, 9, 30, 0).unwrap();
        let ts2 = Utc.with_ymd_and_hms(2024, 3, 17, 10, 30, 0).unwrap();

        let hot = vec![create_test_bar(ts1), create_test_bar(ts2)];
        let archive = vec![create_test_bar(ts1)]; // duplicate

        let merged = MarketRepository::merge_and_deduplicate(hot, archive);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].timestamp, ts1);
        assert_eq!(merged[1].timestamp, ts2);
    }

    #[test]
    fn merge_and_deduplicate_sorts_by_timestamp() {
        let ts1 = Utc.with_ymd_and_hms(2024, 3, 17, 9, 30, 0).unwrap();
        let ts2 = Utc.with_ymd_and_hms(2024, 3, 17, 10, 30, 0).unwrap();

        let hot = vec![create_test_bar(ts2)];
        let archive = vec![create_test_bar(ts1)];

        let merged = MarketRepository::merge_and_deduplicate(hot, archive);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].timestamp, ts1);
        assert_eq!(merged[1].timestamp, ts2);
    }

    // Mock implementations for testing

    struct MockHotStore {
        bars: Vec<Bar>,
    }

    #[async_trait]
    impl HotStoreOps for MockHotStore {
        async fn load_bars(
            &self,
            _provider: &str,
            _exchange: &str,
            _symbol: &str,
            _timeframe: &str,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> Result<Vec<Bar>> {
            Ok(self.bars.clone())
        }
    }

    struct MockManifestStore {
        partitions: Vec<PartitionRecord>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl ManifestStoreOps for MockManifestStore {
        async fn find_partitions(
            &self,
            _provider: &str,
            _exchange: &str,
            _symbol: &str,
            _timeframe: &str,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> Result<Vec<PartitionRecord>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.partitions.clone())
        }
    }

    struct MockParquetReader {
        bars: Vec<Bar>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl ParquetReaderOps for MockParquetReader {
        async fn read_range(
            &self,
            _key: &PartitionKey,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> Result<Vec<MarketDataPoint>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.bars.iter().map(|b| b.clone().into()).collect())
        }
    }

    #[tokio::test]
    async fn load_bars_range_no_gap_does_not_query_parquet() {
        // Setup: hot store returns complete data covering the entire range
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 11, 0, 0).unwrap();

        let hot_bars = vec![
            create_test_bar(start),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap()),
            create_test_bar(end),
        ];

        let manifest_call_count = Arc::new(Mutex::new(0));
        let parquet_call_count = Arc::new(Mutex::new(0));

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![],
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: parquet_call_count.clone(),
            }),
        );

        // Execute
        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Verify: returns hot data only, Parquet not queried
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].timestamp, start);
        assert_eq!(*manifest_call_count.lock().unwrap(), 0, "Manifest should not be queried when no gaps");
        assert_eq!(*parquet_call_count.lock().unwrap(), 0, "Parquet should not be queried when no gaps");
    }

    #[tokio::test]
    async fn load_bars_range_with_gap_queries_parquet_and_merges() {
        // Setup: hot store returns partial data with a gap
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 15, 0, 0).unwrap();

        // Hot store only has data from 12:00 onwards (gap before)
        let hot_start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let hot_bars = vec![
            create_test_bar(hot_start),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap()),
        ];

        // Parquet has data for the gap period
        let archive_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap()),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap()),
        ];

        let manifest_call_count = Arc::new(Mutex::new(0));
        let parquet_call_count = Arc::new(Mutex::new(0));

        let _partition_key = PartitionKey::new(
            "akshare".to_string(),
            "SSE".to_string(),
            "000001".to_string(),
            "1d".to_string(),
            chrono::NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
        );

        let partition_record = PartitionRecord {
            id: 1,
            provider: "akshare".to_string(),
            exchange: "SSE".to_string(),
            symbol: "000001".to_string(),
            timeframe: "1d".to_string(),
            partition_date: chrono::NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
            file_path: "akshare/SSE/000001/1d/2024-03-17.parquet".to_string(),
            row_count: 2,
            min_timestamp: Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap(),
            max_timestamp: Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![partition_record],
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: archive_bars.clone(),
                call_count: parquet_call_count.clone(),
            }),
        );

        // Execute
        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Verify: returns merged data, Parquet was queried
        assert_eq!(result.len(), 4, "Should have 2 hot + 2 archive bars");
        assert_eq!(result[0].timestamp, Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap());
        assert_eq!(result[1].timestamp, Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap());
        assert_eq!(result[2].timestamp, hot_start);
        
        // Note: There are 2 gaps (prefix and suffix), so manifest is queried twice
        assert_eq!(*manifest_call_count.lock().unwrap(), 2, "Manifest should be queried twice (one per gap)");
        assert_eq!(*parquet_call_count.lock().unwrap(), 2, "Parquet should be queried twice (one per partition per gap)");
    }
}
