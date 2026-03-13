# Web: Workspace MVP (Vue)

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`

## Goal

Deliver a **Vue single-page workspace MVP** aligned with Phase 1 backend constraints:

- Single entry: `/workspace`
- Chart-first center canvas
- Right dock: only two long-flow panels
- Bottom dock: only Jobs + Logs
- No WebSocket, no Redis/Kafka dependencies; use HTTP + polling/manual refresh
- URL-as-State for core workspace state

## Constraints / Assumptions

- Backend Phase 1 provides HTTP APIs for jobs/logs/health and market kline. If some endpoints are missing, this task will stub/mock and clearly track required backend follow-ups.
- Frontend app will live in a **non-Cargo** directory (e.g. `web/`) to avoid being treated as a Rust workspace member.

## Scope

### In scope (MVP)

**UI Shell**
- Route: `/workspace`
- Layout: TopBar + Left sidebar (minimal) + Center chart + Right dock + Bottom dock (tabs)

**Core widgets**
- `PriceChartPanel` (K-line + volume; lightweight-charts)
- `DataExplorerPanel` (datasource/dataset list, symbol search, timeframe switch)
- `GovernanceSummaryPanel` (mode, API health, last error; read-only)
- Bottom dock tabs:
  - `JobsTab` (list, select, basic actions if available)
  - `LogsTab` (job logs/system logs; pagination or virtual list)

**State**
- URL-as-State (query params): `symbol`, `timeframe`, `right`, `bottom`
- Minimal stores: workspace/jobs/datasource (Pinia), data fetching via vue-query

**API client**
- HTTP wrapper in `shared/api` with typed request/response and error normalization
- Polling for jobs/logs + explicit refresh

### Out of scope (explicit non-goals)

- WebSocket bridge
- Typed frontend event bus / gap detection
- Optimistic UI state machine
- Multi-chart grid / infinite canvas
- Agent multi-panel collaboration
- Cmd+K command system

## Acceptance Criteria

- [x] App has a single `/workspace` page with the target UI skeleton (TopBar/Center/Right/Bottom).
- [x] Center chart renders OHLCV for selected `symbol` + `timeframe` via HTTP API (or mock if backend not ready).
- [x] Right dock contains exactly `DataExplorerPanel` and `GovernanceSummaryPanel` (no extra MVP panels).
- [x] Bottom dock contains exactly `JobsTab` and `LogsTab`.
- [x] URL query params round-trip:
  - changing symbol/timeframe updates URL
  - loading URL restores UI state
- [x] No payload dumps in logs; errors are user-safe and actionable.
- [x] Minimal build/run instructions are documented in this PRD.
- [x] Review gate completed with `REVIEW: PASS` or `REVIEW: FAIL` recorded here.

## Proposed Tech Stack

- Vue 3 + TypeScript + Vite
- Vue Router
- Pinia
- `@tanstack/vue-query`
- UI: Ant Design Vue (preferred)
- Charts: `lightweight-charts`

## Minimal API Surface (expected)

- Market:
  - `GET /api/market/kline`
- DataSource:
  - `GET /api/datasources`
  - `GET /api/datasets`
- Jobs:
  - `POST /jobs`
  - `GET /jobs`
  - `GET /jobs/:id`
  - `POST /jobs/:id/stop` (optional)
  - `POST /jobs/:id/retry` (optional)
- Logs/Health:
  - `GET /jobs/:id/logs`
  - `GET /system/health`

## Implementation Plan

1. Scaffold `web/` as Vite Vue TS project; add router/pinia/vue-query.
2. Implement `/workspace` layout (grid + docks) and visual tokens (dark neo-glass lite).
3. Implement `PriceChartPanel` with `symbol/timeframe` props and data hook.
4. Implement `JobsTab` + `LogsTab` with polling + error states.
5. Implement `DataExplorerPanel` + `GovernanceSummaryPanel`.
6. Add URL-as-State synchronization (router query <-> store).
7. Add basic smoke tests / lint checks available in this repo; run review gate.

## Build & Run Instructions

```bash
# Install dependencies
cd web
npm install

# Run development server
npm run dev

# Build for production
npm run build
```

## Checklist

- [x] Decide frontend root dir and tooling (`web/`)
- [x] Scaffold app + dependencies
- [x] Implement workspace shell + styling
- [x] Implement chart widget
- [x] Implement right dock panels
- [x] Implement bottom dock tabs
- [x] Implement URL-as-State sync
- [x] Add basic checks + review gate

## Review Findings

### [P1] Frontend build is currently broken

The task cannot pass review because the app does not build. `npm run build` fails in `PriceChartPanel.vue` with a type-only import error for `IChartApi` and a missing `addCandlestickSeries` method on the inferred chart type. Until the frontend builds, none of the MVP acceptance criteria can be considered complete.

Affected areas:
- `A:\zquant\web\src\components\PriceChartPanel.vue`

### [P1] API client paths do not match the task's documented API surface

The PRD defines `GET /api/market/kline` and `GET /jobs/:id/logs` as the expected MVP endpoints, but the implementation calls `/price/${symbol}/${timeframe}` and `/logs` instead. This is a contract mismatch that will break the chart/logs panels against the documented backend surface.

Affected areas:
- `A:\zquant\web\src\shared\api\index.ts`
- this PRD API contract

### [P1] The chart panel still uses mock data and never calls the HTTP API

`PriceChartPanel` generates random candlesticks locally and leaves symbol/timeframe refresh as a TODO. The task acceptance explicitly requires the center chart to render OHLCV via HTTP API (or, if mocked, to record that decision and backend follow-up). The current implementation does neither.

Affected areas:
- `A:\zquant\web\src\components\PriceChartPanel.vue`
- this PRD acceptance state

### [P2] URL-as-State is incomplete relative to the MVP contract

The PRD requires query params for `symbol`, `timeframe`, `right`, and `bottom`, but the implementation only round-trips `symbol`, `timeframe`, and `bottom`. There is no restoration or sync for the right dock state, and no minimal user controls in the shell to change the key URL-driven values.

Affected areas:
- `A:\zquant\web\src\views\WorkspacePage.vue`
- `A:\zquant\web\src\stores\workspace.ts`

## Root Cause

- The current frontend is closer to a visual scaffold than a completed MVP, but the task was presented for review as if the API/data-path work had been finished.
- The implementation diverged from the task PRD's own HTTP contract and did not re-align the document when using placeholders or mocks.
- Validation appears to have skipped the frontend build step, which left compile-time issues undiscovered before review.

## Repair Plan

1. Fix `PriceChartPanel.vue` so the frontend builds cleanly with the installed `lightweight-charts` typings/API.
2. Align `shared/api/index.ts` with the PRD endpoint contract, or explicitly update the PRD to the real backend API after verifying it.
3. Replace chart mock data with actual HTTP-backed loading, or document/mock this explicitly in the PRD together with the backend follow-up.
4. Complete URL-as-State for `right` (and any required controls needed to exercise the route state).
5. Re-run `npm run build` plus any available frontend checks before the next review.

## Review Outcome

### REVIEW: FAIL

The workspace MVP is not review-complete because the frontend does not build and key PRD/API contract items are still unmet.

---

## Review Findings (Round 2)

### [P1] `LogsTab` is still disconnected from the selected job and cannot satisfy `/jobs/:id/logs`

The implementation still hard-codes `selectedJobId = 'latest'` and leaves a TODO to wire job selection later. That means the logs panel is not actually driven by `JobsTab`, does not satisfy the documented `GET /jobs/:id/logs` contract, and cannot reliably show task logs for the job the user selected.

Affected areas:
- `A:\zquant\web\src\components\LogsTab.vue`

### [P1] The `right` query param is written to the URL but does not control the right dock UI

`WorkspacePage.vue` now reads and writes `right`, but the view always renders both `DataExplorerPanel` and `GovernanceSummaryPanel` unconditionally. In other words, `right` is only persisted, not applied. This still fails the URL-as-State requirement because loading a URL with a different `right` value does not produce a different UI state.

Affected areas:
- `A:\zquant\web\src\views\WorkspacePage.vue`
- `A:\zquant\web\src\stores\workspace.ts`

### [P1] `DataExplorerPanel` is still a placeholder and does not implement the MVP interactions

The task and source document require datasource/dataset exploration, symbol search, and timeframe switching in the right dock. The current panel only echoes `symbol` and `timeframe` props and makes no API calls to `/api/datasources` or `/api/datasets`. This leaves a core MVP panel effectively unimplemented.

Affected areas:
- `A:\zquant\web\src\components\DataExplorerPanel.vue`
- `A:\zquant\web\src\shared\api\index.ts`

### [P2] `GovernanceSummaryPanel` is still static and does not consume health or error state

The source document defines this panel as a read-only summary of current mode, API health, and the latest error. The current implementation renders only a static `Status: Active` string and never calls `/system/health` or consumes any last-error state from the API layer/store.

Affected areas:
- `A:\zquant\web\src\components\GovernanceSummaryPanel.vue`
- `A:\zquant\web\src\shared\api\index.ts`

### [P2] API error handling still dumps the raw error object

The acceptance criteria explicitly require user-safe, actionable errors and no payload dumps in logs. `apiClient` still calls `console.error('API Error:', error)`, which forwards the raw Axios error object to the console. That is not normalized frontend error handling and can leak request/response details.

Affected areas:
- `A:\zquant\web\src\shared\api\client.ts`

## Root Cause (Round 2)

- The work fixed the original compile/runtime path issues, but review stopped short of checking whether the newly added state and endpoints are actually wired into the UI behavior.
- Several components are still shell placeholders while the task was presented as a completed MVP.
- The PRD and task metadata were not updated to reflect which widgets are still partial, so the claimed completion state drifted from the actual implementation state.

## Repair Plan (Round 2)

1. Add a real selected-job state path (`JobsTab` -> store -> `LogsTab`) and stop calling `/jobs/:id/logs` with a placeholder ID.
2. Make `right` an actual UI state by wiring it to right-dock rendering and adding a minimal control to switch panels.
3. Implement the minimum `DataExplorerPanel` behavior required by the PRD: datasource/dataset fetch, symbol input, and timeframe switching.
4. Implement `GovernanceSummaryPanel` against real mode/health/error data, or explicitly narrow the PRD if that API is not available.
5. Replace raw `console.error(error)` handling with normalized frontend error mapping that keeps console output and UI messages payload-safe.
6. Re-run `npm run build` and re-review against the PRD after the placeholder components are replaced or the scope is formally reduced.

## Updated Checklist

- [x] Fix lightweight-charts v5 build breakage
- [x] Align kline/logs API paths with the documented HTTP contract
- [x] Replace chart mock data with HTTP loading
- [ ] Wire job selection into `LogsTab`
- [ ] Make `right` query state drive the right dock UI
- [ ] Implement `DataExplorerPanel` MVP interactions and data fetching
- [ ] Implement `GovernanceSummaryPanel` MVP data path
- [ ] Normalize API error handling to avoid raw payload dumps
- [ ] Update task metadata and PRD acceptance state after the above is verified

## Review Outcome (Round 2)

### REVIEW: FAIL

The build issue is fixed, but the task still does not satisfy the documented MVP because key right-dock and logs behaviors remain placeholders, and raw API errors are still dumped in the frontend console.

---

## Review Findings (Round 3)

### [P1] `DataExplorerPanel` still does not implement datasource/dataset exploration

The panel now edits `symbol` and `timeframe`, but it still does not fetch or render `/api/datasources` or `/api/datasets`, which are explicitly part of both the source document and this task PRD. The current implementation is still a control stub rather than the required data exploration panel.

Affected areas:
- `A:\zquant\web\src\components\DataExplorerPanel.vue`
- `A:\zquant\web\src\shared\api\index.ts`

### [P2] `GovernanceSummaryPanel` is only a health badge, not the required governance summary

This panel now queries `/system/health`, which resolves part of the previous finding, but it still omits the other required fields from the PRD/document: current mode and latest error. As implemented, it is a health indicator, not a governance summary.

Affected areas:
- `A:\zquant\web\src\components\GovernanceSummaryPanel.vue`

### [P2] There is still no in-app control to switch the `right` dock state

`WorkspacePage.vue` now applies the `right` query parameter, but there is no UI control anywhere in the shell to change `rightPanel`. In practice, the governance panel is only reachable by manually editing the URL, which is below MVP usability for a documented workspace state dimension.

Affected areas:
- `A:\zquant\web\src\views\WorkspacePage.vue`
- `A:\zquant\web\src\stores\workspace.ts`

### [P2] The PRD still lacks the promised minimal build/run instructions

The acceptance criteria require minimal build/run instructions to be documented in this PRD, but the document still contains no concrete commands such as install/build/dev steps or required environment variables. This means the task cannot pass its own documentation gate yet.

Affected areas:
- `A:\zquant\.trellis\tasks\03-14-web-workspace-mvp-vue\prd.md`

## Root Cause (Round 3)

- The remaining work is no longer about compile errors; it is about closing the gap between "basic shell works" and the stronger MVP contract written in the task and source doc.
- The implementation is converging incrementally, but the task was presented for review before the right-dock features and documentation acceptance items were fully closed.

## Repair Plan (Round 3)

1. Add datasource and dataset fetch methods to `shared/api` and render a minimal list in `DataExplorerPanel`.
2. Extend `GovernanceSummaryPanel` to show at least `mode`, `health`, and `last error`, with a small shared source of truth for the latest API error.
3. Add a visible in-app control to switch `rightPanel` so the `right` query state is user-driven rather than URL-only.
4. Document minimal frontend run/build instructions in this PRD.
5. Re-run `npm run build` after the above and re-review the task against the PRD.

## Updated Checklist (Round 3)

- [x] Fix lightweight-charts v5 build breakage
- [x] Align kline/logs API paths with the documented HTTP contract
- [x] Replace chart mock data with HTTP loading
- [x] Wire job selection into `LogsTab`
- [x] Normalize API error handling to avoid raw payload dumps
- [ ] Make `right` query state user-driven in the workspace UI
- [ ] Implement `DataExplorerPanel` datasource/dataset MVP
- [ ] Implement `GovernanceSummaryPanel` mode + last-error MVP
- [ ] Add minimal build/run instructions to the PRD
- [ ] Update task metadata and PRD acceptance state after the above is verified

## Review Outcome (Round 3)

### REVIEW: FAIL

The frontend now builds and the logs flow is wired, but the task still falls short of the documented MVP because the right-dock panels are not fully implemented and the PRD documentation gate remains open.

---

## Review Findings (Round 4)

No blocking findings. The remaining issues from Round 3 are resolved:

- `DataExplorerPanel` now fetches and renders datasource/dataset lists while keeping symbol and timeframe controls.
- `GovernanceSummaryPanel` now surfaces health plus optional `mode` and `last_error`.
- `WorkspacePage` now provides visible controls to switch the `right` dock state, and the URL stays in sync.
- This PRD now includes minimal install/dev/build instructions.

## Verification (Round 4)

- `npm run build` in `A:\zquant\web`: PASS
- Code review confirmed:
  - `/workspace` route exists
  - chart uses HTTP-backed kline loading
  - right dock is limited to `DataExplorerPanel` and `GovernanceSummaryPanel`
  - bottom dock is limited to `JobsTab` and `LogsTab`
  - URL query sync covers `symbol`, `timeframe`, `right`, `bottom`
  - API error handling logs normalized messages instead of raw error payloads

## Review Outcome (Round 4)

### REVIEW: PASS

The workspace MVP now satisfies the documented acceptance criteria and passes the required build/review gate.
