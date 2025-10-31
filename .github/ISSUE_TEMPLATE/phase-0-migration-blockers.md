# Phase 0 Migration Blockers - Critical Refactoring Required

**Status**: 🔴 **BLOCKING** Phase 1 Migration
**Priority**: P0 (Critical)
**Estimated Effort**: 5-8 business days
**Target**: Must complete before ADR-003 Phase 1 begins

---

## Executive Summary

Six modules currently classified as `domain` or `core` in [LEGACY_MIGRATION_INVENTORY.md](../../docs/LEGACY_MIGRATION_INVENTORY.md) contain **side effects** that violate layered architecture rules. Additionally, feature flag definitions are misaligned between documentation and `Cargo.toml`.

**These issues block Phase 1** because they would introduce forbidden dependency edges (domain→infra, core→infra) if migrated as currently classified.

---

## Critical Blockers

### 1. `shared/config.rs` — Side Effects in Config Loading

**Issue**: Configuration loading mixes pure data structures with I/O operations.

**Location**: `legacy/api/src/shared/config.rs:27-108`

**Violations**:
- Reads environment variables (`std::env::var()`)
- Reads filesystem (`std::fs::read_to_string()`)
- Probes executable paths (`std::env::current_exe()`, `std::env::current_dir()`)

**Current Classification**: ❌ `domain` (INCORRECT)

**Required Action**: **SPLIT**
```rust
// domain/src/config/app_config.rs (Pure DTOs)
pub struct AppConfig {
    pub cache_duration: Duration,
    pub debug_activity: bool,
    pub detector_packs: HashMap<String, PackConfig>,
}

// infra/src/config/loader.rs (Side effects)
pub struct ConfigLoader;
impl ConfigLoader {
    pub fn load_from_env() -> AppResult<AppConfig> {
        // All env/filesystem access here
    }
}
```

**Estimated Effort**: 2 days (includes fixture/test updates)

---

### 2. `observability/errors/app.rs` — Infra Conversions in Error Types

**Issue**: Error types mix pure domain errors with infrastructure-specific conversions.

**Location**: `legacy/api/src/observability/errors/app.rs:363-396, 606-628`

**Violations**:
- Implements `From<rusqlite::Error>` (database infra)
- Implements `From<reqwest::Error>` (HTTP client infra)
- Implements `From<keyring::Error>` (keychain infra)

**Current Classification**: ❌ `domain` (PARTIALLY INCORRECT)

**Required Action**: **SPLIT**
```rust
// domain/src/errors/mod.rs (Pure error enums)
pub enum AppError {
    Db(DbError),
    Http(HttpError),
    Keychain(KeychainError),
}

// infra/src/errors/conversions.rs (External adapter conversions)
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        // Map to domain error
    }
}
```

**Estimated Effort**: 2 days (touches many call-sites/tests)

---

### 3. `integrations/sap/errors.rs` — Transport-Specific Error Wrapping

**Issue**: Directly wraps `reqwest::Error`, coupling to HTTP transport layer.

**Location**: `legacy/api/src/integrations/sap/errors.rs:58-78`

**Violations**:
- `from_reqwest()` method wraps transport errors
- Cannot exist in domain without infra dependency

**Current Classification**: ❌ `domain` (INCORRECT)

**Required Action**: **RECLASSIFY** → `infra/src/integrations/sap/errors.rs`

**Estimated Effort**: <1 day (documentation update + file move during migration)

---

### 4. `integrations/sap/validation.rs` — Direct Database Access

**Issue**: Constructs `WbsValidator` over `DbManager`/`WbsCache` directly.

**Location**: `legacy/api/src/integrations/sap/validation.rs:4-30`

**Violations**:
- Imports `crate::db::manager::DbManager`
- Uses `WbsCache` (database-backed cache)
- No repository port abstraction

**Current Classification**: ❌ `core` (INCORRECT)

**Required Action**: **RECLASSIFY** → `infra/src/integrations/sap/validation.rs`

**Alternative** (if validation logic needed in core):
1. Define `WbsCache` port trait in `core/src/integrations/ports.rs`
2. Move validation logic to `core`
3. Implement port in `infra`

**Estimated Effort**: <1 day (reclassify) OR 2 days (port abstraction)

---

### 5. `preprocess/segmenter.rs` — Raw Database Queries in Business Logic

**Issue**: Segmentation logic directly imports and uses database operations.

**Location**: `legacy/api/src/preprocess/segmenter.rs:5-421`

**Violations**:
- Direct imports: `crate::db::activity::SegmentOperations`
- Uses `LocalDatabase` directly
- Contains raw `rusqlite` queries

**Current Classification**: ❌ `core` (INCORRECT)

**Required Action**: **REFACTOR** (before migration)
```rust
// core/src/tracking/segmenter.rs (Business logic)
pub struct Segmenter<R: SegmentRepository> {
    repository: R,
}

// core/src/tracking/ports.rs (Port definition)
pub trait SegmentRepository: Send + Sync {
    fn save_segment(&self, segment: &ActivitySegment) -> Result<()>;
    fn find_segments_by_date(&self, date: NaiveDate) -> Result<Vec<ActivitySegment>>;
}

// infra/src/database/segment_repository.rs (Port implementation using SqlCipher)
use std::sync::Arc;
use pulsearc_common::storage::sqlcipher::{SqlCipherPool, StorageResult};
use rusqlite::ToSql;

pub struct SqlCipherSegmentRepository {
    pool: Arc<SqlCipherPool>,  // ✅ Use SqlCipherPool (pooled connections)
}

impl SegmentRepository for SqlCipherSegmentRepository {
    fn save_segment(&self, segment: &ActivitySegment) -> Result<()> {
        // ✅ Synchronous API (no .await)
        let conn = self.pool.get_sqlcipher_connection()?;
        let mut stmt = conn.prepare("INSERT INTO segments ...")?;

        let id = segment.id.as_str();
        // ✅ CRITICAL: query_map returns Vec<T>, no .collect() needed
        // ✅ Use &[&dyn ToSql] for parameters
        stmt.query_map(&[&id as &dyn ToSql], |row| Ok(()))?;  // Already returns Vec<T>
        Ok(())
    }
}
```

**🚨 CRITICAL**: Use `SqlCipherPool` from `pulsearc_common::storage::sqlcipher`, **NOT** `LocalDatabase` (being deprecated).

**API Differences**:
1. `SqlCipherStatement::query_map` returns `StorageResult<Vec<T>>` (already collected), unlike rusqlite which returns an iterator. Do **NOT** call `.collect()` on the result.
2. `SqlCipherPool::get_sqlcipher_connection()` is **synchronous** (no `.await`).
3. Parameters use `&[&dyn ToSql]` type, not owned values.

**Estimated Effort**: 3-4 days (largest refactoring effort)

**Recommendation**: Deliver as **pre-Phase-1 PR** to unblock core migration.

---

### 6. `inference/batch_classifier.rs` — Side Effects in Classifier

**Issue**: Batch classifier has infrastructure dependencies (DB, Tauri events, env vars).

**Location**: `legacy/api/src/inference/batch_classifier.rs:5-310`

**Violations**:
- Uses `DbManager` directly
- Uses `tauri::Emitter` (presentation layer concern)
- Reads `OPENAI_API_KEY` environment variable

**Current Classification**: ❌ `core` (INCORRECT)

**Required Action**: **RECLASSIFY** → `infra/src/classification/batch_classifier.rs`

**Note**: Entire module belongs in infra. Optional future split (pure strategy vs. transport) can happen post-migration.

**Estimated Effort**: <1 day (reclassify + update docs)

---

## Feature Flag Misalignment

**Issue**: Documentation references feature flags that don't exist in `Cargo.toml`.

**Documented Features** (in migration inventory):
- `calendar` ❌ Not in Cargo.toml
- `sap` ❌ Not in Cargo.toml
- `ml` ❌ Not in Cargo.toml
- `tree-classifier` ✅ Exists
- `graphql` ✅ Exists

**Current Cargo.toml** (`legacy/api/Cargo.toml:102-105`):
```toml
[features]
default = ["tree-classifier"]
tree-classifier = ["dep:linfa", "dep:linfa-trees", "dep:linfa-logistic", "dep:ndarray"]
graphql = ["dep:graphql_client"]
```

**Required Action**: **ADD MISSING FEATURES**

**Option A** (Recommended): Add features to Cargo.toml
```toml
[features]
calendar = []
sap = []
ml = ["tree-classifier"]  # Alias for ML features
```

**Option B**: Update documentation to match existing features
- Replace `calendar` → mark as "future feature"
- Replace `sap` → mark as "future feature"
- Replace `ml` → use `tree-classifier` instead

**Estimated Effort**: <1 day

**Recommendation**: **Option A** for explicit feature gating during migration.

---

## Recommended Actions & Sequencing

### Immediate (Week 0 - Phase 0)

| Task | Action | Owner | Effort | Blocker Priority |
|------|--------|-------|--------|------------------|
| **batch_classifier** | Reclassify to `infra` | TBD | <1 day | Low (doc update) |
| **sap/errors.rs** | Reclassify to `infra` | TBD | <1 day | Low (doc update) |
| **sap/validation.rs** | Reclassify to `infra` | TBD | <1 day | Low (doc update) |
| **Feature flags** | Add to Cargo.toml | TBD | <1 day | Medium (unblocks wiring) |
| **shared/config.rs** | Split (DTOs → domain, loader → infra) | TBD | 2 days | **HIGH** |
| **errors/app.rs** | Split (types → domain, conversions → infra) | TBD | 2 days | **HIGH** |
| **preprocess/segmenter.rs** | Refactor (add repository ports) | TBD | 3-4 days | **CRITICAL** |

### Parallel Execution Strategy

**Track 1** (Quick wins - Days 1-2):
1. Update inventory with reclassifications (batch_classifier, SAP modules)
2. Add feature flags to Cargo.toml
3. Gate modules with `#[cfg(feature = "...")]`

**Track 2** (Splits - Days 1-4):
1. Split `shared/config.rs` (Days 1-2)
2. Split `observability/errors/app.rs` (Days 3-4)
3. Update all call-sites and tests

**Track 3** (Critical path - Days 1-5):
1. Define `SegmentRepository` + `SnapshotRepository` ports in core (Day 1)
   - Use **synchronous** trait methods (SqlCipherPool API is synchronous, no async)
2. Refactor `preprocess/segmenter.rs` to use ports (Days 2-4)
3. Implement SqlCipher-based repository adapters in infra (Day 5)
   - Use `SqlCipherPool` from `pulsearc_common::storage::sqlcipher` (pooled connections)
   - Use `pool.get_sqlcipher_connection()` (**synchronous**, no `.await`)
   - **DO NOT** use `LocalDatabase` (being deprecated)

### Total Timeline

**Best Case**: 5 business days (with parallel execution)
**Realistic**: 6-7 business days (accounting for reviews/testing)
**Worst Case**: 8 business days (if blockers are discovered)

---

## Success Criteria

**Phase 0 Complete When**:
- [ ] All 6 blocked modules resolved (reclassified or refactored)
- [ ] Feature flags aligned (docs ↔ Cargo.toml)
- [ ] `cargo check --package pulsearc-domain` passes with **zero** infra deps
- [ ] `cargo deny check licenses` passes
- [ ] All existing tests pass
- [ ] Migration inventory updated with final classifications

**Validation Commands**:
```bash
# Verify domain crate purity (after creation)
cargo tree -p pulsearc-domain --edges normal | grep -E "(rusqlite|reqwest|keyring|tauri)"
# Should return nothing

# Verify no LocalDatabase references (being deprecated)
grep -r "LocalDatabase" legacy/api/src/preprocess/ legacy/api/src/core/
# Should return nothing

# Verify SqlCipherPool usage in repository implementations
grep -r "SqlCipherPool" legacy/api/src/infra/repositories/
# Should find repository implementations using SqlCipherPool

# Verify no async in repository implementations (API is synchronous)
grep -r "async fn\|\.await" legacy/api/src/infra/repositories/ | grep -v test
# Should return nothing (synchronous API)

# Verify feature flags
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "pulsearc-legacy-api") | .features'

# Run full CI
cargo ci
```

---

## Risk Assessment

### High-Risk Areas
1. **Segmenter refactor** (largest change, touches hot path)
   - **Mitigation**: Create repository port first, add integration tests before refactoring
2. **Error type split** (touches many call-sites)
   - **Mitigation**: Use IDE refactoring tools, run full test suite frequently
3. **Config split** (used at app startup)
   - **Mitigation**: Maintain backward compatibility via wrapper, validate with smoke tests

### Low-Risk Areas
1. **Reclassifications** (documentation updates, no code changes yet)
2. **Feature flag additions** (additive change, backward compatible)

---

## Definition of Done

- [ ] All tasks in "Recommended Actions" table completed
- [ ] Pull requests merged for splits/refactors
- [ ] Migration inventory document updated
- [ ] Blockers removed from [LEGACY_MIGRATION_INVENTORY.md](../../docs/LEGACY_MIGRATION_INVENTORY.md)
- [ ] Phase 1 unblocked (domain crate can be created without infra deps)

---

## References

- [LEGACY_MIGRATION_INVENTORY.md](../../docs/LEGACY_MIGRATION_INVENTORY.md) (lines 10-44, 227-325)
- [ADR-003: Layered Architecture](../../docs/adr/ADR-003-layered-architecture.md)
- [SHARED_TYPES_ANALYSIS.md](../../docs/SHARED_TYPES_ANALYSIS.md)
- [LEGACY_STRUCT_MAPPING.md](../../docs/LEGACY_STRUCT_MAPPING.md)

---

**Next Steps**: Assign owners, create child issues for each task, begin Track 1 (quick wins) immediately.
