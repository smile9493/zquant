# Data Pipeline Contracts (v1 - Frozen)

> **Status**: Frozen for Phase A/B implementation
> **Last Updated**: 2026-03-13

---

## 1. ProviderContract

### Required Fields

```rust
trait DataProvider {
    // Identity
    fn provider_name(&self) -> &str;

    // Routing metadata
    fn capabilities(&self) -> Vec<Capability>;
    fn markets(&self) -> Vec<Market>;
    fn priority(&self) -> u8;

    // Dataset resolution
    fn supports_dataset_ids(&self) -> bool;
    // OR: fn resolve_dataset(&self, req: &DatasetRequest) -> Option<DatasetId>;

    // Fetch operations
    fn fetch(&self, req: FetchRequest) -> Result<RawData>;
    fn fetch_dataset(&self, req: DatasetRequest) -> Result<RawData>;
}
```

### Optional Fields (Phase B+)

```rust
    fn supports_async(&self) -> bool { false }
    fn metadata(&self) -> ProviderMetadata { ... }
```

---

## 2. Request Contracts

### DatasetRequest

```rust
struct DatasetRequest {
    capability: Capability,      // e.g., OHLCV, Fundamentals
    market: Market,               // e.g., US_EQUITY, CRYPTO
    dataset_id: Option<String>,   // Optional specific dataset
    time_range: Option<TimeRange>,
    forced_provider: Option<String>, // Fail-closed constraint
}
```

### IngestRequest

```rust
struct IngestRequest {
    dataset_request: DatasetRequest,
    // Future: quality_policy, cache_policy, etc.
}
```

---

## 3. Data Quality Contracts

### DqDecision

```rust
enum DqDecision {
    Accept,
    Degraded,
    Reject,
}
```

### DataQualityResult

```rust
struct DataQualityResult {
    decision: DqDecision,
    quality_score: f64,           // 0.0-1.0
    issues: Vec<DqIssue>,
    cleaned_data: NormalizedData,
}

struct DqIssue {
    severity: IssueSeverity,      // Error, Warning, Info
    field: Option<String>,
    message: String,
}
```

---

## 4. IngestResult

```rust
struct IngestResult {
    dataset_id: Option<DatasetId>,
    decision: DqDecision,
    quarantine_id: Option<QuarantineId>,
    persist_receipt: Option<PersistReceipt>,
}

struct PersistReceipt {
    storage_path: String,
    catalog_id: String,
    row_count: usize,
}
```

---

## 5. Pipeline Events (5 Required)

### Event Contracts

```rust
// 1. dataset.fetched
struct DatasetFetchedEvent {
    dataset_id: String,
    provider: String,
    capability: Capability,
    market: Market,
    timestamp: DateTime<Utc>,
    row_count: usize,
}

// 2. dataset.gate.completed
struct DatasetGateCompletedEvent {
    dataset_id: String,
    decision: DqDecision,
    quality_score: f64,
    issue_count: usize,
    timestamp: DateTime<Utc>,
}

// 3. dataset.ingested
struct DatasetIngestedEvent {
    dataset_id: String,
    decision: DqDecision,
    storage_path: String,
    catalog_id: String,
    timestamp: DateTime<Utc>,
}

// 4. dq.rejection
struct DqRejectionEvent {
    quarantine_id: String,
    dataset_id: String,
    reasons: Vec<String>,
    timestamp: DateTime<Utc>,
}

// 5. dq.degraded
struct DqDegradedEvent {
    dataset_id: String,
    quality_score: f64,
    issues: Vec<DqIssue>,
    timestamp: DateTime<Utc>,
}
```

---

## 6. PersistWriter Interface

### High-Level Contract

```rust
trait PersistWriter {
    // Write cleaned data
    fn write_dataset(
        &self,
        data: &NormalizedData,
        metadata: &DatasetMetadata,
    ) -> Result<PersistReceipt>;

    // Write catalog/metadata
    fn write_catalog(
        &self,
        catalog: &CatalogEntry,
    ) -> Result<String>; // Returns catalog_id

    // Write quarantine
    fn write_quarantine(
        &self,
        data: &RawData,
        reason: &DqRejectionEvent,
    ) -> Result<QuarantineId>;
}
```

### Metadata Fields (Reserved for SSOT)

```rust
struct DatasetMetadata {
    dataset_id: String,
    provider: String,
    capability: Capability,
    market: Market,

    // SSOT fields (reserved)
    available_at: Option<DateTime<Utc>>,
    point_in_time: Option<DateTime<Utc>>,
    version: u64,
}
```

---

## 7. Unified Entrypoint

### DataPipelineManager API

```rust
impl DataPipelineManager {
    // Fetch only (no persistence)
    pub fn fetch(&self, req: FetchRequest) -> Result<RawData>;

    // Fetch + normalize (no persistence)
    pub fn fetch_dataset(&self, req: DatasetRequest) -> Result<NormalizedData>;

    // Full pipeline: fetch → normalize → DQ → persist → events
    pub fn ingest_dataset(&self, req: IngestRequest) -> Result<IngestResult>;
}
```

---

## Validation Matrix

| Scenario | Expected Behavior |
|----------|-------------------|
| **Good case** | All fields valid → Accept → persist + catalog + event |
| **Degraded case** | Minor issues → Degraded → persist + catalog + event + warning |
| **Bad case** | Critical issues → Reject → quarantine + event |
| **Forced provider unavailable** | Fail-closed → Error (no silent fallback) |
| **No provider matches** | Error with clear message |

---

## Error Handling

Follow `.trellis/spec/backend/error-handling.md`:
- Use `anyhow::Result` for all fallible operations
- Wrap errors with context at each layer
- DQ rejection is a **business outcome**, not an error

---

## Notes

- This spec is **frozen** for Phase A/B implementation
- Changes require explicit approval and version bump
- All implementations must conform to these contracts
