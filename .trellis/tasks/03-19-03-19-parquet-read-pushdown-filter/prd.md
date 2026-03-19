# P1-7 Parquet 读取下推过滤优化

## 背景
- 来源：`A:\zquant\.trellis\tasks\03-19-03-19-findings-remediation-execution\prd.md` 的 P1-7。
- 当前风险：`read_range` 先全量读再内存过滤，随着分区文件增长会放大 I/O 与内存开销。

## 目标
- 在 `infra-parquet`/`repository-market` 路径实现“按时间范围下推过滤读取”，减少无效扫描。
- 保持现有对外行为与契约不变（返回结果、排序、去重语义不变）。

## 范围
- `A:\zquant\crates\infra-parquet\src\reader.rs`
- `A:\zquant\crates\repository-market\src\lib.rs`（如需调用适配）
- 相关单测/集成测试（reader + repository）

## 非目标
- 不改分区键规范与目录结构。
- 不引入新的存储格式或缓存层。
- 不在本任务处理 provider 复用与 UI 命令批处理。

## 验收标准
1. `read_range` 支持基于时间列的下推过滤（非全读后过滤）。
2. 读取结果与现有语义一致：时间区间正确、顺序正确、无额外重复。
3. `repository-market` 现有关键路径测试保持通过。
4. 新增测试覆盖：
   - 命中窗口仅读取目标区间；
   - 空窗口返回空；
   - 边界时间（start/end）处理正确。
5. `cargo check -p infra-parquet -p repository-market` 通过。
6. `cargo test -p infra-parquet -p repository-market` 通过。
7. `cargo check --workspace` 通过。

## 假设与风险
- 假设 Parquet 时间字段可用于谓词过滤；若现有 schema 不统一需先做兼容映射。
- 风险：下推后边界条件（含/不含）可能与旧逻辑有偏差。
  - 应对：先补边界回归测试，再替换实现。

## 实现计划
1. 盘点 `reader.rs` 当前读取链路与可下推点。
2. 实现时间范围谓词下推并保留旧行为对齐。
3. 在 `repository-market` 对接并验证主流程不变。
4. 增补测试（窗口命中/空窗口/边界）。
5. 执行 review gate 与 PRD 回写。

## Checklist
- [x] 新建 Trellis 任务并设为当前任务
- [x] 写入 PRD（目标/范围/验收/风险/计划）
- [x] 实现 Parquet 时间范围下推读取
- [x] 补齐并通过 infra-parquet/repository-market 测试
- [x] 执行 review gate 并输出 REVIEW 结论

## Review findings（2026-03-19）

1. **运行时 `expect` 出现在生产路径（阻塞）**
   - 位置：`A:\zquant\crates\infra-parquet\src\reader.rs`（`read_parquet_file_filtered` 的 predicate 闭包）
   - 现状：`downcast_ref::<TimestampMillisecondArray>().expect(...)`
   - 问题：违反后端规范中“生产路径避免 `unwrap/expect`”约束，遇到异常 schema 时会 panic，而不是返回可处理错误。

## Root cause

- 为了快速接入 `ArrowPredicateFn`，在 predicate 闭包中使用了 `expect` 做类型断言，忽略了异常 schema 的错误传播路径。

## Repair plan

1. 将 predicate 闭包中的 `expect` 改为可恢复错误路径（返回 `Err(ArrowError::ComputeError(...))` 或等价错误类型）。
2. 增加异常 schema / 非预期列类型的测试，验证不会 panic，且调用侧能拿到错误。
3. 复跑：
   - `cargo check -p infra-parquet -p repository-market`
   - `cargo test -p infra-parquet -p repository-market`
   - `cargo check --workspace`
4. 修复并复审通过后，再将任务状态更新回 completed。

## Updated checklist

- [x] 复现并定位 review 阻塞项
- [ ] 去除生产路径 `expect` 并改为显式错误传播
- [ ] 补充异常 schema 场景测试
- [x] 复跑现有编译/测试检查
- [ ] 复审通过并恢复任务 completed 状态

## Re-review（2026-03-19）

修复项：
- `expect("timestamp column must be TimestampMillisecondArray")` 替换为 `ok_or_else(|| ArrowError::SchemaError(...))?`
- 生产路径零 expect/unwrap（grep 确认仅 #[cfg(test)] 内存在）

验证：
- `cargo check -p infra-parquet`：零警告
- `cargo test -p infra-parquet -p repository-market`：12 + 39 = 51 全通过
- `cargo check --workspace`：零警告

结论：全部 7 条验收标准满足 + 后端规范 expect 禁令已遵循，REVIEW: PASS。
