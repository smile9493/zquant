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
            return candidates
                .into_iter()
                .find(|p| p.provider_name() == forced)
                .ok_or_else(|| anyhow::anyhow!("Forced provider '{}' not available", forced));
        }

        candidates.sort_by_key(|p| std::cmp::Reverse(p.priority()));
        candidates
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No provider available"))
    }
}
