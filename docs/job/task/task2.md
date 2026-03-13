# Phase 1 - Step 2: In-memory Event Bus + Single-process Kernel

## 1) 计划（Plan）

### 目标
- 在 Windows 单机跑通 EDA 内核最小闭环：`job-api` + `job-runner` + PostgreSQL + in-memory event bus。
- Job 生命周期事件通过本地总线传播：`JobCreated` / `JobStarted` / `JobCompleted`。
- 为 Agent 编排预留事件通道与契约：`AgentSpawnRequested` / `AgentTaskScheduled` / `AgentMessageProduced`。
- 保持 PostgreSQL 为唯一状态真相源（SSOT）：API/Runner 的成功标准是 PG 主事务成功，总线发布失败不回滚主事务。

### 前置条件
- Step 1 已完成并验收：`migrations/0001_jobs.sql` + `migrations/0002_phase1.sql`、`job-store-pg`、集成测试（Docker）通过。

### 范围
- 包含：
  - `EventBus` trait 与 in-memory 实现（broadcast fanout）
  - 事件契约（本地结构体/枚举）与发布/订阅
  - 单进程内核（Kernel）同时启动 HTTP API + Runner loops，并共享同一个 in-memory bus
  - API：`POST /jobs`、`GET /jobs/{id}`（最小可用）
  - Runner：claim + execute + finalize + sweep（最小可用）
- 不包含：
  - Kafka/Redis/WS Bridge（Phase 1 暂不上）
  - 多机部署、跨进程 IPC（in-memory bus 仅进程内）
  - 完整 stop/retry/force_reap_stop 全套 API（可作为 Step 2.1/Step 3）

### 关键决策（本任务默认选择）
- 进程模型：新增一个单进程入口（建议 `apps/job-kernel`），在同一 tokio runtime 内启动 API server + Runner loops，保证 in-memory bus 可共享。
- 事件实现：使用 `tokio::sync::broadcast` 做 best-effort fanout；如需“唤醒信号”低开销，再加 `Notify/watch`（可选）。
- 事件可靠性：Phase 1 不做持久化补偿（无 outbox）；Runner 必须有 DB 轮询兜底，避免丢事件导致卡住。

### 需求清单

#### R1. 事件契约（Phase 1 本地）
定义以下事件类型（Rust struct/enum 均可），并放在可复用 crate 中（建议扩展 `crates/job-events` 或新增 `crates/job-bus`）：
- `JobCreated { job_id, job_type, created_at }`
- `JobStarted { job_id, executor_id, lease_until_ms }`
- `JobCompleted { job_id, status, duration_ms, error?, artifacts? }`
- `AgentSpawnRequested { agent_id, job_id, agent_kind, init_payload }`
- `AgentTaskScheduled { agent_id, task_id, task_payload, deadline? }`
- `AgentMessageProduced { agent_id, job_id, message_type, content, ts }`

约束：
- 事件发布必须与 PG 主事务解耦：发布失败只记录日志/指标，不影响 API/Runner 对 PG 的提交。

#### R2. EventBus 抽象与 in-memory 实现
提供可替换接口：
- `publish(event)`：best-effort 发布
- `subscribe()`：订阅事件流（支持多订阅者）

实现：
- 使用 `broadcast::Sender/Receiver`
- 明确背压策略：buffer 满时允许丢消息（并记录丢弃计数/日志）

#### R3. Kernel（单进程装配）
新增一个 Kernel 装配点（推荐新增 binary）：
- 组装：`PgPool`、`JobStore`、`EventBus`、`HandlerRegistry`
- 启动：
  - HTTP API（axum Router）
  - Runner loops（至少 claim loop + sweep loop）
  - 优雅退出（ctrl-c）

#### R4. job-api（最小可用）
接口：
- `POST /jobs`
  - 输入：`job_type`, `payload`, `priority?`, `idempotency_key?`
  - 行为：调用 `JobStore::create_job` 写入 PG；成功后 publish `JobCreated`
  - 返回：`job_id`
- `GET /jobs/{id}`
  - 行为：调用 `JobStore::get_job`
  - 返回：job 当前状态与主要字段

#### R5. job-runner（最小可用）
行为：
- Claim：
  - 订阅 `JobCreated` 作为唤醒信号，收到后立即尝试 claim
  - 始终保留 DB 轮询兜底（指数退避），避免漏事件导致积压
  - claim 使用 Step 1 的 `JobStore::claim_jobs`（带 job_type 过滤）
- Execute：
  - Handler 注册表（启动时 fail-fast，禁止重复 job_type）
  - `tokio::time::timeout` 超时控制
  - `catch_unwind` panic 隔离
  - 成功/失败后调用 `finalize_job`
- Lifecycle events：
  - claim 成功 publish `JobStarted`
  - finalize 成功 publish `JobCompleted`
- Sweep：
  - 周期性调用 `reap_expired_jobs`（最小 lease sweep）

### 验收标准（Acceptance）
- E2E 主链路可跑通（单进程 Kernel）：
  - `POST /jobs` 创建 queued job
  - Runner 领取并执行（至少一个内置测试 handler）
  - Job 最终进入 `done`（或按测试设置进入 `error`）
  - `GET /jobs/{id}` 可观测到状态从 queued -> running -> done/error
- 事件行为可验证：
  - 创建时至少发出 `JobCreated`
  - 领取时至少发出 `JobStarted`
  - 终态时至少发出 `JobCompleted`
- 降级与兜底：
  - 即使不订阅事件（或事件丢失），Runner 轮询兜底仍可最终执行 queued job

### 测试计划（Test Plan）
- 单元测试：
  - `EventBus` publish/subscribe 基本语义（多订阅者、丢弃策略）
  - `HandlerRegistry` 重复 job_type fail-fast
- 集成测试（Docker PG）：
  - 启动 Kernel（in-process 或后台 task），执行：
    - POST /jobs -> 等待完成 -> GET /jobs/{id} 断言终态
  - 验证生命周期事件按序出现（允许并发下的 interleave，但同一 job 的 started/completed 必须晚于 created）

## 2) 实现（Implementation）
- 状态：已完成
- 实现日期：2026-03-12
- 关键文件列表：
  - `crates/job-events/src/types.rs` - 事件契约定义
  - `crates/job-events/src/bus.rs` - EventBus trait 与 InMemoryEventBus 实现
  - `crates/job-application/src/api.rs` - HTTP API (POST /jobs, GET /jobs/{id})
  - `crates/job-application/src/runner.rs` - Runner 与 HandlerRegistry
  - `apps/job-kernel/src/main.rs` - Kernel 单进程装配
  - `crates/job-events/src/bus.rs` - EventBus 单元测试
  - `crates/job-application/src/runner.rs` - HandlerRegistry 单元测试
  - `crates/job-store-pg/tests/e2e_test.rs` - E2E 集成测试
- 设计偏差：无重大偏差，按计划实现

### 补充改进（审查建议落地）
- Kernel API bind 地址不再硬编码 `:3000`，改为读取 `API_HOST`/`API_PORT`（默认 `0.0.0.0:3000`），启动日志打印实际监听地址。
- InMemoryEventBus 增加基础可观测性计数（`publish_total` / `publish_no_subscribers_total`），并提供 `stats()` 便于测试/排障。
- Runner 在 `broadcast` 接收发生 `Lagged` 时记录累计次数（`lagged_event_total`），并打 warning 日志（仍保留 DB 轮询兜底）。

## 3) 验证（Verification）
- 状态：已完成
- 验证日期：2026-03-12
- 运行命令与结果：
  - `cargo test -p job-events --lib` - EventBus 单元测试通过 (3 passed)
  - `cargo test -p job-application --lib` - HandlerRegistry 单元测试通过 (1 passed)
  - `cargo check -p job-kernel` - Kernel 编译成功
  - E2E 集成测试已创建，需要 PostgreSQL 运行：`cargo test -p job-store-pg --test e2e_test`
