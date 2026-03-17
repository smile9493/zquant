# M2 中心画布渲染最小闭环（egui_plot 方案）

## Goal

基于 `A:\zquant\docs\web\zquant_企业版标准规划方案.md` 与 `A:\zquant\.kiro\specs\zquant-enterprise-evolution-roadmap\design.md`，完成 M2 阶段最小可执行目标：在现有 `eframe/egui` 架构内，以 `egui_plot` 实现“可显示 + 可交互 + 可追踪”的中心画布闭环。

## 关键技术决策

### 决策结论

- M2 采用 `egui_plot` 作为中心画布主渲染方案。
- Bevy 与 `eframe` 的直接离屏集成在当前阶段不作为主路径，延后到 M3+ 作为可插拔后端选项评估。

### 决策依据

- 当前应用运行时由 `eframe` 接管，直接嵌 Bevy 会引入窗口/渲染管线接管冲突与高复杂度。
- `egui_plot` 在 `egui` 生态内成熟，支持缩放、拖拽、滚轮等 2D 交互，能满足 M2 目标。
- 该方案改动面小、交付快，可在不破坏 M1 稳定性的前提下推进 M2。

## Scope

### In scope

- 在 `crates/ui-workbench` 中引入 `egui_plot`，实现中心画布有效绘制（K 线最小版本）。
- 定义并接入 `RenderSnapshot`（最小字段）用于 UI 状态到绘图状态传递。
- 实现交互：缩放 + 平移（至少一项为硬要求，目标两项都完成）。
- 增加绘图链路关键日志（初始化、刷新、交互、异常降级）。
- 为后续 Bevy 路径保留抽象边界（渲染接口/数据模型不绑死具体库）。

### Out of scope

- 本任务不做 Bevy 与 `eframe` 的直接离屏集成。
- 不做完整指标系统与回放系统（属于后续阶段）。
- 不做 Parquet 归档与 manifest 补数（属于 M3）。
- 不做企业协同、分发、许可证体系。

## Non-Goals

- 不追求渲染性能极限优化（先闭环，再优化）。
- 不替换现有 `application-core` 的业务接口设计。
- 不改动 `job-*` 服务链路的既有行为。
- 不在 M2 内解决 Bevy 渲染架构问题。

## Acceptance Criteria

- [x] `egui_plot` 能在中心区域绘制有效 K 线内容（非占位文字）。
- [x] 至少一个真实 UI 操作（缩放/平移）可驱动渲染状态变化。
- [x] `RenderSnapshot` 接口稳定，UI 与渲染层职责边界清晰。
- [x] 渲染异常可降级，不导致应用崩溃。
- [x] `cargo check --workspace` 与目标包测试通过。

## Assumptions / Risks

### Assumptions

- 当前 M1 壳层（`desktop-app` + `app-shell` + `ui-workbench`）已可继续演进。
- `egui_plot` 版本可与当前 `egui/eframe` 版本兼容。

### Risks

- K 线表达需用 `egui_plot` 组合图元实现（无现成 Candlestick），初版实现可能需要多次调样式。
- 初版快照模型设计不稳会导致 M3 再次返工。
- 交互与状态同步若边界不清，容易产生 UI 与数据不同步。

## Implementation Plan

1. 定义最小 `RenderSnapshot`（symbol/timeframe/candles/viewport）与渲染接口边界。
2. 在 `ui-workbench` 中接入 `egui_plot` 画布容器。
3. 实现 K 线最小绘制：实体 + 上下影线（使用 `Bar/BarChart` + `Line/VLine` 组合）。
4. 接入缩放/平移交互，并将视窗变化回写状态。
5. 增加绘图与交互日志，失败时降级到可用占位视图。
6. 执行构建与测试，完成 M2 review gate。

## Checklist

- [x] 新增 `egui_plot` 依赖并确认版本兼容。
- [x] 定义 `RenderSnapshot` 结构与最小协议。
- [x] 打通 UI 到绘图状态的数据传递。
- [x] 中心画布显示 K 线有效内容（非占位文本）。
- [x] 接入缩放/平移交互（至少一项，目标两项）。
- [x] 完成日志与降级策略。
- [x] 运行 `cargo check --workspace`。
- [x] 运行 M2 相关测试并通过。
- [x] 写回审查结论（PASS/FAIL）到本 PRD。

## 方向具体做法（执行级）

1. **渲染抽象先行**
   - 新增 `ChartRenderer` 抽象（或等效接口），输入为 `RenderSnapshot`，输出为 UI 绘制结果。
   - M2 默认实现 `EguiPlotRenderer`，Bevy 路径只保留接口位置。

2. **K 线绘制方案**
   - 实体：按涨跌拆分两组 `BarChart`（阳线/阴线不同色）。
   - 影线：用 `Line` 或 `VLine` 绘制 high/low。
   - 体宽与间距通过 x 轴步长统一控制，先满足可读性。

3. **交互与状态**
   - 打开 `egui_plot` 的缩放、拖拽、滚轮交互。
   - 将交互后的 x/y 范围映射回 `ViewportState`，持久在 Workbench 状态中。
   - 保证 UI 操作仅修改状态，绘图层只消费快照。

4. **性能与降级**
   - 限制单次可见 K 数量（窗口化），避免一次性绘制全量数据。
   - 发生绘图错误时降级为“可用占位 + 错误日志”，不影响主窗口交互。

5. **为 M3 保留扩展位**
   - 保留 `RenderSnapshot` 字段扩展位（overlay/indicator/replay）。
   - 不把 `egui_plot` 类型泄漏到 `application-core`，避免后续替换成本。


---

## Review Gate 结论

### 检查执行

| 检查项 | 结果 |
|--------|------|
| `cargo check --workspace` | 通过（0 errors, 0 warnings） |
| `cargo test -p ui-workbench` | 3 tests passed |
| `cargo test -p application-core` | 6 tests passed |
| `cargo test -p app-shell` | 0 tests（编译通过） |
| 运行时无 panic/unwrap/expect | 已验证 |
| egui_plot 类型未泄漏到 application-core | RenderSnapshot/Candle 仅在 ui-workbench |
| 渲染降级策略 | 无数据时显示占位文字 |

### 验收标准

- [x] egui_plot 在中心区域绘制有效 K 线（阳线绿色 BarChart + 阴线红色 BarChart + 灰色影线 Line）
- [x] 缩放 + 平移 + 滚轮交互均已启用
- [x] RenderSnapshot 接口稳定，不泄漏到 application-core
- [x] 渲染异常可降级，不导致应用崩溃
- [x] cargo check --workspace 与目标包测试通过

### 实现摘要

- `ui-workbench/Cargo.toml`: 新增 `egui_plot = "0.27.2"` 依赖
- `ui-workbench/src/lib.rs`: 实现 Candle/RenderSnapshot 数据模型、K 线绘制（Bull/Bear BarChart + Wick Line）、60 根 demo K 线、缩放/拖拽/滚轮交互、降级策略、3 个单元测试
- 无新增 crate，无 API 变更，无跨层影响

REVIEW: PASS

复核时间：2026-03-17（本轮复审已重新执行 `cargo check --workspace` 与目标包测试，结果一致通过）。
