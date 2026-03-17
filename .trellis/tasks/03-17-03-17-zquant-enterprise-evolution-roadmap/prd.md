# zquant 企业版演进方案（后端底座 → 桌面工作台）

## Goal

基于 `A:\zquant\docs\web\zquant_企业版标准规划方案.md`，在不推翻现有 Rust 后端资产的前提下，制定一套可执行的演进路线：将当前“任务服务化后端底座”逐步演进为“Windows 本地研究工作台（egui + Bevy + PostgreSQL + Parquet）”。

## Scope

### In scope

- 对照规划文档与当前仓库结构，给出演进目标架构与落地路径。
- 明确“复用模块 / 新增模块 / 调整模块”的边界。
- 给出分阶段里程碑（M1~M4）及每阶段交付项、验收标准。
- 给出关键技术决策与迁移顺序（API 调用、任务运行时、存储分层、UI 集成）。
- 给出风险清单、缓解策略与回滚思路。

### Out of scope

- 本任务不实现具体代码改造。
- 不进行数据库 schema 变更或线上数据迁移实操。
- 不设计完整企业协同（SSO/审计中心/分发平台）细节实现。

## Non-Goals

- 不追求一次性从服务形态切到完整桌面形态。
- 不强制废弃现有 `job-api`/`job-runner` 等可复用服务。
- 不在当前任务内完成 Bevy 渲染引擎和 Parquet 全量能力实现。

## Current State (As-Is)

- 工作区为 Rust workspace：`apps/*` + `crates/*`。
- 现有能力重心：
  - 任务与执行链路：`job-api` / `job-runner` / `job-kernel` / `job-cache-consumer` / `job-ws-bridge`
  - 领域与应用：`job-domain` / `job-application`
  - 基础设施：`job-store-pg` / `job-cache-redis` / `job-events` / `data-pipeline-*`
- 现有基础设施偏服务化：PostgreSQL + Redis + Kafka + HTTP。

## Target State (To-Be)

- 形态：Windows 本地桌面优先（单机闭环）。
- 前台：`egui` 承担应用壳与工作台布局。
- 可视化：`Bevy renderer` 承担中心高频图形渲染。
- 存储：PostgreSQL（控制面/状态面）+ Parquet（归档面/批量扫描）。
- 执行：保留现有任务与数据管线内核，优先内嵌到桌面进程，保留服务化接口作为扩展路径。

## Gap Analysis (主要差距)

1. **产品层差距**：缺少桌面 App Shell 与工作台交互。
2. **渲染层差距**：缺少中心画布与 Bevy 集成链路。
3. **存储层差距**：缺少 Parquet 归档与 manifest 驱动读取。
4. **调用模型差距**：当前以 HTTP/服务调用为主，需补齐进程内应用服务调用。
5. **Workspace 差距**：缺少桌面级状态快照与恢复机制。

## Implementation Plan

### Phase M1：架构收敛与壳层落地

- 新增 `apps/desktop-app`（空壳可启动）。
- 新增 `crates/app-shell`、`crates/ui-workbench`（TopBar/SideBar/BottomDock 框架）。
- 将现有 `job-application` 能力通过 facade 暴露给桌面（先 mock/本地调用）。
- 形成桌面内命令总线与基础状态容器（Command/Reducer/Snapshot 雏形）。

### Phase M2：可视化链路打通

- 新增 `crates/renderer-bevy`，完成离屏纹理渲染到 egui。
- 中心画布支持基础 K 线/缩放/平移（可用样例数据）。
- 建立 UI 状态与 Render 状态同步机制。

### Phase M3：数据分层闭环（PostgreSQL + Parquet）

- 新增 `crates/infra-parquet` 与 `crates/infra-storage`（如需聚合层）。
- 定义 partition manifest（PostgreSQL）与归档写入流程（tmp+flush+rename）。
- `MarketRepository` 实现热窗口优先、归档补齐、远端拉取回写策略。

### Phase M4：MVP 验收与发布准备

- 完成 workspace 快照恢复、任务面板、日志面板、错误提示链路。
- 完成 Windows 目录规范（config/logs/data/tmp）与自检能力。
- 完成最小安装/运行验证与验收报告。

## Acceptance Criteria

- [ ] 形成可执行演进蓝图：包含目标架构、分阶段计划、风险与验证策略。
- [ ] 明确现有模块的去留与复用策略，并映射到目标模块划分。
- [ ] 给出 M1~M4 每阶段可验收的交付物与完成定义（DoD）。
- [ ] 给出“桌面内调用优先 + 服务接口保留”的双轨策略说明。
- [ ] 给出 Parquet 引入时机、目录规范、读写一致性策略。

## Assumptions / Risks

### Assumptions

- 现有 `job-*` 与 `data-pipeline-*` 资产质量可支撑复用。
- 近期目标优先“本地单机闭环”，不强依赖企业协同平台。
- Windows 环境可提供可用 PostgreSQL 本地实例。

### Risks

- Bevy + egui 集成复杂度可能影响 M2 节奏。
- Parquet 分区设计若不稳，会影响后续回放与查询性能。
- 若前期仍过度依赖 HTTP 服务调用，会增加桌面端时延与复杂度。
- 现有仓库未引入桌面相关依赖，首轮基础设施调整成本较高。

## Mitigation

- 先做离屏渲染最小闭环，冻结集成边界。
- 先从日级/固定维度分区起步，保守扩展分区策略。
- 桌面端优先引入进程内 Facade，HTTP 作为兼容层。
- 每阶段设置“可运行演示”与“回退路径”（保持原服务能力可独立运行）。

## Checklist

- [ ] 梳理 As-Is / To-Be 架构对照图（文档级）。
- [ ] 完成模块映射表（现有 -> 目标）。
- [ ] 明确 M1~M4 的交付物、依赖、验收标准。
- [ ] 明确技术决策：调用模型、状态模型、存储模型。
- [ ] 输出风险矩阵与应对策略。
- [ ] 与团队确认演进节奏与优先级。

