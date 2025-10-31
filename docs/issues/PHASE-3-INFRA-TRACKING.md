# Phase 3: Infrastructure Adapters - Detailed Tracking

**Status:** 🔄 IN PROGRESS (Started October 31, 2025)
**Created:** 2025-01-30
**Updated:** 2025-11-02 (Phase 3F COMPLETE - Observability infrastructure ✅)
**Owner:** TBD
**Dependencies:** ✅ Phase 2 (Core Business Logic) COMPLETE
**Estimated Duration:** 4-6 weeks (23-31 working days)
**Current Progress:** Phase 3A: 1/10 tasks (Task 3A.1 ✅) + MDM cert infrastructure ✅ | **Phase 3F: COMPLETE ✅** (1,217 LOC, 66 tests)

---

## Executive Summary

Phase 3 migrates **~50+ infrastructure modules** (~15,000+ LOC) from `legacy/api/src/` to `crates/infra/`. This is the **largest and most complex phase** of the ADR-003 migration, implementing all port adapters defined in Phase 1 and used by Phase 2 business logic.

**Why This Phase is Critical:**
- Connects business logic to real databases, APIs, and platform services
- Implements platform-specific code (macOS primary, Windows/Linux future work)
- Adds feature-gated integrations (SAP, Calendar, ML)
- Enables end-to-end functionality testing

**Platform Scope:**
- ✅ **macOS** - Full implementation (primary target)
- 🔮 **Windows/Linux** - Placeholder/basic support (future enhancement, not Phase 3 scope)

**Phase Scope:**
- ✅ Database repositories (SqlCipher connection pool, all CRUD operations)
- ✅ Platform providers (macOS Accessibility API, Windows/Linux fallbacks)
- ✅ HTTP clients (reqwest-based, retry/timeout logic)
- ✅ Integration adapters (SAP, Calendar, OpenAI) - feature-gated
- ✅ Schedulers (cron-based background jobs)
- ✅ Sync workers (outbox processing, Neon client, cost tracking)
- ✅ ML adapters (Linfa, training pipeline) - feature-gated
- ✅ Observability infrastructure (metrics collection)

---

## Phase Breakdown

### Sub-Phase Overview

| Phase | Focus | Duration | Modules | Priority | Blockers |
|-------|-------|----------|---------|----------|----------|
| **3A** | Core Infrastructure | 7-10 days | 15 | P1 | Phase 2 complete |
| **3B** | Platform Adapters | 4-6 days | 7 | P1 | 3A complete |
| **3C** | Integration Adapters | 5-7 days | 17 | P2 | 3A complete |
| **3D** | Schedulers & Workers | 4-5 days | 10 | P2 | 3A, 3C complete |
| **3E** | ML Adapters (optional) | 3-4 days | 6 | P3 | 3A complete |
| **3F** | Observability (parallel) | 2-3 days | 5+ | P3 | None |

**Total: 25-34 days (5.0-6.8 weeks)**

---

## Migration Strategy

### Approach: Gradual Migration with Adapter Pattern

**Migration Method:**
- **Gradual migration** - Legacy and new code coexist during migration
- **Legacy code** remains in `legacy/api/src/` (read-only, frozen)
- **New implementations** in `crates/infra/` (active development)
- **Phase 4** will rewire API layer to use new infrastructure
- **No runtime feature flags** needed (compile-time only)

**Benefits:**
- Low risk - Legacy code remains untouched as fallback
- Incremental testing - Each adapter tested independently
- Clear separation - No mixing old and new patterns
- Easy rollback - Simply use legacy implementations if needed

**Migration Flow:**
```
Phase 3: Build new adapters in crates/infra/
         ├─ Implement all port traits
         ├─ Add comprehensive tests
         └─ Verify feature parity with legacy

Phase 4: Rewire API layer
         ├─ Replace legacy calls with new adapters
         ├─ Remove legacy re-exports
         └─ Delete legacy code after validation
```

**Feature Flags Strategy:**
- **Required features** (default): `database`, `platform`
- **Optional features**: `sap`, `calendar`, `ml`
- All combinations must compile independently
- CI tests all feature combinations (see Phase 3 Validation)

---

## Phase 3A: Core Infrastructure (Week 3)

**Goal:** Establish foundational infrastructure adapters
**Duration:** 7-10 days
**Dependencies:** Phase 2 complete (see Phase 2 Blockers below)
**Priority:** P1 (blocks all other sub-phases)

### ⚠️ CRITICAL: SqlCipher API Difference

**IMPORTANT:** `SqlCipherStatement::query_map` returns `Vec<T>` directly, NOT `Rows<'_>` (an iterator).

Unlike standard `rusqlite::Statement::query_map` which returns an iterator, our `SqlCipherStatement::query_map` (line 114 in `agent/storage/sqlcipher/connection.rs`) **immediately collects results** and returns `StorageResult<Vec<T>>`.

```rust
// ❌ WRONG - query_map already returns Vec<T>, not an iterator
let results = stmt
    .query_map(params, |row| Ok(MyStruct { ... }))?
    .collect::<Result<Vec<_>, _>>()  // ❌ ERROR: Vec<T> is not IntoIterator
    .map_err(|e| ...)?;

// ✅ CORRECT - query_map already collected the results
let results = stmt
    .query_map(params, |row| Ok(MyStruct { ... }))?;
```

**Reference:** See [docs/issues/SQLCIPHER-API-REFERENCE.md](../issues/SQLCIPHER-API-REFERENCE.md) for detailed examples.

**Impact:** Every repository migration (Tasks 3A.5-3A.9) will hit this pattern. Review the reference doc before starting database work.

---

### Phase 2 Blockers

**✅ PHASE 2 COMPLETE (November 1, 2025):**
- ✅ Week 1 PR #1: WbsRepository trait and infrastructure (needed for 3C.3)
- ✅ Week 1 PR #2: Signal extraction and mock repositories (needed for 3A.7)
- ✅ Week 1 PR #3: Evidence extractor for OpenAI classification (needed for 3C.1)
- ✅ Week 2 PR #4: ProjectMatcher with FTS5 hybrid matching (needed for 3C.3)
- ✅ Week 2 PR #5: BlockBuilder with time consolidation and idle handling (commit `f220106`)

**Phase 2 Summary:**
- 5 PRs merged (~2,610 lines of business logic)
- 54 tests passing (all critical paths covered)
- Core classification modules complete

**🟢 READY TO START PHASE 3A** - All Phase 2 dependencies resolved!

---

### Newly Identified Legacy Blockers (Updated)

- **Critical – Outbox retry filter regression** (`legacy/api/src/db/outbox/outbox.rs:560`): the current query filters on `status = 'sent'`, so `OutboxWorker::process_pending_entries` never drains new items. Pending entries never sync and retry-after logic is bypassed. **Action:** swap the predicate to `status = 'pending'` and add a regression test before porting the worker in Phase 3D.3.
- **Critical – SAP forwarder hard-coded date fallback** (`legacy/api/src/integrations/sap/forwarder.rs:144`): missing `date` values default to `"2025-10-22"`, corrupting downstream SAP records and breaking idempotency. **Action:** fail fast when the field is absent (or derive it from the entry timestamp) and cover with tests ahead of the adapter migration.

**Additional hardening (parallel to 3A):**
- **High – Outbox status parsing panic** (`legacy/api/src/db/outbox/outbox.rs:70`): `parse().unwrap()` will panic on unexpected status strings. Map the failure to `DbError` (or default with telemetry) so bad rows do not crash the pipeline during migration.
- **Medium-high – Date-based queries bypass indexes** (`legacy/api/src/infra/repositories/segment_repository.rs` & `snapshot_repository.rs`): using `date(column, 'unixepoch')` forces full scans and assumes UTC boundaries. Switch to `[start, end)` range predicates to stay index-friendly and align with domain day semantics.

**Recommendation:** Fix the two critical issues before kicking off Phase 3A to ensure we migrate clean behavior; schedule the follow-up items alongside the Phase 3A database work.

---

### Task 3A.0: Establish Performance Baseline (Day 0 - Pre-work)

**Goal:** Measure legacy performance for comparison with new infrastructure

**Scope:**
- Benchmark legacy database operations (snapshot save, time-range queries)
- Benchmark legacy activity provider (macOS capture with enrichment)
- Benchmark legacy HTTP client (API calls with retry)
- Record baseline metrics for Phase 3 validation

**Implementation Checklist:**
- [x] Create `benchmarks/infra-baselines/benches/baseline.rs` with Criterion harness and shimmed legacy adapters
- [x] Benchmark legacy `DbManager` operations:
  - Single snapshot save
  - Time-range query (1 day, 100 snapshots)
  - Bulk insert (1000 snapshots)
- [x] Benchmark legacy `MacOsActivityProvider`:
  - Activity fetch without enrichment
  - Activity fetch with browser URL enrichment
- [x] Benchmark legacy HTTP client:
  - Single request
  - Request with retry (simulated transient failure)
- [x] Benchmark MDM TLS client (warm + cold handshake)
- [x] Document baseline results (p50/p99) in `docs/performance-baseline.md`

**Baseline Metrics to Capture:**
```
Database Operations (legacy):
- Snapshot save: 0.055 ms (p50), 0.067 ms (p99)
- Time-range query: 0.049 ms (p50), 0.055 ms (p99)
- Bulk insert (1000): 3.58 ms (p50), 4.19 ms (p99)

MDM Client (legacy shim):
- fetch_config (warm): 0.063 ms (p50), 0.066 ms (p99)
- fetch_and_merge (warm): 0.063 ms (p50), 0.069 ms (p99)
- fetch_config (cold TLS): 3.03 ms (p50), 3.17 ms (p99)

Activity Provider (legacy, macOS):
- Fetch (AX granted): 0.96 ms (p50), 1.18 ms (p99)
- Fetch with enrichment (AX granted): 0.95 ms (p50), 1.24 ms (p99)
- Fetch (AX forced off): 0.00011 ms (p50), 0.00014 ms (p99)

HTTP Client (legacy):
- Single request: 0.064 ms (p50), 0.091 ms (p99)
- With retry: 1.003 s (p50), 1.003 s (p99)
```

**Acceptance Criteria:**
- [x] Criterion benchmarks (DB, HTTP, MDM, macOS) run successfully (warm + cold TLS paths)
- [x] Baseline metrics (p50/p99) documented in `docs/performance-baseline.md`
- [x] Results committed for Phase 3 comparison (`benchmarks/infra-baselines/**` harness)
- [x] Repro command published (`cargo bench -p infra-baselines --offline` with env vars)

**Time:** 2-4 hours (can be done in parallel with planning)

---

### Task 3A.1: Configuration Loader (Day 1)

**Source:** `legacy/api/src/shared/config_loader.rs` → `crates/infra/src/config/loader.rs`

**Line Count:** ~200 LOC (estimate)

**Scope:**
- Environment variable reading
- File system probing (config paths)
- Executable path detection
- Config validation

**Implementation Checklist:**
- [ ] Create `crates/infra/src/config/loader.rs`
- [ ] Move `load_from_env()` function
- [ ] Move `load_from_file()` function
- [ ] Move `probe_config_paths()` function
- [ ] Update to use `pulsearc_domain::AppConfig` types
- [ ] Add error handling with `PulseArcError`
- [ ] Add unit tests (env vars, missing files, invalid JSON)
- [ ] Integration test: load valid config from test file

**Acceptance Criteria:**
- [ ] Loads config from environment variables
- [ ] Falls back to file if env vars missing
- [ ] Returns clear error for missing/invalid config
- [ ] Tests cover all branches
- [ ] `cargo test -p pulsearc-infra config::loader` passes

**Related Completion: MDM Infrastructure & Certificate Setup** (2025-10-31)

As part of configuration infrastructure, also completed MDM remote configuration support:

**Certificate Infrastructure:**
- ✅ Created `scripts/mdm/generate-test-certs.sh` - Self-signed certificate generator (270 LOC)
  - Generates CA, server, and client certificates for testing
  - Supports mutual TLS with client certificates
  - Includes PKCS#12 bundles for keychain import
  - Automatic `.gitignore` for private key security
  - Configurable via environment variables
- ✅ Created `scripts/mdm/README.md` - Comprehensive certificate documentation (400 LOC)
  - Rust/reqwest integration examples
  - CI/CD integration guides (GitHub Actions + self-hosted runner)
  - Security best practices
  - Troubleshooting section
- ✅ Added `.mdm-certs/` to root `.gitignore`
- ✅ Updated `docs/issues/MDM_EXTRACTION_GUIDE.md` with certificate requirements section

**MDM HTTP Client:**
- ✅ Created `crates/infra/src/mdm/client.rs` - Remote configuration fetcher (220 LOC)
  - `MdmClient::new()` - Production mode with default TLS validation
  - `MdmClient::with_ca_cert()` - Custom CA certificate support
  - `MdmClient::with_insecure_tls()` - Testing mode (`#[cfg(test)]` only)
  - `fetch_config()` - HTTPS-based config retrieval with validation
  - `fetch_and_merge()` - Merge remote with local config
  - Configurable timeouts via `with_timeout()`
  - Full tracing/logging support
  - Added `.no_proxy()` to avoid macOS dynamic store panics in tests
- ✅ Created `crates/infra/examples/mdm_remote_config.rs` - Complete usage example (130 LOC)
- ✅ Created `crates/infra/src/mdm/README.md` - Complete MDM documentation (400 LOC)
  - Architecture diagrams
  - API reference with examples
  - Security considerations (test vs production certs)
  - Troubleshooting guide
- ✅ All tests passing: 33 tests (27 MDM core + 6 client tests)

**Certificate Decision:**
- **Self-signed certificates** recommended for:
  - ✅ Local development and testing
  - ✅ CI/CD pipelines (self-hosted runner)
  - ✅ Internal testing environments
  - ✅ Unit/integration tests
- **Proper CA certificates** required for:
  - 🔴 Production deployments
  - 🔴 Apple Push Notification Service (APNs)
  - 🔴 Public-facing MDM servers
  - 🔴 Compliance requirements (SOC2, HIPAA, etc.)

**Files Created:**
- `scripts/mdm/generate-test-certs.sh` (270 LOC)
- `scripts/mdm/README.md` (400 LOC)
- `crates/infra/src/mdm/client.rs` (220 LOC)
- `crates/infra/src/mdm/README.md` (400 LOC)
- `crates/infra/examples/mdm_remote_config.rs` (130 LOC)
- **Total**: ~1,420 LOC of infrastructure + documentation

**Impact:**
- MDM module now has complete HTTPS support for remote configuration
- Certificate infrastructure ready for all TLS needs (MDM, HTTP clients, integrations)
- Self-hosted CI runner can generate and use test certificates automatically
- Production-ready path with CA certificate support documented
- Clear guidance on when to use self-signed vs proper certificates

---

### Task 3A.2: Error Conversions (Day 1)

**Source:** `legacy/api/src/observability/errors/conversions.rs` → `crates/infra/src/errors/conversions.rs`

**Line Count:** ~150 LOC (estimate)

**Scope:**
- `From<rusqlite::Error>` implementations
- `From<reqwest::Error>` implementations
- `From<keyring::Error>` implementations
- Other external error mappings

**Implementation Checklist:**
- [x] Create `crates/infra/src/errors/conversions.rs`
- [x] Move `From<rusqlite::Error> for PulseArcError`
- [x] Move `From<reqwest::Error> for PulseArcError`
- [x] Move `From<keyring::Error> for PulseArcError`
- [x] Add any missing external error conversions
- [x] Add unit tests for each conversion
- [x] Verify error messages are user-friendly

**Acceptance Criteria:**
- [x] All rusqlite errors map to domain errors (`InfraError` newtype wraps conversions)
- [x] HTTP errors preserve status codes / semantics (`reqwest::Error` → `PulseArcError`)
- [x] Keychain errors have actionable messages
- [x] `cargo test -p pulsearc-infra errors::conversions` *(covered via http::client suite)*
- [x] Documented usages exported via `InfraError`

**Status:** ✅ Complete (commit `ef5b5e2`, Oct 31 2025)
**Notes:** Conversion logic now lives in `crates/infra/src/errors/conversions.rs` and is re-exported through `InfraError`. HTTP client and other adapters can call `PulseArcError::from(InfraError)` to bubble errors to the domain layer.

---

### Task 3A.3: HTTP Client (Day 2)

**Source:** `legacy/api/src/http/client.rs` → `crates/infra/src/http/client.rs`

**Line Count:** ~300 LOC (estimate)

**Scope:**
- Reqwest-based HTTP client
- Retry logic with exponential backoff
- Timeout configuration
- Error handling

**Implementation Checklist:**
- [x] Create `crates/infra/src/http/client.rs`
- [x] Port `HttpClient` struct (reqwest-based)
- [x] Add retry logic with exponential backoff (local implementation, no external dependency yet)
- [x] Add timeout configuration (default: 30s)
- [x] Add request/response logging with `tracing`
- [x] Add unit tests with mock HTTP server (`wiremock`)
- [ ] Add integration test with real HTTP call (optional)

**Acceptance Criteria:**
- [x] Retries transient failures (5xx, network errors)
- [x] Does not retry 4xx errors
- [x] Respects timeout configuration
- [x] Logs requests and responses at DEBUG level
- [x] `cargo test -p pulsearc-infra http::client` passes

**Status:** ✅ Complete (commit `ef5b5e2` + `8b0a78c`, Oct 31 2025)
**Notes:** `HttpClient` now lives under `crates/infra/src/http/client.rs` and is re-exported via `HttpClient`/`HttpClientBuilder`. Wiremock-based tests cover success, 5xx retry, 4xx no-retry, and connection failures. Consider hooking into `pulsearc_common::resilience` once it lands in Phase 3B.

---

### Task 3A.4: Database Manager (Day 3)
**Deviation (2026-02-07):** Legacy `cargo xtask ci` currently skips clippy for `legacy/api` via `legacy/api/clippy.toml`. See `scripts/ci_legacy_eval.json` for blockers; follow-up issue tracks removing this suppression once legacy lint debt is paid down.

**Source:** `legacy/api/src/db/manager.rs` → `crates/infra/src/database/manager.rs`

**Line Count:** ~400 LOC (estimate)

**Scope:**
- SqlCipher connection pool setup
- Database initialization
- Connection lifecycle management
- Health checks

**Implementation Checklist:**
- [ ] Create `crates/infra/src/database/manager.rs`
- [ ] Port `DbManager` struct (or refactor to use existing `SqlCipherPool`)
- [ ] Add connection pool configuration (min/max connections)
- [ ] Add database initialization logic
- [ ] **Schema Migration Verification:**
  - [ ] Verify database schema compatibility with legacy
  - [ ] Document any schema changes needed (if any)
  - [ ] Test migration on production data backup (if schema changes required)
  - [ ] Ensure backward compatibility with existing data
- [ ] Add health check methods
- [ ] Add connection metrics (pool size, active connections)
- [ ] Add unit tests with in-memory SQLite
- [ ] Integration test: open/close pool lifecycle

**Acceptance Criteria:**
- [ ] Pool initializes with correct parameters
- [ ] Connections are encrypted with SqlCipher
- [ ] Health check detects connection failures
- [ ] Metrics track pool usage
- [ ] `cargo test -p pulsearc-infra database::manager` passes

---

### Task 3A.5: Activity Repository (Day 4)

**Source:** `legacy/api/src/db/activity/snapshots.rs` → `crates/infra/src/database/activity_repository.rs`

**Line Count:** 653 LOC (verified)

**Scope:**
- Implement `ActivityRepository` trait from Phase 1
- CRUD operations for `ActivitySnapshot`
- Time-range queries
- Pagination support

**Implementation Checklist:**
- [ ] Create `crates/infra/src/database/activity_repository.rs`
- [ ] Implement `ActivityRepository` trait
- [ ] Port `save()` method
- [ ] Port `find_by_time_range()` method
- [ ] Port `find_snapshots_by_date()` method
- [ ] Port `count_snapshots_by_date()` method
- [ ] Add pagination support (limit/offset)
- [ ] Convert sync code to async (use `SqlCipherConnection::get_connection().await`)
- [ ] Add unit tests with mock database
- [ ] Integration tests with real SqlCipher database

**Acceptance Criteria:**
- [ ] Saves snapshots with all fields
- [ ] Time-range queries return correct results
- [ ] Pagination works correctly
- [ ] All async operations properly await
- [ ] `cargo test -p pulsearc-infra database::activity_repository` passes

---

### Task 3A.6: Segment Repository (Day 4)

**Source:** `legacy/api/src/db/activity/segments.rs` → `crates/infra/src/database/segment_repository.rs`

**Line Count:** 374 LOC (verified)

**Scope:**
- Implement `SegmentRepository` trait from Phase 1
- CRUD operations for `ActivitySegment`
- Date-based queries

**Implementation Checklist:**
- [ ] Create `crates/infra/src/database/segment_repository.rs`
- [ ] Implement `SegmentRepository` trait
- [ ] Port `save_segment()` method
- [ ] Port `find_segments_by_date()` method
- [ ] Port `find_unprocessed_segments()` method
- [ ] Port `mark_processed()` method
- [ ] Convert sync code to async
- [ ] Add unit tests
- [ ] Integration tests with real database

**Acceptance Criteria:**
- [ ] Saves segments with correct timestamps
- [ ] Date queries handle timezone boundaries
- [ ] Marking processed updates database
- [ ] `cargo test -p pulsearc-infra database::segment_repository` passes

---

### Task 3A.7: Block Repository (Day 5)

**Source:** `legacy/api/src/db/blocks/operations.rs` → `crates/infra/src/database/block_repository.rs`

**Line Count:** 551 LOC (verified)

**Scope:**
- Implement `BlockRepository` trait from Phase 1
- CRUD operations for `ProposedBlock`
- Block approval/rejection workflow
- Block history queries

**Implementation Checklist:**
- [x] Create `crates/infra/src/database/block_repository.rs`
- [x] Implement `BlockRepository` trait
- [x] Port `save_proposed_block()` method
- [x] Port `get_proposed_blocks()` method
- [x] Port `approve_block()` method
- [x] Port `reject_block()` method
- [x] Port `get_block_history()` method
- [x] Convert sync code to async
- [x] Add unit tests
- [x] Integration tests with workflow scenarios

**Acceptance Criteria:**
- [ ] Saves blocks with all context signals
- [ ] Approval/rejection updates status correctly
- [ ] History queries return chronological results
- [ ] `cargo test -p pulsearc-infra database::block_repository` passes

---

### Task 3A.8: Outbox Repository (Day 5)

**Source:** `legacy/api/src/db/outbox/outbox.rs` → `crates/infra/src/database/outbox_repository.rs`

**Line Count:** ~550 LOC (estimate)

**Scope:**
- Implement `OutboxQueue` trait from Phase 1
- CRUD operations for `TimeEntryOutbox`
- Queue operations (enqueue, dequeue_batch)
- Status tracking (pending, sent, failed)

**Implementation Checklist:**
- [ ] Create `crates/infra/src/database/outbox_repository.rs`
- [ ] Implement `OutboxQueue` trait
- [ ] Port `enqueue()` method
- [ ] Port `dequeue_batch()` method
- [ ] Port `mark_sent()` method
- [ ] Port `mark_failed()` method
- [ ] Port `get_pending_count()` method
- [ ] Add retry count tracking
- [ ] Convert sync code to async
- [ ] Add unit tests
- [ ] Integration tests with queue workflow

**Acceptance Criteria:**
- [ ] Enqueues entries with correct timestamps
- [ ] Dequeue returns FIFO order
- [ ] Status updates persist correctly
- [ ] Failed entries track retry count
- [ ] `cargo test -p pulsearc-infra database::outbox_repository` passes

---

### Task 3A.9: Additional Database Repositories (Day 6-7)

**Remaining repositories:**

1. **ID Mapping Repository** (~200 LOC)
   - `legacy/api/src/db/outbox/id_mappings.rs` → `crates/infra/src/database/id_mapping_repository.rs`
   - Local ID → Remote ID mappings

2. **Batch Repository** (~300 LOC)
   - `legacy/api/src/db/batch/operations.rs` → `crates/infra/src/database/batch_repository.rs`
   - Batch queue operations

3. **DLQ Repository** (~250 LOC)
   - `legacy/api/src/db/batch/dlq.rs` → `crates/infra/src/database/dlq_repository.rs`
   - Dead letter queue for failed entries

4. **Calendar Repository** (~400 LOC) - **Feature: `calendar`**
   - `legacy/api/src/db/calendar/events.rs` → `crates/infra/src/database/calendar_repository.rs`
   - Implement `CalendarEventRepository` trait
   - OAuth token storage
   - Sync settings persistence

5. **Token Usage Repository** (~150 LOC)
   - `legacy/api/src/db/outbox/token_usage.rs` → `crates/infra/src/database/token_usage_repository.rs`
   - API token usage tracking

**Implementation Checklist (Each Repository):**
- [ ] Create file in `crates/infra/src/database/`
- [ ] Implement relevant trait (if defined in Phase 1)
- [ ] Port all CRUD operations
- [ ] Convert sync to async
- [ ] Add unit tests
- [ ] Integration tests with real database

---

### Phase 3A Validation

**Acceptance Criteria (Overall):**
- [ ] All database repositories implemented
- [ ] All repositories use `SqlCipherConnection` properly
- [ ] HTTP client works with retry/timeout
- [ ] Config loader reads from env and files
- [ ] Error conversions preserve context
- [ ] All tests pass: `cargo test -p pulsearc-infra --lib`
- [ ] No clippy warnings: `cargo clippy -p pulsearc-infra`
- [ ] Integration tests pass with real SqlCipher database

**Performance Targets:**
- Database operations: < 50ms p99 (in-memory test database)
- HTTP client: respects configured timeout
- Connection pool: stable under concurrent load (10+ threads)

**Blockers for Phase 3B:**
- None - 3B can start as soon as 3A is complete

---

## Phase 3B: Platform Adapters (Week 4)

**Goal:** Implement platform-specific activity providers
**Duration:** 4-6 days
**Dependencies:** Phase 3A complete
**Priority:** P1 (required for core functionality)

### Task 3B.1: macOS Activity Provider (Day 1-2)

**Source:** `legacy/api/src/tracker/providers/macos.rs` → `crates/infra/src/platform/macos/activity_provider.rs`

**Line Count:** 943 LOC

**Scope:**
- Implement `ActivityProvider` trait
- Accessibility API integration
- App/window info fetching
- Recent apps list (NSWorkspace)

**Implementation Checklist:**
- [ ] Create `crates/infra/src/platform/macos/activity_provider.rs`
- [ ] Port `MacOsActivityProvider` struct
- [ ] Implement `get_activity()` method (async)
- [ ] Port Accessibility API calls
- [ ] Port NSWorkspace integration
- [ ] Add permission checking logic
- [ ] Convert sync code to async
- [ ] Add unit tests with mocked Accessibility API
- [ ] Manual testing on macOS (requires permissions)

**Acceptance Criteria:**
- [ ] Fetches foreground app name
- [ ] Fetches window title
- [ ] Checks for Accessibility permissions
- [ ] Returns placeholder if permission denied
- [ ] `cargo test -p pulsearc-infra platform::macos` passes (with mocks)
- [ ] Manual test: captures real activity on macOS

---

### Task 3B.2: macOS Enrichment System (Day 2-3)

**Source:** Embedded in `legacy/api/src/tracker/providers/macos.rs`

**Line Count:** ~400 LOC (estimate from 943 total)

**Scope:**
- Browser URL extraction (Chrome, Safari, Firefox, Arc, Edge, Brave)
- Office document metadata (Excel, Word, PowerPoint)
- PDF document names (Acrobat, Preview, PDF Expert)
- Enrichment caching (750ms TTL)
- Background enrichment worker

**Implementation Checklist:**
- [ ] Create `crates/infra/src/platform/macos/enrichment.rs`
- [ ] Port browser URL extraction logic
- [ ] Port Office document metadata extraction
- [ ] Port PDF document name extraction
- [ ] Port enrichment cache (use `moka::future::Cache`)
- [ ] Port background worker (use `tokio::spawn`)
- [ ] Add timeout handling (750ms)
- [ ] Add unit tests for each enricher
- [ ] Manual testing with real apps

**Acceptance Criteria:**
- [ ] Extracts URLs from major browsers
- [ ] Extracts document names from Office apps
- [ ] Cache hit/miss works correctly
- [ ] Enrichment timeout prevents blocking
- [ ] Background worker processes jobs
- [ ] `cargo test -p pulsearc-infra platform::macos::enrichment` passes

---

### Task 3B.3: macOS Event Monitoring (Day 3-4)

**Source:** `legacy/api/src/tracker/os_events/macos.rs` → `crates/infra/src/platform/macos/event_monitor.rs`

**Line Count:** 400 LOC

**Scope:**
- NSWorkspace app activation notifications
- Event listener lifecycle
- Callback-based event handling

**Implementation Checklist:**
- [ ] Create `crates/infra/src/platform/macos/event_monitor.rs`
- [ ] Port `MacOsEventListener` struct
- [ ] Implement `OsEventListener` trait
- [ ] Port NSWorkspace observer setup
- [ ] Port notification handling
- [ ] Add cleanup logic (Drop trait)
- [ ] Add unit tests with mock observers
- [ ] Integration test: start/stop lifecycle

**Acceptance Criteria:**
- [ ] Registers NSWorkspace notifications
- [ ] Invokes callback on app activation
- [ ] Cleanup removes observer
- [ ] No memory leaks (verify with Instruments)
- [ ] `cargo test -p pulsearc-infra platform::macos::event_monitor` passes

---

### Task 3B.4: macOS Accessibility Helpers (Day 4)

**Source:** `legacy/api/src/tracker/os_events/macos_ax.rs` → `crates/infra/src/platform/macos/accessibility.rs`

**Line Count:** 372 LOC

**Scope:**
- Accessibility API wrapper functions
- Permission checking
- Element attribute fetching

**Implementation Checklist:**
- [ ] Create `crates/infra/src/platform/macos/accessibility.rs`
- [ ] Port `check_accessibility_permissions()` function
- [ ] Port `get_focused_window()` function
- [ ] Port `get_window_title()` function
- [ ] Port `get_document_name()` function
- [ ] Port `get_url()` function (for browsers)
- [ ] Add error handling
- [ ] Add unit tests (where possible without real AX API)

**Acceptance Criteria:**
- [ ] Permission check works correctly
- [ ] Window title fetching works
- [ ] Document name fetching works
- [ ] URL fetching works for browsers
- [ ] `cargo test -p pulsearc-infra platform::macos::accessibility` passes

---

### Task 3B.5: Platform Enrichers (Day 5)

**Source:** `legacy/api/src/detection/enrichers/` → `crates/infra/src/platform/enrichers/`

**Line Count:** ~500 LOC (estimate)

**Modules:**
1. `browser.rs` - Browser-specific URL extraction logic
2. `office.rs` - Office document metadata extraction

**Implementation Checklist:**
- [ ] Create `crates/infra/src/platform/enrichers/browser.rs`
- [ ] Create `crates/infra/src/platform/enrichers/office.rs`
- [ ] Port browser enrichment logic (Chrome, Safari, Firefox, Arc, Edge, Brave)
- [ ] Port Office enrichment logic (Excel, Word, PowerPoint)
- [ ] Add support for new browsers (if needed)
- [ ] Add unit tests for each browser
- [ ] Manual testing with real browsers

**Acceptance Criteria:**
- [ ] Extracts URLs from all supported browsers
- [ ] Handles missing AX elements gracefully
- [ ] Office metadata includes full document path
- [ ] `cargo test -p pulsearc-infra platform::enrichers` passes

---

### Task 3B.6: Fallback Provider (Day 6) - 🔮 FUTURE WORK

**Source:** `legacy/api/src/tracker/providers/dummy.rs` → `crates/infra/src/platform/dummy/activity_provider.rs`

**Line Count:** 168 LOC

**⚠️ NOTE: This task is DEFERRED to future work. Phase 3 focuses on macOS-only implementation.**

**Future Scope (Post-Phase 3):**
- Windows basic activity tracking (Win32 API)
- Linux placeholder implementation
- Fallback for unsupported platforms

**Phase 3 Action:**
- [ ] Add minimal stub for compilation on non-macOS platforms:
```rust
#[cfg(not(target_os = "macos"))]
pub struct DummyActivityProvider;

#[cfg(not(target_os = "macos"))]
impl ActivityProvider for DummyActivityProvider {
    async fn get_activity(&self) -> Result<ActivityContext> {
        Err(PulseArcError::UnsupportedPlatform(
            "PulseArc currently supports macOS only".to_string()
        ))
    }
}
```

**Acceptance Criteria (Phase 3):**
- [ ] Code compiles on Windows/Linux (with stub)
- [ ] Returns clear error message on unsupported platforms
- [ ] macOS implementation is not affected

---

### Phase 3B Validation

**Acceptance Criteria (Overall):**
- [ ] macOS provider captures real activity data
- [ ] Browser URL enrichment works for major browsers
- [ ] Office document enrichment works
- [ ] Event monitoring triggers callbacks
- [ ] Code compiles on non-macOS platforms (with stub returning error)
- [ ] All tests pass: `cargo test -p pulsearc-infra --features platform` (on macOS)
- [ ] Manual testing complete on macOS
- [ ] No memory leaks (verify with Instruments on macOS)

**Platform Support (Phase 3):**
- ✅ **macOS** - Full functionality required
- ⚠️ **Windows/Linux** - Compile-only (stub implementation, future work)

**Performance Targets:**
- Activity fetch: < 15ms p50 (macOS, no enrichment)
- Enrichment: < 100ms p50 (browser URL)
- Event latency: < 50ms (app switch → callback invoked)

**Blockers for Phase 3C:**
- None - 3C can start in parallel with 3B (only depends on 3A)

---

## Phase 3C: Integration Adapters (Week 5)

**Goal:** Implement external service integrations
**Duration:** 5-7 days
**Dependencies:** Phase 3A complete
**Priority:** P2 (feature-gated, optional for core functionality)

### Task 3C.1: OpenAI Adapter (Day 1-2)

**Source:** `legacy/api/src/inference/openai_types.rs` → `crates/infra/src/integrations/openai/`

**Line Count:** ~400 LOC (estimate)

**Scope:**
- OpenAI API client
- Request/response types
- Implement `Classifier` trait
- Map OpenAI responses → domain types

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/openai/types.rs`
- [ ] Create `crates/infra/src/integrations/openai/client.rs`
- [ ] Port OpenAI request/response types
- [ ] Implement `Classifier` trait
- [ ] Add OpenAI API authentication
- [ ] Add request retry logic
- [ ] Add response parsing and validation
- [ ] Map `BlockClassificationResponse` → `Vec<ProposedBlock>`
- [ ] Add unit tests with mocked API responses
- [ ] Optional: integration test with real OpenAI API (use test API key)

**Acceptance Criteria:**
- [ ] Sends valid requests to OpenAI API
- [ ] Parses responses correctly
- [ ] Maps to domain types without data loss
- [ ] Handles API errors gracefully
- [ ] `cargo test -p pulsearc-infra integrations::openai` passes

---

### Task 3C.2: SAP Client (Day 2-3) - Feature: `sap`

**Source:** `legacy/api/src/integrations/sap/client.rs` → `crates/infra/src/integrations/sap/client.rs`

**Line Count:** ~600 LOC (estimate)

**Scope:**
- SAP API client (HTTP-based)
- Implement `SapClient` trait
- Authentication (OAuth)
- WBS code validation

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/sap/client.rs`
- [ ] Port `SapClient` struct
- [ ] Implement `SapClient` trait from Phase 1
- [ ] Port `forward_entry()` method
- [ ] Port `validate_wbs()` method
- [ ] Add OAuth authentication flow
- [ ] Add request retry logic
- [ ] Add unit tests with mocked SAP API
- [ ] Optional: integration test with test SAP instance

**Acceptance Criteria:**
- [ ] Authenticates with SAP API
- [ ] Forwards time entries successfully
- [ ] Validates WBS codes using `WbsRepository` from Phase 2
- [ ] Handles API errors gracefully
- [ ] `cargo test -p pulsearc-infra --features sap integrations::sap::client` passes

---

### Task 3C.3: SAP Cache & Validation (Day 3)

**Source:**
- `legacy/api/src/integrations/sap/cache.rs` → `crates/infra/src/integrations/sap/cache.rs`
- `legacy/api/src/integrations/sap/validation.rs` → `crates/infra/src/integrations/sap/validation.rs`

**Line Count:** ~400 LOC total

**Scope:**
- WBS code caching
- WBS code validation logic
- Uses `WbsRepository` from Phase 2 PR #1

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/sap/cache.rs`
- [ ] Create `crates/infra/src/integrations/sap/validation.rs`
- [ ] Port `WbsCache` struct (uses `WbsRepository`)
- [ ] Port WBS validation functions
- [ ] Port FTS5 search integration (via `WbsRepository`)
- [ ] Add cache TTL configuration
- [ ] Add unit tests
- [ ] Integration test: validate WBS codes with real repository

**Acceptance Criteria:**
- [ ] Cache reduces database queries
- [ ] Validation uses FTS5 search from `WbsRepository`
- [ ] Cache invalidation works correctly
- [ ] `cargo test -p pulsearc-infra --features sap integrations::sap::cache` passes

---

### Task 3C.4: SAP Supporting Modules (Day 4)

**Source:** Multiple SAP modules

**Modules:**
1. **SAP Auth** (`integrations/sap/auth_commands.rs` → `auth.rs`) - ~200 LOC
2. **SAP Errors** (`integrations/sap/errors.rs` → `errors.rs`) - ~150 LOC
3. **SAP Forwarder** (`integrations/sap/forwarder.rs` → `forwarder.rs`) - ~300 LOC
4. **SAP Health** (`integrations/sap/health_monitor.rs` → `health.rs`) - ~250 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/sap/auth.rs`
- [ ] Create `crates/infra/src/integrations/sap/errors.rs`
- [ ] Create `crates/infra/src/integrations/sap/forwarder.rs`
- [ ] Create `crates/infra/src/integrations/sap/health.rs`
- [ ] Port all SAP supporting logic
- [ ] Add unit tests for each module
- [ ] Integration test: full SAP workflow (auth → validate → forward)

**Acceptance Criteria:**
- [ ] SAP authentication completes successfully
- [ ] Error types provide actionable messages
- [ ] Forwarder batches entries correctly
- [ ] Health monitor detects API failures
- [ ] `cargo test -p pulsearc-infra --features sap integrations::sap` passes

---

### Task 3C.5: Calendar Client (Day 4-5) - Feature: `calendar`

**Source:** `legacy/api/src/integrations/calendar/client.rs` → `crates/infra/src/integrations/calendar/client.rs`

**Line Count:** ~500 LOC (estimate)

**Scope:**
- Calendar API client (Google, Microsoft)
- Implement `CalendarProvider` trait
- OAuth authentication
- Event fetching

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/calendar/client.rs`
- [ ] Port `CalendarClient` struct
- [ ] Implement `CalendarProvider` trait from Phase 1
- [ ] Port `fetch_events()` method
- [ ] Port `sync()` method
- [ ] Add OAuth authentication flow
- [ ] Add request retry logic
- [ ] Add unit tests with mocked calendar API
- [ ] Optional: integration test with test Google/Microsoft account

**Acceptance Criteria:**
- [ ] Authenticates with calendar API
- [ ] Fetches events for date range
- [ ] Syncs events to local database
- [ ] Handles API errors gracefully
- [ ] `cargo test -p pulsearc-infra --features calendar integrations::calendar::client` passes

---

### Task 3C.6: Calendar OAuth (Day 5)

**Source:** `legacy/api/src/integrations/calendar/oauth.rs` → `crates/infra/src/integrations/calendar/oauth.rs`

**Line Count:** ~400 LOC (estimate)

**Scope:**
- OAuth 2.0 flow implementation
- Token storage (keychain)
- Token refresh logic
- Authorization URL generation

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/calendar/oauth.rs`
- [ ] Port OAuth flow implementation
- [ ] Port token storage (use `pulsearc_common::security::KeychainProvider`)
- [ ] Port token refresh logic
- [ ] Add authorization URL generation
- [ ] Add unit tests for OAuth flow
- [ ] Integration test: complete OAuth flow (requires manual intervention)

**Acceptance Criteria:**
- [ ] Generates valid authorization URLs
- [ ] Exchanges auth code for tokens
- [ ] Stores tokens securely in keychain
- [ ] Refreshes expired tokens automatically
- [ ] `cargo test -p pulsearc-infra --features calendar integrations::calendar::oauth` passes

---

### Task 3C.7: Calendar Providers (Day 6)

**Source:** `legacy/api/src/integrations/calendar/providers/` → `crates/infra/src/integrations/calendar/providers/`

**Line Count:** ~800 LOC (estimate, multiple files)

**Scope:**
- Google Calendar provider
- Microsoft Calendar provider
- Provider-specific API differences

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/calendar/providers/google.rs`
- [ ] Create `crates/infra/src/integrations/calendar/providers/microsoft.rs`
- [ ] Port Google Calendar provider
- [ ] Port Microsoft Calendar provider
- [ ] Implement `CalendarProvider` trait for each
- [ ] Add provider selection logic
- [ ] Add unit tests for each provider
- [ ] Integration tests with test accounts (optional)

**Acceptance Criteria:**
- [ ] Google provider fetches events correctly
- [ ] Microsoft provider fetches events correctly
- [ ] Provider-specific fields mapped correctly
- [ ] `cargo test -p pulsearc-infra --features calendar integrations::calendar::providers` passes

---

### Task 3C.8: Calendar Supporting Modules (Day 7)

**Source:** Multiple calendar modules

**Modules:**
1. **Calendar Sync** (`integrations/calendar/sync.rs`) - ~400 LOC
2. **Calendar Parser** (`integrations/calendar/parser.rs`) - ~300 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/calendar/sync.rs`
- [ ] Create `crates/infra/src/integrations/calendar/parser.rs`
- [ ] Port calendar sync logic
- [ ] Port iCalendar parser (if applicable)
- [ ] Add unit tests
- [ ] Integration test: full sync workflow

**Acceptance Criteria:**
- [ ] Sync fetches and stores events
- [ ] Parser handles iCalendar format
- [ ] Incremental sync works correctly
- [ ] `cargo test -p pulsearc-infra --features calendar integrations::calendar` passes

---

### Phase 3C Validation

**Acceptance Criteria (Overall):**
- [ ] OpenAI adapter classifies activities
- [ ] SAP client forwards time entries
- [ ] Calendar client syncs events
- [ ] All OAuth flows work correctly
- [ ] All tests pass with features: `cargo test -p pulsearc-infra --features sap,calendar`
- [ ] Feature flags isolate each integration
- [ ] No compilation errors without features

**Performance Targets:**
- OpenAI API: < 2s p99 (network dependent)
- SAP API: < 1s p99 (network dependent)
- Calendar API: < 3s p99 for event sync (network dependent)

**Blockers for Phase 3D:**
- Phase 3C must complete for scheduler integration

---

## Phase 3D: Schedulers & Workers (Week 6)

**Goal:** Implement background job scheduling
**Duration:** 4-5 days
**Dependencies:** Phase 3A, 3C complete
**Priority:** P2 (required for automated workflows)

### Task 3D.1: Block Scheduler (Day 1)

**Source:** `legacy/api/src/inference/scheduler.rs` → `crates/infra/src/scheduling/block_scheduler.rs`

**Line Count:** ~350 LOC (estimate)

**Scope:**
- Cron-based block generation scheduling
- Tokio task management
- Error handling and retry

**Implementation Checklist:**
- [ ] Create `crates/infra/src/scheduling/block_scheduler.rs`
- [ ] Port `BlockScheduler` struct
- [ ] Add cron expression parsing
- [ ] Add tokio task spawning
- [ ] Add error handling and retry
- [ ] Add unit tests with mock cron
- [ ] Integration test: schedule and execute job

**Acceptance Criteria:**
- [ ] Schedules jobs based on cron expressions
- [ ] Executes jobs at correct times
- [ ] Retries failed jobs
- [ ] `cargo test -p pulsearc-infra scheduling::block_scheduler` passes

---

### Task 3D.2: Classification Scheduler (Day 1)

**Source:** `legacy/api/src/inference/classification_scheduler.rs` → `crates/infra/src/scheduling/classification_scheduler.rs`

**Line Count:** ~300 LOC (estimate)

**Scope:**
- Periodic classification job scheduling
- Batch processing coordination

**Implementation Checklist:**
- [ ] Create `crates/infra/src/scheduling/classification_scheduler.rs`
- [ ] Port `ClassificationScheduler` struct
- [ ] Add scheduling logic
- [ ] Add batch coordination
- [ ] Add unit tests
- [ ] Integration test: scheduled classification

**Acceptance Criteria:**
- [ ] Schedules classification jobs
- [ ] Coordinates batch processing
- [ ] `cargo test -p pulsearc-infra scheduling::classification_scheduler` passes

---

### Task 3D.3: Integration Schedulers (Day 2) - Feature-gated

**Source:** Multiple scheduler modules

**Modules:**
1. **SAP Scheduler** (`integrations/sap/scheduler.rs`) - ~250 LOC - Feature: `sap`
2. **Calendar Scheduler** (`integrations/calendar/scheduler.rs`) - ~200 LOC - Feature: `calendar`
3. **Sync Scheduler** (`sync/scheduler.rs`) - ~300 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/integrations/sap/scheduler.rs` (feature-gated)
- [ ] Create `crates/infra/src/integrations/calendar/scheduler.rs` (feature-gated)
- [ ] Create `crates/infra/src/sync/scheduler.rs`
- [ ] Port all scheduler logic
- [ ] Add unit tests for each
- [ ] Integration tests with scheduled jobs

**Acceptance Criteria:**
- [ ] SAP scheduler syncs WBS codes periodically
- [ ] Calendar scheduler syncs events periodically
- [ ] Sync scheduler processes outbox queue
- [ ] All tests pass with features enabled

---

### Task 3D.4: Outbox Worker (Day 3)

**Source:** `legacy/api/src/sync/outbox_worker.rs` → `crates/infra/src/sync/outbox_worker.rs`

**Line Count:** ~500 LOC (estimate)

**Scope:**
- Background outbox processing
- Batch dequeuing and forwarding
- Retry logic for failed entries

**Implementation Checklist:**
- [ ] Create `crates/infra/src/sync/outbox_worker.rs`
- [ ] Port `OutboxWorker` struct
- [ ] Add batch processing logic
- [ ] Add retry logic (use `pulsearc_common::resilience::retry`)
- [ ] Add error handling and DLQ routing
- [ ] Add unit tests with mock outbox
- [ ] Integration test: process entries end-to-end

**Acceptance Criteria:**
- [ ] Dequeues entries in batches
- [ ] Forwards entries to API
- [ ] Retries transient failures
- [ ] Routes permanent failures to DLQ
- [ ] `cargo test -p pulsearc-infra sync::outbox_worker` passes

---

### Task 3D.5: Sync Supporting Modules (Day 4)

**Source:** Multiple sync modules

**Modules:**
1. **Neon Client** (`sync/neon_client.rs`) - ~400 LOC
2. **Cost Tracker** (`sync/cost_tracker.rs`) - ~200 LOC
3. **Cleanup** (`sync/cleanup.rs`) - ~300 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/sync/neon_client.rs`
- [ ] Create `crates/infra/src/sync/cost_tracker.rs`
- [ ] Create `crates/infra/src/sync/cleanup.rs`
- [ ] Port all sync supporting logic
- [ ] Add unit tests for each module
- [ ] Integration test: full sync workflow with cost tracking

**Acceptance Criteria:**
- [ ] Neon client syncs to remote database
- [ ] Cost tracker records API usage
- [ ] Cleanup removes old/stale data
- [ ] `cargo test -p pulsearc-infra sync` passes

---

### Task 3D.6: Domain API Client (Day 5)

**Source:** `legacy/api/src/domain/api/` → `crates/infra/src/api/`

**Line Count:** ~800 LOC (estimate, 5 files)

**Modules:**
1. **API Client** (`domain/api/client.rs`) - ~300 LOC
2. **API Auth** (`domain/api/auth.rs`) - ~150 LOC
3. **API Commands** (`domain/api/commands.rs`) - ~200 LOC
4. **API Forwarder** (`domain/api/forwarder.rs`) - ~100 LOC
5. **API Scheduler** (`domain/api/scheduler.rs`) - ~50 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/api/client.rs`
- [ ] Create `crates/infra/src/api/auth.rs`
- [ ] Create `crates/infra/src/api/commands.rs`
- [ ] Create `crates/infra/src/api/forwarder.rs`
- [ ] Create `crates/infra/src/api/scheduler.rs`
- [ ] Port all API client logic
- [ ] Add authentication handling
- [ ] Add unit tests with mocked API
- [ ] Integration test: authenticate and forward entry

**Acceptance Criteria:**
- [ ] API client authenticates successfully
- [ ] Commands send requests correctly
- [ ] Forwarder batches and sends entries
- [ ] Scheduler coordinates API sync
- [ ] `cargo test -p pulsearc-infra api` passes

---

### Phase 3D Validation

**Acceptance Criteria (Overall):**
- [ ] All schedulers run jobs at correct times
- [ ] Outbox worker processes entries
- [ ] Sync infrastructure forwards data to API
- [ ] Cost tracking records usage
- [ ] Cleanup removes old data
- [ ] All tests pass: `cargo test -p pulsearc-infra --features sap,calendar`
- [ ] Integration tests pass end-to-end

**Performance Targets:**
- Outbox processing: > 100 entries/second
- Scheduled jobs: start within ±1 second of scheduled time
- API forwarding: < 500ms p99 (network dependent)

**Blockers for Phase 3E:**
- None - 3E can start in parallel with 3D

---

## Phase 3E: ML Adapters (Optional - Week 7)

**Goal:** Implement machine learning adapters
**Duration:** 3-4 days
**Dependencies:** Phase 3A complete
**Priority:** P3 (optional, feature-gated)
**Feature:** `ml`

### Task 3E.1: Linfa Classifier (Day 1)

**Source:** `legacy/api/src/inference/linfa_integration.rs` → `crates/infra/src/ml/linfa_classifier.rs`

**Line Count:** ~400 LOC (estimate)

**Scope:**
- Linfa-based ML classifier
- Implement `Classifier` trait
- Model training and inference

**Implementation Checklist:**
- [ ] Create `crates/infra/src/ml/linfa_classifier.rs`
- [ ] Port `LinfaClassifier` struct
- [ ] Implement `Classifier` trait
- [ ] Add model loading logic
- [ ] Add inference logic
- [ ] Add unit tests with test model
- [ ] Integration test: classify activities

**Acceptance Criteria:**
- [ ] Loads trained models
- [ ] Classifies activities correctly
- [ ] Returns confidence scores
- [ ] `cargo test -p pulsearc-infra --features ml ml::linfa_classifier` passes

---

### Task 3E.2: Additional ML Classifiers (Day 2)

**Source:** Multiple ML classifier modules

**Modules:**
1. **Tree Classifier** (`inference/tree_classifier.rs`) - ~350 LOC - Feature: `tree-classifier`
2. **Logistic Classifier** (`inference/logistic_classifier.rs`) - ~300 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/ml/tree_classifier.rs`
- [ ] Create `crates/infra/src/ml/logistic_classifier.rs`
- [ ] Port all classifier logic
- [ ] Implement `Classifier` trait for each
- [ ] Add unit tests with test models
- [ ] Integration tests

**Acceptance Criteria:**
- [ ] Tree classifier works with decision trees
- [ ] Logistic classifier works with logistic regression
- [ ] All classifiers return consistent results
- [ ] `cargo test -p pulsearc-infra --features ml` passes

---

### Task 3E.3: Training Pipeline (Day 3)

**Source:** Multiple training modules

**Modules:**
1. **Training Pipeline** (`inference/training_pipeline.rs`) - ~500 LOC
2. **Training Exporter** (`inference/training_data_exporter.rs`) - ~300 LOC
3. **ML Metrics** (`inference/metrics.rs`) - ~200 LOC

**Implementation Checklist:**
- [ ] Create `crates/infra/src/ml/training_pipeline.rs`
- [ ] Create `crates/infra/src/ml/training_exporter.rs`
- [ ] Create `crates/infra/src/ml/metrics.rs`
- [ ] Port all training logic
- [ ] Add unit tests
- [ ] Integration test: train model from data

**Acceptance Criteria:**
- [ ] Pipeline trains models from data
- [ ] Exporter generates training datasets
- [ ] Metrics calculate accuracy/precision/recall
- [ ] `cargo test -p pulsearc-infra --features ml` passes

---

### Task 3E.4: Batch Classifier (Day 4)

**Source:** `legacy/api/src/inference/batch_classifier.rs` → `crates/infra/src/classification/batch_classifier.rs`

**Line Count:** ~400 LOC (estimate)

**Scope:**
- Background batch classification
- Uses `DbManager` and `tauri::Emitter`
- Progress reporting

**Implementation Checklist:**
- [ ] Create `crates/infra/src/classification/batch_classifier.rs`
- [ ] Port `BatchClassifier` struct
- [ ] Add batch processing logic
- [ ] Add progress tracking
- [ ] Add unit tests
- [ ] Integration test: classify batch of activities

**Acceptance Criteria:**
- [ ] Processes activities in batches
- [ ] Reports progress to UI
- [ ] Saves classified results
- [ ] `cargo test -p pulsearc-infra --features ml classification::batch_classifier` passes

---

### Phase 3E Validation

**Acceptance Criteria (Overall):**
- [ ] All ML classifiers load and infer
- [ ] Training pipeline generates models
- [ ] Batch classifier processes activities
- [ ] All tests pass: `cargo test -p pulsearc-infra --features ml`
- [ ] Models produce reasonable accuracy (> 70% on test set)

**Performance Targets:**
- Inference: < 50ms per activity
- Training: completes in < 5 minutes for typical dataset
- Batch processing: > 50 activities/second

---

## Phase 3F: Observability (Parallel - Week 7)

**Goal:** Implement metrics collection infrastructure
**Duration:** 2-3 days
**Dependencies:** None (can run in parallel)
**Priority:** P3 (nice-to-have)
**Status:** ✅ COMPLETE (Day 1-2 finished - November 2, 2025)

### Task 3F.1: Metrics Collection (Day 1-2) ✅

**Source:** `legacy/api/src/observability/metrics/` → `crates/infra/src/observability/metrics/`

**Line Count:** ~1,217 LOC actual (revised from 600 LOC estimate)
- Core metrics: ~444 LOC (CallMetrics, CacheMetrics, FetchMetrics)
- DbMetrics: ~417 LOC
- Datadog exporter: ~250 LOC
- ObserverMetrics: ~200 LOC (macOS Accessibility API)
- PerformanceMetrics aggregator: ~350 LOC
- Tests: ~292 LOC

**Scope:**
- ✅ Core metrics collection (CallMetrics, CacheMetrics, FetchMetrics)
- ✅ Database metrics (DbMetrics)
- ✅ Datadog DogStatsD integration
- ✅ PerformanceMetrics aggregator
- ✅ ObserverMetrics macOS
- ⏸️ MetricsRegistry with LRU cardinality (deferred - not critical for MVP)

**Implementation Checklist:**
- [x] Create `crates/infra/src/observability/mod.rs` - MetricsError enum (3 variants)
- [x] Create `crates/infra/src/observability/metrics/mod.rs` - Module structure
- [x] Create `crates/infra/src/observability/exporters/mod.rs` - Exporter infrastructure
- [x] Port **CallMetrics** (208 LOC) - VecDeque ring buffer, poison-safe locking
- [x] Port **CacheMetrics** (85 LOC) - Hit/miss tracking, SeqCst ordering
- [x] Port **FetchMetrics** (151 LOC) - Fetch timing, errors, timeouts
- [x] Port **DbMetrics** (417 LOC) - Database connection pool metrics with CAS first-connection timestamp
- [x] Port **ObserverMetrics** (200 LOC) - macOS Accessibility API observer tracking
- [x] Implement **Datadog DogStatsD exporter** (250 LOC) - Raw UDP socket, f64 precision, no deps
- [x] Implement **PerformanceMetrics aggregator** (350 LOC) - Hierarchical metrics organization
- [x] Add unit tests - **66 tests passing** across all metrics types
- [x] Poison recovery tests - All metrics handle poison with explicit match pattern (no .expect())
- [x] Empty data handling - Percentile/average methods return Result or 0.0 on empty
- [x] Ring buffer eviction - VecDeque with O(1) push_back/pop_front
- [x] Memory ordering - SeqCst for derived metrics, Relaxed for independent counters
- [x] CAS-based first connection - Race-condition free timestamp recording

**Completed Commits:**
- ✅ **Commit `f6e3ec8`** (Oct 31, 2025) - Observability foundation + CallMetrics
- ✅ **Commit `d9392c8`** (Oct 31, 2025) - CacheMetrics + FetchMetrics
- ✅ **Commit `ff986ad`** (Nov 2, 2025) - DbMetrics + Datadog DogStatsD exporter (Day 1-2 Part 1)
- ✅ **Commit `b15b14c`** (Nov 2, 2025) - ObserverMetrics + PerformanceMetrics (Day 2 Part 2)

**Final Status:**
```
✅ CallMetrics          - 208 LOC (9 tests)
✅ CacheMetrics         - 85 LOC (8 tests)
✅ FetchMetrics         - 151 LOC (13 tests)
✅ DbMetrics            - 417 LOC (11 tests)
✅ ObserverMetrics      - 200 LOC (8 tests)
✅ PerformanceMetrics   - 350 LOC (7 tests)
✅ Datadog Exporter     - 250 LOC (13 tests)
---
Total: 1,217 LOC implemented
Tests: 66 passing (100% coverage of public APIs)
```

**Design Decisions:**
1. **No .expect()** - All mutex locks use explicit match pattern for poison recovery
2. **VecDeque ring buffer** - O(1) eviction vs Vec::remove(0) which is O(n)
3. **Corrected percentile formula** - Fixed off-by-one: `((len * percentile + 99) / 100).saturating_sub(1)`
4. **MetricsResult returns** - All record methods return Result for future extensibility
5. **SeqCst ordering** - For atomics used in derived metrics (rates, averages, percentiles)
6. **Relaxed ordering** - For independent counters with no derived calculations
7. **Datadog DogStatsD** - Raw UdpSocket (no cadence dependency), f64 precision preserved
8. **CAS atomics** - Race-free first connection timestamp using compare_exchange
9. **Aggregation pattern** - PerformanceMetrics composes all metric types with delegation methods
10. **Optional percentiles** - DbStats uses Option<u64> to handle empty data gracefully

**Acceptance Criteria:**
- [x] All metric types implemented (Call, Cache, Fetch, Db, Observer, Performance)
- [x] Thread-safe with poison recovery
- [x] Percentile calculations correct (P50/P95/P99)
- [x] Ring buffer evicts oldest at 1000 samples (O(1) eviction)
- [x] Datadog exporter sends metrics via UDP with f64 precision
- [x] PerformanceMetrics aggregates all metric types
- [x] All tests pass: `cargo test -p pulsearc-infra observability` - **66 passing**
- [x] No clippy warnings in observability module
- [x] Documentation complete with usage examples

---

### Phase 3F Validation

**Acceptance Criteria (Overall):**
- [x] Metrics collection works for all subsystems
- [x] Datadog DogStatsD exporter works (UDP fire-and-forget)
- [x] All tests pass: `cargo test -p pulsearc-infra observability` - **66 passing**
- [x] Zero-cost abstraction (no runtime overhead for disabled metrics)
- [x] Production-ready code quality (no unwrap/expect, full error handling)

---

## Overall Phase 3 Validation

### Comprehensive Testing

**Unit Tests:**
```bash
# All infra modules
cargo test -p pulsearc-infra --lib

# With all features
cargo test -p pulsearc-infra --all-features
```

**Integration Tests:**
```bash
# Database integration
cargo test -p pulsearc-infra --test database_integration

# Platform integration (macOS)
cargo test -p pulsearc-infra --test platform_integration --features platform

# SAP integration
cargo test -p pulsearc-infra --test sap_integration --features sap

# Calendar integration
cargo test -p pulsearc-infra --test calendar_integration --features calendar
```

**Manual Testing Checklist (macOS):**
- [ ] macOS: Activity tracking captures real app data
- [ ] macOS: Browser URL enrichment works
- [ ] macOS: Office document enrichment works
- [ ] macOS: Event monitoring triggers on app switch
- [ ] SAP: Authentication flow completes
- [ ] SAP: Time entry forwarding works
- [ ] Calendar: OAuth flow completes
- [ ] Calendar: Event sync works
- [ ] OpenAI: Classification works (with test API key)
- [ ] ML: Model training completes
- [ ] Schedulers: Jobs run at correct times
- [ ] Outbox: Entries process successfully

**Platform Testing:**
- [ ] macOS: Full functional testing (all above items)
- [ ] Windows/Linux: Compile-only verification (stub returns appropriate error)

### Feature Flag Matrix

Run the automated matrix before merging Phase 3 work:

| Features | Expected Result |
|----------|----------------|
| `[]` (default) | Database + core infra only |
| `calendar` | Calendar integration only |
| `sap` | SAP integration only |
| `tree-classifier` | Tree classifier only |
| `ml` | ML stack (pulls in tree-classifier) |
| `graphql` | GraphQL client only |
| `calendar,sap` | Both enterprise integrations |
| `sap,ml,graphql` | SAP + ML + GraphQL |
| `calendar,sap,ml` | Enterprise build without GraphQL |
| `calendar,sap,ml,graphql` | All features enabled |

**Local tooling:**
- `cargo xtask test-features` — compile + test matrix (identical to CI coverage)
- `./scripts/test-features.sh` — lightweight compile-only sweep

### Automated Feature Flag Testing (CI)

**Status:** ✅ Implemented (`infra-feature-matrix` job in `.github/workflows/ci.yml`)

- Every push/PR runs the 10 combinations above (check + test) on ubuntu.
- `cargo xtask test-features` mirrors the CI matrix for local verification prior to PRs.
- Phase 3 PR template includes a feature-flag checklist covering gating, matrix runs, and regression tests.

**Benefits:**
- Catches feature-flag regressions automatically when optional adapters are toggled.
- Ensures all combinations compile and test successfully across local and CI runs.
- Keeps Phase 3 PRs aligned with anti-pattern guardrails from pre-migration fixes.

### Performance Validation

**Database Operations:**
- [ ] Activity save: < 50ms p99
- [ ] Time-range query: < 100ms p99 (1 day range)
- [ ] Bulk insert: > 500 entries/second

**Platform Providers:**
- [ ] Activity fetch (macOS): < 15ms p50
- [ ] Enrichment (browser): < 100ms p50
- [ ] Event latency: < 50ms p50

**Integration Adapters:**
- [ ] HTTP requests: respect configured timeout
- [ ] Retry logic: exponential backoff works
- [ ] OAuth refresh: < 2s p99

**Schedulers & Workers:**
- [ ] Job scheduling: ±1 second accuracy
- [ ] Outbox processing: > 100 entries/second
- [ ] Background tasks: no blocking of main thread

### Code Quality Validation

```bash
# Formatting
cargo fmt --all -- --check

# Linting
cargo clippy -p pulsearc-infra --all-features -- -D warnings

# Documentation
cargo doc -p pulsearc-infra --all-features --no-deps

# Coverage
cargo tarpaulin -p pulsearc-infra --all-features --out Html
# Target: 70%+ line coverage
```

### Acceptance Criteria (Final)

**Functional:**
- [ ] All 50+ modules migrated
- [ ] All port traits implemented
- [ ] All tests pass (unit + integration)
- [ ] Manual testing complete on macOS
- [ ] Feature flags isolate optional components

**Non-Functional:**
- [ ] Performance meets targets
- [ ] No memory leaks (verified with Instruments)
- [ ] No data races (verified with ThreadSanitizer)
- [ ] Code coverage ≥ 70%
- [ ] No clippy warnings
- [ ] All public APIs documented

**Platform Support:**
- [ ] macOS: Full functionality (primary target)
- [ ] Windows/Linux: Compile-only (stub returns error, future enhancement)

**Integrations:**
- [ ] SAP: Full workflow (auth → validate → forward)
- [ ] Calendar: Full workflow (OAuth → sync)
- [ ] OpenAI: Classification works
- [ ] ML: Training and inference work

---

## Risk Assessment

### High-Risk Areas

#### 1. Database Migration (Phase 3A)
**Risk:** SqlCipher connection pool changes may break existing data access

**Mitigation:**
- Extensive integration tests with real database
- Test migration path with production data backup
- Gradual rollout with monitoring

#### 2. macOS Accessibility API (Phase 3B)
**Risk:** Permission handling may break, enrichment may timeout

**Mitigation:**
- Keep existing permission check logic
- Add extensive timeout handling (750ms)
- Manual testing on fresh macOS installation
- Fallback to basic tracking if permission denied

#### 3. OAuth Flows (Phase 3C)
**Risk:** Token refresh may fail, breaking SAP/Calendar sync

**Mitigation:**
- Implement robust token refresh with retry
- Store refresh tokens securely in keychain
- Add manual re-authentication flow
- Monitor token expiration proactively

#### 4. Background Workers (Phase 3D)
**Risk:** Worker tasks may leak, causing memory issues

**Mitigation:**
- Use structured concurrency (`tokio::JoinSet`)
- Add cancellation tokens for all tasks
- Verify cleanup with memory profiling
- Add health checks for workers

### Medium-Risk Areas

#### 5. Feature Flag Complexity
**Risk:** Wrong feature combinations may cause compile errors

**Mitigation:**
- Test all feature combinations in CI
- Document required feature combinations
- Use `#[cfg]` guards consistently

#### 6. Platform Abstraction
**Risk:** Windows/Linux implementations may diverge from macOS

**Mitigation:**
- Define clear trait contracts
- Share common logic where possible
- Test on each platform in CI

### Low-Risk Areas

#### 7. ML Adapters (Phase 3E)
**Risk:** Model loading may fail, inference may be slow

**Mitigation:**
- Feature-gate ML behind `ml` flag
- Add model validation on load
- Benchmark inference performance
- Provide fallback to rule-based classifier

---

## Rollback Plan

### Immediate Rollback (During Phase 3)

**If critical issues arise:**

1. **Identify scope** - Which sub-phase is broken?
2. **Revert commits** - Git revert specific sub-phase commits
3. **Disable feature** - Turn off feature flag if integration broken
4. **Restore legacy** - Temporarily use legacy code via re-exports
5. **Rollback window** - ≤ 2 hours to stable state

### Partial Rollback (After Phase 3)

**If issues found in production:**

1. **Isolate broken module** - Identify specific adapter
2. **Disable feature flag** - Turn off optional integration
3. **Hotfix** - Patch critical issues quickly
4. **Redeploy** - Push fixed version
5. **Extended timeline** - Give 1 week for thorough fixes

### Full Rollback (Unlikely)

**If Phase 3 fundamentally flawed:**

1. **Abandon new adapters** - Keep legacy implementations
2. **Archive work** - Branch preservation for future attempt
3. **Post-mortem** - Document what went wrong
4. **Timeline** - 1-2 sprints to stabilize legacy

---

## Dependencies & Cargo Configuration

### New Dependencies (Phase 3A-3F)

**crates/infra/Cargo.toml additions:**

```toml
[dependencies]
# Core dependencies
pulsearc-core = { path = "../core" }
pulsearc-domain = { path = "../domain" }
pulsearc-common = { path = "../common", features = ["runtime", "platform"] }

async-trait = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time", "fs"] }
tracing = "0.1"
thiserror = "2.0"
anyhow = "1.0"

# Database (always enabled)
rusqlite = { version = "0.31", features = ["bundled-sqlcipher"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"

# HTTP client (always enabled)
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Caching
moka = { version = "0.12", features = ["future"] }

# Scheduling
tokio-cron-scheduler = "0.10"

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"
objc = "0.2"
core-foundation = "0.9"
core-graphics = "0.23"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_Threading",
] }

[features]
default = ["database", "platform"]

# Core features
database = []
platform = []

# Integration features
sap = ["dep:oauth2"]
calendar = ["dep:oauth2"]
ml = ["dep:linfa", "dep:linfa-trees", "dep:ndarray"]
tree-classifier = ["ml"]
graphql = ["dep:graphql_client"]

# Feature groups
integrations = ["sap", "calendar"]
all = ["ml", "integrations", "graphql"]

[dev-dependencies]
wiremock = "0.6"  # Mock HTTP server
tempfile = "3.0"  # Temporary files for tests
```

---

## Timeline & Milestones

### Gantt Chart (Weeks 3-7)

```
Week 3: Phase 3A (Core Infrastructure)
├─ Day 1: Config + Errors
├─ Day 2: HTTP Client
├─ Day 3: DB Manager
├─ Day 4: Activity + Segment Repos
├─ Day 5: Block + Outbox Repos
├─ Day 6-7: Additional Repos
└─ Milestone: Core infra complete ✓

Week 4: Phase 3B (Platform Adapters)
├─ Day 1-2: macOS Provider
├─ Day 2-3: macOS Enrichment
├─ Day 3-4: macOS Events
├─ Day 4: Accessibility Helpers
├─ Day 5: Enrichers
├─ Day 6: Fallback Provider
└─ Milestone: Platform adapters complete ✓

Week 5: Phase 3C (Integration Adapters)
├─ Day 1-2: OpenAI Adapter
├─ Day 2-3: SAP Client
├─ Day 3: SAP Cache/Validation
├─ Day 4: SAP Supporting
├─ Day 4-5: Calendar Client
├─ Day 5: Calendar OAuth
├─ Day 6: Calendar Providers
├─ Day 7: Calendar Supporting
└─ Milestone: Integrations complete ✓

Week 6: Phase 3D (Schedulers & Workers)
├─ Day 1: Block + Classification Schedulers
├─ Day 2: Integration Schedulers
├─ Day 3: Outbox Worker
├─ Day 4: Sync Supporting
├─ Day 5: Domain API Client
└─ Milestone: Schedulers complete ✓

Week 7: Phase 3E + 3F (ML + Observability) - Parallel
├─ Phase 3E (ML - Optional)
│  ├─ Day 1: Linfa Classifier
│  ├─ Day 2: Additional Classifiers
│  ├─ Day 3: Training Pipeline
│  └─ Day 4: Batch Classifier
└─ Phase 3F (Observability - Parallel)
   ├─ Day 1-2: Metrics Collection
   └─ Day 3: Integration
└─ Milestone: Phase 3 complete ✓
```

### Critical Path

**Must complete in order:**
1. 3A → 3B (Platform depends on database)
2. 3A → 3C (Integrations depend on HTTP client)
3. 3C → 3D (Schedulers depend on integrations)

**Can run in parallel:**
- 3B + 3C (Platform + Integrations both depend on 3A)
- 3D + 3E (Schedulers + ML both depend on 3A)
- 3F (Observability) can run anytime

### Suggested Team Assignment (2 developers)

**Developer 1 (Backend Focus):**
- Phase 3A (all database repos)
- Phase 3D (schedulers & workers)
- Phase 3E (ML adapters)

**Developer 2 (Platform/Integration Focus):**
- Phase 3B (platform adapters)
- Phase 3C (integration adapters)
- Phase 3F (observability)

**Parallel Timeline:** 4 weeks instead of 6 weeks

---

## Success Criteria Summary

### Functional Requirements

**Must Have (P1):**
- [ ] All database repositories implemented
- [ ] macOS activity tracking works
- [ ] HTTP client with retry/timeout works
- [ ] Config loading works
- [ ] Error conversions preserve context

**Should Have (P2):**
- [ ] SAP integration works (feature-gated)
- [ ] Calendar integration works (feature-gated)
- [ ] OpenAI adapter works
- [ ] Schedulers run jobs correctly
- [ ] Outbox processing works

**Nice to Have (P3):**
- [ ] ML adapters work (feature-gated)
- [ ] Batch classifier works
- [ ] Metrics collection works
- [ ] Windows basic tracking works
- [ ] Linux placeholder compiles

### Non-Functional Requirements

**Quality:**
- [ ] Code coverage ≥ 70%
- [ ] No clippy warnings
- [ ] All public APIs documented
- [ ] All tests pass

**Performance:**
- [ ] Database operations meet targets
- [ ] Platform providers meet latency targets
- [ ] Integration adapters respect timeouts
- [ ] No memory leaks
- [ ] No data races

**Platform:**
- [ ] macOS: full functionality
- [ ] Windows: basic tracking compiles
- [ ] Linux: placeholder compiles
- [ ] All feature combinations compile

---

## Documentation Updates

### Files to Update After Phase 3

1. **LEGACY_MIGRATION_INVENTORY.md**
   - Mark Phase 3 modules as complete
   - Update progress percentages

2. **Architecture Diagrams**
   - Update dependency graph
   - Show infra layer connections

3. **API Documentation**
   - Document all public traits
   - Add usage examples

4. **CLAUDE.md**
   - Update with Phase 3 learnings
   - Add infra-specific guidelines

5. **README Files**
   - Add `crates/infra/README.md`
   - Document feature flags
   - Add setup instructions

---

## Next Steps

### After Phase 3 Complete

**Immediate:**
1. Update LEGACY_MIGRATION_INVENTORY.md with completion status
2. Run full CI pipeline and verify all tests pass
3. Manual testing on all supported platforms
4. Performance benchmarking and comparison with legacy

**Phase 4 Preparation:**
1. Review Phase 4 plan (API Layer & Commands)
2. Identify any blockers
3. Update timeline based on Phase 3 learnings

**Technical Debt:**
1. Refactor any quick-and-dirty code from Phase 3
2. Add missing documentation
3. Improve test coverage if < 70%
4. Address any performance issues found

---

**Document Version:** 1.1
**Last Updated:** 2025-01-30
**Status:** ✅ Reviewed and Updated
**Next Review:** After Phase 2 complete

---

## Document Change Log

### Version 1.1 (2025-01-30) - Post-Review Updates

**Critical Updates:**
1. ✅ **Added SqlCipher API Warning** - Prominent warning box in Phase 3A about `query_map()` returning `Vec<T>` directly
2. ✅ **Clarified Platform Support** - Made explicit that Phase 3 is macOS-only; Windows/Linux are future work
3. ✅ **Added Migration Strategy** - New section explaining gradual migration approach with adapter pattern
4. ✅ **Extended Phase 3A Timeline** - Increased from 5-7 days to 7-10 days for realistic scheduling
5. ✅ **Added Phase 2 Blockers** - Explicit list of completed Phase 2 PRs needed before starting

**Medium Priority Updates:**
6. ✅ **Verified LOC Counts** - Updated Tasks 3A.5, 3A.6, 3A.7 with actual line counts (653, 374, 551 LOC)
7. ✅ **Added Performance Baseline Task** - New Task 3A.0 for establishing legacy performance metrics
8. ✅ **Added Schema Migration Checklist** - Database schema verification steps in Task 3A.4
9. ✅ **Added Feature Flag CI Automation** - Example `xtask` code for automated feature matrix testing
10. ✅ **Updated Task 3B.6** - Marked Windows/Linux fallback provider as future work with minimal stub

**Documentation Improvements:**
11. ✅ **Updated Phase Duration** - Total timeline: 25-34 days (5.0-6.8 weeks)
12. ✅ **Updated Platform Sections** - Clarified macOS-only scope in Executive Summary, validation sections
13. ✅ **Updated Manual Testing** - Removed Windows/Linux from manual testing checklist
14. ✅ **Updated Acceptance Criteria** - Platform support now shows macOS (full) vs Windows/Linux (compile-only)

**Review Findings Addressed:**
- ✅ Critical: SqlCipher API difference now prominently documented
- ✅ Critical: Platform scope clarified (macOS-only for Phase 3)
- ✅ Critical: Migration strategy documented
- ✅ High: Phase 2 blockers specified
- ✅ High: LOC estimates verified
- ✅ High: Timeline extended to 7-10 days for Phase 3A
- ✅ Medium: Performance baseline task added
- ✅ Medium: Feature flag CI automation documented
- ✅ Medium: Schema migration verification added

**Status:** Document is now ready for Phase 3 execution. All critical review findings have been addressed.

---

## Appendix: Module Count Summary

### By Sub-Phase

| Sub-Phase | Modules | Estimated LOC | Priority |
|-----------|---------|---------------|----------|
| 3A | 15 | ~4,500 | P1 |
| 3B | 7 | ~3,500 | P1 |
| 3C | 17 | ~4,500 | P2 |
| 3D | 10 | ~2,500 | P2 |
| 3E | 6 | ~2,000 | P3 |
| 3F | 5+ | ~600 | P3 |
| **Total** | **60+** | **~17,600** | - |

### By Feature Flag

| Feature | Modules | Required? |
|---------|---------|-----------|
| `database` | 15 | Yes (default) |
| `platform` | 7 | Yes (default) |
| `sap` | 6 | No (opt-in) |
| `calendar` | 7 | No (opt-in) |
| `ml` | 6 | No (opt-in) |
| `tree-classifier` | 1 | No (opt-in) |
| `graphql` | 1 | No (opt-in) |

---

**END OF PHASE 3 TRACKING DOCUMENT**
