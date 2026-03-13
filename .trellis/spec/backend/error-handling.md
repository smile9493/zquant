# Error Handling

> How errors are handled in this project.

---

## General Approach

- Internal Rust code typically uses `anyhow::Result<T>` and propagates with `?`.
- Background loops (runner/supervisor) should not crash the process on recoverable errors.
  - Log and continue.

## API Layer

- HTTP handlers map internal failures to `StatusCode::INTERNAL_SERVER_ERROR`.
- Do not leak internal error details to clients by default.

Example: `crates/job-application/src/api.rs`.

## Runner / Supervisor Loops

- Treat the event bus as best-effort:
  - publish failures should not roll back DB writes.
  - subscribe lag is expected; log a warning and rely on polling where applicable.

- Handler execution isolation:
  - Execute work in a spawned task so panics are isolated.
  - Convert join errors (panic/cancel) into `JobStatus::Error`.
  - Apply a timeout to prevent stuck handlers.

Example: `crates/job-application/src/runner.rs`.

## Database Errors

- Prefer returning `anyhow` errors upward from store methods.
- For known expected DB races (idempotency), handle them explicitly (unique violation 23505).

Example: `crates/job-store-pg/src/lib.rs`.
