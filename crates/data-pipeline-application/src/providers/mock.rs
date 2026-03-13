use async_trait::async_trait;
use data_pipeline_domain::{
    Capability, DataProvider, DatasetRequest, FetchRequest, Market, RawData,
};
use serde_json::json;

pub struct MockProvider;

impl Default for MockProvider {
    fn default() -> Self {
        Self
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DataProvider for MockProvider {
    fn provider_name(&self) -> &str {
        "mock"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Ohlcv]
    }

    fn markets(&self) -> Vec<Market> {
        vec![Market::UsEquity]
    }

    fn priority(&self) -> u8 {
        100
    }

    fn supports_dataset_ids(&self) -> bool {
        false
    }

    async fn fetch(&self, _req: FetchRequest) -> anyhow::Result<RawData> {
        Ok(RawData {
            content: json!({
                "data": [
                    {"date": "2024-01-01", "open": 100.0, "high": 105.0, "low": 99.0, "close": 103.0, "volume": 1000000},
                    {"date": "2024-01-02", "open": 103.0, "high": 107.0, "low": 102.0, "close": 106.0, "volume": 1200000}
                ]
            }),
        })
    }

    async fn fetch_dataset(&self, _req: DatasetRequest) -> anyhow::Result<RawData> {
        self.fetch(FetchRequest {
            capability: Capability::Ohlcv,
            market: Market::UsEquity,
            params: json!({}),
        })
        .await
    }
}
