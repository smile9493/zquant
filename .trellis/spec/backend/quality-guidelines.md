# Quality Guidelines

> Code quality standards for backend development.

---

## Scope

These guidelines apply to Rust backend code under `apps/` and `crates/`.

## Testing Requirements

Minimum expectations for changes:

- Unit tests for pure logic and in-process components.
  - Example: `crates/job-events/src/bus.rs` tests publish/subscribe and stats.
  - Example: `crates/job-application/src/agent_supervisor.rs` tests spawn->schedule->message.

- E2E tests for Postgres-backed behavior where applicable.
  - Example: `crates/job-store-pg/tests/e2e_test.rs`.

## Validation Rule

Prefer the narrowest reliable validation:
1. targeted test
2. targeted package check
3. wider workspace checks only if justified

## Forbidden Patterns

- Hardcoding environment-specific values (ports/hosts) outside dev-only defaults.
- Introducing cross-process assumptions in Phase 1 (in-memory bus is process-local).
- Logging full request payloads by default.
- Swallowing DB errors silently in the store.

## Required Patterns

- Keep binaries thin; put orchestration in library crates.
- Store is the source of truth (SSOT): DB commit defines mainline success.
- Event bus is best-effort; publishing must not be part of the DB transaction.
- Use fencing (`lease_version`) for finalize/heartbeat-style operations.

## Code Review Checklist

- Does the change respect SSOT (Postgres) and best-effort bus semantics?
- Are logs structured with the required fields?
- Are tests present for new behavior?
- Are Windows dev/test pitfalls considered (ports, Docker, env vars)?
