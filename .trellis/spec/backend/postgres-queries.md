# PostgreSQL 查询规范

依据 PostgreSQL 官方文档，事务是 all-or-nothing。
查询和写入必须先定义事务边界，再写实现。

规则：

1. 单个业务动作的多步写入必须放进一个事务。
2. 不允许在同一业务操作中混用"部分事务化"写法。
3. Repository 层返回领域对象或专用 DTO，不直接向上暴露生 SQL 结果。
4. 查询必须参数化，禁止字符串拼接 SQL。
5. 所有列表查询默认要有：
   - limit
   - 明确排序
   - 可解释的过滤条件
6. 复杂查询提交前至少跑一次 `EXPLAIN` / `EXPLAIN ANALYZE`。
