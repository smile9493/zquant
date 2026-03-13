# zquant 最小前端架构与 UI 方案（Vue 版）

## 1. 结论

基于 zquant 当前后端完成状态与两份前端架构文档，前端不应直接照搬完整 Terminus 终端，而应落为一个 **Vue 单页工作台 MVP**：

- 单入口：`/workspace`
- 中心永远是图表（Chart First）
- 右侧只保留 2 个长流程面板
- 底部只保留 2 个持续状态面板
- Phase 1 不依赖 WebSocket，不依赖 Kafka/Redis，不做前端实时事件总线完整版
- 先基于 HTTP + PG SSOT + 定时刷新/手动刷新 建立最小可用前端

## 2. 约束依据

### 2.1 来自 zquant 当前后端

Phase 1 当前约束是：
- 不引入 Kafka
- 不引入 Redis
- 不提供 WebSocket Bridge
- 只保证单机内核形态：HTTP `job-api` + `job-runner` + in-memory event bus + PostgreSQL SSOT

因此前端 MVP **不能把 WebSocket First、Typed Event Bus、前端乐观事件缝合作为上线前置条件**，这些只能作为后续阶段增强。

### 2.2 来自 Terminus 文档中值得保留的骨架

建议保留：
- 单一入口（Single Workspace Entry）
- Chart First
- Panel / Dock 交互，而不是多页面后台
- URL-as-State
- Chart Core 独立层
- Jobs / Logs 作为 Bottom Dock 的持续状态区
- Data Explorer 作为 Right Dock 的工具区

### 2.3 来自统一数据管道骨架的前端需求

统一数据接入骨架已经明确：`DataSourceManager` 是统一入口，向 K 线图、回测引擎、AI Agent 提供统一拉取与数据集接口。因此前端最小可见入口必须至少覆盖：
- 行情 / K 线浏览
- 数据集选择 / 数据源探查
- Job 状态查看
- 日志与消息输出

## 3. 最小前端目标（只做 MVP，不做完整版）

MVP 只交付一个 **研究工作台**，不交付完整交易终端：

### 必做
- 单页 `/workspace`
- 一个主图表区域
- 一个右侧工具区
- 一个底部状态区
- HTTP API 封装
- URL 状态同步
- 最小权限/模式显示

### 暂不做
- WebSocket 实时推送
- 前端 Typed Event Bus / Gap Detection
- 乐观 UI 完整状态机
- Agent 多面板协同
- 无限画布 / 多图拖拽网格
- Arrow IPC / OffscreenCanvas / Worker Data Plane
- Cmd+K 指令系统
- Live/Paper 完整交易流

## 4. Vue 最小前端架构

采用 Vue 3 + TypeScript + Vite。

### 4.1 技术栈

- Vue 3
- TypeScript
- Vue Router
- Pinia
- Vue Query（@tanstack/vue-query）
- Ant Design Vue 或 Naive UI（二选一，建议 Ant Design Vue）
- lightweight-charts（主 K 线）
- ECharts（普通统计图，可后置）

### 4.2 分层

```text
src/
├─ app/
│  ├─ router/
│  ├─ providers/
│  └─ layouts/
├─ pages/
│  └─ workspace/
├─ widgets/
│  ├─ chart-panel/
│  ├─ data-explorer/
│  ├─ jobs-dock/
│  └─ logs-dock/
├─ features/
│  ├─ select-symbol/
│  ├─ switch-timeframe/
│  └─ job-actions/
├─ entities/
│  ├─ market/
│  ├─ job/
│  └─ datasource/
├─ shared/
│  ├─ api/
│  ├─ ui/
│  ├─ utils/
│  └─ config/
└─ styles/
```

说明：
- `pages` 只负责页面拼装
- `widgets` 负责可复用业务块
- `features` 只负责用户动作
- `entities` 负责领域读模型
- `shared/api` 统一访问后端 PG 控制面 API

## 5. 最小 UI 结构

### 5.1 页面骨架

```text
┌─────────────────────────────────────────────────────────┐
│ TopBar: mode / symbol / timeframe / refresh / status   │
├───────────────┬───────────────────────────┬────────────┤
│ Left Sidebar  │ Center Canvas             │ Right Dock │
│               │ Price Chart Panel         │ Data       │
│ Watchlist     │                           │ Explorer   │
│ Favorites     │                           │ +          │
│ Quick Nav     │                           │ Governance │
├───────────────┴───────────────────────────┴────────────┤
│ Bottom Dock: Jobs | Logs                                │
└─────────────────────────────────────────────────────────┘
```

### 5.2 四个最小核心块

#### A. TopBar
包含：
- 模式：只启用 `research`
- Symbol 选择
- Timeframe 选择
- 数据刷新按钮
- 后端连接状态（HTTP 可用/失败）

#### B. Center Canvas
只放一个主图：
- `PriceChartPanel`
- 内容：K 线 + 成交量
- 可后续扩展 MA / 指标叠加

#### C. Right Dock
只保留 2 个面板：
- `DataExplorerPanel`
  - 数据源/数据集列表
  - symbol 搜索
  - timeframe 切换
- `GovernanceSummaryPanel`
  - 当前 mode
  - API 健康状态
  - 最近一次错误
  - 只读的系统摘要

#### D. Bottom Dock
只保留 2 个 Tab：
- `JobsTab`
  - 任务列表
  - 状态
  - created_at / updated_at
  - retry / stop（若后端已有）
- `LogsTab`
  - 任务日志 / 系统日志
  - 先用分页或虚拟列表

## 6. 为什么这样最适合 zquant

### 6.1 和当前后端匹配

当前 zquant Phase 1 只有：
- job-api
- job-runner
- PG SSOT
- 本地 in-memory event bus

没有 WebSocket Bridge，所以前端应先基于：
- REST 查询 jobs
- REST 查询 job detail / logs
- REST 查询 datasource / market data
- 轮询刷新 jobs 与 logs

### 6.2 和统一数据管道匹配

`DataSourceManager` 已经是统一入口。前端不应该知道 Provider 细节，只应该消费：
- K 线数据
- 数据集列表
- provider 健康/能力摘要

也就是说，前端右侧 `DataExplorerPanel` 对接的是 **Manager 视图**，不是底层 provider 直连。

### 6.3 和 Terminus 文档的保留原则匹配

保留了最重要的三点：
- 单页工作台
- 图表为中心
- Dock 代替页面跳转

但主动删除了当前 zquant 暂时没有后端条件支撑的复杂能力。

## 7. 最小路由设计

```text
/workspace
/workspace?symbol=BTCUSDT&timeframe=1h&right=data&bottom=jobs
```

URL 最小字段：
- `symbol`
- `timeframe`
- `right`
- `bottom`

当前阶段不需要：
- layout
- modal
- 多 panel 持久化
- overlay 栈回退策略完整版

## 8. 最小状态管理

### Pinia 中只保留三个 Store

#### useWorkspaceStore
- symbol
- timeframe
- rightPanel
- bottomTab
- mode（research only）

#### useJobStore
- jobs list
- selected job id
- last refresh time
- loading / error

#### useDataSourceStore
- datasource summary
- dataset list
- selected dataset
- market filters

### 不做
- optimistic mutation 队列
- seq/gap detection
- global command registry
- event bus reconciler

## 9. 最小 API 面

前端当前只需要 4 组 API：

### 9.1 Market API
- `GET /api/market/kline`

### 9.2 DataSource API
- `GET /api/datasources`
- `GET /api/datasets`

### 9.3 Job API
- `POST /jobs`
- `GET /jobs`
- `GET /jobs/:id`
- `POST /jobs/:id/stop`
- `POST /jobs/:id/retry`

### 9.4 Logs / Observability API
- `GET /jobs/:id/logs`
- `GET /system/health`

如果部分 API 现在还没有，优先先补：
- `GET /jobs`
- `GET /jobs/:id`
- `GET /jobs/:id/logs`
- `GET /datasources`
- `GET /market/kline`

## 10. 最小视觉规范

延续 Neo-Glass 的方向，但降级为轻量版：

### 色板
- 背景：#050505
- 主强调：#2979ff
- 成功：#00e676
- 失败：#f50057
- 边框：rgba(255,255,255,0.08)

### 组件规则
- Center Chart：深色背景，边框弱化
- Right Dock：半透明，弱 glow
- Bottom Dock：清晰分割线，高信息密度
- Modal：仅用于 stop / retry / danger confirm

### 字体
- 主字体：Inter
- 数字与日志：JetBrains Mono

## 11. 建议的 Vue 目录落点

```text
src/
├─ app/
│  ├─ App.vue
│  ├─ router.ts
│  └─ providers/
├─ pages/
│  └─ workspace/
│     ├─ WorkspacePage.vue
│     └─ WorkspaceLayout.vue
├─ widgets/
│  ├─ chart-panel/
│  │  └─ PriceChartPanel.vue
│  ├─ data-explorer/
│  │  └─ DataExplorerPanel.vue
│  ├─ jobs-dock/
│  │  └─ JobsTab.vue
│  └─ logs-dock/
│     └─ LogsTab.vue
├─ features/
│  ├─ symbol-selector/
│  ├─ timeframe-selector/
│  └─ job-actions/
├─ entities/
│  ├─ market/
│  ├─ job/
│  └─ datasource/
├─ shared/
│  ├─ api/
│  ├─ ui/
│  ├─ hooks/
│  └─ utils/
└─ stores/
   ├─ workspace.ts
   ├─ jobs.ts
   └─ datasource.ts
```

## 12. 一句话版本

zquant 的 Vue 前端现在最合理的落点，不是“完整 Terminus”，而是：

**一个以 K 线图为中心的单页工作台（Workspace Shell），右侧放数据探查与治理摘要，底部放 Jobs 与 Logs，全部基于 HTTP + PG SSOT 运行。**

## 13. 下一步实施顺序

### Phase A
- 搭建 Vue Workspace Shell
- 接入 PriceChartPanel
- 接入 JobsTab

### Phase B
- 接入 DataExplorerPanel
- 接入 LogsTab
- 接入 URL 状态同步

### Phase C
- 加 GovernanceSummaryPanel
- 加 stop/retry confirm modal
- 加轮询刷新与错误态

### Phase D（后续增强）
- WebSocket bridge
- Typed Event Bus
- Optimistic UI
- Agent Panel
- 多图网格布局
- Cmd+K
