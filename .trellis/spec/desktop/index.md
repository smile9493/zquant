# Desktop 开发规范索引

> 适用于 zquant 企业版目标形态：Windows 本地研究工作台（`egui + Bevy + PostgreSQL + Parquet`）。

---

## 目标

本目录用于约束桌面侧实现，补齐目前仅有后端规范的缺口，确保演进阶段（M1~M4）有统一标准可执行。

---

## 规范清单

| 规范 | 说明 | 状态 |
|------|------|------|
| [架构硬边界](./architecture-boundaries.md) | `egui-first` + `Bevy runtime` 的不可违反分层规则 | 已建立 |
| [App Shell 规范](./app-shell-guidelines.md) | `egui` 主壳、布局与交互边界 | 已建立 |
| [Bevy 渲染集成规范](./renderer-bevy-guidelines.md) | 中心画布、离屏渲染与状态同步 | 已建立 |
| [Workspace 状态规范](./workspace-state-guidelines.md) | 命令/Reducer/Snapshot 与恢复策略 | 已建立 |
| [Parquet 归档规范](./parquet-archive-guidelines.md) | 分区、manifest、读写一致性 | 已建立 |
| [Windows 运行规范](./windows-runtime-guidelines.md) | 目录、权限、自检、安装与诊断 | 已建立 |

---

## 预开发检查（Desktop）

开始任何桌面开发前至少完成：

- [ ] 阅读本索引与 [架构硬边界](./architecture-boundaries.md)
- [ ] 明确本次变更属于哪个阶段（M1/M2/M3/M4）
- [ ] 明确是否影响跨层契约（UI ↔ Application ↔ Storage ↔ Render）
- [ ] 若涉及存储或接口变更，补充到任务 PRD 的“契约与回滚”部分

---

## 范围边界

- 本目录关注桌面工作台形态，不替代后端通用编码规范。
- Rust 通用规则仍以 `../backend/*.md` 为准。
- 跨层设计仍需配合 `../guides/cross-layer-thinking-guide.md`。
