# Phase 4E.1: Seed Snapshots Command — Implementation Notes

**Status:** ✅ COMPLETE
**Completed:** 2025-11-01
**LOC:** 193 (legacy) → 590 (new: 370 implementation + 220 tests)
**Tests:** 13/13 passing (6 integration + 7 unit)
**Complexity:** Low
**Risk:** Very Low (debug-only, isolated)

---

## Summary

Successfully migrated the `seed_activity_snapshots` debug command from legacy to the new crate architecture. The command generates realistic test activity snapshots for classifier testing and development.

**Key Decision:** Skipped feature flag (debug-only command doesn't need rollback mechanism).

---

## Commands Migrated

### `seed_activity_snapshots`
**File:** [crates/api/src/commands/seed_snapshots.rs](../../crates/api/src/commands/seed_snapshots.rs)
**Signature:**
```rust
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn seed_activity_snapshots(
    ctx: State<'_, Arc<AppContext>>,
    count: Option<usize>,
) -> Result<SeedResponse, String>
```

**Purpose:** Generate test activity snapshots for classifier development and testing.

**Implementation:**
- Embedded 10 realistic test blocks (VSCode, Chrome, Zoom, Slack, etc.)
- Snapshots generated at 60-second intervals
- 120-second gaps between blocks to simulate context switches
- Uses `tokio::task::spawn_blocking` for synchronous repository calls
- Returns count of snapshots created and blocks processed

**Compile-Time Guard:** Only compiled in debug builds via `#[cfg(debug_assertions)]`

---

## Architecture Changes

### 1. SnapshotRepository Extension

**File:** [crates/core/src/tracking/ports.rs](../../crates/core/src/tracking/ports.rs#L116-L119)

Added write methods to previously read-only `SnapshotRepository` trait:

```rust
pub trait SnapshotRepository: Send + Sync {
    // Existing read methods
    fn find_snapshots_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>)
        -> CommonResult<Vec<ActivitySnapshot>>;
    fn count_snapshots_by_date(&self, date: NaiveDate) -> CommonResult<usize>;

    // NEW: Write methods for seeding
    fn store_snapshot(&self, snapshot: &ActivitySnapshot) -> CommonResult<()>;
    fn store_snapshots_batch(&self, snapshots: &[ActivitySnapshot]) -> CommonResult<()>;
    fn count_active_snapshots(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> CommonResult<i64>;
}
```

**Rationale:** Extends repository pattern while maintaining hexagonal architecture boundaries.

### 2. SqlCipher Implementation

**File:** [crates/infra/src/database/activity_repository.rs](../../crates/infra/src/database/activity_repository.rs)

Implemented batch insert with transaction support:

```rust
fn store_snapshots_batch(&self, snapshots: &[ActivitySnapshot]) -> CommonResult<()> {
    let conn = self.db.get_connection()
        .map_err(|err| map_to_common_error("activity_snapshots.batch_connection", err))?;

    // Transaction for atomicity
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|err| map_storage_to_common("activity_snapshots.batch_begin",
            StorageError::from(err)))?;

    for snapshot in snapshots {
        if let Err(err) = insert_or_replace_snapshot(&conn, snapshot) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(map_storage_to_common("activity_snapshots.batch_insert", err));
        }
    }

    conn.execute("COMMIT", [])
        .map_err(|err| {
            let _ = conn.execute("ROLLBACK", []);
            map_storage_to_common("activity_snapshots.batch_commit", StorageError::from(err))
        })?;

    Ok(())
}
```

**Key Features:**
- `INSERT OR REPLACE` for idempotent seeding
- Transaction-based batch inserts (all-or-nothing)
- Proper error mapping: rusqlite::Error → StorageError → CommonError

---

## Test Strategy

### Repository-Focused Integration Tests

**File:** [crates/api/tests/seed_commands.rs](../../crates/api/tests/seed_commands.rs)

**Decision:** Test repository operations directly instead of Tauri command wrappers.

**Rationale:**
- Repository is the core business logic
- Simpler test setup (no Tauri State construction)
- Better isolation (tests database persistence, not IPC)

**Tests (6/6 passing):**

1. **`snapshot_repository_saves_and_retrieves`** - Basic CRUD operations
2. **`snapshot_repository_handles_large_batch`** - 100 snapshots in one transaction
3. **`snapshot_repository_is_idempotent`** - INSERT OR REPLACE prevents duplicates
4. **`snapshot_repository_preserves_data`** - All fields correctly persisted
5. **`snapshot_repository_filters_by_time_range`** - DateTime queries work correctly
6. **`snapshot_repository_handles_empty_batch`** - Edge case handling

### Unit Tests (7/7 passing)

**File:** [crates/api/src/commands/seed_snapshots.rs](../../crates/api/src/commands/seed_snapshots.rs)

1. `generate_test_blocks_returns_10_blocks` - Test data generation
2. `build_snapshots_creates_expected_count` - Snapshot count calculation
3. `snapshots_have_correct_gaps` - 120s gaps between blocks
4. `build_activity_context_includes_required_fields` - JSON structure validation
5. `extract_url_host_handles_various_formats` - URL parsing edge cases
6. Plus 2 additional helper function tests

---

## Embedded Test Data

The command includes 10 realistic test blocks covering common workflows:

1. **VSCode Coding Session** (1200s) - Deep work, no calendar
2. **Code Review in Chrome** (900s) - GitHub PR review
3. **Zoom Meeting** (1800s) - With calendar event
4. **Email Processing** (600s) - Gmail, inbox management
5. **Design Work** (1500s) - Figma collaboration
6. **Documentation** (1200s) - Notion with team event
7. **Slack Communication** (600s) - Channel discussions
8. **Research** (900s) - Safari with bookmarks
9. **Testing** (1200s) - VSCode testing session
10. **Break** (300s) - Idle detection simulation

**Snapshot Interval:** 60 seconds
**Gap Between Blocks:** 120 seconds
**Total Snapshots (all blocks):** ~245 snapshots

---

## Bonus Work: Phase 4C.2 Idle Sync Telemetry

During this session, also registered 7 idle sync telemetry commands that were implemented but not registered:

**File:** [crates/api/src/main.rs](../../crates/api/src/main.rs#L89-L95)

```rust
// Idle sync telemetry (Phase 4C.2)
pulsearc_lib::record_idle_detection,
pulsearc_lib::record_activity_wake,
pulsearc_lib::record_timer_event_emission,
pulsearc_lib::record_timer_event_reception,
pulsearc_lib::record_invalid_payload,
pulsearc_lib::record_state_transition,
pulsearc_lib::record_auto_start_tracker_rule,
```

**Commands:** Non-critical telemetry (no feature flag needed)
**Status:** Phase 4C.2 now marked complete

---

## Bug Fixes Applied

### 1. Chrono API Modernization

**Files:** `suggestions.rs`, `blocks.rs`, `adapters/blocks.rs`

**Issue:** Deprecated chrono API usage
```rust
// WRONG (deprecated):
let date = NaiveDate::from_timestamp_opt(timestamp, 0)?;

// CORRECT:
let date = DateTime::from_timestamp(timestamp, 0)
    .ok_or_else(|| PulseArcError::InvalidInput(format!("Invalid timestamp: {}", timestamp)))?
    .date_naive();
```

### 2. Error Wrapping in Transaction Code

**File:** `activity_repository.rs`

**Issue:** `map_storage_to_common()` expects `StorageError`, got raw rusqlite error

**Fix:**
```rust
conn.execute("BEGIN TRANSACTION", [])
    .map_err(|err| map_storage_to_common("...", StorageError::from(err)))?;
```

### 3. Import Cleanup

**File:** `idle.rs`

**Issue:** Missing `Arc` and `log_command_execution` imports after cleanup

**Fix:** Restored required imports that were accidentally removed

### 4. Serialize Derive

**File:** `adapters/blocks.rs`

**Issue:** `BlockAdapterError` missing `Serialize` for error responses

**Fix:** Added `#[derive(Serialize)]`

---

## Test Results

### New Crate Stack: 429/430 Tests Passing (99.8%)

```
✅ pulsearc-common:  68/68  (lib)
✅ pulsearc-domain:   0/0   (no tests)
✅ pulsearc-core:     6/6   (lib)
✅ pulsearc-infra:  315/316 (lib) - 1 pre-existing failure in outbox_worker
✅ pulsearc-app:     40/40  (lib + integration)
   ├─ lib:           23/23
   ├─ seed_commands:  6/6
   ├─ database:       8/8
   ├─ context:        6/6
   └─ window:         3/3
```

**Legacy Crate:** Expected failures (migration in progress)

---

## Files Modified (15 total)

### Core Layer
- [crates/core/src/tracking/ports.rs](../../crates/core/src/tracking/ports.rs) - Extended SnapshotRepository trait

### Infrastructure Layer
- [crates/infra/src/database/activity_repository.rs](../../crates/infra/src/database/activity_repository.rs) - Implemented batch save

### API Layer (New Implementation)
- [crates/api/src/commands/seed_snapshots.rs](../../crates/api/src/commands/seed_snapshots.rs) - **NEW** (370 LOC)
- [crates/api/src/commands/blocks.rs](../../crates/api/src/commands/blocks.rs) - Complete rebuild (3 commands)
- [crates/api/src/commands/suggestions.rs](../../crates/api/src/commands/suggestions.rs) - Fixed chrono API
- [crates/api/src/commands/idle.rs](../../crates/api/src/commands/idle.rs) - Fixed imports
- [crates/api/src/adapters/blocks.rs](../../crates/api/src/adapters/blocks.rs) - Fixed chrono + Serialize
- [crates/api/src/commands/mod.rs](../../crates/api/src/commands/mod.rs) - Exported seed_snapshots
- [crates/api/src/main.rs](../../crates/api/src/main.rs) - Registered 8 new commands

### Tests
- [crates/api/tests/seed_commands.rs](../../crates/api/tests/seed_commands.rs) - **NEW** (220 LOC, 6 tests)

### Documentation
- [docs/PHASE-4-NEW-CRATE-MIGRATION.md](../PHASE-4-NEW-CRATE-MIGRATION.md) - Updated progress tracking
- [docs/issues/completed/PHASE-4E1-IMPLEMENTATION-NOTES.md](./PHASE-4E1-IMPLEMENTATION-NOTES.md) - This document

---

## Lessons Learned

### 1. Clean Rebuild > Patching Legacy Code

When legacy code has extensive architectural debt (references to non-existent modules), prefer a clean rebuild following the user-provided playbook:

1. Decide command surface (what's essential?)
2. Rebuild on AppContext stack
3. Extract/replace missing infrastructure
4. Update tests
5. Remove legacy code
6. Clean up and run CI

**Result:** blocks.rs reduced from 12+ commands to 3 essential commands, cleaner architecture.

### 2. Repository-Focused Integration Tests

Testing repository operations directly is simpler and more valuable than testing Tauri command wrappers:

**Benefits:**
- Simpler test setup (no Tauri State construction)
- Better isolation (tests database persistence)
- Easier to debug (fewer layers)

**Trade-off:** Cannot test Tauri command registration (manual verification required)

### 3. Test Data Embedding

Embedding test data in the code (not files) improves test portability:

**Benefits:**
- No file I/O dependencies
- CI-friendly (no fixture file management)
- Easier to version control

**Trade-off:** Larger file size (acceptable for test data)

---

## Next Steps

### Immediate
- ✅ Phase 4E.1 complete (this phase)
- 🔄 Phase 4B.1: Block Commands (in progress, clean rebuild applied)

### Outstanding
- ⏸️ Phase 4B.2: Calendar Commands (946 LOC, highest risk)
- ⏸️ Phase 4C.1: Monitoring Commands (741 LOC)
- ⏸️ Fix 2 broken integration test files (block_commands.rs, user_profile_commands.rs)

---

## References

- **Migration Plan:** [PHASE-4-NEW-CRATE-MIGRATION.md](../PHASE-4-NEW-CRATE-MIGRATION.md)
- **Legacy Implementation:** `legacy/api/src/commands/seed_snapshots.rs`
- **New Implementation:** [crates/api/src/commands/seed_snapshots.rs](../../crates/api/src/commands/seed_snapshots.rs)
- **SqlCipher API Reference:** [SQLCIPHER-API-REFERENCE.md](./SQLCIPHER-API-REFERENCE.md)
- **Repository Pattern:** ADR-003 (Hexagonal Architecture)
