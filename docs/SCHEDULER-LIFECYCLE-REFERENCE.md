# Scheduler Lifecycle Reference

**Last Updated:** 2025-10-31
**Status:** ✅ Survey Complete
**Related:** [Phase 4 Migration Plan](active-issue/PHASE-4-NEW-CRATE-MIGRATION.md)

---

## Quick Summary

All four schedulers in PulseArc follow a **consistent lifecycle pattern** with explicit `.start()`, `.stop()`, and `.is_running()` methods. No refactoring needed for Phase 4 integration.

---

## Scheduler Inventory

| Scheduler | Location | `.start()` Line | Status |
|-----------|----------|-----------------|--------|
| **BlockScheduler** | [crates/infra/src/scheduling/block_scheduler.rs](../crates/infra/src/scheduling/block_scheduler.rs) | [Line 136](../crates/infra/src/scheduling/block_scheduler.rs#L136) | ✅ Ready |
| **ClassificationScheduler** | [crates/infra/src/scheduling/classification_scheduler.rs](../crates/infra/src/scheduling/classification_scheduler.rs) | [Line 138](../crates/infra/src/scheduling/classification_scheduler.rs#L138) | ✅ Ready |
| **SyncScheduler** | [crates/infra/src/scheduling/sync_scheduler.rs](../crates/infra/src/scheduling/sync_scheduler.rs) | [Line 209](../crates/infra/src/scheduling/sync_scheduler.rs#L209) | ✅ Ready |
| **CalendarScheduler** | [crates/infra/src/scheduling/calendar_scheduler.rs](../crates/infra/src/scheduling/calendar_scheduler.rs) | [Line 126](../crates/infra/src/scheduling/calendar_scheduler.rs#L126) | ✅ Ready |

---

## Common Lifecycle Pattern

All schedulers implement the same lifecycle interface:

### Methods

```rust
impl Scheduler {
    /// Start the scheduler (spawns background tasks)
    pub async fn start(&mut self) -> SchedulerResult<()> { /* ... */ }

    /// Stop the scheduler (cancels tasks, awaits completion)
    pub async fn stop(&mut self) -> SchedulerResult<()> { /* ... */ }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool { /* ... */ }
}
```

### Safety Features

1. **CancellationToken** — All schedulers use `tokio_util::sync::CancellationToken` for graceful shutdown
2. **JoinHandle tracking** — All spawned tasks are tracked via `JoinHandle<()>`
3. **Drop safety** — `Drop` implementations cancel tasks with warning if still running
4. **Timeouts** — Start/stop operations have configurable timeouts
5. **Fail-fast** — `.start()` returns `Result` and fails if already running

---

## Usage Pattern for AppContext

### Initialization (in `AppContext::new()`)

```rust
impl AppContext {
    pub async fn new() -> Result<Self, AppError> {
        // ... create scheduler instances ...

        // Start all schedulers (fail-fast)
        block_scheduler.start().await?;
        classification_scheduler.start().await?;
        sync_scheduler.start().await?;
        calendar_scheduler.start().await?;

        Ok(Self {
            block_scheduler,
            classification_scheduler,
            sync_scheduler,
            calendar_scheduler,
            // ...
        })
    }
}
```

### Shutdown (in `AppContext::shutdown()`)

```rust
impl AppContext {
    pub async fn shutdown(&mut self) -> Result<(), AppError> {
        // Stop all schedulers (graceful)
        self.block_scheduler.stop().await?;
        self.classification_scheduler.stop().await?;
        self.sync_scheduler.stop().await?;
        self.calendar_scheduler.stop().await?;

        Ok(())
    }
}
```

---

## Scheduler-Specific Notes

### BlockScheduler
- **Purpose:** Cron-based block generation for inference workloads
- **Default schedule:** Every 15 minutes (`"0 */15 * * * *"`)
- **Job timeout:** 300 seconds (5 minutes)
- **Trait:** `BlockJob` for job execution

### ClassificationScheduler
- **Purpose:** Cron-based classification jobs for activity classification
- **Default schedule:** Every 10 minutes (`"0 */10 * * * *"`)
- **Job timeout:** 600 seconds (10 minutes)
- **Trait:** `ClassificationJob` for job execution
- **Thread safety:** Uses `Arc<RwLock<Option<JobScheduler>>>` for scheduler instance

### SyncScheduler
- **Purpose:** Interval-based outbox processing for API sync
- **Default interval:** 900 seconds (15 minutes)
- **Batch size:** 50 items per sync
- **Status:** ⚠️ PENDING - Awaiting repository implementations (Phase 3D follow-up)
- **Dependencies:** `ActivitySegmentRepository`, `ActivitySnapshotRepository` (placeholder traits)

### CalendarScheduler
- **Purpose:** Cron-based calendar event synchronization
- **Default schedule:** Every 15 minutes (`"0 */15 * * * *"`)
- **Job timeout:** 300 seconds (5 minutes)
- **Feature-gated:** Behind `calendar` feature flag
- **Privacy:** Email addresses are hashed (SHA256) in logs

---

## Error Handling

All schedulers return `SchedulerResult<()>` for lifecycle operations:

```rust
pub type SchedulerResult<T> = Result<T, SchedulerError>;

#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Scheduler is already running")]
    AlreadyRunning,

    #[error("Scheduler is not running")]
    NotRunning,

    #[error("Timeout after {duration:?}: {source}")]
    Timeout {
        duration: Duration,
        source: tokio::time::error::Elapsed,
    },

    #[error("Failed to start scheduler: {source}")]
    StartFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to stop scheduler: {source}")]
    StopFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    // ... other variants ...
}
```

---

## Testing

All schedulers have integration tests covering:
- ✅ Lifecycle (start → stop → verify not running)
- ✅ Double start rejection
- ✅ Restart after stop
- ✅ Job execution (where applicable)

Example test pattern:
```rust
#[tokio::test(flavor = "multi_thread")]
async fn lifecycle_runs_successfully() {
    let mut scheduler = create_scheduler();

    scheduler.start().await.expect("start succeeds");
    assert!(scheduler.is_running());

    scheduler.stop().await.expect("stop succeeds");
    assert!(!scheduler.is_running());
}
```

---

## Decision Record

**Survey Date:** 2025-10-31
**Decision:** All schedulers follow consistent lifecycle pattern — no refactoring needed
**Impact:** `AppContext` can safely initialize all schedulers with fail-fast `.start().await?` calls
**Approved By:** Automated survey (Claude Code)

See full decision log entry in [PHASE-4-NEW-CRATE-MIGRATION.md](active-issue/PHASE-4-NEW-CRATE-MIGRATION.md#decision-log)

---

## Future Considerations

1. **Scheduler health monitoring** — Consider adding `.health()` method to expose metrics
2. **Graceful degradation** — If one scheduler fails, should others continue?
3. **Startup ordering** — Are there dependencies between schedulers?
4. **Shutdown timeout** — Should we have a global timeout for all schedulers?

---

## Related Documentation

- [Phase 4 Migration Plan](active-issue/PHASE-4-NEW-CRATE-MIGRATION.md)
- [CLAUDE.md](../CLAUDE.md) — Workspace rules (see section 5: Async & Concurrency)
- [Scheduling Module](../crates/infra/src/scheduling/) — Implementation directory
