# Phase 4: New Crate Migration — `legacy/api/` → `crates/api/pulsearc-app`

**Status:** ⏸️ Ready to Start
**Last Updated:** 2025-10-31 (Revised with implementation notes)
**Timeline:** 5 weeks (2 migration + 2 validation + 1 cleanup)
**Commands to Migrate:** 9 (feature_flags already migrated, ML skipped)
**Total LOC:** ~3,385

---

## Latest Revisions (2025-10-31)

The latest revisions tighten the plan with critical implementation notes:

**Scope Clarification:**
- ✅ Nine remaining commands (feature flags already migrated, ML intentionally skipped)
- ✅ Database work flows through dedicated `DatabaseStatsPort`/`DatabaseStatsRepository`
- ✅ `AppContext::shutdown()` documented as safe no-op (tokio tasks auto-cancel)

**Phase 0 Added:**
- ✅ Pre-migration verification checklist
- ✅ Repository audit with explicit `DatabaseStatsRepository` creation steps
- ✅ Scheduler survey to document lifecycle hooks before Phase 1
- ✅ Database schema verification via migration system (not manual SQL)
- ✅ Backup strategy and performance baseline establishment

**Observability & Logging:**
- ✅ Structured logging helper with `tracing` macros (not `println!`)
- ✅ Sensitive data protection guidelines (no user IDs, tokens, file paths)
- ✅ Metrics collection strategy decision point

**TypeScript Type Safety:**
- ✅ Type generation verification (confirm `ts-gen` feature is current approach)
- ✅ Type comparison workflow (detect breaking changes early)
- ✅ CI integration to fail on unexpected type changes

**Key Implementation Notes:**
1. **Repository Creation:** Follow ADR-003 hexagonal pattern (port in `core/`, implementation in `infra/`, wire to `AppContext`)
2. **Scheduler Survey:** Capture findings in decision log before Phase 1 (document which components lack lifecycle hooks and why)
3. **Schema Migrations:** Route through migration system (`schema.sql` + `SCHEMA_VERSION`), not direct `sqlite3` commands
4. **Logging Best Practices:** Use `tracing` with structured fields, avoid sensitive data
5. **Type Generation:** Verify `ts-gen` is still the active approach before wiring to CI

---

## Table of Contents

1. [Overview](#overview)
2. [Migration Strategy](#migration-strategy)
3. [Updated Architecture Principles](#updated-architecture-principles)
4. [Phase 0: Pre-Migration Verification](#phase-0-pre-migration-verification-day-0)
5. [Phase 1: Infrastructure Baseline](#phase-1-infrastructure-baseline-days-1-2)
6. [Phase 2: Command Migration](#phase-2-command-migration-days-3-13)
7. [Phase 3: Frontend Integration](#phase-3-frontend-integration-days-14-15)
8. [Phase 4: Validation Period](#phase-4-validation-period-weeks-3-4-days-16-29)
9. [Phase 5: Cleanup & Deprecation](#phase-5-cleanup--deprecation-week-5-days-30-34)
10. [Risk Register](#risk-register)
11. [Progress Tracking](#progress-tracking)
12. [Decision Log](#decision-log)
13. [Appendices](#appendices)

---

## Overview

### Goal
Migrate Tauri commands from `legacy/api/` to `crates/api/pulsearc-app` (the new, clean architecture) while maintaining backward compatibility and enabling safe rollback via database-persisted feature flags.

### Why This Approach
- **Clean separation:** New crate uses hexagonal architecture (ADR-003)
- **Gradual migration:** Feature flags allow command-by-command rollout
- **Safe rollback:** Toggle flags in database, restart app (<2 minutes)
- **Future-proof:** Legacy crate will be archived after validation

### Scope
**Commands to Migrate (9):**
1. Database commands (512 LOC)
2. User profile commands (49 LOC)
3. Window commands (61 LOC)
4. Block building commands (632 LOC)
5. Calendar integration (946 LOC)
6. Idle management (193 LOC)
7. Monitoring & stats (741 LOC)
8. Idle sync telemetry (58 LOC)
9. Seed snapshots (193 LOC)

**Commands Already Migrated:**
- Feature flags (107 LOC) — Already in `crates/api/src/commands/feature_flags.rs`

**Commands to Skip:**
- ML training (242 LOC) — Phase 3E not started (intentional)

---

## Migration Strategy

### Principles
1. **One command at a time:** Minimize risk, isolate issues
2. **Feature flags first:** All commands wrapped with fail-safe defaults
3. **Legacy isolation:** Keep old code compilable but separate for clean Phase 5 deletion
4. **Build config priority:** Update CI/build before touching frontend
5. **Validation period:** 2 weeks in production before removing legacy code

### Feature Flag Pattern (Updated)
**Default behavior:** Fail safe to legacy path on any error

```rust
#[tauri::command]
pub async fn my_command(
    context: State<'_, AppContext>,
    arg: String,
) -> Result<MyResponse, String> {
    // 🚨 CRITICAL: Default to false (legacy path) on error
    let use_new = context.feature_flags
        .is_enabled("new_my_command", true)
        .await
        .unwrap_or(false);  // ⬅️ Fail-safe: any flag lookup error → legacy path

    if use_new {
        // NEW: Use repositories + services from crates/{core,infra}
        new_implementation(&context, arg).await
            .map_err(|e| format!("New implementation error: {}", e))
    } else {
        // LEGACY: Isolated legacy code (will be removed in Phase 5)
        legacy_implementation(&context, arg).await
            .map_err(|e| format!("Legacy implementation error: {}", e))
    }
}

// New implementation using clean architecture
async fn new_implementation(
    context: &AppContext,
    arg: String,
) -> Result<MyResponse, anyhow::Error> {
    // Use repositories from crates/infra
    let result = context.my_repository.query(&arg).await?;
    Ok(MyResponse { data: result })
}

// Legacy implementation (isolated, will be deleted in Phase 5)
#[allow(dead_code)]  // Suppress warnings during migration
async fn legacy_implementation(
    context: &AppContext,
    arg: String,
) -> Result<MyResponse, anyhow::Error> {
    // Keep original logic here
    // NOTE: This will be deleted after validation period
    // If this depends on legacy/api/ modules, add temporary feature gate
    todo!("Keep legacy code isolated but compilable")
}
```

---

## Updated Architecture Principles

### Key Changes from Original Phase 4 Plan
1. **Target crate:** `crates/api/pulsearc-app` (NOT `legacy/api/`)
2. **Feature flag defaults:** `unwrap_or(false)` - fail safe to legacy
3. **Scheduler initialization:** All schedulers call `.start().await?` in `AppContext::new()`
4. **Legacy isolation:** Keep compilable but isolated for clean Phase 5 deletion
5. **Build config priority:** Update tauri.conf.json, Makefile, xtask BEFORE frontend work

### AppContext Structure (Target State)
**Location:** `crates/api/src/context/mod.rs`

```rust
pub struct AppContext {
    // Core services (already present)
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<FeatureFlagService>,

    // Schedulers (to be added in Phase 1)
    pub block_scheduler: Arc<BlockScheduler>,
    pub classification_scheduler: Arc<ClassificationScheduler>,
    pub sync_scheduler: Arc<SyncScheduler>,

    #[cfg(feature = "calendar")]
    pub calendar_scheduler: Arc<CalendarSyncScheduler>,

    // Optional components
    #[cfg(feature = "tree-classifier")]
    pub hybrid_classifier: Arc<HybridClassifier>,

    #[cfg(feature = "tree-classifier")]
    pub metrics_tracker: Arc<MetricsTracker>,

    // Instance lock (already present)
    _instance_lock: InstanceLock,
}

impl AppContext {
    pub async fn new(config: Config) -> Result<Self, anyhow::Error> {
        // Initialize core services
        let db = Arc::new(DbManager::new(&config.database_path)?);
        let feature_flags = Arc::new(FeatureFlagService::new(db.clone()));
        let tracking_service = Arc::new(TrackingService::new(db.clone()));

        // Initialize schedulers and START them
        let block_scheduler = Arc::new(BlockScheduler::new(db.clone()));
        block_scheduler.start().await?;  // 🚨 CRITICAL: Start in AppContext::new

        let classification_scheduler = Arc::new(ClassificationScheduler::new(db.clone()));
        classification_scheduler.start().await?;

        let sync_scheduler = Arc::new(SyncScheduler::new(db.clone()));
        sync_scheduler.start().await?;

        #[cfg(feature = "calendar")]
        let calendar_scheduler = {
            let scheduler = Arc::new(CalendarSyncScheduler::new(db.clone()));
            scheduler.start().await?;
            scheduler
        };

        let instance_lock = InstanceLock::acquire()?;

        Ok(Self {
            config,
            db,
            tracking_service,
            feature_flags,
            block_scheduler,
            classification_scheduler,
            sync_scheduler,
            #[cfg(feature = "calendar")]
            calendar_scheduler,
            _instance_lock: instance_lock,
        })
    }

    pub async fn shutdown(&self) -> Result<(), anyhow::Error> {
        // NOTE: Most services/schedulers don't require explicit shutdown
        // as they use tokio::spawn tasks that are automatically cancelled
        // when the runtime shuts down. Only services with explicit cleanup
        // (like database connections, file handles, OAuth tokens) need shutdown calls.

        // Survey results (Phase 1, Task 1.4):
        // - BlockScheduler: No shutdown method (uses tokio::spawn)
        // - ClassificationScheduler: No shutdown method (uses tokio::spawn)
        // - SyncScheduler: No shutdown method (uses tokio::spawn)
        // - CalendarScheduler: No shutdown method (uses tokio::spawn)
        // - TrackingService: No shutdown method
        // - FeatureFlagService: No shutdown method
        //
        // Conclusion: No explicit shutdown calls needed. Tokio runtime handles cleanup.

        Ok(())
    }
}
```

---

## Phase 0: Pre-Migration Verification (Day 0)

### Goal
Verify all prerequisites are met before starting migration. This phase ensures no blockers exist and establishes baseline metrics.

### Tasks

#### 0.1: Repository Audit
- [ ] Verify existing repositories have tests passing:
  - `UserProfileRepository` (7/7 tests passing) ✅
  - `IdlePeriodsRepository` (6/6 tests passing) ✅
  - `BlockRepository` (exists and tested) ✅
  - `CalendarRepository` (exists) ✅
  - `SnapshotRepository` (exists) ✅
  - `SegmentRepository` (exists) ✅

- [ ] **BLOCKER:** Create missing `DatabaseStatsRepository` (required for 4A.1)
  - **Port trait:** `crates/core/src/ports/database_stats.rs`
    - Define `DatabaseStatsPort` trait
    - Must be `Send + Sync` for async compatibility
    - Methods: `get_database_size`, `get_table_stats`, `vacuum_database`, `check_database_health`
  - **Implementation:** `crates/infra/src/repositories/database_stats.rs`
    - Implement `DatabaseStatsPort` for `DatabaseStatsRepository`
    - Constructor takes `Arc<DbManager>` or `Arc<SqlCipherPool>`
    - All queries use parameterized SQL (no string concatenation)
  - **AppContext wiring:** Expose `database_stats_repository` field
    - Add to `AppContext` struct
    - Initialize in `AppContext::new()` after `DbManager`
    - Pass to commands via `context.database_stats_repository`
  - **Testing:** Add unit tests with mock database

**Implementation Note:** The repository creation follows ADR-003 hexagonal pattern:
1. Define port interface in `core/` (business logic layer, no infrastructure dependencies)
2. Implement adapter in `infra/` (concrete implementation with SQLCipher)
3. Wire to context in `api/` (dependency injection)

This ensures `core` remains infrastructure-agnostic and fully testable.

**Acceptance Criteria:**
- All listed repositories exist and tests pass
- `DatabaseStatsRepository` created with passing tests
- Repository exposed from `AppContext`

---

#### 0.2: Scheduler Survey
- [ ] Survey which schedulers expose `.start().await?` method:
  - `BlockScheduler` - Has `.start()`? ⬜ Unknown
  - `ClassificationScheduler` - Has `.start()`? ⬜ Unknown
  - `SyncScheduler` - Has `.start()`? ⬜ Unknown
  - `CalendarSyncScheduler` - Has `.start()`? ⬜ Unknown

- [ ] **Document findings in decision log** (before Phase 1 begins)
  - Which schedulers require `.start()` calls
  - Which schedulers lack `start()`/`shutdown()` hooks
  - Rationale for components without lifecycle hooks (e.g., tokio tasks auto-cancel)
  - Any refactoring needed to add missing hooks

- [ ] **If `.start()` doesn't exist:** Decide strategy
  - **Option A:** Add `.start()` method to scheduler (preferred for consistency)
  - **Option B:** Document why not needed (e.g., scheduler is lazy-initialized on first use)
  - **Option C:** Initialize differently (e.g., pass to constructor, starts automatically)

**Implementation Note:** Capture survey results in the decision log right before Phase 1 work begins. Future maintainers need to know which components still lack explicit lifecycle hooks and why this is acceptable.

**Acceptance Criteria:**
- All schedulers surveyed (documented which have `.start()`)
- Decision log entry created with findings
- Strategy documented for any missing hooks

---

#### 0.3: Database Schema Verification
- [ ] Verify `feature_flags` table exists in production database
  ```sql
  SELECT name FROM sqlite_master
  WHERE type='table' AND name='feature_flags';
  ```

- [ ] **If missing:** Add via migration system (NOT manual sqlite3)
  - Update `crates/infra/src/database/schema.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS feature_flags (
        name TEXT PRIMARY KEY NOT NULL,
        is_enabled BOOLEAN NOT NULL DEFAULT 0,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    ```
  - Increment `SCHEMA_VERSION` in migration code
  - Run migration via `DbManager::migrate()` or equivalent
  - Test migration on copy of production database first

- [ ] **Seed initial feature flags via repository/migration script:**
  - Create migration function or repository method
  - Seed all Phase 4 flags (disabled by default):
    ```rust
    // Via FeatureFlagService or migration script
    feature_flags.insert_if_not_exists("new_database_commands", false, "Phase 4A.1")?;
    feature_flags.insert_if_not_exists("new_user_profile_commands", false, "Phase 4A.2")?;
    feature_flags.insert_if_not_exists("new_window_commands", false, "Phase 4A.3")?;
    feature_flags.insert_if_not_exists("new_block_commands", false, "Phase 4B.1")?;
    feature_flags.insert_if_not_exists("new_calendar_commands", false, "Phase 4B.2")?;
    feature_flags.insert_if_not_exists("new_idle_commands", false, "Phase 4C.1")?;
    feature_flags.insert_if_not_exists("new_monitoring_commands", false, "Phase 4C.2")?;
    feature_flags.insert_if_not_exists("new_idle_sync_commands", false, "Phase 4C.3")?;
    feature_flags.insert_if_not_exists("new_seed_commands", false, "Phase 4C.4")?;
    ```

**Implementation Note:** Route schema changes through the existing migration system (update `schema.sql` and `SCHEMA_VERSION`) rather than running direct `sqlite3` commands. Manual SQL commands are useful for verification, but the authoritative approach must be reproducible and version-controlled.

**Acceptance Criteria:**
- `feature_flags` table exists in production database
- All Phase 4 flags seeded (disabled by default)
- Migration tested on database copy
- Schema version incremented

---

#### 0.4: Backup Strategy
- [ ] Create database backup script: `scripts/backup-db.sh`
  ```bash
  #!/bin/bash
  DB_PATH="${1:-$HOME/Library/Application Support/com.pulsearc.app/pulsearc.db}"
  BACKUP_DIR="${2:-./backups}"
  TIMESTAMP=$(date +%Y%m%d_%H%M%S)

  mkdir -p "$BACKUP_DIR"
  cp "$DB_PATH" "$BACKUP_DIR/pulsearc_backup_$TIMESTAMP.db"
  echo "Backup created: $BACKUP_DIR/pulsearc_backup_$TIMESTAMP.db"
  ```

- [ ] Test backup/restore procedure
  ```bash
  ./scripts/backup-db.sh
  # Corrupt database intentionally
  # Restore from backup
  cp backups/pulsearc_backup_*.db ~/Library/Application\ Support/com.pulsearc.app/pulsearc.db
  # Verify app launches successfully
  ```

- [ ] Document backup location and retention policy
  - Backups stored in: `./backups/` (local) or `~/PulseArcBackups/` (user directory)
  - Retention: Keep last 10 backups, auto-delete older
  - Backup before: Each phase starts, any risky operation

- [ ] **CRITICAL:** Take backup before each phase starts
  - Phase 1: Before expanding AppContext
  - Phase 2: Before first command migration
  - Phase 3: Before frontend changes
  - Phase 4: Before enabling feature flags

**Acceptance Criteria:**
- Backup script exists and is executable
- Backup/restore tested successfully
- Retention policy documented
- Initial backup taken (before Phase 1)

---

#### 0.5: Performance Baseline
- [ ] Run performance benchmark suite on legacy commands (if benchmarks exist)
  ```bash
  cargo bench --bench command_latency -- --save-baseline phase4-legacy
  ```

- [ ] **If benchmarks don't exist:** Create lightweight timing script
  ```rust
  // tests/performance_baseline.rs
  use std::time::Instant;

  #[tokio::test]
  async fn baseline_get_database_stats() {
      let start = Instant::now();
      let result = legacy_get_database_stats().await;
      let duration = start.elapsed();

      assert!(result.is_ok());
      println!("Legacy get_database_stats: {:?}", duration);
      // Store baseline: P50/P95/P99 over 100 iterations
  }
  ```

- [ ] Document P50/P95/P99 latencies for each command
  - Create: `docs/performance-baseline-phase4.md`
  - Include: Command name, sample size, P50/P95/P99, date

- [ ] Create comparison script: `scripts/compare-performance.sh`
  ```bash
  #!/bin/bash
  cargo bench --bench command_latency -- --baseline phase4-legacy
  # Outputs: % change in latency (new vs legacy)
  ```

**Acceptance Criteria:**
- Performance baseline captured (benchmark or manual timing)
- Latency metrics documented (P50/P95/P99)
- Comparison script ready for Phase 4 validation

---

### Phase 0 Success Criteria
- ✅ All existing repositories verified (tests passing)
- ✅ `DatabaseStatsRepository` created and wired to `AppContext`
- ✅ Scheduler survey completed and documented in decision log
- ✅ `feature_flags` table exists (via migration system)
- ✅ All Phase 4 flags seeded (disabled)
- ✅ Backup strategy tested and documented
- ✅ Initial backup taken
- ✅ Performance baseline established
- ✅ No blockers remain before Phase 1

---

## Phase 1: Infrastructure Baseline (Days 1-2)

### Goal
Prepare `crates/api/pulsearc-app` for command migration by expanding AppContext and updating build configuration.

### Tasks

#### 1.1: Verify New Crate Baseline
- [ ] Run `cargo check -p pulsearc-app` and confirm it passes
- [ ] Run `cargo test -p pulsearc-app` and confirm all tests pass
- [ ] Review current AppContext structure vs legacy AppState
- [ ] Document gaps in services/schedulers

**Acceptance Criteria:**
- `cargo check -p pulsearc-app` exits 0
- All existing tests pass
- Gap analysis document created

---

#### 1.2: Expand AppContext with Schedulers
**File:** `crates/api/src/context/mod.rs`

- [ ] Add `block_scheduler: Arc<BlockScheduler>` field
- [ ] Add `classification_scheduler: Arc<ClassificationScheduler>` field
- [ ] Add `sync_scheduler: Arc<SyncScheduler>` field
- [ ] Add `calendar_scheduler: Arc<CalendarSyncScheduler>` field (feature-gated)
- [ ] Add `hybrid_classifier: Arc<HybridClassifier>` field (feature-gated, optional)
- [ ] Add `metrics_tracker: Arc<MetricsTracker>` field (feature-gated, optional)

**Acceptance Criteria:**
- AppContext struct matches legacy AppState in functionality
- All fields use `Arc<T>` for thread-safe shared ownership
- Feature gates match legacy (`#[cfg(feature = "calendar")]`, etc.)
- Code compiles with `cargo check -p pulsearc-app`

---

#### 1.3: Wire Scheduler Constructors with `.start()`
**File:** `crates/api/src/context/mod.rs`

- [ ] Initialize BlockScheduler in `AppContext::new()`
- [ ] Call `block_scheduler.start().await?` (verify it returns Result)
- [ ] Initialize ClassificationScheduler and call `.start().await?`
- [ ] Initialize SyncScheduler and call `.start().await?`
- [ ] Initialize CalendarSyncScheduler (feature-gated) and call `.start().await?`
- [ ] Initialize HybridClassifier (feature-gated, optional, may not have .start())
- [ ] Initialize MetricsTracker (feature-gated, optional)

**Critical Implementation Notes:**
- Each scheduler MUST call `.start().await?` in `AppContext::new()`
- This ensures all background tasks are running before command registration
- If `.start()` doesn't exist, create it or document why it's not needed
- Use feature gates consistently: `#[cfg(feature = "calendar")]`

**Acceptance Criteria:**
- All schedulers initialized in `AppContext::new()`
- All schedulers call `.start().await?` (or document why not needed)
- Error handling: If any scheduler fails to start, `AppContext::new()` returns Err
- Code compiles with `cargo check -p pulsearc-app --all-features`

---

#### 1.4: Implement AppContext::shutdown
**File:** `crates/api/src/context/mod.rs`

- [ ] Create `pub async fn shutdown(&self) -> Result<(), anyhow::Error>` method
- [ ] **Survey schedulers/services:** Check which ones expose `shutdown()` methods
- [ ] **Only add shutdown calls for components that implement it** (e.g., `OAuthManager::shutdown()`)
- [ ] Add comment documenting why most schedulers don't need explicit shutdown (tokio tasks auto-cancel)
- [ ] Write integration test: `tests/context_lifecycle.rs`
- [ ] Test: AppContext::new succeeds and all schedulers start
- [ ] Test: AppContext::shutdown completes without panicking
- [ ] Test: `shutdown()` can be called multiple times (idempotent)

**Acceptance Criteria:**
- `shutdown()` method exists and is public
- Method completes without panicking (even if no shutdowns are called)
- Only calls `shutdown()` on components that actually implement it
- Integration test passes: `cargo test -p pulsearc-app context_lifecycle`
- Test verifies graceful shutdown, not that all tasks are stopped (runtime handles that)

---

#### 1.5: Update Build Configuration
**Files:** `Makefile`, `xtask/src/main.rs`, `crates/api/tauri.conf.json`

**Makefile:**
- [ ] Update `make dev` to use `crates/api/` as working directory
- [ ] Update `make build-tauri` to build new crate
- [ ] Verify `make test` includes `-p pulsearc-app`
- [ ] Add comment: "Building new crate (crates/api/pulsearc-app)"

**xtask:**
- [ ] Update `ci` command to test new crate: `cargo test -p pulsearc-app`
- [ ] Update `clippy` command to include new crate
- [ ] Update `fmt` command to include new crate
- [ ] Verify `cargo xtask ci` passes

**tauri.conf.json:**
- [ ] Verify `frontendDist` points to `../../frontend/dist`
- [ ] Verify `identifier` is `com.pulsearc.app`
- [ ] Verify all icon paths are correct
- [ ] Test: `cargo tauri build` succeeds

**Acceptance Criteria:**
- `make dev` launches new crate (not legacy)
- `make ci` tests new crate
- `cargo tauri build` produces working app bundle
- All paths resolve correctly (frontend dist, icons, etc.)

---

#### 1.6: Observability Setup
**Goal:** Establish logging and metrics infrastructure before command migration.

**Files:** `crates/api/src/utils/logging.rs`, `crates/api/src/utils/metrics.rs` (optional)

##### Structured Logging Helper
- [ ] Create logging utility: `crates/api/src/utils/logging.rs`
  ```rust
  use tracing::{info, warn, error};

  /// Log command execution with structured fields.
  ///
  /// IMPORTANT: Use tracing macros with structured fields.
  /// Avoid embedding user identifiers or sensitive data.
  pub fn log_command_execution(
      command: &str,
      implementation: &str, // "legacy" or "new"
      duration_ms: u64,
      success: bool,
  ) {
      if success {
          info!(
              command = command,
              implementation = implementation,
              duration_ms = duration_ms,
              "command_execution_success"
          );
      } else {
          warn!(
              command = command,
              implementation = implementation,
              duration_ms = duration_ms,
              "command_execution_failure"
          );
      }
  }

  /// Log feature flag evaluation.
  pub fn log_feature_flag_check(
      flag_name: &str,
      is_enabled: bool,
      fallback_used: bool,
  ) {
      info!(
          flag_name = flag_name,
          is_enabled = is_enabled,
          fallback_used = fallback_used,
          "feature_flag_evaluated"
      );
  }
  ```

- [ ] Add to each command wrapper:
  ```rust
  use crate::utils::logging::log_command_execution;

  #[tauri::command]
  pub async fn my_command(context: State<'_, AppContext>) -> Result<Response, String> {
      let start = std::time::Instant::now();
      let use_new = context.feature_flags
          .is_enabled("new_my_command", true)
          .await
          .unwrap_or(false);

      let result = if use_new {
          new_implementation(&context).await
      } else {
          legacy_implementation(&context).await
      };

      log_command_execution(
          "my_command",
          if use_new { "new" } else { "legacy" },
          start.elapsed().as_millis() as u64,
          result.is_ok(),
      );

      result
  }
  ```

##### Logging Best Practices
**Implementation Notes:**
- Use `tracing` macros with structured fields (NOT `println!` or format strings)
- Avoid logging sensitive data:
  - ❌ User identifiers (email, username, UUID)
  - ❌ File paths (may contain usernames)
  - ❌ OAuth tokens, API keys
  - ❌ Database contents
  - ✅ Command names, durations, success/failure
  - ✅ Feature flag states
  - ✅ Error types (not messages with user data)
- Use appropriate log levels:
  - `trace!` - Very verbose, disabled in production
  - `debug!` - Debugging information
  - `info!` - Normal operation milestones
  - `warn!` - Degraded operation, fallbacks used
  - `error!` - Failures requiring attention

##### Metrics Collection (Optional)
- [ ] **Decision needed:** Use Prometheus, custom DB, or in-memory?
  - **Option A:** In-memory counters (simplest, lost on restart)
  - **Option B:** Store in `metrics` table (persistent, queryable)
  - **Option C:** Prometheus exporter (production-grade, requires server)

- [ ] If using database: Create `MetricsRepository`
  - Add table: `command_metrics` (command, implementation, timestamp, duration_ms, success)
  - Repository methods: `record_execution()`, `get_stats(time_range)`
  - Wire to AppContext: `pub metrics_repository: Arc<MetricsRepository>`

- [ ] Track per command:
  - Invocation count (legacy vs new)
  - Error count
  - P50/P95/P99 latency
  - Feature flag toggle events

**Acceptance Criteria:**
- Logging helper created and documented
- All commands log execution (implementation + timing)
- No sensitive data logged (verified via audit)
- Metrics strategy decided and documented in decision log
- If using metrics: Repository created and wired to AppContext

---

### Phase 1 Success Criteria
- ✅ AppContext matches legacy AppState functionality
- ✅ All schedulers start cleanly (`.start().await?` succeeds)
- ✅ AppContext::shutdown exists and completes without panicking
- ✅ Integration tests prove lifecycle works
- ✅ Build/CI targets point to new crate
- ✅ `cargo ci` passes for new crate
- ✅ Observability infrastructure ready (logging helper, metrics strategy decided)

---

## Phase 2: Command Migration (Days 3-13)

### Goal
Migrate 9 Tauri commands from `legacy/api/` to `crates/api/pulsearc-app` using feature flags.

### Migration Checklist Template
For each command file:

1. **Setup:**
   - [ ] Copy file to `crates/api/src/commands/`
   - [ ] Update imports: `use crate::context::AppContext;`
   - [ ] Remove legacy imports from `legacy/api/src/`

2. **Feature Flag Wrapper:**
   - [ ] Add feature flag check at command entry point
   - [ ] Use `unwrap_or(false)` for fail-safe default
   - [ ] Route to `new_implementation()` if enabled
   - [ ] Route to `legacy_implementation()` if disabled

3. **New Implementation:**
   - [ ] Implement using repositories from `crates/infra`
   - [ ] Implement using services from `crates/core`
   - [ ] Use `AppContext` fields (not individual service params)
   - [ ] Add error handling with context
   - [ ] Add `tracing` instrumentation (no `println!`)

4. **Legacy Isolation:**
   - [ ] Move legacy code to `legacy_implementation()` function
   - [ ] Add `#[allow(dead_code)]` to suppress warnings
   - [ ] Ensure legacy code compiles (no missing dependencies)
   - [ ] Add comment: "// LEGACY: Will be removed in Phase 5"

5. **Testing:**
   - [ ] Add unit tests for new implementation only
   - [ ] Add integration test comparing old vs new outputs
   - [ ] Test error paths (database failures, etc.)
   - [ ] Manual smoke test in UI

6. **Registration:**
   - [ ] Register command in `main.rs` invoke_handler
   - [ ] Verify command signature matches frontend expectations
   - [ ] Test command via `invoke()` from frontend

7. **Documentation:**
   - [ ] Update this tracking doc: mark task as completed
   - [ ] Update command docstring with migration notes
   - [ ] Add entry to decision log if any deviations occurred

---

### 2.1: Phase 4A - Core Commands (Days 3-4)

#### 4A.1: Database Commands (Day 3, 512 LOC)
**File:** `crates/api/src/commands/database.rs`
**Feature Flag:** `new_database_commands`
**Priority:** P1 (critical infrastructure)

**Commands:**
- `get_database_stats` - Database size, table counts, index stats
- `get_recent_snapshots` - Last N snapshots for UI display
- `vacuum_database` - SQLite VACUUM for space reclamation
- `get_database_health` - Health check (connectivity, corruption check)

**Dependencies:**
- `DatabaseStatsRepository` (to be created in `crates/infra/src/repositories/database_stats.rs`)
- `DbManager` (already in AppContext, passed to repository)

**Migration Notes:**
- **⚠️ Architecture Decision:** Create `DatabaseStatsRepository` following ADR-003 hexagonal pattern
- Repository encapsulates all database introspection queries (stats, health, vacuum)
- Avoid raw SQL in command layer — all queries go through repository
- Repository port trait: `DatabaseStatsPort` in `crates/core/src/ports/`
- Repository implementation: `DatabaseStatsRepository` in `crates/infra/src/repositories/`
- Keep legacy path for 2-week validation

**Repository Methods to Implement:**
```rust
pub trait DatabaseStatsPort: Send + Sync {
    async fn get_database_size(&self) -> Result<u64>;
    async fn get_table_stats(&self) -> Result<Vec<TableStats>>;
    async fn get_index_stats(&self) -> Result<Vec<IndexStats>>;
    async fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<SnapshotSummary>>;
    async fn vacuum_database(&self) -> Result<VacuumResult>;
    async fn check_database_health(&self) -> Result<DatabaseHealth>;
}
```

**Checklist:**
- [ ] **PREREQUISITE:** Create `DatabaseStatsPort` trait in `crates/core/src/ports/database_stats.rs`
- [ ] **PREREQUISITE:** Implement `DatabaseStatsRepository` in `crates/infra/src/repositories/database_stats.rs`
- [ ] **PREREQUISITE:** Add repository to `AppContext` (wire with `DbManager`)
- [ ] Copy `legacy/api/src/commands/database.rs` → `crates/api/src/commands/database.rs`
- [ ] Add feature flag wrapper: `new_database_commands`
- [ ] Implement new path using `context.database_stats_repository`
- [ ] Isolate legacy path in `legacy_*` functions
- [ ] Add unit tests for repository (with mock DbManager)
- [ ] Add integration test (compare old vs new outputs)
- [ ] Register commands in `main.rs`
- [ ] Manual smoke test
- [ ] Update tracking doc

**Acceptance Criteria:**
- All 4 commands compile and pass tests
- Feature flag toggles between old/new successfully
- Manual test: UI displays database stats correctly

---

#### 4A.2: User Profile Commands (Day 4, 49 LOC)
**File:** `crates/api/src/commands/user_profile.rs`
**Feature Flag:** `new_user_profile_commands`
**Priority:** P1 (core user data)

**Commands:**
- `get_user_profile` - Fetch current user profile
- `upsert_user_profile` - Create or update profile

**Dependencies:**
- `UserProfileRepository` (already exists in `crates/infra`)

**Migration Notes:**
- Repository already tested (7/7 tests passing)
- Straightforward CRUD operations
- Low risk, good first feature migration

**Checklist:**
- [ ] Copy `legacy/api/src/commands/user_profile.rs` → `crates/api/src/commands/user_profile.rs`
- [ ] Add feature flag wrapper: `new_user_profile_commands`
- [ ] Implement new path using `UserProfileRepository`
- [ ] Isolate legacy path
- [ ] Add tests
- [ ] Register commands
- [ ] Manual smoke test
- [ ] Update tracking doc

**Acceptance Criteria:**
- Profile CRUD works in UI
- Feature flag toggles correctly
- No regressions in profile data

---

#### 4A.3: Window Commands (Day 4, 61 LOC)
**File:** `crates/api/src/commands/window.rs`
**Feature Flag:** `new_window_commands`
**Priority:** P1 (UI-only, low risk)

**Commands:**
- `show_animation` - Trigger window animation on macOS
- `hide_window` - Hide main window

**Dependencies:**
- None (UI-only, uses Tauri window API)

**Migration Notes:**
- No database access
- No repositories needed
- UI-only logic, very low risk
- Consider skipping feature flag (unnecessary complexity)

**Checklist:**
- [ ] Copy `legacy/api/src/commands/window.rs` → `crates/api/src/commands/window.rs`
- [ ] Add feature flag wrapper (optional): `new_window_commands`
- [ ] Implement new path (likely identical to legacy)
- [ ] Add basic tests
- [ ] Register commands
- [ ] Manual UI test
- [ ] Update tracking doc

**Acceptance Criteria:**
- Window animations work in macOS
- No visual regressions

---

### 2.2: Phase 4B - Feature Commands (Days 5-9)

#### 4B.1: Block Building Commands (Days 5-6, 632 LOC)
**File:** `crates/api/src/commands/blocks.rs`
**Feature Flag:** `new_block_commands`
**Priority:** P1 (critical feature, high complexity)

**Commands:**
- `build_my_day` - Trigger block building workflow
- `get_proposed_blocks` - Fetch pending block proposals
- `accept_proposed_block` - Accept a block suggestion
- `dismiss_proposed_block` - Dismiss a block suggestion

**Dependencies:**
- `BlockScheduler` (added to AppContext in Phase 1)
- `BlockRepository` (from `crates/infra`)
- `BlockBuildingService` (from `crates/core`)

**Migration Notes:**
- **⚠️ HIGH COMPLEXITY:** Complex business logic, multi-step workflow
- Requires property-based testing (not just unit tests)
- Extended validation period (2 weeks minimum)
- Keep legacy path for 4 weeks (double the standard period)

**Checklist:**
- [ ] Copy `legacy/api/src/commands/blocks.rs` → `crates/api/src/commands/blocks.rs`
- [ ] Add feature flag wrapper: `new_block_commands`
- [ ] Implement new path using `BlockBuildingService`
- [ ] Isolate legacy path
- [ ] Add unit tests (happy path + error paths)
- [ ] Add property-based tests (quickcheck/proptest)
- [ ] Add integration tests (compare old vs new outputs)
- [ ] Register commands
- [ ] Manual smoke test (build full day workflow)
- [ ] Update tracking doc

**Acceptance Criteria:**
- All 4 commands compile and pass tests
- Property tests verify business logic invariants
- Integration tests show <5% output difference from legacy
- Manual test: Build My Day completes successfully

**Risk Mitigation:**
- Keep legacy path for 4 weeks (extended validation)
- Add detailed logging for comparison
- Monitor error rates daily during rollout

---

#### 4B.2: Calendar Integration (Days 7-8, 946 LOC)
**File:** `crates/api/src/commands/calendar.rs`
**Feature Flag:** `new_calendar_commands`
**Priority:** P1 (critical feature, highest risk)

**Commands:**
- `initiate_calendar_auth` - Start OAuth flow
- `handle_oauth_callback` - Handle OAuth redirect
- `sync_calendar_events` - Sync events from Google Calendar
- `get_calendar_events_for_timeline` - Fetch events for UI

**Dependencies:**
- `CalendarSyncScheduler` (added to AppContext in Phase 1, feature-gated)
- OAuth token manager (keychain integration)
- `CalendarRepository` (from `crates/infra`)

**Migration Notes:**
- **⚠️ HIGHEST RISK:** Largest file (946 LOC), OAuth complexity
- Token refresh logic is fragile
- Keychain access requires macOS permissions
- Extended validation period (4 weeks)
- Consider splitting into multiple sub-tasks

**Checklist:**
- [ ] Copy `legacy/api/src/commands/calendar.rs` → `crates/api/src/commands/calendar.rs`
- [ ] Add feature flag wrapper: `new_calendar_commands`
- [ ] Implement new OAuth flow using new token manager
- [ ] Implement new sync logic using `CalendarRepository`
- [ ] Isolate legacy path
- [ ] Add unit tests (OAuth flow, token refresh)
- [ ] Add integration tests (mock OAuth server)
- [ ] Add manual test checklist (full OAuth flow)
- [ ] Register commands
- [ ] Manual smoke test (connect calendar, sync events)
- [ ] Update tracking doc

**Acceptance Criteria:**
- OAuth flow completes successfully
- Token refresh works after 1 hour (Google tokens expire)
- Events sync correctly (no duplicates, no missing events)
- Feature flag toggles correctly
- Manual test: Full calendar connection workflow

**Risk Mitigation:**
- Keep legacy path for 4 weeks (extended validation)
- Add comprehensive logging (no secrets!)
- Test token refresh manually after 1 hour
- Monitor OAuth error rates closely

---

#### 4B.3: Idle Management Commands (Day 9, 193 LOC)
**File:** `crates/api/src/commands/idle.rs`
**Feature Flag:** `new_idle_commands`
**Priority:** P1 (core feature)

**Commands:**
- `get_idle_periods` - Fetch idle periods for date range
- `update_idle_period_action` - Mark idle period as work/break/personal
- `get_idle_summary` - Summary stats for idle time

**Dependencies:**
- `IdlePeriodsRepository` (already exists in `crates/infra`, 6/6 tests passing)

**Migration Notes:**
- Repository already tested and working
- Straightforward CRUD operations
- Medium complexity (some business logic in summary calculation)

**Checklist:**
- [ ] Copy `legacy/api/src/commands/idle.rs` → `crates/api/src/commands/idle.rs`
- [ ] Add feature flag wrapper: `new_idle_commands`
- [ ] Implement new path using `IdlePeriodsRepository`
- [ ] Isolate legacy path
- [ ] Add unit tests (CRUD + summary calculation)
- [ ] Add integration tests
- [ ] Register commands
- [ ] Manual smoke test
- [ ] Update tracking doc

**Acceptance Criteria:**
- All 3 commands compile and pass tests
- Idle periods display correctly in UI
- Summary calculation matches legacy

---

### 2.3: Phase 4C - Monitoring Commands (Days 10-12)

#### 4C.1: Monitoring & Stats Commands (Days 10-11, 741 LOC)
**File:** `crates/api/src/commands/monitoring.rs`
**Feature Flag:** `new_monitoring_commands`
**Priority:** P2 (monitoring, non-critical)

**Commands:**
- `get_sync_stats` - Sync queue stats (pending, failed, succeeded)
- `get_outbox_status` - Outbox worker status (last run, next run, errors)
- `get_cost_tracking_stats` - Cost tracker stats (API costs, limits)
- `get_classification_metrics` - Classification accuracy, model stats

**Dependencies:**
- `OutboxWorker` (Phase 3D complete)
- `CostTracker` (Phase 3D complete)
- `MetricsTracker` (feature-gated, optional)
- `SyncScheduler` (added to AppContext in Phase 1)

**Migration Notes:**
- Medium complexity (stats aggregation)
- Non-critical for core functionality (monitoring only)
- Can be migrated with lower risk tolerance
- Consider splitting into sub-tasks if too large

**Checklist:**
- [ ] Copy `legacy/api/src/commands/monitoring.rs` → `crates/api/src/commands/monitoring.rs`
- [ ] Add feature flag wrapper: `new_monitoring_commands`
- [ ] Implement new path using workers/trackers
- [ ] Isolate legacy path
- [ ] Add unit tests (stats calculations)
- [ ] Add integration tests
- [ ] Register commands
- [ ] Manual smoke test (check stats in UI)
- [ ] Update tracking doc

**Acceptance Criteria:**
- All 4 commands compile and pass tests
- Stats match legacy calculations
- UI displays monitoring data correctly

---

#### 4C.2: Idle Sync Telemetry (Day 12, 58 LOC)
**File:** `crates/api/src/commands/idle_sync.rs`
**Feature Flag:** `new_idle_sync_commands`
**Priority:** P2 (telemetry, non-critical)

**Commands:**
- `get_idle_sync_stats` - Idle period sync stats

**Dependencies:**
- `IdlePeriodsRepository` (already exists)
- Sync telemetry (may need new queries)

**Migration Notes:**
- Very small file (58 LOC)
- Low complexity
- Low risk (telemetry only)

**Checklist:**
- [ ] Copy `legacy/api/src/commands/idle_sync.rs` → `crates/api/src/commands/idle_sync.rs`
- [ ] Add feature flag wrapper: `new_idle_sync_commands`
- [ ] Implement new path
- [ ] Isolate legacy path
- [ ] Add basic tests
- [ ] Register command
- [ ] Manual smoke test
- [ ] Update tracking doc

**Acceptance Criteria:**
- Command compiles and passes tests
- Stats displayed in UI

---

### 2.4: Phase 4E - Dev Tools (Day 13)

#### 4E.1: Seed Snapshots Command (Day 13, 193 LOC)
**File:** `crates/api/src/commands/seed_snapshots.rs`
**Feature Flag:** `new_seed_commands` (or skip flag since debug-only)
**Priority:** P3 (dev tool, debug builds only)

**Commands:**
- `seed_snapshots` - Generate test data (debug builds only)

**Dependencies:**
- `SnapshotRepository` (from `crates/infra`)
- Various test data generators

**Migration Notes:**
- Debug builds only (`#[cfg(debug_assertions)]`)
- Not user-facing (dev tool only)
- Can skip feature flag (unnecessary complexity)
- Lowest priority (can defer if time constrained)

**Checklist:**
- [ ] Copy `legacy/api/src/commands/seed_snapshots.rs` → `crates/api/src/commands/seed_snapshots.rs`
- [ ] Add `#[cfg(debug_assertions)]` guard
- [ ] Implement using `SnapshotRepository`
- [ ] Add basic tests (debug build only)
- [ ] Register command (debug build only)
- [ ] Manual test
- [ ] Update tracking doc

**Acceptance Criteria:**
- Command works in debug builds
- Command is not included in release builds
- Test data generated correctly

---

### Phase 2 Success Criteria
- ✅ All 9 commands migrated and tested
- ✅ Feature flags wrap all command entry points
- ✅ Legacy code isolated and compilable
- ✅ `cargo ci` passes for new crate
- ✅ All commands registered in `main.rs`
- ✅ Manual smoke tests completed for each command

---

## Phase 3: Frontend Integration (Days 14-15)

### Goal
Ensure frontend works seamlessly with new crate and update documentation.

### Tasks

#### 3.1: Verify Frontend Compatibility

##### Command Signature Audit
- [ ] Audit frontend `invoke()` calls for command signatures
- [ ] Verify all command names match legacy (no breaking changes)
- [ ] Verify all parameter types match legacy
- [ ] Verify all return types match legacy

##### TypeScript Type Generation & Synchronization
- [ ] **Verify current ts-gen setup:** Check if `ts-gen` feature is still the active approach
  - Review `crates/api/Cargo.toml` for `ts-rs` dependency and feature flag
  - Check if bindings are generated to `crates/api/bindings/` or another location
  - **Implementation Note:** Confirm `ts-gen` is the correct feature switch before wiring into CI. The type generation strategy may have evolved since ADR-002.

- [ ] Generate TypeScript types from new crate:
  ```bash
  cargo build -p pulsearc-app --features ts-gen
  # Bindings generated to: crates/api/bindings/ (verify location)
  ```

- [ ] Compare generated types vs legacy types:
  ```bash
  # If legacy bindings exist:
  diff -r legacy/api/bindings/ crates/api/bindings/
  # Document any differences
  ```

- [ ] **BLOCKER if types changed:** Decide on mitigation strategy
  - **Option A:** Update frontend to use new types (requires frontend changes)
  - **Option B:** Fix type generation to match legacy (adjust `#[ts(type = "...")]` attributes)
  - **Option C:** Accept breaking change (requires coordinated frontend PR)
  - Document choice in decision log

- [ ] Update frontend imports (if binding path changed):
  ```typescript
  // Old: import { DatabaseStats } from '../bindings/legacy/api/...'
  // New: import { DatabaseStats } from '../bindings/crates/api/...'
  ```

- [ ] **CI Integration:** Add type generation check to CI
  ```yaml
  # .github/workflows/ci.yml or equivalent
  - name: Verify TypeScript Types
    run: |
      cargo build -p pulsearc-app --features ts-gen
      # Fail if bindings directory is dirty (types changed unexpectedly)
      git diff --exit-code crates/api/bindings/
  ```

**Implementation Note:** Double-check that the `ts-gen` feature is still the right switch for type generation. If the project has moved to a different approach (e.g., `specta`, manual types, or different `ts-rs` configuration), update this step accordingly before wiring into CI.

**Risk:** Frontend may break if response types changed (field added/removed/renamed).

**Mitigation:** Run `ts-gen` in CI and fail on type mismatches. Coordinate frontend updates with backend changes.

**Acceptance Criteria:**
- TypeScript types generated successfully
- Type comparison completed (differences documented if any)
- Frontend imports updated (if needed)
- CI checks type generation
- No type-related runtime errors in frontend

---

#### 3.2: Test Error Handling
- [ ] Test error handling (error strings must match frontend expectations)
- All `invoke()` calls work with new crate
- Error messages are user-friendly

---

#### 3.2: Add Runtime Switch (Optional)
**Goal:** Allow QA to test both old and new crates side-by-side

- [ ] Add CLI flag: `--use-legacy-api` (optional)
- [ ] Wire flag to global feature flag toggle (set all `new_*` flags to false)
- [ ] Document flag in README
- [ ] Test: Launch app with `--use-legacy-api`, verify all commands route to legacy

**Acceptance Criteria:**
- CLI flag toggles all feature flags
- QA can test both implementations without code changes

**Decision:** Skip this if complexity is too high (feature flags in database already provide toggle mechanism)

---

#### 3.3: Update Documentation
**Files to Update:**
- [ ] `docs/PHASE-4-API-REWIRING-TRACKING.md` - Add note redirecting to this doc
- [ ] `docs/PHASE-4-START-CHECKLIST.md` - Update prerequisites
- [ ] `docs/adr/003-legacy-migration.md` - Add Phase 4 migration notes
- [ ] `crates/api/README.md` - Add migration context and status
- [ ] `CLAUDE.md` - Update API crate references (if needed)

**Content to Add to `crates/api/README.md`:**
```markdown
## Migration Status

This crate is actively being migrated from `legacy/api/` as part of **Phase 4: API Rewiring**.

**Migration Progress:** X/9 commands migrated (see [PHASE-4-NEW-CRATE-MIGRATION.md](../../docs/active-issue/PHASE-4-NEW-CRATE-MIGRATION.md))

**Feature Flags:** All commands are wrapped with feature flags for gradual rollout.
- Default: Fail-safe to legacy path on error (`unwrap_or(false)`)
- Toggle flags in database via `feature_flags` table
- Rollback: Toggle flag to `false`, restart app

**Timeline:**
- Migration: Weeks 1-2
- Validation: Weeks 3-4
- Cleanup: Week 5
```

**Acceptance Criteria:**
- All documentation updated
- Links work correctly
- Migration status is clear to new contributors

---

### Phase 3 Success Criteria
- ✅ Frontend works with new crate (no code changes)
- ✅ Documentation updated and accurate
- ✅ Optional: Runtime switch for QA testing

---

## Phase 4: Validation Period (Weeks 3-4, Days 16-29)

### Goal
Validate new implementation in production with staged rollout.

### 4.1: Staged Rollout Schedule

#### Days 16-17: Internal Testing
- [ ] Enable feature flags for developers only (manual toggle)
- [ ] Run full manual test suite (all 9 commands)
- [ ] Monitor logs for errors
- [ ] Document any issues in tracking doc

**Success Criteria:**
- No P0/P1 issues found
- All commands work as expected
- Developers approve for beta testing

---

#### Days 18-22: Beta Rollout (10%)
- [ ] Enable feature flags for 10% of users (manual selection)
- [ ] Monitor error rates per command (target: <1%)
- [ ] Monitor latency percentiles (target: no regression)
- [ ] Track feature flag toggle frequency (rollback indicator)
- [ ] Daily check-in: review metrics, user feedback

**Metrics to Track:**
- Error rate per command (target: <1%)
- P95 latency per command (target: no >20% regression)
- Feature flag toggles (frequent toggles = problems)
- User bug reports (target: 0)

**Rollback Triggers:**
- P0 issue: Data loss, security vulnerability → immediate rollback
- P1 issue: Critical feature broken, >5% error rate → rollback
- Multiple P2 issues: >3 P2 bugs in same command → rollback

**Acceptance Criteria:**
- <1% error rate
- No performance regressions
- No user complaints

---

#### Days 23-29: Full Rollout (100%)
- [ ] Enable feature flags for all users
- [ ] Continue monitoring metrics
- [ ] Daily check-in for first 3 days
- [ ] Reduce to weekly check-in after 3 days

**Success Criteria:**
- 7 days without P0/P1 issues
- <1% error rate maintained
- No user complaints
- Performance targets met

---

### 4.2: Monitoring Metrics

**Error Rates (per command):**
- Target: <1%
- Alert threshold: >5%
- Rollback threshold: >10%

**Latency Percentiles (per command):**
- Target: No regression from baseline
- Alert threshold: >20% increase
- Rollback threshold: >50% increase

**Feature Flag Toggles:**
- Target: 0 toggles (no rollbacks)
- Alert threshold: >1 toggle per command
- Rollback threshold: >3 toggles in 24 hours

**User Feedback:**
- Target: 0 bug reports
- Alert threshold: >1 bug report
- Rollback threshold: >3 bug reports for same issue

---

### 4.3: Rollback Procedure

**When to Rollback:**
1. **P0 Issue:** Data loss, security vulnerability
2. **P1 Issue:** Critical feature broken, >5% error rate
3. **Multiple P2 Issues:** >3 bugs in same command

**How to Rollback:**
1. Toggle feature flag to `false` in database:
   ```sql
   UPDATE feature_flags SET is_enabled = 0 WHERE name = 'new_[command]';
   ```
2. Restart app (feature flags are database-persisted)
3. Verify rollback: Command routes to legacy path
4. Monitor metrics: Confirm issue is resolved
5. Investigate root cause
6. Fix issue and re-enable flag

**Time to Rollback:** <2 minutes (toggle flag + restart)

**Acceptance Criteria:**
- Rollback procedure tested in Phase 1
- All team members know how to rollback
- Rollback time meets <2 minute target

---

### Phase 4 Success Criteria
- ✅ 2 weeks (14 days) without P0/P1 issues
- ✅ <1% error rate maintained
- ✅ No performance regressions
- ✅ No user complaints
- ✅ Feature flags enabled for 100% of users

---

## Phase 5: Cleanup & Deprecation (Week 5, Days 30-34)

### Goal
Remove feature flags, delete legacy code, and archive legacy crate.

### 5.1: Remove Feature Flag Wrappers (Days 30-31)

**For each command file:**
- [ ] Remove `if use_new { ... } else { ... }` wrapper
- [ ] Delete `legacy_implementation()` function
- [ ] Remove `#[allow(dead_code)]` annotations
- [ ] Remove feature flag check from command entry point
- [ ] Update tests to only test new implementation
- [ ] Remove integration tests comparing old vs new

**Example:**
```rust
// BEFORE (with feature flag):
#[tauri::command]
pub async fn my_command(context: State<'_, AppContext>) -> Result<Data, String> {
    let use_new = context.feature_flags.is_enabled("new_my_cmd", true).await.unwrap_or(false);
    if use_new {
        new_implementation(&context).await
    } else {
        legacy_implementation(&context).await
    }
}

// AFTER (feature flag removed):
#[tauri::command]
pub async fn my_command(context: State<'_, AppContext>) -> Result<Data, String> {
    new_implementation(&context).await  // Only new path remains
}
```

**Acceptance Criteria:**
- All feature flag checks removed
- All legacy code paths deleted
- All tests updated
- `cargo ci` passes

---

### 5.2: Archive Legacy Crate (Days 32-33)

#### Step 1: Move Legacy Crate
- [ ] Create `archived/` directory at workspace root
- [ ] Move `legacy/api/` to `archived/legacy-api-2025-10-31/`
- [ ] Add `archived/README.md` with context:
  ```markdown
  # Archived Code

  This directory contains legacy code that has been migrated to the new architecture.

  ## legacy-api-2025-10-31/
  Original Tauri API crate, migrated to `crates/api/pulsearc-app` in Phase 4.
  See: docs/PHASE-4-NEW-CRATE-MIGRATION.md
  ```

#### Step 2: Remove from Workspace
- [ ] Remove `legacy/api` from `Cargo.toml` workspace members
- [ ] Run `cargo check` to verify workspace compiles
- [ ] Remove legacy from CI (if separate CI job exists)

#### Step 3: Update Imports
- [ ] Search codebase for `use legacy::` imports
- [ ] Remove or update any remaining references
- [ ] Run `cargo check --workspace` to verify

**Acceptance Criteria:**
- Legacy crate archived
- Workspace compiles without legacy
- No dangling imports
- CI passes without legacy crate

---

### 5.3: Update Documentation (Day 34)

#### Create Retrospective Document
**File:** `docs/PHASE-4-RETROSPECTIVE.md`

**Content:**
- What went well
- What didn't go well
- Lessons learned
- Metrics summary (error rates, performance, timeline)
- Recommendations for future migrations

#### Update Existing Docs
- [ ] Update `CLAUDE.md` to remove legacy references
- [ ] Update `README.md` to reference new crate only
- [ ] Update ADR-003 with Phase 4 completion notes
- [ ] Update `crates/api/README.md` to remove "Migration Status" section
- [ ] Archive Phase 4 tracking docs to `docs/archive/phase-4/`

**Acceptance Criteria:**
- Retrospective document created
- All references to legacy crate removed
- Documentation reflects current architecture

---

### Phase 5 Success Criteria
- ✅ All feature flags removed
- ✅ All legacy code deleted from new crate
- ✅ Legacy crate archived
- ✅ Workspace compiles without legacy
- ✅ Documentation updated
- ✅ Retrospective created

---

## Risk Register

### High-Risk Commands

#### 1. Calendar Integration (946 LOC)
**Risk Level:** 🔴 CRITICAL

**Risks:**
- OAuth token refresh logic is fragile
- Keychain access requires macOS permissions
- Third-party API (Google Calendar) may change
- Largest file, most complexity

**Mitigations:**
- Extended validation period (4 weeks vs 2 weeks)
- Comprehensive OAuth testing (manual + automated)
- Monitor token refresh success rate
- Keep legacy path for 4 weeks
- Add detailed logging (no secrets!)

**Rollback Plan:**
- Toggle `new_calendar_commands` to `false`
- Restart app
- Monitor OAuth success rate
- Investigate logs

---

#### 2. Block Building (632 LOC)
**Risk Level:** 🟡 HIGH

**Risks:**
- Complex business logic (multi-step workflow)
- Critical user-facing feature
- Performance sensitive (large data sets)

**Mitigations:**
- Property-based testing (verify invariants)
- Integration tests comparing old vs new outputs
- Extended validation period (4 weeks)
- Monitor latency percentiles closely
- Add detailed performance logging

**Rollback Plan:**
- Toggle `new_block_commands` to `false`
- Restart app
- Verify block building works with legacy
- Compare outputs for correctness

---

### Medium-Risk Commands

#### 3. Monitoring & Stats (741 LOC)
**Risk Level:** 🟡 MEDIUM

**Risks:**
- Stats aggregation logic is complex
- Non-critical but user-visible
- Depends on OutboxWorker and CostTracker

**Mitigations:**
- Unit tests for stats calculations
- Compare stats outputs (old vs new)
- Standard validation period (2 weeks)

---

### Low-Risk Commands

#### 4-10. All Other Commands
**Risk Level:** 🟢 LOW

**Reasons:**
- Small files (<200 LOC each)
- Straightforward CRUD operations
- Well-tested repositories
- Non-critical or UI-only

**Mitigations:**
- Standard validation period (2 weeks)
- Basic unit + integration tests
- Manual smoke tests

---

### Infrastructure Risks

#### 1. AppContext Initialization
**Risk Level:** 🟡 MEDIUM

**Risk:**
- If any scheduler fails to start, app won't launch
- Complex dependency graph

**Mitigations:**
- Phase 1 focuses exclusively on AppContext
- Integration test proves lifecycle works
- Error handling: If any scheduler fails, `AppContext::new()` returns Err
- Fallback: Can temporarily comment out failing schedulers

---

#### 2. Build Configuration
**Risk Level:** 🟢 LOW

**Risk:**
- Build scripts may point to wrong crate
- CI may test wrong crate

**Mitigations:**
- Update build configs in Phase 1 (before command migration)
- Verify with `make dev`, `make ci`, `cargo tauri build`
- Test early, test often

---

#### 3. Feature Flag Failures
**Risk Level:** 🟡 MEDIUM

**Risk:**
- Feature flag lookup may fail (database connection error)
- Default behavior matters

**Mitigations:**
- **Fail-safe default:** `unwrap_or(false)` → legacy path
- Test feature flag mechanism in Phase 1
- Monitor feature flag lookup error rates

---

## Progress Tracking

### Phase 1: Infrastructure Baseline

| Task | Status | Started | Completed | Notes |
|------|--------|---------|-----------|-------|
| 1.1: Verify Baseline | ⏸️ Pending | - | - | - |
| 1.2: Expand AppContext | ⏸️ Pending | - | - | - |
| 1.3: Wire Schedulers | ⏸️ Pending | - | - | - |
| 1.4: Implement Shutdown | ⏸️ Pending | - | - | - |
| 1.5: Update Build Config | ⏸️ Pending | - | - | - |

---

### Phase 2: Command Migration

| Phase | Command File | LOC | Priority | Status | Started | Completed | Notes |
|-------|--------------|-----|----------|--------|---------|-----------|-------|
| 4A.1 | database.rs | 512 | P1 | ⏸️ Pending | - | - | - |
| 4A.2 | user_profile.rs | 49 | P1 | ⏸️ Pending | - | - | - |
| 4A.3 | window.rs | 61 | P1 | ⏸️ Pending | - | - | - |
| 4B.1 | blocks.rs | 632 | P1 | ⏸️ Pending | - | - | High risk |
| 4B.2 | calendar.rs | 946 | P1 | ⏸️ Pending | - | - | Highest risk |
| 4B.3 | idle.rs | 193 | P1 | ⏸️ Pending | - | - | - |
| 4C.1 | monitoring.rs | 741 | P2 | ⏸️ Pending | - | - | - |
| 4C.2 | idle_sync.rs | 58 | P2 | ⏸️ Pending | - | - | - |
| 4E.1 | seed_snapshots.rs | 193 | P3 | ⏸️ Pending | - | - | Debug only |

**Total:** 3,385 LOC

---

### Phase 3: Frontend Integration

| Task | Status | Started | Completed | Notes |
|------|--------|---------|-----------|-------|
| 3.1: Verify Compatibility | ⏸️ Pending | - | - | - |
| 3.2: Add Runtime Switch | ⏸️ Pending | - | - | Optional |
| 3.3: Update Documentation | ⏸️ Pending | - | - | - |

---

### Phase 4: Validation

| Stage | Duration | Status | Started | Completed | Notes |
|-------|----------|--------|---------|-----------|-------|
| Internal Testing | Days 16-17 | ⏸️ Pending | - | - | - |
| Beta Rollout (10%) | Days 18-22 | ⏸️ Pending | - | - | - |
| Full Rollout (100%) | Days 23-29 | ⏸️ Pending | - | - | - |

---

### Phase 5: Cleanup

| Task | Status | Started | Completed | Notes |
|------|--------|---------|-----------|-------|
| 5.1: Remove Feature Flags | ⏸️ Pending | - | - | - |
| 5.2: Archive Legacy Crate | ⏸️ Pending | - | - | - |
| 5.3: Update Documentation | ⏸️ Pending | - | - | - |

---

## Decision Log

### 2025-10-31: Initial Plan Created
**Decision:** Migrate Phase 4 work to `crates/api/pulsearc-app` instead of `legacy/api/`

**Rationale:**
- Clean architecture separation (hexagonal design)
- Easier to maintain long-term
- Legacy crate can be fully archived after validation

**Impacts:**
- All commands must be copied to new crate
- Build configuration must be updated
- Documentation must be updated

**Approved By:** User (via plan review)

---

### 2025-10-31: Feature Flag Default Changed
**Decision:** Use `unwrap_or(false)` instead of `unwrap_or(true)`

**Rationale:**
- Fail-safe: Any feature flag lookup error falls back to legacy path
- Reduces risk of forcing new implementation on users if flag system fails
- Allows for safer rollout

**Impacts:**
- Feature flags must be explicitly enabled (not enabled by default)
- Rollout requires manual flag toggling (not automatic)

**Approved By:** User (via plan feedback)

---

### 2025-10-31: Scheduler Initialization Strategy
**Decision:** All schedulers call `.start().await?` in `AppContext::new()`

**Rationale:**
- Ensures all background tasks are running before commands are registered
- Fail-fast: If any scheduler fails to start, app won't launch
- Easier to debug: Startup failures are clear and immediate

**Impacts:**
- AppContext::new() may fail (must handle errors gracefully)
- All scheduler constructors must return Result
- Integration test required to prove lifecycle works

**Approved By:** User (via plan feedback)

---

### 2025-10-31: Legacy Code Isolation
**Decision:** Keep legacy code compilable but isolated during migration

**Rationale:**
- Allows for clean deletion in Phase 5
- Prevents accidental coupling between old/new implementations
- Easier to test (can run both paths side-by-side)

**Impacts:**
- Legacy code lives in `legacy_*` functions with `#[allow(dead_code)]`
- May require temporary feature-gated dependency on `legacy/api/` crate
- Phase 5 cleanup is straightforward (delete isolated functions)

**Approved By:** User (via plan feedback)

---

### 2025-10-31: Build Config Priority
**Decision:** Update build configs (Makefile, xtask, tauri.conf.json) in Phase 1

**Rationale:**
- Ensures CI builds and tests new crate from the start
- Prevents diverging build entry points
- Catches build issues early

**Impacts:**
- Phase 1 is slightly longer (includes build config work)
- Must verify builds work before command migration starts

**Approved By:** User (via plan feedback)

---

## Appendices

### Appendix A: Feature Flag Naming Convention

**Pattern:** `new_[command_category]_commands`

**Examples:**
- `new_database_commands` - Database commands (get_database_stats, etc.)
- `new_user_profile_commands` - User profile commands (get_user_profile, etc.)
- `new_block_commands` - Block building commands (build_my_day, etc.)
- `new_calendar_commands` - Calendar integration (sync_calendar_events, etc.)
- `new_idle_commands` - Idle management (get_idle_periods, etc.)
- `new_monitoring_commands` - Monitoring & stats (get_sync_stats, etc.)
- `new_idle_sync_commands` - Idle sync telemetry
- `new_seed_commands` - Seed snapshots (debug only)
- `new_window_commands` - Window commands (optional)

**Database Schema:**
```sql
CREATE TABLE feature_flags (
    name TEXT PRIMARY KEY,
    is_enabled BOOLEAN NOT NULL DEFAULT 0,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

---

### Appendix B: Testing Checklist Template

**For each command migration:**

#### Unit Tests
- [ ] Happy path test (valid inputs, expected outputs)
- [ ] Error path test (invalid inputs, database failures)
- [ ] Edge case tests (empty results, large data sets, etc.)
- [ ] Mock dependencies (no database access in unit tests)

#### Integration Tests
- [ ] Compare old vs new outputs (side-by-side)
- [ ] Verify outputs are identical (or <5% difference with rationale)
- [ ] Test with real database (encrypted SQLCipher)
- [ ] Test feature flag toggle (old path → new path → old path)

#### Manual Tests
- [ ] Full workflow test (end-to-end in UI)
- [ ] Error handling test (trigger errors, verify user-friendly messages)
- [ ] Performance test (measure latency, compare to baseline)
- [ ] UI test (verify data displays correctly)

---

### Appendix C: Rollback Procedure

**Step-by-Step Rollback:**

1. **Identify Issue:**
   - P0: Data loss, security vulnerability
   - P1: Critical feature broken, >5% error rate
   - Decision: Rollback required

2. **Toggle Feature Flag:**
   ```sql
   -- Connect to database
   sqlite3 /path/to/pulsearc.db

   -- Disable feature flag
   UPDATE feature_flags SET is_enabled = 0 WHERE name = 'new_[command]';

   -- Verify
   SELECT name, is_enabled FROM feature_flags WHERE name = 'new_[command]';
   ```

3. **Restart App:**
   - Quit app (Cmd+Q)
   - Relaunch app
   - Feature flags are database-persisted (loaded at startup)

4. **Verify Rollback:**
   - Test command in UI
   - Check logs: Should see "Using legacy implementation"
   - Monitor error rates: Should return to baseline

5. **Investigate Root Cause:**
   - Review logs (filter by command name)
   - Identify error patterns
   - Fix issue in new implementation

6. **Re-enable (after fix):**
   ```sql
   UPDATE feature_flags SET is_enabled = 1 WHERE name = 'new_[command]';
   ```
   - Restart app
   - Test command
   - Monitor closely for 24 hours

**Time to Rollback:** <2 minutes (toggle flag + restart)

---

### Appendix D: Command Signature Reference

**Important:** Command signatures MUST remain identical to legacy for frontend compatibility.

**Pattern:**
```rust
#[tauri::command]
pub async fn my_command(
    context: State<'_, AppContext>,
    param1: String,
    param2: Option<i64>,
) -> Result<MyResponse, String> {
    // Implementation
}
```

**Rules:**
1. First parameter is always `context: State<'_, AppContext>`
2. Return type is always `Result<T, String>` (Tauri requirement)
3. Error strings must be user-friendly (no stack traces)
4. Use `#[serde(rename_all = "camelCase")]` for response structs (match frontend)

**Example Response Struct:**
```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyResponse {
    pub user_id: String,      // ← camelCase in JSON
    pub created_at: String,   // ← camelCase in JSON
}
```

---

## Summary

This document tracks the migration of 9 Tauri commands from `legacy/api/` to `crates/api/pulsearc-app` using database-persisted feature flags for safe rollout and rollback.

**Timeline:** 5 weeks
- Weeks 1-2: Migration (9 commands)
- Weeks 3-4: Validation (staged rollout)
- Week 5: Cleanup (remove legacy code)

**Success Metrics:**
- ✅ All 9 commands migrated and tested
- ✅ `cargo ci` passes
- ✅ 2 weeks without P0/P1 issues
- ✅ <1% error rate
- ✅ No performance regressions
- ✅ Legacy crate archived

**Next Steps:**
1. Review this plan with team
2. Get approval to start Phase 1
3. Begin infrastructure baseline work

---

**Document Status:** ✅ Ready for Review
**Created:** 2025-10-31
**Last Updated:** 2025-10-31
**Next Review:** Start of Phase 1
