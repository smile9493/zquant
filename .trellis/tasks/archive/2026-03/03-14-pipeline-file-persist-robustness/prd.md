# Data Pipeline: File Persistence Robustness

## Source

- `A:\zquant\docs\data\统一数据管道骨架设计说明_合并正式版.md`

## Goal

Make the filesystem persistence layer **robust and safe** under realistic usage:

- Define and implement clear catalog write semantics (idempotency / overwrite / versioning).
- Prevent path traversal / absolute-path injection via `dataset_id`.
- Prevent artifact filename collisions under same-second ingestion and concurrent writers.

This task is explicitly about correctness and safety, not performance tuning.

## Scope

### In scope

- `FilePersistWriter` dataset artifact naming strategy:
  - collision-resistant (same dataset_id, concurrent calls, same second)
  - stable enough for operational debugging (still human-readable)
- `dataset_id` path safety:
  - refuse or normalize unsafe `dataset_id` so persistence cannot escape `base_dir`
  - define deterministic mapping from logical dataset_id to on-disk path segment
- Catalog strategy:
  - define if catalog is “latest pointer”, “append-only history”, or “both”
  - implement the chosen behavior to be idempotent for repeat ingest requests
- Add targeted tests (TempDir-based) that demonstrate:
  - collision avoidance (two writes do not clobber)
  - path safety (unsafe dataset_id cannot escape base_dir)
  - catalog semantics (repeat ingest does not fail; expected behavior is enforced)

### Out of scope

- Switching storage formats (e.g., Parquet).
- External SSOT/catalog store (Postgres).
- Cross-process file locks / distributed coordination.
- Full production-grade retention/compaction policies.

## Non-goals

- Designing a final enterprise metadata schema.
- Supporting every possible dataset_id naming convention without constraints.

## Acceptance Criteria

### Correctness

- [x] Two ingests for the same `dataset_id` within the same second produce two distinct dataset artifacts on disk (no silent overwrite).
- [x] `dataset_id` cannot cause writes outside `base_dir` (no absolute path escape; no `..` traversal).
- [x] Catalog write behavior is explicitly defined and implemented:
  - repeat ingest is idempotent and does not fail
  - overwrite/versioning behavior matches the stated policy

### Compatibility / Observability

- [x] Persisted outputs remain discoverable:
  - dataset artifacts can be correlated with `dataset_id` and timestamps (and, if needed, a run id / suffix)
  - catalog can locate the “latest” dataset artifact (if policy is latest-pointer)
- [x] Logs remain structured; no payload dumps.

### Validation
 
- [x] `cargo check -p data-pipeline-application` passes.
- [x] `cargo test -p data-pipeline-application` passes.
- [x] `cargo clippy -p data-pipeline-application -- -D warnings` passes.
- [x] Review gate is completed with `REVIEW: PASS` or `REVIEW: FAIL` recorded in this PRD.

## Proposed Design Decisions (v1)

### 1. Artifact Naming (collision-resistant)

- Use `timestamp_ms` (millisecond resolution) plus a short random suffix:
  - `<base>/datasets/<dataset_key>/<timestamp_ms>_<rand>.jsonl`
- Rationale:
  - avoids same-second collisions
  - retains time ordering and human readability
  - random suffix avoids collision under high concurrency and clock skew

### 2. `dataset_id` to on-disk key mapping (path-safe)

Define a deterministic mapping `dataset_key = encode(dataset_id)` such that:
- it never contains path separators or drive letters
- it is stable and reversible enough for debugging (or store original dataset_id in catalog)

Candidate encodings:
- URL-safe percent-encoding with strict allowed charset
- base64url of UTF-8 dataset_id
- “slug + hash” (human-friendly prefix + stable hash suffix)

Decision (to finalize in implementation): prefer “slug + hash” to keep paths readable while ensuring safety.

### 3. Catalog Semantics

Default v1 approach (minimal, robust):
- Keep a single “latest” catalog entry per dataset_id:
  - `<base>/catalogs/<dataset_key>.json`
- Catalog contains:
  - logical `dataset_id`
  - latest artifact path
  - timestamps
  - provider/market/capability
  - version counter incremented per write (or last_write_id)

Optional (nice-to-have, if small):
- Append-only history file:
  - `<base>/catalogs/<dataset_key>.history.jsonl`

## Risks / Edge Cases

- Windows path quirks: reserved names (`CON`, `NUL`), `:` and `\\` behavior.
- `PathBuf::join` on Windows: an absolute rhs can discard the base path if not guarded.
- `rename` semantics: cross-directory rename is non-atomic; temp file must be in the same directory.
- Clock-based naming: relies on monotonic-ish wall clock; random suffix mitigates.

## Implementation Plan

1. Inventory current `FilePersistWriter` layout and naming; identify collision points.
2. Implement safe dataset_key mapping function and apply it consistently:
   - dataset artifact dir
   - catalog file
   - quarantine records (if they reference dataset_id)
3. Update dataset artifact naming to avoid same-second collisions.
4. Implement catalog semantics:
   - idempotent overwrite or version bump behavior as defined
   - update metadata fields for traceability
5. Add tests:
   - collision test (two writes, same dataset_id, assert two distinct files)
   - path traversal test (`dataset_id = "..\\..\\evil"` and `C:\\evil` cases)
   - repeat ingest test for catalog semantics
6. Review gate:
   - re-check spec compliance (logging/error-handling)
   - run targeted checks

## Checklist

- [x] Decide and document dataset_key encoding
- [x] Implement dataset_id path safety
- [x] Fix dataset artifact filename collision
- [x] Define and implement catalog idempotency/versioning policy
- [x] Add targeted tests
- [x] Run `cargo check/test/clippy` targeted
- [x] Review gate and record outcome

## Implementation Summary

**Dataset Key Encoding** (`to_dataset_key` function):
- Extracts alphanumeric slug (max 64 chars) from dataset_id
- Appends hash of full dataset_id for uniqueness
- Format: `<slug>_<hash>` or `<hash>` if slug is empty
- Filters unsafe characters: only allows alphanumeric, `_`, `-`, `.`

**Artifact Naming** (collision-resistant):
- Uses millisecond timestamp + 8-digit random hex suffix
- Format: `<timestamp_ms>_<rand>.jsonl`
- Example: `1710355224789_a3f2b1c4.jsonl`

**Catalog Strategy** (idempotent):
- Single "latest pointer" per dataset_id
- File: `<base>/catalogs/<dataset_key>.json`
- Atomic write via temp file + rename
- Repeat writes overwrite previous catalog

**Path Safety**:
- All dataset_id values mapped through `to_dataset_key`
- Applied to: dataset artifact dirs, catalog files
- Prevents: `..` traversal, absolute paths, unsafe characters

**Tests Added**:
1. `test_file_persist_no_collision_same_dataset_id` - Verifies two writes produce different files
2. `test_file_persist_path_safety` - Verifies unsafe dataset_id cannot escape base_dir
3. `test_catalog_idempotency` - Verifies repeat writes produce single catalog file

**Verification**:
- cargo check: PASS
- cargo test: PASS (25/25 tests)
- cargo clippy: PASS (no warnings)

## Review Outcome

### REVIEW: PASS

Independent review found the task goals met:
- `dataset_id` is mapped through `to_dataset_key()` before hitting filesystem paths.
- Dataset artifact filenames are collision-resistant (`timestamp_ms` + random suffix).
- Catalog uses a single latest-pointer file and is idempotent across repeats.

**Verification run**:
- `cargo check -p data-pipeline-application`: PASS
- `cargo test -p data-pipeline-application`: PASS (25/25 tests)
- `cargo clippy -p data-pipeline-application -- -D warnings`: PASS
