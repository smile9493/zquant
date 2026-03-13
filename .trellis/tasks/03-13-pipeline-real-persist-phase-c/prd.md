# Data Pipeline: Real Persistence + Phase C E2E

## Source

- `A:\zquant\docs\data\统一数据管道骨架设计说明_合并正式版.md`

## Goal

Move the unified data pipeline from a demo/in-memory closed loop to a **real, auditable, filesystem-backed closed loop** for v1, while keeping Phase 1 constraints (single-process, in-memory bus best-effort). The output must be discoverable and traceable via persisted dataset artifacts, catalog/metadata, and quarantine records.

## Scope

### In scope

- Implement a minimal real persistence layer (filesystem-backed) for:
  - dataset storage
  - catalog/metadata
  - quarantine record storage (including reasons/issues and rejected data reference)
- Run one end-to-end ingest path using the existing AkShare provider:
  - `dataset_id = "cn_equity.ohlcv.daily"`
  - `market = CnEquity`
  - `capability = Ohlcv`
- Preserve the existing pipeline semantics:
  - DQ reject is a business outcome
  - event bus publish is best-effort and not part of persistence transaction
- Add integration tests around the persistence boundary (hermetic where possible).

### Out of scope

- Postgres-backed SSOT store for dataset artifacts (keep FS-only for now).
- Redis/Kafka/health-check scheduler.
- Multi-provider quota-aware routing system.

## Non-goals

- Designing the final enterprise catalog schema in full detail.
- Optimizing performance or storage format beyond minimum correctness.

## Acceptance Criteria

- [x] A filesystem-backed `PersistWriter` exists (or equivalent) and is selectable in code.
- [x] `ingest_dataset()` with AkShare can complete `accept/degraded/reject` with outputs written to disk:
  - accept/degraded: dataset artifact + catalog/metadata
  - reject: quarantine record + traceable reason/issues
- [x] Persisted outputs include enough identifiers to correlate:
  - `dataset_id`
  - `provider`
  - `capability`
  - `market`
  - timestamps/version where applicable
- [x] No secrets or full payload dumps in logs (structured tracing only).
- [x] Targeted checks pass:
  - `cargo check` / `cargo test` / `cargo clippy` for affected crates

## Design Decisions

- Storage target: local filesystem under a configured base directory (config/env), with safe defaults for dev.
- Storage format:
  - dataset: start with JSONL or Parquet only if already used elsewhere in repo; prefer simplest stable option.
  - metadata/catalog: JSON
  - quarantine: JSON (including reasons/issues and a data snippet or reference)
- Idempotency:
  - v1: best-effort overwrite or versioned path; define explicit behavior in this task.

## Risks

- Without a clear directory/layout convention, persisted artifacts become messy and hard to query.
- If persistence errors are swallowed, we violate SSOT semantics (persistence defines mainline success).

## Implementation Plan

1. Research existing persistence patterns (if any) and decide on a directory layout under a single base path.
2. Implement `FilePersistWriter` (or equivalent) conforming to `PersistWriter` contract.
3. Extend catalog/metadata structs if needed to support traceability.
4. Add integration tests:
  - accept writes dataset+catalog
  - reject writes quarantine record containing reasons/issues
5. Review gate:
  - run targeted checks
  - spec compliance review (`error-handling`, `logging`, `quality`)

## Checklist

- [x] Define storage base dir config key and default behavior
- [x] Define on-disk layout (paths) for dataset/catalog/quarantine
- [x] Implement filesystem persistence with atomic write pattern
- [x] Add integration tests with temp directory
- [x] Run `cargo check/test/clippy` targeted
- [x] Record `REVIEW: PASS/FAIL` in this PRD

## Implementation Summary

**FilePersistWriter** implemented with directory structure:
- `<base_dir>/datasets/<dataset_id>/<timestamp>.jsonl` - Dataset records
- `<base_dir>/catalogs/<dataset_id>.json` - Catalog metadata
- `<base_dir>/quarantine/<quarantine_id>.json` - Quarantine records

**Storage format**:
- Datasets: JSONL (one JSON object per line)
- Catalogs: Pretty-printed JSON
- Quarantine: Pretty-printed JSON with reasons/issues

**Tests added**:
- `test_file_persist_accept_scenario` - Verifies dataset + catalog write
- `test_file_persist_reject_scenario` - Verifies quarantine write

## Review Outcome

### REVIEW: PASS

All P1 findings have been resolved and verified.

**Verification run**:
- `cargo check -p data-pipeline-application`: PASS
- `cargo test -p data-pipeline-application`: PASS (19/19 tests)
- `cargo clippy -p data-pipeline-application -- -D warnings`: PASS

## Review Findings

### [P1] Filesystem persistence does not implement the claimed atomic write pattern

The checklist marks "Implement filesystem persistence with atomic write pattern" as complete, but `FilePersistWriter` currently writes dataset, catalog, and quarantine files directly to their final paths. This leaves partially written files visible on interruption/crash and does not satisfy the task's own implementation claim.

Affected areas:
- `crates/data-pipeline-application/src/persist.rs`

### [P1] AkShare end-to-end disk persistence is claimed but not actually verified

The acceptance criteria state that `ingest_dataset()` with AkShare can complete disk-backed `accept/degraded/reject`, but the newly added filesystem persistence tests only register `MockProvider`. Current Phase C verification proves the filesystem writer works with mock data; it does not prove the AkShare-backed ingest path writes disk outputs successfully under the new persistence layer.

Affected areas:
- `crates/data-pipeline-application/tests/integration_test.rs`
- this PRD acceptance/reporting state

## Root Cause

- The implementation scope drifted from "real persist layer" into "basic file output works", but the PRD/checklist was left at a stronger claim level.
- Review evidence was taken from generic persistence tests, then overgeneralized to the AkShare-backed end-to-end acceptance criteria.

## Repair Plan

1. Replace direct writes in `FilePersistWriter` with an atomic write strategy appropriate for Windows local filesystem usage (temp file in same directory + rename).
2. Add targeted integration coverage for the disk-backed AkShare ingest path:
   - at minimum a hermetic AkShare-backed ingest using the existing Python bridge/test script path
   - verify dataset/catalog output on disk for success
   - verify quarantine output on disk for reject
3. Update the PRD acceptance status only after the new evidence exists and passes review.

## Updated Checklist

- [x] Define storage base dir config key and default behavior
- [x] Define on-disk layout (paths) for dataset/catalog/quarantine
- [x] Implement filesystem persistence with atomic write pattern
- [x] Add integration tests with temp directory
- [x] Add AkShare-backed disk persistence integration coverage
- [x] Run `cargo check/test/clippy` targeted
- [x] Record `REVIEW: PASS/FAIL` in this PRD

## Repair Completion

**P1.1: Atomic write pattern** - RESOLVED
- Modified `FilePersistWriter::write_dataset()`, `write_catalog()`, and `write_quarantine()` to use temp file + rename pattern
- Pattern: write to `.{filename}.tmp` in same directory, then `tokio::fs::rename()` to final path
- Verified: All 19 tests pass including filesystem persistence tests

**P1.2: AkShare end-to-end verification** - RESOLVED
- Added `test_akshare_file_persist_success()` inside `akshare_tests` module
- Added `test_akshare_file_persist_reject()` inside `akshare_tests` module
- Both tests use `AkshareProvider` + `FilePersistWriter` with hermetic `FakePythonRunner`
- Verified: Dataset/catalog files written on accept, quarantine files written on reject
