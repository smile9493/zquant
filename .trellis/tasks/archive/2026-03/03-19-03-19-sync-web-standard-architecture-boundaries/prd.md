# 同步架构硬边界到标准规划方案

## 目标
- 将 Trellis/Kiro 已固化的桌面架构硬边界同步到 `A:\zquant\docs\web\zquant_企业版标准规划方案.md`，实现三份文档同一口径。

## 范围
- 在标准规划方案中新增“不可违反”的架构边界规则。
- 固化 UI/Core/Renderer 契约方向、依赖单向约束、输入路由归属、状态分治规则。
- 对已有描述做最小必要修正，消除跨层直连歧义。

## 非目标
- 不改动任何 Rust 代码和运行时行为。
- 不新增新功能范围和里程碑。
- 不重写整份规划方案，仅做口径对齐补齐。

## 验收标准
- 文档中存在独立“架构硬边界（不可违反）”段落。
- 明确出现 4 条契约方向：
  - `UI -> Core: Command`
  - `Core -> UI: ViewModel/DTO`
  - `Core -> Renderer: RenderScene/RenderCommand`
  - `Renderer -> Core/UI: RenderEvent/PickingResult/FrameStats`
- 明确“输入路由由 app-shell 统一裁决”。
- 明确“业务真状态归 core/domain，渲染派生状态归 renderer-bevy”。
- 明确“ui-workbench（egui 层）与 renderer-bevy 禁止直连双向依赖”。

## 风险与应对
- 风险：新增约束与旧段落冲突造成执行歧义。  
  应对：同步修订相关章节用词，保持主线一致且避免重复定义。

## 实施计划
1. 阅读目标文档相关章节（设计原则、技术路线、架构、状态管理）。
2. 新增硬边界章节并补充依赖方向与契约。
3. 对状态管理章节补齐输入路由和状态分治表达。
4. 执行文档审查并回写结论。

## Checklist
- [x] 创建并设置 Trellis 任务
- [x] 完成 PRD（目标/范围/验收/风险/计划）
- [x] 更新标准规划方案文档
- [x] 核对约束口径一致性
- [x] 执行 review gate 并写入结论

---

## 实施结果

已更新文档：
- `A:\zquant\docs\web\zquant_企业版标准规划方案.md`

主要同步内容：
1. 新增 `6.1 架构硬边界（不可违反）`，固化 5 条硬边界原则。
2. 写入 UI/Core/Renderer 四向契约与禁止直连约束。
3. 在技术路线中补充输入路由协调与渲染契约约束。
4. 在状态管理中补充业务真状态/渲染派生状态分治。
5. 新增 `12.4 输入路由与冲突裁决`。
6. 在附录增加“建议依赖方向（同一口径）”。

---

## Review Gate

执行检查：
- `python ./.trellis/scripts/task.py validate 03-19-03-19-sync-web-standard-architecture-boundaries` ✅
- 关键约束检索：`6.1 架构硬边界`、四向契约、`app-shell` 输入路由、状态分治、ECS 禁止项 ✅
- 同口径依赖方向检索：`app_shell -> core/domain -> infra_*` ✅

验收结论：
- ✅ 已新增独立硬边界段落
- ✅ 四向契约方向完整
- ✅ 输入路由归属明确
- ✅ 状态分治规则明确
- ✅ UI/Renderer 直连禁令明确

**REVIEW: PASS**
