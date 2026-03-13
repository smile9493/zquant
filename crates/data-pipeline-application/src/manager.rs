use data_pipeline_domain::{
    DatasetRequest, FetchRequest, IngestRequest, IngestResult, NormalizedData, RawData,
};

use crate::events::EventEmitter;
use crate::normalizer::Normalizer;
use crate::persist::PersistWriter;
use crate::provider_registry::ProviderRegistry;
use crate::quality_gate::QualityGate;
use crate::route_resolver::RouteResolver;

#[allow(dead_code)]
pub struct DataPipelineManager {
    registry: ProviderRegistry,
    resolver: Box<dyn RouteResolver>,
    normalizer: Box<dyn Normalizer>,
    quality_gate: Box<dyn QualityGate>,
    persist_writer: Box<dyn PersistWriter>,
    event_emitter: Box<dyn EventEmitter>,
}

impl DataPipelineManager {
    pub fn new(
        registry: ProviderRegistry,
        resolver: Box<dyn RouteResolver>,
        normalizer: Box<dyn Normalizer>,
        quality_gate: Box<dyn QualityGate>,
        persist_writer: Box<dyn PersistWriter>,
        event_emitter: Box<dyn EventEmitter>,
    ) -> Self {
        Self {
            registry,
            resolver,
            normalizer,
            quality_gate,
            persist_writer,
            event_emitter,
        }
    }

    #[tracing::instrument(skip(self), fields(capability = ?req.capability, market = ?req.market))]
    pub async fn fetch(&self, req: FetchRequest) -> anyhow::Result<RawData> {
        let candidates = self.registry.find_providers(req.capability, req.market);
        let provider = self.resolver.resolve(&DatasetRequest {
            capability: req.capability,
            market: req.market,
            dataset_id: None,
            time_range: None,
            forced_provider: None,
        }, candidates).await
            .map_err(|e| anyhow::anyhow!("failed to resolve provider for {:?}/{:?}: {}", req.capability, req.market, e))?;
        provider.fetch(req).await
            .map_err(|e| anyhow::anyhow!("failed to fetch data from provider {}: {}", provider.provider_name(), e))
    }

    #[tracing::instrument(skip(self), fields(capability = ?req.capability, market = ?req.market))]
    pub async fn fetch_dataset(&self, req: DatasetRequest) -> anyhow::Result<NormalizedData> {
        let candidates = self.registry.find_providers(req.capability, req.market);
        let provider = self.resolver.resolve(&req, candidates).await
            .map_err(|e| anyhow::anyhow!("failed to resolve provider for {:?}/{:?}: {}", req.capability, req.market, e))?;
        let raw = provider.fetch_dataset(req.clone()).await
            .map_err(|e| anyhow::anyhow!("failed to fetch dataset from provider {}: {}", provider.provider_name(), e))?;
        self.normalizer.normalize(raw).await
            .map_err(|e| anyhow::anyhow!("failed to normalize data: {}", e))
    }

    #[tracing::instrument(skip(self), fields(capability = ?req.dataset_request.capability, market = ?req.dataset_request.market))]
    pub async fn ingest_dataset(&self, req: IngestRequest) -> anyhow::Result<IngestResult> {
        use chrono::Utc;
        use data_pipeline_domain::DqDecision;

        let dataset_id = req.dataset_request.dataset_id.clone()
            .unwrap_or_else(|| format!("ds_{}", uuid::Uuid::new_v4()));
        let dr = &req.dataset_request;

        let normalized = self.fetch_dataset(dr.clone()).await
            .map_err(|e| anyhow::anyhow!("failed to fetch dataset for ingestion: {}", e))?;

        let candidates = self.registry.find_providers(dr.capability, dr.market);
        let provider = self.resolver.resolve(dr, candidates).await
            .map_err(|e| anyhow::anyhow!("failed to resolve provider for ingestion: {}", e))?;

        self.event_emitter
            .emit_dataset_fetched(crate::events::DatasetFetchedEvent {
                dataset_id: dataset_id.clone(),
                provider: provider.provider_name().to_string(),
                capability: dr.capability,
                market: dr.market,
                timestamp: Utc::now(),
                row_count: normalized.records.len(),
            })
            .await?;

        let qr = self.quality_gate.check(&normalized).await?;

        self.event_emitter
            .emit_dataset_gate_completed(crate::events::DatasetGateCompletedEvent {
                dataset_id: dataset_id.clone(),
                decision: qr.decision,
                quality_score: qr.quality_score,
                issue_count: qr.issues.len(),
                timestamp: Utc::now(),
            })
            .await?;

        match qr.decision {
            DqDecision::Accept | DqDecision::Degraded => {
                let metadata = crate::persist::DatasetMetadata {
                    dataset_id: dataset_id.clone(),
                    provider: provider.provider_name().to_string(),
                    capability: dr.capability,
                    market: dr.market,
                    available_at: Some(Utc::now()),
                    point_in_time: None,
                    version: 1,
                };

                let receipt = self
                    .persist_writer
                    .write_dataset(&qr.cleaned_data, &metadata)
                    .await?;

                let catalog = crate::persist::CatalogEntry {
                    dataset_id: dataset_id.clone(),
                    metadata,
                };
                let catalog_id = self.persist_writer.write_catalog(&catalog).await?;

                self.event_emitter
                    .emit_dataset_ingested(crate::events::DatasetIngestedEvent {
                        dataset_id: dataset_id.clone(),
                        decision: qr.decision,
                        storage_path: receipt.storage_path.clone(),
                        catalog_id: catalog_id.clone(),
                        timestamp: Utc::now(),
                    })
                    .await?;

                if qr.decision == DqDecision::Degraded {
                    self.event_emitter
                        .emit_dq_degraded(crate::events::DqDegradedEvent {
                            dataset_id: dataset_id.clone(),
                            quality_score: qr.quality_score,
                            issues: qr.issues.clone(),
                            timestamp: Utc::now(),
                        })
                        .await?;
                }

                Ok(IngestResult {
                    dataset_id: Some(dataset_id),
                    decision: qr.decision,
                    quarantine_id: None,
                    persist_receipt: Some(receipt),
                })
            }
            DqDecision::Reject => {
                let reasons: Vec<String> = qr.issues.iter().map(|i| i.message.clone()).collect();
                let quarantine_record = data_pipeline_domain::QuarantineRecord {
                    rejected_data: qr.cleaned_data.clone(),
                    reasons: reasons.clone(),
                    dq_issues: qr.issues.clone(),
                };
                let quarantine_id = self
                    .persist_writer
                    .write_quarantine(&quarantine_record)
                    .await?;

                self.event_emitter
                    .emit_dq_rejection(crate::events::DqRejectionEvent {
                        quarantine_id: quarantine_id.clone(),
                        dataset_id: dataset_id.clone(),
                        reasons,
                        timestamp: Utc::now(),
                    })
                    .await?;

                Ok(IngestResult {
                    dataset_id: Some(dataset_id),
                    decision: DqDecision::Reject,
                    quarantine_id: Some(quarantine_id),
                    persist_receipt: None,
                })
            }
        }
    }
}
