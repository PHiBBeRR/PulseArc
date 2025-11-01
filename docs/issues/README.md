# GitHub Issue: Phase 0 Migration Blockers

**Copy this content to create a new GitHub issue**

---

## Title
🚨 Phase 0: Critical Refactoring Required Before ADR-003 Migration

## Labels
`priority: P0`, `type: refactoring`, `epic`, `blocking`

## Milestone
Phase 0 - Pre-Migration Refactoring

## Body

### 📋 Summary

Six modules currently violate layered architecture rules and **block Phase 1** of the ADR-003 migration. These modules contain side effects (I/O, database access) but are classified as `domain` or `core` in our migration inventory.

**Estimated Effort**: 5-8 business days
**Status**: 🔴 Blocking Phase 1

---

### 🔴 Critical Blockers

1. **`shared/config.rs`** — Reads env vars + filesystem ➜ **MUST SPLIT** (DTOs → domain, loader → infra)
2. **`observability/errors/app.rs`** — Implements `From<rusqlite::Error>` ➜ **MUST SPLIT** (types → domain, conversions → infra)
3. **`preprocess/segmenter.rs`** — Raw DB queries ➜ **MUST REFACTOR** (add repository ports)
4. **`inference/batch_classifier.rs`** — Uses `DbManager` + `tauri::Emitter` ➜ **RECLASSIFY** to infra
5. **`integrations/sap/errors.rs`** — Wraps `reqwest::Error` ➜ **RECLASSIFY** to infra
6. **`integrations/sap/validation.rs`** — Uses `DbManager` directly ➜ **RECLASSIFY** to infra

### 🏴 Feature Flag Mismatch

Documentation references `calendar`, `sap`, `ml` features but `legacy/api/Cargo.toml` only defines `tree-classifier` and `graphql`.

**Action**: Add missing features or update docs.

---

### 📦 Task Breakdown

**Track 1: Quick Wins** (1-2 days)
- [ ] #️⃣ Reclassify `batch_classifier` → infra
- [ ] #️⃣ Reclassify `sap/errors.rs` → infra
- [ ] #️⃣ Reclassify `sap/validation.rs` → infra
- [ ] #️⃣ Add feature flags to Cargo.toml

**Track 2: Splits** (4 days)
- [ ] #️⃣ Split `shared/config.rs` (2 days)
- [ ] #️⃣ Split `observability/errors/app.rs` (2 days)

**Track 3: Segmenter Refactor** (4-5 days)
- [ ] #️⃣ Define `SegmentRepository` + `SnapshotRepository` ports (1 day)
- [ ] #️⃣ Refactor `preprocess/segmenter.rs` to use ports (2-3 days)
- [ ] #️⃣ Implement SqlCipher-based repository adapters (1 day)
  - ⚠️ **Use `SqlCipherConnection` from `storage/sqlcipher`, NOT `LocalDatabase`**
  - ⚠️ **`query_map` returns `Vec<T>`, not an iterator - do NOT call `.collect()`**

---

### ✅ Success Criteria

Phase 0 complete when:
- [ ] All 6 modules resolved (reclassified or refactored)
- [ ] Feature flags aligned (docs ↔ Cargo.toml)
- [ ] `cargo check --package pulsearc-domain` passes with **zero** infra deps (after crate creation)
- [ ] All existing tests pass
- [ ] Migration inventory updated

**Validation**:
```bash
# Verify no infra deps in pure types
grep -r "rusqlite\|reqwest\|keyring" legacy/api/src/shared/config_types.rs
grep -r "rusqlite\|reqwest\|keyring" legacy/api/src/observability/errors/app.rs
grep -r "crate::db::\|LocalDatabase" legacy/api/src/preprocess/segmenter.rs

# Verify SqlCipherConnection usage (not LocalDatabase)
grep -r "LocalDatabase" legacy/api/src/infra/repositories/ && echo "❌ FAIL: LocalDatabase found" || echo "✅ PASS"
grep -r "SqlCipherConnection" legacy/api/src/infra/repositories/ && echo "✅ PASS" || echo "❌ FAIL: SqlCipherConnection not found"

# Verify feature flags
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "pulsearc-legacy-api") | .features'

# Run CI
cargo ci
```

---

### 📚 Documentation

- **Detailed Ticket**: [phase-0-migration-blockers.md](.github/ISSUE_TEMPLATE/phase-0-migration-blockers.md)
- **Task Tracking**: [docs/issues/PHASE-0-BLOCKERS-TRACKING.md](docs/issues/PHASE-0-BLOCKERS-TRACKING.md)
- **Migration Inventory**: [docs/LEGACY_MIGRATION_INVENTORY.md](docs/LEGACY_MIGRATION_INVENTORY.md)
- **ADR-003**: [docs/adr/ADR-003-layered-architecture.md](docs/adr/ADR-003-layered-architecture.md)

---

### 🎯 Next Steps

1. **Assign owners** for each track
2. **Create child issues** for individual tasks (use tracking doc)
3. **Begin Track 1** (quick wins) immediately
4. **Schedule PRs** for splits and segmenter refactor

---

### 💬 Discussion

Please comment below to:
- Volunteer to own a track
- Raise concerns about approach
- Suggest alternative solutions
- Report blockers

---

**Related**:
- ADR-003 Phase 1 (blocked by this issue)
- Migration inventory audit (completed)

**Assignees**: _To be assigned_
**Timeline**: Target completion by [DATE + 1 week]
