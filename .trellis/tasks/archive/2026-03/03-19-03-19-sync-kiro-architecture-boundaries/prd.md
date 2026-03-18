# 同步架构硬边界到 Kiro roadmap 设计

## 目标
- 将已在 Trellis 固化的桌面架构硬边界，同步写入 `A:\zquant\.kiro\specs\zquant-enterprise-evolution-roadmap\design.md`。
- 确保 Kiro 设计文档与 Trellis 规范在分层边界、契约通信、输入路由上保持一致。

## 范围
- 更新 `design.md` 的架构章节，新增“不可违反”工程约束。
- 明确 UI/Core/Renderer 契约方向、依赖单向约束、状态分治、输入路由归属。
- 对已有架构描述中可能引发歧义的内容做最小必要修正。

## 非目标
- 不改动任何 Rust 代码实现。
- 不改动 Trellis 规范正文（本任务仅同步到 Kiro 设计文档）。
- 不新增里程碑功能项（仅文档约束对齐）。

## 验收标准
- `design.md` 中存在独立“架构硬边界/工程约束”段落。
- 明确出现以下通信契约方向：
  - `UI -> Core: Command`
  - `Core -> UI: ViewModel/DTO`
  - `Core -> Renderer: RenderScene/RenderCommand`
  - `Renderer -> Core/UI: RenderEvent/PickingResult/FrameStats`
- 明确出现输入路由归属：由 `app_shell` 统一裁决。
- 明确出现状态分治：业务真状态归 `core/domain`，渲染派生状态归 `renderer-bevy`。

## 假设与风险
- 假设：Kiro 文档是后续实现与审查的重要依据，需要与 Trellis 保持一致。
- 风险：仅新增段落但不处理冲突描述，可能导致团队执行分歧。
  - 应对：同步检查并最小修正明显冲突表达。

## 实施计划
1. 阅读 `design.md` 架构与组件章节，识别冲突点。
2. 增补“架构硬边界（不可违反）”章节。
3. 在关键组件描述中补齐约束语句（输入路由、契约通信、状态边界）。
4. 执行文档审查并回写 review 结论。

## Checklist
- [x] 创建并设置 Trellis 任务
- [x] 完成 PRD（目标/范围/验收/风险/计划）
- [x] 更新 Kiro 设计文档架构约束
- [x] 核对契约方向与分层边界一致性
- [x] 执行 review gate 并写入结论

---

## 实施结果

已完成文档同步文件：
- `A:\zquant\.kiro\specs\zquant-enterprise-evolution-roadmap\design.md`

主要更新：
1. 在架构章节新增“架构硬边界（不可违反）”与“依赖方向与状态分治”。
2. 固化 UI/Core/Renderer 四向契约与输入路由归属（`app-shell`）。
3. 调整 To-Be 架构图与 M2/M4 流程图，去除 UI/Renderer 直连 `application-core` 的歧义表达，改为由 `app-shell` 协同路由。
4. 在 `app-shell`、`ui-workbench`、`renderer-bevy` 组件接口中写入硬边界说明。
5. 在 `application-core` 状态模型中去除 `ui_state` 歧义，改为 `workspace_state`。

---

## Review Gate

执行检查：
- `python ./.trellis/scripts/task.py validate 03-19-03-19-sync-kiro-architecture-boundaries` ✅
- 关键约束检索（`UI -> Core`、`Core -> Renderer`、`输入路由权归 app-shell`、`业务真状态/渲染派生状态`）✅
- 流程图协同路径检索（`participant Shell as app-shell`、`Shell->>AppCore`、`Shell->>Bevy`、`Shell-->>Workbench`）✅

验收标准结果：
- ✅ 已有独立硬边界章节
- ✅ 四向契约方向完整
- ✅ 输入路由归属明确
- ✅ 状态分治明确并含 ECS 禁止项

**REVIEW: PASS**
