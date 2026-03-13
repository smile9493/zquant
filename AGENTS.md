# AI Agent Instructions

This file contains mandatory rules for all AI assistants working in this project.

---

## Single Source of Truth: .trellis

All task management, planning, review results, and progress tracking MUST use the `.trellis/` system.

Do NOT create or update these files in the repository root:
- `task_plan.md`
- `findings.md`
- `progress.md`

Use these instead:
- Task planning → `.trellis/tasks/{task-name}/prd.md`
- Task tracking → `.trellis/tasks/{task-name}/task.json`
- Session notes → `.trellis/workspace/{developer}/journal-N.md`

For the same task, all planning, review findings, repair plans, and progress updates MUST stay inside the same Trellis task directory.

---

## Platform-Specific Commands

This project runs on Windows.

Always use `python` (not `python3`) when calling Trellis scripts or any Python commands in this project.

✅ Correct:
`python ./.trellis/scripts/task.py list`

❌ Wrong:
`python3 ./.trellis/scripts/task.py list`

---

## Session Start Rules

At the start of any non-trivial coding session:

1. Run `$start`
2. Read `.trellis/workflow.md`
3. Read relevant guidance in `.trellis/spec/` before coding

Key resources:
- Development workflow: `.trellis/workflow.md`
- Backend guidelines: `.trellis/spec/backend/index.md`
- Thinking guides: `.trellis/spec/guides/index.md`

If this repository defines a custom Trellis start alias, follow that alias. Otherwise use `$start`.

---

## Task Classification

Before doing any work, classify the request:

### A. Q&A only
If the user is only asking for explanation, analysis, or discussion, answer directly.
Do not create or modify a task unless the user asks for implementation.

### B. Trivial edit
A task may be treated as trivial only if all of the following are true:
- single-file or very small text-only change
- no logic change
- no API / schema / cross-layer impact
- no meaningful risk of regression

For trivial edits, a full Trellis task is optional.

### C. Development task
Anything else is a Development Task and MUST follow the full Trellis workflow below.

When unsure, treat the work as a Development Task.

---

## Mandatory Trellis Workflow for Development Tasks

For any Development Task:

1. Check whether there is already an active Trellis task matching the request.
2. If not, create a new task with the Trellis task script.
3. Set that task as the current task.
4. Before coding, update the task PRD:
   - goal
   - scope
   - non-goals
   - acceptance criteria
   - assumptions / risks
   - implementation plan
   - checklist of concrete steps
5. Only after the PRD is updated may implementation begin.

Do not start coding before the task plan exists in `.trellis/tasks/.../prd.md`.

---

## Required Task Document Behavior

The task PRD is the main planning and review document for the task.

For the same task, do NOT create a second planning document elsewhere.

If the plan changes during implementation, update the existing `prd.md`.

If review finds problems, append the findings and repair plan to the same `prd.md`.

The PRD should evolve with the task. It is not a one-time file.

---

## Implementation Rules

Before coding:
- read the relevant `.trellis/spec/` guidance
- align the implementation with repository conventions
- prefer the minimum safe change that satisfies the acceptance criteria

During coding:
- keep the task checklist updated
- record important deviations or discoveries in the task document
- do not silently change scope without updating the PRD

---

## Mandatory Review Gate

After implementation, do NOT immediately declare the task complete.

You MUST perform a review phase.

The review phase should include, when relevant:
- spec compliance review
- code correctness review
- cross-layer consistency review
- lint / type-check / test / build commands appropriate for the repo
- any Trellis review commands relevant to the change

Examples:
- backend change → run the backend review flow
- frontend change → run the frontend review flow
- API / schema / contract / type change → run cross-layer review

---

## Explicit Review Outcome

At the end of review, output exactly one of these:

- `REVIEW: PASS`
- `REVIEW: FAIL`

No other completion state is allowed.

---

## What PASS Means

Use `REVIEW: PASS` only if all of the following are true:
- acceptance criteria are satisfied
- required checks passed
- relevant specs were followed
- no unresolved review findings remain
- the task document reflects the final implementation state

If the review passes:
1. summarize what was implemented
2. summarize what was verified
3. mark the task finished using the Trellis workflow
4. archive the task if the workflow calls for archiving

---

## What FAIL Means

Use `REVIEW: FAIL` if any of the following is true:
- a required check failed
- acceptance criteria are incomplete
- the implementation conflicts with specs
- there are unresolved review findings
- documentation / task state is inconsistent with the actual code

If the review fails, you MUST do all of the following:

1. Do NOT mark the task complete
2. Do NOT create a separate findings file outside `.trellis/`
3. Append the failed review details to the same task `prd.md`
4. Add or update these sections in `prd.md`:
   - Review findings
   - Root cause
   - Repair plan
   - Updated checklist
5. Convert each review finding into one or more concrete repair tasks
6. Continue work against the same task
7. Re-run the review gate after repairs

A failed review must result in write-back to the existing task document.

---

## Review Write-Back Rule

If review fails, the repair work must be planned in the same task that originally introduced the change.

Do not fork the task into a new root-level plan.
Do not create `findings.md`.
Do not create `progress.md`.
Do not move the repair plan outside `.trellis/tasks/{task-name}/`.

The same task document must contain:
- original plan
- implementation updates
- review findings
- repair plan
- final pass state

---

## Chat Output Requirements

Before implementation, clearly state:
- whether you are reusing or creating a Trellis task
- the task path or task name
- the short implementation plan

After review, clearly state:
- `REVIEW: PASS` or `REVIEW: FAIL`

If FAIL, also state:
- the failed findings
- that they were written back into the existing task PRD
- the concrete repair tasks added

If PASS, also state:
- the checks run
- the acceptance criteria satisfied

---

## Priority Rules

When instructions conflict, follow this order:
1. explicit user request
2. nearest project-specific agent instructions
3. this `AGENTS.md`
4. personal/global Codex instructions

---

## Enforcement

These rules are mandatory for all AI agents.

Violations will result in:
- incorrect task tracking
- review findings being lost
- command execution failures on Windows
- drift away from the Trellis workflow

When in doubt, refer to:
- `.trellis/README.md`
- `.trellis/workflow.md`