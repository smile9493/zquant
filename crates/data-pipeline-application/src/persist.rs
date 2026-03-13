use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_pipeline_domain::{
    Capability, Market, NormalizedData, PersistReceipt, QuarantineId, RawData,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct DatasetMetadata {
    pub dataset_id: String,
    pub provider: String,
    pub capability: Capability,
    pub market: Market,
    pub available_at: Option<DateTime<Utc>>,
    pub point_in_time: Option<DateTime<Utc>>,
    pub version: u64,
}

pub struct CatalogEntry {
    pub dataset_id: String,
    pub metadata: DatasetMetadata,
}

pub struct QuarantineReason {
    pub reasons: Vec<String>,
}

#[async_trait]
pub trait PersistWriter: Send + Sync {
    async fn write_dataset(
        &self,
        data: &NormalizedData,
        metadata: &DatasetMetadata,
    ) -> anyhow::Result<PersistReceipt>;

    async fn write_catalog(&self, catalog: &CatalogEntry) -> anyhow::Result<String>;

    async fn write_quarantine(
        &self,
        data: &RawData,
        reason: &QuarantineReason,
    ) -> anyhow::Result<QuarantineId>;
}

pub struct InMemoryPersistWriter {
    datasets: Arc<Mutex<HashMap<String, NormalizedData>>>,
    catalogs: Arc<Mutex<HashMap<String, CatalogEntry>>>,
    quarantines: Arc<Mutex<HashMap<String, RawData>>>,
}

impl Default for InMemoryPersistWriter {
    fn default() -> Self {
        Self {
            datasets: Arc::new(Mutex::new(HashMap::new())),
            catalogs: Arc::new(Mutex::new(HashMap::new())),
            quarantines: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl InMemoryPersistWriter {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PersistWriter for InMemoryPersistWriter {
    async fn write_dataset(
        &self,
        data: &NormalizedData,
        metadata: &DatasetMetadata,
    ) -> anyhow::Result<PersistReceipt> {
        let storage_path = format!("memory://{}", metadata.dataset_id);
        let row_count = data.records.len();

        self.datasets
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock on datasets: {}", e))?
            .insert(metadata.dataset_id.clone(), data.clone());

        Ok(PersistReceipt {
            storage_path,
            catalog_id: metadata.dataset_id.clone(),
            row_count,
        })
    }

    async fn write_catalog(&self, catalog: &CatalogEntry) -> anyhow::Result<String> {
        let catalog_id = catalog.dataset_id.clone();
        self.catalogs
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock on catalogs: {}", e))?
            .insert(catalog_id.clone(), CatalogEntry {
                dataset_id: catalog.dataset_id.clone(),
                metadata: DatasetMetadata {
                    dataset_id: catalog.metadata.dataset_id.clone(),
                    provider: catalog.metadata.provider.clone(),
                    capability: catalog.metadata.capability,
                    market: catalog.metadata.market,
                    available_at: catalog.metadata.available_at,
                    point_in_time: catalog.metadata.point_in_time,
                    version: catalog.metadata.version,
                },
            });
        Ok(catalog_id)
    }

    async fn write_quarantine(
        &self,
        data: &RawData,
        _reason: &QuarantineReason,
    ) -> anyhow::Result<QuarantineId> {
        let quarantine_id = format!("quarantine_{}", uuid::Uuid::new_v4());
        self.quarantines
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock on quarantines: {}", e))?
            .insert(quarantine_id.clone(), data.clone());
        Ok(quarantine_id)
    }
}
