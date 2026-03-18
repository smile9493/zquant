# M4 任务执行与状态管理最小闭环

## Goal

基于 `A:\zquant\.kiro\specs\zquant-enterprise-evolution-roadmap\design.md`，完成 M4 的最小可运行闭环：  
实现任务生命周期管理、任务面板状态展示、Workspace 快照恢复与保存。

## Scope

### In scope

- `jobs-runtime`：`submit -> running -> success/failed/cancelled` 生命周期。
- `application-core`：接入 `RefreshData` 命令并触发异步任务。
- `ui-workbench`：任务面板展示任务状态与日志更新。
- `domain-workspace`：启动恢复、退出保存（最小路径）。
- M4 相关单元/集成测试与 review gate。

### Out of scope

- 企业协同能力（权限/分发/许可证）。
- 分布式任务调度与多机执行。
- 高级任务编排（依赖 DAG、优先级队列、抢占式调度）。

## Non-Goals

- 不替换 M1-M3 已稳定通过的架构与接口。
- 不在本任务内完成完整运维监控平台。

## Acceptance Criteria

- [x] 可创建并运行刷新任务，状态正确流转。
- [x] 任务面板可展示实时状态与结果（成功/失败/取消）。
- [x] Workspace 可在启动时恢复、退出时保存。
- [x] 异常路径有明确日志与用户可见反馈。
- [x] `cargo test`（M4 相关）与 `cargo check --workspace` 通过。

## Assumptions / Risks

### Assumptions

- M3 数据分层能力已可稳定提供数据访问基础。
- 当前 PostgreSQL schema 可承载任务状态持久化。

### Risks

- UI 状态与任务真实状态不同步。
- 任务取消与退出保存之间存在竞态条件。
- 异常路径覆盖不足导致“假成功”。

## Implementation Plan

1. 梳理并固化任务生命周期模型与状态变迁约束。
2. 在 `jobs-runtime` 落地最小生命周期执行链路。
3. 在 `application-core` 接入 `RefreshData` 命令到运行时。
4. 在 `ui-workbench` 展示任务状态、日志与错误反馈。
5. 打通 Workspace 恢复/保存最小闭环。
6. 补充测试并执行 M4 review gate。

## Checklist

- [x] 定义/确认任务状态机与状态迁移规则。
- [x] 完成 `jobs-runtime` 生命周期主流程与错误分支。
- [x] 接入 `RefreshData` 命令到任务运行时。
- [x] 完成任务面板状态与日志展示。
- [x] 完成 Workspace 启动恢复与退出保存。
- [x] 增加 M4 相关测试。
- [x] 运行 `cargo test`（M4 相关）并通过。
- [x] 运行 `cargo check --workspace` 并通过。
- [x] 写回审查结论（`REVIEW: PASS/FAIL`）。

## Review findings（2026-03-18 第 1 轮）

1. **任务取消语义不完整（功能性缺口）**
   - 现状：`TaskRuntime::cancel(id)` 仅把状态改成 `Cancelled`，但没有向运行中的任务发送取消信号。
   - 证据：`TaskHandle` 持有 `cancel_tx`（`crates/jobs-runtime/src/lib.rs`），`submit` 中创建了 `watch::channel(false)`，但 `cancel(id)` 无法访问并发送该信号。
   - 影响：长任务在 UI 显示“已取消”后仍可能继续执行（仅状态被短路），不满足“可取消任务”的真实执行语义。

2. **Trellis 任务状态与审查记录未闭环（流程缺口）**
   - 现状：`task.json` 仍为 `"status": "planning"`，PRD 主 Checklist 未按实际实现和验证结果回写。
   - 影响：与“任务完成”声明不一致，不满足仓库审查门禁的可追踪性要求。

## Root cause

- 运行时设计只实现了“状态层取消”，未实现“执行层取消信号路由”（缺少 `task_id -> cancel_tx` 管理）。
- 提交“完成”前未执行 Trellis 文档状态同步步骤（`task.json` + PRD Checklist + 审查结论）。

## Repair plan

1. 在 `jobs-runtime` 增加取消信号注册表（`TaskId -> watch::Sender<bool>`），并在 `cancel(id)` 中发送信号，再更新状态。
2. 调整任务执行包装逻辑：在任务已取消时尽早退出，避免继续执行业务闭包。
3. 补充行为测试：
   - `cancel_running_task_propagates_signal_and_stops`
   - `cancelled_task_does_not_emit_success_after_completion`
4. 回写 Trellis 元数据：
   - 将 `task.json` 状态从 `planning` 更新到实际阶段（至少 `in_progress`，通过后再 `done`/归档）。
   - 勾选已完成 Checklist，保留未完成项并附验证说明。
5. 修复后重新执行 review gate，并在本 PRD 追加最终审查结论。

## Updated checklist（审查后）

- [x] 已新增 `jobs-runtime` 并接入 `application-core` / `ui-workbench` / `app-shell`。
- [x] 已执行 `cargo test -p jobs-runtime -p application-core -p ui-workbench -p app-shell -p repository-market -p infra-parquet -p store-manifest`。
- [x] 已执行 `cargo check --workspace`。
- [x] 修复任务取消“仅改状态、不停执行”的语义缺口。
- [x] 回写 `task.json` 与 PRD 主 Checklist 的最终状态一致性。
- [x] 复审并给出最终 `REVIEW: PASS`。

## Final review（2026-03-18 第 2 轮）

### 修复核对

- 已引入 `task_id -> cancel sender` 路由，`cancel(id)` 会下发取消信号并更新状态为 `Cancelled`。
- 已补充行为测试：
  - `cancel_running_task_propagates_signal_and_stops`
  - `cancelled_task_does_not_emit_success_after_cancel`
- 已确认状态机不允许 `Cancelled -> Running/Success` 非法回退，取消后不会被成功态覆盖。

### 验证命令

- `cargo test -p jobs-runtime -p application-core -p ui-workbench -p app-shell -p repository-market -p infra-parquet -p store-manifest`
- `cargo check --workspace`

### 审查结论

`REVIEW: PASS`
