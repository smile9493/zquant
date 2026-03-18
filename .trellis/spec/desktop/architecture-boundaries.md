# Desktop 架构硬边界（不可违反）

## 适用范围

适用于 `apps/desktop-app` 与以下模块：`app-shell`、`core/domain`、`ui-egui`、`renderer-bevy`、`infra_*`。

---

## 总纲（固定表述）

本项目采用 **“Bevy runtime + bevy_egui integration + egui-first workbench shell + Bevy-first rendering subsystem”** 架构。  
业务状态集中在独立 `core/domain`；UI 与渲染通过 `DTO / command / event` 协议解耦；默认插件行为不构成产品架构依据，界面编排权始终归属 `egui` 主壳与 `app_shell`。

---

## 五条硬边界

1. **core/domain 纯净性（MUST）**
   - `core/domain` 只包含业务模型、用例、状态机、命令、事件、校验与调度策略。
   - 禁止依赖 `egui`、`bevy`、`bevy_ecs`、`wgpu`。

2. **契约通信（MUST）**
   - UI 与渲染器不得互相直连内部状态。
   - 跨层通信必须通过应用层契约（`Command / DTO / Event`）。

3. **egui 主编排（MUST）**
   - 主界面布局（导航、菜单、Dock/Tab、日志、设置、弹窗）由 `egui` 决定。
   - `Bevy` 只负责视口渲染，不负责主应用界面编排。

4. **拒绝默认耦合（MUST NOT）**
   - 不以 `bevy_ui` 作为主业务 UI 框架。
   - 不把 `bevy_egui` 当“仅调试 overlay”使用。
   - 不让默认 render graph 顺序决定产品分层。
   - 不把 ECS 当业务数据库。

5. **输入路由归一（MUST）**
   - 输入焦点与快捷键冲突统一由 `app_shell` 裁决。
   - `egui` 与 `renderer-bevy` 仅消费被分发后的输入。

---

## 固定通信契约

- `UI -> Core`: `Command`
- `Core -> UI`: `ViewModel / DTO`
- `Core -> Renderer`: `RenderScene / RenderCommand`
- `Renderer -> Core/UI`: `RenderEvent / PickingResult / FrameStats`

禁止关系：
- `ui-egui <-> renderer-bevy` 直接双向依赖
- `ui-egui -> renderer-bevy` 直接改 world/resource
- `renderer-bevy -> ui-egui` 直接读取 widget 内部状态

---

## 推荐目录与依赖方向

```text
crates/
  app_shell/
  core/
  ui_egui/
  renderer_bevy/
  infra_data/
  infra_bus/
  common/
```

依赖必须单向：

```text
ui_egui ------\
               -> app_shell -> core
renderer_bevy -/            -> infra_*
```

---

## 状态分治规则

业务真状态（归 `core/domain`）：
- 当前项目
- 策略参数
- 数据源配置
- 任务状态
- 工作区布局配置
- 回测条件

渲染派生状态（归 `renderer-bevy`）：
- 相机位置
- GPU 高亮态
- 帧统计
- hover / picking 临时结果
- 离屏纹理句柄
- 动画插值中间态

---

## 审查门禁（Desktop 架构）

- 是否出现 `core/domain` 对图形/UI 框架依赖？
- 是否存在 UI 与渲染器跨层直连？
- 是否由 `egui` 决定主布局与渲染面板尺寸位置？
- 输入冲突是否由 `app_shell` 统一路由？
- 业务真状态与渲染派生状态是否混存？
