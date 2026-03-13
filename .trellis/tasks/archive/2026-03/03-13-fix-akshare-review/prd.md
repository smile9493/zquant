# Fix AkShare Provider Review Findings

## Source

- Parent task: `.trellis/tasks/archive/2026-03/03-13-akshare-provider-phase-b/prd.md` (lines 160-209)
- Review outcome: REVIEW: FAIL (2 findings)

## Goal

Fix 2 independent review findings in the AkShare provider implementation to meet the frozen contract requirements.

## Findings to Fix

### Finding 1 (P1): Real AkShare output is not normalized to contract fields

**Problem**:
- Frozen contract requires: `date/open/high/low/close/volume`
- Real AkShare returns Chinese columns: `日期/开盘/收盘/最高/最低/成交量`
- Python script only does `df.to_dict(orient="records")` without mapping
- Hermetic test used fake English payload, so didn't catch this

**Files**:
- `crates/data-pipeline-application/python/akshare_cn_equity_ohlcv_daily.py:41`
- `.trellis/spec/backend/akshare-dataset-contracts.md` (contract reference)

### Finding 2 (P2): Python subprocess failures lose error messages

**Problem**:
- Python script writes error JSON to `stdout` on failure (exit code 1)
- Rust runner only reads `stderr` on non-zero exit, ignores `stdout`
- Result: adapter error details are lost

**Files**:
- `crates/data-pipeline-application/python/akshare_cn_equity_ohlcv_daily.py:53`
- `crates/data-pipeline-application/src/python_runner.rs:80`

## Scope

### In scope

- Add explicit column name mapping in Python adapter (Chinese → English)
- Add hermetic test with real AkShare-style Chinese column names
- Unify subprocess error transport (read stdout for error JSON on non-zero exit)
- Add regression test for error message propagation

### Out of scope

- Any other AkShare provider changes
- Performance optimizations
- Additional dataset support

## Acceptance Criteria

- [x] Python adapter maps Chinese columns to English contract fields
- [x] New test validates Chinese→English mapping with fake runner
- [x] Subprocess runner preserves adapter error messages on failure
- [x] New test validates error message propagation
- [x] All existing tests still pass (15/15 passed)
- [x] `cargo check/test/clippy` pass for data-pipeline-application

## Review Outcome (2026-03-13 - Final Review)

### REVIEW: PASS ✅

所有问题已修复，通过最终独立复审。

### 实际复核验证

**真实 AkShare 子进程成功路径**：
- `akshare_cn_equity_ohlcv_daily.py` 正常返回 JSON
- date 列可序列化
- 输出包含契约要求的 date/open/high/low/close/volume

**真实错误路径**：
- 缺少 symbol 时返回 `{"status":"error","message":"missing required field: symbol (string)"}`
- Rust runner 正确保留错误消息

**质量检查**：
- `cargo test -p data-pipeline-application`: PASS (15/15)
- `cargo clippy -p data-pipeline-application -- -D warnings`: PASS

### 非阻断备注

中文列名映射测试使用独立 hermetic 脚本而非真实 AkShare 脚本，但真实 AkShare 成功路径已额外验证，不构成发布阻断。

---

## Review Outcome (2026-03-13 - Second Review)

### REVIEW: PASS ✅

All issues from the second review have been fixed.

### What Changed

**P1 Fix - Date Serialization**:
- Added date column conversion to string before JSON serialization in Python script
- Prevents "Object of type date is not JSON serializable" error

**P2 Fix - Real Column Mapping Test**:
- Created `test_chinese_column_mapping.py` test script that returns mock Chinese column data
- Modified test to directly call SubprocessPythonRunner with test script
- Test now verifies Chinese columns are mapped to English contract fields

### Verification

- `cargo test -p data-pipeline-application`: PASS (15/15 tests)
- `cargo clippy -p data-pipeline-application -- -D warnings`: PASS
- Chinese column mapping test: PASS (verified with real subprocess)
- Error message propagation test: PASS

## Implementation Plan

1. Fix Finding 1:
   - Add column mapping dict in Python script after `df.to_dict()`
   - Map: 日期→date, 开盘→open, 收盘→close, 最高→high, 最低→low, 成交量→volume
   - Add test with Chinese column fake payload

2. Fix Finding 2:
   - Modify `SubprocessPythonRunner::run()` to parse stdout on non-zero exit
   - Extract error message from JSON if present
   - Add test that verifies error details are preserved

3. Verify:
   - Run `cargo test -p data-pipeline-application`
   - Run `cargo clippy -p data-pipeline-application -- -D warnings`
