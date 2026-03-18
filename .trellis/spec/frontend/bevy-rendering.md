# Bevy 渲染规范

依据 Bevy 官方文档：

- `App` 是应用入口。
- `Plugin` 负责配置 App。
- `System` 运行在 schedule 中，通常通过 `add_systems(Update, ...)` 注册。
- 系统执行可能并行，不能依赖书写顺序表达执行顺序。

规则：

1. Bevy 只负责实时画布与可视化，不承载传统表单式 GUI。
2. 所有渲染功能必须通过插件组织：
   - `ChartPlugin`
   - `OverlayPlugin`
   - `InteractionPlugin`
   - `DataSyncPlugin`
3. ECS component 只存数据，不塞业务服务对象。
4. system 必须按阶段或 set 分组：
   - input
   - transform
   - layout
   - render-sync
5. 有顺序要求的 system 必须显式声明 before/after 或 in_set。
6. 默认禁止无必要地启用 Bevy 默认能力；feature 维持最小集。
7. Bevy 世界与 egui 壳之间只通过明确的数据桥接层通信。

禁止项：

- 在 system 中直接访问数据库连接池
- 在 system 中直接写磁盘
- 在 component 中持有复杂所有权对象
- 通过"碰巧运行顺序"维持逻辑正确性
