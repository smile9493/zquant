# 第一版确立方案（V1）

## 1. 方案定位

**项目定位：Rust 原生量化研究编排平台**

不是 Qlib 的 UI 外壳，也不是单纯回测工具。  
目标是建立一套可扩展的：

- 研究规格化
- 研究编排化
- 研究执行化
- 研究资产化

体系。

Qlib 在本方案中的角色是：

- **参考架构**
- **对照基线**
- **过渡期执行适配器**

不是最终生产内核。

---

## 2. 总体架构

整体采用 6 层架构：

### 2.1 Workbench Shell
- `egui-first` 的桌面工作台
- 承载项目树、Spec 编辑器、无限画布、DAG 编排、运行队列、结果面板、日志与 Artifact 浏览器

### 2.2 Research Orchestrator
- 系统中枢
- 负责 `Spec -> Plan(DAG) -> Run`
- 管理依赖、调度、失败恢复、重试、断点续跑

### 2.3 Domain Core
- 纯 Rust 业务内核
- 不依赖 `egui / Bevy / Python`
- 保存唯一业务真相

### 2.4 Execution Engines
- 可插拔执行器层
- 分为：
  - `DataEngine`
  - `FactorEngine`
  - `ModelEngine`
  - `StrategyBacktestEngine`

### 2.5 Data Plane
- 数据接入、标准化、分区、列式查询、时间对齐、缓存、版本化

### 2.6 Artifact & Recorder System
- 负责 `params / metrics / artifacts / lineage / logs` 的统一记录
- 对齐 Qlib 的 `ExperimentManager -> Experiment -> Recorder` 结构，但做 Rust 原生实现

---

## 3. 核心对象

第一版固定以下核心对象：

- `ResearchContext`
- `ResearchSpec`
- `TaskTemplate`
- `ResearchPlan`
- `PlanNode`
- `ResearchRun`
- `ArtifactRef`
- `MetricSet`
- `LineageGraph`
- `RecorderEntry`

### 3.1 ResearchContext
- `provider`
- `region`
- `market`
- `calendar`
- `execution_profile`

### 3.2 ResearchSpec
- `dataset_spec`
- `factor_spec`
- `model_spec`
- `strategy_spec`
- `backtest_spec`
- `analysis_spec`

### 3.3 TaskTemplate
对齐 Qlib task 结构，第一版固定为：

- `Model`
- `Dataset`
- `Record`

### 3.4 ResearchRun
- `run_id`
- `spec_hash`
- `plan_hash`
- `state`
- `artifacts`
- `metrics`
- `lineage`

---

## 4. 固定流水线

V1 固定研究流水线为 8 类节点：

- `prepare_data`
- `build_dataset`
- `compute_factor`
- `train_model`
- `infer_signal`
- `construct_portfolio`
- `run_backtest`
- `analyze_publish`

---

## 5. 任务编排模型

第一版研究编排采用“两级模型”。

### 5.1 Level 1：单任务运行
- 一个 `TaskTemplate`
- 一次 `ResearchRun`
- 对应单条研究流水线

### 5.2 Level 2：任务族编排
- `TaskGenerator` 基于模板生成多个任务
- `TaskManager` 负责存储、排队、状态管理
- `Collector` 负责汇总结果

---

## 6. 执行边界

第一版明确采用“接口先行，执行器可替换”的策略。

统一定义四类引擎接口：

- `DataEngine`
- `FactorEngine`
- `ModelEngine`
- `StrategyBacktestEngine`

每类接口允许两种实现：

- `PythonAdapter/*`
- `NativeRust/*`

### V1 原则
- 编排层、记录层、Artifact 层必须 Rust 原生
- 执行层允许过渡期兼容 Python/Qlib
- 最终目标是 `NativeRust` 替换执行器，不替换对象模型

---

## 7. Artifact 体系

第一版把“结果”定义为 Artifact，而不是内存对象。

固定 8 类 Artifact：

- `dataset_artifact`
- `factor_artifact`
- `model_artifact`
- `signal_artifact`
- `portfolio_artifact`
- `backtest_artifact`
- `analysis_artifact`
- `report_artifact`

完成标准不是“函数跑完”，而是：

- 节点产物落盘
- Recorder 建索引
- Run 绑定 lineage
- Workspace 可回放 / 可比较 / 可追踪

---

## 8. 状态机

统一 `ResearchRunState`：

- `Draft`
- `Planned`
- `Queued`
- `Running`
- `Succeeded`
- `Failed`
- `Canceled`
- `Recovered`

任务级状态机由项目自行实现，但必须支持：

- 排队
- 运行
- 完成
- 失败
- 恢复

---

## 9. UI 与编排关系

工作台只是编排壳，不是研究真相层。

`egui` 主壳固定负责：

- Spec 编辑
- DAG 可视化
- Run 队列
- Artifact 浏览
- 结果面板
- 日志与告警

无限画布只是 `ResearchPlan(DAG)` 的可视化表达，不反向决定核心对象模型。

---

## 10. 第一版约束

V1 固定 6 条：

1. 业务真相只存在于 `Domain Core`
2. 一切研究过程必须落到 `Spec / Plan / Run / Artifact`
3. Qlib 只作参考架构与对照基线，不作最终生产内核
4. 编排层、记录层、Artifact 层必须 Rust 原生
5. UI 不直接持有研究真状态
6. DAG 只是执行图，不是业务模型本身

---

## 11. 第一版范围

V1 只确立以下内容：

- 统一对象模型
- 统一研究流水线
- 统一编排边界
- 统一 Artifact 体系
- 统一 Recorder 体系
- 统一执行器接口
- 统一 UI 编排关系

### V1 不解决
- 原生模型训练细节
- 高频撮合细节
- GPU 加速方案
- 分布式调度实现
- 最终数据存储实现细节

---

## 12. 最终表述

**第一版确立方案：**

本项目以 Microsoft Qlib 的研究对象边界与编排链路为参考，建立 Rust 原生的量化研究编排平台。系统采用 `Workbench Shell + Research Orchestrator + Domain Core + Execution Engines + Data Plane + Artifact/Recorder` 六层架构；以 `ResearchContext / ResearchSpec / TaskTemplate / ResearchPlan / ResearchRun / ArtifactRef` 为核心对象；以 `prepare_data -> build_dataset -> compute_factor -> train_model -> infer_signal -> construct_portfolio -> run_backtest -> analyze_publish` 为固定研究流水线；以 `TaskGenerator / TaskManager / Collector` 作为任务族编排骨架；Python/Qlib 仅作为过渡期执行适配器与结果对照基线，不作为最终生产内核。