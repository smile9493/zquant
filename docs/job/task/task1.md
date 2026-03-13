# Phase 1 - Step 1: Postgres Schema + `job-store-pg`

## 1) 计划（Plan）

### 目标
- 以 PostgreSQL 作为 Job 状态唯一真相源（SSOT）。
- 完成原子 Claim + fencing（`lease_version`）能力。
- 提供可直接供 runner 调用的稳定 `job-store-pg` API。

### 范围
- 包含：数据库迁移、`job-domain` 对齐、`job-store-pg` 核心接口、集成测试。
- 不包含：in-memory event bus、runner loop、Redis、Kafka、WS bridge。

### 需求清单
- 数据库迁移：新增 `lease_version`、`stop_reason`，约束 `status` 值域与 `running` 条件，补齐 claim/sweep/idempotency 索引，`jobs_idempotency` 增加 `expires_at`。
- 领域模型：`JobStatus::Reaped`，`Job.stop_reason`，`Job.lease_version`。
- 存储接口：
  - `create_job`, `get_job`, `claim_jobs`
  - `heartbeat_job`, `finalize_job`, `request_stop`
  - `reap_expired_jobs`
- 验收标准：
  - 迁移可执行并落地字段/约束/索引
  - 并发 claim 无重复领取
  - stop 请求可阻止 queued job 被 claim
  - heartbeat/finalize 版本不匹配返回 false
  - 关键链路测试可运行

## 2) 实现（Implementation）

### 执行日期
- 实现完成：2026-03-11

### 已落地变更
- 数据库迁移：`migrations/0002_phase1.sql`
  - 添加 `lease_version`、`stop_reason`
  - 添加 `jobs_status_check`、`jobs_running_requires_executor`
  - 添加优化索引（claim / lease sweep / stop sweep / idempotency expires）
  - 设置 `jobs.version` 默认值为 `0`
  - 迁移支持可重入（约束存在检测 + `CREATE INDEX IF NOT EXISTS`）
- 领域模型：`crates/job-domain/src/lib.rs`
  - 增加 `JobStatus::Reaped`
  - 增加 `stop_reason`、`lease_version`
- 存储实现：`crates/job-store-pg/src/lib.rs`
  - 完成 `create_job/get_job/claim_jobs/heartbeat_job/finalize_job/request_stop/reap_expired_jobs`
  - `heartbeat/finalize` 使用 `executor_id + lease_version` 做 fencing
  - 修复并发幂等竞态：唯一冲突后回滚当前事务并回查 canonical job

## 3) 验证（Verification）

### 验证日期
- Docker 验证完成：2026-03-12

### 测试用例
- 文件：`crates/job-store-pg/tests/pg_store_phase1.rs`
- 已覆盖 8 个用例：
  1. `create_job_no_idempotency_creates_queued`
  2. `create_job_with_idempotency_is_deduped`
  3. `claim_skips_stop_requested`
  4. `claim_increments_lease_version`
  5. `heartbeat_requires_matching_lease_version`
  6. `finalize_requires_matching_lease_version`
  7. `concurrent_claim_no_duplicates`
  8. `concurrent_create_with_same_idempotency_key_returns_same_job`

### 验证方式与结果
- 推荐脚本：`scripts/test_job_store_pg_docker.ps1`
- 自动执行：`migrations/0001_jobs.sql` + `migrations/0002_phase1.sql` + `cargo test -p job-store-pg`
- 最新结果：8/8 通过

### 结论
- Phase 1 - Step 1 已完成，满足当前任务目标，可进入 Step 2（in-memory event bus + runner loop 串联）。
