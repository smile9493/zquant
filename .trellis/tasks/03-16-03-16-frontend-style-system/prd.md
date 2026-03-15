# Frontend Unified Style Management

## Source

- `A:\zquant\docs\web\zquant_最小前端架构与UI_Vue版.md`
- 当前前端实现：
  - `A:\zquant\web\src\components\TopBar.vue`
  - `A:\zquant\web\src\components\JobsTab.vue`
  - `A:\zquant\web\src\components\LogsTab.vue`
  - `A:\zquant\web\src\components\LeftSidebar.vue`
  - `A:\zquant\web\src\components\DataExplorerPanel.vue`

## Background

当前前端样式存在以下问题：

- 颜色、间距、圆角、边框、阴影在多个组件中直接硬编码
- 面板、列表项、状态标签、工具栏等视觉模式重复出现，但没有统一抽象
- Ant Design Vue 组件样式与业务自定义样式没有统一桥接层
- 后续如果引入主题切换、品牌换肤、统一状态色规则，当前结构扩展成本高

下一阶段需要先完成“统一样式管理”的设计与落地方案，使前端视觉层具备一致性、可维护性和可扩展性。

## Goal

建立一套可执行的前端统一样式管理方案，并完成最小落地骨架，覆盖：

- 设计 token
- 主题层
- 通用样式原语
- 现有核心界面的迁移策略

目标不是一次性重写所有页面，而是先建立一个后续可持续演进的样式系统。

## Scope

### In scope

#### 1. Design tokens
- 统一定义颜色、文字、间距、圆角、边框、阴影、层级、过渡时间
- 区分基础 token 与语义 token：
  - 基础 token：色板、间距标尺、radius 标尺
  - 语义 token：panel、toolbar、selected、danger、success、warning、muted text

#### 2. Theme layering
- 建立至少一套默认主题层
- 为后续 light / dark 切换预留结构
- 主题值通过 CSS variables 暴露，不在业务组件内部散落定义

#### 3. Shared UI primitives
- 统一抽象以下公共样式语义：
  - panel
  - toolbar
  - list item
  - selected item
  - status badge
  - action group
  - empty / loading / error text

#### 4. Ant Design Vue bridge
- 明确哪些样式走 antd token
- 明确哪些业务区域走自定义 CSS variables
- 保证业务组件不直接依赖大量 antd 默认色值

#### 5. Migration plan
- 明确首批迁移组件：
  - `TopBar`
  - `JobsTab`
  - `LogsTab`
  - `LeftSidebar`
  - `DataExplorerPanel`
- 规定迁移顺序和边界，避免大面积样式回归

#### 6. Verification plan
- 明确样式系统落地后的检查方式：
  - 构建通过
  - 核心页面视觉不破坏
  - 不再新增硬编码颜色 / 间距

### Out of scope

- 完整品牌视觉重做
- 全量页面一次性重构
- 动画系统重做
- 图表主题深度定制
- 无障碍专项整改
- 多品牌主题同时交付

## Non-goals

- 不在本任务里实现所有 UI 页面完全统一
- 不引入 CSS-in-JS、Tailwind 或新的大型样式框架
- 不重写 Ant Design Vue 组件库样式系统

## Constraints / Assumptions

- 前端继续使用 Vue 3 + Vite + Ant Design Vue
- 样式方案优先选择 CSS variables + 普通 CSS 文件 / SFC scoped 样式协作
- 迁移要尽量保持现有结构，避免为了样式系统重写组件逻辑

## Design Direction

建议采用三层结构：

### Layer 1: Tokens
- 文件示例：
  - `web/src/styles/tokens.css`
- 只定义最基础变量，不写业务选择器

示例分类：
- `--zq-color-bg-0`
- `--zq-color-bg-panel`
- `--zq-color-text-primary`
- `--zq-color-text-muted`
- `--zq-color-success`
- `--zq-space-1 ... --zq-space-6`
- `--zq-radius-sm/md/lg`
- `--zq-shadow-sm/md`

### Layer 2: Theme
- 文件示例：
  - `web/src/styles/theme-dark.css`
  - `web/src/styles/theme-light.css`
- 负责为语义 token 赋值
- 未来切换主题时只切换主题类或根节点属性

### Layer 3: Shared component styles
- 文件示例：
  - `web/src/styles/components.css`
  - `web/src/styles/utilities.css`
- 提供通用 UI 原语类：
  - `.zq-panel`
  - `.zq-toolbar`
  - `.zq-list-item`
  - `.zq-status-badge`
  - `.zq-empty-state`
  - `.zq-action-group`

### Component-level styles
- 各 Vue 组件保留自身布局细节
- 但不再新增硬编码色值、重复间距和重复状态色

## Proposed Files

### New
- `A:\zquant\web\src\styles\tokens.css`
- `A:\zquant\web\src\styles\theme-dark.css`
- `A:\zquant\web\src\styles\theme-light.css`
- `A:\zquant\web\src\styles\components.css`
- `A:\zquant\web\src\styles\utilities.css`
- `A:\zquant\web\src\styles\index.css`

### Likely touched
- `A:\zquant\web\src\main.ts`
- `A:\zquant\web\src\App.vue`
- `A:\zquant\web\src\components\TopBar.vue`
- `A:\zquant\web\src\components\JobsTab.vue`
- `A:\zquant\web\src\components\LogsTab.vue`
- `A:\zquant\web\src\components\LeftSidebar.vue`
- `A:\zquant\web\src\components\DataExplorerPanel.vue`

## Acceptance Criteria

### Design
- [ ] 样式系统分为 token / theme / shared component styles 三层
- [ ] 有明确命名规范，避免继续新增魔法值
- [ ] 有 antd bridge 规则，说明哪些走组件库 token，哪些走业务 token

### Implementation
- [ ] 新增全局样式入口并接入应用
- [ ] 至少完成一组核心 token 定义（颜色、间距、圆角、阴影、文字）
- [ ] 至少完成 panel / toolbar / list item / status badge 的公共样式抽象
- [ ] 至少迁移 2-3 个核心组件到新样式体系

### Quality
- [ ] `npm run build` 通过
- [ ] 核心页面无明显视觉回归
- [ ] 新迁移组件不再依赖新增硬编码颜色和随意间距

### Review gate
- [ ] PRD 最终记录 `REVIEW: PASS` 或 `REVIEW: FAIL`

## Risks

- 现有组件多使用局部 `scoped` 样式，统一迁移时可能出现优先级冲突
- Ant Design Vue 默认样式与自定义变量桥接不当，可能导致局部视觉不一致
- 如果一次迁移范围过大，容易引入广泛视觉回归

## Implementation Plan

1. 盘点现有组件中重复出现的硬编码样式
2. 设计 token 命名规范与三层目录结构
3. 建立全局样式入口并接入应用
4. 抽取 panel / toolbar / list / badge 通用样式
5. 首批迁移核心组件
6. 校验 antd 与业务样式边界
7. 执行 build 与人工视觉检查
8. 走 review gate 并更新任务状态

## Checklist

- [ ] 盘点当前样式重复点和硬编码值
- [ ] 明确 token 命名规范
- [ ] 新建 styles 目录与入口文件
- [ ] 定义基础 token 与语义 token
- [ ] 建立 theme 层
- [ ] 建立 shared component styles 层
- [ ] 接入 `main.ts` 或全局入口
- [ ] 迁移 `TopBar`
- [ ] 迁移 `JobsTab`
- [ ] 迁移 `LogsTab` / `LeftSidebar` / `DataExplorerPanel` 中至少一个
- [ ] 执行构建与回归检查
- [ ] 完成 review gate

## Verification Plan

建议执行：

- `cd A:\zquant\web && npm run build`
- 必要时运行组件测试，确保样式重构没有破坏既有交互
- 手动检查以下界面：
  - 工作区主布局
  - TopBar
  - JobsTab
  - LogsTab
  - LeftSidebar

## Review Notes

初始状态：仅创建任务与 PRD，尚未开始实现。

## Review Outcome

REVIEW: FAIL
