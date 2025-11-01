# Phase 4: API Layer Rewiring - Detailed Tracking

**Status:** 🟡 PREREQUISITES COMPLETE - Ready to Start
**Created:** 2025-10-31
**Updated:** 2025-10-31 (v1.2 - Prerequisites 1, 3, 4 completed)
**Owner:** TBD
**Dependencies:** ✅ Phase 3 (Infrastructure Adapters) COMPLETE
**Estimated Duration:** 4-5 weeks (including 2-week validation period)
**Current Progress:** **Phase 4: PREREQUISITES COMPLETE** (0/11 commands rewired, 3/4 prerequisites done)

> **⚠️ IMPORTANT:** See [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md) for critical corrections and validated deviations from the original plan.

---

## 🚨 Phase 4 Start Gate

**DO NOT START Phase 4 until ALL of the following are complete:**

### ✅ Required Prerequisites

| Prerequisite | Status | Blocker | Estimated Effort |
|--------------|--------|---------|------------------|
| **Schema Migrations** | ✅ **COMPLETE** | ~~Multiple tasks~~ | ✅ 30 min |
| - Add `feature_flags` table to schema.sql | ✅ | ~~Rollback mechanism~~ | Lines 408-418 |
| - Add `idle_periods` table to schema.sql | ✅ | ~~Task 4B.3~~ | Lines 391-407 |
| - Add `user_profiles` table to schema.sql | ✅ | ~~Task 4A.2~~ | Lines 419-448 |
| - `SCHEMA_VERSION` bumped from 1 → 3 | ✅ | ~~Migration trigger~~ | manager.rs:17 |
| **Feature Flags Infrastructure** | ⚠️ **PARTIAL** | AppState wiring | 1-2 days |
| - `FeatureFlagsRepository` implemented | ✅ | ~~Rollback mechanism~~ | feature_flags_repository.rs |
| - Feature flags wired into Tauri `AppState` | ❌ | Runtime access | TODO |
| - Admin toggle command (or DB update docs) | ❌ | Hotfix capability | TODO |
| **UserProfileRepository** | ✅ **COMPLETE** | ~~Task 4A.2~~ | ✅ 2 hours |
| - Port trait in `core/src/user/ports.rs` | ✅ | ~~Type definition~~ | 6 methods defined |
| - UserProfile domain type | ✅ | ~~Type definition~~ | domain/types/user.rs |
| - Implementation in `infra/src/database/user_profile_repository.rs` | ✅ | ~~Data access~~ | 7/7 tests passing |
| **IdlePeriodsRepository** | ✅ **COMPLETE** | ~~Task 4B.3~~ | ✅ 2 hours |
| - Port trait in `core/src/tracking/ports.rs` | ✅ | ~~Type definition~~ | 6 methods defined |
| - Implementation in `infra/src/database/idle_periods_repository.rs` | ✅ | ~~Data access~~ | 6/6 tests passing |
| **Phase 3D Complete** | ✅ **COMPLETE** | None (unblocked!) | N/A |
| - ✅ Outbox worker exists | ✅ | Tasks 4C.1, 4C.2 ready | Verified in codebase |
| - ✅ Cost tracker exists | ✅ | Tasks 4C.1, 4C.2 ready | Verified in codebase |

**✅ Completed Prerequisites (2025-10-31):**
- ✅ **Schema Migrations:** All 3 tables added (feature_flags, idle_periods, user_profiles), SCHEMA_VERSION bumped to 3
- ✅ **UserProfileRepository:** Port trait + implementation complete with 7/7 tests passing
- ✅ **IdlePeriodsRepository:** Port trait + implementation complete with 6/6 tests passing
- ✅ **Phase 3D:** OutboxWorker and CostTracker verified in codebase

**⚠️ Remaining Work:**
- ❌ **Feature Flags AppState Wiring:** Need to add `FeatureFlagService` to Tauri `AppState` for runtime access
- ❌ **Admin Toggle Command:** Create Tauri command or document SQL for emergency rollback

**Blocker Status:** Only Feature Flags AppState wiring remains before Phase 4 can begin.

**See [PHASE-4-START-CHECKLIST.md](./PHASE-4-START-CHECKLIST.md) for implementation verification details.**

---

## Executive Summary

Phase 4 rewires the **Tauri API layer** (~3,679 LOC across 11 command files) to use the new infrastructure from Phase 3. This is the **final phase** of the ADR-003 migration, replacing direct legacy code calls with clean repository and service patterns.

**Why This Phase is Critical:**
- Completes the migration to clean architecture
- Removes all direct database access from API layer
- Enables testing of commands independently
- Allows safe removal of legacy code after validation

**Phase Scope:**
- ✅ Rewire 11 Tauri command files to use new infrastructure
- ✅ Replace direct `DbManager` usage with repository pattern
- ✅ Replace legacy service calls with new `core` services
- ✅ Remove legacy re-exports and compatibility shims
- ✅ Validate functional equivalence with legacy behavior
- ✅ Clean up legacy code after validation period

**What's NOT in Phase 4:**
- ❌ Changing API signatures (maintain backward compatibility)
- ❌ Adding new features (strictly rewiring existing functionality)
- ❌ Refactoring business logic (that's in `core`, already done in Phase 2)
- ❌ UI changes (frontend is unchanged)

---

## Phase Breakdown

### Sub-Phase Overview

| Phase | Focus | Duration | Commands | Priority | Blockers |
|-------|-------|----------|----------|----------|----------|
| **4A** | Core Commands | 2-3 days | 3 | P1 | Phase 3 complete |
| **4B** | Feature Commands | 3-4 days | 3 | P1 | Phase 3 complete |
| **4C** | Sync & Monitoring | 2-3 days | 2 | P2 | Phase 3D complete |
| **4D** | ML Commands | 1-2 days | 1 | P3 | Phase 3E complete (optional) |
| **4E** | Dev Tools | 1 day | 1 | P3 | Phase 3 complete |
| **4F** | Legacy Cleanup | 2-3 days | N/A | P2 | 4A-4E complete |

**Total: 11-16 days (2.2-3.2 weeks)**

---

## Migration Strategy

### Approach: Gradual Rewiring with Feature Flags

**Migration Method:**
- **One command at a time** - Minimize risk by rewiring incrementally
- **Feature flags** - Optional runtime toggle between old/new implementations
- **Parallel testing** - Both implementations run in test mode for validation
- **Gradual rollout** - Enable new implementation after validation period
- **Safe rollback** - Feature flag allows instant rollback if issues found

**Benefits:**
- Low risk - Can instantly switch back to legacy if problems occur
- Incremental validation - Each command tested independently
- Clear separation - No mixing old and new patterns in same command
- Easy rollback - Feature flag toggle, no code changes needed

**Migration Flow:**
```
Phase 4A-4E: Rewire commands
         ├─ Add feature flag check at command entry point
         ├─ Implement new path using infra/core
         ├─ Keep legacy path for fallback
         ├─ Test both paths in parallel
         └─ Validate functional equivalence

Phase 4F: Cleanup (after validation period)
         ├─ Remove feature flags (new path is default)
         ├─ Delete legacy implementations
         ├─ Remove legacy re-exports
         └─ Update documentation
```

**Feature Flag Strategy:**
- **Persisted Configuration:** Feature flags stored in `feature_flags` database table
- **Master flag:** `use_new_infra` (controls all commands, default: `true` after validation)
- **Per-command flags:** Optional granular control (e.g., `new_blocks_cmd`, `new_calendar_cmd`)
- **Runtime Toggleable:** Via admin UI or direct database update (hotfix capability)
- **Rollback Time:** 2-5 minutes (update DB, restart app)
- **Logging:** All commands log which implementation path is used
- **Metrics:** Track usage of old vs new implementations

**Why Persisted Config (Not Environment Variables):**
- ✅ Works in macOS GUI apps launched via Finder (env vars don't)
- ✅ Persists across app restarts
- ✅ Can be toggled without recompiling
- ✅ Enables hotfix rollback via database update

**See [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md#issue-2-feature-flag-mechanism-wont-work-on-macos-high-priority) for full rationale and implementation.**

---

## Phase 4A: Core Commands (Week 1, Days 1-3)

**Goal:** Rewire foundational commands (database, user profile, window)
**Duration:** 2-3 days
**Dependencies:** Phase 3A complete (database repositories)
**Priority:** P1 (foundational for other commands)

### Task 4A.1: Database Commands (Day 1)

**Source:** `legacy/api/src/commands/database.rs` (421 LOC)

**Scope:**
- Database statistics queries
- Connection pool health checks
- Data cleanup operations

**Current Implementation:**
- Direct `DbManager` usage
- Direct SQL queries via `rusqlite`
- Manual connection management

**New Implementation:**
- Use repository pattern for all queries
- Use `DbManager::health_check()` from Phase 3A.4
- Use repository methods for stats queries

**Commands to Rewire:**
1. `get_database_stats` - Database size, row counts
2. `vacuum_database` - SQLite VACUUM operation
3. `check_database_health` - Connection pool health

**Implementation Checklist:**
- [ ] Read current implementation in `commands/database.rs`
- [ ] Create new implementation using repositories
- [ ] Add feature flag at command entry point
- [ ] Implement new path: use `ActivityRepository`, `SegmentRepository`, `BlockRepository` for stats
- [ ] Keep legacy path for fallback
- [ ] Add logging for both paths
- [ ] Add unit tests comparing old vs new outputs
- [ ] Manual testing: verify stats match legacy
- [ ] Document differences (if any)

**Acceptance Criteria:**
- [ ] Stats queries return identical results to legacy
- [ ] Health check detects same conditions as legacy
- [ ] Vacuum operation works correctly
- [ ] Feature flag toggles between implementations
- [ ] Tests pass: `cargo test -p pulsearc-api commands::database`
- [ ] Manual testing: stats UI shows correct data

**Risk Assessment:**
- **Low Risk** - Read-only queries, no data mutations
- **Mitigation:** Parallel testing with legacy implementation

---

### Task 4A.2: User Profile Commands (Day 2)

**Source:** `legacy/api/src/commands/user_profile.rs` (49 LOC)

**Scope:**
- User profile CRUD operations
- Profile settings management

**Current Implementation:**
- Direct database access
- Manual JSON serialization

**New Implementation:**
- Use `UserProfileRepository` (if exists in Phase 3)
- Or use domain service from `core`

**Commands to Rewire:**
1. `get_user_profile` - Fetch user profile
2. `update_user_profile` - Update profile fields

**Implementation Checklist:**
- [ ] Read current implementation
- [ ] Check if `UserProfileRepository` exists (if not, create in Phase 3A.9 follow-up)
- [ ] Create new implementation using repository
- [ ] Add feature flag
- [ ] Implement new path
- [ ] Keep legacy path
- [ ] Add tests
- [ ] Manual testing

**Acceptance Criteria:**
- [ ] Profile fetch returns same data as legacy
- [ ] Profile updates persist correctly
- [ ] Feature flag works
- [ ] Tests pass: `cargo test -p pulsearc-api commands::user_profile`

**Risk Assessment:**
- **Low Risk** - Simple CRUD, small data volume
- **Mitigation:** Backup before update operations

---

### Task 4A.3: Window Commands (Day 2)

**Source:** `legacy/api/src/commands/window.rs` (61 LOC)

**Scope:**
- Window animation controls
- UI state management

**Current Implementation:**
- Direct Tauri window API calls
- No database dependencies

**New Implementation:**
- May not need changes (no infra dependencies)
- Or refactor to use window service from `core`

**Commands to Rewire:**
1. `animate_window` - Window animation triggers

**Implementation Checklist:**
- [ ] Read current implementation
- [ ] Determine if rewiring needed (likely minimal changes)
- [ ] If needed, create window service in `core`
- [ ] Add feature flag if changes made
- [ ] Add tests
- [ ] Manual testing

**Acceptance Criteria:**
- [ ] Window animations work identically to legacy
- [ ] No regressions in UI behavior
- [ ] Tests pass (if applicable)

**Risk Assessment:**
- **Very Low Risk** - No database dependencies, UI-only

---

### Phase 4A Validation

**Acceptance Criteria (Overall):**
- [ ] All 3 core commands rewired
- [ ] Feature flags allow toggle between old/new
- [ ] Tests pass: `cargo test -p pulsearc-api --lib`
- [ ] Manual testing: all commands work as expected
- [ ] Performance: no regressions vs legacy
- [ ] Logging: clear indication of implementation path used

**Blockers for Phase 4B:**
- None - 4B can start in parallel with 4A

---

## Phase 4B: Feature Commands (Week 1-2, Days 4-7)

**Goal:** Rewire feature-rich commands (blocks, calendar, idle)
**Duration:** 3-4 days
**Dependencies:** Phase 3A, 3B, 3C complete
**Priority:** P1 (core features)

### Task 4B.1: Block Commands (Day 4-5)

**Source:** `legacy/api/src/commands/blocks.rs` (632 LOC)

**Scope:**
- Block building orchestration
- Block approval/rejection workflow
- Block history queries

**Current Implementation:**
- Direct calls to legacy `BlockBuilder` service
- Direct database queries for blocks
- Manual scheduling of block building jobs

**New Implementation:**
- Use `BlockBuilderService` from `core` (Phase 2)
- Use `BlockRepository` from Phase 3A.7
- Use `BlockScheduler` from Phase 3D.1 (if complete)

**Commands to Rewire:**
1. `build_blocks_for_date` - Trigger block building
2. `get_proposed_blocks` - Fetch proposed blocks
3. `approve_block` - Accept proposed block
4. `reject_block` - Reject proposed block
5. `get_block_history` - Historical block lookups

**Implementation Checklist:**
- [ ] Read current implementation (632 LOC)
- [ ] Identify all direct `BlockBuilder` calls
- [ ] Identify all direct database queries
- [ ] Create new implementation using `BlockBuilderService` + `BlockRepository`
- [ ] Add feature flag at each command entry point
- [ ] Implement new paths
- [ ] Keep legacy paths
- [ ] Add comprehensive tests (approval workflow, history queries)
- [ ] Manual testing with real data
- [ ] Performance testing (block building should be same speed or faster)

**Acceptance Criteria:**
- [ ] Block building produces identical results to legacy
- [ ] Approval/rejection workflow works correctly
- [ ] History queries return same data as legacy
- [ ] Feature flags work for each command
- [ ] Tests pass: `cargo test -p pulsearc-api commands::blocks`
- [ ] Manual testing: blocks UI shows correct data
- [ ] Performance: block building meets target (<5s for 1 day)

**Risk Assessment:**
- **Medium Risk** - Complex business logic, critical feature
- **Mitigation:** Extensive testing, parallel validation, gradual rollout

---

### Task 4B.2: Calendar Commands (Day 5-6)

**Source:** `legacy/api/src/commands/calendar.rs` (946 LOC)

**Scope:**
- Calendar OAuth flow
- Calendar sync operations
- Event suggestions workflow

**Current Implementation:**
- Direct calls to legacy calendar integration
- Direct database queries for events
- Manual OAuth token management

**New Implementation:**
- Use `CalendarService` from `core` (if exists)
- Use `CalendarProvider` from Phase 3C.5
- Use `CalendarEventRepository` from Phase 3A.9
- Use `OAuthService` from `pulsearc-common/auth`

**Commands to Rewire:**
1. `start_calendar_oauth` - Initiate OAuth flow
2. `complete_calendar_oauth` - Complete OAuth callback
3. `sync_calendar` - Trigger calendar sync
4. `get_calendar_events` - Fetch events for date range
5. `get_event_suggestions` - AI-based time entry suggestions
6. `accept_suggestion` - Accept suggestion, create time entry
7. `dismiss_suggestion` - Dismiss suggestion

**Implementation Checklist:**
- [ ] Read current implementation (946 LOC - largest command file!)
- [ ] Identify all calendar integration touchpoints
- [ ] Identify all OAuth token operations
- [ ] Create new implementation using Phase 3C calendar infra
- [ ] Add feature flags (consider per-command granularity)
- [ ] Implement new paths
- [ ] Keep legacy paths
- [ ] Add tests for OAuth flow (mock HTTP)
- [ ] Add tests for sync operations
- [ ] Add tests for suggestion workflow
- [ ] Manual testing with real Google/Microsoft accounts
- [ ] Test OAuth refresh logic

**Acceptance Criteria:**
- [ ] OAuth flow works for Google Calendar
- [ ] OAuth flow works for Microsoft Calendar
- [ ] Calendar sync fetches events correctly
- [ ] Event suggestions generate correctly
- [ ] Suggestion acceptance creates time entries
- [ ] Feature flags work for each command
- [ ] Tests pass: `cargo test -p pulsearc-api commands::calendar --features calendar`
- [ ] Manual testing: calendar sync works end-to-end
- [ ] No token refresh failures

**Risk Assessment:**
- **High Risk** - Complex OAuth flow, external API dependencies, largest command file
- **Mitigation:** Extensive OAuth testing, mock HTTP responses, gradual rollout per provider

---

### Task 4B.3: Idle Commands (Day 6-7)

**Source:**
- `legacy/api/src/commands/idle.rs` (193 LOC)
- `legacy/api/src/commands/idle_sync.rs` (58 LOC)

**Scope:**
- Idle period query and management
- User action updates (kept/discarded/pending)
- Idle summary statistics
- Idle sync telemetry

**Current Implementation:**
- Direct SQL queries for `idle_periods` table
- Direct `DbManager.get_connection()` calls
- Manual idle sync scheduling

**New Implementation:**
- Use `IdlePeriodsRepository` for all idle period CRUD operations
- Keep legacy idle detection infrastructure (tracker/detector) - **no migration needed**
- Use repository pattern for clean architecture

**Commands to Rewire:**
1. `get_idle_periods` - Fetch idle periods for date range
2. `update_idle_period_action` - User decision (kept/discarded/auto_excluded)
3. `get_idle_summary` - Aggregated idle time statistics
4. `sync_idle_telemetry` - Sync idle metrics to analytics (if exists)

**⚠️ Note on Idle Detection:**
The idle **detection** infrastructure (`MacOsIdleDetector`, `IdlePeriodTracker`, `LockScreenListener`) stays in `legacy/api/src/tracker/idle/` for now. These are part of the tracker infrastructure, not the API layer. They will be migrated in a future phase (Phase 5: Tracker Refactoring).

**This task only rewires the CRUD commands** that query the `idle_periods` table.

**Implementation Checklist:**
- [ ] **Prerequisite:** Verify `idle_periods` table exists in schema.sql (added in prerequisites)
- [ ] **Prerequisite:** Verify `IdlePeriodsRepository` implemented (added in prerequisites)
- [ ] Read current implementation (251 LOC total)
- [ ] Identify all SQL queries for `idle_periods` table
- [ ] Map queries to repository methods:
  - `query_range(start_ts, end_ts)` → replaces time range SELECT
  - `update_action(period_id, action, notes)` → replaces UPDATE query
  - `get_summary(start_ts, end_ts)` → replaces aggregation SELECT
- [ ] Create new implementation using `IdlePeriodsRepository`
- [ ] Add feature flags (`use_new_idle_commands`)
- [ ] Implement new paths (repository calls)
- [ ] Keep legacy paths (direct SQL)
- [ ] Add tests for repository integration
- [ ] Add tests for user action updates
- [ ] Manual testing with real idle periods from UI

**Acceptance Criteria:**
- [ ] Idle period queries return correct data
- [ ] User action updates persist correctly (kept/discarded/pending)
- [ ] Idle summary calculations match legacy behavior
- [ ] Telemetry sync works (if command exists)
- [ ] Feature flags work (can toggle between old/new)
- [ ] Tests pass: `cargo test -p pulsearc-api commands::idle`
- [ ] Manual testing: idle UI shows correct data, actions persist

**Risk Assessment:**
- **Low Risk** - Simple CRUD operations, no complex business logic
- **Note:** Idle **detection** logic (MacOsIdleDetector) is NOT touched in this task
- **Mitigation:** Parallel validation with legacy queries, test user action state machine

---

### Phase 4B Validation

**Acceptance Criteria (Overall):**
- [ ] All 3 feature commands rewired (blocks, calendar, idle)
- [ ] Feature flags allow per-command toggle
- [ ] Tests pass: `cargo test -p pulsearc-api --lib --features calendar`
- [ ] Manual testing: all features work end-to-end
- [ ] Performance: no regressions
- [ ] OAuth flows work for all providers

**Blockers for Phase 4C:**
- Phase 3D (schedulers) must be complete for monitoring commands

---

## Phase 4C: Sync & Monitoring (Week 2, Days 8-10)

**Goal:** Rewire sync and monitoring commands
**Duration:** 2-3 days
**Dependencies:** Phase 3D complete (schedulers & workers)
**Priority:** P2 (operational features)

> **📋 Phase 3D Staged Rollout:** Phase 3D ships in stages. See [PHASE-3-INFRA-TRACKING.md](./PHASE-3-INFRA-TRACKING.md) for detailed schedule:
> 1. **3D.1:** Block Scheduler (`crates/infra/src/scheduling/block_scheduler.rs`)
> 2. **3D.2:** Classification Scheduler (`crates/infra/src/scheduling/classification_scheduler.rs`)
> 3. **3D.3:** Integration Schedulers (Calendar/SAP - `crates/infra/src/scheduling/{calendar,sap}_scheduler.rs`)
> 4. **3D.4:** Outbox Worker (`crates/infra/src/sync/outbox_worker.rs`) ⬅️ **REQUIRED for Task 4C.1**
> 5. **3D.5:** Sync Helpers (Cost Tracker, Neon Client - `crates/infra/src/sync/{cost_tracker,neon_client}.rs`) ⬅️ **REQUIRED for Task 4C.1**
> 6. **3D.6:** Domain API Client (`crates/infra/src/api/`)
>
> **Phase 4C becomes unblocked after:** Tasks 3D.4 and 3D.5 complete.

### Task 4C.1: Monitoring Commands (Day 8-9)

**Source:** `legacy/api/src/commands/monitoring.rs` (741 LOC)

**Scope:**
- Sync status monitoring
- Cost tracking queries
- Outbox queue statistics
- Health checks

**Current Implementation:**
- Direct queries to outbox database
- Direct calls to cost tracker
- Direct calls to sync workers
- Manual health check logic

**New Implementation:**
- Use `OutboxRepository` from Phase 3A.8 (`crates/infra/src/database/outbox_repository.rs`)
- Use `CostTracker` from Phase 3D.5 (`crates/infra/src/sync/cost_tracker.rs`)
- Use `OutboxWorker` from Phase 3D.4 (`crates/infra/src/sync/outbox_worker.rs`)
- Use `NeonClient` from Phase 3D.5 (`crates/infra/src/sync/neon_client.rs`)
- Use health check methods from repositories

**Commands to Rewire:**
1. `get_sync_status` - Overall sync status
2. `get_outbox_stats` - Pending/sent/failed counts
3. `get_cost_summary` - API cost breakdown
4. `retry_failed_entries` - Manually retry failed outbox entries
5. `get_sync_health` - Health check for sync infrastructure

**Implementation Checklist:**
- [ ] Read current implementation (741 LOC)
- [ ] Identify all monitoring queries
- [ ] Identify all cost tracking queries
- [ ] Create new implementation using Phase 3 infra
- [ ] Add feature flag
- [ ] Implement new path
- [ ] Keep legacy path
- [ ] Add tests for each monitoring query
- [ ] Manual testing: monitoring UI shows correct data

**Acceptance Criteria:**
- [ ] Sync status reports accurately
- [ ] Outbox stats match legacy
- [ ] Cost summary shows correct totals
- [ ] Retry operation works
- [ ] Health checks detect same issues as legacy
- [ ] Feature flag works
- [ ] Tests pass: `cargo test -p pulsearc-api commands::monitoring`
- [ ] Manual testing: monitoring dashboard works

**Risk Assessment:**
- **Low Risk** - Read-only monitoring, no critical mutations
- **Mitigation:** Parallel validation of metrics

---

### Task 4C.2: Idle Sync Commands (Day 10)

**Source:** `legacy/api/src/commands/idle_sync.rs` (58 LOC)

**Scope:**
- Idle sync telemetry commands
- Idle metrics reporting

**Current Implementation:**
- Direct calls to idle sync worker
- Direct telemetry queries

**New Implementation:**
- Use idle sync infrastructure from Phase 3D (if exists)
- Use telemetry repository

**Commands to Rewire:**
1. `trigger_idle_sync` - Manually trigger idle sync
2. `get_idle_sync_stats` - Idle sync statistics

**Implementation Checklist:**
- [ ] Read current implementation (58 LOC)
- [ ] Determine if separate idle sync worker exists
- [ ] Create new implementation using Phase 3 infra
- [ ] Add feature flag
- [ ] Implement new path
- [ ] Keep legacy path
- [ ] Add tests
- [ ] Manual testing

**Acceptance Criteria:**
- [ ] Idle sync triggers correctly
- [ ] Stats report accurately
- [ ] Feature flag works
- [ ] Tests pass: `cargo test -p pulsearc-api commands::idle_sync`

**Risk Assessment:**
- **Very Low Risk** - Small file, telemetry only
- **Mitigation:** Simple parallel validation

---

### Phase 4C Validation

**Acceptance Criteria (Overall):**
- [ ] Both sync/monitoring commands rewired
- [ ] Feature flags work
- [ ] Tests pass: `cargo test -p pulsearc-api --lib`
- [ ] Manual testing: monitoring dashboard shows accurate data
- [ ] Performance: monitoring queries are fast (<100ms)

**Blockers for Phase 4D:**
- Phase 3E (ML adapters) must be complete (optional - only if using ML)

---

## Phase 4D: ML Commands (Week 2, Days 11-12) - Optional

**Goal:** Rewire ML training commands
**Duration:** 1-2 days
**Dependencies:** Phase 3E complete (ML adapters)
**Priority:** P3 (optional, only if ML features enabled)
**Feature Flag:** Requires `ml` feature enabled

### Task 4D.1: ML Training Commands (Day 11-12)

**Source:** `legacy/api/src/commands/ml_training.rs` (242 LOC)

**Scope:**
- ML model training orchestration
- Training data export
- Model evaluation metrics

**Current Implementation:**
- Direct calls to legacy training pipeline
- Direct database queries for training data
- Manual model file management

**New Implementation:**
- Use `TrainingPipeline` from Phase 3E.3
- Use `TrainingExporter` from Phase 3E.3
- Use repositories for training data queries

**Commands to Rewire:**
1. `export_training_data` - Export labeled data for training
2. `train_model` - Trigger model training
3. `get_training_metrics` - Fetch training accuracy/loss metrics
4. `evaluate_model` - Run model evaluation on test set

**Implementation Checklist:**
- [ ] Read current implementation (242 LOC)
- [ ] Identify training pipeline touchpoints
- [ ] Create new implementation using Phase 3E infra
- [ ] Add feature flag
- [ ] Implement new path (behind `#[cfg(feature = "ml")]`)
- [ ] Keep legacy path
- [ ] Add tests (with test model)
- [ ] Manual testing: train model, verify metrics

**Acceptance Criteria:**
- [ ] Training data exports correctly
- [ ] Model training completes successfully
- [ ] Metrics match legacy (accuracy, precision, recall)
- [ ] Evaluation works correctly
- [ ] Feature flag works
- [ ] Tests pass: `cargo test -p pulsearc-api commands::ml_training --features ml`
- [ ] Manual testing: train model end-to-end

**Risk Assessment:**
- **Low Risk** - Optional feature, ML training is non-critical
- **Mitigation:** Feature-gated, test with small datasets

---

### Phase 4D Validation

**Acceptance Criteria (Overall):**
- [ ] ML commands rewired (if ML feature enabled)
- [ ] Feature flag works
- [ ] Tests pass: `cargo test -p pulsearc-api --features ml`
- [ ] Manual testing: model training works
- [ ] Model quality: no regression vs legacy

---

## Phase 4E: Development Tools (Week 2-3, Day 13)

**Goal:** Rewire development/testing commands
**Duration:** 1 day
**Dependencies:** Phase 3 complete
**Priority:** P3 (development tools only)

### Task 4E.1: Seed Snapshots Command (Day 13)

**Source:** `legacy/api/src/commands/seed_snapshots.rs` (193 LOC)

**Scope:**
- Development data seeding
- Test data generation

**Current Implementation:**
- Direct database inserts
- Hardcoded test data

**New Implementation:**
- Use repositories for seeding
- Use test data builders from `pulsearc-common/testing`

**Commands to Rewire:**
1. `seed_development_data` - Seed database with test snapshots/segments/blocks

**Implementation Checklist:**
- [ ] Read current implementation (193 LOC)
- [ ] Identify seed data structures
- [ ] Create new implementation using repositories
- [ ] Add feature flag (behind `#[cfg(debug_assertions)]`)
- [ ] Implement new path
- [ ] Keep legacy path
- [ ] Add tests
- [ ] Manual testing: seed data, verify UI

**Acceptance Criteria:**
- [ ] Seed data inserts correctly via repositories
- [ ] UI displays seeded data correctly
- [ ] Feature flag works
- [ ] Tests pass: `cargo test -p pulsearc-api commands::seed_snapshots`
- [ ] Manual testing: seed command works in dev mode

**Risk Assessment:**
- **Very Low Risk** - Development-only tool
- **Mitigation:** Only available in debug builds

---

### Phase 4E Validation

**Acceptance Criteria (Overall):**
- [ ] Seed command rewired
- [ ] Feature flag works
- [ ] Tests pass
- [ ] Manual testing: seeding works in dev mode

---

## Phase 4F: Legacy Cleanup (Week 3, Days 14-16)

**Goal:** Remove legacy code and finalize migration
**Duration:** 2-3 days
**Dependencies:** Phase 4A-4E complete, validation period passed
**Priority:** P2 (cleanup, not blocking)

### Task 4F.1: Remove Feature Flags (Day 14)

**Scope:**
- Remove feature flag checks from all commands
- Make new implementation the default (and only) path
- Remove legacy code paths

**Implementation Checklist:**
- [ ] Identify all feature flag locations
- [ ] Remove feature flag environment variable checks
- [ ] Delete legacy code paths from each command
- [ ] Remove `if/else` branching on feature flags
- [ ] Update logging (remove "using new/old implementation" logs)
- [ ] Run full test suite
- [ ] Manual smoke testing

**Acceptance Criteria:**
- [ ] No feature flag checks remain in commands
- [ ] All commands use new infrastructure exclusively
- [ ] Tests pass: `cargo test -p pulsearc-api`
- [ ] Manual testing: all commands work

---

### Task 4F.2: Remove Legacy Re-exports (Day 14)

**Scope:**
- Remove legacy module re-exports from `lib.rs`
- Remove compatibility shims
- Update internal imports

**Implementation Checklist:**
- [ ] Identify all legacy re-exports in `legacy/api/src/lib.rs`
- [ ] Remove re-export statements
- [ ] Fix any broken imports in commands
- [ ] Update module structure
- [ ] Run full test suite
- [ ] Verify no dangling references

**Acceptance Criteria:**
- [ ] No legacy re-exports remain
- [ ] All imports updated to new paths
- [ ] Tests pass: `cargo test -p pulsearc-api`
- [ ] Clippy clean: `cargo clippy -p pulsearc-api -- -D warnings`

---

### Task 4F.3: Delete Legacy Code (Day 15-16)

**Scope:**
- Archive legacy codebase for reference
- Delete legacy infrastructure code from main branch
- Update documentation

> **📁 What Stays vs What Goes:**
>
> **STAY (Rewired in Place):**
> - `legacy/api/src/commands/` — **All 11 command files stay here** (rewired to use new infra)
> - `legacy/api/src/lib.rs` — Tauri app library (updated, not deleted)
> - `legacy/api/src/main.rs` — Tauri app entry point (updated, not deleted)
>
> **DELETE (Replaced by `crates/infra/`):**
> - `legacy/api/src/db/` → Replaced by `crates/infra/src/database/`
> - `legacy/api/src/domain/` → Replaced by `crates/core/`
> - `legacy/api/src/http/` → Replaced by `crates/infra/src/http/`
> - `legacy/api/src/inference/` → Split to `crates/core/` and `crates/infra/ml/`
> - `legacy/api/src/integrations/` → Replaced by `crates/infra/src/integrations/`
> - `legacy/api/src/observability/` → Replaced by `crates/infra/src/observability/`
> - `legacy/api/src/preprocess/` → Replaced by `crates/core/`
> - `legacy/api/src/shared/` → Replaced by `crates/common/`
> - `legacy/api/src/sync/` → Replaced by `crates/infra/src/sync/`
> - `legacy/api/src/tracker/` → Replaced by `crates/infra/src/platform/`
> - `legacy/api/src/utils/` → Replaced by `crates/domain/`
>
> **Final Structure:**
> ```
> legacy/api/           ← Tauri app crate (stays, much smaller)
> ├── src/
> │   ├── commands/     ← API layer (rewired, stays)
> │   ├── lib.rs        ← Thin wrapper (stays)
> │   └── main.rs       ← App init (stays)
> └── Cargo.toml        ← Deps: core, infra, domain
>
> crates/               ← New architecture (Phase 1-3)
> ├── core/             ← Business logic
> ├── domain/           ← Pure types
> ├── infra/            ← Infrastructure adapters
> └── common/           ← Shared utilities
> ```

**Implementation Checklist:**
- [ ] Create archive branch: `git branch archive/legacy-code`
- [ ] Tag legacy codebase: `git tag legacy-final-v1.0`
- [ ] Delete legacy infrastructure directories (see "DELETE" list above)
- [ ] Verify `commands/` directory remains intact
- [ ] Remove legacy dependencies from `Cargo.toml`
- [ ] Update LEGACY_MIGRATION_INVENTORY.md (mark Phase 4 complete)
- [ ] Update CLAUDE.md (remove legacy references)
- [ ] Update README files
- [ ] Create migration completion documentation
- [ ] Run full test suite
- [ ] Create PR with cleanup

**Acceptance Criteria:**
- [ ] Legacy code archived in branch/tag
- [ ] Legacy directory deleted from main branch
- [ ] Documentation updated
- [ ] Tests pass: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --workspace -- -D warnings`
- [ ] PR created and reviewed

**⚠️ IMPORTANT:**
- **Backup before deletion** - Ensure legacy code is safely archived
- **Validation period** - Only delete after 1-2 weeks of production validation
- **Rollback plan** - Keep archive branch accessible for emergency rollback

---

### Task 4F.4: Update Documentation (Day 16)

**Scope:**
- Update architecture documentation
- Update API documentation
- Create migration retrospective

**Implementation Checklist:**
- [ ] Update architecture diagrams (show final state)
- [ ] Update dependency graph (no legacy dependencies)
- [ ] Update API documentation (document new paths)
- [ ] Create migration retrospective document:
  - What went well
  - What went wrong
  - Lessons learned
  - Metrics (before/after performance, LOC reduction, etc.)
- [ ] Update CONTRIBUTING.md with new architecture guidelines
- [ ] Update README.md with Phase 4 completion status

**Acceptance Criteria:**
- [ ] All documentation updated
- [ ] Architecture diagrams reflect final state
- [ ] Migration retrospective created
- [ ] CONTRIBUTING.md has current architecture guidelines

---

### Phase 4F Validation

**Acceptance Criteria (Overall):**
- [ ] Feature flags removed
- [ ] Legacy re-exports removed
- [ ] Legacy code deleted (after validation period)
- [ ] Documentation updated
- [ ] Tests pass: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --workspace -- -D warnings`
- [ ] Manual testing: full regression testing complete
- [ ] Production validation: 1-2 weeks without issues

---

## Overall Phase 4 Validation

### Comprehensive Testing

**Unit Tests:**
```bash
# All commands
cargo test -p pulsearc-api --lib

# With all features
cargo test -p pulsearc-api --all-features

# Specific command
cargo test -p pulsearc-api commands::blocks
```

**Integration Tests:**
```bash
# Command integration (with real infra)
cargo test -p pulsearc-api --test command_integration

# Feature-gated commands
cargo test -p pulsearc-api --features calendar
cargo test -p pulsearc-api --features ml
```

**Manual Testing Checklist:**
- [ ] Database commands: stats UI shows correct data
- [ ] User profile: settings update correctly
- [ ] Block commands: build/approve/reject workflow works
- [ ] Calendar commands: OAuth flow completes, sync works
- [ ] Idle commands: idle periods detected and displayed
- [ ] Monitoring: dashboard shows accurate metrics
- [ ] ML commands: training completes successfully (if enabled)
- [ ] Seed commands: development data loads correctly

**Regression Testing:**
- [ ] All Tauri commands callable from frontend
- [ ] No command signature changes
- [ ] Performance: no regressions vs legacy
- [ ] Error handling: errors display correctly in UI
- [ ] Concurrency: no deadlocks or race conditions

### Performance Validation

**Command Performance Targets:**
- Database stats: < 100ms
- Block building: < 5s for 1 day
- Calendar sync: < 10s for 100 events
- Monitoring queries: < 100ms
- Idle period detection: < 50ms

**Benchmark Commands:**
```bash
# Compare legacy vs new implementation
cargo bench -p pulsearc-api

# Specific command benchmarks
cargo bench -p pulsearc-api commands::blocks
```

### Code Quality Validation

```bash
# Formatting
cargo fmt --all -- --check

# Linting
cargo clippy -p pulsearc-api --all-features -- -D warnings

# Documentation
cargo doc -p pulsearc-api --all-features --no-deps

# Check for remaining TODOs
rg "TODO|FIXME|XXX|HACK" legacy/api/src/commands/
```

### Acceptance Criteria (Final)

**Functional:**
- [ ] All 11 commands rewired to new infrastructure
- [ ] Feature flags work for gradual rollout
- [ ] All tests pass (unit + integration)
- [ ] Manual testing complete
- [ ] Regression testing: no functional changes
- [ ] Performance: meets targets

**Non-Functional:**
- [ ] Code coverage ≥ 70% for commands
- [ ] No clippy warnings
- [ ] All public command APIs documented
- [ ] No memory leaks
- [ ] No deadlocks or race conditions

**Cleanup:**
- [ ] Feature flags removed (after validation)
- [ ] Legacy re-exports removed
- [ ] Legacy code deleted (after validation period)
- [ ] Documentation updated

**Production Readiness:**
- [ ] 1-2 week validation period passed
- [ ] No P0/P1 issues found in production
- [ ] Rollback plan tested and documented
- [ ] Monitoring shows healthy metrics

---

## Risk Assessment

### High-Risk Areas

#### 1. Calendar OAuth Flow (Phase 4B.2)
**Risk:** OAuth token refresh may fail, breaking calendar sync

**Mitigation:**
- Extensive OAuth testing with mock HTTP
- Test token refresh logic thoroughly
- Implement automatic retry on token refresh failures
- Add monitoring for OAuth failures
- Keep legacy OAuth path for emergency fallback

#### 2. Block Building Workflow (Phase 4B.1)
**Risk:** Block building logic may produce different results

**Mitigation:**
- Parallel validation: run old and new in parallel, compare outputs
- Extensive unit tests covering all edge cases
- Manual testing with real data
- Gradual rollout with feature flag
- Keep legacy path for rollback

#### 3. Database Migrations (Phase 4A.1)
**Risk:** Repository queries may not match legacy SQL behavior

**Mitigation:**
- Query result comparison tests
- Test with real database (not just mocks)
- Verify NULL handling, edge cases
- Performance testing: ensure no N+1 queries

### Medium-Risk Areas

#### 4. Feature Flag Complexity
**Risk:** Feature flag logic may cause bugs or confusion

**Mitigation:**
- Clear logging of which path is used
- Metrics tracking old vs new usage
- Simple flag structure (environment variable)
- Remove flags as soon as validation passes

#### 5. Command Signature Changes
**Risk:** Accidentally changing command signatures breaks frontend

**Mitigation:**
- Careful review of command signatures
- Integration tests with frontend
- No changes to command parameters or return types
- Maintain backward compatibility

### Low-Risk Areas

#### 6. Development Tools (Phase 4E)
**Risk:** Seed commands may not work correctly

**Mitigation:**
- Only affects development environments
- Easy to fix if broken
- Not critical for production

---

## Rollback Plan

### Immediate Rollback (During Phase 4)

**If critical issues arise:**

1. **Identify broken command** - Which command is failing?
2. **Toggle feature flag via database update:**
   ```bash
   sqlite3 ~/Library/Application\ Support/com.pulsearc.app/pulsearc.db

   # Disable specific command
   UPDATE feature_flags SET enabled = 0 WHERE flag_name = 'new_blocks_cmd';

   # Or disable all new infrastructure
   UPDATE feature_flags SET enabled = 0 WHERE flag_name = 'use_new_infra';
   ```
3. **Restart application** - Feature flags load from database on startup
4. **Verify legacy path works** - Confirm old implementation is functional
5. **Fix issues** - Debug and fix new implementation
6. **Re-enable new path** - Update database again when ready
7. **Rollback window** - 2-5 minutes (DB update + app restart)

**Alternative: Admin UI Toggle (if implemented):**
- Open dev panel: `Cmd+Shift+D`
- Navigate to "Feature Flags" tab
- Toggle flag to `disabled`
- Restart app

### Partial Rollback (After Phase 4F - Feature Flags Removed)

**If issues found in production after cleanup:**

1. **Revert recent commits** - Git revert Phase 4F commits
2. **Restore feature flags** - Re-add flag checks to broken commands
3. **Deploy hotfix** - Push flag-enabled version
4. **Toggle to legacy** - Set environment variable to use old path
5. **Extended timeline** - Give 1 week for thorough fixes

### Full Rollback (Unlikely)

**If Phase 4 is fundamentally flawed:**

1. **Restore legacy code** - Checkout `archive/legacy-code` branch
2. **Cherry-pick fixes** - Bring forward any bug fixes from Phase 4
3. **Restore legacy re-exports** - Re-enable legacy module imports
4. **Revert all Phase 4 PRs** - Clean slate rollback
5. **Post-mortem** - Document what went wrong
6. **Timeline** - 1-2 sprints to stabilize
7. **Plan retry** - Determine if/when to retry Phase 4

---

## Dependencies & Feature Flags

### Required Dependencies

**Phase 3 Completion Status:**

| Phase | Status | Required For | Notes |
|-------|--------|--------------|-------|
| 3A | ✅ Complete | 4A, 4B, 4C, 4E | Core infrastructure |
| 3B | ✅ Complete | 4B (idle) | Platform adapters |
| 3C | ⏸️ Partial | 4B (calendar) | Calendar complete, SAP pending |
| 3D | ⏸️ Pending | 4C | Schedulers & workers |
| 3E | ⏸️ Pending | 4D | ML adapters (optional) |
| 3F | ✅ Complete | 4C | Observability |

**Phase 4 Can Start When:**
- ✅ Phase 3A complete (database repositories)
- ✅ Phase 3B complete (platform adapters)
- ✅ Phase 3C.5-3C.8 complete (calendar integration)

**Phase 4 Blocked On:**
- ⏸️ Phase 3D (for Task 4C - monitoring commands)
- ⏸️ Phase 3E (for Task 4D - ML commands, optional)

### Feature Flags

**Feature Flags (Persisted in Database):**

```sql
-- Stored in feature_flags table
INSERT INTO feature_flags (flag_name, enabled) VALUES
    ('use_new_infra', 1),      -- Master flag (default: enabled)
    ('new_database_cmd', 1),
    ('new_blocks_cmd', 1),
    ('new_calendar_cmd', 1),
    ('new_idle_cmd', 1),
    ('new_monitoring_cmd', 1),
    ('new_ml_cmd', 1);
```

**Implementation Pattern (Using FeatureFlagsRepository):**

```rust
#[tauri::command]
pub async fn build_blocks_for_date(
    date: NaiveDate,
    state: State<'_, AppState>,
) -> Result<Vec<ProposedBlock>, String> {
    // Check feature flag from database (NOT env var)
    let use_new_infra = state.feature_flags
        .is_enabled("new_blocks_cmd")
        .await
        .unwrap_or(true); // Default: enabled

    if use_new_infra {
        tracing::info!("Using new block building infrastructure");
        // NEW IMPLEMENTATION: Use BlockBuilderService + BlockRepository
        let builder_service = &state.block_builder_service;
        builder_service.build_blocks_for_date(date).await
            .map_err(|e| e.to_string())
    } else {
        tracing::info!("Using legacy block building infrastructure");
        // LEGACY IMPLEMENTATION: Keep existing code
        legacy::build_blocks_for_date(date, state).await
            .map_err(|e| e.to_string())
    }
}
```

**Tauri AppState Setup:**

```rust
pub struct AppState {
    pub db_manager: Arc<DbManager>,
    pub feature_flags: Arc<FeatureFlags>,
    pub block_builder_service: Arc<BlockBuilder>,
    // ... other services
}

pub struct FeatureFlags {
    repo: FeatureFlagsRepository,
    cache: Mutex<HashMap<String, bool>>, // Cache for performance
}

impl FeatureFlags {
    pub async fn is_enabled(&self, flag_name: &str) -> Result<bool> {
        // Check cache first
        if let Some(&enabled) = self.cache.lock().unwrap().get(flag_name) {
            return Ok(enabled);
        }

        // Query database
        let enabled = self.repo.is_enabled(flag_name).await?;

        // Update cache
        self.cache.lock().unwrap().insert(flag_name.to_string(), enabled);

        Ok(enabled)
    }
}
```

**See [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md) lines 299-372 for complete `FeatureFlagsRepository` implementation.**

---

## Timeline & Milestones

### Gantt Chart (Weeks 1-3)

```
Week 1: Phase 4A + 4B (Core + Feature Commands)
├─ Day 1: Database Commands (4A.1)
├─ Day 2: User Profile + Window Commands (4A.2, 4A.3)
├─ Day 3: Phase 4A Validation
├─ Day 4-5: Block Commands (4B.1)
├─ Day 5-6: Calendar Commands (4B.2)
├─ Day 6-7: Idle Commands (4B.3)
└─ Milestone: Core features rewired ✓

Week 2: Phase 4C + 4D + 4E (Sync + ML + Dev Tools)
├─ Day 8-9: Monitoring Commands (4C.1)
├─ Day 10: Idle Sync Commands (4C.2)
├─ Day 11-12: ML Training Commands (4D.1, optional)
├─ Day 13: Seed Commands (4E.1)
└─ Milestone: All commands rewired ✓

Week 3: Phase 4F (Validation + Cleanup)
├─ Day 14: Remove Feature Flags (4F.1)
├─ Day 14: Remove Legacy Re-exports (4F.2)
├─ Day 15-16: Delete Legacy Code (4F.3)
├─ Day 16: Update Documentation (4F.4)
└─ Milestone: Phase 4 complete, legacy removed ✓
```

### Critical Path

**Must complete in order:**
1. Phase 3A → 4A (database repos needed for database commands)
2. Phase 3C → 4B (calendar infra needed for calendar commands)
3. Phase 3D → 4C (schedulers needed for monitoring commands)
4. 4A-4E → 4F (all commands must be rewired before cleanup)

**Can run in parallel:**
- 4A + 4B (if Phase 3A, 3B, 3C complete)
- 4C + 4D (if Phase 3D, 3E complete)

### Validation Period

**Before Phase 4F Cleanup:**
- **Duration:** 1-2 weeks of production usage
- **Monitoring:** Track error rates, performance metrics
- **Rollback criteria:** Any P0/P1 issues trigger rollback via feature flags
- **Success criteria:** No regressions, performance meets targets

---

## Success Criteria Summary

### Functional Requirements

**Must Have (P1):**
- [ ] All 11 commands rewired to new infrastructure
- [ ] Feature flags enable gradual rollout
- [ ] Backward compatibility maintained (no API changes)
- [ ] All tests pass (unit + integration + manual)
- [ ] Performance meets or exceeds legacy targets

**Should Have (P2):**
- [ ] Legacy code removed after validation period
- [ ] Documentation updated with new architecture
- [ ] Migration retrospective completed
- [ ] Monitoring shows healthy metrics

**Nice to Have (P3):**
- [ ] Improved error messages vs legacy
- [ ] Better logging/tracing than legacy
- [ ] Performance improvements vs legacy

### Non-Functional Requirements

**Quality:**
- [ ] Code coverage ≥ 70% for commands
- [ ] No clippy warnings
- [ ] All command APIs documented
- [ ] All tests pass

**Performance:**
- [ ] No regressions vs legacy (per-command benchmarks)
- [ ] Database queries: < 100ms p99
- [ ] Block building: < 5s for 1 day
- [ ] Calendar sync: < 10s for 100 events

**Reliability:**
- [ ] No memory leaks
- [ ] No deadlocks or race conditions
- [ ] Graceful error handling (no panics)
- [ ] Rollback plan tested and documented

---

## Documentation Updates

### Files to Update After Phase 4

1. **LEGACY_MIGRATION_INVENTORY.md**
   - Mark Phase 4 complete
   - Update overall migration progress to 100%
   - Document final architecture state

2. **CLAUDE.md**
   - Remove legacy code references
   - Update with new command patterns
   - Document repository usage in commands
   - Add Phase 4 lessons learned

3. **Architecture Diagrams**
   - Update to show final clean architecture
   - Remove legacy dependencies from diagrams
   - Show command → service → repository → database flow

4. **API Documentation**
   - Document new command implementations
   - Update command examples
   - Remove legacy code examples

5. **CONTRIBUTING.md**
   - Update with new command development guidelines
   - Show how to add new commands using repositories
   - Update testing guidelines

6. **README.md**
   - Update project status (migration complete!)
   - Update architecture overview
   - Add link to migration retrospective

7. **Migration Retrospective** (new file)
   - `docs/MIGRATION_RETROSPECTIVE.md`
   - Document full migration journey (Phases 1-4)
   - Metrics: LOC reduced, performance improvements, technical debt eliminated
   - Lessons learned
   - Recommendations for future migrations

---

## Next Steps

### After Phase 4 Complete

**Immediate:**
1. Create GitHub release: `v2.0.0-clean-architecture`
2. Update changelog with all Phase 4 changes
3. Run full CI pipeline and verify all tests pass
4. Manual regression testing on all supported platforms (macOS primary)
5. Deploy to production with monitoring

**Post-Deployment:**
1. Monitor for 1-2 weeks (validation period)
2. Track error rates, performance metrics
3. Collect user feedback
4. Address any issues found

**Cleanup (after validation period):**
1. Execute Phase 4F (remove feature flags, delete legacy code)
2. Create migration retrospective document
3. Celebrate migration completion! 🎉

**Future Work:**
1. Performance optimizations (now that architecture is clean)
2. New features (easier to add with clean architecture)
3. Technical debt reduction (continue refactoring)
4. Consider Phase 5: Frontend refactoring (if needed)

---

**Document Version:** 1.0
**Last Updated:** 2025-10-31
**Status:** ✅ Ready for Execution (Pending Phase 3 Complete)
**Next Review:** After Phase 3 complete

---

## Document Change Log

### Version 1.0 (2025-10-31) - Initial Creation

**Document Created:**
- Comprehensive Phase 4 tracking document created based on Phase 3 audit findings
- 11 command files identified for rewiring (~3,679 LOC)
- 6 sub-phases defined (4A-4F)
- Feature flag strategy documented for gradual rollout
- Validation period strategy defined
- Rollback plan documented

**Scope Defined:**
- Phase 4A: Core commands (database, user profile, window)
- Phase 4B: Feature commands (blocks, calendar, idle)
- Phase 4C: Sync & monitoring commands
- Phase 4D: ML commands (optional, feature-gated)
- Phase 4E: Development tools (seed commands)
- Phase 4F: Legacy cleanup (remove flags, delete legacy code)

**Risk Assessment:**
- High risk: Calendar OAuth, block building workflow
- Medium risk: Feature flags, command signatures
- Low risk: Development tools

**Timeline:**
- Estimated: 2-3 weeks (11-16 days)
- Critical path: 3A → 4A → 4F
- Parallel opportunities: 4A + 4B, 4C + 4D

**Status:** Document ready for Phase 4 execution when Phase 3 completes.

---

## Appendix: Command File Inventory

### Complete Command Listing

| Command File | Size | Phase 4 Task | New Dependencies | Priority |
|--------------|------|--------------|------------------|----------|
| `database.rs` | 421 LOC | 4A.1 | Repositories (Activity, Segment, Block) | P1 |
| `user_profile.rs` | 49 LOC | 4A.2 | UserProfileRepository (or domain service) | P1 |
| `window.rs` | 61 LOC | 4A.3 | None (UI-only) | P1 |
| `blocks.rs` | 632 LOC | 4B.1 | BlockBuilderService, BlockRepository | P1 |
| `calendar.rs` | 946 LOC | 4B.2 | CalendarProvider, OAuthService, CalendarEventRepository | P1 |
| `idle.rs` | 193 LOC | 4B.3 | IdleDetector, repositories | P1 |
| `monitoring.rs` | 741 LOC | 4C.1 | OutboxRepository, CostTracker, OutboxWorker | P2 |
| `idle_sync.rs` | 58 LOC | 4C.2 | Idle sync infrastructure | P2 |
| `ml_training.rs` | 242 LOC | 4D.1 | TrainingPipeline, TrainingExporter (feature: ml) | P3 |
| `seed_snapshots.rs` | 193 LOC | 4E.1 | Repositories, test data builders | P3 |
| `mod.rs` | 78 LOC | 4F.2 | N/A (module organization) | P2 |
| **Total** | **3,679 LOC** | - | - | - |

### Command Dependencies on Phase 3

| Command | Depends On | Concrete Modules | Status | Blocker |
|---------|------------|------------------|--------|---------|
| Database | Phase 3A | `infra/database/*_repository.rs` | ✅ Ready | None |
| User Profile | Phase 3A | `infra/database/user_profile_repository.rs` | ❌ Blocked | Repo doesn't exist yet |
| Window | None | N/A (UI-only) | ✅ Ready | None |
| Blocks | Phase 3A, 2 | `core/classification/block_builder.rs`<br>`infra/database/block_repository.rs` | ✅ Ready | None |
| Calendar | Phase 3C.5-3C.8 | `infra/integrations/calendar/provider.rs`<br>`common/auth/oauth_service.rs` | ✅ Ready | None |
| Idle | Phase 3B, 2 | `infra/platform/macos/activity_provider.rs`<br>`core/tracking/service.rs` (idle detection TBD) | ⚠️ Partial | Idle service unclear |
| Monitoring | Phase 3D | `infra/sync/outbox_worker.rs` (3D.4)<br>`infra/sync/cost_tracker.rs` (3D.5)<br>`infra/sync/neon_client.rs` (3D.5) | ⏸️ Blocked | 3D.4, 3D.5 incomplete |
| Idle Sync | Phase 3D | `infra/sync/*` (3D.5) | ⏸️ Blocked | 3D.5 incomplete |
| ML Training | Phase 3E | `infra/ml/training_pipeline.rs` (3E.3)<br>`infra/ml/training_exporter.rs` (3E.3) | ⏸️ Blocked | 3E not started (optional) |
| Seed Snapshots | Phase 3A | `infra/database/*_repository.rs` | ✅ Ready | None |

**Corrected Readiness:**
- **5/11 commands ready** (4A.1, 4A.3, 4B.1, 4B.2, 4E.1)
- **1/11 partially ready** (4B.3 - idle detection approach unclear)
- **1/11 blocked by missing repo** (4A.2 - UserProfileRepository)
- **2/11 blocked by Phase 3D** (4C.1, 4C.2 - need Tasks 3D.4, 3D.5)
- **1/11 blocked by Phase 3E** (4D.1 - optional)

**NOTE:** Original count of "7/11 ready" was incorrect. See [PHASE-4-ERRATA.md](./PHASE-4-ERRATA.md#issue-1-missing-phase-3-deliverables-high-priority) for details.

---

**END OF PHASE 4 TRACKING DOCUMENT**
