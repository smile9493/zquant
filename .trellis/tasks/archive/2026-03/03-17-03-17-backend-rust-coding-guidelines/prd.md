# Backend Rust Coding Guidelines

## Goal

Add a dedicated backend Rust coding guideline document under `.trellis/spec/backend/` that is aligned with official Rust documentation and this repository's existing backend conventions.

## Scope

### In scope

- Add `rust-coding-guidelines.md` in `.trellis/spec/backend/`.
- Cover practical Rust coding rules for this codebase:
  - ownership/borrowing patterns
  - type design and trait usage
  - error handling (`anyhow`, domain errors, API boundary mapping)
  - async/concurrency patterns (`tokio`, cancellation, timeouts)
  - module visibility and API surface
  - testing and clippy/rustfmt expectations
  - docs/comments expectations
- Include official Rust references (Book, API Guidelines, Rust Reference, rustc/clippy docs).
- Update `.trellis/spec/backend/index.md` to include the new guide.

### Out of scope

- Refactoring existing Rust crates.
- Changing runtime behavior in apps/crates.
- Rewriting existing backend specs beyond index linkage.

## Non-goals

- Not creating a generic Rust tutorial.
- Not duplicating every rule already fully documented in existing backend files.
- Not introducing project policies unrelated to Rust backend development.

## Assumptions / Risks

- Existing backend docs in this directory are English-first; new guide should remain in English.
- Rule overlap with existing docs (error/logging/quality) may cause duplication; we should cross-reference instead of copying.
- Overly strict rules could conflict with existing code; guidance should define preferred patterns with justified exceptions.

## Acceptance Criteria

- [x] New file exists: `.trellis/spec/backend/rust-coding-guidelines.md`.
- [x] Guide references official Rust resources.
- [x] Guide content is consistent with existing backend docs (`error-handling`, `logging-guidelines`, `quality-guidelines`).
- [x] Backend index includes the new guide entry.
- [x] Document language and structure match `.trellis/spec/backend/` conventions.

## Implementation Plan

1. Inspect existing backend guideline style and avoid conflicts.
2. Draft Rust guideline sections using official sources as baseline.
3. Add explicit cross-references to project-specific backend docs.
4. Update backend index table.
5. Perform doc review pass for consistency and completeness.

## Checklist

- [x] Create `rust-coding-guidelines.md` with complete sections.
- [x] Add official Rust reference links in the guide.
- [x] Add/verify cross-references to existing backend specs.
- [x] Update `.trellis/spec/backend/index.md`.
- [x] Verify formatting, headings, and language consistency.
- [x] Run final review gate and record outcome.

## Review Notes

Implemented: added backend Rust coding guidelines and linked it from backend index. Verified guide file presence, official reference links, and index entry update.

## Review Outcome

REVIEW: PASS

