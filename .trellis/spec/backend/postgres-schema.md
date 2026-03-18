# PostgreSQL Schema 规范

PostgreSQL 是唯一关系型真相源。

规则：

1. 表名使用 `snake_case`，统一复数名词。
2. 主键默认 `bigint` 或 `uuid`，同一子域保持一致。
3. 所有业务表必须包含：
   - `created_at`
   - `updated_at`
4. 外键命名为 `{target_singular}_id`。
5. 枚举优先用受控文本 + CHECK，除非 PostgreSQL enum 有明显收益。
6. JSONB 只用于半结构扩展字段，不作为核心关系替代。
7. schema 设计必须先定义唯一约束与外键，再考虑索引。

索引规则：

- 默认从 B-tree 开始。
- 只有出现明确查询场景时才引入 GIN/GiST/BRIN。
- 新增索引时必须说明：
  - 支持哪条查询
  - 预期过滤条件
  - 写放大成本
- 可以使用表达式索引或部分索引，但必须写清适用 SQL。
