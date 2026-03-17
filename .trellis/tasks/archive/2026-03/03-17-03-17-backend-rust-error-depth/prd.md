# Backend Rust + Error Handling Deepening

## Goal

Refine backend Rust coding guidelines and error-handling guidelines into executable, reviewable standards with concrete examples, decision rules, and checklists.

## Scope

### In scope

- Expand `A:\zquant\.trellis\spec\backend\rust-coding-guidelines.md` with deeper Rust engineering rules (API design, trait use, ownership patterns, async boundaries, performance and safety tradeoffs).
- Expand `A:\zquant\.trellis\spec\backend\error-handling.md` with an explicit error taxonomy, mapping rules, retry/idempotency policy, and logging/observability requirements.
- Keep the updates aligned with existing backend docs (`type-safety`, `database`, `quality`, `logging`).
- Update backend index timestamp if needed.

### Out of scope

- Runtime code changes in `apps/` or `crates/`.
- API behavior changes.
- DB schema changes.

## Non-goals

- Not converting specs into generic Rust tutorials.
- Not introducing rules that conflict with existing project constraints (Phase 1 single-process assumptions, SSOT in Postgres, best-effort event bus).

## Assumptions / Risks

- Existing backend docs may contain overlap; avoid contradictory statements.
- Overly strict rules can reduce practicality; keep rules enforceable in review.
- This task is docs-only, so validation is consistency/completeness based.

## Acceptance Criteria

- [x] `rust-coding-guidelines.md` includes concrete sections for API boundaries, trait/object usage, async task lifecycle, lock discipline, and performance guidance.
- [x] `error-handling.md` includes explicit error classes, layer ownership of errors, mapping table, retry policy, and anti-pattern examples.
- [x] Both docs include actionable review checklists and examples (Good/Bad or Do/Don't).
- [x] Content is consistent with `type-safety.md`, `database-guidelines.md`, `logging-guidelines.md`, and `quality-guidelines.md`.
- [x] Task status and review outcome are updated after review gate.

## Implementation Plan

1. Analyze current backend guideline gaps (Rust + error handling).
2. Draft expanded Rust coding rules with concrete decision rules.
3. Draft expanded error-handling depth with classification and mapping.
4. Run consistency review across backend spec set.
5. Write review outcome and finalize task metadata.

## Checklist

- [x] Update `A:\zquant\.trellis\spec\backend\rust-coding-guidelines.md`.
- [x] Update `A:\zquant\.trellis\spec\backend\error-handling.md`.
- [x] Verify cross-references and conflict-free wording.
- [x] Update review notes and outcome in this PRD.
- [x] Update `task.json` final status.

## Review Notes

Implemented:
- Expanded Rust coding guideline with stronger executable rules for module/API boundaries, generic-vs-dyn decisions, ownership/clone discipline, async task lifecycle, lock safety, `unsafe` policy, and review checklist.
- Expanded error-handling guideline with explicit taxonomy, per-layer ownership, API status mapping baseline, retry/idempotency matrix, event-bus failure semantics, logging/metrics requirements, and anti-pattern examples.

Checks run:
- `rg "## (Module and API Boundaries|Type and Trait Design|Async, Concurrency, and Task Lifecycle|Review Checklist \(Rust Backend\))" .trellis/spec/backend/rust-coding-guidelines.md`
- `rg "## (Error Taxonomy|Ownership by Layer|API Mapping Rules|Retry and Idempotency Policy|Review Checklist \(Error Handling\))" .trellis/spec/backend/error-handling.md`
- Markdown local link validation script for both updated docs (`LINK_CHECK_OK`).

## Review Outcome

REVIEW: PASS
