# Error Handling

> Backend error-handling policy for this project.

---

## Scope

Applies to Rust backend code under `apps/` and `crates/`, including API handlers, runner/supervisor loops, provider adapters, and persistence layers.

This guide complements:
- `rust-coding-guidelines.md`
- `type-safety.md`
- `database-guidelines.md`
- `logging-guidelines.md`
- `quality-guidelines.md`

---

## Core Policy

1. **Classify errors explicitly** (domain / validation / infra-transient / infra-permanent / external).
2. **Handle errors at the correct layer** (where decision context exists).
3. **Preserve cause chain** for operators and reviewers.
4. **Do not leak internals** to external API responses.
5. **Retry intentionally, not blindly**.

---

## Error Taxonomy

### 1) Domain errors

Business rule violation or invalid state transition.

Examples:
- Retry requested for non-terminal job.
- Finalize called with stale lease version.

Handling:
- Return typed domain error.
- Map to deterministic status (often 4xx or controlled no-op).

### 2) Validation errors

Input shape or semantic validation failures.

Examples:
- Missing required field.
- Invalid enum string/value range.

Handling:
- Reject early at boundary.
- Include field-level reason in response (without internals).

### 3) Infra-transient errors

Temporary failures that can recover.

Examples:
- DB timeout / connection pool exhaustion.
- Temporary network failure to provider.

Handling:
- Mark retryable.
- Backoff + jitter where applicable.
- Log warning with retry metadata.

### 4) Infra-permanent errors

Likely unrecoverable in current request path.

Examples:
- SQL syntax mismatch with schema.
- Corrupt persisted data violating contract.

Handling:
- Fail fast.
- Emit error-level log and alert-relevant metrics.

### 5) External dependency errors

Failure from external systems (provider/script/subprocess).

Handling:
- Preserve adapter-provided reason where possible.
- Normalize output into internal error shape.
- Decide retry based on classified reason, not generic failure.

---

## Ownership by Layer

| Layer | Primary responsibility | Output form |
|------|-------------------------|-------------|
| API boundary | Validate request and map response status | Stable client response + minimal safe message |
| Application/orchestration | Decide retry/degrade/reject and state transition | Context-rich internal error |
| Store/repository | Return DB errors with operation context | `Result<T, anyhow::Error>` or typed store error |
| Provider adapter | Normalize external failure payloads | Structured adapter error |
| Runner/Supervisor loops | Isolate failures and keep loop alive | Logged + status transition |

Rule: map errors once per boundary; avoid repetitive wrapping that hides root cause.

---

## Result Type Policy

### Internal flow

- Use `anyhow::Result<T>` for orchestration glue code.
- Add context at operation boundary:
  - DB query
  - subprocess invocation
  - serialization/deserialization
  - event emit

### Domain branch points

Use typed errors when caller behavior differs by error class.

```rust
pub enum RetryError {
    NotFound,
    NotTerminal,
    Store(anyhow::Error),
}
```

Do not flatten these into plain strings before decision logic.

---

## API Mapping Rules

### HTTP status mapping baseline

- `400 Bad Request`: validation failure
- `404 Not Found`: resource absent
- `409 Conflict`: state conflict (idempotency, lease/version conflict)
- `422 Unprocessable Entity`: semantically invalid but syntactically correct request
- `500 Internal Server Error`: unexpected/internal failures
- `503 Service Unavailable`: temporary upstream dependency outage (optional, endpoint-specific)

### Response content rules

- Client response must be stable and concise.
- Do not include SQL statements, stack traces, file paths, env names, or secrets.
- Correlate with server logs via request/job identifiers.

---

## Runner / Supervisor Error Policy

### Loop resilience

- Loop-level recoverable errors must not crash process.
- Panic in worker task should be isolated and converted to failure status.
- Timeout should be explicit and handled as controlled failure.

### Event bus semantics

- Event bus is best-effort.
- Publish failure does not roll back committed DB state.
- Log publish failure with event type and identifiers.
- Preserve eventual consistency via polling/snapshot mechanisms.

---

## Retry and Idempotency Policy

### Retry matrix (default)

| Error class | Retry? | Notes |
|------------|--------|-------|
| Validation | No | Caller must fix input |
| Domain conflict | Usually No | May require state refresh |
| Infra-transient | Yes | Bounded attempts + backoff + jitter |
| Infra-permanent | No | Escalate and fix root cause |
| External dependency | Depends | Classify by reason (timeout vs contract mismatch) |

### Idempotency

- Replays/retries must be idempotent at store boundary.
- Expected duplicate key race (e.g., `23505`) should be explicitly handled.
- Never rely on "probably no duplicate" assumptions.

---

## Logging and Metrics Requirements for Errors

### Required log fields (where available)

- `job_id`, `job_type`
- `dataset_id`, `provider`
- `stage` (`fetch|normalize|dq|persist|emit|api|runner|store`)
- `decision` (`accept|degraded|reject`) for DQ pipeline
- `attempt`, `retryable`

### Metrics expectations

- Stage error counter increments on handled failures.
- Stage duration records for success and failure paths.
- Distinguish fallback/degraded paths from hard failures.

See `logging-guidelines.md` for field/level conventions.

---

## Good / Bad Patterns

### Good

```rust
let rows = sqlx::query_as::<_, JobRow>(SQL)
    .fetch_all(pool)
    .await
    .with_context(|| "list_jobs query failed")?;
```

```rust
if let Err(err) = bus.publish(event).await {
    warn!(job_id = %job_id, error = %err, "event publish failed; continue");
}
```

### Bad

```rust
let rows = sqlx::query_as::<_, JobRow>(SQL).fetch_all(pool).await.unwrap();
```

```rust
if bus.publish(event).await.is_err() {
    return Err(anyhow!("publish failed")); // incorrectly aborts committed flow
}
```

---

## Forbidden Patterns

- Runtime `unwrap`/`expect` in production paths.
- Swallowing errors with `let _ = ...` on critical operations.
- Returning only "operation failed" without contextual fields.
- Mapping all failures to the same HTTP status.
- Retrying non-retryable errors without classification.

---

## Review Checklist (Error Handling)

- Is each failure path classified (validation/domain/transient/permanent/external)?
- Is error handling performed at the layer that owns the decision?
- Are context and identifiers preserved for diagnostics?
- Are HTTP mappings explicit and stable for clients?
- Are retries bounded and applied only to retryable failures?
- Are idempotency and conflict races handled explicitly?
- Do loop/task failures degrade gracefully without process collapse?
- Are logs safe (no sensitive leak) and operationally useful?
- Are metrics emitted for both error count and stage latency?
- Is behavior consistent with SSOT (DB) + best-effort bus semantics?

---

## Notes

When guidance conflicts, frozen contract docs are authoritative:
- `data-pipeline-contracts.md`
- `akshare-dataset-contracts.md`
