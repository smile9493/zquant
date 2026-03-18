# M4 数据拉取跑通（最小闭环）

## 目标
- 在当前代码基线下打通一次“远端数据拉取 → 仓储返回”的可执行链路，确保不是仅测试桩通过，而是实际可运行。

## 范围
- 聚焦 `repository-market` 的拉取路径可用性。
- 补齐当前阻塞“拉取跑通”的最小实现缺口（Provider 接入、仓储装配、必要配置）。
- 提供可重复执行的本地验证命令与结果。

## 非目标
- 不扩展新数据源类型。
- 不重构整体架构与既有任务边界。
- 不做与“拉取跑通”无关的 UI 或渲染改造。

## 验收标准
- 能通过一个明确入口触发真实数据拉取（非 NoOp Provider）。
- 拉取流程遇到远端失败时按规范降级，不发生 panic/进程硬退出。
- 至少有针对“成功拉取”路径的自动化测试通过。
- `cargo check --workspace` 通过。

## 假设与风险
- 假设当前仓库已有可复用的数据源适配能力（如 AkShare provider）。
- 风险：外部环境依赖（Python/网络/第三方 API）可能导致本地实拉不稳定。
- 风险应对：保留可控的回退路径与可注入测试替身，区分“代码可用”与“外部服务可达”。

## 实施方案
1. 先定位当前“拉取跑不通”的直接阻塞点（NoOp/占位实现/装配缺失）。
2. 在 `repository-market` 内做最小闭环接入，替换关键占位件。
3. 增加/修正测试，覆盖成功与降级分支。
4. 运行针对性测试与工作区编译检查。
5. 回写本 PRD 的实现与审查结论。

## 执行清单
- [x] 定位阻塞点并记录
- [x] 完成最小实现补齐
- [x] 补充或修复测试
- [x] 完成验证命令执行
- [x] 完成审查结论写回

## 实施记录
- 阻塞点确认：
  - `crates/repository-market/src/lib.rs` 默认使用 `NoOpProvider`，远端拉取永远返回空。
  - `HotStore` 仍是占位实现，但不影响“远端拉取返回数据”最小闭环验证。
- 代码变更：
  1. `repository-market` 接入真实远端 Provider 路由器 `RoutedRemoteProvider`，默认支持 `akshare`。
  2. 复用 `data-pipeline-application::AkshareProvider` 子进程拉取能力，按 frozen contract 组装 `DatasetRequest`。
  3. 新增 AkShare 返回记录到 `Bar` 的解析逻辑（支持数值和数字字符串）。
  4. 新增 `MarketRepository::with_provider(...)` 便于后续注入自定义 provider。
  5. 新增可执行示例 `crates/repository-market/examples/fetch_cn_daily.rs`，用于本地手动拉取验证。
- 测试补充：
  - `routed_remote_provider_parses_akshare_payload`
  - `routed_remote_provider_rejects_non_daily_timeframe`

## Review Findings
- 无未解决阻断项。

## Root Cause
- M3 T3 阶段保留了“接口骨架 + mock 验证”策略，生产默认 provider 仍为 NoOp，导致真实拉取链路未接通。

## Repair Plan
- 本轮已完成修复并闭环，无新增 repair 待办。

## 验证记录
- `cargo test -p repository-market`：28 passed, 0 failed
- `cargo check --workspace`：通过
- `cargo run -p repository-market --example fetch_cn_daily -- 000001 2024-01-01 2024-01-10`：拉取成功（7 条日线）

## 审查结论
REVIEW: PASS
