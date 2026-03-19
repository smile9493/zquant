# P1-9 命令消费批处理

## 背景来源
- 来源任务：`A:\zquant\.trellis\tasks\archive\2026-03\03-19-03-19-findings-remediation-execution\prd.md`
- 对应阶段：P1-9「命令消费批处理」。

## 目标
- 将 UI 命令消费从“每帧仅消费 1 条”升级为“可控批量 drain”。
- 在保证顺序语义与不丢命令的前提下，降低高负载下 UI 命令积压与交互延迟。

## 范围
- `A:\zquant\crates\ui-workbench\src\lib.rs`（命令队列生产/消费接口）
- `A:\zquant\crates\app-shell\src\app.rs`（每帧消费调度与上限控制）
- 相关测试（队列顺序、批量上限、无丢失）

## 非目标
- 不改变命令类型定义及业务语义。
- 不调整渲染管线、数据拉取策略或任务执行器。
- 不引入跨线程无界并发队列替换。

## 验收标准
1. 每帧命令消费支持可配置批量上限（例如 `max_commands_per_frame`）。
2. 消费顺序保持 FIFO，不改变既有命令执行顺序语义。
3. 当队列长度 > 上限时，剩余命令保留到后续帧继续处理（无丢失）。
4. 至少新增 3 个测试覆盖：
   - 批量上限生效；
   - FIFO 顺序保持；
   - 跨帧 drain 完整性。
5. `cargo check -p ui-workbench -p app-shell` 通过。
6. `cargo test -p ui-workbench -p app-shell` 通过。
7. `cargo check --workspace` 通过。

## 假设与风险
- 假设当前命令处理为单线程 UI 主循环，可安全按帧分批执行。
- 风险：批量过大导致单帧耗时上升。
  - 应对：设置保守默认值，并在后续加入可调参数。
- 风险：改造后出现命令饥饿或顺序偏移。
  - 应对：通过顺序与跨帧完整性测试锁定行为。

## 实现计划
1. 盘点当前命令队列 API（单条 `poll`）与消费点。
2. 新增批量 drain API（返回当帧命令批次，受上限控制）。
3. 在 `app-shell` 每帧循环接入批量消费。
4. 补充测试并验证无顺序漂移/无丢失。
5. 执行 review gate 并回写 PRD。

## Checklist
- [x] 新建 Trellis 任务并设为当前任务
- [x] 写入 PRD（目标/范围/非目标/验收/风险/计划）
- [ ] 实现命令批量 drain 与每帧上限控制
- [ ] 补充并通过 ui-workbench/app-shell 相关测试
- [ ] 执行 review gate 并输出 REVIEW 结论
