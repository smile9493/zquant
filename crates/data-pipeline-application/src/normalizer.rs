use async_trait::async_trait;
use data_pipeline_domain::{NormalizedData, RawData};
use serde_json::json;

#[async_trait]
pub trait Normalizer: Send + Sync {
    async fn normalize(&self, raw: RawData) -> anyhow::Result<NormalizedData>;
}

pub struct BasicNormalizer;

impl Default for BasicNormalizer {
    fn default() -> Self {
        Self
    }
}

impl BasicNormalizer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Normalizer for BasicNormalizer {
    #[tracing::instrument(skip(self, raw), fields(record_count))]
    async fn normalize(&self, raw: RawData) -> anyhow::Result<NormalizedData> {
        let records = raw
            .content
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        tracing::Span::current().record("record_count", records.len());

        Ok(NormalizedData {
            records,
            metadata: json!({"source": "normalized"}),
        })
    }
}
