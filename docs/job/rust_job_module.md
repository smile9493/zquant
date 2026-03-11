# Job 调度模块设计蓝图（Rust 新项目）

## 1. 架构（Architecture）

本模块是新项目的核心基础模块之一，用于统一管理系统中的异步任务执行、状态流转、生命周期事件与外部可观测性。

设计原则如下：

- PostgreSQL 是唯一状态真相源（SSOT）
- Kafka 只负责生命周期事件流与 Runner 唤醒信号
- Redis 只负责读缓存、运行集投影与近似统计
- Job Claim 必须在 PostgreSQL 中原子完成
- WebSocket 生命周期消息只走 Kafka Bridge 单路径
- 事件投影为 Best-Effort，不反向污染主事务

### 1.1 系统结构

```text
Clients
   │
   ▼
Job API (HTTP / WS)
   │
   ▼
PostgreSQL (State Source)
   │
   ├── Redis (Cache / Projection)
   │
   └── Kafka
        ├── wq.jobs.lifecycle.v1
        └── wq.jobs.dispatch.v1
                 │
                 ▼
            Job Runner
                 │
                 ▼
            Job Handler
1.2 组件职责
Job API

职责：

提供 Job 管理接口

创建任务

查询任务状态

停止任务

失败任务重试

提供 WebSocket 订阅入口

技术建议：

axum

tower

serde

sqlx

Job API 只负责接入层编排，不负责调度决策。

PostgreSQL

职责：

Job 主表与幂等表持久化

Job 状态转换

原子 Claim

Lease 管理

Stop 标记管理

Finalize 提交

Claim 必须通过 PostgreSQL 原子完成：

SELECT id
FROM jobs
WHERE status = 'queued'
ORDER BY priority DESC, created_at ASC
FOR UPDATE SKIP LOCKED
LIMIT $1;

该机制用于保证：

多 Runner 并发下无重复领取

Claim 与状态转换具备事务一致性

Kafka 不可用时系统仍可独立运行

Job Runner

职责：

Claim Job

执行 Handler

写入 Heartbeat / Lease

Finalize 终态

执行周期性 Sweep

Runner 内部由四类子循环组成：

claim_loop
heartbeat_loop
dispatch_wait_loop
sweep_loop
Kafka

Kafka 只承担两类职责：

生命周期事件流

Runner 唤醒信号

Topic 约定：

wq.jobs.lifecycle.v1
wq.jobs.dispatch.v1

说明：

lifecycle topic 用于传播任务状态变化

dispatch topic 只用于唤醒 Runner，不能参与 Claim

Redis

Redis 仅用于读优化与投影，不承载权威状态。

典型用途：

Job 热路径缓存

running 集合投影

queue depth 近似统计

Key 示例：

wq:jobs:{job_id}
wq:jobs:running
wq:jobs:queue:depth:{job_type}
WebSocket Bridge

生命周期事件通过 Kafka Bridge 推送给客户端：

Kafka lifecycle
      │
      ▼
WS Bridge
      │
      ▼
/ws/events

API 本地不直接双发 lifecycle 事件。

1.3 领域模型
Job 状态机

对外可见的主状态为：

queued
running
done
error
stopped
reaped

补充说明：

- stopped：用户主动停止任务
- reaped：系统因租约过期或异常而回收任务

为区分"用户主动停止"与"系统回收停止"，JobStatus 增加 reaped 作为主状态。
stopped 仍保留，用于用户主动调用 stop API 的场景。
stop_reason 字段用于区分具体的停止原因。

状态流转：

queued  -> running -> done
                   -> error
                   -> stopped（用户主动停止）

queued  -> stopped（用户主动停止，任务尚未被 Claim）

running -> reaped  -> stopped（系统回收后转换）

Job 主实体

建议 Rust 领域对象定义如下：

pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
    Stopped,
    Reaped,
}

pub struct Job {
    pub job_id: String,
    pub job_type: String,
    pub status: JobStatus,
    pub payload: serde_json::Value,
    pub progress: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub artifacts: Option<serde_json::Value>,
    pub executor_id: Option<String>,
    pub stop_requested: bool,
    pub stop_reason: Option<String>,
    pub lease_until_ms: Option<i64>,
    pub lease_version: i64,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
数据表建议

建议至少保留两张核心表：

jobs

jobs_idempotency

jobs 负责任务状态与运行信息，jobs_idempotency 负责去重与重复提交控制。

jobs 表关键字段说明：

- status：任务主状态，包括 queued、running、done、error、stopped、reaped
- stop_requested：是否收到停止请求
- stop_reason：停止原因，用于区分"用户主动停止"和"系统回收"
- lease_version：租约版本号，用于防止"双执行"
- version：乐观锁版本号，用于并发更新控制

1.4 事件契约

生命周期事件建议采用两层结构：

外层：Records Envelope

内层：JobLifecycleEvent

原因：

统一事件基础字段

便于跨模块复用消费逻辑

保持 topic version 与业务 event version 解耦

Topic Version
wq.jobs.lifecycle.v1
wq.jobs.dispatch.v1
Event Type Version
jobs.created@v1
jobs.started@v1
jobs.done@v1
jobs.error@v1
jobs.stopped@v1
jobs.reaped@v1
Rust 契约建议
pub struct RecordsEnvelope<T> {
    pub event_id: String,
    pub r#type: String,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub data: T,
    pub producer_id: String,
    pub idempotency_key: Option<String>,
}

pub struct JobLifecycleEvent {
    pub event_id: String,
    pub event_type: String,
    pub schema_v: i32,
    pub event_ts: chrono::DateTime<chrono::Utc>,
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub executor_id: Option<String>,
    pub progress: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub duration_ms: Option<i64>,
}
1.5 工程分层建议

建议采用 Rust workspace：

crates/
  job-domain/
  job-store-pg/
  job-cache-redis/
  job-events/
  job-runner/
  job-api/
  job-observability/
apps/
  job-api/
  job-runner/
  job-cache-consumer/
  job-ws-bridge/

设计意图：

domain 只承载领域对象与状态机

store-pg 只承载 PostgreSQL 读写

cache-redis 只承载缓存与投影

events 只承载 Kafka 生产消费与契约编解码

runner 只承载运行时与 Handler 调度

api 只承载 HTTP / WS 接入

2. 目标（Goals）
2.1 统一任务调度模型

提供统一的任务生命周期与调度模型，避免不同业务模块各自维护异步任务实现。

2.2 构建可靠执行平面

系统必须支持：

多 Runner 并发执行

幂等提交

失败重试

租约过期回收

Stop 控制

Handler 超时保护

2.3 建立事件驱动可观测体系

所有任务状态变化都必须输出为生命周期事件，用于：

Redis 投影

WebSocket 推送

审计追踪

指标聚合

2.4 成为新项目的统一基础设施

该模块要作为新项目所有异步任务的统一基础设施，而不是只服务某一个业务场景。

3. 需求（Requirements）
3.1 API 需求

必须提供以下接口：

POST /jobs
GET /jobs/{id}
GET /jobs
POST /jobs/{id}/stop
POST /jobs/{id}/retry
POST /jobs/{id}/force_reap_stop
GET /ws/events
3.2 Claim 与执行需求

Runner 必须支持：

1. **PostgreSQL 原子 Claim**：
   - 多实例并发安全
   - Claim 后立即写入 running 状态与租约

2. **租约 Fencing 机制**：
   - 每次任务被 Claim 时，生成一个递增的 `lease_version`
   - Runner 在续期和 Finalize 时必须携带该版本号
   - PostgreSQL 通过乐观锁防止旧版本操作
   - 回收任务时需版本校验，确保原 Runner 已被隔离
   - 防止因网络分区导致的"双执行"问题

3. **Handler 执行完成后统一 Finalize**

3.3 Handler 机制需求

所有任务必须通过统一 Handler 接口接入：

#[async_trait]
pub trait JobHandler: Send + Sync {
    fn job_types(&self) -> &'static [&'static str];

    async fn handle(&self, ctx: JobContext) -> anyhow::Result<JobResult>;
}

要求：

1. **Handler 注册**：
   - Handler 启动时显式注册。
   - 禁止重复 `job_type`。
   - 支持别名映射，但必须显式声明。
   - 未注册 Handler 的 `job_type` 禁止被 Claim。

2. **动态注册能力**：
   - 预留 Handler 注册表的动态更新能力（如使用 `ArcSwap`），支持运行时加载新任务类型。
   - 适用于业务模块热插拔场景，避免重启系统。

3. **超时控制**：
   - 每个 Handler 执行必须设置超时时间（如通过 `tokio::time::timeout`）。
   - 超时后任务标记为 `error`，并记录超时原因。

4. **Panic 隔离机制**：
   - 使用 `catch_unwind` 捕获 Handler panic，防止崩溃扩散。
   - 单个 Handler panic 不影响 Runner 主循环，任务标记为 `error` 并记录堆栈信息。

3.4 Redis 热路径需求

查询链路必须支持：

Redis -> Miss -> PostgreSQL -> 回写 Redis

要求：

Redis miss 不影响查询正确性

terminal 状态缓存应支持 TTL

running 集投影必须可重建

3.5 Dispatch 唤醒需求

创建任务时必须执行两类副作用：

create job
   │
   ├─ emit lifecycle event
   └─ emit dispatch signal

Runner 等待模型：

1. **优先接收 dispatch 唤醒**：
   - Runner 监听 `wq.jobs.dispatch.v1` topic。
   - 收到信号后立即尝试 Claim。

2. **轮询退避策略**：
   - Kafka 不可用或超时时回退数据库轮询。
   - 采用指数退避策略：初始间隔 100ms，最大间隔 5s。
   - 退避参数可通过配置调整。

3. **无效唤醒过滤**：
   - dispatch 信号可携带 `job_id`，Runner 可据此判断是否需要立即处理。
   - 避免所有 Runner 都被同一信号唤醒却无实际任务可处理。

4. **降级机制**：
   - Kafka 连续失败 N 次后，自动切换到纯数据库轮询模式。
   - 记录告警，通知运维介入。

3.6 Sweep 需求

Runner 必须定期执行三类清理：

stop_queued sweep
lease_expired sweep
idempotency sweep

调度策略：
- 定时执行（如每 30 秒），可通过配置调整。
- 避免与 Claim 操作并发冲突（使用 `FOR UPDATE SKIP LOCKED`）。

**stop_queued sweep 说明**：

- stop API 应立即更新数据库 `stop_requested` 标记
- 若任务仍在 queued 状态，下一次被 Claim 时应直接转为 stopped，不应继续执行
- 尝试发送中断信号给正在执行的 Runner（可通过 Kafka dispatch 带 stop 标记）
- 后台 sweep 仅用于兜底清理异常残留（如 Runner 崩溃导致的停止信号未送达）
3.7 可观测性需求

至少输出以下指标：

jobs_created_total
jobs_completed_total
jobs_duration_seconds
jobs_lifecycle_emit_total
job_cache_hit_total
jobs_dispatch_wake_total
jobs_queue_depth
jobs_runner_claim_latency_seconds
jobs_runner_poll_idle_total
4. 约束（Constraints）
4.1 PostgreSQL 是唯一真相源

任何 Job 的权威状态都必须来自 PostgreSQL。

4.2 Kafka 不参与调度真相

Kafka 只能做：

生命周期事件流
Runner 唤醒信号

Kafka 不能做：

任务分配
任务状态真相
幂等判断
4.3 Redis 只能做缓存与投影

Redis 中的数据必须允许丢失后重建，不能承载调度真相。

4.4 主流程与投影解耦

Job 主流程成功标准为：

PostgreSQL 主事务成功提交

Kafka、Redis、WS 失败只能影响投影与可观测性，不能导致 Job 主流程回滚。

4.5 Handler 注册必须 Fail-Fast

系统启动时必须校验：

重复 job_type

未声明别名映射

注册表冲突

非法 Handler 配置

发现错误必须直接启动失败。

**动态更新支持**：

- 运行时可通过 API 或配置热更新 Handler 注册表。
- 更新前需校验新 Handler 的 `job_type` 是否冲突。
- 支持灰度发布，逐步切换新 Handler。
2. 潜在风险与改进建议

2.0 核心架构改进（已纳入设计）

基于社区反馈，以下潜在问题已纳入架构设计：

1. **reaped 状态与停止原因区分**：
   - 在 JobStatus 中增加 Reaped 状态
   - 新增 stop_reason 字段，区分"用户主动停止"和"系统回收停止"
   - 事件和日志中明确携带 reason 字段

2. **stop_queued sweep 行为明确**：
   - stop API 立即更新数据库 stop_requested 标记
   - 若任务仍在 queued 状态，下一次被 Claim 时直接转为 stopped
   - 后台 sweep 仅用于兜底清理异常残留

3. **租约 fencing 机制**：
   - 每次任务被 Claim 时生成递增的 lease_version
   - Runner 在续期和 Finalize 时必须携带版本号
   - PostgreSQL 通过乐观锁防止旧版本操作
   - 防止因网络分区导致的"双执行"问题

2.1 PostgreSQL 性能与扩展性

风险：
所有调度决策（Claim、状态更新、租约续期）都依赖 PostgreSQL，在高并发场景下可能成为瓶颈。
即使使用 `FOR UPDATE SKIP LOCKED` 避免锁竞争，仍需关注锁等待和索引优化。

建议：

1. **索引优化**：
   - 为 `jobs` 表设计合适的索引，确保 Claim 查询高效。
   - 建议索引：`(status, priority, created_at)`，覆盖 `WHERE status = 'queued' ORDER BY priority DESC, created_at ASC`。

2. **分片策略**：
   - 考虑将 `running` 状态的任务按 `executor_id` 分片。
   - 或引入轻量级任务分桶策略，减少单表压力。

3. **部分索引**：
   - 对 `stop_requested` 和 `lease_until_ms` 字段建立部分索引，加速扫表回收操作。
   - 示例：`CREATE INDEX idx_jobs_lease_expired ON jobs (lease_until_ms) WHERE lease_until_ms IS NOT NULL;`

4. **读写分离**：
   - 后期可引入读写分离或分库分表，但需权衡复杂度。

2.2 Kafka 唤醒信号可靠性

风险：
Runner 依赖 Kafka 的 dispatch 信号进行快速唤醒，若 Kafka 短暂不可用，Runner 会回退到数据库轮询。但轮询间隔设置不当可能导致任务延迟增加，或增加数据库压力。

建议：

1. **明确轮询退避策略**：
   - 采用指数退避（Exponential Backoff）加最大间隔限制。
   - 例如：初始间隔 100ms，每次失败加倍，最大不超过 5s。
   - 将退避策略暴露为配置项，便于运维调整。

2. **减少无效唤醒**：
   - 在 dispatch topic 中增加 `job_id` 或最小元数据，Runner 可据此判断是否需要立即 Claim。
   - 避免所有 Runner 都被同一信号唤醒却无实际任务可处理。

3. **主动降级机制**：
   - 评估是否需要在 Kafka 不可用时主动降级，保证核心调度不中断。
   - 例如：Kafka 连续失败 N 次后，自动切换到纯数据库轮询模式，并记录告警。

2.3 Kafka 可靠性与性能

风险：
Kafka 只负责生命周期事件流与 Runner 唤醒信号，若 Kafka 不可用，Runner 需回退轮询，可能增加延迟。
事件投影为 Best-Effort，若 Kafka 延迟或丢失消息，可能导致投影不一致。

建议：

1. **消息持久化与重试**：
   - 确保 Kafka topic 配置合理（如 `retention.ms`、`min.insync.replicas`）。
   - 生产者配置重试策略和确认机制（`acks=all`）。

2. **Runner 唤醒优化**：
   - 优先接收 dispatch 唤醒，但需设置超时机制，避免长时间等待。
   - Kafka 不可用时，Runner 应快速回退数据库轮询，避免阻塞。

3. **事件投影一致性**：
   - 考虑引入补偿机制（如定期全量重建投影），确保 Redis 投影与 PostgreSQL 状态一致。

2.4 Redis 缓存一致性

风险：
Redis 仅用于读优化与投影，不承载权威状态。若 Redis 数据丢失或与 PostgreSQL 不一致，可能导致查询结果错误。

建议：

1. **缓存失效策略**：
   - Terminal 状态（done、error、stopped）应设置 TTL，避免长期占用内存。
   - 运行中任务（running）投影应支持重建，确保 Redis 不可用时 API 仍可回退 PostgreSQL。

2. **缓存重建机制**：
   - 引入缓存预热或定期重建机制，确保 Redis 与 PostgreSQL 状态同步。
   - 例如，启动时通过 PostgreSQL 全量加载 running 集合到 Redis。

2.5 Handler 注册与生命周期管理

风险：
文档要求 Handler 启动时显式注册，并禁止重复 job_type。但未提及 Handler 的热更新或动态加载场景（如业务模块热插拔）。

建议：

1. **动态注册能力**：
   - 预留 Handler 注册表的动态更新能力（如使用 `ArcSwap`），支持运行时加载新任务类型。
   - 适用于业务模块热插拔场景，避免重启系统。

2. **超时控制**：
   - 每个 Handler 执行必须设置超时时间（如通过 `tokio::time::timeout`）。
   - 超时后任务标记为 `error`，并记录超时原因。

3. **Panic 隔离机制**：
   - 使用 `catch_unwind` 捕获 Handler panic，防止崩溃扩散。
   - 单个 Handler panic 不影响 Runner 主循环，任务标记为 `error` 并记录堆栈信息。

4. **兼容性检查**：
   - 系统启动时，校验所有已存在 job_type 是否都有对应 Handler。
   - 提供工具或 API 查询未注册的 job_type。

2.6 事件版本与兼容性

风险：
事件契约采用两层结构（RecordsEnvelope + JobLifecycleEvent），并区分 topic version 与 event type version，设计良好。但未明确向后兼容策略。

建议：

1. **事件 schema 演进规则**：
   - 仅添加可选字段，不删除或修改必填字段。
   - 新增字段必须为 `Option<T>` 类型，确保旧版 consumer 可忽略。
   - 不改变现有字段的语义或类型。

2. **版本迁移指南**：
   - 提供事件 schema 版本迁移文档，说明每个版本的变更内容。
   - 示例：`v1` 到 `v2` 的字段增加说明、废弃字段标记。

3. **Consumer 端版本适配层**：
   - 实现版本适配层，确保旧版事件能被正确消费或忽略。
   - 例如：根据 `schema_v` 字段选择对应的解析逻辑。
   - 对于无法解析的事件，记录日志并跳过，避免阻塞消费流。

2.7 缓存一致性与重建

风险：
Redis 作为投影层，数据可丢失重建。但重建过程可能产生较大数据库压力，尤其在 running 集投影重建时。

建议：

1. **增量重建策略**：
   - 设计增量重建机制，定期扫描最近变更的 job，而不是全量扫描。
   - 例如：基于 PostgreSQL 的 `updated_at` 字段，仅重建最近 N 分钟变更的任务。

2. **合理的 TTL 设置**：
   - 为 Redis 缓存设置合理的 TTL，避免冷数据堆积。
   - Terminal 状态（done、error、stopped）设置较短 TTL（如 1 小时）。
   - Running 状态任务实时更新，不依赖 TTL。

3. **原子操作优化**：
   - 使用 Redis Lua 脚本保证部分原子操作（如 running 集添加/移除），减少竞争。
   - 示例：Lua 脚本处理 `SADD` 和 `SREM` 操作，确保一致性。

4. **重建触发机制**：
   - 支持手动触发全量重建（如通过 API 或 CLI）。
   - 自动触发：Redis 连接恢复后，启动增量重建任务。

2.6 可观测性指标完备性

需求：
文档列出了多项指标，但未提及对 Runner 内部循环（如心跳、sweep）的健康度监控。

建议：

1. **增加 Runner 内部循环指标**：
   - `jobs_runner_heartbeat_failures_total`：心跳失败次数。
   - `jobs_sweep_duration_seconds`：Sweep 操作耗时分布。
   - `jobs_runner_claim_latency_seconds`：Claim 操作耗时分布（已列，需明确细化）。

2. **暴露调度异常指标**：
   - 租约续期成功率：`jobs_lease_renewal_success_total` / `jobs_lease_renewal_failure_total`。
   - 停请求处理延迟：`jobs_stop_request_latency_seconds`。
   - Runner 空闲时间：`jobs_runner_idle_duration_seconds`。

3. **完善告警机制**：
   - 设置关键指标告警（如任务积压、Claim 失败率、Handler 错误率、心跳失败）。
   - 定期生成健康报告，评估系统状态。

2.8 可观测性与监控

风险：
若指标输出不完整或监控不到位，可能难以定位问题（如任务卡住、性能瓶颈）。

建议：

1. **指标完善**：
   - 输出更多细粒度指标，如 Claim 耗时分布、Handler 执行耗时、Redis 缓存命中率。
   - 引入分布式追踪（如 OpenTelemetry），跟踪任务全链路。

2. **告警机制**：
   - 设置关键指标告警（如任务积压、Claim 失败率、Handler 错误率）。
   - 定期生成健康报告，评估系统状态。

2.7 测试与集成验证

需求：
验收要求包含异常场景验证，但未提及具体的测试策略（如单元测试、集成测试、混沌测试）。

建议：

1. **构建集成测试环境**：
   - 模拟 Kafka/Redis 故障，验证降级逻辑。
   - 使用 Docker Compose 搭建本地测试环境，支持一键启动。

2. **关键流程确定性测试**：
   - 对 Claim、Finalize、Stop 等关键流程编写确定性测试，确保状态机正确性。
   - 使用 `proptest` 或 `quickcheck` 进行属性测试，覆盖边界条件。

3. **压力测试**：
   - 评估数据库和 Kafka 在高负载下的表现。
   - 模拟高并发任务提交，验证系统吞吐量和稳定性。
   - 测试多 Runner 并发 Claim，确保无重复领取。

4. **混沌测试**：
   - 引入故障注入（如随机停止 Kafka、Redis），验证系统容错能力。

2.9 测试与验证

风险：
若缺乏充分测试，可能遗漏边界场景（如并发冲突、网络分区、数据不一致）。

建议：

1. **集成测试**：
   - 覆盖主链路（创建、Claim、执行、Finalize、事件发布）。
   - 模拟异常场景（Kafka 不可用、Redis 不可用、Handler panic）。

2. **压力测试**：
   - 模拟高并发任务提交，验证系统吞吐量和稳定性。
   - 测试多 Runner 并发 Claim，确保无重复领取。

3. **混沌测试**：
   - 引入故障注入（如随机停止 Kafka、Redis），验证系统容错能力。

5. 最终验收（Acceptance）

5.1 功能验收

必须验证以下主链路：

创建 Job

Runner Claim Job

Handler 执行

Finalize 终态

发布 lifecycle 事件

Redis 投影更新

WS 事件推送

5.2 状态一致性验收

必须保证：

Claim 不重复

状态转换合法

Stop 请求可收敛

Lease 过期可回收

幂等提交可生效

5.3 事件一致性验收

生命周期 topic 必须可观测到以下事件：

jobs.created@v1
jobs.started@v1
jobs.done@v1
jobs.error@v1
jobs.stopped@v1
jobs.reaped@v1
5.4 可用性验收

必须验证以下异常场景：

Kafka 不可用时 Runner 仍可通过轮询执行

Redis 不可用时 API 仍可回退 PostgreSQL

单个 Handler panic 或失败不影响 Runner 主循环

多 Runner 并发执行下不发生重复 Claim

5.5 工程验收

必须达到：

workspace 分层清晰，无循环依赖

API / Runner / Cache Consumer / WS Bridge 可独立部署

关键契约具备集成测试

PostgreSQL migration 可独立执行

本地开发环境可一键拉起 PostgreSQL / Redis / Kafka

6. 总结

该设计蓝图充分考虑了分布式任务调度的核心挑战，并基于 Rust 生态给出了清晰、务实的解决方案。其强调状态与事件分离、组件解耦、可观测性，符合现代微服务架构的最佳实践。

通过采纳上述建议（包括风险与改进建议章节中的内容），可进一步提升系统的健壮性、可扩展性和可维护性，确保其作为新项目统一异步任务基础设施的长期可靠性。

该设计的核心不是“把任务放进 Kafka 里消费”，而是建立一个以 PostgreSQL 为状态中心、以 Kafka 为事件与唤醒总线、以 Redis 为读优化层、以 Runner 为执行平面的 Rust Job 调度基础设施。

这是新项目中的基础模块，应作为统一异步任务框架长期演进，而不是按业务重复建设。