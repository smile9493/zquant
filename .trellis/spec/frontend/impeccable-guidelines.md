# Impeccable Guidelines

> Operational guideline for using the Impeccable marketplace plugin in frontend work.

---

## Purpose

Impeccable is used as a structured design-quality reviewer. It does not replace product requirements, design system tokens, or functional correctness checks.

Use it to improve:
- visual hierarchy and readability,
- interaction clarity,
- consistency and polish,
- a11y/performance-related UI quality.

---

## Scope

Applies to:
- page-level UI changes,
- reusable component changes,
- copy and UX flow changes,
- pre-release visual review.

Not required for:
- pure backend changes,
- non-visual refactors with no UI diff.

---

## Command Set (Primary)

### Baseline review flow

1. `/audit [optional-scope]`
2. `/normalize [optional-scope]`
3. `/polish [optional-scope]`

Use this as the default sequence for medium/large UI changes.

### Optional focused commands

- Content clarity: `/clarify`
- Robustness and edge cases: `/harden`
- Simplification: `/distill`
- Motion tuning: `/animate`
- Color strategy: `/colorize`
- Responsive adaptation: `/adapt`
- Component extraction: `/extract`

### Context setup command

- `/teach-impeccable` can be used once per new feature stream to provide design context.

---

## When to Use Which Command

| Scenario | Required Commands | Optional Commands |
|----------|-------------------|-------------------|
| New screen / large layout | `/audit` → `/normalize` → `/polish` | `/adapt`, `/distill`, `/colorize` |
| Existing page tune-up | `/audit` → `/polish` | `/clarify`, `/quieter`, `/bolder` |
| Reusable component update | `/audit` → `/normalize` | `/extract`, `/harden` |
| Release candidate visual QA | `/audit` → `/polish` | `/delight` |

---

## Hard Constraints (Anti-Patterns)

The following are prohibited unless there is a documented exception:

- Overusing default/system feel typography without hierarchy intent.
- Gray text on saturated/colorful backgrounds with low contrast.
- Pure black/gray palettes without hue nuance where readability suffers.
- Card-over-card nesting as default layout strategy.
- Bouncy/elastic easing patterns for standard product interactions.

If an exception is needed, add rationale in PR/task notes.

---

## Output Handling Rule

Impeccable output is guidance, not auto-acceptance.

Each finding must be triaged as one of:
- **Apply now**: clear quality benefit and no product conflict.
- **Defer**: valid idea but outside current scope.
- **Reject with reason**: conflicts with requirement, token system, or technical constraints.

Do not claim completion with unresolved P1 design-quality issues.

---

## Integration with Trellis Workflow

In the same task PRD:

- Add a short **Impeccable Review** section in Review Notes:
  - commands executed,
  - key findings,
  - what was applied/deferred/rejected.

For UI tasks, review gate should not be marked pass until:
- required Impeccable pass commands are executed (or explicitly waived),
- unresolved high-severity quality findings are resolved or documented with rationale.

---

## Minimal Review Template

```markdown
### Impeccable Review
- Commands: /audit workspace-page, /normalize jobs-tab, /polish jobs-tab
- Applied: improved spacing rhythm, tightened status badge contrast
- Deferred: animation micro-interactions (next iteration)
- Rejected: proposed color shift conflicts with brand token set
```

---

## Environment Fallback

If Impeccable commands are temporarily unavailable:

1. Run manual review checklist from `quality-guidelines.md`.
2. Mark review note as **Impeccable unavailable** with timestamp.
3. Execute Impeccable review in the next available iteration before release.

---

## Review Checklist (Impeccable)

- Was the baseline command flow executed for this change type?
- Were anti-pattern constraints checked?
- Were findings triaged (apply/defer/reject) with rationale?
- Are remaining issues non-blocking and documented?
- Is PRD review note updated with command evidence?
