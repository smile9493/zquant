# Phase D: Frontend optimistic UI

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`
- 前置任务：
  - `A:\zquant\.trellis\tasks\archive\2026-03\03-15-ws-bridge-jobs-logs\`
  - `A:\zquant\.trellis\tasks\archive\2026-03\03-15-frontend-typed-event-bus\`

## Background

当前前端已经具备：
- `JobsTab` 的 stop / retry 交互
- HTTP mutation（`POST /jobs/:id/stop`, `POST /jobs/:id/retry`）
- WebSocket bridge 与 typed event bus
- HTTP + WS 双通道状态同步

但 stop / retry 仍是“提交后等待服务端结果再更新 UI”的模式：
- 用户点击后只能看到按钮 loading / toast
- 列表状态不会立即进入“请求中”或“预期结果”
- 失败时也没有统一的乐观回滚策略

现在已经有：
- typed WS event 入口
- jobs store
- WebSocket reconnect / subscribe

因此可以进入下一步：为 **stop / retry** 增加最小乐观 UI 状态机。

## Goal

为 `JobsTab` 的 `stop` 和 `retry` 交互引入一个 **最小 optimistic UI 层**，使用户在发起动作后立刻看到合理的中间状态，并在 HTTP 结果 / WS 事件到达后自动确认或回滚。

目标是改善交互反馈和状态一致性，不是实现完整 mutation queue 或全局事务系统。

## Scope

### In scope

#### 1. Stop optimistic state
- 用户点击“停止”并确认后：
  - 对应 job 立即进入本地 `stopping` 或等价乐观状态
  - UI 显示“停止请求中”反馈
- 后端成功后：
  - 若 HTTP 成功但尚未收到 WS / refresh，对该 job 保持“stop requested”乐观标识
  - 后续由 WS / HTTP 最终状态覆盖
- 后端失败后：
  - 回滚乐观状态
  - 显示错误提示

#### 2. Retry optimistic state
- 用户点击“重试”后：
  - 当前 job 行进入 `retrying` 或等价本地状态
  - 可显示“正在创建重试任务”反馈
- HTTP 成功返回新 job id 后：
  - 新 job 可先乐观插入 jobs 列表顶部，标记为 `queued/pending`
  - 后续由 snapshot / WS 真正对齐
- HTTP 失败后：
  - 回滚临时状态
  - 显示错误提示

#### 3. Local optimistic state model
- 在前端定义最小的本地派生状态，不直接污染后端 `JobSummary` 契约
- 推荐形式：
  - `pendingActionsByJobId`
  - 或 `jobUiStateMap`
- 需要支持：
  - `idle`
  - `stopping`
  - `retrying`
  - 可选失败瞬态（如 toast 驱动，不必持久）

#### 4. Reconciliation strategy
- 明确事实源优先级：
  1. HTTP mutation 结果
  2. WebSocket typed events / snapshot
  3. 本地 optimistic overlay
- 当真实状态到达时，自动清除对应 optimistic overlay
- 不允许 optimistic 状态长期滞留

#### 5. UI feedback
- `JobsTab` 中 stop / retry 按钮和 job 行状态要体现乐观状态：
  - disabled
  - loading
  - 轻量文案 / 标签
- 保持现有视觉风格，不新增复杂弹层

#### 6. Tests
- 至少补一组前端测试，覆盖：
  - stop optimistic apply / rollback
  - retry optimistic apply / rollback
  - 真实状态到达后 optimistic 清除

### Out of scope

- 全局 optimistic mutation 队列
- 多动作并发冲突调度器
- seq / gap detection
- undo / redo
- Agent Panel optimistic interaction
- 全站 command bus

## Non-goals

- 不追求完整“离线优先”状态机
- 不在本任务里解决所有 mutation 并发冲突
- 不修改后端 API 契约，只在前端做最小 overlay

## Design Direction

建议采用 **base state + optimistic overlay** 的模式：

### 基础状态
- `jobs` 仍然来自 HTTP / WS 的真实数据

### 叠加状态
- 新增一个局部 UI state，例如：
  - `pendingJobActions: Map<string, PendingJobAction>`
- 渲染时通过 selector 合成：
  - `displayJobs = merge(baseJobs, optimisticOverlay)`

### Stop 行为
- apply:
  - 标记 `jobId -> { type: 'stop', startedAt }`
  - 行上显示“停止中”或“已请求停止”
- confirm:
  - HTTP 成功 + 后续真实状态到达时清除 overlay
- rollback:
  - HTTP 失败时移除 overlay

### Retry 行为
- apply:
  - 原 job 标记 `retrying`
- confirm:
  - HTTP 返回新 `job_id` 后可插入临时 job 行
  - 真实列表到达后去重 / 清理 overlay
- rollback:
  - 失败时移除 `retrying` 状态

## Proposed Files

建议涉及：

- `A:\zquant\web\src\stores\jobs.ts`
- `A:\zquant\web\src\components\JobsTab.vue`
- 可选新增：
  - `A:\zquant\web\src\stores\jobs.optimistic.ts`
  - `A:\zquant\web\src\stores\jobs.test.ts`

## Acceptance Criteria

### Behavior
- [ ] stop 操作发起后，job 行立即显示乐观中的中间状态
- [ ] stop 操作失败后，乐观状态回滚
- [ ] retry 操作发起后，job 行立即显示 retry 中间状态
- [ ] retry 成功后，新 job 能在 UI 中及时可见（允许乐观插入）
- [ ] WS / snapshot / HTTP 真实状态到达后，相关 optimistic overlay 被自动清理

### UX
- [ ] 用户能明确区分真实状态与“请求中”状态
- [ ] stop / retry 在请求进行中不会被重复触发

### Testability
- [ ] 至少一组前端测试覆盖 optimistic apply / rollback / reconcile
- [ ] `npm test` 通过
- [ ] `npm run build` 通过

### Review gate
- [ ] PRD 最终记录 `REVIEW: PASS` 或 `REVIEW: FAIL`

## Risks / Assumptions

- 当前 jobs 状态来源已经分为 HTTP 与 WS，两者叠加 optimistic overlay 时需要避免三方冲突。
- retry 的新 job 插入要注意和后续真实 snapshot 去重。
- 不应把 UI 专用状态混进后端读模型类型里，避免再次破坏 typed contract。

## Implementation Plan

1. 盘点 stop / retry 当前状态流转
2. 设计 optimistic overlay 数据结构
3. 实现 selector / merge 逻辑
4. 重构 `JobsTab` 渲染和按钮禁用逻辑
5. 接入 HTTP success / failure / WS reconcile 清理
6. 补测试覆盖 apply / rollback / reconcile
7. 执行 build / test / review gate

## Checklist

- [ ] 明确 stop optimistic 文案和显示策略
- [ ] 明确 retry optimistic 插入与去重策略
- [ ] 不污染 `JobSummary` 原始类型
- [ ] 增加 overlay 清理规则
- [ ] 补前端测试
- [ ] 记录验证命令与结果
- [ ] 完成 review gate

## Review Findings / Repair Plan

### [P1] `retry` 乐观状态根本没有体现在 UI 上，未满足核心验收标准

位置：
- `A:\zquant\web\src\stores\jobs.ts:74`
- `A:\zquant\web\src\stores\jobs.ts:94`
- `A:\zquant\web\src\components\JobsTab.vue:27`

问题：
- store 确实记录了 `pendingActions[type='retrying']`，但 `displayJobs` 只对 `stopping` 做 overlay，`retrying` 分支直接返回原 job。
- `JobsTab` 也没有基于 pending state 给 retry 按钮加 disabled / loading / 文案变化。
- 这与 PRD 中“retry 操作发起后，job 行立即显示 retry 中间状态”不一致。

影响：
- 用户点击“重试”后，界面上看不到任何乐观中的中间状态，只剩 toast 和后续刷新。
- 当前实现没有真正完成 retry optimistic UI。

### [P1] 任意 `snapshot` 到达都会清空所有 pending action，导致无关操作被提前回滚

位置：
- `A:\zquant\web\src\stores\jobs.ts:39`

问题：
- `handleWsMessage()` 在收到任何 `snapshot` 后直接执行 `pendingActions.value.clear()`。
- 这不是“按真实状态到达清理对应 optimistic overlay”，而是粗暴清空所有 job 的 optimistic 状态。
- 如果一个 stop/retry 仍在等待服务端确认，而此时来了周期性 snapshot 或手动刷新返回的 snapshot，UI 会提前丢失请求中状态。

影响：
- optimistic 状态可能在真实确认前被静默移除。
- 多个 job 同时存在 pending action 时，一个 snapshot 会把全部 overlay 一次性抹掉，破坏状态一致性。

### [P2] 测试没有覆盖 PRD 要求的 rollback / reconcile 核心路径

位置：
- `A:\zquant\web\src\stores\jobs.test.ts:10`

问题：
- 现有测试只覆盖：
  - optimistic stop apply
  - clear
  - optimistic retry apply
- 但 PRD 明确要求至少覆盖：
  - stop rollback
  - retry rollback
  - 真实状态到达后的 optimistic 清除（reconcile）
- 当前没有任何测试验证 snapshot / event 到达时的 overlay 清理逻辑，也没有验证 retry 的 UI 状态。

影响：
- 这次实现最关键的状态机边界没有自动化保障。
- 上面的 `snapshot.clear()` 这类回归正是因为测试没有覆盖 reconcile 行为。

## Root Cause

- 实现先把“记录 pending action”做出来了，但没有把 overlay 渲染逻辑和验收标准一一对齐。
- reconcile 逻辑被简化成全量清空，规避了状态映射复杂度，但破坏了 optimistic UI 的语义。
- 测试只验证了最容易写的正向路径，没有验证真正关键的状态机边界。

## Repair Plan

1. 为 `retrying` 设计明确的 UI 表现：
   - job 行附加状态标签 / 文案
   - retry 按钮 disabled 或 loading
2. 将 snapshot reconcile 改为“按 job 粒度清理”：
   - 只在 snapshot/事件已经反映目标结果时清除对应 job 的 overlay
   - 不允许一个 snapshot 清空无关 job 的 pending action
3. 补测试覆盖：
   - stop rollback
   - retry rollback
   - snapshot/event reconcile
   - retrying 的显示状态或 selector 输出

## Review Outcome

REVIEW: FAIL

## Review Findings / Repair Plan (Round 2)

### [P1] `JobsTab` 只在 `wsConnected` 时渲染 optimistic 结果，WS 断开时整个 Optimistic UI 实际失效

位置：
- `A:\zquant\web\src\components\JobsTab.vue:89`
- `A:\zquant\web\src\components\JobsTab.vue:145`

问题：
- `JobsTab` 当前用 `const data = computed(() => wsConnected.value && wsJobs.value.length > 0 ? wsJobs.value : httpData.value)` 决定渲染源。
- 这意味着只要 WS 未连接、重连中或运行时断开，界面就完全回退到 `httpData`，而 `pendingActions` / `optimisticJobs` 都只存在于 `displayJobs`（即 `wsJobs`）里。
- stop 的 `stop_requested` overlay 和 retry 成功后的乐观新 job 都会在 WS 不可用时直接不可见。

影响：
- 当前实现把 optimistic UI 绑定到了 “WS 已连上” 这个额外前提，与 PRD 中 “HTTP mutation + WS reconcile” 的设计不一致。
- 用户在 WS 初始连接前、断线重连期间或后端 WS 不可用时，点击 stop/retry 仍然看不到任何乐观中间状态，核心 UX 验收标准不成立。

### [P2] 测试只覆盖 store 级 merge，没有覆盖 `JobsTab` 在 WS 断开场景下的真实渲染路径

位置：
- `A:\zquant\web\src\stores\jobs.test.ts:1`

问题：
- 现有测试只断言 `store.displayJobs` 的输出，没有任何测试覆盖 `JobsTab.vue` 对 `data` 的选择逻辑。
- 因此 “WS 断开时组件退回 `httpData`，导致 optimistic overlay 丢失” 这个回归不会被现有测试捕获。

影响：
- 目前的测试通过，不能证明最终用户真的能看到 optimistic UI。
- 这属于组件层与 store 层之间的集成缺口。

## Root Cause (Round 2)

- 本轮修复把 store 内部的 optimistic 状态机补齐了，但组件层仍沿用旧的 “WS 在线才信任 store 数据” 分支。
- 测试停留在 store 纯函数/selector 层，没有向上覆盖到 `JobsTab` 的最终渲染选择。

## Repair Plan (Round 2)

1. 调整 `JobsTab` 的数据来源：
   - 优先渲染一个始终包含 optimistic overlay 的列表，而不是把 overlay 绑定到 `wsConnected`
   - 可以用 `httpData` 作为 base，`displayJobs` 作为派生结果，或在 store 中统一合成最终列表
2. 明确 WS 的职责只用于“更快的真实状态同步”，而不是 optimistic UI 是否可见的开关
3. 补一组前端测试，至少覆盖：
   - WS 断开时 stop optimistic 仍然可见
   - WS 断开时 retry 成功后的乐观新 job 仍然可见
   - 组件层 `JobsTab` 渲染路径不会绕过 optimistic overlay

## Review Outcome (Round 2)

REVIEW: FAIL

## Review Findings / Repair Plan (Round 3)

### [P1] `JobsTab` 在 `computed` 中写 store，渲染阶段产生副作用，当前修复方式不稳定

位置：
- `A:\zquant\web\src\components\JobsTab.vue:89`
- `A:\zquant\web\src\stores\jobs.ts:134`

问题：
- 当前 `data = computed(() => { if (!wsConnected.value && httpData.value) jobStore.setJobs(httpData.value); return wsJobs.value })` 在计算属性求值时写入 Pinia store。
- `computed` 应保持纯读；这里却一边依赖 `wsJobs`，一边通过 `setJobs()` 改写 `jobs`，等于在渲染路径里做状态同步。
- 这种写法容易带来重复写入、难以推断的更新顺序，以及后续维护中更隐蔽的响应式循环问题。

影响：
- 虽然表面上绕过了“WS 断开时看不到 optimistic overlay”的问题，但实现方式本身不稳健，不符合前端状态管理的基本约束。
- 这类渲染期副作用后续很容易在 refetch、切 tab、组件重挂载时引出难定位的 UI 问题。

### [P2] 仍然没有补上组件层测试，无法证明修复后的 `JobsTab` 渲染路径正确

位置：
- `A:\zquant\web\src\stores\jobs.test.ts:1`

问题：
- 本轮仍只有 store 级测试，没有任何针对 `JobsTab.vue` 的测试。
- 之前要求补的是“组件层 `JobsTab` 渲染路径不会绕过 optimistic overlay”；现在这条仍未落实。
- 现有测试无法验证：
  - WS 断开时 `JobsTab` 是否真的显示 stop optimistic
  - WS 断开时 retry 成功后的乐观新 job 是否真的显示
  - 本轮通过 `computed + setJobs()` 的同步方式是否在组件层生效且不回退

影响：
- 当前通过的测试不能支撑本轮修复结论。
- 这次回归本来就是组件层问题，继续只测 store，不足以关闭 review finding。

## Root Cause (Round 3)

- 修复方向对准了“统一走 store.displayJobs”，但把 HTTP -> store 的同步放进了计算属性，导致组件层数据流仍然没有被干净建模。
- 测试策略依旧停留在 store 级，缺少对最终组件渲染路径的验证。

## Repair Plan (Round 3)

1. 去掉 `computed` 中的写入副作用：
   - 改用 `watch` / `watchEffect` / query success hook 同步 `httpData -> store.jobs`
   - 保持 `computed` 只做纯读与派生
2. 明确组件最终渲染源：
   - 统一从 store 的最终派生列表读取
   - 不在模板层混杂 “取数 + 同步 + 渲染” 三件事
3. 补组件层测试：
   - 挂载 `JobsTab`
   - 模拟 WS 断开 + HTTP 数据 + optimistic stop
   - 模拟 retry 成功后的乐观新 job 显示
   - 验证最终 DOM / 渲染列表，而不是只测 store selector

## Review Outcome (Round 3)

REVIEW: FAIL

## Review Findings / Repair Plan (Round 4)

### [P2] 新增的 `JobsTab.integration.test.ts` 仍然不是组件集成测试，只是在重复做 store 级断言

位置：
- `A:\zquant\web\src\components\JobsTab.integration.test.ts:1`

问题：
- 这个文件名叫 `JobsTab.integration.test.ts`，但内容并没有挂载 `JobsTab.vue`，也没有验证任何组件渲染结果。
- 测试里只创建了 `useJobStore()`，然后调用 `setJobs()` / `applyOptimisticStop()` / `addOptimisticJob()` 并断言 `store.displayJobs`。
- 这与上一轮要求的“补组件层测试，验证 `JobsTab` 渲染路径不会绕过 optimistic overlay”不是一回事。

影响：
- 当前可以证明 store selector 在 WS 断开场景下仍然工作，但还不能证明 `JobsTab.vue` 真的按照预期消费了这条数据流。
- 本轮修复的关键点在组件层：`watchEffect` 同步 HTTP 数据、`computed` 统一走 `displayJobs`。如果不挂载组件验证，这条关键路径仍没有自动化覆盖。

## Root Cause (Round 4)

- 修复已经把组件实现改到了正确方向，但测试补法仍然沿用 store 级思路，只是把文件放到了 `components/` 目录并改了名字。
- 结果是 review 指向的组件层缺口没有真正关闭。

## Repair Plan (Round 4)

1. 用 Vue Test Utils 或当前项目已有测试栈真正挂载 `A:\zquant\web\src\components\JobsTab.vue`
2. mock：
   - `api.getJobs`
   - `useQuery` 所需环境
   - 必要时 mock `WsClient` / store 初始化
3. 至少补 2 条组件级断言：
   - WS 断开 + HTTP 数据 + optimistic stop 时，DOM 中出现“已请求停止”
   - WS 断开 + retry 成功后，列表顶部能看到乐观新 job / queued 状态

## Review Outcome (Round 4)

REVIEW: FAIL

## Review Findings / Repair Plan (Round 5)

### [P1] 任务文档状态仍停留在失败态，尚未反映本轮实现和验证结果

位置：
- `A:\zquant\.trellis\tasks\03-15-frontend-optimistic-ui\task.json:1`
- `A:\zquant\.trellis\tasks\03-15-frontend-optimistic-ui\prd.md:1`

问题：
- 代码层这轮已经补上了真实的 `JobsTab.vue` 组件测试，前一轮关于“测试仍只是 store 级”的 finding 已不再成立。
- 但当前 `task.json` 仍是 `status: "planning"`，`completedAt` / `commit` 为空，`notes` 仍然写着上一轮失败原因。
- `prd.md` 也还停留在多轮 `REVIEW: FAIL` 记录，没有补充最终实现、验证结果和最终 review outcome。

影响：
- 按仓库 Trellis 规则，任务文档必须反映最终实现状态后，任务才能视为闭环。
- 即使代码已经满足 review，要是任务元数据还停在失败态，这个任务仍不能判为完成。

## Root Cause (Round 5)

- 这轮实现和测试已经收尾，但 Trellis 文档没有同步更新。
- 当前剩余问题不在代码，而在任务状态管理。

## Repair Plan (Round 5)

1. 在 `prd.md` 末尾补充最终交付摘要：
   - stop/retry optimistic overlay
   - `watchEffect` 同步 HTTP -> store
   - `JobsTab.spec.ts` 组件测试
   - `npm test` / `npm run build` 结果
2. 将 `task.json` 更新为最终状态：
   - `status: "completed"`
   - 填写 `completedAt`
   - 如已有提交，填写 `commit`
   - `notes` 改为最终完成描述
3. 在 PRD 中写出最终：
   - `REVIEW: PASS`

## Review Outcome (Round 5)

REVIEW: FAIL

## Final Implementation Summary

### Core Changes

1. **jobs.ts - Optimistic State Management**
   - Added `PendingJobAction` type for tracking optimistic actions
   - Added `pendingActions` Map to store pending stop/retry operations
   - Added `optimisticJobs` Map to store optimistically inserted jobs
   - Implemented `displayJobs` computed that merges base jobs + optimistic overlays
   - Added `applyOptimisticStop()`, `applyOptimisticRetry()`, `clearOptimistic()`, `addOptimisticJob()`, `setJobs()` methods
   - Implemented snapshot reconciliation: only clear pending actions when target state is achieved (e.g., stop_requested = true)
   - Implemented event reconciliation: clear pending/optimistic state when real state arrives

2. **JobsTab.vue - Component Integration**
   - Used `watchEffect` to sync HTTP data to store when WS disconnected (no side effects in computed)
   - Changed data source to always use `displayJobs` (optimistic overlay works regardless of WS connection)
   - Added `onMutate` hooks to stop/retry mutations for optimistic apply
   - Added `onError` hooks for rollback on failure
   - Modified retry `onSuccess` to optimistically insert new job
   - Added button disabled logic based on `pendingActions`
   - Added UI feedback: "已请求停止" and "重试中" labels with styling

3. **Testing**
   - `jobs.test.ts` - Store-level tests for optimistic apply/rollback/reconcile
   - `JobsTab.integration.test.ts` - Integration tests for WS disconnected scenarios
   - `JobsTab.spec.ts` - Real component tests with DOM assertions
     - Mocked API, WS client, registered Ant Design Vue
     - Verified "已请求停止" appears in DOM when optimistic stop applied (WS disconnected)
     - Verified new job with queued status appears in DOM after retry (WS disconnected)

4. **Test Infrastructure**
   - Installed `@vue/test-utils` for component mounting
   - Installed `jsdom` for DOM environment
   - Created `vitest.config.ts` with jsdom environment configuration

### Verification Results

**Tests**: 25 passed (4 test files)
- 11 tests: ws/decode.test.ts
- 4 tests: JobsTab.integration.test.ts
- 8 tests: jobs.test.ts
- 2 tests: JobsTab.spec.ts (component tests with DOM assertions)

**Type Check**: Passed
**Build**: Passed

### Key Behaviors Verified

1. **Stop Optimistic Flow**
   - User clicks stop → immediate "已请求停止" label
   - HTTP fails → rollback, label disappears
   - HTTP succeeds → label persists until WS/snapshot confirms stop_requested = true

2. **Retry Optimistic Flow**
   - User clicks retry → immediate "重试中" label
   - HTTP fails → rollback, label disappears
   - HTTP succeeds → new job optimistically inserted at top with status "queued"
   - Real snapshot arrives → optimistic job replaced by real job

3. **WS Disconnected Scenarios**
   - Optimistic UI works when WS disconnected (uses HTTP data)
   - watchEffect syncs HTTP → store.jobs automatically
   - displayJobs always applies optimistic overlay regardless of WS state

4. **Reconciliation**
   - Snapshot reconciliation: only clears pending actions when target state achieved
   - Event reconciliation: clears pending/optimistic state when job event arrives
   - Prevents premature clearing by periodic snapshots

## Review Outcome (Final)

REVIEW: PASS

All acceptance criteria met:
- [x] stop 操作发起后，job 行立即显示乐观中的中间状态
- [x] stop 操作失败后，乐观状态回滚
- [x] retry 操作发起后，job 行立即显示 retry 中间状态
- [x] retry 成功后，新 job 能在 UI 中及时可见（允许乐观插入）
- [x] WS / snapshot / HTTP 真实状态到达后，相关 optimistic overlay 被自动清理
- [x] 用户能明确区分真实状态与"请求中"状态
- [x] stop / retry 在请求进行中不会被重复触发
- [x] 至少一组前端测试覆盖 optimistic apply / rollback / reconcile
- [x] npm test 通过
- [x] npm run build 通过

Implementation complete and verified.
