# AI Agent Instructions

This file contains mandatory rules for all AI assistants working in this project.

---

## Single Source of Truth: .trellis

**Rule**: All task management, planning, and progress tracking MUST use the `.trellis/` system.

**Forbidden**: Do NOT create or update these files in the repository root:
- `task_plan.md`
- `findings.md`
- `progress.md`

**Why**: These files are deprecated. The project now uses `.trellis/tasks/` as the single source of truth.

**What to use instead**:
- Task planning → `.trellis/tasks/{task-name}/prd.md`
- Task tracking → `.trellis/tasks/{task-name}/task.json`
- Session notes → `.trellis/workspace/{developer}/journal-N.md`

---

## Platform-Specific Commands

**Windows Environment**: This project runs on Windows. Use the correct Python command:

✅ **Correct**: `python ./.trellis/scripts/task.py list`
❌ **Wrong**: `python3 ./.trellis/scripts/task.py list`

**Rule**: Always use `python` (not `python3`) when calling Trellis scripts or any Python commands in this project.

---

## Trellis Workflow

**Starting a session**:
1. Use `/trellis:start` command to initialize
2. Read `.trellis/workflow.md` for detailed workflow
3. Read relevant guidelines in `.trellis/spec/` before coding

**Key resources**:
- Development workflow: `.trellis/workflow.md`
- Backend guidelines: `.trellis/spec/backend/index.md`
- Thinking guides: `.trellis/spec/guides/index.md`

---

## Enforcement

These rules are mandatory for all AI agents. Violations will result in:
- Incorrect task tracking
- Command execution failures on Windows
- Confusion between deprecated and current systems

When in doubt, refer to `.trellis/README.md` and `.trellis/workflow.md`.
