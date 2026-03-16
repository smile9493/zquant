# Frontend Impeccable Spec Integration

## Goal

Integrate the Impeccable marketplace plugin into `.trellis` frontend development standards so designers/developers have a consistent, repeatable design-quality workflow.

## Scope

### In scope

- Create frontend spec directory baseline under `A:\zquant\.trellis\spec\frontend\` if missing.
- Add an explicit Impeccable guideline document with command usage, phase gates, anti-pattern constraints, and review checklist.
- Update frontend quality guideline to include Impeccable-driven review gate.
- Update frontend index to register the new guideline and statuses.

### Out of scope

- Source code/UI implementation changes in `web/`.
- Installing/upgrading marketplace plugin binaries.
- Rewriting backend specs.

## Non-goals

- Not turning Impeccable guidance into hard visual style mandates for all pages.
- Not requiring every command on every change; use scenario-based gate rules.

## Assumptions / Risks

- Team already has access to the Impeccable command set in their agent environment.
- Some commands may be unavailable in specific environments; fallback procedure must be defined.
- Existing frontend spec files are currently absent; minimal baseline docs may be required for index consistency.

## Acceptance Criteria

- [x] `A:\zquant\.trellis\spec\frontend\index.md` exists and lists Impeccable integration status.
- [x] `A:\zquant\.trellis\spec\frontend\impeccable-guidelines.md` defines command mapping (`/audit`, `/normalize`, `/polish`, etc.), when to use each, and anti-pattern guards.
- [x] `A:\zquant\.trellis\spec\frontend\quality-guidelines.md` contains a concrete Impeccable review gate and PASS/FAIL criteria.
- [x] Frontend index links are valid (no broken local links in frontend spec set).
- [x] Task PRD and `task.json` updated with final review state.

## Implementation Plan

1. Create/normalize frontend spec directory and index.
2. Draft Impeccable-specific guideline with executable workflow.
3. Update frontend quality guideline with review gate integration.
4. Validate local links and section completeness.
5. Record review result and finalize task state.

## Checklist

- [x] Create `A:\zquant\.trellis\spec\frontend\index.md`.
- [x] Create `A:\zquant\.trellis\spec\frontend\impeccable-guidelines.md`.
- [x] Create/update `A:\zquant\.trellis\spec\frontend\quality-guidelines.md`.
- [x] Run local markdown link check for frontend spec docs.
- [x] Update review notes/outcome and set task status.

## Review Notes

Implemented:
- Created `frontend` spec directory baseline and index.
- Added `impeccable-guidelines.md` with command matrix, usage scenarios, anti-pattern constraints, Trellis write-back rules, and fallback mode.
- Added frontend `quality-guidelines.md` with explicit Impeccable review gate and checklist.
- Added placeholder docs for `component-guidelines`, `hook-guidelines`, `state-management`, `type-safety`, and `directory-structure` so index links are valid.

Checks run:
- Frontend markdown local link validation script: `FRONTEND_LINK_CHECK_OK`.
- Manual content inspection for required sections and command coverage.

## Review Outcome

REVIEW: PASS
