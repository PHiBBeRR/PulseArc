# Phase 3 Pre-Migration Bug Fixes & Anti-Patterns

**Created**: November 1, 2025
**Status**: ✅ All critical legacy bugs identified and documented
**Purpose**: Ensure legacy bugs are NOT ported to new `crates/infra` during Phase 3 migration

---

## 🚨 Critical Bugs Found in Legacy Code

### Summary

During Phase 3 readiness review, 4 critical bugs were identified in the gitignored `legacy/api/src/` codebase. These bugs would cause **data corruption, performance degradation, and system failures** if accidentally ported to the new infrastructure.

**Good News**: None of these bugs exist in the tracked `crates/` code yet.
**Action Required**: Use this document as a checklist during Phase 3 migration to avoid re-introducing these bugs.

---

## 🔴 Issue 1: Outbox Retry Filter Bug (CRITICAL - Data Loss)

### Location (Legacy)
`legacy/api/src/db/outbox/outbox.rs:560` (GITIGNORED)

### Bug Description
```rust
// ❌ WRONG - Legacy bug
"SELECT {} FROM time_entry_outbox WHERE status = 'sent' AND (retry_after IS NULL OR retry_after <= ?1)"
```

**Impact**:
- OutboxWorker never processes `pending` entries (they stay stuck forever)
- Only entries marked `sent` get retried (semantic error)
- **Result**: New time entries never sync to backend → DATA LOSS

### Correct Implementation
```rust
// ✅ CORRECT - Use this in Phase 3
"SELECT {} FROM time_entry_outbox WHERE status = 'pending' AND (retry_after IS NULL OR retry_after <= ?1)"
```

### Phase 3 Tasks Affected
- **Task 3D.3**: OutboxWorker implementation
- **Task 3A.1**: Outbox repository implementation

### Test Requirement
```rust
#[tokio::test]
async fn test_outbox_retry_filter_uses_pending_status() {
    // Create entries with status = 'pending' and 'sent'
    // Verify that get_pending_entries_ready_for_retry returns ONLY pending
    // Verify that sent entries are NOT returned
}
```

**Regression status:** Implemented in `crates/infra/tests/outbox_retry_regression.rs` (current `[ignore]` until Phase 3A.1 wiring). The helper harness seeds real rows against the new `SqliteOutboxRepository`.

---

## 🔴 Issue 2: SAP Forwarder Hard-Coded Date (CRITICAL - Data Corruption)

### Location (Legacy)
`legacy/api/src/integrations/sap/forwarder.rs:146` (GITIGNORED)

### Bug Description
```rust
// ❌ WRONG - Legacy bug
date: payload
    .get("date")
    .and_then(|v| v.as_str())
    .unwrap_or("2025-10-22")  // 🚨 Hard-coded fallback!
    .to_string(),
```

**Impact**:
- All entries missing `date` field get stamped with "2025-10-22"
- **Data corruption**: Wrong date in SAP time tracking system
- **Idempotency collisions**: Multiple entries with same date conflict
- **Audit trail broken**: Time entries appear on wrong dates

### Correct Implementation
```rust
// ✅ CORRECT - Derive from entry.created_at if missing
date: payload
    .get("date")
    .and_then(|v| v.as_str())
    .map(String::from)
    .unwrap_or_else(|| {
        // Derive from entry.created_at if missing
        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(entry.created_at, 0)
            .unwrap_or_else(|| chrono::Utc::now())
            .format("%Y-%m-%d")
            .to_string();
        log::warn!(
            "Missing date field for entry {}, deriving from created_at: {}",
            entry.id,
            date
        );
        date
    }),
```

### Phase 3 Tasks Affected
- **Task 3C.2**: SAP forwarder adapter (feature: `sap`)
- **Task 3C.3**: Time entry formatting logic

### Test Requirement
```rust
#[tokio::test]
async fn test_sap_forwarder_derives_date_from_created_at() {
    // Create outbox entry with missing date field in payload_json
    // Verify that forwarder uses entry.created_at instead of hard-coded fallback
    // Verify warning is logged
}
```

**Regression status:** Implemented in `crates/infra/tests/sap_forwarder_date_regression.rs` behind `#[cfg(feature = "sap")]` and `#[ignore]` until Phase 3C.2 hooks the adapter into the pipeline.

---

## 🟡 Issue 3: Outbox Status Parsing Panic (HIGH - System Crash)

### Location (Legacy)
`legacy/api/src/db/outbox/outbox.rs:70` (GITIGNORED)

### Bug Description
```rust
// ❌ WRONG - Legacy bug
status: row.get::<_, String>(5)?.parse().unwrap(),  // 🚨 Panics on bad data
```

**Impact**:
- Any unexpected status value (legacy migrations, manual DB edits, bugs) panics entire pipeline
- **Resilience issue**: One bad row breaks all outbox processing
- **Phase 3 migration risk**: SqlCipher migration may expose new status values

### Correct Implementation
```rust
// ✅ CORRECT - Default to Pending with warning
let status_str = row.get::<_, String>(5)?;
let status = status_str.parse().unwrap_or_else(|e| {
    log::warn!(
        "Invalid outbox status '{}', defaulting to Pending: {}",
        status_str,
        e
    );
    OutboxStatus::Pending
});
```

### Phase 3 Tasks Affected
- **Task 3A.1**: Outbox repository implementation
- **Task 3D.3**: OutboxWorker error handling

### Test Requirement
```rust
#[tokio::test]
async fn test_outbox_status_parsing_handles_invalid_values() {
    // Insert entry with invalid status string (e.g., "unknown", "retrying")
    // Verify that parsing defaults to Pending instead of panicking
    // Verify warning is logged
}
```

**Regression status:** Implemented in `crates/infra/tests/outbox_status_parsing_regression.rs`; the integration cases stay `#[ignore]` pending repository adoption, while the FromStr unit test runs today.

---

## 🟡 Issue 4: Date Query Index Bypass (MEDIUM-HIGH - Performance)

### Locations (Legacy)
- `legacy/api/src/infra/repositories/segment_repository.rs:136` (GITIGNORED)
- `legacy/api/src/infra/repositories/snapshot_repository.rs:84` (GITIGNORED)

### Bug Description
```sql
-- ❌ WRONG - Legacy bug (can't use index on start_ts/timestamp)
WHERE date(start_ts, 'unixepoch') = ?1

-- Passed parameter: date_str = "2025-10-30" (string)
```

**Impact**:
- **Performance**: Full table scans instead of index seeks
- **Scalability**: Query time grows O(n) with table size instead of O(log n)
- **Incorrect semantics**: `date()` assumes UTC boundaries, but domain may expect local time
- **Phase 3A risk**: Performance baseline will show poor results

### Correct Implementation
```rust
// ✅ CORRECT - Use explicit range predicates to preserve index
let day_start = date
    .and_hms_opt(0, 0, 0)
    .ok_or_else(|| CommonError::storage("Invalid date for day start"))?
    .and_utc()
    .timestamp();
let day_end = date
    .succ_opt()
    .ok_or_else(|| CommonError::storage("Date overflow calculating next day"))?
    .and_hms_opt(0, 0, 0)
    .ok_or_else(|| CommonError::storage("Invalid date for day end"))?
    .and_utc()
    .timestamp();

let sql = "SELECT ... FROM activity_segments WHERE start_ts >= ?1 AND start_ts < ?2";
stmt.query_map(&[&day_start as &dyn ToSql, &day_end as &dyn ToSql], ...)
```

### Phase 3 Tasks Affected
- **Task 3A.1**: SegmentRepository implementation
- **Task 3A.1**: SnapshotRepository implementation
- **Task 3A.0**: Performance baseline (will detect this if present)

### Test Requirement
```rust
#[test]
fn test_segment_repository_uses_index_friendly_date_queries() {
    // Query segments by date
    // Verify SQL uses range predicates (>= and <) instead of date() function
    // Verify query plan uses index on start_ts
}

#[test]
fn test_performance_date_queries_under_10ms() {
    // Create 10,000 segments across 30 days
    // Query single day
    // Verify query completes in <10ms (index seek, not full scan)
}
```

**Regression status:** Implemented in `crates/infra/tests/date_query_performance_regression.rs` with a shared harness that provisions indexed tables. Four heavy tests remain `#[ignore]` until Phase 3A.1 enables real repository usage; the range conversion unit test already passes.

---

## ✅ Verification Checklist for Phase 3

### Before Starting Phase 3A (Core Infrastructure)

- [ ] Review this document with all Phase 3 contributors
- ✅ Added all 4 regression suites to `crates/infra/tests/` (currently `#[ignore]` guarded where adapters are not yet wired)
- [ ] Verify `crates/infra` does NOT contain any of these anti-patterns
- [ ] Run `cargo clippy` and ensure no `unwrap()` in production parsing code

### During Phase 3A.1 (Database Repositories)

**OutboxRepository**:
- [ ] Verify retry filter uses `status = 'pending'` (not 'sent')
- [ ] Verify status parsing has error handling (no `unwrap()`)
- [ ] Add regression test: `test_outbox_retry_filter_uses_pending_status`
- [ ] Add regression test: `test_outbox_status_parsing_handles_invalid_values`

**SegmentRepository & SnapshotRepository**:
- [ ] Verify date queries use range predicates (`>= AND <`)
- [ ] Verify NO use of `date(column, 'unixepoch')` in WHERE clauses
- [ ] Add regression test: `test_segment_repository_uses_index_friendly_date_queries`
- [ ] Add performance test: `test_performance_date_queries_under_10ms`

### During Phase 3C.2 (SAP Integration)

**SAP Forwarder**:
- [ ] Verify date field derivation from `entry.created_at` if missing
- [ ] Verify NO hard-coded date fallbacks (e.g., "2025-10-22")
- [ ] Add regression test: `test_sap_forwarder_derives_date_from_created_at`
- [ ] Verify warning is logged when date is missing

### Phase 3 Code Review Checklist

Add these to ALL Phase 3 PR templates:

- [ ] No `unwrap()` or `expect()` in production parsing/database code
- [ ] No hard-coded date/time fallbacks
- [ ] All SQL date queries use range predicates (not `date()` function)
- [ ] All retry/queue logic uses correct status predicates
- [ ] All changes have corresponding regression tests

---

## 📊 Impact Assessment

### If These Bugs Are Ported to Phase 3

| Issue | Severity | MTTR | Data Loss Risk | Performance Impact |
|-------|----------|------|----------------|-------------------|
| #1: Outbox retry filter | 🔴 CRITICAL | Hours | HIGH (entries stuck forever) | N/A |
| #2: SAP hard-coded date | 🔴 CRITICAL | Days | HIGH (corrupt SAP data) | N/A |
| #3: Status parsing panic | 🟡 HIGH | Minutes | MEDIUM (pipeline crash) | N/A |
| #4: Date query index bypass | 🟡 MEDIUM | N/A | NONE | HIGH (full table scans) |

**Total Risk**: CRITICAL (2 data corruption bugs + 1 crash bug + 1 performance bug)

---

## 🎯 Success Criteria

Phase 3 is successful ONLY if:

1. ✅ All 4 regression tests pass in `crates/infra/tests/`
2. ✅ Zero occurrences of anti-patterns in `git grep` across `crates/infra/`
3. ✅ Performance baseline shows date queries <10ms (Task 3A.0)
4. ✅ Code review checklist completed for ALL Phase 3 PRs

---

## 📝 Lessons Learned

### Why These Bugs Exist in Legacy Code

1. **Lack of tests**: No regression tests for edge cases (missing data, invalid status)
2. **Copy-paste errors**: Wrong status predicate ('sent' vs 'pending')
3. **Over-reliance on unwrap()**: No graceful degradation for parsing errors
4. **SQL anti-patterns**: Using functions in WHERE clauses (bypasses indexes)

### How Phase 3 Avoids These

1. **Test-first approach**: Write regression tests BEFORE implementation
2. **Code review**: Mandatory checklist for ALL database/parsing code
3. **Clippy enforcement**: `-D warnings` catches `unwrap()` usage
4. **Performance baseline**: Task 3A.0 catches index bypass issues early

---

## 🔗 Related Documents

- [Phase 3 Infrastructure Tracking](PHASE-3-INFRA-TRACKING.md) - Main migration plan
- [SqlCipher API Reference](SQLCIPHER-API-REFERENCE.md) - Database API patterns
- [Legacy Migration Inventory](../LEGACY_MIGRATION_INVENTORY.md) - Full migration scope

---

**Document Status**: 🟢 READY FOR PHASE 3
**Last Updated**: November 1, 2025
**Next Review**: Before Phase 3A.1 starts
