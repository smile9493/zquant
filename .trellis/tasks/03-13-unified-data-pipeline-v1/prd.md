# Unified Data Pipeline Skeleton (v1)

## Source Document

- `A:\zquant\docs\data\统一数据管道骨架设计说明_合并正式版.md` (v1.0-draft, 2026-03-13)

## Goal

Implement the **unified data pipeline skeleton** for the new zquant project so that callers no longer bind directly to concrete providers. All data access goes through a single entrypoint which owns:

- provider selection + fallback routing
- normalization
- DQ gating (reject / degraded / accept)
- persistence + quarantine
- event emission
- observability (tracing/metrics/logs)

This work is intended to integrate with the existing Phase 1 EDA kernel (jobs/agents/event bus/observability baseline).

## Scope (v1)

### In scope

- A single unified entrypoint: `DataPipelineManager` (naming to be confirmed)
- Provider unified contract + registry
- Capability / Market / Dataset request contract (minimal stable fields)
- Minimal route resolver (capability+market filter + priority + fallback)
- Normalization pipeline (minimum viable set for OHLCV + metadata completion)
- DQ gate integrated into `ingest_dataset` mainline (fail-controlled, not best-effort)
- Persistence writers (cleaned data + metadata/catalog) + quarantine writer
- Domain event emission (fetched / gated / ingested / rejected / degraded)
- Integration points with existing job/agent/event bus and current observability stack

### Explicit non-goals (v1)

- Full Redis two-level cache system
- Kafka end-to-end as the primary message backbone
- Full quota-aware routing / health-probe scheduler
- Frontend quota management UI
- Bulk migration of the entire provider ecosystem
- Full backward compatibility layer to mirror the legacy registry/runtime

## Acceptance Criteria

- [ ] There is exactly one supported entrypoint for external data access (no direct caller-to-provider binding in the supported paths).
- [ ] `fetch_dataset()` and `ingest_dataset()` exist and are usable by at least one caller path (job/agent/script/API).
- [ ] At least one provider combination works (market/reference/mock is acceptable for v1).
- [ ] `ingest_dataset()` supports the three outcomes: `accept`, `degraded`, `reject`.
- [ ] Quarantine writes happen on `reject`, with enough information to trace why it was rejected.
- [ ] Metadata/catalog is written for accepted/degraded ingests.
- [ ] Events are emitted with stable, versioned fields for: `dataset.fetched`, `dataset.gate.completed`, `dataset.ingested`, `dq.rejection`, `dq.degraded`.
- [ ] Tracing/metrics/logging allow locating failures in provider fetch, normalization, DQ, persistence, and event emission.
- [ ] The core boundaries are replaceable to allow future Redis/Kafka/health-check enhancements without breaking callers.

## Assumptions / Constraints

- Phase 1 EDA kernel exists and remains the runtime baseline (do not introduce a parallel runtime).
- Thresholds / policies are configuration-driven (no caller hard-coding).
- Fail-closed semantics apply when constraints are strong (e.g., forced provider), to avoid silent semantic drift.
- DQ reject is a **controlled business outcome**, not just an exception path.

## Risks

- Under-specified provider contract causes rework when more providers are added.
- DQ not truly in the mainline leads to inconsistent semantics downstream.
- Persistence/event emission boundaries become blurry and hinder audit/debugging.
- Over-scoping (introducing Redis/Kafka too early) dilutes the skeleton goal.

## Open Questions

- Final naming: `DataPipelineManager` vs `DataSourceManager` (or unify).
- Which initial providers to implement first and their priority rules.
- Final metadata/catalog storage schema and minimum external fields.
- Minimum DQ report fields that must be returned and persisted.
- Event naming/versioning strategy relative to existing domain events.

## Implementation Plan (Phased)

### Phase A — Skeleton landing

- [ ] Define request/response contracts (capability/market/dataset, ingest result)
- [ ] Provider trait + registry
- [ ] Route resolver (minimal)
- [ ] Normalizer interface (min viable normalization)
- [ ] DQ gate interface
- [ ] Persistence writer interfaces
- [ ] Event emitter interface

### Phase B — Runnable minimal closed loop

- [ ] Implement 1–2 baseline providers (at least one usable path end-to-end)
- [ ] `fetch_dataset()` runnable end-to-end
- [ ] `ingest_dataset()` runnable end-to-end
- [ ] Three-state decision with quarantine + metadata/catalog writes
- [ ] Emit events for all key steps

### Phase C — Async + observability hardening

- [ ] Integrate as a job handler where appropriate
- [ ] Wire to current event bus
- [ ] Ensure tracing/metrics/logging coverage for all stages
- [ ] Add basic regression/integration tests for accept/degraded/reject

## Concrete Task Checklist (Next Actions)

- [ ] Run a focused codebase research pass to find existing patterns for: event emission, persistence, config, and job/agent integration.
- [ ] Decide dev type + initialize Trellis context (`backend` expected).
- [ ] Produce minimal Rust module layout for pipeline components and contracts.
- [ ] Define the initial provider(s) to ship with v1 (mock + one real if available).

