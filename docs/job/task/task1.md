# Phase 1 — Step 1：Postgres Schema + `job-store-pg`（执行面真相接口）

## Summary
把“单机 EDA 内核”的 **状态真相层** 先做扎实：完善 PG 表结构/索引/约束，并把 `job-store-pg` 收敛成唯一可信的 Job 状态读写与 Claim/Fencing 接口。此步完成后，Step 2 才能可靠地把 in-memory bus + runner loop 串起来。

---

## 目标（Goal）
- PG 作为 SSOT：Job 的创建、Claim、租约、终态写入在 **不依赖 Kafka/Redis/WS** 的情况下可正确工作。
- 提供 **fencing 能力**（`lease_version`）：防止旧 Runner/旧租约对同一 Job 的续租与 Finalize 发生越权更新（为后续并发与异常恢复打底）。
- `job-store-pg` 对外提供“可直接被 runner loop 调用”的稳定 API，避免 runner 自己拼 SQL。

---

## 需求（Requirements）

### R1. 数据库迁移（新增 `0002_phase1.sql`，不改 `0001_jobs.sql`）
对 `jobs` 表补齐 Phase 1 所需字段/约束/索引（兼容现有结构）：

1) 字段
- `lease_version BIGINT NOT NULL DEFAULT 0`（fencing 版本号，每次 Claim +1）
- `stop_reason TEXT NULL`（Phase 1 可仅用于 `reaped` / 用户 stop 归因）
- `updated_at` 自动更新：Phase 1 先保持应用层写入即可（不强制触发器）
- 视需要把 `version` 默认调整为 `0`（当前是 `1`），并明确语义：乐观锁版本（可选；若启用则每次状态更新 +1）

2) 约束（用 CHECK，避免引入 PG enum 迁移复杂度）
- `status` 仅允许：`queued|running|done|error|stopped|reaped`（Phase 1 即使不启用 reaped 行为，也建议把值域放开）
- `status='running'` 时：`executor_id IS NOT NULL AND lease_until_ms IS NOT NULL`
- 可选：`lease_until_ms IS NULL` 当且仅当 `status!='running'`（防止终态残留）

3) 索引（按 Phase 1 热路径）
- Claim：部分索引（推荐）
  - `CREATE INDEX ... ON jobs (priority DESC, created_at ASC) WHERE status='queued' AND stop_requested=false;`
- lease sweep：部分索引
  - `CREATE INDEX ... ON jobs (lease_until_ms) WHERE status='running' AND lease_until_ms IS NOT NULL;`
- stop sweep（Phase 1 可先不做 sweep，但索引先留）
  - `CREATE INDEX ... ON jobs (created_at) WHERE status='queued' AND stop_requested=true;`
- job_id 查找：已有 UNIQUE（保留）

4) `jobs_idempotency`（Phase 1 最小可用）
- 增加：`expires_at TIMESTAMPTZ NOT NULL`
- 索引：`expires_at`（便于后续 sweep）
- 保持 FK：`job_id REFERENCES jobs(job_id) ON DELETE CASCADE`

> 说明：本阶段仍用 `job_id` 文本（现状是 `VARCHAR(255)`），不强推 UUID 类型；等 Phase 2 再决定是否升级为 UUID。

---

### R2. `job-domain`（与 Phase 1 schema 对齐）
调整 `Job` / `JobStatus`，使其可承载 fencing 与停止原因：
- `JobStatus` 增加 `Reaped`（即使 Phase 1 行为暂时把 reaped 直接落到 stopped，也建议模型先对齐蓝图）
- `Job` 增加：
  - `stop_reason: Option<String>`
  - `lease_version: i64`
- 明确 `version` 的含义（保留 `i32` 可，但 store 更新要一致）

---

### R3. `job-store-pg` 对外 API（Phase 1 必需集）
`JobStore` 对外提供这些方法（方法名可微调，但语义固定）：

1) `create_job(...) -> Job`
- 输入：`job_type`, `payload`, `priority`, `idempotency_key: Option<String>`
- 行为：
  - 无幂等键：插入一条 `queued` job
  - 有幂等键：在 `jobs_idempotency` 上做唯一约束；重复提交时返回已存在的 `job_id`
- 事务边界：插入 job 与幂等表写入在同一事务内

2) `get_job(job_id) -> Option<Job>`

3) `claim_jobs(executor_id, lease_duration_ms, limit, allowed_job_types) -> Vec<Job>`
- 必须原子（单事务）
- Claim 选择条件（Phase 1）：
  - `status='queued' AND stop_requested=false AND job_type = ANY($allowed_job_types)`
  - 排序：`priority DESC, created_at ASC`
  - `FOR UPDATE SKIP LOCKED`
- Claim 更新（同事务）：
  - `status='running'`
  - `executor_id=...`
  - `lease_until_ms=now+lease_duration_ms`
  - `lease_version=lease_version+1`
  - `version=version+1`（若启用乐观锁）
  - `updated_at=now`
- 返回值必须包含最新 `lease_version`（runner 后续 heartbeat/finalize 要用）

4) `heartbeat_job(job_id, executor_id, lease_version, lease_duration_ms) -> Result<bool>`
- 续租必须带 fencing 条件：
  - `WHERE job_id=$1 AND status='running' AND executor_id=$2 AND lease_version=$3`
- 返回 `bool`：true=续租成功；false=被抢占/已终态/版本不匹配（runner 需停止对该 job 的所有操作）

5) `finalize_job(job_id, executor_id, lease_version, terminal_status, artifacts, error) -> Result<bool>`
- 终态仅允许：`done|error|stopped|reaped`
- 必须带 fencing 条件（同 heartbeat）
- 终态写入后建议清空：
  - `lease_until_ms=NULL`
  - `executor_id` 可保留用于审计（不强制清空）
- 返回 `bool`：true=落库成功；false=版本不匹配/状态不允许（runner 记录告警并停止）

6) `request_stop(job_id, reason: Option<String>) -> Result<()>`
- `stop_requested=true`, `stop_reason=reason`, `updated_at=now`
- Phase 1 不要求立即中断正在执行的 handler（Step 3/4 再补），但必须保证 queued job 不再被 claim

7) `reap_expired_jobs(now_ms, batch) -> Vec<Job>`（Phase 1 可选但推荐）
- 找到 `running AND lease_until_ms < now` 的任务，原子更新为 `reaped` 或 `stopped`（两种二选一，见 Assumptions）
- 目的：为 runner sweep loop 提供可靠 DB primitive

---

## 验收（Acceptance）
满足以下全部条件视为 Step 1 完成：

1) 迁移
- 本地 PG 上能顺序执行 `0001` + `0002`，且 `jobs/jobs_idempotency` schema 符合 R1（字段、约束、索引齐备）。

2) 正确性（单机并发）
- 两个并发 `claim_jobs` 调用不会返回同一条 job（可用并发测试验证）。
- `stop_requested=true` 的 queued job 不会被 claim。
- `heartbeat_job/finalize_job` 在 lease_version 不匹配时返回 false 且不改数据。

3) fencing 语义
- 同一 job 被再次 claim 后（lease_version +1），旧 lease_version 的 finalize/heartbeat 必须失败。

4) 可用性
- `cargo test`（至少 job-store-pg 的集成测试）可运行并覆盖主链路：create → claim → heartbeat → finalize。

---

## Test Plan（必须写成可执行用例）
- 集成测试（推荐 `tests/pg_store_phase1.rs`）：
  1) `create_job_no_idempotency_creates_queued`
  2) `create_job_with_idempotency_is_deduped`
  3) `claim_skips_stop_requested`
  4) `claim_increments_lease_version`
  5) `heartbeat_requires_matching_lease_version`
  6) `finalize_requires_matching_lease_version`
  7) `concurrent_claim_no_duplicates`（tokio 并发 + 两个 store 实例同库）
- 运行方式（固定）：
  - 通过 `DATABASE_URL` 指向本机 Postgres（Windows 单机目标）；测试执行前由测试代码创建/清理 schema（或使用独立 test database）。

---

## Assumptions / Defaults（本步骤默认选择）
- `reaped`：Phase 1 **允许**写入为终态之一；若你更倾向“reaped 只做瞬时中间态最终落到 stopped”，则 `reap_expired_jobs` 直接落 `stopped` 并写 `stop_reason='reaped'`（实现更简单）。
- 事件总线：本步骤不实现 bus（仅提供能被 Step 2 调用的 store primitives）。
- 不引入 `sqlx` offline 模式与 `sqlx prepare`（先保证 Windows 单机能跑通）。
