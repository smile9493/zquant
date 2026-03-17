# 后端规范索引

> 适用于 Rust 应用层、任务运行时、数据访问与契约稳定性。

---

## 范围说明

本项目“后端”指 Rust 服务层与数据访问层，不要求必须以独立网络服务形态部署。  
在桌面演进路线中，后端能力可被桌面进程内复用。

---

## 规范清单

| 规范 | 说明 | 状态 |
|------|------|------|
| [目录结构](./directory-structure.md) | 工程与模块放置规范 | 已建立 |
| [数据库规范](./database-guidelines.md) | SQLx、迁移、查询与事务约束 | 已建立 |
| [错误处理](./error-handling.md) | 错误分类、映射、重试策略 | 已建立 |
| [类型安全](./type-safety.md) | DTO/Domain 边界、类型建模 | 已建立 |
| [日志规范](./logging-guidelines.md) | tracing 字段与日志级别 | 已建立 |
| [质量规范](./quality-guidelines.md) | 测试与质量门禁 | 已建立 |
| [Rust 编码规范](./rust-coding-guidelines.md) | Rust 语言与并发实践 | 已建立 |

## 契约规范

| 规范 | 说明 | 状态 |
|------|------|------|
| [Data Pipeline Contracts](./data-pipeline-contracts.md) | Provider/DQ/Event/Persist 契约 | 冻结 |
| [AkShare Dataset Contracts](./akshare-dataset-contracts.md) | `cn_equity.ohlcv.daily` 契约 | 冻结 |

---

## 与桌面规范关系

- 后端规范只约束 Rust 后端能力，不覆盖桌面工作台交互与渲染细节。
- 桌面相关规范请阅读：`../desktop/index.md`。
