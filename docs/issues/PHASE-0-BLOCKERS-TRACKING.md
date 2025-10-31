# Phase 0 Migration Blockers - Tracking Document

**Epic**: Phase 0 Refactoring (Pre-Migration)
**Status**: 🔴 Not Started
**Created**: 2025-10-30
**Target Completion**: 2025-11-08 (1 week from start)

---

## Overview

This document tracks the resolution of 6 critical blockers + 1 feature flag issue that must be resolved before Phase 1 of the ADR-003 migration can begin.

**Master Issue**: See [phase-0-migration-blockers.md](../../.github/ISSUE_TEMPLATE/phase-0-migration-blockers.md)

---

## Task Breakdown

### Quick Wins (Reclassifications)

#### Task 1.1: Reclassify `inference/batch_classifier.rs`
- **Status**: ⬜ Todo
- **Action**: Update LEGACY_MIGRATION_INVENTORY.md to reclassify from `core` → `infra`
- **Effort**: 0.5 days
- **Owner**: _Unassigned_
- **Checklist**:
  - [ ] Update `docs/LEGACY_MIGRATION_INVENTORY.md` (line 158)
  - [ ] Change "Target Crate" column from `❌ BLOCKED` to `infra`
  - [ ] Update "Target Path" to `infra/src/classification/batch_classifier.rs`
  - [ ] Change "Priority" to `✅ Priority 3`
  - [ ] Update row color/status badge

#### Task 1.2: Reclassify `integrations/sap/errors.rs`
- **Status**: ⬜ Todo
- **Action**: Update LEGACY_MIGRATION_INVENTORY.md to reclassify from `domain` → `infra`
- **Effort**: 0.5 days
- **Owner**: _Unassigned_
- **Checklist**:
  - [ ] Update `docs/LEGACY_MIGRATION_INVENTORY.md` (line 184)
  - [ ] Change "Target Crate" column from `❌ BLOCKED` to `infra`
  - [ ] Update "Target Path" to `infra/src/integrations/sap/errors.rs`
  - [ ] Change "Priority" to `✅ Priority 3`
  - [ ] Update row color/status badge

#### Task 1.3: Reclassify `integrations/sap/validation.rs`
- **Status**: ⬜ Todo
- **Action**: Update LEGACY_MIGRATION_INVENTORY.md to reclassify from `core` → `infra`
- **Effort**: 0.5 days
- **Owner**: _Unassigned_
- **Checklist**:
  - [ ] Update `docs/LEGACY_MIGRATION_INVENTORY.md` (line 185)
  - [ ] Change "Target Crate" column from `❌ BLOCKED` to `infra`
  - [ ] Update "Target Path" to `infra/src/integrations/sap/validation.rs`
  - [ ] Change "Priority" to `✅ Priority 3`
  - [ ] Update row color/status badge

---

### Feature Flags

#### Task 2.1: Add Missing Feature Flags to Cargo.toml
- **Status**: ⬜ Todo
- **Action**: Add `calendar`, `sap`, `ml` features to `legacy/api/Cargo.toml`
- **Effort**: 0.5 days
- **Owner**: _Unassigned_
- **Files**:
  - `legacy/api/Cargo.toml`
- **Implementation**:
  ```toml
  [features]
  default = ["tree-classifier"]
  tree-classifier = ["dep:linfa", "dep:linfa-trees", "dep:linfa-logistic", "dep:ndarray"]
  graphql = ["dep:graphql_client"]
  calendar = []  # NEW
  sap = []       # NEW
  ml = ["tree-classifier"]  # NEW (alias)
  ```

#### Task 2.2: Gate Feature-Flagged Modules
- **Status**: ⬜ Todo
- **Action**: Add `#[cfg(feature = "...")]` guards to relevant modules
- **Effort**: 1 day
- **Owner**: _Unassigned_
- **Depends On**: Task 2.1
- **Modules to Gate**:
  - Calendar: `integrations/calendar/**/*.rs`, `db/calendar/**/*.rs`, `commands/calendar.rs`
  - SAP: `integrations/sap/**/*.rs`
  - ML: `inference/*_classifier.rs`, `inference/training_*.rs`

---

### Critical Splits

#### Task 3.1: Split `shared/config.rs`
- **Status**: ⬜ Todo
- **Action**: Separate pure DTOs from I/O operations
- **Effort**: 2 days
- **Owner**: _Unassigned_
- **PR Checklist**:
  - [ ] Create `legacy/api/src/shared/config_types.rs` (pure DTOs)
  - [ ] Create `legacy/api/src/shared/config_loader.rs` (I/O operations)
  - [ ] Update all imports across codebase
  - [ ] Update fixtures and tests
  - [ ] Verify no env/filesystem access in `config_types.rs`
  - [ ] Add unit tests for loader
  - [ ] Update docs to classify `config_types.rs` → `domain`, `config_loader.rs` → `infra`

**Verification**:
```bash
# Ensure DTOs have no I/O
grep -n "std::env\|std::fs\|std::path::PathBuf" legacy/api/src/shared/config_types.rs
# Should return nothing
```

#### Task 3.2: Split `observability/errors/app.rs`
- **Status**: ⬜ Todo
- **Action**: Separate pure error enums from external conversions
- **Effort**: 2 days
- **Owner**: _Unassigned_
- **PR Checklist**:
  - [ ] Keep error enums in `observability/errors/app.rs`
  - [ ] Create `observability/errors/conversions.rs` for `From` impls
  - [ ] Move `From<rusqlite::Error>`, `From<reqwest::Error>`, `From<keyring::Error>` to conversions module
  - [ ] Update all error construction call-sites
  - [ ] Run full test suite
  - [ ] Update docs to classify enums → `domain`, conversions → `infra`

**Verification**:
```bash
# Ensure error types have no external deps
grep -n "rusqlite\|reqwest\|keyring" legacy/api/src/observability/errors/app.rs
# Should only show in comments/docs, not impl blocks
```

---

### Critical Refactoring

#### Task 4.1: Define Repository Ports for Segmenter
- **Status**: ⬜ Todo
- **Action**: Create port traits in preparation for segmenter refactor
- **Effort**: 1 day
- **Owner**: _Unassigned_
- **Deliverables**:
  - [ ] Create `legacy/api/src/core/ports/segment_repository.rs`
  - [ ] Define `SegmentRepository` trait
  - [ ] Define `SnapshotRepository` trait
  - [ ] Add trait documentation with expected behavior
  - [ ] Create mock implementations for testing

**Port Definition**:
```rust
// legacy/api/src/core/ports/segment_repository.rs
use pulsearc_common::error::CommonResult;

// ✅ CORRECT: Synchronous traits (SqlCipherPool API is synchronous)
pub trait SegmentRepository: Send + Sync {
    fn save_segment(&self, segment: &ActivitySegment) -> CommonResult<()>;
    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>>;
    fn find_unprocessed_segments(&self, limit: usize) -> CommonResult<Vec<ActivitySegment>>;
    fn mark_processed(&self, segment_id: &str) -> CommonResult<()>;
}

pub trait SnapshotRepository: Send + Sync {
    fn find_snapshots_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CommonResult<Vec<ActivitySnapshot>>;
    fn count_snapshots_by_date(&self, date: NaiveDate) -> CommonResult<usize>;
}
```

#### Task 4.2: Refactor `preprocess/segmenter.rs` to Use Ports
- **Status**: ⬜ Todo
- **Action**: Remove direct database dependencies from segmenter
- **Effort**: 2-3 days
- **Owner**: _Unassigned_
- **Depends On**: Task 4.1
- **PR Checklist**:
  - [ ] Refactor `Segmenter` to be generic over repository traits
  - [ ] Remove direct imports of `LocalDatabase`, `SegmentOperations`
  - [ ] Replace raw rusqlite queries with repository calls
  - [ ] Update all call-sites to inject repository implementations
  - [ ] Add integration tests with mock repositories
  - [ ] Add integration tests with real SQLite repositories
  - [ ] Verify no direct DB access in segmenter logic
  - [ ] Update docs to classify segmenter → `core`

**Before**:
```rust
// legacy/api/src/preprocess/segmenter.rs:5-15
use crate::db::activity::SegmentOperations;
use crate::db::local::LocalDatabase;  // ❌ Being deprecated

pub struct Segmenter {
    db: LocalDatabase,  // ❌ Direct DB dependency
}
```

**After**:
```rust
// legacy/api/src/preprocess/segmenter.rs
use crate::core::ports::{SegmentRepository, SnapshotRepository};

pub struct Segmenter<S, A>
where
    S: SegmentRepository,
    A: SnapshotRepository,
{
    segment_repo: S,
    snapshot_repo: A,
}
```

**Verification**:
```bash
# Ensure no direct DB imports (LocalDatabase or rusqlite)
grep -n "crate::db::\|rusqlite::\|LocalDatabase" legacy/api/src/preprocess/segmenter.rs
# Should return nothing
```

#### Task 4.3: Implement Repository Adapters in Infra
- **Status**: ⬜ Todo
- **Action**: Create SqlCipher-based implementations of repository ports
- **Effort**: 1 day
- **Owner**: _Unassigned_
- **Depends On**: Task 4.1, Task 4.2
- **Deliverables**:
  - [ ] Create `legacy/api/src/infra/repositories/segment_repository.rs`
  - [ ] Implement `SegmentRepository` using `SqlCipherConnection` (pooled)
  - [ ] Implement `SnapshotRepository` using `SqlCipherConnection` (pooled)
  - [ ] Move raw queries from segmenter to repository impls
  - [ ] Add repository-level unit tests
  - [ ] Wire up in DI container

**🚨 CRITICAL: Use SqlCipherPool, NOT LocalDatabase**

`LocalDatabase` is being **deprecated**. Use `SqlCipherPool` from `pulsearc_common::storage::sqlcipher` for pooled connections.

**Repository Implementation Pattern**:
```rust
// legacy/api/src/infra/repositories/segment_repository.rs
use std::sync::Arc;
use pulsearc_common::storage::sqlcipher::{SqlCipherPool, StorageResult};
use rusqlite::ToSql;
use crate::core::ports::SegmentRepository;

pub struct SqlCipherSegmentRepository {
    pool: Arc<SqlCipherPool>,  // ✅ Use SqlCipherPool (not SqlCipherConnection)
}

impl SegmentRepository for SqlCipherSegmentRepository {
    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>> {
        // ✅ CORRECT: Synchronous API (no .await, no async)
        let conn = self.pool.get_sqlcipher_connection()
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let sql = "SELECT id, start_time, end_time FROM segments WHERE date = ?1";
        let mut stmt = conn.prepare(sql)
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let date_str = date.to_string();
        // ✅ CORRECT: query_map returns Vec<T>, no .collect() needed
        // ✅ CORRECT: Use &[&dyn ToSql] - reference to date_str, not owned value
        let segments = stmt
            .query_map(&[&date_str as &dyn ToSql], |row| {
                Ok(ActivitySegment {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                })
            })
            .map_err(|e| CommonError::storage(e.to_string()))?;  // Already returns Vec

        Ok(segments)
    }
}
```

**🚨 CRITICAL API DIFFERENCES**:

`SqlCipherStatement::query_map` (line 116 in `crates/common/src/storage/sqlcipher/connection.rs`) returns `StorageResult<Vec<T>>`, NOT an iterator.

```rust
// ❌ WRONG - query_map already returns Vec<T>
let results = stmt
    .query_map(params, |row| Ok(MyStruct { ... }))?
    .collect::<Result<Vec<_>, _>>()  // ❌ ERROR: Vec<T> is not IntoIterator
    .map_err(|e| ...)?;

// ✅ CORRECT - query_map already collected
let results = stmt
    .query_map(params, |row| Ok(MyStruct { ... }))?;
```

**Why**: Unlike standard `rusqlite::Statement::query_map` which returns an iterator, `SqlCipherStatement::query_map` internally calls `.collect()` and returns the fully materialized `Vec<T>`.

---

## Progress Tracking

| Track | Tasks | Completed | In Progress | Blocked | Progress |
|-------|-------|-----------|-------------|---------|----------|
| **Track 1: Quick Wins** | 3 | 0 | 0 | 0 | 0% |
| **Track 2: Feature Flags** | 2 | 0 | 0 | 0 | 0% |
| **Track 3: Splits** | 2 | 0 | 0 | 0 | 0% |
| **Track 4: Segmenter** | 3 | 0 | 0 | 0 | 0% |
| **TOTAL** | **10** | **0** | **0** | **0** | **0%** |

---

## Daily Standup Template

**Date**: YYYY-MM-DD

**Completed Today**:
- Task X.Y: [Brief description]

**In Progress**:
- Task X.Y: [Brief description] - [Blocker if any]

**Planned for Tomorrow**:
- Task X.Y: [Brief description]

**Blockers**:
- None / [Description]

---

## Completion Checklist

### Phase 0 Complete When:
- [ ] All 10 tasks above marked as complete
- [ ] All PRs merged to main branch
- [ ] Migration inventory updated with final classifications
- [ ] Documentation updated (LEGACY_MIGRATION_INVENTORY.md, SHARED_TYPES_ANALYSIS.md)
- [ ] Validation commands pass:
  ```bash
  # No infra deps in domain-bound types
  grep -r "rusqlite\|reqwest\|keyring" legacy/api/src/shared/config_types.rs
  grep -r "rusqlite\|reqwest\|keyring" legacy/api/src/observability/errors/app.rs
  grep -r "crate::db::" legacy/api/src/preprocess/segmenter.rs

  # Feature flags present
  cargo metadata --format-version 1 | jq '.packages[] | select(.name == "pulsearc-legacy-api") | .features' | grep -E "calendar|sap|ml"

  # All tests pass
  cargo test --workspace
  cargo ci
  ```

---

## Risk Log

| Date | Risk | Impact | Mitigation | Status |
|------|------|--------|------------|--------|
| 2025-10-30 | Segmenter refactor touches hot path | High | Add comprehensive integration tests before refactoring | Open |
| 2025-10-30 | Error split affects many call-sites | Medium | Use IDE refactoring, run tests frequently | Open |

---

## Links

- **Master Issue**: [phase-0-migration-blockers.md](../../.github/ISSUE_TEMPLATE/phase-0-migration-blockers.md)
- **Migration Inventory**: [LEGACY_MIGRATION_INVENTORY.md](../LEGACY_MIGRATION_INVENTORY.md)
- **ADR-003**: [ADR-003-layered-architecture.md](../adr/ADR-003-layered-architecture.md)

---

**Last Updated**: 2025-10-30
**Status**: 🔴 Not Started
