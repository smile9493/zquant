# P1-8 Provider 复用抽象

## 背景来源
- 来源任务：`A:\zquant\.trellis\tasks\archive\2026-03\03-19-03-19-findings-remediation-execution\prd.md`
- 对应阶段：P1-8「Provider 复用抽象」。

## 目标
- 收敛 `AkShare` / `PyTDX` 的重复模板逻辑，建立共享 provider 抽象层。
- 在不改变现有外部行为的前提下降低重复代码与维护成本。

## 范围
- `A:\zquant\crates\data-pipeline-application`（provider 路由/调用编排）
- `A:\zquant\crates\data-pipeline-domain`（必要的契约扩展）
- `A:\zquant\crates\data-pipeline-application\tests`（回归与新增测试）

## 非目标
- 不新增 provider 类型。
- 不调整数据质量判定规则（DqDecision / DqIssue）。
- 不变更现有 UI 或任务调度层交互协议。

## 验收标准
1. 提取共享 provider 抽象（策略/模板）并替换现有重复路径。
2. `AkShare` 与 `PyTDX` 现有能力/错误语义保持不变（行为等价）。
3. 数据集 ID 校验、参数校验、错误透传等关键路径有回归测试覆盖。
4. 至少新增 2 个“复用抽象层”单测（模板流程 + 差异化分支）。
5. `cargo check -p data-pipeline-application -p data-pipeline-domain` 通过。
6. `cargo test -p data-pipeline-application -p data-pipeline-domain` 通过。
7. `cargo check --workspace` 通过。

## 假设与风险
- 假设两类 provider 的共性流程可抽象为“参数校验 -> 拉取执行 -> 结果标准化 -> 错误映射”。
- 风险：过度抽象导致差异化逻辑被掩盖。
  - 应对：将差异点显式留在策略接口，不强行统一。
- 风险：改造后行为漂移。
  - 应对：先补回归测试再迁移实现。

## 实现计划
1. 盘点 `AkShare/PyTDX` 重复段与差异段，形成抽象边界。
2. 引入共享抽象层（trait/模板函数/策略对象）。
3. 迁移两个 provider 到新抽象，并保持现有对外接口不变。
4. 补充回归与新增测试，覆盖关键分支。
5. 执行 review gate 并回写 PRD 结果。

## Checklist
- [x] 新建 Trellis 任务并设为当前任务
- [x] 写入 PRD（目标/范围/非目标/验收/风险/计划）
- [x] 盘点并拆分 provider 共性/差异逻辑
- [x] 实现共享抽象并迁移 AkShare/PyTDX
- [x] 补齐回归与新增测试
- [x] 执行 review gate 并输出 REVIEW 结论

## Review Gate 结果

**REVIEW: PASS**

### 验证执行
- `cargo check -p data-pipeline-application -p data-pipeline-domain` — 通过
- `cargo test -p data-pipeline-application -p data-pipeline-domain` — 37+4=41 全通过
- `cargo check --workspace` — 通过

### 验收标准满足情况
1. ✅ 提取 `PythonDatasetConfig` trait + `python_fetch_dataset` 共享模板
2. ✅ AkShare/PyTDX 行为等价（34 项原有回归测试通过）
3. ✅ 关键路径回归测试覆盖
4. ✅ 新增 3 个抽象层测试（模板一致性 / 差异化分支 / 共享校验）
5. ✅ cargo check 两包通过
6. ✅ cargo test 两包通过
7. ✅ cargo check --workspace 通过

### 规范合规
- 无运行时 unwrap/expect
- 抽象层 pub(crate) 可见性最小化
- 公共 API 签名不变
- 差异点显式留在策略接口
