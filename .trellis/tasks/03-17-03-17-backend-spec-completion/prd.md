# Backend Spec Completion

## Goal

Complete missing backend specification items under `.trellis/spec/backend/` to match the repository's spec-writing guide and current backend development workflow.

## Scope

### In scope

- Add missing backend guideline file: `.trellis/spec/backend/type-safety.md`.
- Update `.trellis/spec/backend/index.md` to include the new guideline.
- Normalize backend index statuses from generic `Drafted` to actionable state labels used by the team (`Filled` / `To fill`).
- Keep existing backend guidelines consistent and cross-linked.

### Out of scope

- Frontend spec completion.
- Any code/runtime changes in `apps/` or `crates/`.
- Large rewrites of existing backend guideline content.

## Non-goals

- Not turning backend specs into language tutorials.
- Not introducing policy that conflicts with existing repository conventions.

## Assumptions / Risks

- Specs in `.trellis/spec/backend/` are maintained in English.
- Existing docs may have overlap; prefer references over duplication.
- Status relabeling must reflect actual content quality and may require later refinement.

## Acceptance Criteria

- [x] `.trellis/spec/backend/type-safety.md` exists with concrete, project-usable rules.
- [x] New type-safety guide is listed in `.trellis/spec/backend/index.md`.
- [x] Backend index status table uses `Filled` / `To fill` consistently.
- [x] New guideline references existing backend docs where relevant.
- [x] Task PRD and task.json are updated to final review state.

## Implementation Plan

1. Inspect current backend spec files and index format.
2. Draft backend `type-safety.md` with practical Rust patterns and anti-patterns.
3. Update backend index entries and statuses.
4. Run a doc review pass for consistency and completeness.
5. Record final review outcome.

## Checklist

- [x] Add `.trellis/spec/backend/type-safety.md`.
- [x] Add type-safety row into backend index.
- [x] Update backend index status labels.
- [x] Verify cross-links and language consistency.
- [x] Update task docs and review outcome.

## Review Notes

Implemented: added `.trellis/spec/backend/type-safety.md`, updated `.trellis/spec/backend/index.md` (guide statuses + type-safety entry + contract section), and verified links/file presence.

## Review Outcome

REVIEW: PASS

