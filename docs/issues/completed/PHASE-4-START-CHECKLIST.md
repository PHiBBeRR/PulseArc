# ⚠️ DEPRECATED: Phase 4 Start Checklist

> **🚨 THIS DOCUMENT IS DEPRECATED**
>
> **New Plan:** Phase 4 has been redesigned to migrate commands to `crates/api/pulsearc-app` instead of rewiring in `legacy/api/`.
>
> **See:** [PHASE-4-NEW-CRATE-MIGRATION.md](../../PHASE-4-NEW-CRATE-MIGRATION.md) for the current migration plan (includes updated checklist).
>
> **Reason:** This checklist was for the original Phase 4 plan (in-place rewiring). The new plan uses a different approach.
>
> **Date Deprecated:** 2025-10-31

---

# Phase 4 Start Checklist (ORIGINAL PLAN - DEPRECATED)

**Status:** 🔴 DEPRECATED - See new plan above
**Last Updated:** 2025-10-31 (All prerequisites complete, Feature Flags infrastructure implemented)

> **✅ UPDATE (2025-10-31 PM):** Feature flags infrastructure is now complete! All prerequisites met. Ready to start Phase 4A. (NOTE: This refers to the OLD Phase 4 plan)

---

## Quick Status

| Category | Status | Details |
|----------|--------|---------|
| Phase 3 Complete? | ✅ **YES (except 3E)** | 3A ✅, 3B ✅, 3C ✅, 3D ✅, 3E ❌ (skipping) |
| Schema Migrations? | ✅ **YES** | ✅ feature_flags, idle_periods, user_profiles tables added, SCHEMA_VERSION = 1 (schema evolution) |
| Feature Flags Ready? | ✅ **YES** | ✅ Repository, Service, AppState wiring, Tauri commands - ALL COMPLETE |
| Missing Repositories? | ✅ **NO** | ✅ All repos exist! UserProfileRepository + IdlePeriodsRepository added |
| Missing Services? | ✅ **NO** | ✅ `CostTracker`, ✅ `OutboxWorker`, ✅ `FeatureFlagService` exist! |
| Can Start Phase 4A? | ✅ **YES** | ✅ All 3 tasks ready (UserProfileRepository complete!) |
| Can Start Phase 4B? | ✅ **YES** | ✅ All dependencies met (IdlePeriodsRepository complete!) |
| Can Start Phase 4C? | ✅ **YES** | **Phase 3D complete!** (OutboxWorker + CostTracker verified) |
| Can Start Phase 4D? | ❌ **SKIP** | Phase 3E not started (skipping ML features) |
| Can Start Phase 4E? | ✅ **YES** | All repositories exist |

---

## Prerequisites Checklist

### ✅ Complete (Ready)

- [x] **Phase 3A: Database repositories (ALL COMPLETE - 11/11)**
  - [x] UserProfileRepository added (2025-10-31) - 7/7 tests passing
  - [x] IdlePeriodsRepository added (2025-10-31) - 6/6 tests passing
- [x] Phase 3B: Platform adapters
- [x] Phase 3C: Calendar integration (complete)
- [x] **Phase 3D: Schedulers & Workers (VERIFIED COMPLETE!)**
  - [x] `OutboxWorker` exists at `crates/infra/src/sync/outbox_worker.rs`
  - [x] `CostTracker` exists at `crates/infra/src/sync/cost_tracker.rs`
  - [x] Phase 3D marked complete in tracking doc (Oct 31, 2025)
- [x] Phase 3F: Observability
- [x] `BlockBuilder` exists in core
- [x] **Schema Migrations (COMPLETE - 2025-10-31)**
  - [x] `feature_flags` table added to schema.sql (lines 408-418)
  - [x] `idle_periods` table added to schema.sql (lines 391-407)
  - [x] `user_profiles` table added to schema.sql (lines 419-448)
  - [x] `SCHEMA_VERSION` kept at 1 (schema evolution within v1, per Option A)
- [x] **Feature Flags Infrastructure (COMPLETE - 2025-10-31 PM)**
  - [x] ✅ Port trait: `FeatureFlagsPort` in core/feature_flags_ports.rs
  - [x] ✅ Repository: `SqlCipherFeatureFlagsRepository` in infra/database (320 LOC, 5 tests)
  - [x] ✅ Service: `FeatureFlagService` with caching in infra/services (290 LOC, 4 tests)
  - [x] ✅ Tauri commands: 4 commands in legacy/api/src/commands/feature_flags.rs
  - [x] ✅ AppState wiring: Initialized in main.rs, added to AppState
  - [x] ✅ All tests passing (9/9), CI passed
  - **Total:** ~750 LOC across 7 new files + 5 modified files

### ⚠️ Incomplete (Remaining)

#### **Low Priority - Manual GUI Testing**

- [ ] **Test Feature Flags in macOS GUI**
  - Manual testing recommended before Phase 4 start
  - Test persistence across app restarts
  - Verify rollback time <2 minutes
  - Estimated: 30 minutes
  - **Note:** Not blocking - infrastructure is production-ready

#### **Low Priority - Optional ML Features**

- [ ] **Skip Phase 3E** (ML Adapters - Intentionally Not Implementing)
  - `TrainingPipeline` (Phase 3E.3) - not needed
  - `TrainingExporter` (Phase 3E.3) - not needed
  - **Impact:** Task 4D (ML Training Commands) will be skipped
  - **Note:** ML commands in [legacy/api/src/commands/ml_training.rs](legacy/api/src/commands/ml_training.rs) (242 LOC) can remain using legacy ML or be disabled
  - **Decision:** Skip Phase 4D entirely - ML features are optional and feature-gated

---

## What You Can Start Now

### ✅ Ready to Start (10/11 tasks - Almost All of Phase 4!)

**Phase 4A Tasks (ALL 3 READY!):**

**Task 4A.1: Database Commands** (421 LOC)
- Dependencies: ✅ All repositories exist
- Risk: Low
- Estimated: 1 day

**Task 4A.2: User Profile Commands** (49 LOC)
- Dependencies: ✅ UserProfileRepository implemented! (2025-10-31)
- Risk: Very Low
- Estimated: 0.5 day

**Task 4A.3: Window Commands** (61 LOC)
- Dependencies: ✅ None (UI-only)
- Risk: Very Low
- Estimated: 2 hours

**Phase 4C Tasks (BOTH ready - Phase 3D complete!):**

**Task 4C.1: Monitoring Commands** (741 LOC)
- Dependencies: ✅ OutboxWorker, CostTracker, OutboxRepository all exist!
- Risk: Low (read-only monitoring)
- Estimated: 2 days

**Task 4C.2: Idle Sync Commands** (58 LOC)
- Dependencies: ✅ Sync infrastructure complete
- Risk: Very Low (telemetry only)
- Estimated: 0.5 day

**Phase 4E Task:**

**Task 4E.1: Seed Snapshots** (193 LOC)
- Dependencies: ✅ All repositories exist
- Risk: Very Low (dev tool only)
- Estimated: 0.5 day

**Phase 4B Tasks (all 3 ready with workaround):**

**Task 4B.1: Block Commands** (632 LOC)
- Dependencies: ✅ `BlockBuilder` exists, repositories exist
- ⚠️ **Feature flags needed** (use temporary env var as workaround - won't work in production GUI)
- Risk: Medium (complex business logic)
- Estimated: 2 days

**Task 4B.2: Calendar Commands** (946 LOC)
- Dependencies: ✅ Calendar infrastructure complete (Phase 3C)
- ⚠️ **Feature flags needed** (use temporary env var as workaround - won't work in production GUI)
- Risk: High (OAuth complexity)
- Estimated: 2 days

**Task 4B.3: Idle Commands** (193 LOC)
- Dependencies: ✅ IdlePeriodsRepository implemented! (2025-10-31)
- Note: Idle detection stays in legacy (only CRUD operations migrated)
- ⚠️ **Feature flags needed** (use temporary env var as workaround - won't work in production GUI)
- Risk: Low
- Estimated: 1 day

### ❌ Blocked Tasks (1 task - ML only)

**Task 4D.1: ML Training Commands** (242 LOC)
- ❌ **Blocked:** Phase 3E not implemented (intentionally skipped)
- **Decision:** Skip this task entirely - ML features are optional
- Alternative: Leave using legacy ML or disable feature

---

## Recommended Action Plan

### ✅ Prerequisites Status (Updated 2025-10-31)

**Completed:**
- ✅ Schema Migrations (feature_flags, idle_periods, user_profiles tables)
- ✅ UserProfileRepository (7/7 tests passing)
- ✅ IdlePeriodsRepository (6/6 tests passing)

**Remaining:**
- ⚠️ Feature Flags AppState wiring (2-4 hours)

### Option 1: Complete Feature Flags, Then Start (RECOMMENDED)

**Timeline:** 2-4 hours before starting Phase 4

1. **Today:** Complete Feature Flags wiring
   - Wire `FeatureFlagService` into Tauri AppState
   - Create admin toggle command or document SQL
   - Test feature flags on macOS GUI app

2. **Start Phase 4 immediately after**
   - All prerequisites met
   - Feature flags working for emergency rollback
   - Clean execution with no blockers

**Pros:**
- ✅ Clean execution, no blockers
- ✅ Feature flags work properly (persisted config)
- ✅ All dependencies resolved
- ✅ Lower risk
- ✅ Emergency rollback capability

**Cons:**
- ⏸️ Minimal delay (2-4 hours)

---

### Option 2: Start Immediately Without Feature Flags (ACCEPTABLE)

**Timeline:** Start immediately with 10/11 tasks

1. **This Week:** Start ready tasks without feature flags wired
   - **6 tasks safe to start:** 4A.1, 4A.2, 4A.3, 4C.1, 4C.2, 4E.1 (no runtime feature flags needed for dev/testing)
   - **3 tasks with workaround:** 4B.1, 4B.2, 4B.3 (use temp env var for dev, defer production rollback capability)
   - Total: **10/11 tasks** ready! (only 4D.1 skipped - ML)
   - Note: All prerequisites complete except AppState wiring

2. **Add feature flags later:** Wire AppState when needed for production rollback

**Pros:**
- ✅ **Immediate progress on almost all of Phase 4** (10/11 tasks!)
- ✅ All repositories complete - no CRUD blockers
- ✅ Phase 4A, 4C, 4E fully unblocked
- ✅ Can start most work immediately

**Cons:**
- ⚠️ Feature flags won't work in production GUI (limits rollback options)
- ⚠️ Will need to wire AppState later before production deployment
- ⚠️ Temporary env var workaround for 4B tasks during development

---

## Critical Findings (Updated 2025-10-31)

> **✅ UPDATE:** Prerequisites 1, 3, and 4 are now complete! Only Feature Flags AppState wiring remains.

### ✅ Issue 1 FULLY RESOLVED: All Prerequisites Complete!
- ✅ `CostTracker` EXISTS at [crates/infra/src/sync/cost_tracker.rs](../crates/infra/src/sync/cost_tracker.rs)
- ✅ `OutboxWorker` EXISTS at [crates/infra/src/sync/outbox_worker.rs](../crates/infra/src/sync/outbox_worker.rs)
- ✅ Phase 3D marked complete in tracking doc (Oct 31, 2025)
- ✅ `UserProfileRepository` IMPLEMENTED (2025-10-31) - 7/7 tests passing
- ✅ `IdlePeriodsRepository` IMPLEMENTED (2025-10-31) - 6/6 tests passing
- ✅ Schema migrations COMPLETE (feature_flags, idle_periods, user_profiles tables added)
- ❌ `TrainingPipeline` doesn't exist (Phase 3E - intentionally skipped)
- **Impact:** Only 1/11 tasks blocked (not 6/11):
  - 4D.1 (ML Training) - blocked by skipped Phase 3E (acceptable, feature-gated)

### 🟡 Issue 2: Feature Flags Partially Complete
- ✅ `FeatureFlagsRepository` implemented (database persistence ready)
- ⚠️ Need to wire `FeatureFlagService` into Tauri AppState
- ⚠️ Need admin toggle command or SQL documentation
- **Impact:** Can start Phase 4, but production rollback capability limited until wiring complete

### 🟡 Issue 3: Timeline Conflicts
- Phase 4F scheduled for Days 14-16
- But requires 1-2 week validation period first
- **Impact:** Actual timeline is 4-5 weeks (not 2-3 weeks)

---

## Before You Start Phase 4

### ✅ Verify These Exist

Run these commands to verify dependencies:

```bash
# Check for UserProfileRepository (NOW EXISTS!)
rg "UserProfileRepository" crates/ --type rust
# Expected: ✅ Definitions in core/infra

# Check for IdlePeriodsRepository (NOW EXISTS!)
rg "IdlePeriodsRepository" crates/ --type rust
# Expected: ✅ Definitions in core/infra

# Check for BlockBuilder (should exist)
rg "pub struct BlockBuilder" crates/core/ --type rust
# Expected: ✅ crates/core/src/classification/block_builder.rs

# Check for repositories (ALL 11 EXIST NOW!)
ls crates/infra/src/database/*_repository.rs
# Expected: ✅ 11 files including user_profile_repository.rs and idle_periods_repository.rs

# Check for feature flags table (NOW EXISTS!)
rg "feature_flags" crates/infra/src/database/schema.sql
# Expected: ✅ Table definition exists

# ✅ Verify Phase 3D modules exist (should exist!)
rg "CostTracker|OutboxWorker" crates/infra/src/sync/ --type rust
# Expected: FOUND in cost_tracker.rs and outbox_worker.rs

# Check for Phase 3E modules (should NOT exist - intentionally skipped)
ls crates/infra/src/ml/ 2>/dev/null || echo "ML directory doesn't exist (expected - Phase 3E skipped)"
# Expected: Directory not found (Phase 3E intentionally skipped)
```

### ✅ Implement Prerequisites

1. **Schema Migrations (30 minutes):**
   ```bash
   # Step 1: Add tables to schema
   # Edit crates/infra/src/database/schema.sql
   # Add:
   # - CREATE TABLE feature_flags (
   #     flag_name TEXT PRIMARY KEY,
   #     enabled BOOLEAN NOT NULL DEFAULT 0,
   #     description TEXT,
   #     updated_at INTEGER NOT NULL
   # );
   #
   # - CREATE TABLE idle_periods (
   #     id TEXT NOT NULL PRIMARY KEY,
   #     start_ts INTEGER NOT NULL,
   #     end_ts INTEGER NOT NULL,
   #     duration_secs INTEGER NOT NULL,
   #     system_trigger TEXT NOT NULL,
   #     user_action TEXT,
   #     threshold_secs INTEGER NOT NULL,
   #     created_at INTEGER NOT NULL,
   #     reviewed_at INTEGER,
   #     notes TEXT,
   #     UNIQUE(start_ts, end_ts)
   # );
   #
   # - CREATE INDEX idx_idle_periods_time_range
   #     ON idle_periods(start_ts, end_ts);
   # - CREATE INDEX idx_idle_periods_user_action
   #     ON idle_periods(user_action, start_ts);

   # Step 2: Bump schema version
   # Edit crates/infra/src/database/manager.rs
   # Change: const SCHEMA_VERSION: i32 = 1;
   # To:     const SCHEMA_VERSION: i32 = 3;

   # Step 3: Test migration
   cargo test -p pulsearc-infra database::manager::test_migrations
   ```

2. **Feature Flags Infrastructure (1-2 days):**
   ```bash
   # Step 1: Create repository (following existing pattern)
   touch crates/infra/src/database/feature_flags_repository.rs
   # Use Arc<DbManager>, call get_connection(), use spawn_blocking
   # See docs/PHASE-4-ERRATA.md lines 299-372 for full example

   # Step 2: Update Tauri state
   # Edit legacy/api/src/main.rs
   # Add FeatureFlags to AppState
   ```

3. **UserProfileRepository (2-4 hours):**
   ```bash
   # Create port trait
   touch crates/core/src/user/ports.rs

   # Create implementation
   touch crates/infra/src/database/user_profile_repository.rs

   # Add tests
   # Edit crates/infra/src/database/mod.rs
   ```

4. **IdlePeriodsRepository (2-3 hours):**
   ```bash
   # Add port trait to existing file
   # Edit crates/core/src/tracking/ports.rs
   # Add trait:
   # pub trait IdlePeriodsRepository: Send + Sync {
   #     async fn query_range(&self, start_ts: i64, end_ts: i64) -> Result<Vec<IdlePeriod>>;
   #     async fn update_action(&self, period_id: &str, action: &str, notes: Option<String>) -> Result<()>;
   #     async fn get_summary(&self, start_ts: i64, end_ts: i64) -> Result<IdleSummary>;
   # }

   # Create implementation
   touch crates/infra/src/database/idle_periods_repository.rs
   # Follow pattern from activity_repository.rs (Arc<DbManager>, spawn_blocking)

   # Add tests
   # Edit crates/infra/src/database/mod.rs
   ```

5. **Test on macOS GUI:**
   ```bash
   # Build release binary
   cargo build --release

   # Copy to Applications
   # Launch via Finder (NOT terminal)
   # Verify feature flags work
   ```

---

## Summary

**Current Status:** 🟢 **READY** - All prerequisites complete!

**Actual Readiness:**
- ✅ **10/11 tasks ready** - ALL non-ML tasks ready to start!
  - Phase 4A: 3/3 tasks ready (4A.1, 4A.2, 4A.3)
  - Phase 4B: 3/3 tasks ready (4B.1, 4B.2, 4B.3)
  - Phase 4C: 2/2 tasks ready (4C.1, 4C.2)
  - Phase 4E: 1/1 task ready (4E.1)
- ⏸️ **1/11 task skipped** by Phase 3E decision (4D.1 - ML features)

**Prerequisites Completed:**
1. ✅ **Schema migrations (COMPLETE)** - feature_flags, idle_periods, user_profiles tables added
2. ✅ **Feature flags infrastructure (COMPLETE)** - Full rollback capability operational
3. ✅ **UserProfileRepository (COMPLETE)** - 7/7 tests passing
4. ✅ **IdlePeriodsRepository (COMPLETE)** - 6/6 tests passing

**Infrastructure Stats:**
- ✅ Phase 3D: 5,423 LOC, 281 tests passing
- ✅ Feature Flags: ~750 LOC, 9 tests passing, CI green
- ✅ Total: 11/11 repositories implemented

**Recommendation:** ✅ **Start Phase 4A immediately** - all prerequisites met!

**Production Rollback Ready:**
- Feature flags persist across restarts
- Toggle via Tauri commands or SQL
- Target rollback time: <2 minutes

---

**Next Steps:**
1. ✅ **Optional:** Manual GUI testing of feature flags (30 min)
   - Test from frontend console
   - Verify persistence across restarts
   - Confirm rollback time <2 minutes
2. 🚀 **Start Phase 4A:** Database Commands (Task 4A.1 recommended first)
   - All dependencies met
   - Estimated: 3-4 days for all 3 tasks
3. 📋 **Reference:** See [PHASE-4-API-REWIRING-TRACKING.md](./PHASE-4-API-REWIRING-TRACKING.md) for detailed task breakdown

---

**Document Version:** 2.0 (All Prerequisites Complete!)
**Last Updated:** 2025-10-31 PM
**Status:** 🟢 **READY** - All 10 non-ML tasks ready to start!
**Verified Files:**
- ✅ [crates/infra/src/sync/outbox_worker.rs](../crates/infra/src/sync/outbox_worker.rs) (exists)
- ✅ [crates/infra/src/sync/cost_tracker.rs](../crates/infra/src/sync/cost_tracker.rs) (exists)
- ✅ [crates/infra/src/database/user_profile_repository.rs](../crates/infra/src/database/user_profile_repository.rs) (exists, 7 tests)
- ✅ [crates/infra/src/database/idle_periods_repository.rs](../crates/infra/src/database/idle_periods_repository.rs) (exists, 6 tests)
- ✅ [crates/infra/src/services/feature_flag_service.rs](../crates/infra/src/services/feature_flag_service.rs) (exists, 4 tests)
- ✅ Feature flags infrastructure complete (~750 LOC, 9 tests passing)
