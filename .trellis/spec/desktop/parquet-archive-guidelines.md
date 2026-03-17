# Parquet 归档规范

## 目标

建立 PostgreSQL（控制面）+ Parquet（归档面）的分层存储闭环，支撑历史数据扫描与回放。

## 职责划分

- PostgreSQL：元数据、同步水位、任务状态、partition manifest。
- Parquet：历史分区数据本体、批量扫描输入、导入导出载体。

## 分区建议

推荐路径维度（按需裁剪）：

- provider
- market/exchange
- symbol
- timeframe
- year/month（或日期范围）

示例：

`{archive_root}/{provider}/{exchange}/{symbol}/{timeframe}/year=2026/month=03/part-00001.parquet`

## 写入一致性

- 使用 `tmp -> flush -> rename` 原子流程。
- 成功写入后再更新 manifest，manifest 以 PostgreSQL 记录为准。
- 禁止仅依赖文件系统扫描判断“数据可见”。

## 读取策略

1. 优先读取 PostgreSQL 热窗口。  
2. 热窗口不足时依据 manifest 定位 Parquet 分区补齐。  
3. 必要时再拉取远端并回写。  
4. 统一从 `MarketRepository` 对上暴露，不透出底层细节。  

## 失败与恢复

- 写入失败必须可检测并保留诊断上下文（分区、symbol、timeframe、批次）。
- 中断后允许基于 manifest 做幂等重试。
- 发现坏分区时要支持隔离与重建，不影响全局可用性。

## 验收要点

- 新分区可被正确登记并查询。
- manifest 与文件系统状态保持一致。
- 归档失败不会导致应用全局不可用。
