use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use data_pipeline_domain::{
    Capability, DataProvider, DatasetRequest, FetchRequest, Market, RawData,
};

use crate::python_runner::{PythonRunner, SubprocessPythonRunner};

use super::{python_fetch_dataset, PythonDatasetConfig};

pub struct PytdxProvider {
    runner: Arc<dyn PythonRunner>,
}

impl PytdxProvider {
    pub const PROVIDER_NAME: &'static str = "pytdx";
    pub const DATASET_ID_CN_EQUITY_OHLCV_DAILY: &'static str = "cn_equity.ohlcv.daily";

    pub fn new(runner: Arc<dyn PythonRunner>) -> Self {
        Self { runner }
    }

    pub fn new_subprocess() -> Self {
        Self::new(Arc::new(SubprocessPythonRunner::new()))
    }

    fn script_path_cn_equity_daily() -> PathBuf {
        // Allow override via environment variable for external plugin directory
        if let Ok(path) = std::env::var("ZQUANT_PYTDX_SCRIPT_CN_EQUITY_DAILY") {
            return PathBuf::from(path);
        }
        // Default: embedded script in python/ directory
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/python/pytdx_cn_equity_ohlcv_daily.py"
        ))
    }
}

impl PythonDatasetConfig for PytdxProvider {
    fn provider_name(&self) -> &str {
        Self::PROVIDER_NAME
    }

    fn dataset_id(&self) -> &str {
        Self::DATASET_ID_CN_EQUITY_OHLCV_DAILY
    }

    fn script_path(&self) -> Box<dyn AsRef<Path> + Send> {
        Box::new(Self::script_path_cn_equity_daily())
    }

    // No extra_input — pytdx uses default (empty)

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Ohlcv]
    }

    fn markets(&self) -> Vec<Market> {
        vec![Market::CnEquity]
    }

    fn priority(&self) -> u8 {
        40
    }
}

#[async_trait]
impl DataProvider for PytdxProvider {
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
        anyhow::bail!("PytdxProvider does not support generic fetch(); use fetch_dataset()");
    }

    async fn fetch_dataset(&self, req: DatasetRequest) -> anyhow::Result<RawData> {
        python_fetch_dataset(self, &self.runner, req).await
    }
}
