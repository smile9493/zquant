# Directory Structure

> How backend code is organized in this project.

---

## Top-level Layout

- `apps/`: Rust binaries (entrypoints). Current baseline includes `job-*` services; desktop evolution introduces `apps/desktop-app`.
- `crates/`: Rust library crates (domain, application logic, infra).
- `migrations/`: SQL migrations (Postgres), used by `sqlx::test`.
- `deploy/`: local docker compose (dev services).
- `scripts/`: developer utilities (including Docker-based test scripts).
- `docs/`: planning and design documents.

## Rust Workspace Pattern

This repo uses a Cargo workspace with small, focused crates:

- Domain crate: data types and traits that should not depend on infrastructure.
  - Example: `crates/job-domain/` (e.g. `Job`, `JobStatus`, `JobHandler`, `JobContext`).

- Contracts + bus crate: event contracts and in-process event bus interface.
  - Example: `crates/job-events/` (e.g. `types.rs`, `bus.rs`).

- Application crate: orchestration logic wiring store + bus + handlers.
  - Example: `crates/job-application/` (e.g. `api.rs`, `runner.rs`, `agent_supervisor.rs`).

- Storage/infra crates: concrete implementations.
  - Example: `crates/job-store-pg/` (SQLx + Postgres store).

## Binaries (apps)

Keep binaries thin:
- Initialize logging.
- Build shared components (store/bus/registry).
- Start loops (API server, runner loops, supervisor loop) or desktop shell lifecycle.

Examples:
- Service/kernel mode: `apps/job-kernel/src/main.rs`
- Desktop mode (target): `apps/desktop-app/src/main.rs`

## Naming Conventions

- Crate names: kebab-case (Cargo), usually prefixed with `job-`.
- Rust modules/files: snake_case.
- Public entrypoints:
  - Application: `job_application::router`, `Runner`, `AgentSupervisor`.
  - Events: `job_events::bus::{Event, EventBus, InMemoryEventBus}` and `job_events::types::*`.

## Where to Put New Code

- New job handler types: keep them close to the binary that wires them (or introduce a dedicated `crates/job-handlers/` if they grow).
- New event contracts: `crates/job-events/src/types.rs` and referenced from `bus.rs`.
- New store methods: `crates/job-store-pg/src/lib.rs`.
- New orchestration/loops: `crates/job-application/src/*`.
- Desktop shell/workbench-specific UI orchestration should live in dedicated desktop crates (see `.trellis/spec/desktop/`).
