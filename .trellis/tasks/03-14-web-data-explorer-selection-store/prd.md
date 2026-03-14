# Web: Data Explorer Selection + DataSource Store

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`

## Goal

Make `DataExplorerPanel` a real “manager view” tool, with a dedicated store:

- introduce `useDataSourceStore` (doc-required)
- add datasource + dataset selection (not just list rendering)
- keep selection state consistent across right dock and chart use cases

## Scope

### In scope

- Store: `useDataSourceStore`
  - datasources list (`GET /api/datasources`)
  - datasets list (`GET /api/datasets`)
  - selected datasource id (optional) and selected dataset id
  - simple filters (market/type) if already represented by backend payloads
- UI: `DataExplorerPanel`
  - selectable list (click selects, highlight)
  - clear empty/loading/error states
  - optional search box for symbol (doc mentions)
- Integration:
  - selection does not need to be encoded into URL in Phase 1 (unless required)
  - selection should be available to other panels via store

### Out of scope

- building a full catalog browser with pagination
- advanced filtering UI unless backend supports it cleanly
- provider-level details; keep it manager-oriented

## Assumptions / Risks

- Backend `GET /api/datasources` and `GET /api/datasets` exist and return stable shapes.
- Current frontend `DataSource`/`DataSet` types are minimal; may need to expand once backend fields are known.

## Acceptance Criteria

- [ ] `useDataSourceStore` exists and is the single source of truth for selected datasource/dataset
- [ ] `DataExplorerPanel` supports selecting a datasource and dataset (with visible selection state)
- [ ] Loading/empty/error states are explicit and user-safe
- [ ] `npm run build` passes in `A:\zquant\web`
- [ ] Review gate recorded as `REVIEW: PASS` or `REVIEW: FAIL`

## Implementation Plan (Planning Only)

1. Confirm the backend response shapes for `/api/datasources` and `/api/datasets` (fields and identifiers).
2. Define the store state and actions; decide which selections are persisted (session-only vs localStorage).
3. Update `DataExplorerPanel` to render selectable lists, not plain text.
4. Add a minimal manual test checklist.

## Checklist

- [ ] Confirm API response shapes and required fields
- [ ] Define store state + actions + selection rules
- [ ] Define UI selection affordances (active state + clear selection)
- [ ] Validate non-goals (no provider details)
- [ ] Run `npm run build`
- [ ] Review gate

