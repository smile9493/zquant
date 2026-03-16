# Type Safety Guidelines

> Type-safety conventions for Rust backend code in this repository.

---

## Scope

These rules apply to Rust backend code under `apps/` and `crates/`.

This guide complements:
- `rust-coding-guidelines.md`
- `error-handling.md`
- `database-guidelines.md`
- `quality-guidelines.md`

---

## Core Rules

- Prefer explicit domain types over primitive aliases in public/backend boundaries.
- Avoid `serde_json::Value` as a long-lived internal type unless schema is truly dynamic.
- Keep API input/output structs explicit and versionable.
- Avoid type erasure (`dyn Any`, unchecked downcast) in core flows.

---

## Domain Modeling

### Required

- Use enums for finite state and protocol decisions.
- Use newtypes for identifiers and semantically distinct primitives.
- Keep invalid states unrepresentable where practical.

### Example (Good)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobId(pub String);
```

### Example (Bad)

```rust
// Stringly-typed status and id across layers
pub type JobStatus = String;
pub type JobId = String;
```

---

## API and Serialization Types

### Required

- Separate transport DTOs from domain entities when semantics differ.
- Derive only traits you need (`Serialize`, `Deserialize`, `FromRow`, etc.).
- Validate and convert at boundaries; do not propagate unvalidated transport fields deep into domain logic.

### Guidance

- For backwards-compatible API growth, prefer optional fields with explicit defaults.
- Use dedicated request/response structs rather than tuple/anonymous maps.

---

## Error Types and Results

- Use `anyhow::Result<T>` for internal orchestration paths.
- Use typed domain errors where callers need to branch on error kind.
- Preserve context with `context(...)`/`with_context(...)`.
- Do not flatten errors into opaque strings too early.

See also: `error-handling.md`.

---

## Database Type Safety

- Prefer typed mapping with `#[derive(sqlx::FromRow)]`.
- Keep SQL bind parameters typed and explicit.
- Do not interpolate SQL fragments with user input.
- Convert DB rows into domain types at repository/store boundary.

See also: `database-guidelines.md`.

---

## Async and Concurrency Type Safety

- Ensure data crossing task/thread boundaries is `Send`/`Sync` as required.
- Prefer immutable shared data with `Arc<T>` and scoped mutability.
- Avoid sharing mutable state unless synchronization strategy is explicit.
- Keep channel payloads strongly typed.

---

## Forbidden Patterns

- `unwrap()`/`expect()` in runtime paths (except tightly scoped tests/tools).
- Broad `as` casts without range/semantic checks.
- Propagating raw JSON maps through multiple layers when schema is known.
- Public functions returning loosely typed blobs when a struct/enum is available.

---

## Review Checklist (Type Safety)

- Are key domain concepts represented by explicit types?
- Are DTO/domain boundaries clear?
- Are enums/newtypes used where they prevent invalid states?
- Are conversions validated at boundaries?
- Are `unwrap`/unchecked casts avoided in production paths?
- Are DB and async boundaries strongly typed?

---

## Notes

When in conflict, project-specific backend contracts are authoritative:
- `data-pipeline-contracts.md`
- `akshare-dataset-contracts.md`
