# 服务层规范

1. service 层负责编排业务动作，不负责 UI，不负责渲染。
2. service 可以调用 repository、job runtime、cache。
3. service 返回稳定结果类型：
   - `Result<T, AppError>`
4. 一个 service 方法只表达一个业务意图。
5. 禁止把"为了某个 UI 临时凑数据"的逻辑下沉污染通用 service。
