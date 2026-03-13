# Backend: Jobs List Stop Retry API

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`
- `A:\zquant\.trellis\tasks\03-14-web-workspace-controls-job-actions\prd.md`
- `A:\zquant\crates\job-application\src\api.rs`
- `A:\zquant\crates\job-domain\src\lib.rs`
- `A:\zquant\crates\job-store-pg\src\lib.rs`

## Goal

Expose the missing **HTTP jobs API surface** required by the frontend workspace next phase:

- `GET /jobs`
- `POST /jobs/:id/stop`
- `POST /jobs/:id/retry`

The implementation must stay aligned with the existing job domain/store model and must not invent a contract the backend cannot actually honor.

## Why This Is Next

The completed workspace MVP already depends on jobs data, and the next frontend task explicitly requires richer job interaction. Local backend verification shows:

- `POST /jobs` exists
- `GET /jobs/:id` exists
- `GET /jobs`, `POST /jobs/:id/stop`, and `POST /jobs/:id/retry` do not currently exist in the HTTP router
- stop has partial store support (`request_stop`)
- retry has no visible API/service implementation yet

Until this API surface exists, the frontend either blocks or degrades. We have chosen backend-first sequencing.

## Scope

### In scope

**Jobs list API**
- add `GET /jobs`
- return a stable list payload suitable for `JobsTab`
- include enough fields for the current web plan: `job_id`, `job_type`, `status`, `stop_requested`, `created_at`, `updated_at`

**Stop API**
- add `POST /jobs/:id/stop`
- connect the route to existing store-level stop request capability
- define success / not-found / invalid-state behavior clearly

**Retry API**
- add `POST /jobs/:id/retry`
- define retry semantics explicitly using the existing domain model
- if retry means “create a new queued job from the original job_type + payload”, implement it consistently and document the response contract

**Contract / validation**
- keep responses JSON and frontend-friendly
- avoid leaking raw internal errors
- preserve current router style and status-code conventions unless there is a strong reason to improve them

### Out of scope

- WebSocket or push updates
- Full job history filtering / pagination unless required for correctness
- Deep redesign of job execution semantics
- Changes to unrelated data-pipeline APIs

## Current Backend State

Verified locally:

- router currently exposes only:
  - `POST /jobs`
  - `GET /jobs/:id`
- `JobStore` currently supports:
  - `create_job(...)`
  - `get_job(...)`
  - `request_stop(...)`
- no retry service path or retry route was found

## Assumptions / Risks

- `GET /jobs` may require a new `JobStore::list_jobs(...)` method.
- retry semantics are not defined in code today, so this task must make them explicit in the PRD and implementation rather than guessing silently.
- the frontend likely does not need the full raw `Job` payload; a thinner API response model may be preferable to avoid overexposing payload/error blobs.
- if logs API also proves incomplete later, that should be handled in a separate follow-up unless tightly coupled to this work.

## Proposed API Contract

### `GET /jobs`

Response:

- array of job summaries ordered by newest or otherwise documented server order
- each item includes:
  - `job_id`
  - `job_type`
  - `status`
  - `stop_requested`
  - `created_at`
  - `updated_at`

### `POST /jobs/:id/stop`

Behavior:

- requests cooperative stop for an existing job
- response should be success even if the stop request is idempotently repeated
- return `404` when job does not exist

### `POST /jobs/:id/retry`

Proposed behavior:

- fetch existing job by `id`
- create a new queued job using the original `job_type` and `payload`
- return the new `job_id`
- return `404` when original job does not exist

This behavior must be kept explicit in implementation and tests.

## Acceptance Criteria

- [x] `GET /jobs` is exposed by the HTTP router.
- [x] `POST /jobs/:id/stop` is exposed by the HTTP router.
- [x] `POST /jobs/:id/retry` is exposed by the HTTP router.
- [x] `GET /jobs` returns a stable JSON contract suitable for the frontend jobs list.
- [x] `POST /jobs/:id/stop` is wired to the existing stop request path and handles not-found cleanly.
- [x] `POST /jobs/:id/retry` creates a new queued job from the original job's `job_type` and `payload`, or the PRD is updated if a different retry semantic is chosen.
- [x] New/updated tests cover list, stop, and retry behavior.
- [x] `cargo test` for the relevant job crates passes.
- [x] Review gate is completed and recorded here as `REVIEW: PASS` or `REVIEW: FAIL`.

## Implementation Plan

1. Define response/request DTOs for jobs list, stop, and retry.
2. Add any missing store methods needed for list and retry support.
3. Extend `job-application` router with `GET /jobs`, `POST /jobs/:id/stop`, and `POST /jobs/:id/retry`.
4. Add tests for new routes and retry semantics.
5. Run targeted Rust tests and complete review.

## Checklist

- [x] Define API DTOs and status-code behavior
- [x] Add `list_jobs` store capability if missing
- [x] Add stop handler and route
- [x] Add retry handler and route
- [x] Add tests for list / stop / retry
- [x] Run relevant `cargo test`
- [x] Complete review gate

## Review Findings

No blocking findings after the latest verification round.

## Root Cause

Resolved:

- `job-application` integration tests now clean their shared database before each run.
- `job-store-pg` test binaries no longer collide on the same live database because `e2e_test.rs` uses its own isolated database path and `pg_store_phase1.rs` keeps explicit cleanup on the shared one.

## Repair Plan

Completed.

## Verification

- `docker start zquant-postgres-test`: PASS
- `cargo test -p job-application` with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: PASS
- `cargo test -p job-store-pg` with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: PASS
- Verified sequentially in the same review run with no test-data conflict between crates.

## Review Outcome

### REVIEW: PASS

The missing jobs API surface is implemented, the required success/error-path tests are present, and the relevant job crate test suites now pass sequentially with the configured PostgreSQL test environment.

---

## Review Findings (2026-03-14 rerun)

- [P1] `cargo test -p job-store-pg` still fails in the current environment because `tests/e2e_test.rs` times out waiting for `Event::JobCompleted` and panics at `assert!(completed_ok)`. The crate-level acceptance criterion "`cargo test` for the relevant job crates passes" is therefore not satisfied.
- [P1] The task was archived with `REVIEW: PASS` before the full review gate was actually stable. Current task state and real test state are inconsistent.

## Root Cause (2026-03-14 rerun)

- `tests/e2e_test.rs` now isolates its database, but the event-driven completion assertion remains flaky or broken under the current runner/test timing.
- The previous pass decision over-weighted database isolation and did not require a clean rerun of the full `job-store-pg` crate after the last test changes.

## Repair Plan (2026-03-14 rerun)

1. Reproduce `cargo test -p job-store-pg --test e2e_test` until the failure mode is understood.
2. Fix the e2e lifecycle test so `JobCompleted` is observed deterministically, or adjust the test to validate completion through a stable contract instead of a timing-sensitive event assertion.
3. Re-run:
   - `cargo test -p job-application`
   - `cargo test -p job-store-pg`
4. Only after both commands pass in the same review run may this task return to `REVIEW: PASS`.

## Updated Checklist (2026-03-14 rerun)

- [x] Implement `GET /jobs`, `POST /jobs/:id/stop`, and `POST /jobs/:id/retry`
- [x] Add success/error-path API tests for list / stop / retry
- [x] Isolate API/store database usage across test binaries
- [ ] Make `job-store-pg` e2e test pass reliably
- [ ] Re-run full review gate and restore PASS only if all checks pass

## Verification (2026-03-14 rerun)

- `docker start zquant-postgres-test`: PASS
- `cargo check -p job-application`: PASS
- `cargo test -p job-application` with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: PASS
- `cargo test -p job-store-pg` with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: FAIL
  - failing test: `test_e2e_job_lifecycle`
  - failure point: `A:\\zquant\\crates\\job-store-pg\\tests\\e2e_test.rs:83`

## Review Outcome (2026-03-14 rerun)

### REVIEW: FAIL

The HTTP API work is present, but the relevant crate review gate is not actually green in the current environment. This task must not be treated as complete until `cargo test -p job-store-pg` passes cleanly.

---

## Review Findings (2026-03-14 final rerun)

No blocking findings after the stability fix and full rerun.

## Root Cause (2026-03-14 final rerun)

Resolved:

- `tests/e2e_test.rs` had a startup race between spawning the runner loop and creating/publishing the first job event.
- Adding a short startup delay before job creation made the lifecycle assertion deterministic in the current test harness.

## Repair Plan (2026-03-14 final rerun)

Completed.

## Updated Checklist (2026-03-14 final rerun)

- [x] Implement `GET /jobs`, `POST /jobs/:id/stop`, and `POST /jobs/:id/retry`
- [x] Add success/error-path API tests for list / stop / retry
- [x] Isolate API/store database usage across test binaries
- [x] Make `job-store-pg` e2e test pass reliably
- [x] Re-run full review gate and restore PASS only if all checks pass

## Verification (2026-03-14 final rerun)

- `docker start zquant-postgres-test`: PASS
- `cargo test -p job-application` with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: PASS
- `cargo test -p job-store-pg` with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: PASS
- `cargo test -p job-store-pg --test e2e_test` repeated 10 times with `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres`: PASS
- Verified sequentially in the same review run with no test-data conflict between crates.

## Review Outcome (2026-03-14 final rerun)

### REVIEW: PASS

The HTTP jobs API surface is implemented, the required API tests are present, database isolation is working, and the `job-store-pg` e2e lifecycle test now passes reliably in repeated runs.
