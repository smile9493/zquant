use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use data_pipeline_domain::{
    Capability, DataProvider, DatasetRequest, FetchRequest, Market, RawData,
};

use crate::python_runner::{PythonRunner, SubprocessPythonRunner};

use super::{python_fetch_dataset, PythonDatasetConfig};

pub struct AkshareProvider {
    runner: Arc<dyn PythonRunner>,
}

impl AkshareProvider {
    pub const PROVIDER_NAME: &'static str = "akshare";
    pub const DATASET_ID_CN_EQUITY_OHLCV_DAILY: &'static str = "cn_equity.ohlcv.daily";

    pub fn new(runner: Arc<dyn PythonRunner>) -> Self {
        Self { runner }
    }

    pub fn new_subprocess() -> Self {
        Self::new(Arc::new(SubprocessPythonRunner::new()))
    }

    fn script_path_cn_equity_daily() -> &'static Path {
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/python/akshare_cn_equity_ohlcv_daily.py"
        ))
    }
}

impl PythonDatasetConfig for AkshareProvider {
    fn provider_name(&self) -> &str {
        Self::PROVIDER_NAME
    }

    fn dataset_id(&self) -> &str {
        Self::DATASET_ID_CN_EQUITY_OHLCV_DAILY
    }

    fn script_path(&self) -> Box<dyn AsRef<Path> + Send> {
        Box::new(Self::script_path_cn_equity_daily().to_path_buf())
    }

    fn extra_input(&self) -> serde_json::Value {
        serde_json::json!({ "adjust": "" })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Ohlcv]
    }

    fn markets(&self) -> Vec<Market> {
        vec![Market::CnEquity]
    }

    fn priority(&self) -> u8 {
        50
    }
}

#[async_trait]
impl DataProvider for AkshareProvider {
    fn provider_name(&self) -> &str {
        Self::PROVIDER_NAME
    }

    fn capabilities(&self) -> Vec<Capability> {
        PythonDatasetConfig::capabilities(self)
    }

    fn markets(&self) -> Vec<Market> {
        PythonDatasetConfig::markets(self)
    }

    fn priority(&self) -> u8 {
        PythonDatasetConfig::priority(self)
    }

    fn supports_dataset_ids(&self) -> bool {
        true
    }

    async fn fetch(&self, _req: FetchRequest) -> anyhow::Result<RawData> {
        anyhow::bail!("AkshareProvider does not support generic fetch(); use fetch_dataset()");
    }

    async fn fetch_dataset(&self, req: DatasetRequest) -> anyhow::Result<RawData> {
        python_fetch_dataset(self, &self.runner, req).await
    }
}
