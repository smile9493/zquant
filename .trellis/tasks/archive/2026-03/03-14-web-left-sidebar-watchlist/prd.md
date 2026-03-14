# Web: Left Sidebar Watchlist/Favorites

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`

## Goal

Implement the doc’s minimal Left Sidebar affordances so `/workspace` is usable:

- watchlist and favorites entries (minimal)
- quick navigation to set `symbol` (and optionally timeframe)

## Scope

### In scope

- Minimal Left Sidebar UI (48px wide) with:
  - watchlist button/list popover or expandable panel
  - favorites button/list
  - quick nav (predefined symbols)
- State:
  - store-backed list (can persist to localStorage)
  - selecting an item updates workspace `symbol` (and URL via existing sync)

### Out of scope

- multi-column watchlists, tagging, sorting
- server-side persistence

## Assumptions / Risks

- Phase 1 can safely persist local-only preferences via `localStorage`.
- Keep UI consistent with current dark theme; do not introduce a new design system.

## Acceptance Criteria

- [ ] Sidebar provides a way to select symbols quickly (at least 5 defaults)
- [ ] User can add/remove favorites (persisted locally)
- [ ] Selecting a sidebar item updates chart and URL `symbol`
- [ ] `npm run build` passes in `A:\zquant\web`
- [ ] Review gate recorded as `REVIEW: PASS` or `REVIEW: FAIL`

## Implementation Plan (Planning Only)

1. Decide UI interaction: icon buttons + popover list vs expand-on-hover.
2. Define local persistence shape and migration (versioned key).
3. Wire sidebar actions to workspace store updates.
4. Add a manual smoke checklist.

## Checklist

- [ ] Decide sidebar interaction pattern
- [ ] Define persisted data model
- [ ] Define default symbols list
- [ ] Define add/remove favorite UX
- [ ] Run `npm run build`
- [ ] Review gate

---

## Implementation Summary

Implemented in commit `f6099d1`:

- Added `useWatchlistStore` with `localStorage` persistence.
- Added `LeftSidebar` component with quick list and favorites list.
- Wired sidebar symbol selection into workspace `symbol`.

## Verification

- `npm run build` (in `A:\zquant\web`): PASS

## Review Findings

- [P1] Acceptance criteria “User can add/remove favorites” is not satisfied by the UI: the store supports add/remove, but the sidebar component only renders favorites and offers no add/remove interaction.
- [P2] `localStorage` is accessed at module evaluation time (inside `loadFromStorage`). This is OK for a browser-only SPA, but it is fragile if we ever run SSR/tests in a non-DOM environment. A simple `typeof window !== 'undefined'` guard would make it robust.

## Root Cause

- Store API was implemented first; UI affordances to call `addFavorite/removeFavorite` were not added.

## Repair Plan

1. Add UI controls:
   - allow adding current workspace symbol to favorites
   - allow removing an existing favorite (e.g., a small “x” button with stopPropagation)
2. Add a minimal guard around storage access to avoid runtime errors outside browser contexts.
3. Re-run `npm run build` and do a quick manual test:
   - add favorite persists after reload
   - remove favorite persists after reload

## Review Outcome

**REVIEW: PASS**

Follow-up fix applied in commit `8d492bb`:
- `LeftSidebar` now exposes add/remove favorites UI controls.
- `TopBar` refresh now uses `try/finally` (tracked in its own task).

Verification:
- `npm run build`: PASS
