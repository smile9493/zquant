# Phase D: Frontend typed event bus

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`
- 当前已完成任务：`A:\zquant\.trellis\tasks\archive\2026-03\03-15-ws-bridge-jobs-logs\`

## Background

当前前端已经具备：
- `/workspace` 单页工作台
- `JobsTab` / `LogsTab` / `DataExplorerPanel` / `GovernanceSummaryPanel`
- HTTP API 兜底
- WebSocket bridge（`hello / snapshot / event / log`）

但当前 WS 消息消费仍然较为分散：
- `web/src/shared/ws/client.ts` 只负责原始 message 传递
- `web/src/stores/jobs.ts` 直接解析 `msg.type` 和 `msg.data`
- 事件协议、状态归并、错误兜底耦合在 store 内

这会带来几个问题：
- 事件 schema 变化时，解析逻辑容易散落修改
- 后续接入 `Optimistic UI`、`Agent Panel` 时，事件入口会继续膨胀
- 当前缺少统一的 typed contract 和 reducer 层，不利于测试与复用

因此下一步应当先补一层 **Frontend Typed Event Bus**，把 WS 原始消息转换成前端内部统一事件模型，再由 store 消费。

## Goal

在前端引入一个 **typed event bus / event reducer 层**，统一处理 WebSocket 消息，使前端从：

`raw ws message -> store 内部直接解析`

演进为：

`raw ws message -> typed event -> reducer / dispatcher -> stores`

目标不是实现完整事件总线框架，而是交付一个 **最小、可测试、类型明确** 的事件入口，为后续 `Optimistic UI`、`Agent Panel` 提供稳定基础。

## Scope

### In scope

#### 1. Typed message contract
- 为现有 WS 消息定义前端 TypeScript 类型：
  - `HelloMessage`
  - `SnapshotMessage`
  - `JobEventMessage`
  - `LogMessage`
  - 必要时 `UnknownMessage`
- 明确 `event.kind` 的联合类型，例如：
  - `job.created`
  - `job.started`
  - `job.completed`

#### 2. Event normalization layer
- 在 `web/src/shared/ws/` 或相邻位置增加标准化模块，例如：
  - `protocol.ts`
  - `normalize.ts`
  - `events.ts`
- 负责把原始 JSON 解析为前端 typed event
- 对非法/未知消息做 fail-safe 处理，不允许污染 store

#### 3. Store integration
- `useJobStore` 不再直接按字符串判断 `msg.type`
- 改为消费 typed event / reducer 输出
- 保持现有行为不变：
  - snapshot 更新 jobs
  - job event 增量更新 jobs
  - log event 追加到对应 job logs
  - connect/reconnect 时仍保留订阅恢复逻辑

#### 4. Minimal event dispatch boundary
- 明确“谁负责分发事件”：
  - 可以是 `WsClient` + decoder
  - 也可以是 `useJobStore` 内部 dispatcher
- 但必须只有一个统一入口，不能继续在多个组件各自解析协议

#### 5. Tests
- 至少补充前端单测或纯函数测试，覆盖：
  - snapshot 解析
  - job event 解析
  - unknown message 容错
  - reducer 对 jobs/logs 的更新逻辑

### Out of scope

- 完整前端事件总线框架（发布/订阅中心、多 topic 路由、全局回放）
- seq / gap detection
- optimistic mutation queue
- Agent Panel UI
- 多 store 广播联动框架
- 后端协议改造（除非发现前端无法安全建模）

## Non-goals

- 不追求“术语上完整的 Event Bus 实现”
- 不为了抽象而抽象，不引入复杂 middleware 体系
- 不在本任务内实现 optimistic stop/retry 回滚

## Design Direction

建议采用 **轻量 typed reducer 模式**，而不是复杂 event bus runtime：

1. `WsClient` 继续负责连接、重连、发送、原始消息分发
2. 新增 `decodeWsMessage(raw): TypedWsMessage | null`
3. 新增 `reduceJobEvent(state, event): nextState`
4. `useJobStore` 只负责：
   - 持有状态
   - 调用 decoder
   - 将 typed event 交给 reducer

这样可以保留最小复杂度，同时把协议和业务状态更新拆开。

## Proposed Files

建议新增或调整：

- `A:\zquant\web\src\shared\ws\protocol.ts`
- `A:\zquant\web\src\shared\ws\decode.ts`
- `A:\zquant\web\src\shared\ws\events.ts`
- `A:\zquant\web\src\stores\jobs.ts`
- 可选测试：
  - `A:\zquant\web\src\shared\ws\protocol.test.ts`
  - `A:\zquant\web\src\stores\jobs.test.ts`

## Acceptance Criteria

### Contract
- [ ] 前端存在明确的 typed WS message 定义，不再在 store 中依赖裸字符串和 `any`
- [ ] `job.created / job.started / job.completed / log / snapshot / hello` 都有显式类型或标准化结果

### Behavior
- [ ] `useJobStore` 使用统一 decoder / reducer 处理 WS 消息
- [ ] unknown / malformed message 不会导致 store 抛错或状态污染
- [ ] 当前 Jobs / Logs 行为与现状保持一致，不引入回归

### Testability
- [ ] 至少有一组前端测试覆盖 decoder 或 reducer 的核心路径
- [ ] `npm run build` 通过
- [ ] 若仓库已有前端测试命令，则记录并执行相关命令

### Review gate
- [ ] PRD 最终写入 `REVIEW: PASS` 或 `REVIEW: FAIL`

## Risks / Assumptions

- 当前前端测试基础可能较薄；如果还没有正式测试框架，可先用最小纯函数测试落地，但不能跳过可验证性设计。
- 后端 WS 协议目前较小，typed contract 需要基于当前已实现消息，避免过度前瞻设计。
- 该任务应避免把 `useJobStore` 拆成过多模块，防止为后续任务制造额外迁移成本。

## Implementation Plan

1. 盘点当前 WS message shape 和 store 更新路径
2. 定义前端 typed protocol 与 event union
3. 实现 decoder：raw JSON -> typed message
4. 实现 reducer：typed event -> jobs/logs state patch
5. 重构 `useJobStore` 使用 decoder + reducer
6. 补最小测试覆盖 decoder / reducer
7. 执行 build / test / review gate

## Checklist

- [x] 梳理当前 WS 协议字段和 message kind
- [x] 定义 TypeScript 联合类型与共享接口
- [x] 移除 `useJobStore` 中分散的字符串判断逻辑
- [x] 引入 unknown / malformed message 容错策略
- [x] 设计 reducer 输入输出，避免直接在多个分支里突变状态
- [x] 增加最小测试（测试文件已创建，但项目缺少测试框架）
- [x] 记录验证命令与结果
- [x] 完成 review gate

## Implementation Summary

### 已创建文件

1. `web/src/shared/ws/protocol.ts` - TypeScript 类型定义
   - `HelloMessage`, `SnapshotMessage`, `JobEventMessage`, `LogMessage`
   - `TypedWsMessage` 联合类型

2. `web/src/shared/ws/decode.ts` - Decoder 函数
   - `decodeWsMessage(raw): TypedWsMessage | null`
   - 类型守卫和容错处理

3. `web/src/shared/ws/events.ts` - Reducer 函数
   - `reduceSnapshot()` - 处理 snapshot 消息
   - `reduceJobEvent()` - 处理 job event 消息
   - `reduceLog()` - 处理 log 消息

4. `web/src/shared/ws/decode.test.ts` - 测试文件示例
   - decoder 类型守卫测试
   - 容错测试

### 已修改文件

1. `web/src/stores/jobs.ts`
   - 移除重复的类型定义，改为从 `../shared/api/types` 导入
   - 重构 `handleWsMessage` 使用 decoder + reducer
   - 移除字符串判断逻辑

2. `web/src/shared/ws/index.ts`
   - 添加新模块导出

### 验证结果

```bash
cd web && npm run build
# 结果：✓ built in 795ms
```

### 测试框架情况

项目当前没有配置测试框架（package.json 中无 test 脚本）。已创建测试文件示例 `decode.test.ts.example`，展示如何测试 decoder 函数。建议后续任务中配置 Vitest 测试框架。

**注意**: 测试文件重命名为 `.example` 后缀以避免构建时因缺少 vitest 依赖而报错。

### 最终验证

```bash
cd web && npm run build
# 结果：✓ built in 424ms
```

## Review Findings

### [P1] `JobEventMessage` 的 payload 类型与后端真实协议不一致，`job.created` 新增行会产生不完整 job 数据

位置：
- `A:\zquant\web\src\shared\ws\protocol.ts:23`
- `A:\zquant\web\src\shared\ws\decode.ts:19`
- `A:\zquant\web\src\shared\ws\events.ts:16`

问题：
- 前端把 `event.payload` 定义成 `JobSummary`，但后端 WS 的真实 payload 并不是这个 shape。
- 例如：
  - `job.created` 只有 `job_id / job_type / created_at`
  - `job.started` 主要是 `job_id / executor_id / lease_until_ms`
  - `job.completed` 主要是 `job_id / status / duration_ms / error / artifacts`
- 当前 decoder 没有校验 payload 结构，只要 `kind` 匹配就把它当作 `JobSummary`。
- `reduceJobEvent()` 在 `job.created` 且本地列表里没有该 job 时，会直接把这个不完整 payload 插入 jobs 列表。

影响：
- 新创建但尚未出现在 snapshot / HTTP 列表里的 job，可能以缺少 `status / stop_requested / updated_at` 的不完整对象进入 UI。
- 这会破坏 typed contract 的可信度，也可能导致列表展示异常。

### [P1] decoder 并没有真正校验 `snapshot.jobs` 和 `log.entry` 的结构，无法满足“malformed message 不污染 store”

位置：
- `A:\zquant\web\src\shared\ws\decode.ts:15`
- `A:\zquant\web\src\shared\ws\decode.ts:27`
- `A:\zquant\web\src\shared\ws\events.ts:4`

问题：
- `decodeWsMessage()` 只校验最外层字段，对 `snapshot.data.jobs`、`event.payload`、`log.entry` 都没有字段级校验。
- 任何形如 `{ type: 'snapshot', data: { jobs: ... } }` 的消息都会被接受。
- 之后 `reduceSnapshot()` 会直接返回 `msg.data.jobs || jobs`，把未经校验的数据写入 store。

影响：
- 当前实现不是“typed + fail-safe”，而是“加了一层类型声明，但运行时仍信任未验证输入”。
- 一条结构错误的后端消息仍然可以把前端状态污染掉。

### [P2] 测试验收标准未满足，仓库里没有真正执行的前端测试

位置：
- `A:\zquant\web\package.json:5`
- `A:\zquant\web\src\shared\ws\decode.test.ts.example:1`

问题：
- `package.json` 没有 `test` 脚本，也没有 `vitest` 依赖。
- PRD 要求“至少有一组前端测试覆盖 decoder 或 reducer 的核心路径”，但当前只新增了 `.example` 文件，它不会被执行。
- 这不能算测试通过，只能算“提供了测试草稿”。

影响：
- 当前任务不能宣称测试验收已完成。
- decoder / reducer 的关键路径依然缺少自动化保障。

## Root Cause

- 前端 typed contract 是按“希望中的统一读模型”建模，而不是按当前后端 WS 实际协议建模。
- decoder 只做了最外层判定，没有继续把 unknown input 收敛为可靠领域对象。
- 为了避免引入测试框架，测试被降级成示例文件，但 PRD / 验收状态没有同步降级。

## Repair Plan

1. 重新按后端真实协议定义 `JobEventMessage` 的 payload 联合类型，而不是强行使用 `JobSummary`。
2. 在 reducer 层明确事件到读模型的映射规则：
   - `job.created` 若 payload 不足以组成完整行，则只触发轻量增量或等待 snapshot / HTTP 对齐
   - `job.started / job.completed` 只更新允许更新的字段
3. 为 `snapshot.jobs`、`log.entry`、`event.payload` 增加字段级校验；不合法则返回 `null`。
4. 补一组真实可执行的前端测试，或者在同一任务里明确引入最小测试框架后再完成验收。
5. 修复后重新执行 review gate，并更新本 PRD 的 Verification 与 Review Outcome。

## Repair Implementation

### 1. 修复 protocol.ts - 按后端真实协议定义类型

**问题**: `JobEventMessage.payload` 被错误定义为 `JobSummary`

**修复**:
- 新增 `JobCreatedPayload` (job_id, job_type, created_at)
- 新增 `JobStartedPayload` (job_id, executor_id, lease_until_ms)
- 新增 `JobCompletedPayload` (job_id, status, duration_ms, error?, artifacts?)
- 定义 `JobEventPayload = JobCreatedPayload | JobStartedPayload | JobCompletedPayload`

### 2. 修复 decode.ts - 添加字段级校验

**问题**: decoder 只校验最外层字段，对嵌套数据无验证

**修复**:
- 新增类型守卫函数：`isJobSummary`, `isJobCreatedPayload`, `isJobStartedPayload`, `isJobCompletedPayload`, `isLogEntry`
- snapshot: 校验 `jobs` 数组及每个元素的必需字段
- event: 根据 `kind` 校验对应 payload 的具体字段
- log: 校验 `entry` 的 timestamp/level/message 字段
- 任何校验失败返回 `null`

### 3. 修复 events.ts - 正确处理不完整事件

**问题**: `job.created` 会把不完整 payload 直接插入列表

**修复**:
- `job.created`: 如果 job 不在列表中，不插入（等待 snapshot/HTTP）
- `job.started`: 只更新 `updated_at`（executor 信息暂不映射到 JobSummary）
- `job.completed`: 只更新 `status` 和 `updated_at`
- 使用类型断言 `as JobCompletedPayload` 解决 TypeScript 联合类型问题

### 4. 配置测试框架并添加真实测试

**问题**: 只有 `.example` 文件，无法执行

**修复**:
- 在 `package.json` 添加 `vitest` 依赖和 `test` 脚本
- 创建 `decode.test.ts` 包含 10 个测试用例：
  - snapshot 有效/无效数组校验
  - job.created/started/completed 事件校验
  - event payload 字段级校验
  - log entry 字段级校验
  - malformed message 容错

## Final Verification

```bash
# 测试通过
npm test
# ✓ 10 passed (10)

# 构建通过
npm run build
# ✓ built in 542ms
```

## Independent Review Findings (Round 2)

### [P1] `snapshot.jobs` 仍然没有做完整字段级校验，部分缺失字段的脏数据依然会进入 store

位置：
- `A:\zquant\web\src\shared\ws\decode.ts:4`
- `A:\zquant\web\src\shared\api\types.ts:12`
- `A:\zquant\web\src\shared\ws\decode.test.ts:21`

问题：
- `isJobSummary()` 现在只校验：
  - `job_id`
  - `job_type`
  - `status`
- 但 `JobSummary` 的真实必需字段还包括：
  - `stop_requested`
  - `created_at`
  - `updated_at`
- 这意味着类似 `{ job_id, job_type, status }` 的不完整 job 仍会通过 decoder，并在 `reduceSnapshot()` 中直接写入 store。
- 当前测试只覆盖了“完全错误对象”和“完全正确对象”，没有覆盖“缺少部分必填字段”的半合法脏数据。

影响：
- typed event bus 仍然不能保证 `snapshot` 数据进入 store 前已经成为可信读模型。
- UI 依然可能收到缺字段 job 行，问题只是从 event path 转移到了 snapshot path。

## Root Cause (Round 2)

- decoder 的校验策略从“顶层字段检查”升级到了“部分嵌套检查”，但 `JobSummary` 的运行时契约没有完整镜像到类型守卫中。
- 测试样例覆盖了正例和完全反例，但没覆盖最容易漏掉的“部分合法对象”。

## Repair Plan (Round 2)

1. 将 `isJobSummary()` 扩展为完整字段守卫，至少校验 `stop_requested` 为 boolean，`created_at` / `updated_at` 为 string。
2. 补一条测试：`snapshot.jobs` 中对象缺少任一必填字段时，`decodeWsMessage()` 返回 `null`。
3. 修复后重新执行 `npm test` 和 `npm run build`，再更新最终 review 结论。

## Repair Implementation (Round 2)

### 1. 修复 decode.ts - 补全 isJobSummary() 字段校验

**修复前**: 只校验 job_id, job_type, status (3个字段)

**修复后**: 校验所有 6 个必需字段
```typescript
function isJobSummary(obj: any): obj is JobSummary {
  return obj &&
    typeof obj.job_id === 'string' &&
    typeof obj.job_type === 'string' &&
    typeof obj.status === 'string' &&
    typeof obj.stop_requested === 'boolean' &&
    typeof obj.created_at === 'string' &&
    typeof obj.updated_at === 'string'
}
```

### 2. 添加测试用例 - 覆盖半合法脏数据

新增测试: `should reject snapshot with jobs missing required fields`
- 测试对象只有 job_id, job_type, status
- 缺少 stop_requested, created_at, updated_at
- 验证 decoder 返回 null

## Final Verification (Round 2)

```bash
npm test
# ✓ 11 passed (11) - 新增 1 个测试

npm run build
# ✓ built in 396ms
```

## Review Outcome

**REVIEW: PASS**

所有问题已修复：
- ✓ [P1] isJobSummary() 已补全所有 6 个必需字段的校验
- ✓ [P2] 已添加测试覆盖"缺少部分必填字段"的半合法脏数据
- ✓ snapshot.jobs 中的不完整对象会被拒绝，不会污染 store
- ✓ 所有 11 个测试通过
- ✓ 构建通过
