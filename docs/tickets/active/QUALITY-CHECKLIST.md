# Quality Improvement Checklist

**Quick reference for daily progress tracking**
**Full details:** [CRATE-QUALITY-10-10.md](./CRATE-QUALITY-10-10.md)

---

## 🔴 Week 1: Critical Path (8.5 → 9.0)

### Day 1: Setup & Quick Wins
- [ ] Remove `crates/api/Cargo.lock`
- [ ] Add `crates/*/Cargo.lock` to `.gitignore`
- [ ] Create `crates/domain/tests/` directory
- [ ] Create `crates/core/tests/services/` directory
- [ ] Create `crates/api/tests/commands/` directory

### Day 2-3: Domain Tests
- [ ] `tests/classification_types.rs` (50+ tests)
- [ ] `tests/database_types.rs` (40+ tests)
- [ ] `tests/stats_types.rs` (30+ tests)
- [ ] Add inline `#[cfg(test)]` modules
- [ ] Run `cargo tarpaulin -p pulsearc-domain` → 80%+

### Day 4-5: Core Tests
- [ ] `tests/services/classification_service_test.rs` (25+ tests)
- [ ] `tests/services/tracking_service_test.rs` (25+ tests)
- [ ] `tests/services/batch_operations_test.rs` (15+ tests)
- [ ] `tests/services/sync_operations_test.rs` (20+ tests)
- [ ] Add mock infrastructure with `mockall`
- [ ] Run `cargo tarpaulin -p pulsearc-core` → 70%+

**Week 1 Goal:** ✅ 9.0/10 rating

---

## 🟡 Week 2: High Impact (9.0 → 9.5)

### Day 1-2: API Tests
- [ ] `tests/commands/tracking_commands_test.rs` (15+ tests)
- [ ] `tests/commands/classification_commands_test.rs` (10+ tests)
- [ ] `tests/commands/sync_commands_test.rs` (10+ tests)
- [ ] `tests/commands/feature_flags_test.rs` (8+ tests)
- [ ] `tests/integration/end_to_end_test.rs` (5+ tests)
- [ ] Run `cargo test -p pulsearc-app`

### Day 3: Property Testing
**📋 See:** [PROPERTY-MUTATION-TESTING.md](./PROPERTY-MUTATION-TESTING.md) for comprehensive plan
- [ ] Add `proptest` to workspace dependencies
- [ ] `crates/domain/tests/property_tests.rs` (20+ tests)
- [ ] `crates/core/tests/property_tests.rs` (15+ tests)
- [ ] Add snapshot tests with `insta`

### Day 4: Mutation Testing
**📋 See:** [PROPERTY-MUTATION-TESTING.md](./PROPERTY-MUTATION-TESTING.md) for comprehensive plan
- [ ] Install `cargo-mutants`
- [ ] Create `.cargo/mutants.toml`
- [ ] Run `cargo mutants` on domain/core
- [ ] Fix tests until 80%+ mutation score

### Day 5: Infra Tests
- [ ] Review infra test coverage
- [ ] Add repository tests
- [ ] Add API integration tests with `wiremock`
- [ ] Run `cargo tarpaulin -p pulsearc-infra` → 50%+

**Week 2 Goal:** ✅ 9.5/10 rating

---

## 🟢 Week 3: Polish (9.5 → 10.0)

### Day 1: Examples
- [ ] `crates/common/examples/basic_resilience.rs`
- [ ] `crates/common/examples/oauth_flow.rs`
- [ ] `crates/common/examples/encrypted_storage.rs`
- [ ] `crates/common/examples/validation_framework.rs`
- [ ] `crates/core/examples/classification_service.rs`
- [ ] `crates/core/examples/tracking_service.rs`
- [ ] Test all examples: `cargo run --example <name>`

### Day 2: Benchmarks
- [ ] Create `crates/domain/benches/`
- [ ] Create `crates/core/benches/`
- [ ] `benches/classification_bench.rs`
- [ ] `benches/tracking_bench.rs`
- [ ] Run `cargo bench --workspace`

### Day 3: ADRs
- [ ] `0001-layered-architecture.md`
- [ ] `0002-domain-driven-design.md`
- [ ] `0003-ports-and-adapters.md`
- [ ] `0004-feature-flag-tiers.md`
- [ ] `0005-sqlcipher-migration.md`
- [ ] `0006-workspace-organization.md`
- [ ] `0007-testing-strategy.md`

### Day 4: Documentation
- [ ] Update `crates/domain/README.md`
- [ ] Update `crates/core/README.md`
- [ ] Update `crates/infra/README.md`
- [ ] Create `crates/api/README.md`
- [ ] Fix `cargo doc` warnings
- [ ] Add missing doc comments

### Day 5: CI & Metrics
- [ ] Install `cargo-tarpaulin`
- [ ] Create `scripts/coverage.sh`
- [ ] Create `scripts/quality-metrics.sh`
- [ ] Update `.github/workflows/ci.yml`
- [ ] Add coverage reporting
- [ ] Add mutation testing to CI

**Week 3 Goal:** ✅ 10/10 rating

---

## Quick Commands

### Run All Tests
```bash
cargo test --workspace
```

### Check Coverage
```bash
cargo tarpaulin --workspace --out Html
open tarpaulin-report.html
```

### Run Mutation Tests
```bash
cargo mutants -p pulsearc-domain -p pulsearc-core
```

### Run Benchmarks
```bash
cargo bench --workspace
```

### Check Documentation
```bash
cargo doc --workspace --no-deps --open
```

### Quality Dashboard
```bash
./scripts/quality-metrics.sh
```

---

## Success Criteria Summary

| Metric | Current | Week 1 | Week 2 | Week 3 |
|--------|---------|--------|--------|--------|
| **Rating** | 8.5/10 | 9.0/10 | 9.5/10 | 10/10 |
| **Domain Tests** | 0 | 150+ | 170+ | 170+ |
| **Core Tests** | ~20 | 120+ | 135+ | 135+ |
| **API Tests** | 0 | 0 | 50+ | 50+ |
| **Coverage** | ~15% | 60% | 70% | 70%+ |
| **Mutation Score** | - | - | 80%+ | 80%+ |
| **Examples** | 0 | 0 | 0 | 10+ |
| **ADRs** | 0 | 0 | 0 | 7+ |

---

## Daily Progress Template

Copy and update daily:

```markdown
### YYYY-MM-DD Progress

**Completed:**
- [ ] Task 1
- [ ] Task 2

**In Progress:**
- [ ] Task 3

**Blockers:**
- None

**Notes:**
- Observation 1
- Observation 2

**Tomorrow:**
- [ ] Task 4
- [ ] Task 5
```

---

**Created:** 2025-10-31
**Status:** Ready to start
**Next Action:** Remove `crates/api/Cargo.lock` ⚡
