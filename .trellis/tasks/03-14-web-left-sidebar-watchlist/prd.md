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

