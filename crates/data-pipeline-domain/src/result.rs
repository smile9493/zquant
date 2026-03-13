use serde::{Deserialize, Serialize};

use crate::quality::DqDecision;

pub type DatasetId = String;
pub type QuarantineId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistReceipt {
    pub storage_path: String,
    pub catalog_id: String,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub dataset_id: Option<DatasetId>,
    pub decision: DqDecision,
    pub quarantine_id: Option<QuarantineId>,
    pub persist_receipt: Option<PersistReceipt>,
}
