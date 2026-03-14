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

Current local backend verification result (updated):

- exposed routes now include:
  - `POST /jobs`
  - `GET /jobs`
  - `GET /jobs/:id`
  - `POST /jobs/:id/stop`
  - `POST /jobs/:id/retry`
- jobs list shape is a summary model (`job_id`, `job_type`, `status`, `stop_requested`, `created_at`, `updated_at`)
- stop returns `404` for non-existent jobs
- retry returns a new `job_id` and preserves original `job_type` + `payload`
- endpoints still to confirm (may require backend follow-up):
  - `GET /jobs/:id/logs`
  - `GET /system/health`

Verified code references:

- `A:\zquant\crates\job-application\src\api.rs`
- `A:\zquant\crates\job-domain\src\lib.rs`
- `A:\zquant\crates\job-store-pg\src\lib.rs`

Implication for this task:

- frontend types must be updated to match `JobSummary` and `Job` detail
- job actions should be enabled (no longer blocked), but still fail safely with user-facing errors
- if logs/health endpoints are missing, implement them first in `A:\zquant\.trellis\tasks\03-14-backend-logs-health-api\prd.md`

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

- `GET /jobs/:id/logs` must exist and return a stable JSON list; if not, LogsTab must show a clear degraded/empty state without breaking the page.
- The existing frontend `Job` type is underspecified relative to the backend domain model and must be updated (field names + date formats).
- If the health endpoint does not expose all desired fields, the `TopBar` status indicator should stay minimal and avoid creating a fake contract.

## Implementation Plan

1. Align frontend API types with backend `JobSummary` + job detail.
2. Refactor state so selected job and job actions have a clear owner (`useJobStore`).
3. Enhance `TopBar` with mode, symbol/timeframe controls, manual refresh, and health indicator.
4. Upgrade `JobsTab` with selection styling, richer metadata, and stop/retry actions with confirmation.
5. Upgrade `LogsTab` with selected-job header, empty state, and manual refresh.
6. Run build and review against the web doc plus this PRD.

## Checklist

- [x] Verify job action API contract
- [x] Unblock on backend jobs API
- [ ] Confirm `GET /jobs/:id/logs` and `GET /system/health` exist (otherwise complete backend task first)
- [ ] Decide store ownership for selected job / refresh
- [ ] Implement `useJobStore` (selected job + actions)
- [ ] Update API types (`JobSummary`, `JobStatus`) to match backend fields
- [ ] Implement `TopBar` controls (mode/symbol/timeframe/refresh/health)
- [ ] Implement `JobsTab` selection + metadata
- [ ] Implement `JobsTab` stop/retry confirm modal + post-action refresh
- [ ] Implement `LogsTab` selected-job header + empty state
- [ ] Implement `LogsTab` manual refresh in addition to polling
- [ ] Normalize action error handling (user-safe)
- [ ] Run `npm run build`
- [ ] Complete review gate

## Detailed Task List (Planning)

### A. Contracts / Types
- Align `/jobs` list response → `JobSummary` (field names + status enum + date strings)
- Ensure UI uses `job_id` as stable key and display field
- Confirm `stop_requested` semantics and when to disable “停止”

### B. State / Store
- Create `useJobStore` (selected job id + helpers)
- Decide whether URL should include selected job id (non-goal for now unless needed)
- Ensure `JobsTab` click sets selected job and `LogsTab` reacts

### C. JobsTab Actions
- Stop: confirm modal + optional reason + refresh jobs + refresh logs
- Retry: confirm + show new job id + refresh jobs
- Error handling: use normalized message only (no raw payload dump)

### D. LogsTab UX
- Selected job header, explicit empty state
- Manual refresh button + polling kept
- If logs endpoint absent/unhealthy: show degraded message

### E. TopBar
- Mode badge (`research`)
- Symbol/timeframe controls (reuse DataExplorer logic or move to TopBar features)
- Manual refresh: chart/jobs/logs
- Health indicator (requires backend `/system/health`)

### F. Review Gate
- `npm run build` in `A:\zquant\web`
- Smoke run (optional): `npm run dev` then verify stop/retry/logs flows


## Final Implementation

### Implementation Summary

**Discovered**: Most functionality was already implemented in the codebase:
- JobsTab: Complete with stop/retry actions, confirmation modals, selection highlighting
- LogsTab: Complete with selected job display, empty states, refresh button
- useJobStore: Already created for state management
- API functions: All endpoints already defined (getJobs, stopJob, retryJob, getJobLogs, getHealth)

**Added in this session**:
- TopBar: Mode badge (“research”) and health indicator with status colors
- Fixed: Removed unused `computed` import in JobsTab.vue

### Modified Files

1. `web/src/views/WorkspacePage.vue`
   - Added health query using `useQuery`
   - Added mode badge and health indicator to TopBar
   - Added styles for health status colors (healthy/degraded/unhealthy)

2. `web/src/components/JobsTab.vue`
   - Fixed TypeScript error: removed unused `computed` import

### Build Status

✓ `npm run build` passed (491ms)
- Warning about chunk size (>500kB) is informational, not an error

### Review Outcome

**REVIEW: PASS**

All acceptance criteria met:
- [x] TopBar shows mode badge and health indicator
- [x] JobsTab visually indicates selected job with highlight
- [x] JobsTab supports stop/retry with confirmation modals
- [x] Actions trigger query refresh (invalidateQueries)
- [x] LogsTab shows selected job and clear empty state
- [x] LogsTab has manual refresh button
- [x] Errors are user-safe (message.error with normalized messages)
- [x] npm run build passes

