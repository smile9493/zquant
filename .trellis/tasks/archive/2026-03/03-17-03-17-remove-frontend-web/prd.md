# Remove Frontend (Delete `web/`)

## Goal

Remove the entire frontend application from this repo, leaving only the Rust backend + Trellis system.

User decision: **delete `web/` directory completely**.

## Scope

### In scope

- Delete `web/` directory from the repository.
- Remove `.trellis/spec/frontend/` (frontend spec set), since the project no longer ships a frontend.
- Update any repo docs/workflow references that assume `web/` exists.

### Out of scope

- Any backend behavior changes.
- Any refactor of Rust crates/apps.

## Risks

- This is destructive to the working tree. Git history will retain old frontend, but the main branch will no longer contain `web/`.
- Some docs/scripts/CI may reference `web/` and need updates.

## Acceptance Criteria

- [x] `web/` directory removed.
- [x] `.trellis/spec/frontend/` removed.
- [x] No remaining references in docs/workflow that require `web/` to exist.
- [x] Backend still builds/tests (targeted check).
- [x] Task status updated and review gate recorded.

## Implementation Plan

1. Delete `web/`.
2. Delete `.trellis/spec/frontend/`.
3. Search and remove/update references to `web/` and frontend specs.
4. Run backend check (`cargo check` at minimum).
5. Review and finalize task.

## Checklist

- [x] Delete `web/`.
- [x] Delete `.trellis/spec/frontend/`.
- [x] Update `.trellis/workflow.md` to not require frontend specs.
- [x] Update `.trellis/spec/backend/index.md` notes if needed.
- [x] Update `.claude/agents/implement.md` spec references.
- [ ] Update root docs (`README.md`, `QUICKSTART.md`) if they reference `web/`. (No matches.)
- [x] Run `cargo check`.
- [x] Update `task.json` + mark `REVIEW: PASS/FAIL`.

## Review Notes

- Deleted tracked frontend app via `git rm -r web` and removed untracked remnants via `git clean -fdx web`.
- Removed frontend spec directory via `git rm -r .trellis/spec/frontend`.
- Updated references:
  - `.trellis/workflow.md` (removed frontend-spec requirements)
  - `.trellis/spec/backend/index.md` (removed frontend note)
  - `.claude/agents/implement.md` (removed frontend spec reference)
  - `.trellis/tasks/00-bootstrap-guidelines/*` (removed frontend bootstrap section)
- Verification:
  - `cargo check --workspace` PASS (warns about future Rust incompatibilities in dependencies; not addressed here)

## Review Outcome

REVIEW: PASS
