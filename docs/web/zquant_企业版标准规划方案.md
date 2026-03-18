# zquant 企业版标准规划方案

## 1. 文档信息

**项目名称**：zquant 本地研究工作台（Windows 优先）  
**文档类型**：企业级标准规划方案  
**版本**：V1.0  
**适用阶段**：立项评审 / 架构评审 / 开发启动 / 分阶段验收  
**目标产品形态**：本地单机优先的量化研究与可视化桌面客户端  

---

## 2. 执行摘要

zquant 定位为一套面向研究、分析、回放与数据管理场景的本地桌面工作台。Phase 1 以 **Windows 平台单机可运行** 为第一目标，采用 **`egui` 作为桌面业务 GUI 壳、`Bevy renderer` 作为中央高频可视化引擎、`PostgreSQL + Parquet` 作为本地双层存储体系** 的总体技术路线。

本方案不以浏览器端、微服务优先或复杂分布式实时通信为启动前提，而是先建立一套可独立部署、可本地运行、可持续扩展、可逐步企业化演进的桌面研究终端基础平台。后续可在此基础上逐步演进到多数据源、多任务、多策略、多用户协作和受控企业分发模式。

本规划方案的核心目标有三项：

1. 建立符合企业研发标准的产品边界、技术边界和实施边界。  
2. 在不牺牲后续扩展性的前提下，优先实现 Phase 1 的最小可用闭环。  
3. 将数据存储、任务执行、渲染显示、配置管理、日志审计等能力纳入统一规范，避免后期反复推翻架构。

---

## 3. 项目背景与建设必要性

### 3.1 背景

现有量化研究或市场数据分析工作通常存在以下问题：

- 数据拉取、落库、归档、浏览、分析链路分散，缺少统一桌面入口。
- 通用 Web 控制台更适合业务管理，不适合高频图形可视化与本地研究工作流。
- Python 脚本式研究流程灵活但缺少工程化壳层，不利于标准化交付。
- 历史数据文件、元数据索引、任务状态、研究工作区常被混杂管理，后期难以维护。
- Windows 桌面环境是实际使用主场景，但很多方案并未从 Windows 的部署与文件语义出发设计。

### 3.2 建设必要性

zquant 需要解决的不是“做一个更炫的前端”，而是构建一套面向研究工作流的标准化本地客户端基础设施，具体必要性如下：

- 为量化研究、图形分析、数据回放提供统一入口。
- 为后续指标、脚本、策略、扩展模块提供稳定宿主。
- 为远端数据提供者与本地数据归档建立可追踪、可恢复、可演进的中间层。
- 为企业内部后续版本提供标准化架构基线，降低新增功能的系统性风险。

---

## 4. 建设目标

### 4.1 总体目标

构建一套以 Windows 为主要运行平台、以本地单机为优先部署方式、以图表与可视化分析为核心交互形态的桌面研究工作台。

### 4.2 分阶段目标

#### Phase 1：本地单机 MVP

- 本地运行一个完整桌面应用。
- 通过 HTTP 接口拉取 symbol 与 OHLCV 数据。
- 使用 PostgreSQL 保存元数据、热数据、任务状态、workspace。
- 使用 Parquet 保存历史归档与批量分区数据。
- 中央图表可进行浏览、切换、缩放、刷新与基本叠加显示。
- 具备右侧数据面板、底部日志/任务面板与基本状态栏。

#### Phase 2：研究增强版

- 增加指标系统、标注系统、回放系统。
- 增加导入导出、数据校验、任务重试、分区压缩能力。
- 增加本地脚本/分析扩展接口。
- 增加更多 provider 接入能力。

#### Phase 3：企业协同版

- 增加统一配置下发能力。
- 增加标准化安装包、升级链路、部署约束和许可证机制。
- 增加企业内部认证、操作审计、受控扩展与受控数据接入。
- 逐步支持远端任务编排、集中同步与团队协作场景。

---

## 5. 项目范围

### 5.1 本期范围（In Scope）

- Windows 桌面客户端。
- 单用户、本地单机运行模式。
- HTTP 数据拉取。
- 本地 PostgreSQL + Parquet 存储体系。
- 图表浏览与中心画布渲染。
- Workspace、任务、日志、数据浏览、刷新管理。
- 基础配置、错误提示、日志记录、异常恢复。

### 5.2 非本期范围（Out of Scope）

- Web 版控制台。
- 多用户协同编辑。
- SaaS 化多租户。
- 高频交易执行终端。
- 复杂分布式消息系统（Kafka/Redis）作为 Phase 1 前提。
- 运行时动态 DLL 热插拔插件市场。
- 企业域控、SSO、集中审计等企业级治理能力在 Phase 1 的全面落地。

---

## 6. 总体设计原则

1. **单机优先**：先确保脱离复杂基础设施也可完整运行。  
2. **中心画布优先**：图表与可视化是产品主交互，不以页面跳转为核心。  
3. **壳核分离**：`egui` 承担传统 GUI；`Bevy renderer` 承担高频图形渲染。  
4. **控制面与归档面分离**：PostgreSQL 管状态与索引；Parquet 管历史数据本体。  
5. **接口先行**：Provider、Repository、Job、Renderer、Workspace 均以 trait 或稳定接口抽象。  
6. **静态扩展优先**：本阶段采用模块化与静态注册扩展，不做运行时热插拔。  
7. **Windows 约束前置**：路径、文件写入、进程模型、安装方式从一开始就纳入方案。  
8. **可治理性优先于炫技**：保证可部署、可诊断、可追踪、可恢复。  
9. **架构硬边界不可违反**：插件默认行为不等于产品架构，必须遵守分层契约。  

### 6.1 架构硬边界（不可违反）

> 本节为实现期强制规则，优先级高于局部实现便利性。

1. **core/domain 纯净性（MUST）**
   - `core/domain` 仅承载业务模型、用例、状态机、命令、事件、校验与调度策略。
   - 禁止依赖 `egui`、`bevy`、`bevy_ecs`、`wgpu`。

2. **契约通信（MUST）**
   - `UI -> Core`：`Command`
   - `Core -> UI`：`ViewModel / DTO`
   - `Core -> Renderer`：`RenderScene / RenderCommand`
   - `Renderer -> Core/UI`：`RenderEvent / PickingResult / FrameStats`
   - 禁止 `ui-workbench（egui 层） <-> renderer-bevy` 直接双向依赖或共享可变内部状态。

3. **egui 主编排（MUST）**
   - 主界面结构由 `egui` 控制；`Bevy` 仅负责渲染面板内容。
   - 不允许让 `Bevy` 接管整个桌面 GUI 生命周期。

4. **拒绝默认耦合（MUST NOT）**
   - 不以 `bevy_ui` 作为主业务 UI 框架。
   - 不把 `bevy_egui` 当纯调试 overlay 使用。
   - 不把 ECS 当业务数据库。

5. **输入路由归一（MUST）**
   - 输入焦点、滚轮、拖拽、快捷键冲突统一由 `app-shell` 裁决后分发。
   - `ui-workbench` 与 `renderer-bevy` 仅消费路由后的输入。

---

## 7. 技术路线与定版结论

### 7.1 前端/桌面壳

- **技术选型**：`eframe/egui`
- **职责**：窗口宿主、工具栏、右侧面板、底部面板、日志区、配置区、状态区、快捷操作、输入路由协调
- **原因**：开发效率高、Rust 原生、适合传统桌面交互与快速迭代

### 7.2 中心可视化渲染

- **技术选型**：`Bevy renderer`（以 Bevy 渲染能力为核心，不让 Bevy 管整个桌面 GUI）
- **职责**：K 线、成交量、overlay、交互式时间轴、缩放、平移、光标、标记、回放画布
- **契约约束**：仅消费 `RenderScene/RenderCommand`，并通过 `RenderEvent` 回传交互结果
- **原因**：适合高频刷新与复杂图元渲染，利于后续扩展为策略回放与图形分析内核

### 7.3 本地存储

- **技术选型**：`PostgreSQL + Parquet`
- **结论**：该组合为固定架构约束，不再使用 SQLite 作为默认主存储

#### PostgreSQL 负责

- 元数据
- workspace 状态
- 任务状态
- 热数据窗口
- partition manifest
- 同步水位、错误记录、刷新状态

#### Parquet 负责

- 历史归档
- 大范围列式扫描
- 分区存储
- 导入导出
- 后续批量分析输入

### 7.4 网络与后台任务

- **技术选型**：`tokio + reqwest`
- **职责**：HTTP 拉取、定时刷新、后台任务执行、取消控制、错误传播

### 7.5 日志与诊断

- **技术选型**：`tracing`
- **职责**：运行日志、任务日志、错误链、审计上下文、诊断定位

---

## 8. 企业版目标架构

### 8.1 逻辑架构

```text
┌──────────────────────────────────────────────────────────────┐
│                       zquant-desktop.exe                     │
├──────────────────────────────────────────────────────────────┤
│ Presentation Layer                                           │
│  - egui App Shell                                             │
│  - TopBar / Sidebar / Right Dock / Bottom Dock               │
├──────────────────────────────────────────────────────────────┤
│ Visualization Layer                                           │
│  - Bevy Renderer                                              │
│  - Chart Surface / Overlay / Replay Surface                  │
├──────────────────────────────────────────────────────────────┤
│ Application Layer                                             │
│  - Command Bus / Reducer / Workspace State                   │
│  - Jobs Runtime / Scheduler / Notifications                  │
│  - Use Cases (Refresh, Query, Load, Export, Replay)          │
├──────────────────────────────────────────────────────────────┤
│ Domain Layer                                                  │
│  - Market Model / Candle / Symbol / Dataset                  │
│  - Job Model / Workspace Model / Partition Model             │
├──────────────────────────────────────────────────────────────┤
│ Infrastructure Layer                                          │
│  - HTTP Provider                                              │
│  - PostgreSQL Catalog Store                                   │
│  - Parquet Archive Store                                      │
│  - File Paths / Config / Logging                              │
└──────────────────────────────────────────────────────────────┘
```

### 8.2 物理部署架构（Phase 1）

```text
Windows Workstation
 ├─ zquant-desktop.exe
 ├─ Local PostgreSQL Service / Existing PostgreSQL Instance
 └─ Local File System (Parquet Archive, Logs, Temp, Config)
```

### 8.3 演进架构（Phase 2 / 3）

```text
Windows Clients
 ├─ zquant-desktop.exe
 ├─ Remote Provider APIs
 ├─ Optional Central Metadata / Policy Service
 └─ Optional Shared Distribution / Update Channel
```

---

## 9. 功能规划

### 9.1 核心功能域

#### 9.1.1 市场数据管理

- symbol 清单同步
- OHLCV 拉取与刷新
- timeframe 切换
- 热数据缓存
- 历史数据归档
- 数据完整性检查

#### 9.1.2 图表与可视化

- 中央主图
- 成交量区
- 十字光标
- 缩放与平移
- 叠加线条与标记
- 基础回放控制

#### 9.1.3 工作区管理

- 当前 symbol / timeframe 状态保存
- 画布视图状态恢复
- 侧边面板状态保存
- 最近打开对象记录

#### 9.1.4 任务管理

- 数据刷新任务
- 导入任务
- 导出任务
- 压缩/整理任务
- 任务状态、重试、取消、错误详情

#### 9.1.5 日志与诊断

- 运行日志
- 任务日志
- 用户提示
- 错误堆栈与上下文
- 基础健康检查

### 9.2 企业增强功能域（后续）

- 配置模板与策略模板下发
- 企业证书/许可证机制
- 受控 provider 接入
- 审计日志导出
- 集中升级与灰度发布
- 安装检测与环境自检

---

## 10. UI 规划

### 10.1 标准界面布局

```text
┌──────────────────────────────────────────────────────────────┐
│ TopBar: Mode | Symbol | Timeframe | Refresh | Health        │
├───────────────┬─────────────────────────────┬───────────────┤
│ Left Sidebar  │ Center Canvas               │ Right Dock    │
│ Watchlist     │ Bevy Chart Surface          │ Data Explorer │
│ Favorites     │ Main Chart / Volume         │ Governance    │
│ Quick Nav     │ Overlay / Replay            │               │
├───────────────┴─────────────────────────────┴───────────────┤
│ Bottom Dock: Jobs | Logs | Notifications                     │
└──────────────────────────────────────────────────────────────┘
```

### 10.2 UI 原则

- 中央画布占比优先。
- 右侧信息密度高但流程少。
- 底部只保留持续性信息：任务、日志、通知。
- 减少页面跳转，强调工作台式交互。
- 用户状态尽可能自动恢复。

---

## 11. 数据架构规划

### 11.1 存储职责划分

#### PostgreSQL 表建议分类

- `symbols`
- `datasets`
- `dataset_sync_state`
- `workspace_snapshots`
- `job_runs`
- `job_events`
- `parquet_partitions`
- `refresh_watermarks`
- `app_settings`
- `error_records`

#### Parquet 分区建议

建议按以下维度设计：

- provider
- market / exchange
- symbol
- timeframe
- year / month 或日期区间

示例：

```text
{archive_root}/{provider}/{exchange}/{symbol}/{timeframe}/year=2026/month=03/part-00001.parquet
```

### 11.2 数据读取策略

1. 优先查询 PostgreSQL 中的热窗口。  
2. 热窗口不足时，根据 manifest 定位 Parquet 分区补齐。  
3. 必要时再走远端 HTTP 拉取并回写。  
4. 对 UI 暴露统一 `MarketRepository`，不暴露底层存储细节。  

### 11.3 数据写入策略

- 远端拉取后的增量数据先进入 PostgreSQL 热窗口。
- 满足归档条件时异步写入 Parquet 新分区。
- 写入采用临时文件 + flush + rename 方式。
- Manifest 记录以 PostgreSQL 为准，避免仅通过文件系统推断可见性。

---

## 12. 应用与状态管理规划

### 12.1 状态分层

- **UI State**：面板开关、焦点、临时交互状态
- **Workspace State**：symbol、timeframe、视图范围、最近操作上下文
- **Domain State**：当前数据集、任务状态、同步状态
- **Render State**：送入 Bevy 的可视化快照

状态归属补充：
- **业务真状态**（归 `core/domain`）：项目、策略参数、数据源配置、任务状态、回测条件、工作区持久化布局
- **渲染派生状态**（归 `renderer-bevy`）：相机、GPU 高亮、帧统计、picking/hover、纹理句柄
- 禁止将业务真状态挂入 ECS Resource 作为权威来源

### 12.2 状态演进模型

采用：

- `Command`
- `Reducer`
- `Snapshot`
- `Event/Notification`

的标准方式，避免直接在各模块间共享可变状态。

### 12.3 任务模型

任务统一纳入 `jobs-runtime`，每类任务至少具备：

- 唯一任务 ID
- 任务类型
- 状态（Pending / Running / Success / Failed / Cancelled）
- 发起时间、结束时间
- 错误原因
- 可重试性
- 可取消性

### 12.4 输入路由与冲突裁决

- 输入路由权归 `app-shell`，不归 `ui-workbench` 或 `renderer-bevy`。
- 当焦点位于渲染面板时，滚轮/拖拽等输入优先路由到渲染交互。
- 当焦点位于表单或列表时，输入必须保留在 UI 层处理。
- 输入冲突处理结果应作为事件进入状态演进链路，禁止旁路改写状态。

---

## 13. 模块化与插件化规划

### 13.1 原则

本项目支持“模块化扩展”，但 **Phase 1 不支持运行时 DLL 热插拔**。企业版规划中，插件化应理解为：

- trait 接口边界明确
- 模块可替换
- 编译期注册或静态注册
- 后续再升级为受控扩展体系

### 13.2 可扩展点

- `MarketDataProvider`
- `CatalogStore`
- `ArchiveStore`
- `IndicatorProvider`
- `ExportProvider`
- `ReplaySource`
- `JobHandler`

### 13.3 企业版受控扩展路线

后续若确需扩展市场化或企业化插件体系，建议单独建设：

- 插件签名规范
- 版本兼容矩阵
- 沙箱与权限边界
- 加载白名单
- 崩溃隔离与审计机制

当前阶段不将其作为核心交付项。

---

## 14. 非功能需求规划

### 14.1 性能

- 冷启动时间可控。
- UI 基本交互无明显阻塞。
- 中央画布在常规研究数据量下保持流畅缩放和平移。
- 热窗口查询与刷新延迟可接受。

### 14.2 可靠性

- 异常退出后可恢复 workspace。
- 任务失败不导致应用整体不可用。
- Parquet 写入失败可检测、可回滚、可重试。
- PostgreSQL 连接失败有明确诊断提示。

### 14.3 可运维性

- 本地日志目录标准化。
- 错误日志带上下文。
- 关键路径提供 health 信息。
- 提供环境自检结果与依赖检查。

### 14.4 可扩展性

- 新 provider 接入不破坏 UI 主体。
- 新指标、新图层、新任务可以按模块增加。
- 数据量增长后仍可通过热窗口 + 归档分层保持可管理性。

### 14.5 安全性

- 本地配置与连接信息不明文散落。
- 关键配置目录权限合理。
- 后续支持加密存储敏感配置。
- 企业版可接入证书、签名与升级校验机制。

---

## 15. Windows 平台专项规划

### 15.1 运行环境标准

- Windows 为主要目标平台。
- PostgreSQL 建议使用标准本地安装实例或企业预装实例。
- 使用 `localhost TCP` 连接数据库。
- 数据、日志、临时文件分别落到 `%APPDATA%` / `%LOCALAPPDATA%` 体系。

### 15.2 文件目录建议

- `%APPDATA%\zquant\config`
- `%LOCALAPPDATA%\zquant\logs`
- `%LOCALAPPDATA%\zquant\data\parquet`
- `%LOCALAPPDATA%\zquant\tmp`

### 15.3 打包与分发建议

- Phase 1：内部安装包或便携安装方案
- 企业版：标准 MSI/EXE 安装器、升级器、环境检查器
- 安装过程应检查 PostgreSQL 连接配置、目录权限、磁盘空间与写入能力

---

## 16. 研发实施规划

### 16.1 推荐工作分解结构（WBS）

#### WBS-1：基础工程搭建

- Rust workspace 规划
- 日志框架
- 配置框架
- 错误模型
- 目录规范

#### WBS-2：桌面壳与布局骨架

- eframe/egui 主壳
- TopBar / Sidebar / Right Dock / Bottom Dock
- 基本状态管理

#### WBS-3：Bevy 画布集成

- 离屏纹理渲染
- egui 中嵌入中心画布
- 基础图表交互

#### WBS-4：存储层建设

- PostgreSQL schema
- CatalogStore
- ArchiveStore
- MarketRepository

#### WBS-5：数据接入与任务层

- HTTP provider
- 刷新任务
- 导入导出任务
- 日志与通知

#### WBS-6：工作区与恢复

- workspace 保存/加载
- 最近状态恢复
- 面板布局恢复

#### WBS-7：测试与打包

- 单元测试
- 集成测试
- 基础性能测试
- Windows 安装与运行验证

### 16.2 里程碑建议

#### M1：架构与脚手架完成

验收标准：

- 工程目录、配置、日志、错误体系建立
- 基础窗口可启动
- 关键模块接口定义完成

#### M2：中心画布与基础 UI 完成

验收标准：

- 中央画布可显示模拟图表
- 布局骨架稳定
- workspace 基本状态可切换

#### M3：存储与数据流闭环完成

验收标准：

- 能拉取远端数据
- PostgreSQL 可写入元数据与热窗口
- Parquet 可完成归档写入与读取补齐

#### M4：MVP 验收

验收标准：

- 本地单机完整运行
- 图表浏览、刷新、日志、任务、workspace 恢复均可用
- Windows 安装与基本诊断通过

---

## 17. 测试与质量保障规划

### 17.1 测试策略

- 单元测试：领域模型、时间序列处理、路径逻辑、任务状态流转
- 集成测试：PostgreSQL / Parquet / Repository / Provider
- UI 冒烟测试：主窗口、基本交互、画布嵌入
- Windows 专项验证：路径、权限、写入、安装运行

### 17.2 质量门禁

- 关键模块必须通过 CI 构建
- 存储层与任务层必须具备集成测试
- 重大异常场景必须有可复现日志
- 每个里程碑有明确验收清单

---

## 18. 风险与应对

### 18.1 主要风险

1. **Bevy 与 egui 集成复杂度高于预期**  
2. **PostgreSQL 本地安装与用户环境差异导致部署复杂**  
3. **Parquet 分区设计不当导致后续读写效率差**  
4. **状态管理无边界导致并发写乱与 UI 混乱**  
5. **过早追求插件化或企业特性导致 Phase 1 延迟**  

### 18.2 应对策略

- 采用离屏纹理方案，固定渲染边界。
- PostgreSQL 先采用明确前置依赖方案，不一开始做全自动托管。
- Manifest 与 Archive 分离，先保守设计分区策略。
- 明确 Command/Reducer/Snapshot 模型。
- 企业能力分阶段推进，不在 MVP 前置全部实现。

---

## 19. 组织与职责建议

### 19.1 建议角色

- 产品/架构负责人
- Rust 客户端负责人
- 存储与数据负责人
- 图形渲染负责人
- 测试与发布负责人

### 19.2 职责边界

- 产品/架构：范围控制、里程碑、标准收敛
- 客户端：egui 主壳、状态流、任务流、交互体验
- 渲染：Bevy 画布、图元、交互响应
- 存储：PostgreSQL schema、Parquet 分区、Repository
- 测试发布：安装包、环境验证、测试报告、回归控制

---

## 20. 结论与立项建议

本项目适合按照“企业标准、分阶段交付”的方式推进，而不适合以试验性 Demo 的方式反复重做。当前最优路径不是继续在 Web 控制台思路上迭代，而是正式确立以下企业级基线：

- **产品基线**：本地研究工作台，而非浏览器后台。
- **GUI 基线**：`egui` 主壳 + `Bevy renderer` 中央可视化。
- **存储基线**：`PostgreSQL` 主控面 + `Parquet` 主归档面。
- **平台基线**：Windows 优先。
- **实施基线**：Phase 1 先完成单机闭环，Phase 2/3 再进入企业增强。

建议本方案作为当前阶段的正式规划底稿，用于后续输出：

1. 详细技术设计说明书  
2. Rust workspace 与 crate 划分方案  
3. PostgreSQL 表结构设计书  
4. Parquet 分区与归档设计书  
5. Phase 1 开发排期与任务分解表  

---

## 21. 附录：建议的 crate 划分方向

```text
zquant-workspace/
├─ apps/
│  └─ desktop-app
├─ crates/
│  ├─ app-shell
│  ├─ ui-workbench
│  ├─ renderer-bevy
│  ├─ domain-market
│  ├─ domain-workspace
│  ├─ application-core
│  ├─ jobs-runtime
│  ├─ infra-http-provider
│  ├─ infra-postgres
│  ├─ infra-parquet
│  ├─ infra-storage
│  ├─ infra-config
│  └─ infra-logging
└─ docs/
```

该划分仅作为架构方向，不作为最终冻结版本，后续应在详细设计阶段进一步收敛。

建议依赖方向（同一口径）：

```text
ui_workbench(egui) -\
                     -> app_shell -> core/domain -> infra_*
renderer_bevy ------/
```
