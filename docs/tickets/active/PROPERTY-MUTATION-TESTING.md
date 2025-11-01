# Property-Based & Mutation Testing Implementation

**Status:** Planning
**Priority:** High
**Type:** Testing Infrastructure
**Estimated Effort:** 3-4 weeks (phased approach)

## Overview

Implement property-based tests (proptest) and mutation testing (cargo-mutants) for critical paths in PulseArc to:
- Improve test quality beyond code coverage metrics
- Find edge cases and boundary conditions
- Validate that business invariants always hold
- Ensure tests actually catch regressions

## Background

### What are Property-Based Tests?

Property tests verify **invariants** hold true for a wide range of randomly generated inputs, rather than testing specific examples. They excel at finding edge cases developers wouldn't think to test manually.

**Example:** Instead of testing `add(2, 3) == 5`, test that `add(a, b) == add(b, a)` for ANY integers.

### What is Mutation Testing?

Mutation testing **modifies your code** (e.g., changes `>` to `>=`, `+` to `-`) and checks if tests catch the changes. It measures test quality, not just coverage.

**Example:** If changing `retry_count > 3` to `retry_count > 4` doesn't fail any tests, you have a testing gap.

## Critical Paths for Testing

### Phase 1: Core Business Logic (Highest Risk)

#### 1.1 Classification - Block Builder (`core/classification/block_builder.rs`)

**Why Critical:** Core algorithm that aggregates activities into billable time blocks. Errors here affect billing accuracy.

**Properties to Test:**
```rust
proptest! {
    // Invariant: All input activities must be in output blocks
    #[test]
    fn preserves_all_activities(activities in vec_of_activities(1..100)) {
        let builder = BlockBuilder::new();
        let blocks = builder.build_blocks(&activities)?;

        let total: usize = blocks.iter().map(|b| b.activities.len()).sum();
        prop_assert_eq!(total, activities.len());
    }

    // Invariant: Blocks should be time-ordered and non-overlapping
    #[test]
    fn blocks_are_non_overlapping(activities in vec_of_activities(1..100)) {
        let blocks = BlockBuilder::new().build_blocks(&activities)?;

        for window in blocks.windows(2) {
            prop_assert!(window[0].end_time <= window[1].start_time);
        }
    }

    // Invariant: Block duration = sum of activity durations
    #[test]
    fn block_duration_matches_activities(activities in vec_of_activities(1..50)) {
        let blocks = BlockBuilder::new().build_blocks(&activities)?;

        for block in blocks {
            let expected = block.end_time - block.start_time;
            let actual: i64 = block.activities.iter()
                .map(|a| /* calculate duration */)
                .sum();
            prop_assert_eq!(expected.num_seconds(), actual);
        }
    }
}
```

**Mutation Test Focus:**
- Boundary conditions in time gap detection
- Threshold values for merging blocks
- Signal similarity comparisons

---

#### 1.2 Classification - Signal Extractor (`core/classification/signal_extractor.rs`)

**Why Critical:** Extracts classification signals from URLs, titles, paths. Errors cause misclassification.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Should never panic on any string input
    #[test]
    fn never_panics_on_malformed_input(url in ".*", title in ".*", path in ".*") {
        let extractor = SignalExtractor::new();
        let _ = extractor.extract_signals(url, title, path);  // Should not panic
    }

    // Invariant: URL domain extraction should be idempotent
    #[test]
    fn domain_extraction_is_idempotent(url in valid_url_strategy()) {
        let extractor = SignalExtractor::new();
        let domain1 = extractor.extract_domain(&url);
        let domain2 = extractor.extract_domain(&url);
        prop_assert_eq!(domain1, domain2);
    }

    // Invariant: Should extract at least one signal if input is non-empty
    #[test]
    fn extracts_signals_from_valid_input(
        url in "https://[a-z]+\\.com/.*",
        title in "[A-Z]+-[0-9]+.*"
    ) {
        let extractor = SignalExtractor::new();
        let signals = extractor.extract_signals(url, title, None);
        prop_assert!(!signals.is_empty());
    }
}
```

**Mutation Test Focus:**
- Regex patterns for URL/path extraction
- String parsing logic
- Signal type classification

---

#### 1.3 Classification - Project Matcher (`core/classification/project_matcher.rs`)

**Why Critical:** Matches activities to project codes for billing. Errors cause incorrect billing.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Confidence scores should be between 0.0 and 1.0
    #[test]
    fn confidence_in_valid_range(signals in vec_of_signals(0..20)) {
        let matcher = ProjectMatcher::new(projects);
        let result = matcher.match_project(&signals);

        if let Some(match_result) = result {
            prop_assert!(match_result.confidence >= 0.0);
            prop_assert!(match_result.confidence <= 1.0);
        }
    }

    // Invariant: More signals should not decrease confidence
    #[test]
    fn more_signals_increase_confidence(
        base_signals in vec_of_signals(1..5),
        extra_signals in vec_of_signals(1..5)
    ) {
        let matcher = ProjectMatcher::new(projects);

        let conf1 = matcher.match_project(&base_signals).unwrap().confidence;

        let mut all_signals = base_signals.clone();
        all_signals.extend(extra_signals);
        let conf2 = matcher.match_project(&all_signals).unwrap().confidence;

        prop_assert!(conf2 >= conf1);
    }
}
```

**Mutation Test Focus:**
- Confidence threshold constants
- Signal weight calculations
- Project matching logic

---

#### 1.4 Tracking Service (`core/tracking/service.rs`)

**Why Critical:** Manages activity capture lifecycle. Errors cause data loss.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Start → Stop should always leave clean state
    #[test]
    fn start_stop_is_idempotent(iterations in 1..10) {
        let service = TrackingService::new(mock_deps());

        for _ in 0..iterations {
            service.start().await?;
            service.stop().await?;
        }

        // Should be in stopped state
        prop_assert!(!service.is_running());
    }

    // Invariant: Snapshots should be monotonically increasing in time
    #[test]
    fn snapshots_are_time_ordered(count in 1usize..100) {
        let service = TrackingService::new(mock_deps());
        service.start().await?;

        let snapshots = service.capture_snapshots(count).await?;

        for window in snapshots.windows(2) {
            prop_assert!(window[0].timestamp <= window[1].timestamp);
        }
    }
}
```

**Mutation Test Focus:**
- State transition logic (started/stopped)
- Interval calculations
- Error handling paths

---

### Phase 2: Domain Validation (Data Integrity)

#### 2.1 TimeEntry Validation (`domain/types/classification.rs`)

**Why Critical:** Ensures time entries meet business rules before billing submission.

**Properties to Test:**
```rust
proptest! {
    // Invariant: end_time must always be after start_time
    #[test]
    fn end_time_after_start_time(
        start in timestamp_strategy(),
        duration in 1u64..28800  // 1 sec to 8 hours
    ) {
        let end = start + Duration::seconds(duration as i64);
        let entry = TimeEntry {
            start_time: start,
            end_time: end,
            duration_hours: duration as f64 / 3600.0,
            // ...
        };

        prop_assert!(entry.validate().is_ok());
    }

    // Invariant: Invalid entries should always fail validation
    #[test]
    fn rejects_invalid_entries(
        start in timestamp_strategy(),
        negative_duration in -86400i64..-1
    ) {
        let end = start + Duration::seconds(negative_duration);
        let entry = TimeEntry {
            start_time: start,
            end_time: end,  // Before start!
            duration_hours: negative_duration as f64 / 3600.0,
            // ...
        };

        prop_assert!(entry.validate().is_err());
    }

    // Invariant: duration_hours should match time range
    #[test]
    fn duration_matches_range(entry in time_entry_strategy()) {
        let expected = (entry.end_time - entry.start_time).num_seconds() as f64 / 3600.0;
        let diff = (entry.duration_hours - expected).abs();
        prop_assert!(diff < 0.0001);  // Allow floating point error
    }
}
```

**Mutation Test Focus:**
- Validation boundary conditions (`>` vs `>=`)
- Constants (MIN_DURATION, MAX_DURATION)
- Required field checks

---

#### 2.2 Activity Validation (`domain/types/activity.rs`)

**Why Critical:** Activities are the foundation of all tracking data.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Serialization round-trip should preserve data
    #[test]
    fn serialization_roundtrip(activity in activity_strategy()) {
        let json = serde_json::to_string(&activity)?;
        let deserialized: Activity = serde_json::from_str(&json)?;
        prop_assert_eq!(activity, deserialized);
    }

    // Invariant: Activity timestamps should never be in future
    #[test]
    fn timestamp_not_in_future(activity in activity_strategy()) {
        prop_assert!(activity.timestamp <= Utc::now());
    }
}
```

**Mutation Test Focus:**
- Required field validation
- Timestamp validation logic
- Default value assignments

---

### Phase 3: Infrastructure Reliability (Data Persistence)

#### 3.1 Repository Implementations (`infra/database/*_repository.rs`)

**Why Critical:** Database operations must preserve data integrity.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Save → Load should return same data
    #[test]
    fn save_load_roundtrip(activity in activity_strategy()) {
        let repo = SqlActivityRepository::new(db_manager);

        repo.save(&activity).await?;
        let loaded = repo.find_by_id(&activity.id).await?.unwrap();

        prop_assert_eq!(activity, loaded);
    }

    // Invariant: Queries should be deterministic
    #[test]
    fn query_results_deterministic(
        activities in vec_of_activities(1..50),
        start in timestamp_strategy(),
        end in timestamp_strategy()
    ) {
        let repo = SqlActivityRepository::new(db_manager);
        for activity in &activities {
            repo.save(activity).await?;
        }

        let result1 = repo.find_by_time_range(start, end).await?;
        let result2 = repo.find_by_time_range(start, end).await?;

        prop_assert_eq!(result1, result2);
    }

    // Invariant: Delete should remove all traces
    #[test]
    fn delete_is_complete(activity in activity_strategy()) {
        let repo = SqlActivityRepository::new(db_manager);

        repo.save(&activity).await?;
        repo.delete(&activity.id).await?;

        let result = repo.find_by_id(&activity.id).await?;
        prop_assert!(result.is_none());
    }
}
```

**Mutation Test Focus:**
- SQL WHERE clauses
- ORDER BY and LIMIT logic
- Transaction boundaries

---

#### 3.2 Outbox Worker (`infra/sync/outbox_worker.rs`)

**Why Critical:** Ensures reliable data sync with remote systems.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Failed items should move to DLQ after max retries
    #[test]
    fn moves_to_dlq_after_retries(item in outbox_item_strategy()) {
        let worker = OutboxWorker::new(config_with_max_retries(3));

        // Simulate 3 failures
        for _ in 0..3 {
            worker.process_item(&item).await?;
        }

        let dlq_items = worker.get_dlq_items().await?;
        prop_assert!(dlq_items.contains(&item));
    }

    // Invariant: Successful processing should remove from outbox
    #[test]
    fn removes_on_success(item in outbox_item_strategy()) {
        let worker = OutboxWorker::new(config);

        worker.enqueue(&item).await?;
        worker.process_all().await?;  // Assume success

        let remaining = worker.get_pending_items().await?;
        prop_assert!(!remaining.contains(&item));
    }
}
```

**Mutation Test Focus:**
- Retry count logic
- Backoff calculations
- DLQ threshold values

---

### Phase 4: Security & Cryptography (Critical for Data Protection)

#### 4.1 Encryption (`common/crypto/encryption.rs`)

**Why Critical:** Protects sensitive user data. Errors expose data.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Encrypt → Decrypt should return original plaintext
    #[test]
    fn encrypt_decrypt_roundtrip(plaintext in ".*") {
        let encryptor = Encryptor::new(key);

        let ciphertext = encryptor.encrypt(&plaintext)?;
        let decrypted = encryptor.decrypt(&ciphertext)?;

        prop_assert_eq!(plaintext, decrypted);
    }

    // Invariant: Same plaintext with different nonces should produce different ciphertexts
    #[test]
    fn different_nonces_different_ciphertexts(plaintext in ".*") {
        let encryptor = Encryptor::new(key);

        let ct1 = encryptor.encrypt(&plaintext)?;
        let ct2 = encryptor.encrypt(&plaintext)?;

        prop_assert_ne!(ct1, ct2);
    }

    // Invariant: Corrupted ciphertext should fail to decrypt
    #[test]
    fn corrupted_ciphertext_fails(
        plaintext in ".*",
        corruption_index in 0usize..100
    ) {
        let encryptor = Encryptor::new(key);

        let mut ciphertext = encryptor.encrypt(&plaintext)?;
        if corruption_index < ciphertext.len() {
            ciphertext[corruption_index] ^= 0xFF;  // Flip bits
            prop_assert!(encryptor.decrypt(&ciphertext).is_err());
        }
    }
}
```

**Mutation Test Focus:**
- Key derivation logic
- Nonce generation
- Authentication tag verification

---

#### 4.2 Key Management (`common/security/encryption/keys.rs`)

**Why Critical:** Improper key handling exposes all encrypted data.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Key rotation should allow decrypting old data with new key
    #[test]
    fn rotation_preserves_access(plaintext in ".*") {
        let manager = KeyManager::new();

        let encrypted = manager.encrypt(&plaintext)?;
        manager.rotate_key().await?;
        let decrypted = manager.decrypt(&encrypted)?;

        prop_assert_eq!(plaintext, decrypted);
    }
}
```

---

### Phase 5: Error Handling & Resilience (System Reliability)

#### 5.1 Retry Logic (`common/resilience/retry.rs`)

**Why Critical:** Incorrect retry logic can cause infinite loops or missed retries.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Should respect max_retries limit
    #[test]
    fn respects_max_retries(max_retries in 0u32..10) {
        let policy = RetryPolicy::new(max_retries);
        let mut attempt_count = 0;

        let result = retry_with_policy(&policy, || {
            attempt_count += 1;
            Err::<(), _>("fail")
        }).await;

        prop_assert_eq!(attempt_count, max_retries + 1);  // Initial + retries
    }

    // Invariant: Backoff should increase exponentially
    #[test]
    fn exponential_backoff_increases(base_ms in 10u64..1000) {
        let backoff = ExponentialBackoff::new(Duration::from_millis(base_ms));

        let delay1 = backoff.delay(1);
        let delay2 = backoff.delay(2);
        let delay3 = backoff.delay(3);

        prop_assert!(delay2 > delay1);
        prop_assert!(delay3 > delay2);
    }
}
```

**Mutation Test Focus:**
- Retry count bounds
- Backoff multipliers
- Timeout values

---

#### 5.2 Circuit Breaker (`common/resilience/circuit_breaker.rs`)

**Why Critical:** Prevents cascading failures. Errors can cause system overload.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Should open after failure_threshold consecutive failures
    #[test]
    fn opens_after_threshold(threshold in 1u32..20) {
        let breaker = CircuitBreaker::new(threshold);

        for _ in 0..threshold {
            let _ = breaker.call(|| Err::<(), _>("fail")).await;
        }

        prop_assert!(breaker.is_open());
    }

    // Invariant: Should transition to half-open after timeout
    #[test]
    fn transitions_to_half_open(timeout_ms in 100u64..5000) {
        let breaker = CircuitBreaker::with_timeout(
            threshold: 3,
            Duration::from_millis(timeout_ms)
        );

        // Open the breaker
        for _ in 0..3 {
            let _ = breaker.call(|| Err::<(), _>("fail")).await;
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(timeout_ms + 100)).await;

        prop_assert!(breaker.is_half_open());
    }
}
```

---

### Phase 6: Validation Framework (`common/validation/`)

**Why Critical:** Used throughout app for input validation. Errors allow invalid data.

**Properties to Test:**
```rust
proptest! {
    // Invariant: Email validator should accept valid emails
    #[test]
    fn accepts_valid_emails(
        local in "[a-z0-9]+",
        domain in "[a-z]+\\.[a-z]{2,4}"
    ) {
        let email = format!("{}@{}", local, domain);
        let validator = EmailValidator::new();
        prop_assert!(validator.validate(&email).is_ok());
    }

    // Invariant: Range validator should reject out-of-range values
    #[test]
    fn rejects_out_of_range(
        min in 0i32..100,
        max in 101i32..200,
        value in -1000i32..-1
    ) {
        let validator = RangeValidator::new(min, max);
        prop_assert!(validator.validate(&value).is_err());
    }
}
```

---

## Implementation Plan

### Dependencies

```toml
[dev-dependencies]
proptest = "1.4"
```

Install mutation testing tool:
```bash
cargo install cargo-mutants
```

### Phased Rollout

#### Week 1-2: Phase 1 (Core Business Logic)
- [ ] Add proptest to `pulsearc-core`
- [ ] Implement property tests for BlockBuilder
- [ ] Implement property tests for SignalExtractor
- [ ] Implement property tests for ProjectMatcher
- [ ] Run initial mutation tests, establish baseline

**Success Criteria:**
- 80%+ mutation score for classification module
- All property tests passing
- No performance regression in test suite

#### Week 2-3: Phase 2 (Domain Validation)
- [ ] Add property tests for TimeEntry validation
- [ ] Add property tests for Activity validation
- [ ] Add serialization round-trip tests
- [ ] Run mutation tests on domain layer

**Success Criteria:**
- 90%+ mutation score for validation logic
- All domain types have serialization tests

#### Week 3-4: Phases 3-6 (Infrastructure & Utilities)
- [ ] Add property tests for repositories
- [ ] Add property tests for sync/outbox
- [ ] Add property tests for crypto/security
- [ ] Add property tests for error handling
- [ ] Comprehensive mutation testing report

**Success Criteria:**
- 75%+ mutation score for infrastructure
- 85%+ mutation score for crypto/security
- All critical paths have property tests

### CI Integration

Add to `.github/workflows/ci.yml`:

```yaml
# Property-based tests (run on every PR)
- name: Run property tests
  run: |
    cargo test --workspace --all-features -- --include-ignored

# Mutation tests (run weekly or on-demand)
- name: Mutation testing
  if: github.event_name == 'schedule' || contains(github.event.head_commit.message, '[mutation]')
  run: |
    cargo install cargo-mutants
    cargo mutants -p pulsearc-core --output mutants.json
    cargo mutants --check --minimum-score 80

- name: Upload mutation report
  uses: actions/upload-artifact@v3
  with:
    name: mutation-report
    path: mutants.json
```

---

## Test Strategy Generators

Create reusable proptest strategies in `test-utils`:

```rust
// crates/common/src/testing/strategies.rs

use proptest::prelude::*;
use chrono::{DateTime, Utc};

pub fn timestamp_strategy() -> impl Strategy<Value = DateTime<Utc>> {
    (0i64..2_000_000_000).prop_map(|ts| {
        DateTime::from_timestamp(ts, 0).unwrap()
    })
}

pub fn activity_strategy() -> impl Strategy<Value = Activity> {
    (
        timestamp_strategy(),
        "[A-Za-z ]{1,50}",  // app_name
        ".*",               // window_title
        option::of("https://[a-z]+\\.com/.*"),  // url
    ).prop_map(|(timestamp, app, title, url)| {
        Activity {
            id: Uuid::new_v4(),
            context: ActivityContext {
                app_name: app,
                window_title: title,
                url,
                local_path: None,
                bundle_id: None,
                timestamp,
            },
            created_at: timestamp,
            segment_id: None,
        }
    })
}

pub fn time_entry_strategy() -> impl Strategy<Value = TimeEntry> {
    (
        timestamp_strategy(),
        1u64..28800,  // duration in seconds (1s to 8h)
        "[A-Z]+-[0-9]+",  // project_code
    ).prop_map(|(start, duration_secs, project_code)| {
        let end = start + Duration::seconds(duration_secs as i64);
        TimeEntry {
            id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            project_code,
            task_description: "Test task".to_string(),
            start_time: start,
            end_time: end,
            duration_hours: duration_secs as f64 / 3600.0,
            evidence: vec![],
            submitted_at: None,
            external_id: None,
        }
    })
}
```

---

## Metrics & Reporting

### Track These Metrics

1. **Mutation Score**: % of mutants killed by tests
   - Target: 80%+ for core business logic
   - Target: 75%+ for infrastructure
   - Target: 90%+ for security/crypto

2. **Property Test Coverage**: % of modules with property tests
   - Target: 100% of critical paths (defined above)

3. **Test Execution Time**: Monitor performance impact
   - Property tests should add <30% to test time
   - Mutation tests run separately (too slow for CI)

4. **Bugs Found**: Track bugs discovered by property/mutation tests
   - Document in test commit messages

### Monthly Review

- Run full mutation test suite
- Review survived mutants
- Add tests for any gaps
- Update this document with new critical paths

---

## Success Metrics

- [ ] 80%+ mutation score for `pulsearc-core`
- [ ] 90%+ mutation score for validation logic
- [ ] 85%+ mutation score for crypto/security
- [ ] Zero panics in property tests (fuzz-tested robustness)
- [ ] All critical paths have property tests
- [ ] Mutation tests run weekly in CI
- [ ] Property tests run on every PR
- [ ] Test suite runs in <5 minutes (with property tests)

---

## Resources

- [proptest Documentation](https://docs.rs/proptest/latest/proptest/)
- [cargo-mutants GitHub](https://github.com/sourcefrog/cargo-mutants)
- [Property-Based Testing Book](https://fsharpforfunandprofit.com/posts/property-based-testing/)
- [Mutation Testing Explained](https://stryker-mutator.io/docs/)

---

## Open Questions

1. Should we run mutation tests on every PR or just weekly?
   - **Recommendation:** Weekly scheduled + on-demand (commit message trigger)
   - **Rationale:** Too slow for every PR (~10-30 min per crate)

2. What's acceptable test suite slowdown from property tests?
   - **Recommendation:** <30% increase in test time
   - **Mitigation:** Limit iterations for fast feedback, more iterations in CI

3. Should we test infrastructure exhaustively or focus on core logic?
   - **Recommendation:** Focus on core first (Phase 1-2), infrastructure second
   - **Rationale:** Core bugs affect all users, infra bugs are more isolated

4. How do we handle flaky property tests (rare failures)?
   - **Recommendation:** Use `proptest`'s shrinking to find minimal failing case
   - **Recommendation:** Set random seed for reproducibility

---

## Related Documents

- [QUALITY-CHECKLIST.md](./QUALITY-CHECKLIST.md) - General quality gates
- [SQLCIPHER-API-REFERENCE.md](./SQLCIPHER-API-REFERENCE.md) - Database testing patterns
- [CLAUDE.md](../../CLAUDE.md) - Testing requirements (section 6)

---

## Next Steps

1. **Review & Approve** this plan with team
2. **Prioritize** critical paths (confirm list above)
3. **Assign** ownership for each phase
4. **Kick off** Phase 1 implementation
5. **Track** progress weekly in standup