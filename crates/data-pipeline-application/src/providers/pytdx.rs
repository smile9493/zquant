use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use data_pipeline_domain::{Capability, DataProvider, DatasetRequest, FetchRequest, Market, RawData};

use crate::python_runner::{PythonRunner, SubprocessPythonRunner};

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

#[async_trait]
impl DataProvider for PytdxProvider {
    fn provider_name(&self) -> &str {
        Self::PROVIDER_NAME
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Ohlcv]
    }

    fn markets(&self) -> Vec<Market> {
        vec![Market::CnEquity]
    }

    fn priority(&self) -> u8 {
        // Lower priority than AkShare (50) — AkShare is more stable/documented
        40
    }

    fn supports_dataset_ids(&self) -> bool {
        true
    }

    async fn fetch(&self, _req: FetchRequest) -> anyhow::Result<RawData> {
        anyhow::bail!("PytdxProvider does not support generic fetch(); use fetch_dataset()");
    }

    async fn fetch_dataset(&self, req: DatasetRequest) -> anyhow::Result<RawData> {
        if req.dataset_id.as_deref() != Some(Self::DATASET_ID_CN_EQUITY_OHLCV_DAILY) {
            anyhow::bail!(
                "unsupported dataset_id for PytdxProvider: {:?}",
                req.dataset_id
            );
        }

        let symbol = req
            .symbol_scope
            .first()
            .ok_or_else(|| anyhow::anyhow!("missing required symbol_scope for dataset {}", Self::DATASET_ID_CN_EQUITY_OHLCV_DAILY))?
            .to_string();

        if req.symbol_scope.len() != 1 {
            anyhow::bail!(
                "PytdxProvider currently supports exactly 1 symbol; got {}",
                req.symbol_scope.len()
            );
        }

        let (start_date, end_date) = match req.time_range {
            None => (None, None),
            Some(tr) => (
                Some(tr.start.format("%Y%m%d").to_string()),
                Some(tr.end.format("%Y%m%d").to_string()),
            ),
        };

        let input = serde_json::json!({
            "symbol": symbol,
            "start_date": start_date,
            "end_date": end_date,
        });

        let value = self
            .runner
            .run_json(&Self::script_path_cn_equity_daily(), input)
            .await
            .map_err(|e| anyhow::anyhow!("pytdx subprocess failed: {}", e))?;

        Ok(RawData { content: value })
    }
}
