# 任务与错误规范

错误：

- 所有对外错误使用统一 `AppError`。
- 错误必须保留 machine-readable code。
- 不直接 `anyhow!` 到 UI 边界。

任务：

- 后台任务必须有状态：
  - queued
  - running
  - succeeded
  - failed
  - cancelled
- 长任务必须可观测，可取消，能输出进度。
- UI 只订阅任务状态，不自己推断后台过程。
