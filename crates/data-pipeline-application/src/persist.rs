use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_pipeline_domain::{
    Capability, Market, NormalizedData, PersistReceipt, QuarantineId, QuarantineRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub dataset_id: String,
    pub provider: String,
    pub capability: Capability,
    pub market: Market,
    pub available_at: Option<DateTime<Utc>>,
    pub point_in_time: Option<DateTime<Utc>>,
    pub version: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub dataset_id: String,
    pub metadata: DatasetMetadata,
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
        record: &QuarantineRecord,
    ) -> anyhow::Result<QuarantineId>;
}

pub struct InMemoryPersistWriter {
    datasets: Arc<Mutex<HashMap<String, NormalizedData>>>,
    catalogs: Arc<Mutex<HashMap<String, CatalogEntry>>>,
    quarantines: Arc<Mutex<HashMap<String, QuarantineRecord>>>,
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
    #[tracing::instrument(skip(self, data), fields(dataset_id = %metadata.dataset_id, row_count))]
    async fn write_dataset(
        &self,
        data: &NormalizedData,
        metadata: &DatasetMetadata,
    ) -> anyhow::Result<PersistReceipt> {
        let storage_path = format!("memory://{}", metadata.dataset_id);
        let row_count = data.records.len();

        tracing::Span::current().record("row_count", row_count);

        self.datasets
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock on datasets: {}", e))?
            .insert(metadata.dataset_id.clone(), data.clone());

        Ok(PersistReceipt {
            storage_path,
            catalog_id: metadata.dataset_id.clone(),
            row_count,
        })
    }

    #[tracing::instrument(skip(self, catalog), fields(catalog_id = %catalog.dataset_id))]
    async fn write_catalog(&self, catalog: &CatalogEntry) -> anyhow::Result<String> {
        let catalog_id = catalog.dataset_id.clone();
        self.catalogs
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock on catalogs: {}", e))?
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

    #[tracing::instrument(skip(self, record), fields(quarantine_id))]
    async fn write_quarantine(
        &self,
        record: &QuarantineRecord,
    ) -> anyhow::Result<QuarantineId> {
        let quarantine_id = format!("quarantine_{}", uuid::Uuid::new_v4());
        tracing::Span::current().record("quarantine_id", quarantine_id.as_str());

        self.quarantines
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock on quarantines: {}", e))?
            .insert(quarantine_id.clone(), record.clone());
        Ok(quarantine_id)
    }
}

fn to_dataset_key(dataset_id: &str) -> String {
    let slug: String = dataset_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .take(64)
        .collect();

    let mut hasher = DefaultHasher::new();
    dataset_id.hash(&mut hasher);
    let hash = hasher.finish();

    if slug.is_empty() {
        format!("{:x}", hash)
    } else {
        format!("{}_{:x}", slug, hash)
    }
}

pub struct FilePersistWriter {
    base_dir: PathBuf,
}

impl FilePersistWriter {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn datasets_dir(&self) -> PathBuf {
        self.base_dir.join("datasets")
    }

    fn catalogs_dir(&self) -> PathBuf {
        self.base_dir.join("catalogs")
    }

    fn quarantine_dir(&self) -> PathBuf {
        self.base_dir.join("quarantine")
    }
}

#[async_trait]
impl PersistWriter for FilePersistWriter {
    #[tracing::instrument(skip(self, data), fields(dataset_id = %metadata.dataset_id, row_count))]
    async fn write_dataset(
        &self,
        data: &NormalizedData,
        metadata: &DatasetMetadata,
    ) -> anyhow::Result<PersistReceipt> {
        let row_count = data.records.len();
        tracing::Span::current().record("row_count", row_count);

        let dataset_key = to_dataset_key(&metadata.dataset_id);
        let dataset_dir = self.datasets_dir().join(&dataset_key);
        tokio::fs::create_dir_all(&dataset_dir).await?;

        let timestamp_ms = Utc::now().timestamp_millis();
        let rand_suffix: u32 = rand::random();
        let filename = format!("{}_{:08x}.jsonl", timestamp_ms, rand_suffix);
        let file_path = dataset_dir.join(&filename);

        let mut lines = Vec::new();
        for record in &data.records {
            lines.push(serde_json::to_string(record)?);
        }
        let content = lines.join("\n");

        let temp_path = dataset_dir.join(format!(".{}.tmp", filename));
        tokio::fs::write(&temp_path, content).await?;
        tokio::fs::rename(&temp_path, &file_path).await?;

        Ok(PersistReceipt {
            storage_path: file_path.to_string_lossy().to_string(),
            catalog_id: metadata.dataset_id.clone(),
            row_count,
        })
    }

    #[tracing::instrument(skip(self, catalog), fields(catalog_id = %catalog.dataset_id))]
    async fn write_catalog(&self, catalog: &CatalogEntry) -> anyhow::Result<String> {
        let catalog_id = catalog.dataset_id.clone();
        let dataset_key = to_dataset_key(&catalog_id);
        let catalog_dir = self.catalogs_dir();
        tokio::fs::create_dir_all(&catalog_dir).await?;

        let filename = format!("{}.json", dataset_key);
        let file_path = catalog_dir.join(&filename);
        let content = serde_json::to_string_pretty(catalog)?;

        let temp_path = catalog_dir.join(format!(".{}.tmp", filename));
        tokio::fs::write(&temp_path, content).await?;
        tokio::fs::rename(&temp_path, &file_path).await?;

        Ok(catalog_id)
    }

    #[tracing::instrument(skip(self, record), fields(quarantine_id))]
    async fn write_quarantine(
        &self,
        record: &QuarantineRecord,
    ) -> anyhow::Result<QuarantineId> {
        let quarantine_id = format!("quarantine_{}", uuid::Uuid::new_v4());
        tracing::Span::current().record("quarantine_id", quarantine_id.as_str());

        let quarantine_dir = self.quarantine_dir();
        tokio::fs::create_dir_all(&quarantine_dir).await?;

        let filename = format!("{}.json", quarantine_id);
        let file_path = quarantine_dir.join(&filename);
        let content = serde_json::to_string_pretty(record)?;

        // Atomic write: temp file + rename
        let temp_path = quarantine_dir.join(format!(".{}.tmp", filename));
        tokio::fs::write(&temp_path, content).await?;
        tokio::fs::rename(&temp_path, &file_path).await?;

        Ok(quarantine_id)
    }
}