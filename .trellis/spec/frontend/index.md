# 前端规范索引

本项目"前端"指 Rust 本地客户端，不指浏览器前端。

包含两层：

- egui：窗口壳、面板、表单、表格、状态栏
- Bevy：中心画布、图表、时间轴、交互可视化

基本原则：

- egui 负责 GUI 壳与交互编排。
- Bevy 负责实时可视化和渲染。
- 不允许把复杂业务逻辑直接写进 egui 绘制函数。
- 不允许把传统 GUI 面板逻辑塞进 Bevy ECS system。
- UI 读取状态并发出命令，不直接拼接数据库访问逻辑。

必读：
- egui-ui.md
- bevy-rendering.md
- app-state.md
- performance.md
