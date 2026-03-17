# Backend Development Guidelines

> Best practices for backend development in this project.

---

## Overview

These documents describe the backend conventions that are actually used in this repo.

Last updated: 2026-03-17

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Module organization and file layout | Filled |
| [Database Guidelines](./database-guidelines.md) | SQLx patterns, queries, migrations | Filled |
| [Error Handling](./error-handling.md) | Error types, propagation, API mapping | Filled |
| [Type Safety](./type-safety.md) | Domain types, DTO boundaries, conversions, and unwrap/cast constraints | Filled |
| [Logging Guidelines](./logging-guidelines.md) | Tracing + structured logging | Filled |
| [Quality Guidelines](./quality-guidelines.md) | Testing and code quality standards | Filled |
| [Rust Coding Guidelines](./rust-coding-guidelines.md) | Rust language usage, ownership, async, and review checklist | Filled |

## Contract Specs

| Spec | Description | Status |
|------|-------------|--------|
| [Data Pipeline Contracts](./data-pipeline-contracts.md) | Frozen pipeline contracts for provider/DQ/events/persist interfaces | Frozen |
| [AkShare Dataset Contracts](./akshare-dataset-contracts.md) | Frozen dataset contract for CN equity daily OHLCV via AkShare | Frozen |

---

## Notes

- Language: English (required for this directory).
- Scope: Rust backend (apps + crates).
