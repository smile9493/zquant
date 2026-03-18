# 审查发现整改执行计划（冗余/Public/算法）

## 背景来源
- 来源任务：`A:\zquant\.trellis\tasks\03-19-03-19-project-redundancy-structure-review\prd.md`
- 目标：将审查发现转为可执行整改任务，避免长期停留在“仅建议”状态。

## 目标
- 完成架构冗余收敛、`pub` 可见性收敛、关键路径算法优化三类整改。
- 优先消除 P0 风险（契约漂移、帧阻塞、边界泄漏），再推进 P1/P2 优化。

## 范围
- 涉及 crate：
  - `data-pipeline-application`
  - `job-events`
  - `application-core`
  - `domain-workspace`
  - `app-shell`
  - `repository-market`
  - `infra-parquet`
  - `ui-workbench`

## 非目标
- 不在一次提交内完成全部重构；按批次增量落地。
- 不改变产品功能范围，仅做结构、契约、性能与可维护性整改。
- 不在本任务直接引入新的业务能力。

## 分阶段整改清单

### P0（必须先完成）
1. **事件契约单一来源**
   - 收敛 `Dataset*`/`Dq*` 事件定义到 `job-events`，应用层仅保留适配。
2. **WorkspaceSnapshot 单一模型**
   - 收敛 `application-core` 与 `domain-workspace` 的双模型表示。
3. **app-shell 帧循环去阻塞**
   - 去掉 UI 帧内重复 `block_on(list_tasks)`，改缓存/事件驱动。
4. **内部测试端口可见性收敛**
   - `repository-market` 注入 trait 从 `pub` 收敛至 `pub(crate)`（或内部模块）。

### P1（第二阶段）
5. **Gap 算法补全**
   - 按 timeframe 识别 prefix / middle / suffix 全量缺口。
6. **合并去重算法优化**
   - 将重复排序合并流程改为线性 merge + 去重。
7. **Parquet 读取下推过滤**
   - `read_range` 避免全读后过滤，增加时间过滤下推。
8. **Provider 复用抽象**
   - 收敛 AkShare/PyTDX 重复模板逻辑为共享基类/策略。
9. **命令消费批处理**
   - UI 命令队列从“每帧 1 条”改为可控批量 drain。

### P2（持续优化）
10. **metrics 零分配化**
    - 将 `String` 标签接口改 `&'static str`/`Cow`。
11. **测试 fake 复用**
    - integration tests 的重复 fake 提取到 `tests/support`。
12. **大文件拆分**
    - `repository-market/src/lib.rs` 与 `ui-workbench/src/lib.rs` 模块化拆分。

## 验收标准
- 每个 P0 子项都有独立变更记录与通过的对应测试/检查。
- 对外 API 面减少（可见性收敛有统计对比）。
- 关键路径性能风险点有明确优化实现（而非仅注释）。
- 每轮整改完成后执行 review gate，并在本任务 PRD 持续回写。

## 风险与回滚
- 风险：契约收敛可能影响多个 crate 编译链路。
  - 应对：每次只改一类契约，配套适配层和测试。
- 风险：算法优化影响现有行为。
  - 应对：先补行为测试再改算法，实现前后结果等价验证。

## 执行方式
- 采用“同任务持续回写”模式：发现/修复/复审均写回本 `prd.md`。
- 每轮仅处理一个主题（契约、可见性、算法、拆分）以控制风险。

## Checklist
- [x] 新建整改任务并设为当前任务
- [x] 写入整改目标与分阶段清单
- [x] 执行 P0-1 事件契约收敛
- [x] 执行 P0-2 WorkspaceSnapshot 模型收敛
- [x] 执行 P0-3 app-shell 帧循环去阻塞
- [x] 执行 P0-4 repository-market 可见性收敛
- [x] P0 复审并输出 REVIEW 结论

## P0 复审记录（2026-03-19）

### 复审范围
- P0-1：事件契约单一来源 + 协议值稳定映射
- P0-2：`WorkspaceState`（UI DTO）与 `domain_workspace::WorkspaceSnapshot`（DB model）分离
- P0-3：`app-shell` 帧循环重复 `block_on(list_tasks)` 去除
- P0-4：`repository-market` 内部注入 trait/构造器可见性收敛

### 复审证据
- 代码核对：
  - `crates/data-pipeline-application/src/events.rs` 已删除本地重复事件结构，改为复用 `job_events::types::*`
  - `crates/data-pipeline-domain/src/types.rs` 与 `crates/data-pipeline-domain/src/quality.rs` 增加 `Display` 稳定 wire 值，并补充 4 个协议值测试
  - `crates/application-core/src/facade.rs` / `crates/application-core/src/lib.rs` / `crates/ui-workbench/src/lib.rs` 完成 `WorkspaceSnapshot -> WorkspaceState` 分离
  - `crates/app-shell/src/app.rs` 状态栏计数改为读取缓存，避免同帧第二次 `list_tasks`
  - `crates/repository-market/src/lib.rs` 5 个内部 trait 与 `with_provider` 收敛为 `pub(crate)`
- 命令检查：
  - `cargo check --workspace`：通过
  - `cargo test -p app-shell -p application-core -p data-pipeline-application -p data-pipeline-domain -p repository-market -p ui-workbench`：通过（105 passed, 0 failed）

### 备注
- `cargo test --workspace` 在当前环境存在与本次改动无关的外部依赖失败（`job-application` 的 DB 连接权限测试），不作为本次 P0 gate 阻塞项。
