use std::sync::Arc;

use data_pipeline_domain::{Capability, DataProvider, Market};

pub struct ProviderRegistry {
    providers: Vec<Arc<dyn DataProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn DataProvider>) {
        self.providers.push(provider);
    }

    pub fn find_providers(
        &self,
        capability: Capability,
        market: Market,
    ) -> Vec<Arc<dyn DataProvider>> {
        self.providers
            .iter()
            .filter(|p| {
                p.capabilities().contains(&capability) && p.markets().contains(&market)
            })
            .cloned()
            .collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
