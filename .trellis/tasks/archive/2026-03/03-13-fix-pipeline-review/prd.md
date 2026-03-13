# Fix Data Pipeline v1 Review Findings

## Goal

Fix 4 critical issues found in code review of unified data pipeline v1 to meet all acceptance criteria.

## Issues to Fix

### P1-1: Reject path quarantine persistence

**Problem**:
- manager.rs writes synthetic `{"rejected": true}` instead of actual rejected data
- persist.rs drops QuarantineReason, losing rejection metadata
- Cannot trace why data was rejected

**Fix**:
- Store rejected data (raw or normalized) in quarantine
- Include rejection reasons and DQ issues in quarantine record
- Update QuarantineRecord structure to hold both data and metadata

### P1-2: dataset_id preservation

**Problem**:
- ingest_dataset() ignores caller-provided dataset_id
- Always generates new UUID, breaking cross-layer contract
- Cannot correlate persisted data with request

**Fix**:
- Use caller-provided dataset_id when present
- Only generate new ID when request omits it
- Preserve ID across persistence and events

### P2-3: fetch() routing inconsistency

**Problem**:
- fetch() bypasses RouteResolver, takes first provider
- Doesn't apply priority/forced-provider policies
- Inconsistent with fetch_dataset() behavior

**Fix**:
- Make fetch() use RouteResolver like fetch_dataset()
- Apply same routing policies to both entrypoints

### P2-4: Event versioning

**Problem**:
- Pipeline events lack schema_v field
- PRD requires "stable, versioned fields"
- Inconsistent with JobLifecycleEvent pattern

**Fix**:
- Add schema_v field to all 5 pipeline events
- Set initial version to "1.0"

## Acceptance Criteria

- [ ] Quarantine records contain rejected data + reasons (traceable)
- [ ] ingest_dataset() preserves caller-provided dataset_id
- [ ] fetch() uses RouteResolver with same policies as fetch_dataset()
- [ ] All 5 pipeline events have schema_v field
- [ ] Tests verify all 4 fixes

## Files to Modify

- `crates/data-pipeline-application/src/manager.rs`
- `crates/data-pipeline-application/src/persist.rs`
- `crates/data-pipeline-domain/src/result.rs` (QuarantineRecord structure)
- `crates/job-events/src/types.rs` (add schema_v to events)
- `crates/data-pipeline-application/tests/integration_test.rs` (add tests)
