# egui UI 规范

依据 egui 官方文档，egui 是 immediate mode GUI。
这意味着：

- 每帧重新声明 UI。
- UI 是状态的投影，不是长期保存的控件树。
- 绘制函数必须轻量、可重复执行。
- 阻塞 IO、数据库访问、重计算不得出现在 `update()` 或绘制闭包中。

规则：

1. 所有 egui 面板函数命名统一为 `show_xxx_panel(...)`。
2. 面板函数只做：
   - 读取 ViewModel / UI state
   - 渲染控件
   - 生成命令或事件
3. 面板函数不得直接：
   - 发 HTTP 请求
   - 执行 SQL
   - 扫描文件系统
   - 进行大规模数据变换
4. 异步资源必须走缓存与后台任务，完成后请求 repaint。
5. 不在 egui 层维护业务真相状态；egui 只持有局部 UI state。

推荐模式：

- `AppState`：全局真相状态
- `UiState`：面板展开/选中/tab/排序等短期状态
- `Command`：UI 发给应用层的命令
