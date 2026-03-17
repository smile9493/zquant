# M3 存储分层闭环（PostgreSQL + Parquet）

## Goal

基于 `A:\zquant\docs\web\zquant_企业版标准规划方案.md` 的 M3 目标，完成“热窗口 + 归档分层”的最小闭环：  
以 PostgreSQL 作为控制面与热数据面，以 Parquet 作为历史归档面，打通查询补齐与写入一致性链路。

## Scope

### In scope

- 新增/完善 Parquet 存储模块（建议 `crates/infra-parquet`）。
- 定义并落地 partition manifest（以 PostgreSQL 为准）。
- 实现 `MarketRepository` 分层读取策略：热窗口优先 → Parquet 补齐 → 远端回写（最小路径）。
- 实现归档写入一致性流程：`tmp -> flush -> rename -> manifest update`。
- 补充关键日志与失败回滚路径。

### Out of scope

- 不做企业协同能力（权限、许可证、分发）。
- 不做完整指标/回放高级功能。
- 不做大规模性能优化（先闭环、后优化）。

## Non-Goals

- 不替换 M2 的 `egui_plot` 渲染方案。
- 不改造现有 `job-*` 服务为分布式架构。
- 不在本任务内完成全部 provider 扩展。

## Acceptance Criteria

- [ ] Parquet 写入路径可用，生成符合分区规则的数据文件。
- [ ] Manifest 能准确登记分区并作为读取来源。
- [ ] 查询路径可在“热窗口不足”时自动补齐 Parquet 数据。
- [ ] 归档写入失败可回滚/重试，不破坏可用性。
- [ ] `cargo check --workspace` 与 M3 相关测试通过。

## Assumptions / Risks

### Assumptions

- M2 画布交互已可稳定运行，不阻塞数据层演进。
- 当前 PostgreSQL 连接与基础 schema 可继续扩展。

### Risks

- 分区策略设计不当导致后续查询效率下降。
- Manifest 与文件系统状态不一致导致“可见性错误”。
- 写入原子性处理不完整导致脏分区文件。

## Implementation Plan

1. 设计分区规则与 manifest schema（provider/exchange/symbol/timeframe/time）。
2. 搭建 Parquet 写入器与读取器最小实现。
3. 在 repository 层实现热窗口优先 + 归档补齐读取策略。
4. 接入远端缺口回写与 manifest 更新顺序控制。
5. 补充异常路径（写入失败、分区缺失、manifest 不一致）与日志。
6. 执行构建与测试，完成 M3 review gate。

## Checklist

- [ ] 确认/创建 `infra-parquet` 模块与依赖。
- [ ] 定义 partition key 与目录规范。
- [ ] 设计并落地 manifest 表结构与访问接口。
- [ ] 完成 Parquet 写入原子流程（tmp/flush/rename）。
- [ ] 完成分层读取与补齐逻辑。
- [ ] 增加错误处理与重试策略。
- [ ] 运行 `cargo check --workspace`。
- [ ] 运行 M3 相关测试并通过。
- [ ] 写回审查结论（PASS/FAIL）到本 PRD。
