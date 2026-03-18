# GUI 数据源与数据集分类展示

## 目标
- 在桌面 GUI 中明确展示“当前数据源（Provider）”与“数据集分类信息（Dataset/Market/Capability）”，避免仅显示 `Symbol/Timeframe` 的信息缺口。

## 范围
- `ui-workbench`：增加可视化区域，展示数据源与数据集分类。
- `application-core`：在图表加载返回结构中补充数据源与数据集元信息。
- `app-shell`：打通命令执行后的状态同步，把元信息传入 workbench 渲染状态。
- 必要的单元测试/集成测试更新。

## 非目标
- 不新增新的数据源接入类型。
- 不改造任务运行时与存储分层策略。
- 不做复杂筛选器/树形目录/权限管理。

## 验收标准
- GUI 可看到当前数据源（如 `akshare`）与数据集分类字段。
- 数据集分类至少包含：`dataset_id`、`market`、`capability`。
- 图表切换后，显示信息与当前加载数据一致，不出现旧状态残留。
- `cargo check --workspace` 通过，相关测试通过。

## 现状与问题
- 左侧“数据源”当前是静态文本，不是动态值。
- 右侧属性仅显示 `Symbol/Timeframe`，无 provider/dataset 分类。
- `application-core::load_chart` 仍是 placeholder，返回结构未包含元信息。

## 假设与风险
- 假设当前拉取链路已能提供 provider 与 dataset 语义（至少有默认值策略）。
- 风险：若后端未返回完整字段，GUI 可能出现空值；需定义兜底展示（`unknown`）。
- 风险：跨层结构调整可能影响已有测试断言。

## 实施计划
1. 扩展 `ChartData` / `RenderSnapshot` 元信息字段模型。
2. 在 `load_chart` 填充 provider 与 dataset 分类信息（先最小可用）。
3. 在 `app-shell` 命令处理后回写 `workbench` 渲染快照。
4. 调整 `ui-workbench` 左/右面板显示与空值兜底。
5. 补齐测试并执行 review gate。

## 执行清单
- [x] 定义跨层元信息字段（provider/dataset_id/market/capability）
- [x] 更新 application-core 返回结构
- [x] 更新 app-shell 与 workbench 的状态同步
- [x] 更新 GUI 展示与文案
- [x] 补充/修正测试
- [x] 运行 `cargo check --workspace`
- [x] 运行相关测试并记录结果
- [x] 写回审查结论（PASS/FAIL）

## Review Findings

### 第 1 轮（2026-03-18）

所有验收标准满足：
1. `ChartData` 新增 provider/dataset_id/market/capability 字段，`load_chart` 填充默认值。
2. `RenderSnapshot` 同步扩展，demo 数据带 metadata。
3. 左侧面板动态显示 Provider，右侧面板展示 dataset_id/market/capability。
4. `app-shell` 通过 mpsc channel 将 LoadChart 结果回传 workbench，每帧 drain 更新。
5. `cargo check --workspace` 通过，25 个测试全部通过。

## Root Cause
N/A

## Repair Plan
N/A

## Review Result
`REVIEW: PASS`

## 复审结论（2026-03-18，独立审查）

### 审查范围
- `crates/application-core/src/facade.rs`
- `crates/ui-workbench/src/lib.rs`
- `crates/app-shell/src/app.rs`

### 复核结果
- `ChartData` 已包含 `provider/dataset_id/market/capability`，并在 `load_chart` 中填充默认值。
- `RenderSnapshot` 已同步扩展上述字段，左侧可见动态 Provider，右侧可见数据集分类信息。
- `app-shell` 已通过异步 channel 回传 `LoadChart` 结果并在 UI 帧循环中更新渲染快照。

### 执行检查
- `cargo test -p application-core -p ui-workbench -p app-shell -p desktop-app`：25 passed, 0 failed
- `cargo check --workspace`：通过
- 代码扫描（上述范围）未发现生产路径 `panic!/expect/unwrap`。

### 最终判定
REVIEW: PASS
