# Web: Workspace Controls and Job Actions

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`
- `A:\zquant\.trellis\tasks\archive\2026-03\03-14-web-workspace-mvp-vue\prd.md`

## Goal

Deliver the **next frontend phase after the workspace MVP** by making `/workspace` operationally usable for day-to-day research:

- complete the `TopBar` control strip
- add real job actions in `JobsTab`
- improve `LogsTab` usability around the selected job
- keep the implementation within the current HTTP-only Phase 1 backend boundary

## Why This Is Next

The MVP shell is now complete, but the current workspace is still weak in the two places the source document leaves for the next phase:

- the `TopBar` still lacks mode / refresh / connection affordances
- `JobsTab` is read-only and does not expose `stop` / `retry`
- `LogsTab` works, but still needs explicit selected-job context and better empty / refresh states

This task closes those interaction gaps without expanding the architecture into WebSocket, event bus, or multi-panel features.

## Backend Contract Check

Current local backend verification result:

- exposed routes currently include:
  - `POST /jobs`
  - `GET /jobs/:id`
- not currently exposed in the local HTTP API:
  - `GET /jobs`
  - `POST /jobs/:id/stop`
  - `POST /jobs/:id/retry`
- there is store-level support for stop requests via `JobStore::request_stop(job_id, reason)`, but no HTTP handler is wired yet
- no retry API or retry service path was found in the current backend code

Verified code references:

- `A:\zquant\crates\job-application\src\api.rs`
- `A:\zquant\crates\job-domain\src\lib.rs`
- `A:\zquant\crates\job-store-pg\src\lib.rs`

Decision:

- this task will **not** invent degraded `stop` / `retry` interactions first
- we will **first** add the missing backend jobs API surface, then return to this frontend task

Implication for this task:

- frontend must not assume `GET /jobs`, `POST /jobs/:id/stop`, or `POST /jobs/:id/retry` are already available
- this task is now explicitly blocked on a backend follow-up that exposes the required routes
- `Job` detail shape is richer than the current frontend model and includes fields such as `job_type`, `updated_at`, `stop_requested`, `stop_reason`, `progress`, `error`, and `artifacts`

## Scope

### In scope

**TopBar enhancement**
- show current mode (`research`)
- show current `symbol` and `timeframe`
- add manual refresh control for chart / jobs / logs queries
- add lightweight API connection status indicator based on existing health endpoint

**Jobs interaction**
- highlight selected job in `JobsTab`
- surface basic metadata already available from the API model
- add `stop` / `retry` actions when backend endpoints are available
- use confirm modal for destructive / operational actions
- refresh jobs and logs after actions complete

**Logs usability**
- show selected job context in `LogsTab`
- show explicit empty state when no job is selected
- add manual refresh action
- preserve current polling behavior

**State / API cleanup needed for the above**
- introduce a dedicated job-oriented store if needed (`useJobStore`) instead of overloading `useWorkspaceStore`
- add typed API wrappers for `GET /jobs/:id`, `POST /jobs/:id/stop`, `POST /jobs/:id/retry` if those routes exist
- normalize action errors into user-safe messages

### Out of scope

- WebSocket bridge
- Typed frontend event bus / optimistic UI queue
- Multi-route workspace expansion
- Multi-chart layout
- Cmd+K / command palette
- Full system logs explorer redesign

## Acceptance Criteria

- [ ] `TopBar` shows `mode`, current `symbol`, current `timeframe`, manual refresh, and API health status.
- [ ] `JobsTab` visually indicates the selected job.
- [ ] `JobsTab` supports `stop` / `retry` actions with confirmation when the backend exposes those endpoints; otherwise the UI clearly disables or hides unavailable actions.
- [ ] Action completion triggers the necessary query refresh so job state and related logs are updated.
- [ ] `LogsTab` shows which job is selected and provides a clear empty state before selection.
- [ ] `LogsTab` exposes a manual refresh control in addition to polling.
- [ ] Errors for job actions remain user-safe and do not dump raw payloads.
- [ ] `npm run build` passes in `A:\zquant\web`.
- [ ] Review gate is completed and recorded here as `REVIEW: PASS` or `REVIEW: FAIL`.

## Assumptions / Risks

- The backend currently does not expose `GET /jobs`, `POST /jobs/:id/stop`, or `POST /jobs/:id/retry` via the local HTTP router. If these remain absent, this task must degrade gracefully rather than fabricate unsupported behavior.
- The existing frontend `Job` type is underspecified relative to the backend domain model and will likely need to grow beyond `id`, `status`, and `created_at`.
- If the health endpoint does not currently expose all desired fields, the `TopBar` status indicator should stay minimal and avoid creating a fake contract.

## Implementation Plan

1. Wait for the backend prerequisite task to expose `GET /jobs`, `POST /jobs/:id/stop`, and `POST /jobs/:id/retry`.
2. Refactor state so selected job and job actions have a clear owner (`useJobStore` if justified).
3. Enhance `TopBar` with mode, symbol/timeframe summary, manual refresh, and health indicator.
4. Upgrade `JobsTab` with selection styling, richer metadata, and action controls that are enabled only when the backend contract supports them.
5. Upgrade `LogsTab` with selected-job header, empty state, and manual refresh.
6. Run build and review against the web doc plus this PRD.

## Checklist

- [x] Verify job action API contract
- [x] Decide frontend-only degrade vs backend contract follow-up
- [x] Choose backend-first sequencing for missing jobs API
- [ ] Decide store ownership for selected job / refresh
- [ ] Implement `TopBar` controls
- [ ] Implement `JobsTab` selection + actions
- [ ] Implement `LogsTab` selected-job UX
- [ ] Normalize action error handling
- [ ] Run `npm run build`
- [ ] Complete review gate

## Review Findings

Pending.

## Root Cause

Pending.

## Repair Plan

Pending.

## Review Outcome

Pending.
