# Phase 4 Tracking Document - Critical Errata

**Created:** 2025-10-31
**Updated:** 2025-10-31 (v1.1 - Technical corrections)
**Status:** 🔴 **CRITICAL CORRECTIONS REQUIRED**
**Impact:** Phase 4 cannot start until these issues are resolved

---

## Document Revisions

### Version 1.1 (2025-10-31) - Technical Corrections

**Fixed:**
- ✅ `FeatureFlagsRepository` now uses `Arc<DbManager>` (not `Arc<SqlCipherPool>`)
- ✅ Added `spawn_blocking` for all database operations (keep Tauri commands non-blocking)
- ✅ Schema migration now includes explicit steps:
  1. Update `crates/infra/src/database/schema.sql`
  2. Bump `SCHEMA_VERSION` constant in `manager.rs`
  3. Test migration
- ✅ Repository pattern matches existing `SqlCipherActivityRepository` pattern

**Pattern Note:**
All repositories in this codebase follow ADR-003:
- Hold `Arc<DbManager>` (not direct pool reference)
- Call `db.get_connection()` to acquire connections
- Use `tokio::task::spawn_blocking` for all database I/O

---

## Overview

The initial Phase 4 tracking document ([PHASE-4-API-REWIRING-TRACKING.md](./PHASE-4-API-REWIRING-TRACKING.md)) contains **3 critical issues** that block execution:

1. 🔴 **HIGH**: Missing Phase 3 deliverables - assumed services don't exist
2. 🔴 **HIGH**: Feature flag mechanism won't work on macOS GUI apps
3. 🟡 **MEDIUM**: Cleanup timeline conflicts with validation period

---

## Issue 1: Missing Phase 3 Deliverables (HIGH PRIORITY)

### Problem

Phase 4 tracking document assumes several services/repositories that **do not exist** in the codebase:

| Assumed Dependency | Phase 4 Task | Current Status | Location Checked |
|-------------------|--------------|----------------|------------------|
| `UserProfileRepository` | 4A.2 (User Profile Commands) | ❌ **MISSING** | Not in `crates/infra/src/database/` |
| `IdleDetector` service | 4B.3 (Idle Commands) | ❌ **MISSING** | Not in `crates/core/` |
| `CostTracker` | 4C.1 (Monitoring Commands) | ❌ **MISSING** | Phase 3D.5 not complete |
| `OutboxWorker` | 4C.1 (Monitoring Commands) | ❌ **MISSING** | Phase 3D.4 not complete |
| `TrainingPipeline` | 4D.1 (ML Training Commands) | ❌ **MISSING** | Phase 3E.3 not complete |
| `TrainingExporter` | 4D.1 (ML Training Commands) | ❌ **MISSING** | Phase 3E.3 not complete |

### What Actually Exists (Verified)

✅ **From Phase 2:**
- `BlockBuilder` - `crates/core/src/classification/block_builder.rs`
- `ClassificationService` - `crates/core/src/classification/service.rs`
- `TrackingService` - `crates/core/src/tracking/service.rs`

✅ **From Phase 3A (Complete):**
- `ActivityRepository` - `crates/infra/src/database/activity_repository.rs`
- `SegmentRepository` - `crates/infra/src/database/segment_repository.rs`
- `BlockRepository` - `crates/infra/src/database/block_repository.rs`
- `OutboxRepository` - `crates/infra/src/database/outbox_repository.rs`
- `IdMappingRepository` - `crates/infra/src/database/id_mapping_repository.rs`
- `TokenUsageRepository` - `crates/infra/src/database/token_usage_repository.rs`
- `BatchRepository` - `crates/infra/src/database/batch_repository.rs`
- `DlqRepository` - `crates/infra/src/database/dlq_repository.rs`
- `CalendarEventRepository` - `crates/infra/src/database/calendar_event_repository.rs`

❌ **Missing from Phase 3:**
- No `UserProfileRepository` (needs to be added to Phase 3A follow-up)
- No idle detection service in `core` (idle logic may be in legacy only)
- Phase 3D not complete (missing `CostTracker`, `OutboxWorker`)
- Phase 3E not started (missing ML training infrastructure)

### Impact

**Phase 4 "Readiness" Table is INCORRECT:**

| Phase 4 Task | Document Claims | Actual Status | Blocker |
|--------------|----------------|---------------|---------|
| 4A.1 (Database) | ✅ Ready | ✅ **READY** | None |
| 4A.2 (User Profile) | ✅ Ready | ❌ **BLOCKED** | Missing `UserProfileRepository` |
| 4A.3 (Window) | ✅ Ready | ✅ **READY** | None (UI-only) |
| 4B.1 (Blocks) | ✅ Ready | ✅ **READY** | `BlockBuilder` exists ✅ |
| 4B.2 (Calendar) | ✅ Ready | ✅ **READY** | Phase 3C.5 complete ✅ |
| 4B.3 (Idle) | ✅ Ready | ⚠️ **PARTIAL** | Need idle service (may exist in legacy) |
| 4C.1 (Monitoring) | ⏸️ Blocked | ⏸️ **BLOCKED** | Phase 3D incomplete |
| 4C.2 (Idle Sync) | ⏸️ Blocked | ⏸️ **BLOCKED** | Phase 3D incomplete |
| 4D.1 (ML Training) | ⏸️ Blocked | ⏸️ **BLOCKED** | Phase 3E not started |
| 4E.1 (Seed Snapshots) | ✅ Ready | ✅ **READY** | Repositories exist ✅ |

**Corrected Readiness: 5/11 tasks ready (not 7/11)**

### Solution

#### Immediate Actions (Phase 3 Follow-ups)

**1. Add UserProfileRepository to Phase 3A.9 Follow-up**

Create port trait in `crates/core/src/user/ports.rs`:
```rust
pub trait UserProfileRepository: Send + Sync {
    async fn get_profile(&self, user_id: &str) -> Result<UserProfile>;
    async fn update_profile(&self, profile: &UserProfile) -> Result<()>;
}
```

Implement in `crates/infra/src/database/user_profile_repository.rs`:
- CRUD operations for `user_profiles` table
- Use `SqlCipherConnection` pattern from Phase 3A
- Estimated: 150 LOC, 3 tests

**2. Create IdleDetector Service in Core (if needed)**

Options:
- **Option A:** Use existing legacy idle detection directly from commands (minimal changes)
- **Option B:** Create port trait + service in `core` (cleaner, more work)

Recommended: **Option A** for Phase 4, defer refactoring to future phase.

**3. Complete Phase 3D Before Starting 4C**

No shortcuts - monitoring commands require:
- ✅ `CostTracker` from Phase 3D.5
- ✅ `OutboxWorker` from Phase 3D.4
- ✅ Schedulers from Phase 3D.1-3

**4. Complete Phase 3E Before Starting 4D (Optional)**

ML commands require:
- ✅ `TrainingPipeline` from Phase 3E.3
- ✅ `TrainingExporter` from Phase 3E.3

Or skip Phase 4D entirely if ML features not needed.

---

## Issue 2: Feature Flag Mechanism Won't Work on macOS (HIGH PRIORITY)

### Problem

The Phase 4 tracking document proposes using **environment variables** for feature flags:

```bash
PULSEARC_USE_NEW_INFRA=true|false
```

**This will NOT work for macOS GUI applications launched via Finder/LaunchServices:**

- ❌ macOS GUI apps **do not inherit shell environment variables**
- ❌ Users launching via Finder will not see env vars set in `.zshrc` or terminal
- ❌ The "5-minute rollback" strategy fails in production builds
- ❌ Production users would be **stuck on whichever code path was compiled in**

### Why This Is Critical

Without working feature flags:
- ⚠️ **No instant rollback** if issues found in production
- ⚠️ **Can't do gradual rollout** (enable command-by-command)
- ⚠️ **Can't do parallel validation** (run old & new side-by-side)
- ⚠️ **Phase 4 rollback plan is invalidated**

### Solution Options

#### **Option 1: Persisted Configuration File (RECOMMENDED)**

Store feature flags in SQLite database or config file:

```rust
// In Tauri app state
pub struct FeatureFlags {
    use_new_infra: AtomicBool,
    new_blocks_cmd: AtomicBool,
    new_calendar_cmd: AtomicBool,
    // ... per-command flags
}

impl FeatureFlags {
    pub fn load_from_db(db: &DbManager) -> Self {
        // Load flags from `feature_flags` table
        // Default: true (new infra)
        // Allow runtime toggle via admin UI or database update
    }

    pub fn use_new_infra(&self) -> bool {
        self.use_new_infra.load(Ordering::Relaxed)
    }
}
```

**Configuration table:**
```sql
CREATE TABLE feature_flags (
    flag_name TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL
);
```

**Benefits:**
- ✅ Works in GUI apps (no env vars needed)
- ✅ Persists across restarts
- ✅ Can be toggled via admin UI
- ✅ Can be toggled via database update (hotfix)
- ✅ Supports per-command granularity

**Implementation:**
1. Add `feature_flags` table to schema (Phase 3A.4 follow-up)
2. Create `FeatureFlagsRepository` (50 LOC)
3. Add `FeatureFlags` to Tauri app state
4. Update all commands to check flags from app state (not env vars)

**Estimated Effort:** 1 day (before starting Phase 4A)

---

#### **Option 2: Tauri State with Runtime Toggle**

Use Tauri's state management + command to toggle:

```rust
#[tauri::command]
pub async fn toggle_feature_flag(
    flag: String,
    enabled: bool,
    state: State<'_, FeatureFlags>,
) -> Result<(), String> {
    match flag.as_str() {
        "new_infra" => state.use_new_infra.store(enabled, Ordering::Relaxed),
        "new_blocks" => state.new_blocks_cmd.store(enabled, Ordering::Relaxed),
        // ...
        _ => return Err("Unknown flag".into()),
    }
    Ok(())
}
```

**Benefits:**
- ✅ Simple implementation
- ✅ Runtime toggleable
- ⚠️ Flags reset on app restart (ephemeral)

**Drawback:**
- ❌ Not persisted (flags reset to default on restart)
- ❌ Requires UI for toggling (not suitable for hotfix)

---

#### **Option 3: Build-Time Feature Flags (NOT RECOMMENDED)**

Use Cargo feature flags:

```toml
[features]
default = ["new-infra"]
new-infra = []
legacy-fallback = []
```

**Benefits:**
- ✅ Simple (compile-time only)

**Drawbacks:**
- ❌ **No runtime rollback** (requires recompile + redeploy)
- ❌ **No gradual rollout** (all-or-nothing)
- ❌ **Defeats entire purpose of Phase 4 strategy**

**Verdict:** ❌ **DO NOT USE** - Eliminates rollback capability

---

### Recommended Implementation (Option 1)

**Before starting Phase 4A:**

1. **Add feature flags table to schema** (Phase 3A follow-up)

   **Step 1: Update `crates/infra/src/database/schema.sql`:**
   ```sql
   -- Add after existing tables
   CREATE TABLE IF NOT EXISTS feature_flags (
       flag_name TEXT PRIMARY KEY,
       enabled INTEGER NOT NULL DEFAULT 1,
       updated_at INTEGER NOT NULL,
       description TEXT
   );

   -- Initialize default flags
   INSERT INTO feature_flags (flag_name, enabled, updated_at, description) VALUES
       ('use_new_infra', 1, strftime('%s', 'now'), 'Master flag for new infrastructure'),
       ('new_database_cmd', 1, strftime('%s', 'now'), 'Database commands'),
       ('new_blocks_cmd', 1, strftime('%s', 'now'), 'Block building commands'),
       ('new_calendar_cmd', 1, strftime('%s', 'now'), 'Calendar commands'),
       ('new_idle_cmd', 1, strftime('%s', 'now'), 'Idle commands'),
       ('new_monitoring_cmd', 1, strftime('%s', 'now'), 'Monitoring commands'),
       ('new_ml_cmd', 1, strftime('%s', 'now'), 'ML training commands');
   ```

   **Step 2: Bump `SCHEMA_VERSION` in `crates/infra/src/database/manager.rs`:**
   ```rust
   // Change from:
   const SCHEMA_VERSION: i32 = 1;

   // To:
   const SCHEMA_VERSION: i32 = 2;
   ```

   **Step 3: Test migration:**
   ```bash
   # Create test with old schema
   cargo test -p pulsearc-infra database::manager::test_migrations

   # Verify schema_version table shows version 2
   ```

2. **Create FeatureFlagsRepository** (following established repository pattern)
   ```rust
   // crates/infra/src/database/feature_flags_repository.rs
   use std::sync::Arc;
   use pulsearc_domain::{Result, PulseArcError};
   use rusqlite::params;
   use tokio::task;

   use super::manager::DbManager;

   /// Repository for managing feature flags stored in the database.
   pub struct FeatureFlagsRepository {
       db: Arc<DbManager>,
   }

   impl FeatureFlagsRepository {
       pub fn new(db: Arc<DbManager>) -> Self {
           Self { db }
       }

       /// Check if a feature flag is enabled.
       ///
       /// Defaults to `true` (enabled) if flag not found.
       pub async fn is_enabled(&self, flag_name: &str) -> Result<bool> {
           let db = Arc::clone(&self.db);
           let flag_name = flag_name.to_string();

           // Use spawn_blocking for database query (keep Tauri commands non-blocking)
           task::spawn_blocking(move || {
               let conn = db.get_connection()?;

               let enabled: i32 = conn
                   .query_row(
                       "SELECT enabled FROM feature_flags WHERE flag_name = ?",
                       params![flag_name],
                       |row| row.get(0),
                   )
                   .unwrap_or(1); // Default to enabled if not found

               Ok(enabled == 1)
           })
           .await
           .map_err(|e| PulseArcError::Internal(format!("Task join error: {}", e)))?
       }

       /// Set a feature flag's enabled state.
       pub async fn set_enabled(&self, flag_name: &str, enabled: bool) -> Result<()> {
           let db = Arc::clone(&self.db);
           let flag_name = flag_name.to_string();
           let enabled_int = if enabled { 1 } else { 0 };

           // Use spawn_blocking for database write
           task::spawn_blocking(move || {
               let conn = db.get_connection()?;

               conn.execute(
                   "UPDATE feature_flags SET enabled = ?, updated_at = strftime('%s', 'now') WHERE flag_name = ?",
                   params![enabled_int, flag_name],
               )
               .map_err(|e| PulseArcError::Database(format!("Failed to update flag: {}", e)))?;

               Ok(())
           })
           .await
           .map_err(|e| PulseArcError::Internal(format!("Task join error: {}", e)))?
       }
   }
   ```

   **Key Pattern Notes:**
   - ✅ Holds `Arc<DbManager>` (not `Arc<SqlCipherPool>`)
   - ✅ Calls `db.get_connection()` for each query
   - ✅ Uses `tokio::task::spawn_blocking` for all database operations
   - ✅ Follows same pattern as `SqlCipherActivityRepository`

3. **Add to Tauri app state**
   ```rust
   pub struct AppState {
       pub db_manager: Arc<DbManager>,
       pub feature_flags: Arc<FeatureFlags>,
       // ...
   }

   pub struct FeatureFlags {
       repo: FeatureFlagsRepository,
       cache: Mutex<HashMap<String, bool>>, // Cache for performance
   }
   ```

4. **Update command pattern**
   ```rust
   #[tauri::command]
   pub async fn build_blocks_for_date(
       date: NaiveDate,
       state: State<'_, AppState>,
   ) -> Result<Vec<ProposedBlock>, String> {
       let use_new_infra = state.feature_flags
           .is_enabled("new_blocks_cmd")
           .await
           .unwrap_or(true); // Default: true

       if use_new_infra {
           // NEW implementation
       } else {
           // LEGACY implementation
       }
   }
   ```

5. **Add admin toggle command**
   ```rust
   #[tauri::command]
   pub async fn toggle_feature_flag(
       flag: String,
       enabled: bool,
       state: State<'_, AppState>,
   ) -> Result<(), String> {
       state.feature_flags.repo.set_enabled(&flag, enabled).await
           .map_err(|e| e.to_string())?;

       // Clear cache to pick up new value
       state.feature_flags.cache.lock().unwrap().clear();

       tracing::warn!("Feature flag toggled: {} = {}", flag, enabled);
       Ok(())
   }
   ```

**Rollback Procedure (With Persisted Flags):**

1. **Via Admin UI:**
   - Open dev panel: `Cmd+Shift+D`
   - Navigate to "Feature Flags" tab
   - Toggle flag to `disabled`
   - Restart app (flags persist)

2. **Via Database Update (Hotfix):**
   ```bash
   # Connect to database
   sqlite3 ~/Library/Application\ Support/com.pulsearc.app/pulsearc.db

   # Disable specific flag
   UPDATE feature_flags SET enabled = 0 WHERE flag_name = 'new_blocks_cmd';

   # Or disable all new infrastructure
   UPDATE feature_flags SET enabled = 0 WHERE flag_name = 'use_new_infra';
   ```

3. **Restart app** - flags load from database on startup

**Estimated Rollback Time:** 2-5 minutes (not instant, but reliable)

---

## Issue 3: Cleanup Timeline Conflicts (MEDIUM PRIORITY)

### Problem 1: Timeline Contradiction

Phase 4F scheduled for **Days 14-16** but requires **1-2 weeks validation**:

- Phase 4F starts on Day 14 (remove feature flags)
- But also requires "1-2 week validation period before cleanup"
- **These timelines are mutually exclusive**

### Problem 2: Directory Structure Contradiction

Cleanup instructions say:
> "Delete `legacy/api/src/` directory (except `commands/` - those stay)"

**This is not actionable:**
- Commands **live inside** `legacy/api/src/commands/`
- Can't delete parent directory while keeping child directory
- Commands are being **rewired in place**, not moved to new crate

### Solution

#### **Revised Cleanup Timeline**

**Phase 4A-4E: Days 1-13** (Rewiring commands)
- Implement feature flags
- Rewire commands to use new infrastructure
- Legacy code paths remain as fallback
- **Do NOT delete anything**

**Validation Period: Days 14-27** (2 weeks production validation)
- Deploy to production with feature flags enabled
- Monitor for issues:
  - Error rates
  - Performance metrics
  - User feedback
  - Feature flag usage (% old vs new)
- **Rollback criteria:** Any P0/P1 issue → toggle flags, investigate
- **Success criteria:** 2 weeks without P0/P1 issues

**Phase 4F: Days 28-31** (Cleanup - AFTER validation)
- Day 28: Remove feature flag checks from commands
  - Delete legacy code paths from each command
  - Remove `if use_new_infra { ... } else { ... }` branching
  - Commands now **only** use new infrastructure

- Day 29: Remove legacy re-exports
  - Clean up `lib.rs` module structure
  - Update internal imports

- Day 30-31: Delete legacy infrastructure code
  - **Keep:** `legacy/api/src/commands/` (rewired in place)
  - **Delete:** All other legacy modules:
    - `legacy/api/src/db/` (replaced by `infra/database/`)
    - `legacy/api/src/detection/` (stays in `domain`, separate refactor)
    - `legacy/api/src/domain/` (replaced by `core` services)
    - `legacy/api/src/http/` (replaced by `infra/http/`)
    - `legacy/api/src/inference/` (split: ML → `infra`, logic → `core`)
    - `legacy/api/src/integrations/` (replaced by `infra/integrations/`)
    - `legacy/api/src/observability/` (replaced by `infra/observability/`)
    - `legacy/api/src/preprocess/` (replaced by `core` + `common`)
    - `legacy/api/src/shared/` (replaced by `common`)
    - `legacy/api/src/sync/` (replaced by `infra/sync/`)
    - `legacy/api/src/tracker/` (replaced by `infra/platform/`)
    - `legacy/api/src/utils/` (replaced by `domain` utils)

- Day 31: Update documentation
  - Mark Phase 4 complete in tracking docs
  - Update architecture diagrams
  - Create migration retrospective

**Revised Timeline: 4-5 weeks total** (not 2-3 weeks)

#### **What Stays, What Goes**

**KEEP (Rewired in Place):**
```
legacy/api/src/
├── commands/          ← KEEP - rewired to use new infra
│   ├── blocks.rs      ← Uses BlockBuilder from core + BlockRepository from infra
│   ├── calendar.rs    ← Uses CalendarProvider from infra
│   ├── database.rs    ← Uses repositories from infra
│   ├── idle.rs        ← Uses idle detection (TBD: legacy or new service)
│   ├── monitoring.rs  ← Uses OutboxWorker + CostTracker from infra (after 3D)
│   ├── ml_training.rs ← Uses TrainingPipeline from infra (after 3E)
│   ├── seed_snapshots.rs ← Uses repositories from infra
│   ├── user_profile.rs ← Uses UserProfileRepository from infra (after created)
���   └── window.rs      ← Minimal changes (UI-only)
├── lib.rs             ← UPDATE - re-exports commands, remove legacy re-exports
└── main.rs            ← UPDATE - initialize new infrastructure, remove legacy setup
```

**DELETE (After Validation):**
```
legacy/api/src/
├── db/                ← DELETE - replaced by crates/infra/src/database/
├── domain/            ← DELETE - replaced by crates/core/
├── http/              ← DELETE - replaced by crates/infra/src/http/
├── inference/         ← DELETE - split between core/infra/domain (see Phase 3 audit)
├── integrations/      ← DELETE - replaced by crates/infra/src/integrations/
├── observability/     ← DELETE - replaced by crates/infra/src/observability/
├── preprocess/        ← DELETE - replaced by crates/core/
├── shared/            ← DELETE - replaced by crates/common/
├── sync/              ← DELETE - replaced by crates/infra/src/sync/
├── tracker/           ← DELETE - replaced by crates/infra/src/platform/
├── utils/             ← DELETE - replaced by crates/domain/ utils
└── tooling/           ← KEEP OR DELETE - decide based on usage
```

**Architecture After Phase 4:**
```
legacy/api/           ← Tauri app crate (stays, but much smaller)
├── src/
│   ├── commands/     ← API layer (rewired to use infra)
│   ├── lib.rs        ← Thin wrapper, re-exports commands
│   └── main.rs       ← App initialization, Tauri setup
└── Cargo.toml        ← Dependencies: crates/core, crates/infra, crates/domain

crates/
├── core/             ← Business logic (Phase 2 ✅)
├── domain/           ← Pure types (Phase 1 ✅)
├── infra/            ← Infrastructure adapters (Phase 3 🔄)
└── common/           ← Shared utilities (already migrated ✅)
```

---

## Summary of Required Changes

### Before Starting Phase 4

**1. Phase 3 Follow-ups (HIGH PRIORITY):**
- [ ] Create `UserProfileRepository` (Phase 3A.9 follow-up)
- [ ] Create `feature_flags` table in schema
- [ ] Implement `FeatureFlagsRepository` (persisted config)
- [ ] Add `FeatureFlags` to Tauri app state
- [ ] Create admin UI for toggling flags (or document database update procedure)

**2. Phase 4 Tracking Document Updates:**
- [ ] Update dependency matrix (5/11 ready, not 7/11)
- [ ] Replace env var feature flags with persisted config pattern
- [ ] Update timeline: 4-5 weeks (not 2-3 weeks)
- [ ] Clarify validation period: 2 weeks BEFORE cleanup
- [ ] Update cleanup instructions: specify what stays vs what goes
- [ ] Add "Before Phase 4A" section listing prerequisites

**3. Blockers to Resolve:**
- [ ] Complete Phase 3D (for Task 4C - monitoring)
- [ ] Complete Phase 3E (for Task 4D - ML, optional)
- [ ] OR: Remove blocked tasks from "ready to start" claims

### Updated Phase 4 Readiness

| Phase 4 Sub-Phase | Can Start? | Blockers | Notes |
|-------------------|------------|----------|-------|
| **4A (Core)** | ⚠️ **PARTIAL** | `UserProfileRepository` missing | 4A.1 ✅, 4A.3 ✅, but 4A.2 ❌ |
| **4B (Features)** | ✅ **YES** | None (if idle uses legacy) | 4B.1 ✅, 4B.2 ✅, 4B.3 ⚠️ |
| **4C (Monitoring)** | ❌ **NO** | Phase 3D incomplete | CostTracker, OutboxWorker missing |
| **4D (ML)** | ❌ **NO** | Phase 3E not started | TrainingPipeline missing (optional) |
| **4E (Dev Tools)** | ✅ **YES** | None | Repositories exist |
| **4F (Cleanup)** | ⏳ **LATER** | 4A-4E + 2 week validation | After validation period |

**Actual Readiness: 3-4 tasks ready** (not 7/11)

---

## Next Steps

### Immediate (Before Phase 4A)

1. **Create Phase 3A.9.1 Follow-up PR:**
   - Add `UserProfileRepository` port trait to `core`
   - Implement repository in `infra`
   - Add `feature_flags` table to schema
   - Create `FeatureFlagsRepository`
   - Estimated: 1-2 days

2. **Update Phase 4 Tracking Document:**
   - Incorporate all corrections from this errata
   - Revise timeline to 4-5 weeks
   - Update dependency matrix
   - Remove env var feature flags, use persisted config
   - Version 1.1 with corrections

3. **Test Feature Flag Mechanism:**
   - Verify flags work when app launched via Finder
   - Verify flags persist across restarts
   - Verify admin toggle command works
   - Document rollback procedure

### Phase 4 Start Criteria

**DO NOT start Phase 4 until:**
- ✅ Phase 3A fully complete (all repositories)
- ✅ Phase 3B complete (platform adapters)
- ✅ Phase 3C.5-3C.8 complete (calendar)
- ✅ `UserProfileRepository` implemented
- ✅ `FeatureFlagsRepository` implemented and tested
- ✅ Feature flag mechanism verified on macOS GUI app
- ✅ Phase 4 tracking document updated (v1.1)

**Optional for full Phase 4:**
- ⏸️ Phase 3D complete (for Task 4C)
- ⏸️ Phase 3E complete (for Task 4D)

---

**Document Version:** 1.1 (Technical corrections applied)
**Last Updated:** 2025-10-31
**Status:** ✅ **READY FOR REVIEW** - Technical patterns corrected
**Next Review:** After prerequisites implemented
