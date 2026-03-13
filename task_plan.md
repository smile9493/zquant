# Task Plan — Phase 1 Windows 单机 EDA 内核

## Source
- `A:\zquant\docs\job\phase1_windows_eda_kernel_plan.md`

## Planning Notes
- `A:\zquant\docs\job\task\` 目录为已完成任务的历史归档：默认不当作当前计划来源，也不做目录级扫描；仅在当前任务需要核对历史实现时，按需读取具体文件。

## Goal
在 Windows 单机上跑通最小 EDA 内核：`job-api` + `job-runner` + `PostgreSQL` + **in-memory event bus**，并让 Agent 编排形态已经是 EDA（只是总线先在本机）。

## Constraints / Principles
- PostgreSQL 是唯一状态真相源（SSOT）：DB 主事务成功即主链路成功。
- EventBus 发布失败不得回滚 DB 写入（best-effort + 可观测）。
- In-memory bus 仅同进程共享，因此 Phase 1 采用单进程 `job-kernel` 形态装配 API + Runner + Bus + Agent 编排。
- Kafka/Redis/WS Bridge 在 Phase 1 明确不做。

## Milestones
| ID | Milestone | Status | Deliverable |
|----|-----------|--------|-------------|
| M1 | Schema + Store (P0) | complete | jobs/jobs_idempotency + claim/finalize + fencing |
| M2 | 本地事件总线 (P0) | complete | EventBus + Kernel 单进程闭环 |
| M3 | Runner 执行与 Agent 编排 (P0) | complete | Agent spawn/schedule/message 本机闭环 |
| M4 | 可观测性与回归 (P1) | complete | tracing/metrics + 集成测试覆盖主链路 |

## Current Status (ground truth)
- M2 已完成（如需回看历史实现记录，可按需读取：`A:\zquant\docs\job\task\task2.md`）。
- M3 已完成：单进程内实现 `AgentSpawnRequested -> AgentTaskScheduled -> AgentMessageProduced` 闭环，并有单元测试覆盖。

## Next: M3 — Agent 编排闭环（本机）

### Objective
在单进程 `job-kernel` 内实现一个最小的 Agent 监督器（supervisor），通过 EventBus 驱动：
- `AgentSpawnRequested` → spawn 一个 Agent（tokio task）
- `AgentTaskScheduled` → 投递任务给指定 Agent
- Agent 处理后发布 `AgentMessageProduced`

### Requirements
- R3.1 Supervisor 订阅 EventBus，维护 `agent_id -> agent_handle/channel` 映射。
- R3.2 Agent 的输入通道（mpsc）与生命周期管理（缺失 agent 时的降级日志/计数）。
- R3.3 发布消息遵循现有事件契约，不引入跨进程依赖。
- R3.4 可观测：关键路径日志 + 丢弃/异常计数（最小即可）。

### Acceptance
- 通过测试或最小演示证明闭环：
  - 发布 `AgentSpawnRequested` 后，系统能创建 agent。
  - 发布 `AgentTaskScheduled` 后，agent 处理并发布 `AgentMessageProduced`。
  - 订阅端能收到对应 `AgentMessageProduced`（可断言内容/次数）。

### Implementation Plan
1. 新增 `AgentSupervisor`（建议放 `crates/job-application` 或新 crate；优先贴近 Runner/Kernel 现有装配点）。
2. 在 `apps/job-kernel` 启动 supervisor loop（与 Runner loops 并行）。
3. 添加单元测试/集成测试（优先单测：bus 驱动 supervisor，断言输出事件）。
4. 跑回归：`cargo test -p job-application --lib`，必要时补 `job-events` 单测。

## After M3: M4（简要）
- tracing/metrics：claim 延迟、执行耗时、completed 计数、bus lag/no-subscriber 计数。
- 集成测试：Docker PG 覆盖 Create→Execute→Finalize；可选校验生命周期事件顺序约束（同一 job 内）。

## Completed: M3 — Agent 编排闭环（本机）
- 代码落点：`crates/job-application/src/agent_supervisor.rs`（supervisor + 最小 agent runtime + 单测）
- 内核装配：`apps/job-kernel/src/main.rs` 启动 supervisor loop
- 验证：`cargo test -p job-application --lib`、`cargo check -p job-kernel`

## Completed: M4 — 可观测性与回归（P1）
- 可观测性：
  - API：`POST /jobs` 创建成功记录结构化日志（job_id/job_type）
  - Runner：claim/execute/finalize 增加结构化日志；新增 `RunnerStats`（claimed/completed/errored/lagged 计数）
- 回归：
  - 单测：`cargo test -p job-application --lib`
  - 编译：`cargo check -p job-kernel`
  - E2E（Docker PG）：`cargo test -p job-store-pg --test e2e_test`

### E2E 环境注意事项（Windows）
- 如果本机已有 `postgres.exe` 占用/复用 `5432`，可能导致连接落到本机 Postgres 并触发 `sqlx` 的 “non-UTF-8 error message” 报错。
- 最稳妥做法：用 Docker Postgres 映射到其他端口（例如 `15432:5432`），并设置 `DATABASE_URL=postgres://postgres:postgres@localhost:15432/postgres` 后再跑 E2E。
## Post-Phase 1 Hardening

### H1: Windows E2E stabilization (job-store-pg)
- Status: complete
- Deliverable: Harden `scripts/test_job_store_pg_docker.ps1` to improve failure diagnostics.
- Acceptance: When `cargo test -p job-store-pg` fails, the script prints `docker logs --tail 50` for the test container and rethrows the original error.
- Validation: `pwsh -File A:\zquant\scripts\test_job_store_pg_docker.ps1` (PASS)

### H2: Repo hygiene hardening
- Status: complete
- Objective: Reduce git noise and prevent local tooling/state from being tracked.
- Scope:
  - Ensure `.claude/settings.local.json` is ignored and not tracked.
  - Decide whether `.trellis/spec/` should be tracked (recommended) while keeping `.trellis/tasks/` and `.trellis/workspace/` ignored.
  - Ensure no build artifacts are tracked (target/ already cleaned).
- Acceptance:
  - `git status` does not show local settings as modified by default.
  - `.trellis` policy is explicit and enforced by `.gitignore`.
- Deliverable:
  - `.claude/settings.local.json` untracked and ignored.
  - `.claude/agents/*.md` tracked (7 agent definitions).
  - `.trellis/spec/**` tracked (9 spec files).
  - `.trellis/` local state (tasks/workspace/.developer) ignored.

