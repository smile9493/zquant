# Logging Guidelines

> How logging is done in this project.

---

## Stack

- Logging library: `tracing`.
- Subscriber initialization: `tracing_subscriber` (or `crates/job-observability` if adopted consistently).

## Principles

- Prefer structured logs (fields) over string interpolation.
- Log lifecycle events at `info`.
- Log recoverable issues at `warn`.
- Log failures that skip work or lose state at `error`.

## Required Fields (Job lifecycle)

When logging runner lifecycle, include:
- `job_id`
- `job_type`
- `executor_id`
- `duration_ms` (on completion)
- `lease_until_ms` (on claim/start)

Examples exist in:
- `crates/job-application/src/runner.rs`
- `crates/job-application/src/api.rs`

## Sensitive Data

- Do not log secrets.
- Treat `payload` as potentially sensitive; do not log it verbatim.
- If you need observability, log a small, explicitly whitelisted subset of fields.

## Event Bus Observability

- Best-effort publish can fail when there are no subscribers; log at `warn`.
- Track counters where reasonable (e.g. publish totals, lagged receive totals).

Example: `crates/job-events/src/bus.rs`.
