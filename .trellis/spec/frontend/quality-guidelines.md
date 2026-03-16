# Quality Guidelines

> Code and design quality standards for frontend development.

---

## Scope

Applies to frontend changes under `web/` (Vue, TypeScript, styles, UI behavior).

---

## Baseline Technical Checks

For frontend changes, run the narrowest effective checks first:

1. Targeted tests for changed modules/components.
2. Type check for frontend workspace.
3. Build verification for integration confidence.

Prefer incremental checks over full-suite runs when scope is local.

---

## UI Quality Gate

A frontend task is not review-pass unless all are true:

- Functional requirement is met.
- Visual hierarchy/readability is acceptable.
- Interaction states are coherent (loading/empty/error/success).
- Responsive behavior is acceptable for intended viewport range.
- No unresolved critical design-quality findings remain.

---

## Impeccable Review Gate (Required for UI-facing tasks)

For tasks that change UI output, execute Impeccable flow from `impeccable-guidelines.md`:

- Required baseline: `/audit` then `/normalize` or `/polish` (depending on scope).
- Optional commands based on change focus (`/clarify`, `/harden`, `/adapt`, etc.).

In task PRD review notes, include:
- command list,
- applied findings,
- deferred/rejected items with rationale.

If commands are unavailable, document fallback and complete manual checklist before review pass.

---

## Manual UI Checklist (Fallback or Supplement)

- Typography hierarchy is clear and intentional.
- Color contrast and semantic color usage are consistent.
- Spacing rhythm is consistent with system tokens.
- Focus/hover/disabled/loading states are defined.
- Error copy is actionable and specific.
- Primary user path has no visual ambiguity.

---

## Forbidden Patterns

- Shipping UI changes without review notes.
- Ignoring severe a11y/contrast issues.
- Leaving placeholder copy in user-facing screens.
- Introducing custom one-off styles that bypass shared tokens/components without reason.

---

## Review Checklist (Frontend Quality)

- Were technical checks executed at the right scope?
- Was Impeccable review executed (or fallback documented)?
- Were design findings triaged and tracked?
- Are UI states complete for the changed surfaces?
- Are unresolved issues explicitly documented and non-blocking?
