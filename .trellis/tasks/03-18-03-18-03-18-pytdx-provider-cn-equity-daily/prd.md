# PyTDX Provider Plugin: CN equity daily (SH/SZ/BJ)

## Goal

在不把数据源 SDK 绑定进 Rust 代码的前提下（Provider 与项目解耦、可替换），为统一数据管道新增一个 **PyTDX** 数据源插件，实现 **A 股三大市场（沪/深/北交所）历史日线 OHLCV** 的可拉取能力，并保持现有 `DataPipelineManager → normalize → DQ → persist → emit` 闭环与契约稳定。

## Background / Rationale

统一数据管道骨架要求：调用方只声明需求（capability/market/dataset），不得直接依赖具体 Provider；Provider 可替换；主链路统一完成标准化、DQ、持久化与事件发射闭环。

当前项目已有模式：

- Rust 侧 Provider 仅负责 **路由与调用**，通过 `PythonRunner(Subprocess)` 执行 Python 脚本获取数据；
- Python 侧脚本负责依赖第三方包并输出符合冻结契约的 JSON（`{"status":"success","data":[...]}`），Rust 侧只解析 JSON。

本任务沿用该模式，将 PyTDX 接入作为“插件（Python 脚本）”，以最小侵入方式推进数据源能力。

## Scope

- 新增 `pytdx` Provider（Rust 侧适配器 + Python 脚本）。
- 支持数据集：
  - `dataset_id = "cn_equity.ohlcv.daily"`
  - `capability = ohlcv`
  - `market = cn_equity`
- 目标覆盖：
  - 沪市（SH）
  - 深市（SZ）
  - 北交所（BJ）
- 维持现有 fail-closed 语义：
  - 调用方指定 `forced_provider = "pytdx"` 时，若不可用/不支持则必须失败，不得静默切换。

## Non-Goals

- 不实现复权、分红送配、停牌补齐、交易日历对齐等高级处理（后续任务）。
- 不把 PyTDX 作为 Rust 依赖（不在 Cargo.toml 引入 pytdx 相关）。
- 不引入 Redis/Kafka/分布式调度等非本阶段基础设施。
- 不承诺真实公网环境可用性（可做“手工 smoke test 指南”），CI/测试必须保持 hermetic。

## Key Design Decisions

### Provider 解耦方式（插件化）

- Rust：仅实现 `PytdxProvider`，通过 `PythonRunner::run_json(script, input)` 调用脚本。
- Python：脚本内部 `import pytdx...`，并负责连接、分页拉取、字段整理、JSON 输出。
- 脚本路径策略：
  - 默认放在 `crates/data-pipeline-application/python/`（与现有 AkShare 一致）；
  - **允许通过环境变量覆盖脚本路径**（为未来“外置插件目录”预留扩展点）。

### 三市场实现策略（以可落地为第一优先）

PyTDX 官方 `hq` 文档仅明确 market=0/1（深/沪）。北交所在 PyTDX 生态中存在两种可能：

1) 北交所股票被并入 `hq` 的深圳 market（以代码前缀区分）；
2) 北交所/新三板相关标的需要走 `exhq` 的“股份转让(SB)”市场。

本任务采取 **“先 hq 探测 + 明确降级/报错”** 的工程策略：

- 规则优先级：
  1. 若代码明显为沪市（如 `6xxxxx` / `5xxxxx` 等）→ `hq market=1`
  2. 若代码明显为深市（如 `0xxxxx` / `3xxxxx` 等）→ `hq market=0`
  3. 若代码疑似北交所（常见前缀如 `43/83/87/88/92` 等）→ **优先尝试 `hq market=0`**；若返回空/None，再尝试 `exhq(SB)`；仍失败则返回清晰错误（fail-closed）
- 这保证：在缺乏官方“BJ market code”定义情况下，我们不会静默产生错误数据。

> 注意：北交所最终归属（hq vs exhq）需要一次真实环境探测确认；该探测不会进入自动化测试（保持 hermetic），但会记录在 PRD 的“Manual Validation”。

## Interfaces / Contracts

### DatasetRequest（Rust → Python）

通过 stdin JSON 传入（与 AkShare 对齐）：

```json
{
  "symbol": "000001",
  "start_date": "20240101",
  "end_date": "20240301"
}
```

### Python stdout（Python → Rust）

成功：

```json
{ "status": "success", "data": [ { "date": "2024-01-02", "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0, "volume": 123.0 } ] }
```

失败（stdout 输出 error JSON，Rust runner 会优先解析 message）：

```json
{ "status": "error", "message": "..." }
```

## Acceptance Criteria

- [ ] `PytdxProvider` 可被注册到 `ProviderRegistry`，并参与路由选择。
- [ ] 当 `dataset_id="cn_equity.ohlcv.daily"` 且 `forced_provider="pytdx"` 时：
  - [ ] SH/SZ 标的返回非空 records，records 至少包含 `date/open/high/low/close/volume` 字段。
  - [ ] BJ 标的：若能成功拉取则返回同样字段；若不能拉取则返回明确错误（不得返回空成功）。
- [ ] Python 脚本输出 JSON 可被 Rust 解析（包含 date 字符串化，避免 `date is not JSON serializable`）。
- [ ] 新增的测试保持 hermetic（不得依赖真实 TDX 服务器与公网）。
- [ ] `cargo test -p data-pipeline-application` 通过；`cargo clippy -p data-pipeline-application -- -D warnings` 通过。

## Implementation Plan (Checklist)

### Phase 0: Repo Hygiene (prereq)

- [ ] 明确当前工作区存在的未提交改动属于哪个任务；必要时先完成/提交/归档，避免与本任务混杂。

### Phase 1: Provider adapter (Rust)

- [ ] 新增 `crates/data-pipeline-application/src/providers/pytdx.rs`
  - [ ] `provider_name="pytdx"`
  - [ ] 支持 `Capability::Ohlcv`、`Market::CnEquity`
  - [ ] `fetch_dataset()` 仅支持 `dataset_id="cn_equity.ohlcv.daily"`
  - [ ] 构造 Python 输入（symbol + 可选 start/end）
  - [ ] 脚本路径可通过 env 覆盖（如 `ZQUANT_PYTDX_SCRIPT_CN_EQUITY_DAILY`）
- [ ] 更新 `crates/data-pipeline-application/src/providers/mod.rs` 导出 Provider。
- [ ] 在应用装配处注册 `PytdxProvider`（与 AkShare 并存；后续可调整优先级策略）。

### Phase 2: Python plugin (PyTDX)

- [ ] 新增 `crates/data-pipeline-application/python/pytdx_cn_equity_ohlcv_daily.py`
  - [ ] 解析 stdin JSON（symbol/start_date/end_date）
  - [ ] 统一连接策略（支持 host 列表轮询连接；失败输出 error JSON）
  - [ ] 拉取日线：
    - [ ] `hq.get_security_bars(category=9, market=0/1, ...)` 分页（count<=800）
    - [ ] 对 BJ：按“hq 探测 → exhq(SB) 尝试 → 明确失败”策略
  - [ ] 标准化字段：输出 `date/open/high/low/close/volume`（可额外包含 amount 等，但不得破坏既有 DQ）
  - [ ] `date` 字段确保为字符串（ISO 或 YYYY-MM-DD）

### Phase 3: Hermetic tests

- [ ] 为 Rust 侧新增测试：
  - [ ] provider 输入构造正确（symbol/time_range）
  - [ ] forced_provider fail-closed 正确
  - [ ] PythonRunner error message 透传正确（复用已有 runner 语义）
- [ ] 为 Python 侧新增 hermetic 测试脚本（不依赖 pytdx）：
  - [ ] 模拟返回结构（含 date 类型/中文字段等边界），验证输出 JSON 契约与 date 字符串化策略

### Phase 4: Manual validation (non-hermetic)

- [ ] 提供一段可复制的手工验证步骤（在本机有 pytdx 环境时执行）：
  - [ ] SH: 600000
  - [ ] SZ: 000001
  - [ ] BJ: 83xxxx / 87xxxx / 88xxxx / 43xxxx / 92xxxx（选择一个真实存在的代码）
  - [ ] 记录 BJ 的真实可用路径（hq vs exhq），并在 PRD 中更新结论

### Phase 5: Review gate

- [ ] 代码审查：错误处理、日志、超时、fail-closed、契约字段
- [ ] 运行检查：test/clippy
- [ ] PRD 更新：填写 Review Findings/Outcome

## Risks / Open Questions

- 北交所数据在 TDX 生态中的归属存在不确定性（hq vs exhq），需要一次真实环境探测确认。
- 若需要走 `exhq(SB)`，可能需要“代码 → instrument_code”的映射规则，需通过 `get_instrument_info` 进一步研究。

## References

- PyTDX 标准行情（hq）：https://pytdx-docs.readthedocs.io/zh-cn/latest/pytdx_hq/
- PyTDX 扩展行情（exhq）：https://pytdx-docs.readthedocs.io/zh-cn/master/pytdx_exhq/

