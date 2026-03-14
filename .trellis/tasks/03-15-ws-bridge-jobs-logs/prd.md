# Phase D: WebSocket bridge (jobs/logs)

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md` → Phase D（后续增强）：WebSocket bridge

## Background

当前前端工作台已基于 HTTP + PG SSOT 跑通（`/workspace` + Jobs/Logs/DataExplorer/Watchlist），并通过轮询刷新：
- `GET /jobs`（jobs 列表）
- `GET /jobs/:id/logs`（日志，当前后端为 placeholder：空数组）
- `GET /system/health`（健康状态）

下一步（Phase D）要把“轮询为主”升级为“推送为主 + 断线降级轮询”，为后续 Typed Event Bus / Optimistic UI 打基础，但不把这些作为本任务前置。

## Goal

交付一个 **最小 WebSocket Bridge**，让 Workspace 能实时收到：
- jobs 状态变化（created/started/completed/stop/retry 等影响 UI 的变化）
- logs/事件流（至少能把与 job 相关的事件转换为可展示的 log entry）

并且保持：
- HTTP API 仍然可用（兼容现有轮询/手动刷新）
- WS 不可用时自动降级回轮询，不阻塞 UI 使用

## Scope

### In scope

#### Backend（Rust）
- 在现有 API Router 上增加 WS endpoint（建议：`GET /ws`）
- 基于 `job_events::bus::EventBus` 订阅事件并向 WS 客户端推送（JSON 文本消息）
- 连接建立后发送一次 snapshot（至少：health + jobs list）
- 支持客户端订阅某个 `job_id` 的日志流（最小：将 bus 中与 job 相关事件映射为 `LogEntry`，并推送）
- 断连/慢消费者处理（有界缓冲 + 丢弃/断开策略，避免 OOM）

#### Frontend（Vue）
- 增加一个 WS 客户端桥接层（例如 `shared/ws` 或 `stores/jobs` 内部）
- WS 连接成功后：降低/关闭 jobs+logs 的轮询刷新；WS 断开时：恢复轮询
- UI 端显示最小连接状态（例如：connected / reconnecting / disabled），并不影响手动刷新按钮

### Out of scope

- Kafka / Redis
- 前端 Typed Event Bus / Gap Detection / Reconciler（仅做最小 bridge）
- 完整的乐观 UI 状态机（stop/retry 的最终一致性依旧以 PG 为准）
- 权限/鉴权体系（默认内网/开发环境）

## Proposed API / Protocol

### Endpoint

- `GET /ws`（WebSocket）

### Message format（v1，文本 JSON）

统一外层 envelope，保证可扩展：
- `v`: 协议版本（固定 `1`）
- `type`: `"hello" | "snapshot" | "event" | "log" | "error" | "ping" | "pong"`
- `ts`: ISO 时间字符串（或 RFC3339）
- `data`: payload

建议约定（示例）：
- `hello`: `{ server: "job-kernel", schema_v: "1.0" }`
- `snapshot`: `{ health, jobs: JobSummary[] }`
- `event`: `{ kind, payload }`（kind 如 `job.created|job.started|job.completed|agent.message_produced`）
- `log`: `{ job_id, entry: { timestamp, level, message } }`

### Client → server（最小订阅协议）

客户端发：
- `{"v":1,"type":"subscribe","data":{"job_id":"job_xxx"}}`：表示要接收该 job 的 log 流（不提供则仅收 jobs/snapshot）

（注：若实现简单，也可不做 subscribe，直接推送所有 job 相关 log；但需评估噪声与性能）

## Design Notes / Decisions

### 事件来源选择

优先方案：**直接订阅 `EventBus`**（当前 `job_application::ApiState` 已包含 `bus: Arc<dyn EventBus>`）。
- 优点：实时、实现成本最低
- 风险：`EventBus` 是 in-memory；若未来拆分 `job-api` 与 `job-runner` 为独立进程，WS 只能看到本进程发布的事件

兼容性策略（写进实现/文档）：
- WS 事件流视为“加速层”，PG 仍是 SSOT
- 客户端必须可在重连后通过 HTTP snapshot 对齐状态

### logs 最小可用定义

由于 `GET /jobs/:id/logs` 当前为 placeholder，本任务对 logs 的“最小可用”定义为：
- 将 `AgentMessageProduced` 等事件映射为 `LogEntry` 并通过 WS 推送（可先不落库）
- 允许后续任务把 logs 持久化到 PG，并让 HTTP logs endpoint 变为真实数据源

## Acceptance Criteria

### Backend
- [ ] WS endpoint 存在并可连接（建议路径：`/ws`）
- [ ] 连接后会收到 `hello` + `snapshot`（snapshot 至少包含 `health` 与 `jobs` 列表）
- [ ] 当通过 HTTP 创建 job（`POST /jobs`）后，WS 客户端可收到对应的 job 事件或下一次 snapshot 反映该变化
- [ ] 支持订阅某个 `job_id` 的日志流（最小：能收到与该 job 相关的 `log` 消息）
- [ ] 慢消费者/断开连接不会导致服务 panic 或无界内存增长

### Frontend
- [ ] WS 可用时：JobsTab/LogsTab 能在不依赖 5s 轮询的情况下更新（允许保留低频兜底轮询）
- [ ] WS 不可用时：自动降级回轮询，UI 功能不受影响
- [ ] `npm run build` 通过（`A:\zquant\web`）

### Review gate
- [ ] 后端相关 `cargo test` / `cargo clippy` 通过（明确在“Verification”记录）
- [ ] Review Outcome 最终记录为 `REVIEW: PASS` 或 `REVIEW: FAIL`

## Implementation Plan (Planning Only)

1. Backend：在 `crates/job-application` 的 router 增加 `GET /ws`，实现 WS upgrade 与连接生命周期管理
2. Backend：为每个 WS 连接创建订阅任务：`bus.subscribe()` → 序列化为 JSON → 发送到 socket（有界 channel）
3. Backend：实现 `subscribe(job_id)`：维护连接的订阅集合，仅转发匹配 job_id 的 logs/event
4. Backend：实现 snapshot：连接建立时通过 `store.list_jobs()` + `get_health()` 组装一次性 snapshot
5. Frontend：实现 `WsBridge`（connect/reconnect/backoff + message dispatch）
6. Frontend：将 `useJobStore` / `LogsTab` 接入 WS（ws 更新 store；轮询做兜底）
7. Tests：后端增加最小 ws 集成测试（至少：connect→snapshot；create job→收到 event）
8. Review：跑全量检查并在本 PRD 写入 verification 结果与 review outcome

## Checklist

- [ ] 确认 ws URL 约定（同源：`ws(s)://<host>/ws`），并明确 dev/prod 配置项
- [ ] 确认 Event → message 的 mapping（哪些事件映射成 logs，哪些映射成 job event）
- [ ] 确认 snapshot 的字段与现有 HTTP shape 一致（复用 `JobSummary` / `HealthResponse`）
- [ ] 确认慢消费者策略（drop + metrics + optional disconnect）
- [ ] 写后端 ws 测试计划（可用 tokio-tungstenite 或 axum test client）
- [ ] 前端断线降级策略写清楚（轮询 interval、重连 backoff、UI 提示）
- [ ] Review gate（记录命令与结果）

## Review Findings / Repair Plan

（待实现后补充：若 review fail，必须把 findings 与 repair plan 写在本文件内）

## Review Outcome

（待实现后补充）

