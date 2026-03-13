# Database Guidelines

> Database patterns and conventions for this project.

---

## Stack

- Database: Postgres.
- Rust library: `sqlx`.
- Migrations: raw SQL files under `migrations/`.

Primary store implementation: `crates/job-store-pg/src/lib.rs`.

## Connection and Configuration

- Use `DATABASE_URL` (environment variable) as the primary configuration.
- In binaries, default values may exist for local dev, but tests should explicitly set `DATABASE_URL`.

## Query Patterns

- Prefer explicit SQL strings with bind parameters (`$1`, `$2`, ...).
- Prefer `query_as`/`query_scalar` with typed mapping structs (`#[derive(sqlx::FromRow)]`).
- Use transactions when:
  - inserting a job + idempotency record
  - performing a multi-step read/modify/write that must be atomic

Example pattern (idempotency reservation) exists in `JobStore::create_job`.

## Concurrency / Claiming Work

- Use `FOR UPDATE SKIP LOCKED` for claiming jobs to avoid double-claim.
- Use fencing with `lease_version` to prevent stale finalize/heartbeat.

Store methods to follow:
- `claim_jobs(...)`
- `finalize_job(...)`
- `heartbeat_job(...)` (if/when used)

## Idempotency

- Table: `jobs_idempotency`.
- Unique constraint on `idempotency_key`.
- Handle the race by detecting unique-violation error code `23505` and returning the canonical job.

Implementation: `is_unique_violation` helper in `crates/job-store-pg/src/lib.rs`.

## Migrations

- Migrations live in `migrations/*.sql` and are referenced by tests.
- E2E tests use:
  - `#[sqlx::test(migrations = "../../migrations")]`
  - Example: `crates/job-store-pg/tests/e2e_test.rs`

## Common Windows Gotcha (E2E)

On Windows, avoid using host port 5432 for Docker Postgres if a local Postgres service may already be listening.
Use a non-5432 port mapping (e.g. 15432 or 55432) and set `DATABASE_URL` accordingly.
