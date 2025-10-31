# Phase 4 Start Checklist

**Status:** 🔴 **NOT READY** - Prerequisites incomplete
**Last Updated:** 2025-10-31

> **⚠️ CRITICAL:** Phase 4 tracking document contains errors. See [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md) for details.

---

## Quick Status

| Category | Status | Details |
|----------|--------|---------|
| Phase 3 Complete? | 🟡 **PARTIAL** | 3A ✅, 3B ✅, 3C ⚠️ (partial), 3D ❌, 3E ❌ |
| Feature Flags Ready? | ❌ **NO** | Need persisted config (env vars won't work on macOS) |
| Missing Repositories? | ❌ **YES** | `UserProfileRepository` doesn't exist |
| Missing Services? | ⚠️ **SOME** | `CostTracker`, `OutboxWorker`, `TrainingPipeline` (Phase 3D/3E) |
| Can Start Phase 4A? | ⚠️ **PARTIAL** | 2/3 tasks ready (4A.2 blocked) |
| Can Start Phase 4B? | ✅ **YES** | All dependencies met |
| Can Start Phase 4C? | ❌ **NO** | Phase 3D incomplete |
| Can Start Phase 4D? | ❌ **NO** | Phase 3E not started (optional) |
| Can Start Phase 4E? | ✅ **YES** | All dependencies met |

---

## Prerequisites Checklist

### ✅ Complete (Ready)

- [x] Phase 3A: Database repositories
- [x] Phase 3B: Platform adapters
- [x] Phase 3C.5-3C.8: Calendar integration
- [x] Phase 3F: Observability
- [x] `BlockBuilder` exists in core
- [x] Most repositories exist in infra

### ❌ Incomplete (Blocking)

#### **High Priority - Phase 3 Follow-ups**

- [ ] **Create `UserProfileRepository`** (Phase 3A.9.1 follow-up)
  - Port trait in `core/src/user/ports.rs`
  - Implementation in `infra/src/database/user_profile_repository.rs`
  - Estimated: 150 LOC, 3 tests, 2-4 hours
  - **Blocks:** Task 4A.2 (User Profile Commands)

- [ ] **Implement Feature Flags Infrastructure**
  - Add `feature_flags` table to `schema.sql`
  - Bump `SCHEMA_VERSION` in `manager.rs` (from 1 to 2)
  - Create `FeatureFlagsRepository` in infra (use `Arc<DbManager>` + `spawn_blocking`)
  - Add `FeatureFlags` to Tauri app state
  - Create admin toggle command (or document DB update)
  - Estimated: 1-2 days
  - **Blocks:** ALL of Phase 4 (rollback mechanism)

#### **Medium Priority - Phase 3D/3E**

- [ ] **Complete Phase 3D** (Schedulers & Workers)
  - `CostTracker` (Phase 3D.5)
  - `OutboxWorker` (Phase 3D.4)
  - Schedulers (Phase 3D.1-3)
  - **Blocks:** Task 4C (Monitoring Commands)

- [ ] **Complete Phase 3E** (ML Adapters - Optional)
  - `TrainingPipeline` (Phase 3E.3)
  - `TrainingExporter` (Phase 3E.3)
  - **Blocks:** Task 4D (ML Training Commands)

#### **Low Priority - Clarifications**

- [ ] **Decide on IdleDetector approach**
  - Option A: Use legacy idle detection directly (minimal changes)
  - Option B: Create port trait + service in core (cleaner)
  - **Blocks:** Task 4B.3 (Idle Commands) - partial

---

## What You Can Start Now

### ✅ Ready to Start (3 tasks)

**Task 4A.1: Database Commands** (421 LOC)
- Dependencies: ✅ All repositories exist
- Risk: Low
- Estimated: 1 day

**Task 4A.3: Window Commands** (61 LOC)
- Dependencies: ✅ None (UI-only)
- Risk: Very Low
- Estimated: 2 hours

**Task 4E.1: Seed Snapshots** (193 LOC)
- Dependencies: ✅ All repositories exist
- Risk: Very Low
- Estimated: 0.5 day

### ⚠️ Can Start (but need workaround)

**Task 4B.1: Block Commands** (632 LOC)
- Dependencies: ✅ `BlockBuilder` exists, repositories exist
- ⚠️ Feature flags needed (use temporary env var as workaround?)
- Risk: Medium
- Estimated: 2 days

**Task 4B.2: Calendar Commands** (946 LOC)
- Dependencies: ✅ Calendar infrastructure complete
- ⚠️ Feature flags needed (use temporary env var as workaround?)
- Risk: High (OAuth complexity)
- Estimated: 2 days

**Task 4B.3: Idle Commands** (193 LOC)
- Dependencies: ⚠️ IdleDetector approach unclear
- ⚠️ Feature flags needed
- Risk: Low-Medium
- Estimated: 1 day

---

## Recommended Action Plan

### Option 1: Wait for Prerequisites (RECOMMENDED)

**Timeline:** 3-5 days before starting Phase 4

1. **Week 1:** Complete Phase 3 follow-ups
   - Days 1-2: Implement feature flags infrastructure
   - Day 3: Create `UserProfileRepository`
   - Day 4: Test feature flags on macOS GUI app
   - Day 5: Update Phase 4 tracking document (v1.1)

2. **Week 2:** Start Phase 4A (Core Commands)
   - All prerequisites met
   - Feature flags working
   - Clean execution with no blockers

**Pros:**
- ✅ Clean execution, no blockers
- ✅ Feature flags work properly (persisted config)
- ✅ All dependencies resolved
- ✅ Lower risk

**Cons:**
- ⏸️ Delayed start (3-5 days)

---

### Option 2: Start with Ready Tasks (RISKY)

**Timeline:** Start immediately, but limited scope

1. **This Week:** Start 4A.1, 4A.3, 4E.1
   - Only 3/11 tasks
   - Use temporary env var for feature flags (knowing it won't work in production)
   - Skip blocked tasks

2. **Next Week:** Wait for prerequisites, continue with 4B/4C

**Pros:**
- ✅ Immediate progress
- ✅ Low-risk tasks to start

**Cons:**
- ❌ Feature flags won't work on macOS GUI (blocks production rollout)
- ❌ Can't do full Phase 4A (missing 4A.2)
- ❌ Will need to refactor feature flag mechanism later
- ❌ Limited scope (only 3/11 tasks)

---

## Critical Errors to Fix

See [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md) for full details:

### 🔴 Issue 1: Missing Phase 3 Deliverables
- `UserProfileRepository` doesn't exist
- `CostTracker`, `OutboxWorker` don't exist (Phase 3D)
- `TrainingPipeline` doesn't exist (Phase 3E)
- **Impact:** 6/11 tasks blocked or partially blocked

### 🔴 Issue 2: Feature Flags Won't Work
- Environment variables don't work for macOS GUI apps
- Need persisted config (database or state file)
- **Impact:** Entire rollback strategy invalidated

### 🟡 Issue 3: Timeline Conflicts
- Phase 4F scheduled for Days 14-16
- But requires 1-2 week validation period first
- **Impact:** Actual timeline is 4-5 weeks (not 2-3 weeks)

---

## Before You Start Phase 4

### ✅ Verify These Exist

Run these commands to verify dependencies:

```bash
# Check for UserProfileRepository
rg "UserProfileRepository" crates/ --type rust
# Expected: Definitions in core/infra (currently: NONE)

# Check for BlockBuilder (should exist)
rg "pub struct BlockBuilder" crates/core/ --type rust
# Expected: crates/core/src/classification/block_builder.rs

# Check for repositories (should exist)
ls crates/infra/src/database/*_repository.rs
# Expected: 9-10 repository files

# Check for feature flags table (should NOT exist yet)
rg "feature_flags" crates/infra/src/database/ --type rust
# Expected: NONE (needs to be added)

# Check for Phase 3D/3E modules (should NOT exist yet)
rg "CostTracker|OutboxWorker|TrainingPipeline" crates/ --type rust
# Expected: NONE (Phase 3D/3E incomplete)
```

### ✅ Implement Prerequisites

1. **Feature Flags Infrastructure (1-2 days):**
   ```bash
   # Step 1: Add table to schema
   # Edit crates/infra/src/database/schema.sql
   # Add CREATE TABLE feature_flags (...)

   # Step 2: Bump schema version
   # Edit crates/infra/src/database/manager.rs
   # Change: const SCHEMA_VERSION: i32 = 1;
   # To:     const SCHEMA_VERSION: i32 = 2;

   # Step 3: Create repository (following existing pattern)
   touch crates/infra/src/database/feature_flags_repository.rs
   # Use Arc<DbManager>, call get_connection(), use spawn_blocking
   # See docs/PHASE-4-ERRATA.md lines 299-372 for full example

   # Step 4: Update Tauri state
   # Edit legacy/api/src/main.rs
   # Add FeatureFlags to AppState
   ```

2. **UserProfileRepository (2-4 hours):**
   ```bash
   # Create port trait
   touch crates/core/src/user/ports.rs

   # Create implementation
   touch crates/infra/src/database/user_profile_repository.rs

   # Add tests
   # Edit crates/infra/src/database/mod.rs
   ```

3. **Test on macOS GUI:**
   ```bash
   # Build release binary
   cargo build --release

   # Copy to Applications
   # Launch via Finder (NOT terminal)
   # Verify feature flags work
   ```

---

## Summary

**Current Status:** ❌ **NOT READY**

**Blockers:**
1. Feature flags infrastructure missing (HIGH)
2. `UserProfileRepository` missing (HIGH)
3. Phase 3D/3E incomplete (MEDIUM - blocks 4C/4D only)

**Recommendation:** **Wait 3-5 days** for prerequisites, then start Phase 4 cleanly.

**Quick Wins Available:** Tasks 4A.1, 4A.3, 4E.1 (3/11 tasks, low risk)

---

**Next Steps:**
1. Read [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md) for full details
2. Implement feature flags infrastructure (1-2 days)
3. Create `UserProfileRepository` (2-4 hours)
4. Update Phase 4 tracking document (v1.1)
5. Start Phase 4A when ready

---

**Document Version:** 1.1 (Technical corrections applied)
**Last Updated:** 2025-10-31
**Status:** 🔴 Prerequisites incomplete (see [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md) v1.1)
