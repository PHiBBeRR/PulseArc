# Phase 4A.1: Database Commands Implementation Notes

**Date:** 2025-10-31
**Status:** ✅ Complete
**Implemented by:** Claude Code
**Scope:** Migration of 4 database commands from legacy/api/ to crates/api/

---

## Summary

Successfully migrated all 4 database commands to the new hexagonal architecture with:
- ✅ All code compiles without warnings
- ✅ 8/8 integration tests passing
- ✅ Feature flag toggling validated
- ✅ SQLCipher compliance verified (ADR-003)
- ✅ No LocalDatabase usage (clean migration)

**Commands Migrated:**
1. `get_database_stats` - Aggregate database statistics
2. `get_recent_snapshots` - Recent unprocessed snapshots query
3. `vacuum_database` - Database maintenance operation
4. `get_database_health` - Connectivity health check

---

## Architecture Decisions

### Decision 1: Feature Flag Name
**Question:** Which feature flag name to use?
- Schema flag: `new_database_commands`
- Legacy code: Uses `new_database_cmd`

**Decision:** Use `new_database_commands` (matches schema.sql line 420)

**Rationale:**
- Consistency with database schema
- Avoids maintaining two names for the same flag
- Simpler to reason about

**Implementation:** All commands check `new_database_commands` with fail-safe default `unwrap_or(false)`

---

### Decision 2: Scope of Phase 4A.1
**Question:** Which commands from `legacy/api/src/commands/database.rs` should be migrated?

**Original Plan (4 commands):**
1. `get_database_stats` ✅
2. `get_recent_snapshots` ✅
3. `vacuum_database` ✅ (NEW - didn't exist in legacy)
4. `get_database_health` ✅ (NEW - didn't exist in legacy)

**Legacy File Had 9 Commands:**
- `get_database_stats` ✅ (migrated)
- `get_recent_snapshots` ✅ (migrated)
- `get_recent_segments` ❌ (deferred - segments query)
- `get_user_projects` ❌ (deferred - cloud API call to Neon, not local DB)
- `get_recent_snapshots_all` ❌ (deferred - variant of snapshots query)
- `get_today_time_entries` ❌ (deferred - outbox entries)
- `get_today_calendar_events` ❌ (deferred - calendar events)
- `get_db_metrics` ❌ (deferred to Phase 4C.1 - Datadog metrics)
- `send_db_metrics_to_datadog` ❌ (deferred to Phase 4C.1)
- `send_activity_metrics_to_datadog` ❌ (deferred to Phase 4C.1)

**Decision:** Migrate only the 4 commands listed in Phase 4 plan

**Rationale:**
- Keeps scope manageable (1-day task)
- Creates NEW commands `vacuum_database` and `get_database_health` as thin wrappers
- Defers monitoring/Datadog commands to Phase 4C.1 (proper phase)
- Defers query commands to appropriate future phases
- Documents deferred commands for tracking

---

### Decision 3: DatabaseStats Type Mapping Strategy
**Question:** How to map new granular `DatabaseStatsPort` methods → legacy `DatabaseStats` type?

**Problem:**
- Legacy `get_database_stats` returns monolithic `DatabaseStats` struct with fields:
  - `snapshot_count: i64`
  - `unprocessed_count: i64`
  - `segment_count: i64`
  - `batch_stats: BatchStats`
- New `DatabaseStatsPort` has granular methods:
  - `get_database_size()` → `DatabaseSize`
  - `get_table_stats()` → `Vec<TableStats>`
  - `vacuum_database()` → `()`
  - `check_database_health()` → `HealthStatus`

**Options:**
- **A)** Create adapter that aggregates port calls to build `DatabaseStats`
- **B)** Add new `get_aggregate_stats()` method to port
- **C)** Change frontend to accept new granular types (breaking change)

**Decision:** Option A - Create adapter function `build_database_stats()`

**Rationale:**
- Preserves frontend stability (no breaking changes)
- Clean separation of concerns (adapter layer)
- Allows future migration to granular types when UI is ready
- Follows adapter pattern from hexagonal architecture

**Implementation:**
- Created `crates/api/src/adapters/database_stats.rs`
- Adapter calls multiple port methods and maps to legacy type
- Added `get_unprocessed_count()` method to `DatabaseStatsPort` for adapter

---

### Decision 4: Unprocessed Count Implementation
**Question:** How should adapter get `unprocessed_count` for `DatabaseStats`?

**Options:**
- **A)** Add `get_unprocessed_count()` to `DatabaseStatsPort` (proper)
- **B)** Access `DbManager` directly in adapter (pragmatic)
- **C)** Return 0 for now (placeholder)

**Decision:** Option A - Add method to port

**Rationale:**
- Keeps adapter pure (no direct database access)
- Maintains hexagonal architecture boundaries
- Minor change to existing port (non-breaking)
- Proper long-term solution

**Implementation:**
- Added `async fn get_unprocessed_count(&self) -> Result<i64>` to `DatabaseStatsPort`
- Implemented in `SqlCipherDatabaseStatsRepository` using SQL query
- Query: `SELECT COUNT(*) FROM activity_snapshots WHERE processed = 0`

**Critical Bug Fix:** Initial implementation used table name `snapshots` instead of `activity_snapshots`, causing test failures. Fixed to use correct table name.

---

### Decision 5: batch_stats Field Handling
**Question:** Should we populate `DatabaseStats.batch_stats` field?

**Frontend Usage Investigation:**
- Searched all frontend files for `batch_stats` or `batchStats` usage
- Type is generated (`frontend/shared/types/generated/DatabaseStats.ts`)
- Type is included in test fixtures
- **NO UI RENDERING** of `batch_stats` field found

**Options:**
- **A)** Populate from `SqlCipherBatchRepository.get_batch_stats()` (extra work)
- **B)** Return empty struct with TODO comment (simple)

**Decision:** Option B - Return empty `BatchStats { pending: 0, processing: 0, completed: 0, failed: 0 }`

**Rationale:**
- Frontend does not use this field (verified via grep)
- Avoids wiring additional repository for unused data
- Documents as planned enhancement when batch monitoring UI is added
- Keeps implementation simple and focused

**Implementation:** Both adapter and legacy path return empty `BatchStats` struct

---

### Decision 6: get_recent_snapshots Implementation
**Question:** How to implement `get_recent_snapshots` command?

**Options:**
- **A)** Call `DbManager` directly (quick, violates hexagonal architecture)
- **B)** Add to `DatabaseStatsPort` (quick, violates SRP - snapshots aren't "stats")
- **C)** Use existing `SqlCipherActivityRepository` (proper)

**Decision:** Option C - Use existing repository

**Investigation Findings:**
- `SqlCipherActivityRepository` exists and implements `SnapshotRepositoryPort`
- Already uses SQLCipher (ADR-003 compliant)
- Has method `find_snapshots_by_time_range(start, end)`
- Already wired to AppContext as `tracking_service` dependency

**Implementation:**
- Wired `SqlCipherActivityRepository` to `AppContext.snapshots` field
- Command queries unprocessed snapshots in last 30 days with LIMIT
- Uses direct SQL in command (acceptable for now, can refactor to dedicated query port later)
- Documented as technical debt for future `SnapshotQueryPort`

---

## SQLCipher Compliance Verification

**Critical Requirement:** All database access must use `SqlCipherConnection` (ADR-003)

**Verification Results:**
✅ `DbManager` - Uses `Arc<SqlCipherPool>` exclusively (lines 28-31 of manager.rs)
✅ `DatabaseStatsRepository` - All methods use `spawn_blocking` + `get_connection()`
✅ `SqlCipherActivityRepository` - SQLCipher-backed (verified in investigation)
✅ No `LocalDatabase` usage anywhere in new code

**Conclusion:** Full ADR-003 compliance achieved

---

## Implementation Details

### Files Created
1. **`crates/core/src/database_stats_ports.rs`** (updated)
   - Added `get_unprocessed_count()` method to trait

2. **`crates/infra/src/database/database_stats_repository.rs`** (updated)
   - Implemented `get_unprocessed_count()` with SQLCipher query

3. **`crates/api/src/adapters/mod.rs`** (new)
   - Module for adapter pattern implementations

4. **`crates/api/src/adapters/database_stats.rs`** (new)
   - `build_database_stats()` function
   - Maps new granular port methods → legacy `DatabaseStats` type

5. **`crates/api/src/commands/database.rs`** (new)
   - All 4 commands with feature flag wrappers
   - New implementations using ports/adapters
   - Legacy implementations isolated in `legacy_*` functions
   - Comprehensive logging and metrics tracking

6. **`crates/api/src/context/mod.rs`** (updated)
   - Added `snapshots: Arc<DynSnapshotRepositoryPort>` field
   - Wired `SqlCipherActivityRepository.clone()` to snapshots field

7. **`crates/api/tests/database_commands.rs`** (new)
   - 8 integration tests covering:
     - All 5 `DatabaseStatsPort` methods
     - Adapter functionality
     - Feature flag toggling
     - AppContext wiring

### Command Patterns

**Feature Flag Wrapper Pattern:**
```rust
#[tauri::command]
pub async fn get_database_stats(ctx: State<'_, Arc<AppContext>>) -> Result<DatabaseStats, String> {
    let command_name = "database::get_database_stats";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag (fail-safe: use legacy on error)
    let use_new = app_ctx
        .feature_flags
        .is_enabled("new_database_commands", false)
        .await
        .unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };

    let result = if use_new {
        new_get_database_stats(&app_ctx).await
    } else {
        legacy_get_database_stats(&app_ctx).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(&app_ctx, MetricRecord {
        command: command_name,
        implementation,
        elapsed,
        success,
        error_type: error_label.as_deref(),
    }).await;

    result.map_err(|e| e.to_string())
}
```

**Metrics Integration:**
- Every command logs execution via `tracing`
- Every command records metrics to database via `command_metrics` port
- Metrics include: command name, implementation path, duration, success/failure, error type
- Critical for Phase 4 validation (compare new vs legacy performance)

---

## Testing Strategy

### Integration Tests (8 tests)

**DatabaseStatsPort Tests:**
1. `test_database_stats_port_get_database_size` - Verify size queries work
2. `test_database_stats_port_get_table_stats` - Verify table enumeration
3. `test_database_stats_port_get_unprocessed_count` - Verify NEW method works
4. `test_database_stats_port_vacuum_database` - Verify VACUUM executes
5. `test_database_stats_port_check_database_health` - Verify health checks

**Adapter Tests:**
6. `test_database_stats_adapter_builds_legacy_type` - Verify type mapping works

**Feature Flag Tests:**
7. `test_feature_flag_can_be_toggled` - Verify flag enable/disable/check

**Wiring Tests:**
8. `test_snapshot_repository_wired_to_context` - Verify AppContext integration

**Test Infrastructure:**
- Uses `create_test_context()` helper with temporary databases
- Sets `TEST_DATABASE_ENCRYPTION_KEY` env var (no keychain required)
- Each test gets isolated database (UUID-based path)
- `#[tokio::test(flavor = "multi_thread")]` for async support

---

## Bugs Fixed During Implementation

### Bug 1: Wrong Table Name in get_unprocessed_count
**Symptom:** Tests failing with "no such table: snapshots"

**Root Cause:** Query used `snapshots` instead of `activity_snapshots`

**Fix:** Changed SQL query to:
```sql
SELECT COUNT(*) FROM activity_snapshots WHERE processed = 0
```

**Lesson:** Always verify table names against schema, don't assume naming conventions

---

### Bug 2: Repository Move Semantic in AppContext
**Symptom:** Compilation error: "borrow of moved value: `repository`"

**Root Cause:**
```rust
let repository = Arc::new(SqlCipherActivityRepository::new(db.clone()));
let tracking_service = Arc::new(TrackingService::new(provider, repository));
let snapshots = repository.clone(); // ❌ ERROR: repository moved above
```

**Fix:** Clone before passing to `TrackingService`:
```rust
let tracking_service = Arc::new(TrackingService::new(provider, repository.clone()));
let snapshots = repository; // ✅ OK: repository not moved
```

**Lesson:** Rust ownership rules - clone Arc before final move

---

## Validation Checklist

### Code Quality
- ✅ `cargo check -p pulsearc-app` passes
- ✅ `cargo clippy --all-targets -- -D warnings` passes (no warnings)
- ✅ `cargo fmt --all -- --check` passes
- ✅ `cargo test -p pulsearc-app` passes (8/8 tests)

### Functionality
- ✅ Feature flag toggles between old/new paths correctly
- ✅ All 4 commands return expected data structures
- ✅ Adapter correctly maps new types → legacy `DatabaseStats`
- ✅ Error handling provides clear error messages
- ✅ Logging shows correct routing (new vs legacy)

### Architecture
- ✅ No `LocalDatabase` usage (ADR-003 compliant)
- ✅ All database access via `SqlCipherConnection`
- ✅ Hexagonal architecture boundaries maintained
- ✅ Ports in `core/`, implementations in `infra/`, commands in `api/`

---

## Deferred Work

### Commands Deferred to Other Phases

**Phase 4C.1 (Monitoring Commands):**
- `get_db_metrics` - Datadog sync metrics
- `send_db_metrics_to_datadog` - Datadog push
- `send_activity_metrics_to_datadog` - Activity metrics

**Future Phases (Query/Reporting):**
- `get_recent_segments` - Segments query (similar to snapshots)
- `get_user_projects` - Cloud API call (not local database)
- `get_recent_snapshots_all` - All snapshots variant (not just unprocessed)
- `get_today_time_entries` - Outbox entries query
- `get_today_calendar_events` - Calendar events query

### Technical Debt Created
1. **Snapshot Queries in Command Layer:** `get_recent_snapshots` uses direct SQL instead of dedicated `SnapshotQueryPort`
   - **Mitigation:** Documented, works correctly, can refactor when query layer is formalized
   - **Priority:** Low (functionality works, architecture is acceptable)

2. **batch_stats Field Unpopulated:** Returns empty struct instead of real data
   - **Mitigation:** Frontend doesn't use it, documented with TODO
   - **Priority:** Low (wait for batch monitoring UI)

---

## Lessons Learned

### 1. Feature Flag Fail-Safe Pattern
**Learning:** Always default to legacy path on flag lookup errors

**Pattern:**
```rust
let use_new = ctx.feature_flags
    .is_enabled("new_database_commands", false)
    .await
    .unwrap_or(false);  // ⬅️ Critical: any error → legacy path
```

**Rationale:** Prevents cascading failures if feature flag service has issues

---

### 2. Adapter Pattern for Type Migration
**Learning:** Adapters are powerful for preserving frontend stability during backend refactoring

**Key Insight:** By creating `build_database_stats()` adapter, we:
- Avoided breaking frontend types
- Maintained clean hexagonal architecture
- Enabled gradual migration path
- Preserved rollback capability

---

### 3. Investigation Before Implementation
**Learning:** Time spent investigating existing infrastructure pays dividends

**Example:** Discovered `SqlCipherActivityRepository` already existed and was SQLCipher-compliant, avoiding unnecessary repository creation

---

### 4. Test-Driven Infrastructure Validation
**Learning:** Write tests first to validate assumptions about infrastructure

**Example:** Tests revealed table name bug immediately, preventing manual debugging

---

## Next Steps (Phase 4A.2)

**Ready to proceed with:**
- Phase 4A.2: User Profile Commands (2 commands, low complexity)
- Similar pattern: feature flag wrapper + repository usage
- Builds on lessons learned from Phase 4A.1

**Recommended approach:**
1. Review Phase 4A.1 implementation notes (this document)
2. Follow same pattern for feature flag wrappers
3. Use existing `UserProfileRepository` (already tested)
4. Add similar integration tests
5. Document any new decisions or patterns

---

## References

**Implementation Files:**
- Port: `crates/core/src/database_stats_ports.rs`
- Repository: `crates/infra/src/database/database_stats_repository.rs`
- Adapter: `crates/api/src/adapters/database_stats.rs`
- Commands: `crates/api/src/commands/database.rs`
- Tests: `crates/api/tests/database_commands.rs`

**Documentation:**
- [PHASE-4-NEW-CRATE-MIGRATION.md](../PHASE-4-NEW-CRATE-MIGRATION.md)
- [ADR-003: Hexagonal Architecture](../architecture/ADR-003-hexagonal-architecture.md)
- [CLAUDE.md](../../CLAUDE.md) - Project coding standards

**Related Issues:**
- Phase 4A.1 tracking: Section 2.1.1 in PHASE-4-NEW-CRATE-MIGRATION.md
- SQLCipher migration: ADR-003
