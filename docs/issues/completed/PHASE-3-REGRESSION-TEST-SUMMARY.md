# Phase 3 Regression Test Summary for Infra Squad

**Created:** November 1, 2025
**Status:** ✅ Regression suites implemented (guarded with `#[ignore]` until Phase 3 wiring)
**Location:** `crates/infra/tests/`
**Reference:** [PHASE-3-PRE-MIGRATION-FIXES.md](PHASE-3-PRE-MIGRATION-FIXES.md)

---

## Overview

During Phase 3 readiness review, we identified **4 critical bugs** in the gitignored legacy code that would cause data corruption, system crashes, and performance issues if ported to `crates/infra`.

**Good news:** None of these bugs exist in tracked code yet.
**Action required:** Implement regression tests BEFORE porting adapters to prevent re-introducing these bugs.

---

## Test Files Created (∼520 lines, 13 tests)

> New helper: `crates/infra/tests/support.rs` provisions temp SQLCipher-ish schemas and captures `log::warn!` output so these suites can exercise the real repositories without additional scaffolding.

### 1. **outbox_retry_regression.rs** (109 lines, 2 tests)
**Issue:** Outbox retry filter uses wrong status predicate
**Legacy bug:** `WHERE status = 'sent'` instead of `WHERE status = 'pending'`
**Impact:** 🔴 CRITICAL - Data loss (pending entries never sync)

**Tests:**
- `test_outbox_retry_filter_uses_pending_status()` - Verifies correct predicate
- `test_outbox_retry_respects_retry_after_timestamp()` - Verifies retry timing

**Status:** Implemented, currently `#[ignore]` pending Phase 3A.1 repository wiring

---

### 2. **sap_forwarder_date_regression.rs** (118 lines, 3 tests)
**Issue:** SAP forwarder uses hard-coded date fallback
**Legacy bug:** `.unwrap_or("2025-10-22")` when date field missing
**Impact:** 🔴 CRITICAL - Data corruption in SAP system, idempotency collisions

**Tests:**
- `test_sap_forwarder_derives_date_from_created_at()` - Verifies date derivation
- `test_sap_forwarder_uses_payload_date_when_present()` - Verifies preference
- `test_sap_forwarder_handles_invalid_created_at()` - Verifies graceful degradation

**Status:** Implemented behind `#[ignore]` (Phase 3C.2 will unskip once the SAP feature flag wiring is complete)
**Feature flag:** `sap`

---

### 3. **outbox_status_parsing_regression.rs** (150 lines, 3 tests)
**Issue:** Outbox status parsing panics on invalid data
**Legacy bug:** `parse().unwrap()` crashes on unexpected status values
**Impact:** 🟡 HIGH - System crash, one bad row breaks entire pipeline

**Tests:**
- `test_outbox_status_parsing_handles_invalid_values()` - Verifies no panic
- `test_outbox_status_parsing_handles_valid_values()` - Verifies correct parsing
- `test_outbox_status_string_parse_error_handling()` - ✅ PASSING (unit test)

**Status:** Implemented. Two integration-style cases remain `#[ignore]` until the port is active; the string parsing unit test runs today.

---

### 4. **date_query_performance_regression.rs** (180 lines, 5 tests)
**Issue:** Date queries bypass database indexes
**Legacy bug:** `WHERE date(timestamp, 'unixepoch') = ?` forces full table scans
**Impact:** 🟡 MEDIUM-HIGH - O(n) performance, slow with scale

**Tests:**
- `test_segment_repository_uses_index_friendly_date_queries()` - Verifies range predicates
- `test_snapshot_repository_uses_index_friendly_date_queries()` - Verifies range predicates
- `test_date_query_performance_under_10ms()` - Performance benchmark (p99 < 10ms)
- `test_date_query_correctness_at_day_boundaries()` - Boundary condition test
- `test_date_to_timestamp_range_conversion()` - ✅ PASSING (unit test)

**Status:** Implemented. Four database-heavy checks stay `#[ignore]` until the segment/snapshot repositories ship; the range conversion unit test runs now.

---

## Test Status Summary

| Test File | Total Tests | Passing | Ignored | Phase |
|-----------|-------------|---------|---------|-------|
| outbox_retry_regression.rs | 2 | 0 | 2 | 3A.1 |
| sap_forwarder_date_regression.rs | 3 | 0 | 3 | 3C.2 |
| outbox_status_parsing_regression.rs | 3 | 1 ✅ | 2 | 3A.1 |
| date_query_performance_regression.rs | 5 | 1 ✅ | 4 | 3A.1 |
| **Total** | **13** | **2** | **11** | - |

**Current status:**
- ✅ All suites compile against real infra implementations (outbox/segment/snapshot repositories plus SAP forwarder)
- ✅ 2 unit tests run in the default test pass
- ⏸️ 11 integration/perf tests marked `#[ignore]` — remove the attribute once the corresponding adapter is wired and data fixtures are stable

---

## Implementation Timeline

### Phase 3A.1: Database Repositories (Week 1-2)
**Next steps:**
1. Remove `#[ignore]` on the four outbox tests once `SqliteOutboxRepository` is plugged into the core adapters (Phase 3A.1).
2. Unskip the segment/snapshot suites when the repositories back the tracking pipeline and realistic data fixtures exist (Phase 3A.1).
3. Keep the performance benchmark optional locally; wire it into CI once it consistently meets the <10 ms SLA.

### Phase 3C.2: SAP Integration (Week 4-5)
**Next steps:**
1. Remove `#[ignore]` after the SAP forwarder is exercised through the integration port.
2. Ensure the SAP feature matrix job runs these via `cargo test --features sap -- --include-ignored` or equivalent once the adapter is hooked up.

---

## Anti-Patterns to Avoid

### ❌ NEVER Do This

```rust
// 1. Wrong status predicate
WHERE status = 'sent' AND retry_after <= ?1  // ❌ WRONG

// 2. Hard-coded date fallback
.unwrap_or("2025-10-22")  // ❌ WRONG

// 3. Panic on parse error
status: row.get::<_, String>(5)?.parse().unwrap()  // ❌ WRONG

// 4. Date function in WHERE clause
WHERE date(timestamp, 'unixepoch') = ?1  // ❌ WRONG (bypasses index)
```

### ✅ Always Do This

```rust
// 1. Correct status predicate
WHERE status = 'pending' AND (retry_after IS NULL OR retry_after <= ?1)  // ✅ CORRECT

// 2. Derive date from created_at
.unwrap_or_else(|| {
    let date = DateTime::from_timestamp(entry.created_at, 0)
        .unwrap_or_else(|| Utc::now())
        .format("%Y-%m-%d")
        .to_string();
    log::warn!("Missing date for entry {}, deriving from created_at: {}", entry.id, date);
    date
})  // ✅ CORRECT

// 3. Graceful degradation on parse error
let status = status_str.parse().unwrap_or_else(|e| {
    log::warn!("Invalid status '{}', defaulting to Pending: {}", status_str, e);
    OutboxStatus::Pending
});  // ✅ CORRECT

// 4. Range predicates for dates
WHERE timestamp >= ?1 AND timestamp < ?2  // ✅ CORRECT (uses index)
```

---

## Code Review Checklist

Before approving ANY Phase 3 PR:

- [ ] No hard-coded date/time fallbacks (check for `.unwrap_or("YYYY-MM-DD")`)
- [ ] No `unwrap()` or `expect()` in production parsing/database code
- [ ] All SQL date queries use range predicates (`>= AND <`), NOT `date()` function
- [ ] All retry/queue logic uses correct status predicates (`status = 'pending'`)
- [ ] Regression test implemented and passing (remove `#[ignore]` attribute)
- [ ] Warning logs added when deriving missing data or handling errors

---

## Running the Tests

- Quick compile matrix: `./scripts/test-features.sh`
- Full compile + test matrix (matches CI): `cargo xtask test-features`

### Run all regression tests
```bash
cargo test -p pulsearc-infra --tests
```

### Run specific test file
```bash
cargo test -p pulsearc-infra --test outbox_retry_regression
```

### Run specific test (including ignored)
```bash
cargo test -p pulsearc-infra test_outbox_retry_filter_uses_pending_status -- --ignored
```

### Run all tests including ignored
```bash
cargo test -p pulsearc-infra --tests -- --include-ignored
```

**CI coverage:** Once the adapters land, extend `infra-feature-matrix` to run ignored suites with `--include-ignored` for the relevant feature sets so the regressions stay active.

---

## Success Criteria

Phase 3 is NOT complete until:

1. ✅ All 13 regression tests pass (no `#[ignore]` attributes)
2. ✅ All 4 anti-patterns eliminated (verified via `git grep`)
3. ✅ Code review checklist completed for ALL Phase 3 PRs
4. ✅ Performance tests meet targets (date queries <10ms p99)

---

## Questions for Infra Squad

1. **Test ownership:** Who owns implementing each test during Phase 3A.1 and 3C.2?
2. **CI integration:** Should these tests run in a separate CI job or part of main test suite?
3. **Performance baselines:** Do we need to establish performance baselines before Phase 3A.1?
4. **Feature flag testing:** See [PHASE-3-FEATURE-FLAG-MATRIX.md](PHASE-3-FEATURE-FLAG-MATRIX.md) for complete matrix

---

## Related Documents

- [PHASE-3-PRE-MIGRATION-FIXES.md](PHASE-3-PRE-MIGRATION-FIXES.md) - Detailed bug documentation
- [PHASE-3-INFRA-TRACKING.md](PHASE-3-INFRA-TRACKING.md) - Main migration plan
- [SQLCIPHER-API-REFERENCE.md](SQLCIPHER-API-REFERENCE.md) - Database API patterns

---

**Document Status:** 🟢 READY FOR REVIEW
**Next Review:** Before Phase 3A.1 starts
**Contact:** @infra-squad for questions
