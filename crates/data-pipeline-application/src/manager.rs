use data_pipeline_domain::{
    DatasetRequest, FetchRequest, IngestRequest, IngestResult, NormalizedData, RawData,
};
use std::time::Instant;
use tracing::Instrument;

use crate::events::EventEmitter;
use crate::metrics;
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
            symbol_scope: Vec::new(),
            time_range: None,
            forced_provider: None,
        }, candidates).await
            .map_err(|e| anyhow::anyhow!("failed to resolve provider for {:?}/{:?}: {}", req.capability, req.market, e))?;
        provider.fetch(req).await
            .map_err(|e| anyhow::anyhow!("failed to fetch data from provider {}: {}", provider.provider_name(), e))
    }

    #[tracing::instrument(skip(self), fields(capability = ?req.capability, market = ?req.market, provider))]
    pub async fn fetch_dataset(&self, req: DatasetRequest) -> anyhow::Result<NormalizedData> {
        let candidates = self.registry.find_providers(req.capability, req.market);
        let provider = self.resolver.resolve(&req, candidates).await
            .map_err(|e| anyhow::anyhow!("failed to resolve provider for {:?}/{:?}: {}", req.capability, req.market, e))?;

        tracing::Span::current().record("provider", provider.provider_name());

        let raw = match provider.fetch_dataset(req.clone()).await {
            Ok(data) => data,
            Err(e) => {
                metrics::record_stage_error("provider".to_string());
                return Err(anyhow::anyhow!("failed to fetch dataset from provider {}: {}", provider.provider_name(), e));
            }
        };

        match self.normalizer.normalize(raw).await {
            Ok(data) => Ok(data),
            Err(e) => {
                metrics::record_stage_error("normalize".to_string());
                Err(anyhow::anyhow!("failed to normalize data: {}", e))
            }
        }
    }

    #[tracing::instrument(skip(self), fields(capability = ?req.dataset_request.capability, market = ?req.dataset_request.market, dataset_id, decision))]
    pub async fn ingest_dataset(&self, req: IngestRequest) -> anyhow::Result<IngestResult> {
        use chrono::Utc;
        use data_pipeline_domain::DqDecision;

        let dataset_id = req.dataset_request.dataset_id.clone()
            .unwrap_or_else(|| format!("ds_{}", uuid::Uuid::new_v4()));
        let dr = &req.dataset_request;

        tracing::Span::current().record("dataset_id", dataset_id.as_str());

        let fetch_start = Instant::now();
        let normalized = match self.fetch_dataset(dr.clone()).await {
            Ok(data) => {
                metrics::record_stage_duration("fetch".to_string(), fetch_start.elapsed().as_secs_f64());
                data
            }
            Err(e) => {
                metrics::record_stage_error("fetch".to_string());
                return Err(anyhow::anyhow!("failed to fetch dataset for ingestion: {}", e));
            }
        };

        let candidates = self.registry.find_providers(dr.capability, dr.market);
        let provider = self.resolver.resolve(dr, candidates).await
            .map_err(|e| anyhow::anyhow!("failed to resolve provider for ingestion: {}", e))?;

        let emit_start = Instant::now();
        let emit_result = self.event_emitter
            .emit_dataset_fetched(crate::events::DatasetFetchedEvent {
                schema_v: "1.0".to_string(),
                dataset_id: dataset_id.clone(),
                provider: provider.provider_name().to_string(),
                capability: dr.capability.to_string(),
                market: dr.market.to_string(),
                timestamp: Utc::now(),
                row_count: normalized.records.len(),
            })
            .instrument(tracing::info_span!(
                "emit_event",
                event_type = "dataset_fetched",
                dataset_id = %dataset_id,
                provider = %provider.provider_name(),
                capability = ?dr.capability,
                market = ?dr.market
            ))
            .await;
        if let Err(e) = emit_result {
            metrics::record_stage_error("emit".to_string());
            return Err(e);
        }
        metrics::record_stage_duration("emit".to_string(), emit_start.elapsed().as_secs_f64());

        let dq_start = Instant::now();
        let qr = self.quality_gate.check(&normalized).await?;
        metrics::record_stage_duration("dq".to_string(), dq_start.elapsed().as_secs_f64());

        let emit_start = Instant::now();
        let emit_result = self.event_emitter
            .emit_dataset_gate_completed(crate::events::DatasetGateCompletedEvent {
                schema_v: "1.0".to_string(),
                dataset_id: dataset_id.clone(),
                decision: qr.decision.to_string(),
                quality_score: qr.quality_score,
                issue_count: qr.issues.len(),
                timestamp: Utc::now(),
            })
            .instrument(tracing::info_span!(
                "emit_event",
                event_type = "gate_completed",
                dataset_id = %dataset_id,
                decision = ?qr.decision
            ))
            .await;
        if let Err(e) = emit_result {
            metrics::record_stage_error("emit".to_string());
            return Err(e);
        }
        metrics::record_stage_duration("emit".to_string(), emit_start.elapsed().as_secs_f64());

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

                let persist_start = Instant::now();
                let receipt = match self.persist_writer.write_dataset(&qr.cleaned_data, &metadata).await {
                    Ok(r) => r,
                    Err(e) => {
                        metrics::record_stage_error("persist".to_string());
                        return Err(e);
                    }
                };

                let catalog = crate::persist::CatalogEntry {
                    dataset_id: dataset_id.clone(),
                    metadata,
                };
                let catalog_id = match self.persist_writer.write_catalog(&catalog).await {
                    Ok(id) => id,
                    Err(e) => {
                        metrics::record_stage_error("persist".to_string());
                        return Err(e);
                    }
                };
                metrics::record_stage_duration("persist".to_string(), persist_start.elapsed().as_secs_f64());

                let emit_start = Instant::now();
                let emit_result = self.event_emitter
                    .emit_dataset_ingested(crate::events::DatasetIngestedEvent {
                        schema_v: "1.0".to_string(),
                        dataset_id: dataset_id.clone(),
                        decision: qr.decision.to_string(),
                        storage_path: receipt.storage_path.clone(),
                        catalog_id: catalog_id.clone(),
                        timestamp: Utc::now(),
                    })
                    .instrument(tracing::info_span!(
                        "emit_event",
                        event_type = "dataset_ingested",
                        dataset_id = %dataset_id,
                        decision = ?qr.decision
                    ))
                    .await;
                if let Err(e) = emit_result {
                    metrics::record_stage_error("emit".to_string());
                    return Err(e);
                }
                metrics::record_stage_duration("emit".to_string(), emit_start.elapsed().as_secs_f64());

                if qr.decision == DqDecision::Degraded {
                    let emit_start = Instant::now();
                    let emit_result = self.event_emitter
                        .emit_dq_degraded(crate::events::DqDegradedEvent {
                            schema_v: "1.0".to_string(),
                            dataset_id: dataset_id.clone(),
                            quality_score: qr.quality_score,
                            issues: qr.issues.iter().map(|i| crate::events::DqIssue {
                                severity: i.severity.to_string(),
                                field: i.field.clone(),
                                message: i.message.clone(),
                            }).collect(),
                            timestamp: Utc::now(),
                        })
                        .instrument(tracing::info_span!(
                            "emit_event",
                            event_type = "dq_degraded",
                            dataset_id = %dataset_id
                        ))
                        .await;
                    if let Err(e) = emit_result {
                        metrics::record_stage_error("emit".to_string());
                        return Err(e);
                    }
                    metrics::record_stage_duration("emit".to_string(), emit_start.elapsed().as_secs_f64());
                }

                let decision_str = match qr.decision {
                    DqDecision::Accept => "accept",
                    DqDecision::Degraded => "degraded",
                    _ => "unknown",
                };
                metrics::record_ingest_result(decision_str.to_string());
                tracing::Span::current().record("decision", decision_str);

                Ok(IngestResult {
                    dataset_id: Some(dataset_id),
                    decision: qr.decision,
                    quarantine_id: None,
                    persist_receipt: Some(receipt),
                })
            }
            DqDecision::Reject => {
                metrics::record_stage_error("dq".to_string());

                let reasons: Vec<String> = qr.issues.iter().map(|i| i.message.clone()).collect();
                let quarantine_record = data_pipeline_domain::QuarantineRecord {
                    rejected_data: qr.cleaned_data.clone(),
                    reasons: reasons.clone(),
                    dq_issues: qr.issues.clone(),
                };

                let persist_start = Instant::now();
                let quarantine_id = match self.persist_writer.write_quarantine(&quarantine_record).await {
                    Ok(id) => id,
                    Err(e) => {
                        metrics::record_stage_error("persist".to_string());
                        return Err(e);
                    }
                };
                metrics::record_stage_duration("persist".to_string(), persist_start.elapsed().as_secs_f64());

                let emit_start = Instant::now();
                let emit_result = self.event_emitter
                    .emit_dq_rejection(crate::events::DqRejectionEvent {
                        schema_v: "1.0".to_string(),
                        quarantine_id: quarantine_id.clone(),
                        dataset_id: dataset_id.clone(),
                        reasons,
                        timestamp: Utc::now(),
                    })
                    .instrument(tracing::info_span!(
                        "emit_event",
                        event_type = "dq_rejection",
                        dataset_id = %dataset_id
                    ))
                    .await;
                if let Err(e) = emit_result {
                    metrics::record_stage_error("emit".to_string());
                    return Err(e);
                }
                metrics::record_stage_duration("emit".to_string(), emit_start.elapsed().as_secs_f64());

                metrics::record_ingest_result("reject".to_string());
                tracing::Span::current().record("decision", "reject");

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
