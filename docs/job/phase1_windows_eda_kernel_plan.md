# Phase 1：Windows 单机 EDA 内核（目标规划）

> 本文档基于 `docs/job/rust_job_module.md` 的总蓝图，定义 **Phase 1（Windows 单机）** 的可交付范围与落地路线。
> 目标是让 **Agent 编排已经是 EDA**，只是事件总线暂时在本机内存中，后续可平滑替换为 Kafka/WS Bridge/Redis 投影。

## 1. Phase 1 目标与非目标

### 1.1 目标（Goals）
- 在 Windows 单机上跑通最小 EDA 内核：`job-api` + `job-runner` + `PostgreSQL` + **in-memory event bus**。
- 事件仍然存在并驱动编排，但通过本地总线传播：
  - `JobCreated`
  - `JobStarted`
  - `JobCompleted`
  - `AgentSpawnRequested`
  - `AgentMessageProduced`
  - `AgentTaskScheduled`
- 以 PostgreSQL 为唯一状态真相源（SSOT）：Job 状态、租约、终态写入必须可独立运行。
- 为后续 Phase（Redis/Kafka/WS Bridge）预留清晰扩展点：事件契约、发布/订阅接口不因实现替换而破坏上层。

### 1.2 非目标（Non-goals）
- 不引入 Kafka（事件流/唤醒信号全部先走本地总线）。
- 不引入 Redis（热路径缓存/投影先不上）。
- 不提供 WebSocket Bridge（客户端订阅先不做）。
- 不做分布式 Runner、多机容灾；Phase 1 只保证单机可靠执行语义。

## 2. 进程模型（单机内存总线约束）

**关键约束**：in-memory event bus 只能在同一进程内共享。

Phase 1 推荐采用 **单进程“内核”运行形态**：
- 一个进程内同时启动：
  - HTTP `job-api`（对外接口）
  - `job-runner`（执行循环 + Agent 编排）
  - in-memory event bus（统一发布/订阅）
- 逻辑上仍保留 `job-api` / `job-runner` 两个“组件边界”，只是不强制进程隔离。

> 如果一定要拆成两个独立进程，则“in-memory bus”不成立，需要改成本地 IPC（如 TCP/NamedPipe/PG LISTEN/NOTIFY）。该拆分留到后续 Phase 再做。

## 3. 组件职责（Phase 1）

### 3.1 job-api（HTTP）
- 创建任务：写入 PG（幂等可选）并发布 `JobCreated`。
- 查询任务：优先读 PG（后续再加 Redis cache）。
- 控制任务：`stop/retry` 写入 PG 状态/标记，并发布对应事件（Phase 1 可先最小化）。

### 3.2 job-runner（执行面）
- 从 PG 原子 Claim `queued` 任务并写入 `running + lease`。
- 执行 Handler（超时、panic 隔离）并最终 `Finalize` 终态。
- 发布生命周期事件（本地总线）驱动 Agent 编排：
  - `JobStarted` / `JobCompleted`
  - `AgentSpawnRequested` / `AgentTaskScheduled` / `AgentMessageProduced`
- Sweep：定时回收租约过期任务、清理幂等记录（Phase 1 可先实现 lease sweep）。

### 3.3 PostgreSQL（状态真相源）
- jobs：状态机、租约、执行者信息、终态数据。
- jobs_idempotency：幂等键（Phase 1 可先实现最小版本，后续补 expires/sweep）。

### 3.4 In-memory event bus（本地总线）
- 提供发布/订阅能力，支撑：
  - 创建后唤醒 runner（等价于未来的 `dispatch topic`）
  - 生命周期广播（等价于未来的 `lifecycle topic`）
  - Agent 编排事件（Phase 1 的核心）

## 4. 事件总线设计（可替换实现）

### 4.1 设计原则
- **发布与主事务解耦**：PG 写入成功是主链路成功标准；总线发布失败不能回滚数据库事务。
- **事件契约稳定**：事件结构先定稿，后续替换 Kafka/WS 时不改字段语义。
- **订阅是 best-effort**：允许订阅端漏消费（Phase 1 不做持久化补偿），但必须可观测（日志/指标）。

### 4.2 建议接口形态（Rust）
- `EventBus` trait：`publish(event)` + `subscribe()`。
- 本地实现建议用：
  - `tokio::sync::broadcast` 做 fanout（生命周期/编排事件广播）
  - 必要时对特定“唤醒信号”用 `Notify` 或 `watch` 进一步降开销

## 5. Phase 1 事件契约（本地总线）

> 事件类型先以 Rust enum/struct 表达；后续接 Kafka 时再做 Envelope（event_id/ts/producer_id）。

### 5.1 JobCreated
- 触发：API 创建任务成功提交后
- 作用：唤醒 runner 尽快 Claim（等价未来 dispatch 信号）
- 最小字段：`job_id`, `job_type`, `created_at`

### 5.2 JobStarted
- 触发：runner Claim 并将状态写入 `running` 成功后
- 最小字段：`job_id`, `executor_id`, `lease_until_ms`

### 5.3 JobCompleted
- 触发：runner Finalize 终态后（done/error/stopped/reaped）
- 最小字段：`job_id`, `status`, `duration_ms`, `error?`, `artifacts?`

### 5.4 AgentSpawnRequested
- 触发：由 JobStarted 或特定 JobHandler 逻辑触发
- 作用：请求生成一个 Agent（本机 tokio task）
- 最小字段：`agent_id`, `job_id`, `agent_kind`, `init_payload`

### 5.5 AgentTaskScheduled
- 触发：编排层将一个子任务/步骤安排给 Agent
- 最小字段：`agent_id`, `task_id`, `task_payload`, `deadline?`

### 5.6 AgentMessageProduced
- 触发：Agent 执行过程中产生消息（日志、工具输出、模型响应等）
- 作用：供编排层聚合、写库（可选）、或未来 WS 推送
- 最小字段：`agent_id`, `job_id`, `message_type`, `content`, `ts`

## 6. 最小业务流程（Phase 1）

### 6.1 创建到执行（Create → Claim → Execute → Finalize）
1. `job-api`：`POST /jobs` 写入 PG（status=queued）。
2. 提交成功后发布 `JobCreated`（本地 bus）。
3. `job-runner`：
   - 收到 `JobCreated` 立即尝试 Claim（失败则退避轮询）。
   - Claim 成功后写入 `running + executor_id + lease_until_ms`，发布 `JobStarted`。
4. `job-runner` 执行对应 `JobHandler`：
   - 超时控制（tokio timeout）
   - panic 隔离（catch_unwind）
5. `job-runner` Finalize 终态（done/error/stopped/（可选 reaped）），发布 `JobCompleted`。

### 6.2 Agent 编排（EDA）
- `JobStarted` / handler 逻辑 → 发布 `AgentSpawnRequested`。
- 编排层订阅 `AgentSpawnRequested`，spawn tokio task 作为 Agent 实例。
- 编排层根据策略发布 `AgentTaskScheduled` 推进 Agent 执行。
- Agent 产生消息发布 `AgentMessageProduced`，编排层聚合并更新 Job progress（可选）。

## 7. 数据与状态机（Phase 1 最小集）

### 7.1 Job 状态
最小对外状态建议仍沿用蓝图主状态：
- `queued` → `running` → `done|error|stopped`
- `reaped`（可选）：Phase 1 可先预留字段与枚举，但行为可后置到 Phase 1.5/2。

### 7.2 租约（Lease）
Phase 1 最小实现：
- Claim 时设置 `lease_until_ms = now + lease_duration_ms`
- 心跳续租（可选）：Phase 1 可先不做 heartbeat_loop，但至少要有 lease_expired sweep 兜底
- sweep 回收：发现 `running` 且 lease 过期 → 标记为 `reaped` 或直接 `stopped`（按蓝图最终收敛到 stopped）

> fencing（`lease_version` 递增 + 校验）是蓝图 P0 能力，建议 Phase 1 直接做，否则后续补会牵扯到 store 接口与 SQL 变更。

## 8. API（Phase 1 最小）

建议最小接口（先保证主链路可用）：
- `POST /jobs`：创建任务（支持 `job_type + payload + idempotency_key?`）
- `GET /jobs/{id}`：查询

可选（Phase 1.1）：
- `POST /jobs/{id}/stop`
- `POST /jobs/{id}/retry`

## 9. 里程碑（可交付拆分）

### M1：Schema + Store（P0）
- jobs/jobs_idempotency 迁移脚本完善（索引/约束/字段：stop_reason、lease_version、reaped 等）。
- PG 原子 Claim + Finalize（带 fencing 校验）。

### M2：本地事件总线（P0）
- `EventBus` trait + in-memory 实现（broadcast）。
- `job-api` 创建后发布 `JobCreated`。
- `job-runner` 订阅唤醒并 Claim，发布 `JobStarted/JobCompleted`。

### M3：Runner 执行与 Agent 编排（P0）
- Handler 注册表（fail-fast、禁止重复 job_type）。
- 执行超时/隔离。
- Agent：spawn、schedule、message 事件闭环跑通（本机）。

### M4：可观测性与回归（P1）
- tracing 贯通、关键指标（claim 延迟、执行耗时、completed 计数）。
- 集成测试：PG 真库（Docker/本机服务），覆盖 Create→Execute→Finalize 主链路。

## 10. Phase 1 → 总蓝图的迁移点
- 将 `EventBus` 实现替换为 Kafka producer/consumer（保持事件结构不变）。
- 补 Redis 投影与 cache consumer。
- 增加 WS Bridge 将生命周期事件推给客户端。
- 引入 outbox（或可靠投影）以提升事件投影一致性。

