# Desktop 架构硬边界固化（egui-first + bevy runtime）

## 目标
- 将桌面工作台架构约束从“对话共识”升级为“可执行工程规范”。
- 固化 `core/domain`、`app_shell`、`ui_egui`、`renderer_bevy` 的职责边界与依赖方向，避免实现期回退为跨层直连。

## 范围
- 更新 `.trellis/spec/desktop/*` 与 `.trellis/spec/guides/*`，写入不可违反的架构原则。
- 统一定义 UI/Core/Renderer 的命令、DTO、事件通信契约。
- 明确输入路由归属（`app_shell` 统一裁决）与状态分层（业务真状态 vs 渲染派生状态）。
- 给出建议目录与依赖方向，作为后续任务（M2/M3/M4）的前置约束。

## 非目标
- 不改动任何运行时代码或 crate 结构。
- 不在本任务内实现 DTO/Command/Event 的具体 Rust 类型。
- 不在本任务内完成 Bevy 渲染或 GUI 交互功能开发。

## 硬边界原则（不可违反）
1. **纯业务内核**：`core/domain` 必须保持纯 Rust 业务内核，不依赖 `egui`、`bevy`、`bevy_ecs`、`wgpu`。
2. **契约通信**：UI 与渲染器只能通过 `Command / DTO / Event` 契约通信，禁止跨层直连内部状态。
3. **egui 主编排**：`egui` 负责主界面布局与交互编排，`Bevy` 仅负责渲染面板内容。
4. **拒绝默认耦合**：插件默认行为不等于产品架构；禁止把 `bevy_ui` 当主业务 UI、禁止把 ECS 当业务数据库。
5. **输入路由归一**：输入焦点与快捷键冲突仲裁必须由 `app_shell` 统一处理。

## 验收标准
- 桌面规范文档中可检索到上述 5 条硬边界，且表述一致。
- 文档中明确通信契约方向：
  - `UI -> Core: Command`
  - `Core -> UI: ViewModel/DTO`
  - `Core -> Renderer: RenderScene/RenderCommand`
  - `Renderer -> Core/UI: RenderEvent/PickingResult/FrameStats`
- 文档中明确依赖单向约束与禁止关系（含 `ui_egui <-> renderer_bevy` 直连禁令）。
- 文档中明确“业务真状态”和“渲染派生状态”的分治规则。

## 假设与风险
- 假设：当前实现尚未大规模固化错误依赖，文档约束能在下一阶段及时阻断架构漂移。
- 风险：若仅更新单一文档，约束会在跨层任务中被遗漏。
  - 应对：同时更新 `desktop` 规范与 `cross-layer` 思考指南，并在审查阶段核对一致性。

## 实施计划
1. 在任务 PRD 中记录硬边界与验收标准。
2. 更新 `app-shell` 规范，明确编排权、输入路由、通信边界。
3. 更新 `renderer-bevy` 规范，明确渲染职责、禁止跨层读取、状态分治。
4. 更新 `workspace-state` 与 `cross-layer` 指南，补齐状态归属与依赖方向约束。
5. 执行文档审查，确认术语一致、无冲突规则、可直接用于后续任务评审。

## Checklist
- [x] 建立任务并设置为当前任务
- [x] 写入 PRD（目标/范围/非目标/验收/风险/计划）
- [x] 更新 desktop/app-shell 规范
- [x] 更新 desktop/renderer-bevy 规范
- [x] 更新 desktop/workspace-state 规范
- [x] 更新 guides/cross-layer-thinking-guide
- [x] 执行 review gate 并写入结论

---

## 实施结果（已完成）

### 新增规范
- `.trellis/spec/desktop/architecture-boundaries.md`
  - 固化 5 条不可违反架构原则
  - 固化 UI/Core/Renderer 契约方向
  - 固化目录建议、依赖单向约束、状态分治与审查门禁

### 已更新规范
- `.trellis/spec/desktop/index.md`
  - 将“架构硬边界”加入桌面规范清单并设为预开发必读
- `.trellis/spec/desktop/app-shell-guidelines.md`
  - 强化 `app_shell` 输入路由职责
  - 明确 `egui` 主编排与跨层契约约束
- `.trellis/spec/desktop/renderer-bevy-guidelines.md`
  - 明确 `Bevy` 仅负责渲染子系统、禁止 `bevy_ui` 主业务化
  - 明确 ECS 非业务数据库、状态分治规则
- `.trellis/spec/desktop/workspace-state-guidelines.md`
  - 增补“业务真状态 vs 渲染派生状态”与禁止事项
- `.trellis/spec/guides/cross-layer-thinking-guide.md`
  - 升级为中文跨层指南并加入 Desktop 专项硬约束
- `.trellis/spec/guides/index.md`
  - 增补必读入口，链接跨层指南与 desktop 硬边界

---

## Review Gate

### 审查项
- 规范一致性审查：检查 desktop 与 cross-layer 文档术语和边界是否一致。
- 规则可检索性审查：检查关键约束词是否在目标文档中可检索。
- Trellis 任务完整性审查：PRD 与 checklist 是否回写完整。

### 执行检查
- `python ./.trellis/scripts/task.py validate 03-19-03-19-architecture-hard-boundaries` ✅
- `Select-String` 关键字检索（`core/domain`、`ui-egui <-> renderer-bevy`、`app_shell`、`Command`、`RenderScene`、`ECS`）✅

### 验收结论
- ✅ 5 条硬边界已写入规范并统一口径
- ✅ UI/Core/Renderer 契约方向已固定
- ✅ 依赖单向与直连禁令已写入
- ✅ 业务真状态与渲染派生状态分治已写入

**REVIEW: PASS**
