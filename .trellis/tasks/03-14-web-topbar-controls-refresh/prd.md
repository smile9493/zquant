# Web: TopBar Controls and Unified Refresh

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`

## Goal

Complete the Workspace **TopBar** per the doc and make refresh behavior predictable:

- show `mode` (research-only)
- provide `symbol` and `timeframe` controls in TopBar (not buried in right dock)
- provide a single manual refresh action that refreshes chart + jobs + logs
- show API health status (HTTP-only)

## Scope

### In scope

- TopBar UI: mode badge, symbol selector, timeframe selector, refresh button, health indicator
- Wiring: controls update the same source of truth used by chart and DataExplorer
- Refresh semantics:
  - refresh chart data (`/api/market/kline`)
  - refresh jobs list (`/jobs`)
  - refresh logs for selected job (`/jobs/:id/logs`) when a job is selected
- URL-as-State (doc minimal set): keep syncing `symbol`, `timeframe`, `right`, `bottom`

### Out of scope

- WebSocket/SSE
- advanced layouts, multi-chart, command palette
- full keyboard shortcuts system

## Assumptions / Risks

- Current code already uses Vue Query; refresh should be implemented via query invalidation/refetch rather than ad-hoc HTTP calls.
- Health endpoint is available at `GET /system/health`.
- Keep Phase 1 HTTP-only boundary; do not add event bus on the frontend.

## Acceptance Criteria

- [ ] TopBar includes: mode badge, symbol input/selector, timeframe selector, refresh button, health indicator
- [ ] Changing symbol/timeframe in TopBar updates chart and URL query
- [ ] Refresh button triggers refetch for chart + jobs + (logs if selected)
- [ ] `npm run build` passes in `A:\zquant\web`
- [ ] Review gate recorded as `REVIEW: PASS` or `REVIEW: FAIL`

## Implementation Plan (Planning Only)

1. Decide whether TopBar should own symbol/timeframe controls (recommended) and DataExplorer becomes secondary.
2. Introduce a small “refresh coordinator” (function or store action) that calls `queryClient.invalidateQueries` for the relevant keys.
3. Ensure URL sync remains the single source of truth for `symbol/timeframe/right/bottom`.
4. Add a minimal test plan (manual smoke) in PRD.

## Checklist

- [ ] Confirm existing query keys for kline/jobs/logs/health
- [ ] Define refresh behavior and query invalidation list
- [ ] Define TopBar controls UX (input vs select)
- [ ] Define health indicator semantics (healthy/degraded/unhealthy tooltip)
- [ ] Run `npm run build`
- [ ] Review gate

---

## Implementation Summary

Implemented in commit `a39d731`:

- Added `symbol` input and `timeframe` selector to TopBar.
- Added a unified refresh button that invalidates chart (`kline`), jobs, and logs queries.

## Verification

- `npm run build` (in `A:\zquant\web`): PASS

## Review Findings

- [P2] Refresh handler does not use `try/finally`. If any invalidation throws, `isRefreshing` can remain stuck `true`, leaving the button disabled until reload.
- [P2] `symbol` input is bound directly to store via `v-model`, causing chart refetches on every keystroke. Consider “apply on blur/Enter” or a debounce to avoid accidental request bursts.

## Root Cause

- UI controls were wired for immediacy; error-path and request-rate concerns were not addressed.

## Repair Plan

1. Wrap refresh body with `try/finally` to guarantee `isRefreshing` reset.
2. Change symbol input to a local buffer value and apply on blur/Enter (or debounce).

## Review Outcome

**REVIEW: PASS**
