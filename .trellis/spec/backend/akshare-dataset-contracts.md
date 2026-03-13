# AkShare Dataset Contracts (v1 - Frozen)

> **Status**: Frozen for AkShare Provider implementation
> **Last Updated**: 2026-03-13

---

## Dataset Contract: CN Equity Daily OHLCV

### Stable Identifier

```
dataset_id = "cn_equity.ohlcv.daily"
```

### Capability and Market

```rust
capability: Capability::Ohlcv
market: Market::CnEquity
```

### Provider

```
provider_name = "akshare"
```

### Request Parameters

**Required in DatasetRequest**:
- `symbol_scope: Vec<String>` - List of stock symbols (e.g., ["000001", "600000"])
- `time_range: Option<TimeRange>` - Start and end dates for data fetch

**Example**:
```rust
DatasetRequest {
    capability: Capability::Ohlcv,
    market: Market::CnEquity,
    dataset_id: Some("cn_equity.ohlcv.daily".to_string()),
    symbol_scope: vec!["000001".to_string()],
    time_range: Some(TimeRange {
        start: Utc.ymd(2024, 1, 1).and_hms(0, 0, 0),
        end: Utc.ymd(2024, 12, 31).and_hms(0, 0, 0),
    }),
    forced_provider: Some("akshare".to_string()),
}
```

---

## Fail-Closed Semantics

### forced_provider = Some("akshare")

**Behavior**:
- If AkShare provider is unavailable → Return explicit error
- If dataset/market/capability mismatch → Return explicit error
- **No silent fallback to other providers**

**Error Messages**:
- "Provider 'akshare' not available"
- "Provider 'akshare' does not support capability/market combination"

---

## Output Fields (Normalized)

After normalization, data must contain at minimum:

```rust
{
    "date": "2024-01-01",      // ISO date string
    "open": 10.5,              // f64
    "high": 11.0,              // f64
    "low": 10.2,               // f64
    "close": 10.8,             // f64
    "volume": 1000000          // i64
}
```

---

## Python Adapter Contract

### Input (JSON stdin)

```json
{
    "symbol": "000001",
    "start_date": "2024-01-01",
    "end_date": "2024-12-31",
    "adjust": "qfq"
}
```

### Output (JSON stdout)

```json
{
    "status": "success",
    "data": [
        {
            "date": "2024-01-01",
            "open": 10.5,
            "high": 11.0,
            "low": 10.2,
            "close": 10.8,
            "volume": 1000000
        }
    ]
}
```

### Error Output

```json
{
    "status": "error",
    "message": "Failed to fetch data: <reason>"
}
```

---

## Notes

- `dataset_id` is a **type identifier**, not a request instance ID
- Symbol and time_range are **request parameters**, not part of dataset_id
- This contract is frozen for v1 implementation
- Changes require explicit approval and version bump
