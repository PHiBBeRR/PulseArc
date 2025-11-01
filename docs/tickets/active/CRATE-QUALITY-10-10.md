# Crate Quality Improvement: 8.5/10 → 10/10

**Status:** 📋 Planned
**Priority:** High
**Effort:** 3 weeks
**Current Rating:** 8.5/10
**Target Rating:** 10/10

---

## Executive Summary

Improve crates folder from excellent (8.5/10) to world-class (10/10) through systematic testing, documentation, and quality improvements. The architecture is already excellent - this work focuses on validation, examples, and polish.

### Current Strengths
- ✅ Pristine layered architecture (DDD + Hexagonal)
- ✅ Exceptional workspace configuration
- ✅ Tiered feature flags in common crate
- ✅ Comprehensive documentation
- ✅ Proper dependency management

### Critical Gaps
- ❌ Domain layer: 0 tests for 2,869 LOC (critical)
- ❌ API layer: 0 tests for Tauri commands (high risk)
- ❌ Core layer: Only 4 test files for 4,568 LOC (insufficient)
- ⚠️ Workspace member lockfile exists (violates conventions)

---

## 🔴 Phase 1: Critical Path (Week 1) → 9.0/10

**Goal:** Fix critical issues and establish test baseline
**Duration:** 5 days
**Impact:** +0.5 rating points

### Task 1.1: Remove Workspace Member Lockfile ⚡
**Effort:** 5 minutes
**Priority:** Critical

- [ ] Remove `crates/api/Cargo.lock` from git
  ```bash
  git rm crates/api/Cargo.lock
  ```
- [ ] Add ignore rule to prevent future occurrences
  ```bash
  echo "crates/*/Cargo.lock" >> .gitignore
  ```
- [ ] Verify only root `Cargo.lock` exists
  ```bash
  find . -name "Cargo.lock" -type f
  ```

**Acceptance Criteria:**
- Only root `Cargo.lock` exists
- `.gitignore` prevents workspace member lockfiles
- Clean `git status`

---

### Task 1.2: Domain Layer Test Suite 📊
**Effort:** 2-3 days
**Priority:** Critical
**Target Coverage:** 80%+

#### Setup
- [ ] Create test directory structure
  ```bash
  mkdir -p crates/domain/tests
  ```
- [ ] Add test dependencies to `crates/domain/Cargo.toml`
  ```toml
  [dev-dependencies]
  tokio = { workspace = true, features = ["test-util"] }
  proptest = "1.5"
  insta = "1.40"  # Snapshot testing for serialization
  ```

#### Test Files to Create

- [ ] **`tests/classification_types.rs`** (Priority: Critical)
  - Test classification rules and logic
  - Test project matching algorithms
  - Test category validation
  - Test confidence score calculations
  - **Target:** 50+ test cases for [classification.rs](../crates/domain/src/types/classification.rs) (22KB)

- [ ] **`tests/database_types.rs`** (Priority: Critical)
  - Test entity constructors and builders
  - Test field validation and constraints
  - Test serialization/deserialization (use `insta` for snapshots)
  - Test type conversions
  - **Target:** 40+ test cases for [database.rs](../crates/domain/src/types/database.rs) (24KB)

- [ ] **`tests/stats_types.rs`** (Priority: High)
  - Test statistical calculations
  - Test aggregation logic
  - Test time window computations
  - Test metrics formulas
  - **Target:** 30+ test cases for [stats.rs](../crates/domain/src/types/stats.rs) (9.8KB)

- [ ] **`tests/idle_types.rs`** (Priority: Medium)
  - Test idle detection logic
  - Test threshold calculations
  - **Target:** 10+ test cases

- [ ] **`tests/sap_types.rs`** (Priority: Medium)
  - Test SAP-specific domain logic
  - **Target:** 10+ test cases

- [ ] **`tests/user_types.rs`** (Priority: Medium)
  - Test user profile validation
  - Test user preferences
  - **Target:** 10+ test cases

#### Inline Unit Tests
- [ ] Add `#[cfg(test)]` modules to each domain type file
  ```rust
  // In each .rs file under types/
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_invariants() { /* ... */ }

      #[test]
      fn test_edge_cases() { /* ... */ }
  }
  ```

#### Property-Based Tests
- [ ] Add property tests for domain invariants
  ```rust
  // tests/property_tests.rs
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn test_classification_roundtrip(
          category in "[A-Za-z]+",
          confidence in 0.0f64..1.0
      ) {
          // Test serialization roundtrip
      }
  }
  ```

**Acceptance Criteria:**
- [ ] At least 150 total test cases for domain layer
- [ ] 80%+ line coverage (measured with `cargo tarpaulin`)
- [ ] All tests pass in CI
- [ ] No panics, unwraps, or expects in domain code (only `Result<T, E>`)

**Verification:**
```bash
cd crates/domain
cargo test
cargo tarpaulin --out Html
# Open tarpaulin-report.html, verify 80%+ coverage
```

---

### Task 1.3: Core Business Logic Tests 🧪
**Effort:** 3-4 days
**Priority:** Critical
**Target Coverage:** 70%+

#### Setup
- [ ] Expand test directory
  ```bash
  mkdir -p crates/core/tests/{services,ports,integration}
  ```

#### Service Tests (Port Mocking)

- [ ] **`tests/services/classification_service_test.rs`**
  - Mock `BlockRepository`, `ProjectMatcher`, `Classifier` ports
  - Test classification orchestration logic
  - Test error handling paths
  - Test edge cases (empty input, missing data)
  - **Target:** 25+ test scenarios

- [ ] **`tests/services/tracking_service_test.rs`**
  - Mock `ActivityRepository`, `SegmentRepository`, `SnapshotRepository` ports
  - Test activity tracking workflows
  - Test snapshot creation logic
  - Test idle detection integration
  - **Target:** 25+ test scenarios

- [ ] **`tests/services/batch_operations_test.rs`**
  - Mock `BatchRepository`, `DlqRepository` ports
  - Test batch processing logic
  - Test retry and failure handling
  - Test DLQ routing
  - **Target:** 15+ test scenarios

- [ ] **`tests/services/sync_operations_test.rs`**
  - Mock `OutboxQueue`, `IdMappingRepository`, `TokenUsageRepository` ports
  - Test sync orchestration
  - Test conflict resolution
  - Test idempotency
  - **Target:** 20+ test scenarios

#### Port Contract Tests

- [ ] **`tests/ports/port_contracts_test.rs`**
  - Test that all port traits are object-safe
  - Test that port methods have proper error handling
  - Document expected behavior for each port
  - **Target:** Contract tests for all 15+ ports

#### Integration Tests (Cross-Module)

- [ ] **`tests/integration/end_to_end_workflow_test.rs`**
  - Test complete workflows across multiple services
  - Use in-memory implementations of ports
  - Test error propagation across layers
  - **Target:** 10+ end-to-end scenarios

#### Mock Infrastructure
- [ ] Add `mockall` for port mocking
  ```toml
  [dev-dependencies]
  mockall = "0.13"
  ```
- [ ] Create mock helpers in `tests/common/mocks.rs`
  ```rust
  use mockall::mock;

  mock! {
      pub BlockRepository {}

      impl BlockRepository for BlockRepository {
          async fn get_block(&self, id: &str) -> Result<Block, Error>;
          // ... other methods
      }
  }
  ```

**Acceptance Criteria:**
- [ ] At least 100 total test cases for core layer
- [ ] 70%+ line coverage (measured with `cargo tarpaulin`)
- [ ] All port traits have contract tests
- [ ] All services have happy path + error path tests
- [ ] End-to-end integration tests pass

**Verification:**
```bash
cd crates/core
cargo test
cargo tarpaulin --out Html
```

---

## 🟡 Phase 2: High Impact (Week 2) → 9.5/10

**Goal:** Add API tests and advanced testing techniques
**Duration:** 5 days
**Impact:** +0.5 rating points

### Task 2.1: API/Tauri Command Tests 🎯
**Effort:** 2-3 days
**Priority:** High

#### Setup
- [ ] Create test infrastructure
  ```bash
  mkdir -p crates/api/tests/{commands,integration}
  ```
- [ ] Add test dependencies
  ```toml
  [dev-dependencies]
  tauri = { version = "2.9", features = ["test"] }
  mockall = "0.13"
  ```

#### Command Tests

- [ ] **`tests/commands/tracking_commands_test.rs`**
  - Test `get_current_activity` command
  - Test `start_tracking` command
  - Test `stop_tracking` command
  - Test error responses
  - **Target:** 15+ command tests

- [ ] **`tests/commands/classification_commands_test.rs`**
  - Test `classify_activity` command
  - Test `get_classification_rules` command
  - Test `update_classification` command
  - **Target:** 10+ command tests

- [ ] **`tests/commands/sync_commands_test.rs`**
  - Test `sync_data` command
  - Test `get_sync_status` command
  - Test `force_sync` command
  - **Target:** 10+ command tests

- [ ] **`tests/commands/feature_flags_test.rs`**
  - Test `get_feature_flags` command
  - Test `toggle_feature` command
  - Test feature flag propagation
  - **Target:** 8+ command tests

#### Integration Tests

- [ ] **`tests/integration/end_to_end_test.rs`**
  - Test complete user workflows via Tauri commands
  - Test state management across commands
  - Test error recovery
  - **Target:** 5+ end-to-end scenarios

#### Context Tests

- [ ] **`tests/context/app_context_test.rs`**
  - Test context initialization
  - Test dependency injection
  - Test lifecycle management
  - **Target:** 10+ context tests

**Acceptance Criteria:**
- [ ] All Tauri commands have tests
- [ ] Happy path + error path coverage
- [ ] Integration tests cover key user workflows
- [ ] 60%+ coverage for API layer

**Verification:**
```bash
cd crates/api
cargo test
```

---

### Task 2.2: Advanced Testing Techniques 🔬
**Effort:** 1-2 days
**Priority:** High

#### Property-Based Testing Expansion

- [ ] Add `proptest` to workspace dependencies
  ```toml
  [workspace.dependencies]
  proptest = "1.5"
  ```

- [ ] **Domain property tests** (`crates/domain/tests/property_tests.rs`)
  - Test serialization roundtrips
  - Test domain invariants hold under random inputs
  - Test validation logic with fuzzing
  - **Target:** 20+ property tests

- [ ] **Core property tests** (`crates/core/tests/property_tests.rs`)
  - Test service idempotency
  - Test commutative operations
  - Test associative operations
  - **Target:** 15+ property tests

#### Snapshot Testing

- [ ] Add `insta` for snapshot testing
  ```toml
  [workspace.dependencies]
  insta = { version = "1.40", features = ["json", "redactions"] }
  ```

- [ ] Create snapshot tests for serialization
  - Domain type JSON snapshots
  - API response snapshots
  - Error message snapshots
  - **Target:** 30+ snapshot tests

#### Mutation Testing Setup

- [ ] Install `cargo-mutants`
  ```bash
  cargo install cargo-mutants
  ```

- [ ] Create `.cargo/mutants.toml`
  ```toml
  exclude_crates = ["pulsearc-app"]
  test_crates = ["pulsearc-domain", "pulsearc-core"]
  minimum_test_timeout = 300
  ```

- [ ] Run mutation tests
  ```bash
  cargo mutants
  ```

- [ ] Fix tests until 80%+ mutation score achieved

**Acceptance Criteria:**
- [ ] Property tests cover domain invariants
- [ ] Snapshot tests cover all serialization
- [ ] Mutation score ≥ 80% for domain/core
- [ ] CI runs property tests on every PR

**Verification:**
```bash
cargo test --workspace
cargo mutants
# Check mutants.out/caught.txt for score
```

---

### Task 2.3: Infra Layer Test Improvements 🏗️
**Effort:** 1-2 days
**Priority:** Medium

- [ ] Review existing infra tests (6 test files, 94 source files)
- [ ] Identify untested repositories
- [ ] Add tests for:
  - [ ] All repository implementations
  - [ ] API client integrations (use `wiremock`)
  - [ ] Database migrations
  - [ ] Platform-specific code (macOS)

**Target:** 50%+ coverage for infra layer

**Acceptance Criteria:**
- [ ] All repositories have happy path tests
- [ ] HTTP integrations use `wiremock`
- [ ] Database tests use temp databases
- [ ] Platform code has feature-gated tests

---

## 🟢 Phase 3: Polish (Week 3) → 10/10

**Goal:** Examples, documentation, and final polish
**Duration:** 5 days
**Impact:** +0.5 rating points

### Task 3.1: Examples Directory 📚
**Effort:** 1-2 days
**Priority:** Medium

#### Setup
- [ ] Create examples directories
  ```bash
  mkdir -p crates/{common,core,infra}/examples
  ```

#### Common Crate Examples

- [ ] **`examples/basic_resilience.rs`**
  - Circuit breaker usage
  - Retry with backoff
  - Combined resilience patterns
  - **Target:** Complete working example

- [ ] **`examples/oauth_flow.rs`**
  - Full OAuth 2.0 + PKCE flow
  - Token management
  - Auto-refresh
  - **Target:** Runnable OAuth demo

- [ ] **`examples/encrypted_storage.rs`**
  - SQLCipher connection setup
  - Key management
  - Encrypted queries
  - **Target:** Working storage example

- [ ] **`examples/validation_framework.rs`**
  - Field validation
  - Rule composition
  - Custom validators
  - **Target:** Comprehensive validation demo

- [ ] **`examples/cache_patterns.rs`**
  - TTL cache usage
  - Cache stats
  - Async cache
  - **Target:** Cache usage patterns

#### Core Crate Examples

- [ ] **`examples/classification_service.rs`**
  - Service initialization
  - Classification workflow
  - Error handling
  - **Target:** End-to-end classification example

- [ ] **`examples/tracking_service.rs`**
  - Activity tracking setup
  - Snapshot management
  - Idle detection
  - **Target:** End-to-end tracking example

#### Infra Crate Examples

- [ ] **`examples/repository_pattern.rs`**
  - Repository implementation
  - Transaction handling
  - Error mapping
  - **Target:** Repository best practices

- [ ] **`examples/api_integration.rs`**
  - HTTP client usage
  - Retry logic
  - Circuit breaker integration
  - **Target:** External API integration pattern

#### Documentation

- [ ] Update each example's Cargo.toml
  ```toml
  [[example]]
  name = "basic_resilience"
  path = "examples/basic_resilience.rs"
  required-features = ["runtime"]
  ```

- [ ] Add README to each examples directory
- [ ] Ensure all examples are runnable
  ```bash
  cargo run --example basic_resilience --features runtime
  ```

**Acceptance Criteria:**
- [ ] At least 10 working examples across crates
- [ ] All examples documented with comments
- [ ] All examples runnable with `cargo run --example`
- [ ] Examples cover common use cases

---

### Task 3.2: Benchmark Suite Expansion 📈
**Effort:** 1 day
**Priority:** Medium

#### Setup
- [ ] Create benchmark directories
  ```bash
  mkdir -p crates/{core,domain}/benches
  ```

#### Domain Benchmarks

- [ ] **`benches/serialization_bench.rs`**
  - Benchmark domain type serialization
  - Compare serde formats (JSON vs bincode)
  - **Target:** Baseline performance metrics

- [ ] **`benches/validation_bench.rs`**
  - Benchmark field validation
  - Benchmark rule composition
  - **Target:** Performance characteristics

#### Core Benchmarks

- [ ] **`benches/classification_bench.rs`**
  - Benchmark classification service
  - Benchmark pattern matching
  - Benchmark project lookup
  - **Target:** Identify bottlenecks

- [ ] **`benches/tracking_bench.rs`**
  - Benchmark activity creation
  - Benchmark snapshot generation
  - **Target:** Performance baseline

#### Configuration

- [ ] Add to Cargo.toml
  ```toml
  [[bench]]
  name = "classification_bench"
  harness = false

  [dev-dependencies]
  criterion = { workspace = true }
  ```

- [ ] Create CI job for benchmark tracking
  ```yaml
  # .github/workflows/bench.yml
  - name: Run benchmarks
    run: cargo bench --workspace
  ```

**Acceptance Criteria:**
- [ ] Critical paths benchmarked
- [ ] Baseline metrics established
- [ ] CI tracks benchmark results
- [ ] Performance regressions detected

**Verification:**
```bash
cargo bench --workspace
# Check target/criterion/reports/index.html
```

---

### Task 3.3: Architecture Decision Records (ADRs) 📝
**Effort:** 4-6 hours
**Priority:** Medium

#### Setup
- [ ] Create ADR directory
  ```bash
  mkdir -p docs/architecture/decisions
  ```

#### ADRs to Create

- [ ] **`0001-layered-architecture.md`**
  - Document 5-layer architecture decision
  - Rationale for DDD approach
  - Consequences and trade-offs

- [ ] **`0002-domain-driven-design.md`**
  - Pure domain model rationale
  - Bounded contexts
  - Ubiquitous language

- [ ] **`0003-ports-and-adapters.md`**
  - Hexagonal architecture choice
  - Port trait design
  - Adapter implementation strategy

- [ ] **`0004-feature-flag-tiers.md`**
  - Tiered feature flag design (foundation/runtime/platform)
  - Compilation time benefits
  - Optional dependency rationale

- [ ] **`0005-sqlcipher-migration.md`**
  - Decision to use SqlCipherConnection
  - LocalDatabase deprecation
  - Migration strategy

- [ ] **`0006-workspace-organization.md`**
  - Workspace structure rationale
  - Shared dependency strategy
  - Lint configuration approach

- [ ] **`0007-testing-strategy.md`**
  - Test pyramid for each layer
  - Mock vs integration test trade-offs
  - Property-based testing adoption

#### Template

```markdown
# ADR-XXXX: Title

**Status:** Accepted | Proposed | Deprecated | Superseded
**Date:** YYYY-MM-DD
**Deciders:** Team members

## Context
What is the issue we're seeing that is motivating this decision?

## Decision
What is the change we're proposing and/or doing?

## Consequences
What becomes easier or more difficult to do because of this change?

### Positive
- Benefit 1
- Benefit 2

### Negative
- Trade-off 1
- Trade-off 2

## Alternatives Considered
- Alternative 1: Why rejected
- Alternative 2: Why rejected

## References
- Link to related docs
- Link to implementation
```

**Acceptance Criteria:**
- [ ] At least 7 ADRs documented
- [ ] All major architectural decisions covered
- [ ] ADR index/README created
- [ ] ADRs linked from main documentation

---

### Task 3.4: Coverage & Quality Metrics 📊
**Effort:** 1 day
**Priority:** High

#### Code Coverage Setup

- [ ] Install `cargo-tarpaulin`
  ```bash
  cargo install cargo-tarpaulin
  ```

- [ ] Create coverage script
  ```bash
  #!/bin/bash
  # scripts/coverage.sh
  cargo tarpaulin \
    --workspace \
    --out Html \
    --out Json \
    --output-dir coverage \
    --exclude-files "benches/*" "tests/*" "examples/*"

  echo "Coverage report: coverage/tarpaulin-report.html"
  ```

- [ ] Add CI coverage job
  ```yaml
  # .github/workflows/coverage.yml
  - name: Code Coverage
    run: |
      cargo tarpaulin --workspace --out Xml
      bash <(curl -s https://codecov.io/bash)
  ```

#### Quality Metrics

- [ ] Create quality dashboard script
  ```bash
  #!/bin/bash
  # scripts/quality-metrics.sh

  echo "=== Code Coverage ==="
  cargo tarpaulin --workspace --print-summary

  echo -e "\n=== Test Counts ==="
  cargo test --workspace -- --list | wc -l

  echo -e "\n=== Mutation Score ==="
  cargo mutants --list | tail -1

  echo -e "\n=== Documentation Coverage ==="
  cargo doc --workspace --no-deps 2>&1 | grep -i warning

  echo -e "\n=== Lint Warnings ==="
  cargo clippy --workspace -- -D warnings
  ```

#### Targets

- [ ] **Domain:** 80%+ coverage
- [ ] **Core:** 70%+ coverage
- [ ] **Infra:** 50%+ coverage
- [ ] **API:** 60%+ coverage
- [ ] **Overall:** 70%+ coverage

**Acceptance Criteria:**
- [ ] Coverage targets met
- [ ] CI enforces coverage thresholds
- [ ] Quality metrics tracked over time
- [ ] Coverage badges added to README

---

### Task 3.5: Documentation Polish 📖
**Effort:** 1 day
**Priority:** Medium

#### Crate-Level Documentation

- [ ] **Update README files**
  - [x] `crates/common/README.md` (already excellent)
  - [ ] `crates/domain/README.md` (needs expansion)
  - [ ] `crates/core/README.md` (needs expansion)
  - [ ] `crates/infra/README.md` (needs expansion)
  - [ ] `crates/api/README.md` (create from scratch)

#### API Documentation

- [ ] Add missing doc comments
  ```rust
  /// Classifies an activity based on context.
  ///
  /// # Arguments
  /// * `context` - The activity context to classify
  ///
  /// # Returns
  /// * `Ok(Classification)` - Successful classification
  /// * `Err(ClassificationError)` - Classification failed
  ///
  /// # Examples
  /// ```rust
  /// let classification = service.classify(&context).await?;
  /// ```
  pub async fn classify(&self, context: &ActivityContext) -> Result<Classification> {
      // ...
  }
  ```

- [ ] Generate and review docs
  ```bash
  cargo doc --workspace --no-deps --document-private-items
  open target/doc/pulsearc_domain/index.html
  ```

- [ ] Fix all `missing_docs` warnings
  ```bash
  cargo clippy --workspace -- -D missing_docs
  ```

#### Module-Level Documentation

- [ ] Add module-level docs to all public modules
  ```rust
  //! # Classification Module
  //!
  //! Provides activity classification services using pattern matching
  //! and machine learning.
  //!
  //! ## Overview
  //! The classification system matches activities to projects based on...
  //!
  //! ## Usage
  //! ```rust
  //! use pulsearc_core::ClassificationService;
  //! // ...
  //! ```
  ```

**Acceptance Criteria:**
- [ ] All crates have comprehensive READMEs
- [ ] All public APIs documented with examples
- [ ] No `missing_docs` warnings
- [ ] `cargo doc` builds without warnings

---

### Task 3.6: CI/CD Pipeline Enhancement 🚀
**Effort:** 4 hours
**Priority:** High

#### Test Matrix

- [ ] Create comprehensive CI pipeline
  ```yaml
  # .github/workflows/ci.yml
  name: CI

  on: [push, pull_request]

  jobs:
    test:
      strategy:
        matrix:
          crate:
            - pulsearc-domain
            - pulsearc-core
            - pulsearc-common
            - pulsearc-infra
            - pulsearc-app
      steps:
        - name: Test ${{ matrix.crate }}
          run: cargo test -p ${{ matrix.crate }}

    coverage:
      steps:
        - name: Coverage
          run: cargo tarpaulin --workspace --out Xml
        - name: Upload to Codecov
          uses: codecov/codecov-action@v3

    mutation:
      steps:
        - name: Mutation Testing
          run: |
            cargo mutants -p pulsearc-domain -p pulsearc-core
            # Fail if mutation score < 80%

    bench:
      steps:
        - name: Benchmarks
          run: cargo bench --workspace --no-fail-fast
        - name: Archive results
          uses: actions/upload-artifact@v3
          with:
            name: benchmark-results
            path: target/criterion
  ```

#### Quality Gates

- [ ] Add branch protection rules
  - Require CI passing
  - Require 70%+ coverage
  - Require mutation score ≥ 80%

**Acceptance Criteria:**
- [ ] All tests run on every PR
- [ ] Coverage uploaded to Codecov
- [ ] Mutation tests run weekly
- [ ] Benchmarks archived

---

## 📊 Success Metrics

Track progress with these commands:

```bash
# Overall test count
cargo test --workspace -- --list | wc -l
# Target: 300+ tests

# Coverage
cargo tarpaulin --workspace --print-summary
# Target: 70%+ overall

# Mutation score
cargo mutants
# Target: 80%+ for domain/core

# Documentation coverage
cargo doc --workspace --no-deps 2>&1 | grep -i warning | wc -l
# Target: 0 warnings

# Lint status
cargo clippy --workspace --all-targets -- -D warnings
# Target: 0 warnings
```

---

## Definition of Done

### Phase 1 Complete (9.0/10) ✅
- [ ] No workspace member lockfile
- [ ] Domain: 150+ tests, 80%+ coverage
- [ ] Core: 100+ tests, 70%+ coverage
- [ ] All CI checks passing

### Phase 2 Complete (9.5/10) ✅
- [ ] API: 50+ tests, 60%+ coverage
- [ ] Property tests for domain/core
- [ ] Mutation score ≥ 80%
- [ ] Infra coverage ≥ 50%

### Phase 3 Complete (10/10) ✅
- [ ] 10+ working examples
- [ ] Benchmarks for critical paths
- [ ] 7+ ADRs documented
- [ ] Coverage tracked in CI
- [ ] All crates have comprehensive READMEs
- [ ] Zero documentation warnings
- [ ] Quality metrics dashboard

---

## Resources

### Tools to Install
```bash
cargo install cargo-tarpaulin  # Coverage
cargo install cargo-mutants    # Mutation testing
cargo install cargo-audit       # Security audits
cargo install cargo-outdated    # Dependency updates
```

### Documentation
- [Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Property-Based Testing](https://github.com/proptest-rs/proptest)
- [Mutation Testing](https://mutants.rs/)
- [ADR Template](https://github.com/joelparkerhenderson/architecture-decision-record)

### References
- Current rating analysis: (this document)
- CLAUDE.md: Project rules and conventions
- Common API Guide: `crates/common/docs/API_GUIDE.md`

---

## Risk Assessment

### High Risk
- **Time investment:** 3 weeks is significant
  - **Mitigation:** Break into phases, deliver value incrementally
- **Test maintenance:** More tests = more maintenance
  - **Mitigation:** Focus on high-value tests, use property tests

### Medium Risk
- **Coverage targets may be ambitious**
  - **Mitigation:** Adjust targets based on complexity
- **Mutation testing may reveal test quality issues**
  - **Mitigation:** Expected and desired - fix tests iteratively

### Low Risk
- **Breaking existing functionality**
  - **Mitigation:** Only adding tests, not changing code
- **CI pipeline may get slow**
  - **Mitigation:** Parallel execution, caching, selective runs

---

## Notes

- This ticket assumes the codebase is already stable and functional
- Focus is on validation, not implementation
- Tests should serve as documentation and regression prevention
- Quality metrics should be tracked over time, not just hit once
- Examples should be runnable and maintained

---

**Last Updated:** 2025-10-31
**Created By:** Claude (Architecture Review)
**Reviewed By:** _Pending_
