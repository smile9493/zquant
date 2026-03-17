//! Partition manifest store for PostgreSQL.
//!
//! Provides CRUD operations for partition metadata tracking.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use infra_parquet::{PartitionKey, PartitionMetadata};
use sqlx::PgPool;
use std::path::PathBuf;
use tracing::{debug, info};

/// Partition manifest store
#[derive(Clone)]
pub struct ManifestStore {
    pool: PgPool,
}

impl ManifestStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Register a new partition in the manifest
    pub async fn register(&self, metadata: &PartitionMetadata) -> Result<i64> {
        let key = &metadata.partition_key;
        
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO partition_manifest 
                (provider, exchange, symbol, timeframe, partition_date, 
                 file_path, row_count, min_timestamp, max_timestamp, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            ON CONFLICT (provider, exchange, symbol, timeframe, partition_date)
            DO UPDATE SET
                file_path = EXCLUDED.file_path,
                row_count = EXCLUDED.row_count,
                min_timestamp = EXCLUDED.min_timestamp,
                max_timestamp = EXCLUDED.max_timestamp,
                updated_at = EXCLUDED.updated_at
            RETURNING id
            "#,
        )
        .bind(&key.provider)
        .bind(&key.exchange)
        .bind(&key.symbol)
        .bind(&key.timeframe)
        .bind(key.date)
        .bind(metadata.file_path.to_string_lossy().as_ref())
        .bind(metadata.row_count)
        .bind(metadata.min_timestamp)
        .bind(metadata.max_timestamp)
        .bind(metadata.created_at)
        .fetch_one(&self.pool)
        .await
        .context("Failed to register partition in manifest")?;

        info!(
            partition = ?key,
            id,
            "Partition registered in manifest"
        );

        Ok(id)
    }

    /// Find partitions covering a time range
    pub async fn find_partitions(
        &self,
        provider: &str,
        exchange: &str,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PartitionRecord>> {
        let records = sqlx::query_as::<_, PartitionRecord>(
            r#"
            SELECT 
                id, provider, exchange, symbol, timeframe, partition_date,
                file_path, row_count, min_timestamp, max_timestamp,
                created_at, updated_at
            FROM partition_manifest
            WHERE provider = $1
              AND exchange = $2
              AND symbol = $3
              AND timeframe = $4
              AND max_timestamp >= $5
              AND min_timestamp < $6
            ORDER BY partition_date ASC
            "#,
        )
        .bind(provider)
        .bind(exchange)
        .bind(symbol)
        .bind(timeframe)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .context("Failed to find partitions")?;

        debug!(
            provider,
            exchange,
            symbol,
            timeframe,
            start = %start,
            end = %end,
            found = records.len(),
            "Found partitions for time range"
        );

        Ok(records)
    }

    /// Check if a partition exists
    pub async fn exists(&self, key: &PartitionKey) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM partition_manifest
                WHERE provider = $1
                  AND exchange = $2
                  AND symbol = $3
                  AND timeframe = $4
                  AND partition_date = $5
            )
            "#,
        )
        .bind(&key.provider)
        .bind(&key.exchange)
        .bind(&key.symbol)
        .bind(&key.timeframe)
        .bind(key.date)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check partition existence")?;

        Ok(exists)
    }

    /// Delete a partition from manifest
    pub async fn delete(&self, key: &PartitionKey) -> Result<bool> {
        let rows_affected = sqlx::query(
            r#"
            DELETE FROM partition_manifest
            WHERE provider = $1
              AND exchange = $2
              AND symbol = $3
              AND timeframe = $4
              AND partition_date = $5
            "#,
        )
        .bind(&key.provider)
        .bind(&key.exchange)
        .bind(&key.symbol)
        .bind(&key.timeframe)
        .bind(key.date)
        .execute(&self.pool)
        .await
        .context("Failed to delete partition from manifest")?
        .rows_affected();

        if rows_affected > 0 {
            info!(partition = ?key, "Partition deleted from manifest");
            Ok(true)
        } else {
            debug!(partition = ?key, "Partition not found in manifest");
            Ok(false)
        }
    }
}

/// Partition record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PartitionRecord {
    pub id: i64,
    pub provider: String,
    pub exchange: String,
    pub symbol: String,
    pub timeframe: String,
    pub partition_date: NaiveDate,
    pub file_path: String,
    pub row_count: i64,
    pub min_timestamp: DateTime<Utc>,
    pub max_timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PartitionRecord {
    /// Convert to PartitionKey
    pub fn to_key(&self) -> PartitionKey {
        PartitionKey::new(
            self.provider.clone(),
            self.exchange.clone(),
            self.symbol.clone(),
            self.timeframe.clone(),
            self.partition_date,
        )
    }

    /// Get file path as PathBuf
    pub fn file_path_buf(&self) -> PathBuf {
        PathBuf::from(&self.file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_record_to_key() {
        use chrono::NaiveDate;
        
        let record = PartitionRecord {
            id: 1,
            provider: "akshare".to_string(),
            exchange: "SSE".to_string(),
            symbol: "600000".to_string(),
            timeframe: "1d".to_string(),
            partition_date: NaiveDate::from_ymd_opt(2024, 3, 17).unwrap(),
            file_path: "akshare/SSE/600000/1d/2024-03-17.parquet".to_string(),
            row_count: 100,
            min_timestamp: Utc::now(),
            max_timestamp: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let key = record.to_key();
        assert_eq!(key.provider, "akshare");
        assert_eq!(key.symbol, "600000");
    }
}
