# Rust Coding Guidelines

> Rust backend coding standards for this repository.

---

## Scope

These rules apply to Rust code under `apps/` and `crates/`.

This guide complements, but does not replace:
- `database-guidelines.md`
- `error-handling.md`
- `logging-guidelines.md`
- `quality-guidelines.md`
- `type-safety.md`

---

## Official References

Use the following as primary baselines:

- The Rust Programming Language (Book): https://doc.rust-lang.org/book/
- Rust Reference: https://doc.rust-lang.org/reference/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- std docs: https://doc.rust-lang.org/std/
- Tokio docs: https://docs.rs/tokio/latest/tokio/
- Clippy lint index: https://doc.rust-lang.org/clippy/
- Rust Edition Guide: https://doc.rust-lang.org/edition-guide/

Project-specific rules in this repository take precedence when there is a conflict.

---

## Engineering Principles

1. **Correctness first**: state transitions, idempotency, and boundary contracts are explicit.
2. **Simple over clever**: prefer predictable code over highly abstract designs.
3. **Thin binaries**: orchestration goes in crates, app entrypoints only wire dependencies.
4. **Observable behavior**: meaningful errors, structured logs, and testable flow.
5. **Change safety**: prefer local, reversible changes with narrow blast radius.

---

## Module and API Boundaries

### Required

- Keep modules focused by responsibility (API, orchestration, store, contracts).
- Restrict visibility: default to private; promote to `pub(crate)` before `pub`.
- Public APIs must express constraints via type signatures, not comments only.
- Binary crate (`apps/*`) should avoid owning business logic.

### Example

```rust
// Good: typed boundary + explicit dependency
pub struct Runner<S: JobStore, B: EventBus> {
    store: S,
    bus: B,
}
```

```rust
// Bad: weak boundary with hidden global state
pub fn run() {
    // uses global mutable singleton store/bus
}
```

---

## Type and Trait Design

### Required

- Use enums for finite protocol/state decisions.
- Use newtypes for semantically distinct primitives (`JobId`, `LeaseVersion`).
- Prefer generics for compile-time contracts when call sites are bounded.
- Use trait objects only when runtime polymorphism is required by architecture.

### Decision Rule: Generic vs `dyn Trait`

- Use **generics** when:
  - performance and inlining matter,
  - there are few implementations,
  - caller controls concrete type.
- Use **`dyn Trait`** when:
  - plugin-style runtime swapping is required,
  - implementation set is open-ended,
  - object safety and indirection are acceptable.

### Forbidden

- Stringly-typed states in core domain paths.
- Trait hierarchies that hide ownership/lifetime constraints.
- Blanket `pub` exports of internal modules.

---

## Ownership, Borrowing, and Allocation

### Required

- Prefer borrowing (`&T`, `&mut T`) for read paths and local transforms.
- Clone only for explicit ownership transfer or async lifetime requirements.
- For hot paths, avoid repeated heap allocations inside tight loops.
- Prefer iterator adapters and slices over building temporary vectors unless needed.

### Anti-patterns

- Clone cascades (`.clone().clone()`) to bypass borrow checker decisions.
- Returning owned collections when a borrowed iterator or slice is sufficient.

### Review cue

If `.clone()` appears in non-test code, reviewer should ask: **is ownership transfer truly required?**

---

## Async, Concurrency, and Task Lifecycle

### Required

- Every spawned task must have a lifecycle policy: complete, cancel, or supervised restart.
- Add timeout/cancellation boundaries for external I/O and long-running work.
- Never hold a mutex guard across `.await` unless unavoidable and documented.
- Slow-consumer paths must be bounded (channel capacity / drop strategy / backpressure).

### Spawn policy

- `tokio::spawn` is allowed only when:
  - panic/result handling is explicit,
  - task ownership is clear,
  - shutdown behavior is defined.

### Good pattern

```rust
let handle = tokio::spawn(async move { process_job(job).await });
match tokio::time::timeout(timeout, handle).await {
    Ok(Ok(result)) => result?,
    Ok(Err(join_err)) => return Err(anyhow::anyhow!("task join failed: {join_err}")),
    Err(_) => return Err(anyhow::anyhow!("task timeout")),
}
```

---

## Error Boundaries (Rust-side)

- Internal orchestration can use `anyhow::Result<T>`.
- When caller branches by error kind, use typed domain errors.
- Add context at boundaries (`context`, `with_context`) for diagnosis.
- Convert low-level errors once at boundary; avoid multiple remaps across layers.

See `error-handling.md` for full policy.

---

## Logging and Observability

### Required

- Use `tracing` with structured fields (`job_id`, `job_type`, `dataset_id`, `provider`, etc.).
- Log state transitions and retries with reason and attempt index.
- Avoid logging full payloads and secrets.

### Level guidance

- `debug`: branch decisions and internal detail.
- `info`: lifecycle milestones.
- `warn`: recoverable degradation/retry.
- `error`: dropped work, invariant break, or user-visible failure.

---

## Performance and Safety Tradeoffs

- Optimize only after measuring (`criterion`, targeted benchmarks, or production metrics).
- Avoid premature micro-optimizations that reduce clarity.
- Prefer deterministic behavior over maximum throughput in control-plane paths.
- Use `unsafe` only with documented invariants and a test proving safety boundaries.

### `unsafe` policy

- `unsafe` requires:
  - comment describing invariant,
  - unit/integration test covering invariant boundary,
  - reviewer sign-off.

---

## Testing and Documentation

### Required

- New behavior must have tests at the narrowest effective level.
- Public APIs should have rustdoc comments with contract-level semantics.
- Non-obvious invariants must be documented near code that relies on them.

### Preferred validation order

1. Targeted unit test(s)
2. Targeted crate test/check
3. Wider workspace checks when impact is broad

---

## Common Mistakes (Do/Don't)

### Do

- Keep state transitions explicit with enums and match arms.
- Add context before returning infra errors.
- Bound channels and background task queues.
- Keep handler side effects ordered and observable.

### Don't

- Hide failures with `let _ = ...` on critical paths.
- Panic in runtime code with `unwrap`/`expect`.
- Rely on implicit fallback behavior for provider or routing decisions.
- Mix API parsing, orchestration, and persistence in one function.

---

## Review Checklist (Rust Backend)

- Are module boundaries and visibility minimal and coherent?
- Are public APIs strongly typed and contract-driven?
- Are ownership and clone decisions justified?
- Are spawned tasks bounded, supervised, and cancel-safe?
- Are lock usage and `.await` interactions safe?
- Are errors contextualized and mapped at proper boundaries?
- Are logs structured and sensitive data excluded?
- Are tests added at the correct level for new behavior?
- Is any `unsafe` code justified, documented, and tested?
- Does this change remain consistent with backend specs in `.trellis/spec/backend/`?
