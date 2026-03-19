pub mod akshare;
pub mod mock;
pub mod pytdx;

pub use akshare::AkshareProvider;
pub use mock::MockProvider;
pub use pytdx::PytdxProvider;

use std::path::Path;
use std::sync::Arc;

use data_pipeline_domain::{Capability, DatasetRequest, Market, RawData};

use crate::python_runner::PythonRunner;

// ---------------------------------------------------------------------------
// Shared abstraction for Python-backed dataset providers
// ---------------------------------------------------------------------------

/// Strategy trait that captures the per-provider differences.
///
/// The shared template [`python_fetch_dataset`] handles the common flow:
/// dataset_id validation → symbol_scope check → time_range extraction →
/// JSON construction → runner invocation → result wrapping.
pub(crate) trait PythonDatasetConfig: Send + Sync {
    /// Human-readable provider name used in error messages.
    fn provider_name(&self) -> &str;

    /// The single dataset ID this provider supports.
    fn dataset_id(&self) -> &str;

    /// Resolve the Python script path for the dataset.
    fn script_path(&self) -> Box<dyn AsRef<Path> + Send>;

    /// Build provider-specific extra fields merged into the JSON input.
    /// Default returns an empty map (no extra fields).
    fn extra_input(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    fn capabilities(&self) -> Vec<Capability>;
    fn markets(&self) -> Vec<Market>;
    fn priority(&self) -> u8;
}

/// Shared fetch_dataset template that delegates differences to [`PythonDatasetConfig`].
pub(crate) async fn python_fetch_dataset(
    cfg: &dyn PythonDatasetConfig,
    runner: &Arc<dyn PythonRunner>,
    req: DatasetRequest,
) -> anyhow::Result<RawData> {
    // 1. dataset_id validation
    if req.dataset_id.as_deref() != Some(cfg.dataset_id()) {
        anyhow::bail!(
            "unsupported dataset_id for {}: {:?}",
            cfg.provider_name(),
            req.dataset_id
        );
    }

    // 2. symbol_scope: exactly one symbol required
    let symbol = req
        .symbol_scope
        .first()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "missing required symbol_scope for dataset {}",
                cfg.dataset_id()
            )
        })?
        .to_string();

    if req.symbol_scope.len() != 1 {
        anyhow::bail!(
            "{} currently supports exactly 1 symbol; got {}",
            cfg.provider_name(),
            req.symbol_scope.len()
        );
    }

    // 3. time_range extraction
    let (start_date, end_date) = match req.time_range {
        None => (None, None),
        Some(tr) => (
            Some(tr.start.format("%Y%m%d").to_string()),
            Some(tr.end.format("%Y%m%d").to_string()),
        ),
    };

    // 4. Build JSON input: base fields + provider-specific extras
    let mut input = serde_json::json!({
        "symbol": symbol,
        "start_date": start_date,
        "end_date": end_date,
    });
    let extras = cfg.extra_input();
    if let (Some(base), Some(ext)) = (input.as_object_mut(), extras.as_object()) {
        for (k, v) in ext {
            base.insert(k.clone(), v.clone());
        }
    }

    // 5. Run Python script
    let script = cfg.script_path();
    let value = runner
        .run_json((*script).as_ref(), input)
        .await
        .map_err(|e| anyhow::anyhow!("{} subprocess failed: {}", cfg.provider_name(), e))?;

    Ok(RawData { content: value })
}
