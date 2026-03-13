use async_trait::async_trait;

use crate::request::{DatasetRequest, FetchRequest};
use crate::types::{Capability, Market, RawData};

#[async_trait]
pub trait DataProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn capabilities(&self) -> Vec<Capability>;
    fn markets(&self) -> Vec<Market>;
    fn priority(&self) -> u8;
    fn supports_dataset_ids(&self) -> bool;

    async fn fetch(&self, req: FetchRequest) -> anyhow::Result<RawData>;
    async fn fetch_dataset(&self, req: DatasetRequest) -> anyhow::Result<RawData>;
}

pub struct ProviderMetadata {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub markets: Vec<Market>,
}
