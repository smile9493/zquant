//! Market data repository with layered storage strategy.
//!
//! Read flow: Hot PostgreSQL → Parquet archive → Remote provider
//! Write flow: PostgreSQL + Parquet (atomic) + Manifest update

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use data_pipeline_application::AkshareProvider;
use data_pipeline_domain::{Capability, DataProvider, DatasetRequest, Market, TimeRange};
use infra_parquet::{ArchiveConfig, MarketDataPoint, PartitionKey, ParquetReader, ParquetWriter};
use sqlx::PgPool;
use store_manifest::{ManifestStore, PartitionRecord};
use tracing::{debug, info, warn};

mod error;
mod gap;
mod hot_store;

pub use error::{classify_error, retry_on_transient, ErrorKind, RetryConfig};
pub use gap::{Gap, GapCalculator};
pub use hot_store::HotStore;

/// Routed remote provider that currently supports AkShare CN daily OHLCV.
struct RoutedRemoteProvider {
    akshare: AkshareProvider,
}

impl RoutedRemoteProvider {
    fn new() -> Self {
        Self {
            akshare: AkshareProvider::new_subprocess(),
        }
    }

    #[cfg(test)]
    fn with_akshare_provider(akshare: AkshareProvider) -> Self {
        Self { akshare }
    }

    async fn fetch_akshare_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>> {
        if !Self::is_daily_timeframe(timeframe) {
            anyhow::bail!(
                "AkShare only supports daily timeframe for now, got '{}'",
                timeframe
            );
        }

        let request = DatasetRequest {
            capability: Capability::Ohlcv,
            market: Market::CnEquity,
            dataset_id: Some(AkshareProvider::DATASET_ID_CN_EQUITY_OHLCV_DAILY.to_string()),
            symbol_scope: vec![symbol.to_string()],
            time_range: Some(TimeRange { start, end }),
            forced_provider: Some(AkshareProvider::PROVIDER_NAME.to_string()),
        };

        let raw = self
            .akshare
            .fetch_dataset(request)
            .await
            .context("AkShare fetch_dataset failed")?;

        Self::parse_akshare_bars(&raw.content)
    }

    fn is_daily_timeframe(timeframe: &str) -> bool {
        matches!(timeframe.to_ascii_lowercase().as_str(), "1d" | "d" | "day" | "daily")
    }

    fn parse_akshare_bars(payload: &serde_json::Value) -> Result<Vec<Bar>> {
        let records = payload
            .get("data")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("AkShare response missing 'data' array"))?;

        let mut bars = Vec::with_capacity(records.len());

        for (index, record) in records.iter().enumerate() {
            let timestamp = Self::parse_date_field(record, "date")
                .with_context(|| format!("invalid date at record index {}", index))?;
            let open = Self::parse_number_field(record, "open")
                .with_context(|| format!("invalid open at record index {}", index))?;
            let high = Self::parse_number_field(record, "high")
                .with_context(|| format!("invalid high at record index {}", index))?;
            let low = Self::parse_number_field(record, "low")
                .with_context(|| format!("invalid low at record index {}", index))?;
            let close = Self::parse_number_field(record, "close")
                .with_context(|| format!("invalid close at record index {}", index))?;
            let volume = Self::parse_number_field(record, "volume")
                .with_context(|| format!("invalid volume at record index {}", index))?;

            bars.push(Bar {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            });
        }

        bars.sort_by_key(|bar| bar.timestamp);
        Ok(bars)
    }

    fn parse_date_field(record: &serde_json::Value, field: &str) -> Result<DateTime<Utc>> {
        let date_str = record
            .get(field)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing or non-string date field '{}'", field))?;

        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y%m%d"))
            .with_context(|| format!("unable to parse date '{}' with supported formats", date_str))?;

        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid date value '{}'", date_str))?;

        Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
    }

    fn parse_number_field(record: &serde_json::Value, field: &str) -> Result<f64> {
        let value = record
            .get(field)
            .ok_or_else(|| anyhow::anyhow!("missing numeric field '{}'", field))?;

        if let Some(number) = value.as_f64() {
            return Ok(number);
        }

        if let Some(text) = value.as_str() {
            let parsed = text
                .parse::<f64>()
                .with_context(|| format!("failed to parse '{}' from '{}'", field, text))?;
            return Ok(parsed);
        }

        anyhow::bail!("field '{}' is neither number nor numeric string", field);
    }
}

#[async_trait]
impl ProviderOps for RoutedRemoteProvider {
    async fn fetch_bars(
        &self,
        provider: &str,
        _exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>> {
        if provider.eq_ignore_ascii_case(AkshareProvider::PROVIDER_NAME) {
            return self.fetch_akshare_bars(symbol, timeframe, start, end).await;
        }

        anyhow::bail!("unsupported remote provider '{}'", provider);
    }
}

#[async_trait]
impl HotStoreWriter for HotStore {
    async fn upsert_bars(
        &self,
        _provider: &str,
        _exchange: &str,
        _symbol: &str,
        _timeframe: &str,
        _bars: &[Bar],
    ) -> Result<usize> {
        // Placeholder: would insert/update bars in PostgreSQL
        Ok(0)
    }
}

/// Trait for provider operations (fetching remote data)
#[async_trait]
pub(crate) trait ProviderOps: Send + Sync {
    async fn fetch_bars(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Bar>>;
}

/// Trait for hot store write operations
#[async_trait]
pub(crate) trait HotStoreWriter: Send + Sync {
    async fn upsert_bars(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        bars: &[Bar],
    ) -> Result<usize>;
}

/// Trait for hot store operations (enables testing)
#[async_trait]
pub(crate) trait HotStoreOps: Send + Sync {
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
pub(crate) trait ManifestStoreOps: Send + Sync {
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
pub(crate) trait ParquetReaderOps: Send + Sync {
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
    hot_store_writer: Box<dyn HotStoreWriter>,
    manifest_store: Box<dyn ManifestStoreOps>,
    parquet_reader: Box<dyn ParquetReaderOps>,
    #[allow(dead_code)] // Reserved for future archive write-back path
    parquet_writer: ParquetWriter,
    provider: Box<dyn ProviderOps>,
    retry_config: RetryConfig,
}

impl MarketRepository {
    pub fn new(pool: PgPool, archive_config: ArchiveConfig) -> Self {
        Self {
            hot_store: Box::new(HotStore::new(pool.clone())),
            hot_store_writer: Box::new(HotStore::new(pool.clone())),
            manifest_store: Box::new(ManifestStore::new(pool)),
            parquet_reader: Box::new(ParquetReader::new(archive_config.clone())),
            parquet_writer: ParquetWriter::new(archive_config),
            provider: Box::new(RoutedRemoteProvider::new()),
            retry_config: RetryConfig::default(),
        }
    }

    #[allow(dead_code)] // Reserved for runtime provider injection
    pub(crate) fn with_provider(mut self, provider: Box<dyn ProviderOps>) -> Self {
        self.provider = provider;
        self
    }

    #[cfg(test)]
    fn new_with_deps(
        hot_store: Box<dyn HotStoreOps>,
        hot_store_writer: Box<dyn HotStoreWriter>,
        manifest_store: Box<dyn ManifestStoreOps>,
        parquet_reader: Box<dyn ParquetReaderOps>,
        parquet_writer: ParquetWriter,
        provider: Box<dyn ProviderOps>,
    ) -> Self {
        Self {
            hot_store,
            hot_store_writer,
            manifest_store,
            parquet_reader,
            parquet_writer,
            provider,
            retry_config: RetryConfig {
                max_retries: 2,
                base_delay_ms: 1,
                max_delay_ms: 5,
            },
        }
    }

    /// Load bars for a time range with layered strategy
    /// 
    /// Strategy:
    /// 1. Query hot store (PostgreSQL)
    /// 2. Calculate gaps
    /// 3. Fill gaps from Parquet archive
    /// 4. Merge hot + archive, recalculate remaining gaps
    /// 5. Fill remaining gaps from remote provider, write back to hot store
    /// 6. Final merge and deduplicate
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

        // Step 4: Merge hot + archive, then check for remaining gaps
        let after_parquet = Self::merge_and_deduplicate(hot_bars, archive_bars);

        // Step 5: Remote backfill – fetch remaining gaps from provider
        let remaining_gaps = GapCalculator::calculate_gaps(start, end, &after_parquet);
        let mut remote_bars = Vec::new();

        if !remaining_gaps.is_empty() {
            info!(
                remaining_gap_count = remaining_gaps.len(),
                "Gaps remain after Parquet, querying remote provider"
            );

            for gap in &remaining_gaps {
                let gap_start = gap.start;
                let gap_end = gap.end;

                let fetch_result = retry_on_transient(
                    &self.retry_config,
                    "provider.fetch_bars",
                    || self.provider.fetch_bars(provider, exchange, symbol, timeframe, gap_start, gap_end),
                )
                .await;

                match fetch_result {
                    Ok(fetched) if !fetched.is_empty() => {
                        debug!(
                            gap_start = %gap_start,
                            gap_end = %gap_end,
                            fetched_count = fetched.len(),
                            "Fetched bars from remote provider"
                        );

                        // Write back to hot store (best-effort)
                        if let Err(e) = self
                            .hot_store_writer
                            .upsert_bars(provider, exchange, symbol, timeframe, &fetched)
                            .await
                        {
                            warn!(
                                error = %e,
                                "Failed to write remote bars to hot store, continuing"
                            );
                        }

                        remote_bars.extend(fetched);
                    }
                    Ok(_) => {
                        debug!(
                            gap_start = %gap_start,
                            gap_end = %gap_end,
                            "Remote provider returned no data for gap"
                        );
                    }
                    Err(e) => {
                        let error_kind = classify_error(&e);
                        warn!(
                            gap_start = %gap_start,
                            gap_end = %gap_end,
                            error = %e,
                            error_kind = %error_kind,
                            "Failed to fetch from remote provider, gap remains"
                        );
                    }
                }
            }
        }

        // Step 6: Final merge
        let merged = Self::merge_and_deduplicate(after_parquet, remote_bars);

        info!(
            total = merged.len(),
            "Merged hot + archive + remote data"
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
            info!(
                provider,
                exchange,
                symbol,
                timeframe,
                start = %start,
                end = %end,
                "No partitions found in manifest for time range"
            );
            return Ok(Vec::new());
        }

        debug!(
            provider,
            exchange,
            symbol,
            timeframe,
            partition_count = partitions.len(),
            "Found partitions in manifest"
        );

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
                    let error_kind = classify_error(&e);
                    warn!(
                        partition = ?key,
                        error = %e,
                        error_kind = %error_kind,
                        error_source = ?e.source(),
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
    use data_pipeline_application::PythonRunner;
    use chrono::TimeZone;
    use std::path::Path;
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

    struct FakePythonRunner {
        response: serde_json::Value,
    }

    #[async_trait]
    impl PythonRunner for FakePythonRunner {
        async fn run_json(
            &self,
            _script_path: &Path,
            _input: serde_json::Value,
        ) -> Result<serde_json::Value> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn routed_remote_provider_parses_akshare_payload() {
        let fake = Arc::new(FakePythonRunner {
            response: serde_json::json!({
                "status": "success",
                "data": [
                    {
                        "date": "2024-03-17",
                        "open": 10.5,
                        "high": 11.0,
                        "low": 10.2,
                        "close": 10.8,
                        "volume": "1000000"
                    }
                ]
            }),
        });

        let provider = RoutedRemoteProvider::with_akshare_provider(AkshareProvider::new(fake));

        let start = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 31, 0, 0, 0).unwrap();
        let result = provider
            .fetch_bars("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].open, 10.5);
        assert_eq!(result[0].high, 11.0);
        assert_eq!(result[0].low, 10.2);
        assert_eq!(result[0].close, 10.8);
        assert_eq!(result[0].volume, 1_000_000.0);
        assert_eq!(
            result[0].timestamp,
            Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn routed_remote_provider_rejects_non_daily_timeframe() {
        let fake = Arc::new(FakePythonRunner {
            response: serde_json::json!({
                "status": "success",
                "data": []
            }),
        });

        let provider = RoutedRemoteProvider::with_akshare_provider(AkshareProvider::new(fake));

        let start = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 31, 0, 0, 0).unwrap();
        let result = provider
            .fetch_bars("akshare", "SSE", "000001", "1h", start, end)
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("daily timeframe"));
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

    struct MockHotStoreWriter {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl HotStoreWriter for MockHotStoreWriter {
        async fn upsert_bars(
            &self,
            _provider: &str,
            _exchange: &str,
            _symbol: &str,
            _timeframe: &str,
            _bars: &[Bar],
        ) -> Result<usize> {
            *self.call_count.lock().unwrap() += 1;
            Ok(0)
        }
    }

    struct MockProvider {
        bars: Vec<Bar>,
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl ProviderOps for MockProvider {
        async fn fetch_bars(
            &self,
            _provider: &str,
            _exchange: &str,
            _symbol: &str,
            _timeframe: &str,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> Result<Vec<Bar>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.bars.clone())
        }
    }

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
        should_fail: bool,
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
            
            if self.should_fail {
                anyhow::bail!("Simulated Parquet read failure");
            }
            
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
        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: writer_call_count.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![],
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: parquet_call_count.clone(),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: vec![],
                call_count: provider_call_count.clone(),
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
        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

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

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: writer_call_count.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![partition_record],
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: archive_bars.clone(),
                call_count: parquet_call_count.clone(),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: vec![],
                call_count: provider_call_count.clone(),
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

    #[tokio::test]
    async fn load_bars_range_with_empty_manifest_returns_hot_only() {
        // Setup: hot store returns partial data, but manifest has no partitions
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 15, 0, 0).unwrap();

        let hot_start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let hot_bars = vec![
            create_test_bar(hot_start),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap()),
        ];

        let manifest_call_count = Arc::new(Mutex::new(0));
        let parquet_call_count = Arc::new(Mutex::new(0));
        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: writer_call_count.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![], // Empty manifest
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: parquet_call_count.clone(),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: vec![],
                call_count: provider_call_count.clone(),
            }),
        );

        // Execute
        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Verify: returns hot data only, Parquet not queried due to empty manifest
        assert_eq!(result.len(), 2, "Should return only hot data");
        assert_eq!(result[0].timestamp, hot_start);
        assert_eq!(*manifest_call_count.lock().unwrap(), 2, "Manifest should be queried for gaps");
        assert_eq!(*parquet_call_count.lock().unwrap(), 0, "Parquet should not be queried when manifest is empty");
    }

    #[tokio::test]
    async fn load_bars_range_with_parquet_read_failure_returns_hot_only() {
        // Setup: hot store returns partial data, manifest has partitions, but Parquet read fails
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 15, 0, 0).unwrap();

        let hot_start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let hot_bars = vec![
            create_test_bar(hot_start),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap()),
        ];

        let manifest_call_count = Arc::new(Mutex::new(0));
        let parquet_call_count = Arc::new(Mutex::new(0));
        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

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

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: writer_call_count.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![partition_record],
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: parquet_call_count.clone(),
                should_fail: true, // Simulate Parquet read failure
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: vec![],
                call_count: provider_call_count.clone(),
            }),
        );

        // Execute
        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Verify: returns hot data only, Parquet read failed but didn't break the flow
        assert_eq!(result.len(), 2, "Should return only hot data when Parquet fails");
        assert_eq!(result[0].timestamp, hot_start);
        assert_eq!(*manifest_call_count.lock().unwrap(), 2, "Manifest should be queried for gaps");
        assert_eq!(*parquet_call_count.lock().unwrap(), 2, "Parquet should be attempted but failed");
    }

    #[tokio::test]
    async fn load_bars_range_with_remote_backfill() {
        // Setup: hot store has partial data, Parquet has nothing,
        // remote provider fills the gap and writes back to hot store.
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 15, 0, 0).unwrap();

        let hot_start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let hot_bars = vec![
            create_test_bar(hot_start),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap()),
        ];

        // Remote provider returns bars for the gap period
        let remote_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap()),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap()),
        ];

        let manifest_call_count = Arc::new(Mutex::new(0));
        let parquet_call_count = Arc::new(Mutex::new(0));
        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: writer_call_count.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![], // No Parquet partitions
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: parquet_call_count.clone(),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: remote_bars.clone(),
                call_count: provider_call_count.clone(),
            }),
        );

        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Verify: hot + remote merged (4 bars total)
        assert_eq!(result.len(), 4, "Should have 2 hot + 2 remote bars");
        assert_eq!(result[0].timestamp, Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap());
        assert_eq!(result[1].timestamp, Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap());
        assert_eq!(result[2].timestamp, hot_start);

        // Provider was called for remaining gaps
        assert!(*provider_call_count.lock().unwrap() > 0, "Provider should be called for remaining gaps");

        // Hot store writer was called to persist remote data
        assert!(*writer_call_count.lock().unwrap() > 0, "Hot store writer should be called to persist remote bars");
    }

    #[tokio::test]
    async fn load_bars_range_remote_backfill_idempotent() {
        // Setup: hot store already has full data, provider should NOT be called
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 11, 0, 0).unwrap();

        let hot_bars = vec![
            create_test_bar(start),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap()),
            create_test_bar(end),
        ];

        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: writer_call_count.clone(),
            }),
            Box::new(MockManifestStore {
                partitions: vec![],
                call_count: Arc::new(Mutex::new(0)),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: vec![create_test_bar(start)], // Would return data if called
                call_count: provider_call_count.clone(),
            }),
        );

        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        assert_eq!(result.len(), 3, "Should return all hot bars");
        assert_eq!(*provider_call_count.lock().unwrap(), 0, "Provider should NOT be called when no gaps");
        assert_eq!(*writer_call_count.lock().unwrap(), 0, "Writer should NOT be called when no gaps");
    }

    #[tokio::test]
    async fn load_bars_range_remote_provider_failure_returns_partial() {
        // Setup: hot store partial, Parquet empty, provider fails → still returns hot data
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 15, 0, 0).unwrap();

        let hot_start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let hot_bars = vec![
            create_test_bar(hot_start),
        ];

        let provider_call_count = Arc::new(Mutex::new(0));

        // Provider that always fails
        struct FailingProvider {
            call_count: Arc<Mutex<usize>>,
        }

        #[async_trait]
        impl ProviderOps for FailingProvider {
            async fn fetch_bars(
                &self,
                _provider: &str,
                _exchange: &str,
                _symbol: &str,
                _timeframe: &str,
                _start: DateTime<Utc>,
                _end: DateTime<Utc>,
            ) -> Result<Vec<Bar>> {
                *self.call_count.lock().unwrap() += 1;
                anyhow::bail!("Simulated provider failure")
            }
        }

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore {
                bars: hot_bars.clone(),
            }),
            Box::new(MockHotStoreWriter {
                call_count: Arc::new(Mutex::new(0)),
            }),
            Box::new(MockManifestStore {
                partitions: vec![],
                call_count: Arc::new(Mutex::new(0)),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(FailingProvider {
                call_count: provider_call_count.clone(),
            }),
        );

        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Verify: returns hot data only, provider was attempted but failed gracefully
        assert_eq!(result.len(), 1, "Should return only hot data when provider fails");
        assert!(*provider_call_count.lock().unwrap() > 0, "Provider should be attempted");
    }

    #[tokio::test]
    async fn load_bars_range_full_three_layer_integration() {
        // Full integration: hot partial → Parquet fills some → remote fills rest
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 16, 0, 0).unwrap();

        // Hot store: only 12:00-13:00
        let hot_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap()),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap()),
        ];

        // Parquet: covers 9:00-10:00 (fills prefix gap partially)
        let parquet_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap()),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 10, 0, 0).unwrap()),
        ];

        // Remote provider: fills remaining gaps
        let remote_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 11, 0, 0).unwrap()),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 14, 0, 0).unwrap()),
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 15, 0, 0).unwrap()),
        ];

        let manifest_call_count = Arc::new(Mutex::new(0));
        let parquet_call_count = Arc::new(Mutex::new(0));
        let writer_call_count = Arc::new(Mutex::new(0));
        let provider_call_count = Arc::new(Mutex::new(0));

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

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore { bars: hot_bars }),
            Box::new(MockHotStoreWriter { call_count: writer_call_count.clone() }),
            Box::new(MockManifestStore {
                partitions: vec![partition_record],
                call_count: manifest_call_count.clone(),
            }),
            Box::new(MockParquetReader {
                bars: parquet_bars,
                call_count: parquet_call_count.clone(),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: remote_bars,
                call_count: provider_call_count.clone(),
            }),
        );

        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // All three layers contributed data
        assert!(result.len() >= 5, "Should have bars from all three layers, got {}", result.len());
        assert!(*manifest_call_count.lock().unwrap() > 0, "Manifest should be queried");
        assert!(*provider_call_count.lock().unwrap() > 0, "Provider should be called for remaining gaps");
        assert!(*writer_call_count.lock().unwrap() > 0, "Writer should persist remote bars");

        // Verify sorted order
        for i in 1..result.len() {
            assert!(result[i].timestamp >= result[i - 1].timestamp, "Results should be sorted by timestamp");
        }
    }

    #[tokio::test]
    async fn load_bars_range_transient_provider_retried_then_succeeds() {
        // Provider fails with transient error twice, then succeeds on third attempt
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();

        let hot_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 11, 0, 0).unwrap()),
        ];

        let call_count = Arc::new(Mutex::new(0u32));

        struct TransientThenSuccessProvider {
            call_count: Arc<Mutex<u32>>,
        }

        #[async_trait]
        impl ProviderOps for TransientThenSuccessProvider {
            async fn fetch_bars(
                &self,
                _provider: &str,
                _exchange: &str,
                _symbol: &str,
                _timeframe: &str,
                _start: DateTime<Utc>,
                _end: DateTime<Utc>,
            ) -> Result<Vec<Bar>> {
                let mut count = self.call_count.lock().unwrap();
                *count += 1;
                let current = *count;
                drop(count);

                if current <= 2 {
                    anyhow::bail!("connection timed out"); // transient
                }
                Ok(vec![Bar {
                    timestamp: Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap(),
                    open: 100.0, high: 105.0, low: 99.0, close: 103.0, volume: 1000.0,
                }])
            }
        }

        let writer_call_count = Arc::new(Mutex::new(0));

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore { bars: hot_bars }),
            Box::new(MockHotStoreWriter { call_count: writer_call_count.clone() }),
            Box::new(MockManifestStore {
                partitions: vec![],
                call_count: Arc::new(Mutex::new(0)),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(TransientThenSuccessProvider {
                call_count: call_count.clone(),
            }),
        );

        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Provider retried and eventually succeeded
        assert!(result.len() >= 2, "Should have hot + remote bars after retry");
        assert!(*call_count.lock().unwrap() >= 3, "Provider should be called at least 3 times (2 failures + 1 success)");
        assert!(*writer_call_count.lock().unwrap() > 0, "Writer should be called after successful retry");
    }

    #[tokio::test]
    async fn load_bars_range_hot_store_writer_failure_still_returns_data() {
        // Hot store writer fails, but remote data should still be in the result
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();

        let hot_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 11, 0, 0).unwrap()),
        ];

        let remote_bars = vec![
            create_test_bar(Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap()),
        ];

        struct FailingHotStoreWriter;

        #[async_trait]
        impl HotStoreWriter for FailingHotStoreWriter {
            async fn upsert_bars(
                &self,
                _provider: &str,
                _exchange: &str,
                _symbol: &str,
                _timeframe: &str,
                _bars: &[Bar],
            ) -> Result<usize> {
                anyhow::bail!("Database connection refused")
            }
        }

        let tmp_dir = tempfile::TempDir::new().unwrap();
        let archive_config = ArchiveConfig::new(tmp_dir.path().to_path_buf());

        let repo = MarketRepository::new_with_deps(
            Box::new(MockHotStore { bars: hot_bars }),
            Box::new(FailingHotStoreWriter),
            Box::new(MockManifestStore {
                partitions: vec![],
                call_count: Arc::new(Mutex::new(0)),
            }),
            Box::new(MockParquetReader {
                bars: vec![],
                call_count: Arc::new(Mutex::new(0)),
                should_fail: false,
            }),
            ParquetWriter::new(archive_config),
            Box::new(MockProvider {
                bars: remote_bars,
                call_count: Arc::new(Mutex::new(0)),
            }),
        );

        let result = repo
            .load_bars_range("akshare", "SSE", "000001", "1d", start, end)
            .await
            .unwrap();

        // Remote data still included even though writeback failed
        assert_eq!(result.len(), 2, "Should have hot + remote bars despite writer failure");
        assert_eq!(result[0].timestamp, Utc.with_ymd_and_hms(2024, 3, 17, 9, 0, 0).unwrap());
    }
}
