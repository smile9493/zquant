use std::sync::Arc;

use async_trait::async_trait;
use data_pipeline_domain::{DataProvider, DatasetRequest};

#[async_trait]
pub trait RouteResolver: Send + Sync {
    async fn resolve(
        &self,
        req: &DatasetRequest,
        candidates: Vec<Arc<dyn DataProvider>>,
    ) -> anyhow::Result<Arc<dyn DataProvider>>;
}

pub struct PriorityRouteResolver;

impl Default for PriorityRouteResolver {
    fn default() -> Self {
        Self
    }
}

impl PriorityRouteResolver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RouteResolver for PriorityRouteResolver {
    async fn resolve(
        &self,
        req: &DatasetRequest,
        mut candidates: Vec<Arc<dyn DataProvider>>,
    ) -> anyhow::Result<Arc<dyn DataProvider>> {
        if let Some(forced) = &req.forced_provider {
            tracing::debug!("Forced provider constraint: {}", forced);

            let provider = candidates
                .into_iter()
                .find(|p| p.provider_name() == forced)
                .ok_or_else(|| {
                    tracing::warn!("Forced provider '{}' not available", forced);
                    anyhow::anyhow!("Provider '{}' not available", forced)
                })?;

            if !provider.capabilities().contains(&req.capability) || !provider.markets().contains(&req.market) {
                tracing::warn!(
                    "Forced provider '{}' does not support capability={:?}/market={:?}",
                    forced, req.capability, req.market
                );
                anyhow::bail!(
                    "Provider '{}' does not support this capability/market",
                    forced
                );
            }

            tracing::info!("Selected provider: {}", provider.provider_name());
            return Ok(provider);
        }

        if candidates.is_empty() {
            tracing::warn!(
                "No provider found for capability={:?}/market={:?}",
                req.capability, req.market
            );
            anyhow::bail!("No provider available");
        }

        candidates.sort_by_key(|p| std::cmp::Reverse(p.priority()));
        let provider = candidates.into_iter().next().unwrap();
        tracing::info!("Selected provider: {}", provider.provider_name());
        Ok(provider)
    }
}
