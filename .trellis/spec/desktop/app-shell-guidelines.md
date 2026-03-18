# App Shell 规范（egui）

## 适用范围

适用于 `apps/desktop-app` 与 `crates/app-shell`、`crates/ui-workbench`。

开始实现前必须先读：`./architecture-boundaries.md`。

## 布局原则

- 固定工作台框架：`TopBar | Left Sidebar | Center Canvas | Right Dock | Bottom Dock`。
- 中心画布优先：默认情况下中心区域应占主视图面积。
- 高信息密度区域（Right Dock、Bottom Dock）不承担主流程跳转。
- `egui` 决定主界面结构与渲染区域尺寸/位置分配；`Bevy` 只渲染被分配区域内容。

## 职责边界

- `app-shell`：窗口生命周期、全局菜单、快捷键、输入路由、主题、状态栏、跨层协同。
- `ui-workbench`：面板编排、组件组合、交互事件转发、ViewModel 消费与展示。
- 业务逻辑不得直接写在 UI 组件中，必须通过应用层命令接口调用。
- 禁止 `ui-workbench` 直接访问 `renderer-bevy` 内部 world/resource。

## 契约约束

- `UI -> Core`：只发送 `Command`。
- `Core -> UI`：只下发 `ViewModel / DTO`。
- `Core -> Renderer`：只下发 `RenderScene / RenderCommand`。
- `Renderer -> Core/UI`：只回传 `RenderEvent / PickingResult / FrameStats`。
- 禁止 `ui-egui <-> renderer-bevy` 直接双向依赖。

## 状态约束

- UI 临时状态（面板展开、选中态）与工作区状态分离。
- 任意可恢复状态都需要可序列化快照。
- 不允许跨组件共享可变全局状态作为隐式通信手段。
- `core/domain` 保存业务真状态；UI 仅保存展示与临时交互态。

## 交互约束

- 关键动作（刷新、导入、回放启动）必须有可见反馈。
- 异常反馈分级：可恢复提示（warn）与阻断错误（error）分离。
- 快捷操作必须有菜单入口或按钮入口，避免“仅快捷键可达”。
- 输入冲突（滚轮、拖拽、快捷键）必须由 `app-shell` 统一裁决再分发。

## 质量门禁

- 主窗口可启动且布局稳定。
- 主要面板（Top/Left/Right/Bottom）可独立显示与开关。
- 状态栏可显示最小健康信息（连接、任务数、错误数）。
- 检查 UI 层是否只通过契约访问业务与渲染，不存在直连状态写入。
