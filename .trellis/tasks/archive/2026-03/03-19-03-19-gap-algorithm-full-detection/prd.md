# P1-5 Gap Algorithm: Full Prefix/Middle/Suffix Detection

## Goal
Enhance `GapCalculator::calculate_gaps` to detect middle gaps (between consecutive bars) in addition to existing prefix/suffix gap detection, using timeframe-aware expected intervals.

## Background
Current implementation only detects:
- Prefix gap: requested_start < first_bar.timestamp
- Suffix gap: last_bar.timestamp < requested_end

Missing:
- Middle gaps: consecutive bars separated by more than the expected timeframe interval
- The `timeframe` parameter is available in `load_bars_range` but not passed to `calculate_gaps`

## Scope
- `crates/repository-market/src/gap.rs`: Add timeframe-aware middle gap detection
- `crates/repository-market/src/lib.rs`: Pass `timeframe` to `calculate_gaps` calls
- Unit tests for all gap types

## Non-Goals
- Calendar-aware gap detection (skip weekends/holidays) — future enhancement
- Sub-daily timeframe support beyond 1h/4h — only daily/1h/4h for now
- Changing the 3-layer fetch strategy in `load_bars_range`

## Acceptance Criteria
1. `calculate_gaps` accepts a `timeframe` parameter
2. Middle gaps detected when consecutive bar timestamps exceed expected interval
3. Prefix and suffix gap behavior unchanged
4. Timeframe parsing: "1d" -> 24h, "1h" -> 1h, "4h" -> 4h, unknown -> fallback to prefix/suffix only
5. All existing tests pass
6. New tests cover: middle gap single, middle gap multiple, no middle gap when continuous, unknown timeframe fallback
7. `cargo check -p repository-market` zero warnings
8. `cargo test -p repository-market` all pass

## Risks
- Changing `calculate_gaps` signature affects 2 call sites in `load_bars_range`
- Timeframe string parsing needs to be robust (unknown values should not panic)

## Implementation Plan
1. Add `timeframe` param to `calculate_gaps`, parse to `chrono::Duration`
2. Add middle gap detection loop between sorted bars
3. Update 2 call sites in `lib.rs`
4. Add unit tests for middle gaps
5. Verify existing tests still pass

## Checklist
- [x] Create task and write PRD
- [x] Implement timeframe-aware `calculate_gaps`
- [x] Update call sites in `lib.rs`
- [x] Add middle gap tests
- [x] Verify all tests pass
- [x] Review Gate

## Review Record (2026-03-19)

### Scope Reviewed
- `crates/repository-market/src/gap.rs`
- `crates/repository-market/src/lib.rs`

### What Was Verified
- `calculate_gaps` signature updated to include `timeframe`.
- Prefix/middle/suffix gap detection works for known timeframe values.
- Unknown timeframe falls back to prefix/suffix only.
- `load_bars_range` call sites pass `timeframe` in both initial and remaining-gap calculations.

### Checks Run
- `cargo check -p repository-market` ✅
- `cargo test -p repository-market` ✅ (36 passed, 0 failed)
- `cargo check --workspace` ✅

### Review Findings
- No blocking findings.
