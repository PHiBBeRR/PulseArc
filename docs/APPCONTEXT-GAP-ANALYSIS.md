# AppContext vs AppState Gap Analysis

**Date:** 2025-10-31
**Purpose:** Document differences between new `AppContext` and legacy `AppState` to guide Phase 1 infrastructure work
**Status:** Phase 1.1 Baseline

---

## Executive Summary

**New Crate Status:** ✅ Compiles and passes tests (0 tests currently)
- `cargo check -p pulsearc-app`: ✅ PASS
- `cargo test -p pulsearc-app`: ✅ PASS (0 tests)

**Key Findings:**
- ✅ AppContext has core infrastructure (database, tracking, feature flags)
- ❌ Missing: All 4 schedulers (block, classification, sync, calendar)
- ❌ Missing: ML classifier infrastructure
- ⚠️ Different patterns: AppContext uses direct Arc references vs AppState's Arc<Mutex<Option<...>>>

---

## Structure Comparison

### New AppContext (crates/api/src/context/mod.rs:16-24)

```rust
pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<FeatureFlagService>,
    pub database_stats: Arc<DynDatabaseStatsPort>,
    _instance_lock: InstanceLock,
}
```

**Total Fields:** 6 (5 public + 1 private)

### Legacy AppState (legacy/api/src/lib.rs:444-457)

```rust
pub struct AppState {
    pub sync_scheduler: Arc<Mutex<Option<Arc<SyncScheduler>>>>,
    pub block_scheduler: Arc<Mutex<Option<Arc<BlockScheduler>>>>,
    pub classification_scheduler: Arc<Mutex<Option<Arc<ClassificationScheduler>>>>,
    #[cfg(feature = "calendar")]
    pub calendar_scheduler: Arc<Mutex<Option<CalendarSyncScheduler>>>,
    #[cfg(feature = "tree-classifier")]
    pub hybrid_classifier: Arc<Mutex<Option<Arc<HybridClassifier>>>>,
    #[cfg(feature = "tree-classifier")]
    pub metrics_tracker: Arc<MetricsTracker>,
    pub feature_flags: Arc<FeatureFlagService>,
}
```

**Total Fields:** 7 (4 always-compiled + 3 feature-gated)

---

## Field-by-Field Analysis

### ✅ Present in Both

| Field | AppContext | AppState | Notes |
|-------|-----------|----------|-------|
| `feature_flags` | `Arc<FeatureFlagService>` | `Arc<FeatureFlagService>` | ✅ Identical |

### ✅ Present in AppContext Only

| Field | Type | Purpose |
|-------|------|---------|
| `config` | `Config` | Application configuration |
| `db` | `Arc<DbManager>` | Database connection pool |
| `tracking_service` | `Arc<TrackingService>` | Activity tracking service |
| `database_stats` | `Arc<DynDatabaseStatsPort>` | Database statistics repository |
| `_instance_lock` | `InstanceLock` | Single-instance enforcement |

### ❌ Missing from AppContext (Present in AppState)

| Field | Type | Purpose | Priority |
|-------|------|---------|----------|
| `sync_scheduler` | `Arc<Mutex<Option<Arc<SyncScheduler>>>>` | Periodic outbox sync | **HIGH** |
| `block_scheduler` | `Arc<Mutex<Option<Arc<BlockScheduler>>>>` | Block generation for inference | **HIGH** |
| `classification_scheduler` | `Arc<Mutex<Option<Arc<ClassificationScheduler>>>>` | Activity classification | **HIGH** |
| `calendar_scheduler` | `Arc<Mutex<Option<CalendarSyncScheduler>>>` | Calendar sync (feature-gated) | **MEDIUM** |
| `hybrid_classifier` | `Arc<Mutex<Option<Arc<HybridClassifier>>>>` | ML classifier (feature-gated) | **LOW** |
| `metrics_tracker` | `Arc<MetricsTracker>` | ML metrics tracking (feature-gated) | **LOW** |

---

## Gap Categories

### Category 1: Schedulers (HIGH Priority)

**Missing:** All 4 schedulers from new infra are not in AppContext

From [Scheduler Lifecycle Reference](SCHEDULER-LIFECYCLE-REFERENCE.md):
- ✅ `BlockScheduler` exists in `crates/infra/src/scheduling/block_scheduler.rs`
- ✅ `ClassificationScheduler` exists in `crates/infra/src/scheduling/classification_scheduler.rs`
- ✅ `SyncScheduler` exists in `crates/infra/src/scheduling/sync_scheduler.rs`
- ✅ `CalendarScheduler` exists in `crates/infra/src/scheduling/calendar_scheduler.rs`

**Action Required:**
- Add scheduler fields to `AppContext`
- Initialize schedulers in `AppContext::new()`
- Call `.start().await?` on all schedulers (fail-fast)
- Implement `.shutdown()` method to call `.stop().await?`

### Category 2: ML Infrastructure (LOW Priority)

**Status:** Phase 3E not started (intentional - see Phase 4 plan)

- `HybridClassifier` - ML-based activity classification
- `MetricsTracker` - Training metrics collection

**Action Required:**
- ⚠️ Skip for Phase 4 (explicitly mentioned in migration plan)
- Revisit in future phase if ML training is prioritized

### Category 3: Pattern Differences

**AppState Pattern:**
```rust
pub sync_scheduler: Arc<Mutex<Option<Arc<SyncScheduler>>>>
```

**AppContext Pattern (recommended):**
```rust
pub sync_scheduler: Arc<SyncScheduler>
```

**Why Different?**
- **Legacy:** Arc<Mutex<Option<...>>> allows lazy initialization and runtime replacement
- **New:** Direct Arc<...> is simpler, initialized in constructor

**Recommendation:** Use direct `Arc<Scheduler>` pattern
- Schedulers initialized in `AppContext::new()`
- Call `.start().await?` immediately (fail-fast)
- No runtime replacement needed (restart app to reload)
- Simpler lifetime management

---

## Required Infrastructure Additions

### Phase 1: Add Scheduler Fields

**File:** `crates/api/src/context/mod.rs`

```rust
pub struct AppContext {
    // Existing fields
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<FeatureFlagService>,
    pub database_stats: Arc<DynDatabaseStatsPort>,

    // ADD: Schedulers
    pub block_scheduler: Arc<BlockScheduler>,
    pub classification_scheduler: Arc<ClassificationScheduler>,
    pub sync_scheduler: Arc<SyncScheduler>,
    #[cfg(feature = "calendar")]
    pub calendar_scheduler: Arc<CalendarScheduler>,

    // Existing private field
    _instance_lock: InstanceLock,
}
```

### Phase 2: Initialize Schedulers in Constructor

**Pattern (from [Scheduler Lifecycle Reference](SCHEDULER-LIFECYCLE-REFERENCE.md)):**

```rust
impl AppContext {
    pub async fn new() -> Result<Self> {
        // ... existing initialization ...

        // Initialize schedulers
        let metrics = Arc::new(PerformanceMetrics::new());

        let mut block_scheduler = BlockScheduler::new(
            "0 */15 * * * *".into(), // every 15 minutes
            Arc::new(block_job),
            metrics.clone(),
        )?;
        block_scheduler.start().await?;

        // ... similar for other schedulers ...

        Ok(Self {
            // ... existing fields ...
            block_scheduler: Arc::new(block_scheduler),
            // ... other schedulers ...
        })
    }

    /// Gracefully shutdown all schedulers
    pub async fn shutdown(mut self) -> Result<()> {
        // Stop schedulers in reverse order
        #[cfg(feature = "calendar")]
        Arc::get_mut(&mut self.calendar_scheduler)
            .ok_or_else(|| PulseArcError::Internal("cannot shutdown shared scheduler".into()))?
            .stop()
            .await?;

        Arc::get_mut(&mut self.sync_scheduler)
            .ok_or_else(|| PulseArcError::Internal("cannot shutdown shared scheduler".into()))?
            .stop()
            .await?;

        // ... similar for other schedulers ...

        Ok(())
    }
}
```

### Phase 3: Required Dependencies

**Missing from `Cargo.toml`:**
- None - all scheduler types already exist in `pulsearc-infra`

**Required Imports:**
```rust
use pulsearc_infra::{
    BlockScheduler,
    ClassificationScheduler,
    SyncScheduler,
    #[cfg(feature = "calendar")]
    CalendarScheduler,
    PerformanceMetrics,
};
```

---

## Scheduler Initialization Requirements

### BlockScheduler
- **Dependency:** `Arc<dyn BlockJob>`
- **Config:** Cron expression, timeouts
- **Status:** ✅ Type exists, ❌ Job implementation TBD

### ClassificationScheduler
- **Dependency:** `Arc<dyn ClassificationJob>`
- **Config:** Cron expression, timeouts
- **Status:** ✅ Type exists, ❌ Job implementation TBD

### SyncScheduler
- **Dependency:** `ApiForwarder`, `ActivitySegmentRepository`, `ActivitySnapshotRepository`
- **Config:** Interval, batch size
- **Status:** ⚠️ PENDING - Awaiting repository implementations (Phase 3D)

### CalendarScheduler (feature = "calendar")
- **Dependency:** `CalendarSyncWorker`
- **Config:** Cron expression, user emails
- **Status:** ✅ Type exists, ❌ Worker wiring TBD

---

## Testing Gaps

### Current Test Coverage
- **AppContext tests:** 0 (none exist)
- **Integration tests:** 0 (none exist)

### Required Tests (Phase 1)
1. **Unit Tests:**
   - `test_appcontext_new_succeeds` - Verify construction
   - `test_appcontext_initializes_all_services` - Check all fields populated
   - `test_appcontext_fails_without_encryption_key` - Error handling

2. **Integration Tests (Phase 2):**
   - `test_schedulers_start_on_init` - Verify `.start()` called
   - `test_schedulers_stop_on_shutdown` - Verify `.stop()` called
   - `test_scheduler_failure_prevents_startup` - Fail-fast behavior

---

## Build Configuration

### Current State
- ✅ `pulsearc-app` compiles without errors
- ✅ All dependencies resolve
- ⚠️ No Tauri integration yet (handled in Phase 3)

### Required Feature Flags
- `calendar` - Enables calendar scheduler
- `tree-classifier` - Skipped for Phase 4

---

## Migration Strategy

### Phase 1: Infrastructure Baseline (This Phase)
1. ✅ Verify baseline compiles
2. ✅ Document gaps (this document)
3. ❌ Add scheduler fields to AppContext
4. ❌ Add unit tests

### Phase 2: Scheduler Integration
1. Implement job traits (BlockJob, ClassificationJob)
2. Wire scheduler dependencies
3. Add `.start()` calls in constructor
4. Add `.shutdown()` method
5. Write integration tests

### Phase 3: Command Migration
1. Migrate commands one-by-one
2. Use feature flags for rollout
3. Validate scheduler behavior

---

## Decision Points

### 1. Async Constructor?

**Current:** `AppContext::new()` is synchronous
**Required:** Schedulers need `.start().await?`

**Options:**
- **A)** Make `AppContext::new()` async → ✅ **RECOMMENDED**
- **B)** Add separate `.start()` method on AppContext → Less ergonomic
- **C)** Lazy-start schedulers on first use → Breaks fail-fast principle

**Decision:** Make constructor async (see Phase 4 Decision Log)

### 2. Scheduler Ownership Pattern?

**Current:** None (schedulers not present)

**Options:**
- **A)** `Arc<Scheduler>` (direct) → ✅ **RECOMMENDED**
- **B)** `Arc<Mutex<Option<Arc<Scheduler>>>>` (like legacy) → Overly complex

**Decision:** Use direct `Arc<Scheduler>` pattern

### 3. Shutdown Mechanism?

**Current:** None (no lifecycle management)

**Options:**
- **A)** Explicit `.shutdown()` method → ✅ **RECOMMENDED**
- **B)** Rely on `Drop` implementations → Less predictable
- **C)** No shutdown (let OS clean up) → Ungraceful

**Decision:** Add explicit `.shutdown()` method

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Async constructor breaks Tauri integration | **HIGH** | Test Tauri setup in Phase 3 before full migration |
| Scheduler `.start()` fails at runtime | **MEDIUM** | Fail-fast: App won't launch if scheduler fails |
| Missing job implementations block testing | **MEDIUM** | Create no-op mock jobs for Phase 1 testing |
| Calendar scheduler requires user config | **LOW** | Use empty user list for Phase 1, populate in Phase 3 |

---

## Next Steps (Phase 1.2)

1. Add scheduler fields to `AppContext` struct
2. Update `AppContext::new()` to be async
3. Add skeleton scheduler initialization (with TODOs for job implementations)
4. Add `.shutdown()` method
5. Write unit tests for new fields
6. Update Phase 4 checklist (mark 1.1 complete)

---

## References

- [Scheduler Lifecycle Reference](SCHEDULER-LIFECYCLE-REFERENCE.md)
- [Phase 4 Migration Plan](active-issue/PHASE-4-NEW-CRATE-MIGRATION.md)
- [ADR-003: Hexagonal Architecture](../adrs/ADR-003-hexagonal-architecture.md)
- New AppContext: [crates/api/src/context/mod.rs](../crates/api/src/context/mod.rs)
- Legacy AppState: [legacy/api/src/lib.rs:444](../legacy/api/src/lib.rs#L444)
