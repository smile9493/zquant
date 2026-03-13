# Findings

## Context
- Phase 1 目标：Windows 单机 EDA 内核（SSOT=Postgres，事件总线本机 in-memory）。
- 当前 M1/M2 已完成；下一项是 M3（Agent 编排闭环）。
- `A:\zquant\docs\job\task\` 为历史归档：默认不扫描；仅在需要对照历史实现时按需读取具体文件。

## Key Decisions
- Phase 1 采用单进程 `job-kernel` 装配（API + Runner + Bus + Supervisor），避免跨进程 IPC。
- EventBus 为 best-effort；主链路以 PG 提交为准。

## Notes
- 事件契约已包含：`AgentSpawnRequested` / `AgentTaskScheduled` / `AgentMessageProduced`。
- M3 已实现：新增 `AgentSupervisor`（订阅本地 bus，spawn tokio agent，路由任务并发布 message）。

## Implementation Notes (M3)
- `AgentSupervisor` 在 `new()` 时就创建 `broadcast::Receiver` 并持有，避免“启动 race”导致漏收早期事件。
- `AgentTaskScheduled` 可能先于 spawn 到达：按 agent_id 做小型 pending 队列（有上限），spawn 后尽量 flush。
- agent 输出事件：`AgentMessageProduced.message_type = "task_completed"`，`content` 至少包含 `task_id`，并回显 `task_payload/deadline` 便于观测。
- 当 agent 的 mpsc 通道关闭（agent task 退出）时：从映射中移除该 agent，并把当前 task 回退到 pending 队列，避免“死 agent_id”导致后续无法恢复。

## Implementation Notes (M4)
- API：`POST /jobs` 写入成功后，记录结构化日志（`job_id`/`job_type`），便于在单机模式下串联调用链。
- Runner：在 claim/execute/finalize 增加结构化日志字段（`job_id`/`job_type`/`executor_id`/`duration_ms`）。
- Runner：新增 `RunnerStats`（AtomicU64 计数），用于最小“可观测数值面”：
  - `claimed_total`：claim 到的 job 数累计
  - `completed_total`：done 终态累计
  - `errored_total`：非 done 终态累计
  - `lagged_event_total`：broadcast lagged 次数累计
- Windows E2E：如果本机 `postgres.exe` 监听 5432，`sqlx::test` 可能连到本机 Postgres 并因本地化错误消息触发 non-UTF8 报错；建议 Docker PG 映射到非 5432 端口（如 15432）。
## Windows E2E: Docker test script diagnostics
- `scripts/test_job_store_pg_docker.ps1` now prints the test Postgres container logs (last 50 lines) if `cargo test -p job-store-pg` fails, then rethrows.
- Default host port is non-5432 (currently 55432) to avoid accidental connections to a local Postgres service.

## Repo hygiene: .claude/ and .trellis/ gitignore policy
- **Decision**: Share agent definitions and specs; ignore local state.
- **`.claude/` policy**:
  - Ignore: `/.claude/*` (default ignore all)
  - Track: `!/.claude/agents/*.md` (7 agent definitions: check, code-worker, debug, dispatch, implement, plan, research)
  - Ignore: `.claude/settings.local.json` (per-user config)
- **`.trellis/` policy**:
  - Ignore: `.trellis/*` (default ignore all)
  - Track: `!.trellis/spec/**` (9 spec files: backend guidelines + thinking guides)
  - Ignore: `.trellis/tasks/`, `.trellis/workspace/`, `.trellis/.developer` (local state)
- **Validation**: `git check-ignore` confirms correct ignore/track boundaries.

