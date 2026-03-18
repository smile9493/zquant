# 客户端状态规范

状态分 4 层：

1. DomainState：行情、任务、图表、会话等业务真相
2. UiState：停靠面板、筛选条件、选中项、滚动位置
3. ViewModel：为 egui / Bevy 整理后的只读投影
4. Command/Event：状态变化入口

规则：

- 真相状态只能经 reducer / service 更新。
- UI 不得绕过 service 直接修改领域对象。
- 后台任务完成后只提交结果对象，不直接触发多处写入。
- 所有跨线程共享状态必须有明确 owner。
