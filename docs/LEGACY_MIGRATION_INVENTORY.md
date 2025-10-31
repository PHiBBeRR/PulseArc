# Legacy Code Migration Inventory

**Generated**: October 30, 2025
**Last Updated**: October 30, 2025 (Post-Critical Review)
**Purpose**: Classify all `legacy/api/src/` modules by target crate for ADR-003 migration
**Status**: ⚠️ BLOCKED - Critical issues require refactoring before Phase 1

---

## ⚠️ CRITICAL ISSUES - MUST RESOLVE BEFORE MIGRATION

### Blockers Requiring Immediate Action

Several modules classified as `domain` or `core` contain **side effects** that violate layered architecture rules. These must be refactored or reclassified before Phase 1 can begin.

**Critical Reclassifications:**
1. ❌ `shared/config.rs` (line 82) → **Cannot move to domain** (reads env vars, filesystem)
2. ❌ `observability/errors/app.rs` (line 169) → **Split required** (domain types + infra conversions)
3. ❌ `integrations/sap/errors.rs` (line 147) → **Move to infra** (wraps reqwest::Error)
4. ❌ `integrations/sap/validation.rs` (line 148) → **Move to infra** (uses DbManager)
5. ❌ `preprocess/segmenter.rs` (line 103) → **Refactor required** (raw DB calls)
6. ❌ `inference/batch_classifier.rs` (line 121) → **Move to infra** (DbManager + Tauri)

**Feature Flag Mismatch:**
- Inventory documents `calendar`, `sap`, `ml` features
- Actual Cargo.toml only defines `tree-classifier`, `graphql`
- Either rename docs or add missing feature declarations

**Decision Required:** Split or reclassify these modules before beginning Phase 1.

### Quick Reference: Blockers by Action Required

| Module | Action | Effort | Blocker Type |
|--------|--------|--------|--------------|
| `shared/config.rs` | Split (types → domain, loader → infra) | Medium | Side Effects |
| `observability/errors/app.rs` | Split (types → domain, conversions → infra) | Medium | Infra Dependencies |
| `preprocess/segmenter.rs` | Refactor (add repository port) | High | Direct DB Access |
| `inference/batch_classifier.rs` | Reclassify (→ infra) | Low | Side Effects |
| `integrations/sap/errors.rs` | Reclassify (→ infra) | Low | Transport Coupling |
| `integrations/sap/validation.rs` | Reclassify (→ infra) | Low | DB Access |
| Feature flags (`calendar`, `sap`, `ml`) | Add to Cargo.toml | Low | Missing Declarations |

**Total Effort Estimate**: 1 week (5-8 business days)

---

## Executive Summary

This inventory classifies ~150+ modules from `legacy/api/src/` into target crates (`domain`, `core`, `infra`, `api`) according to the layered architecture defined in ADR-003.

**Key Exclusions**:
- Code marked with `FEATURE-*` or `PHASE-*` comments are noted but migration priority TBD
- Experimental/WIP code excluded from initial migration
- Feature-flagged code (ML, GraphQL) migrated behind equivalent flags

---

## Classification Rules

### Domain (`pulsearc-domain`)
**Criteria**: Pure data types, zero side effects, no infrastructure
- Data structures (structs, enums)
- Configuration types
- Domain errors
- Constants
- Pure utility functions (string helpers, generic extractors)

### Core (`pulsearc-core`)
**Criteria**: Business logic, hexagonal ports (traits), use cases
- Service implementations
- Port trait definitions
- Business rule validation
- Domain event handling
- Domain-specific utilities (e.g., platform-specific extraction rules)

### Infra (`pulsearc-infra`)
**Criteria**: Port implementations, all side effects
- Database repositories
- HTTP clients
- Platform providers (macOS APIs)
- Integration adapters

### API (`pulsearc-api`)
**Criteria**: Presentation layer, DI wiring
- Tauri command handlers
- AppContext (DI container)
- Request/response mapping
- Error mapping

---

## Module Classification Table

| Legacy Path | Target Crate | Target Path | Status | Notes |
|-------------|--------------|-------------|--------|-------|
| **Database & Models** |
| `db/models.rs` | `domain` | `domain/src/types/database.rs` | ✅ Priority 1 | Core domain types: ActivitySnapshot, ActivitySegment, TimeEntryOutbox, etc. |
| `db/models_idle.rs` | `domain` | `domain/src/types/idle.rs` | ✅ Priority 1 | IdlePeriod, IdleSummary types |
| `db/manager.rs` | `infra` | `infra/src/database/manager.rs` | ✅ Priority 2 | DbManager with connection pooling |
| `db/local.rs` | `infra` | `infra/src/database/local.rs` | ✅ Priority 2 | Local database operations |
| `db/migrations.rs` | `infra` | `infra/src/database/migrations.rs` | ✅ Priority 2 | Schema migrations |
| `db/activity/snapshots.rs` | `infra` | `infra/src/database/activity_repository.rs` | ✅ Priority 2 | Implement `ActivityRepository` trait |
| `db/activity/segments.rs` | `infra` | `infra/src/database/segment_repository.rs` | ✅ Priority 2 | Implement `SegmentRepository` trait |
| `db/blocks/operations.rs` | `infra` | `infra/src/database/block_repository.rs` | ✅ Priority 2 | Implement `BlockRepository` trait |
| `db/calendar/events.rs` | `infra` | `infra/src/database/calendar_repository.rs` | ⚠️ Priority 2 | Feature-gated: calendar |
| `db/calendar/tokens.rs` | `infra` | `infra/src/database/calendar_repository.rs` | ⚠️ Priority 2 | Feature-gated: calendar |
| `db/calendar/sync_settings.rs` | `infra` | `infra/src/database/calendar_repository.rs` | ⚠️ Priority 2 | Feature-gated: calendar |
| `db/calendar/suggestions.rs` | `infra` | `infra/src/database/calendar_repository.rs` | ⚠️ Priority 2 | Feature-gated: calendar |
| `db/outbox/outbox.rs` | `infra` | `infra/src/database/outbox_repository.rs` | ✅ Priority 2 | Implement `OutboxQueue` trait |
| `db/outbox/id_mappings.rs` | `infra` | `infra/src/database/id_mapping_repository.rs` | ✅ Priority 2 | ID mapping operations |
| `db/outbox/token_usage.rs` | `infra` | `infra/src/database/token_usage_repository.rs` | ✅ Priority 2 | Token tracking |
| `db/batch/operations.rs` | `infra` | `infra/src/database/batch_repository.rs` | ✅ Priority 2 | Batch queue operations |
| `db/batch/dlq.rs` | `infra` | `infra/src/database/dlq_repository.rs` | ✅ Priority 2 | Dead letter queue |
| `db/utils/stats.rs` | `infra` | `infra/src/database/stats.rs` | ✅ Priority 2 | Database statistics |
| `db/utils/raw_queries.rs` | `infra` | `infra/src/database/raw_queries.rs` | ✅ Priority 2 | Raw SQL operations |
| **Shared Types & Config** |
| `shared/types/mod.rs` | `domain` | `domain/src/types/activity.rs` | ✅ Priority 1 | ActivityContext, WindowContext, WorkType, ActivityCategory |
| `shared/types/stats.rs` | `domain` | `domain/src/types/stats.rs` | ✅ Priority 1 | BatchStats, statistics types |
| `shared/config.rs` | ❌ **BLOCKED** | **SPLIT REQUIRED** | ⚠️ Refactor | Config **structs** → domain; `from_env()` + filesystem I/O → infra loader |
| `shared/constants/mod.rs` | `domain` | `domain/src/constants.rs` | ✅ Priority 1 | Application constants |
| `shared/auth/` | `infra` | `infra/src/auth/` | ⚠️ Priority 3 | OAuth implementation (feature-gated) |
| `shared/cache.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::cache` instead |
| `shared/extractors/pattern.rs` | `domain` | `domain/src/utils/pattern_extractor.rs` | ✅ Priority 1 | Pure utility builder (no business logic) |
| **Tracker & Activity Provider** |
| `tracker/core.rs` | `core` | `core/src/tracking/service.rs` | ✅ Priority 1 | TrackingService business logic |
| `tracker/provider.rs` | `core` | `core/src/tracking/ports.rs` | ✅ Priority 1 | `ActivityProvider` trait definition |
| `tracker/providers/macos.rs` | `infra` | `infra/src/platform/macos/activity_provider.rs` | ✅ Priority 2 | Implement `ActivityProvider` for macOS |
| `tracker/providers/dummy.rs` | `infra` | `infra/src/platform/dummy/activity_provider.rs` | ✅ Priority 2 | Test/fallback provider |
| `tracker/os_events/macos_ax.rs` | `infra` | `infra/src/platform/macos/accessibility.rs` | ✅ Priority 2 | macOS Accessibility API |
| `tracker/os_events/macos.rs` | `infra` | `infra/src/platform/macos/event_monitor.rs` | ✅ Priority 2 | Event monitoring |
| `tracker/os_events/traits.rs` | `core` | `core/src/tracking/ports.rs` | ✅ Priority 2 | EventProvider trait |
| `tracker/os_events/fallback.rs` | `infra` | `infra/src/platform/fallback/event_provider.rs` | ✅ Priority 2 | Fallback implementation |
| `tracker/idle/detector.rs` | `core` | `core/src/idle/detector.rs` | ✅ Priority 2 | Idle detection business logic |
| `tracker/idle/period_tracker.rs` | `core` | `core/src/idle/period_tracker.rs` | ✅ Priority 2 | Period tracking logic |
| `tracker/idle/recovery.rs` | `core` | `core/src/idle/recovery.rs` | ✅ Priority 2 | Recovery logic |
| `tracker/idle/config.rs` | `domain` | `domain/src/config/idle_config.rs` | ✅ Priority 1 | IdleConfig types |
| `tracker/idle/types.rs` | `domain` | `domain/src/types/idle.rs` | ✅ Priority 1 | Idle-related types |
| `tracker/idle/lock_detection.rs` | `infra` | `infra/src/platform/macos/lock_detection.rs` | ✅ Priority 2 | Platform-specific lock detection |
| **Preprocessing** |
| `preprocess/segmenter.rs` | ❌ **BLOCKED** | **REFACTOR REQUIRED** | ⚠️ Refactor | Currently uses `LocalDatabase` + raw rusqlite; needs `SegmentRepository` port first |
| `preprocess/trigger.rs` | `core` | `core/src/tracking/trigger.rs` | ✅ Priority 2 | Trigger logic |
| `preprocess/redact.rs` | `core` | `core/src/privacy/redactor.rs` | ✅ Priority 2 | PII redaction logic |
| **Inference & Classification** |
| `inference/types.rs` | `domain` | `domain/src/types/classification.rs` | ✅ Priority 1 | ProposedBlock, ContextSignals, ProjectMatch, ActivityBreakdown |
| `inference/signals.rs` | `core` | `core/src/classification/signals.rs` | ✅ Priority 2 | SignalExtractor business logic |
| `inference/project_matcher.rs` | `core` | `core/src/classification/project_matcher.rs` | ✅ Priority 2 | ProjectMatcher business logic |
| `inference/evidence_extractor.rs` | `core` | `core/src/classification/evidence.rs` | ✅ Priority 2 | EvidenceExtractor logic |
| `inference/block_builder.rs` | `core` | `core/src/classification/block_builder.rs` | ✅ Priority 2 | BlockBuilder orchestration |
| `inference/hybrid_classifier.rs` | `core` | `core/src/classification/hybrid.rs` | ⚠️ Priority 2 | Feature-gated: tree-classifier |
| `inference/rules_classifier.rs` | `core` | `core/src/classification/rules.rs` | ⚠️ Priority 2 | Feature-gated: tree-classifier |
| `inference/logistic_classifier.rs` | `infra` | `infra/src/ml/logistic_classifier.rs` | ⚠️ Priority 3 | Feature-gated: ml |
| `inference/tree_classifier.rs` | `infra` | `infra/src/ml/tree_classifier.rs` | ⚠️ Priority 3 | Feature-gated: ml |
| `inference/linfa_integration.rs` | `infra` | `infra/src/ml/linfa_classifier.rs` | ⚠️ Priority 3 | Feature-gated: ml |
| `inference/training_pipeline.rs` | `infra` | `infra/src/ml/training_pipeline.rs` | ⚠️ Priority 3 | Feature-gated: ml |
| `inference/training_data_exporter.rs` | `infra` | `infra/src/ml/training_exporter.rs` | ⚠️ Priority 3 | Feature-gated: ml |
| `inference/weights_config.rs` | `domain` | `domain/src/config/weights_config.rs` | ⚠️ Priority 2 | Feature-gated: ml |
| `inference/metrics.rs` | `infra` | `infra/src/ml/metrics.rs` | ⚠️ Priority 3 | Feature-gated: ml |
| `inference/batch_classifier.rs` | ❌ **BLOCKED** | `infra/src/classification/batch_classifier.rs` | ⚠️ Reclassify | Uses `DbManager` + `tauri::Emitter` (side effects) → belongs in infra |
| `inference/scheduler.rs` | `infra` | `infra/src/scheduling/block_scheduler.rs` | ✅ Priority 3 | Scheduler implementation |
| `inference/classification_scheduler.rs` | `infra` | `infra/src/scheduling/classification_scheduler.rs` | ✅ Priority 3 | Classification scheduler |
| `inference/timezone_utils.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::time` instead |
| `inference/openai_types.rs` | `infra` | `infra/src/integrations/openai/types.rs` | ✅ Priority 2 | OpenAI adapter DTOs (map to domain types in adapter) |
| **Detection Packs** |
| `detection/default.rs` | `core` | `core/src/detection/default.rs` | ✅ Priority 2 | Default detection logic |
| `detection/enrichers/browser.rs` | `infra` | `infra/src/platform/enrichers/browser.rs` | ✅ Priority 2 | Browser enrichment (platform-specific) |
| `detection/enrichers/office.rs` | `infra` | `infra/src/platform/enrichers/office.rs` | ✅ Priority 2 | Office enrichment |
| `detection/packs/**/*.rs` | `core` | `core/src/detection/packs/` | ⚠️ Priority 3 | Industry-specific packs (consulting, deals, finance, legal, sales, technology) |
| **Integrations** |
| `integrations/calendar/client.rs` | `infra` | `infra/src/integrations/calendar/client.rs` | ⚠️ Priority 3 | Feature-gated: calendar |
| `integrations/calendar/oauth.rs` | `infra` | `infra/src/integrations/calendar/oauth.rs` | ⚠️ Priority 3 | Feature-gated: calendar |
| `integrations/calendar/parser.rs` | `core` | `core/src/integrations/calendar_parser.rs` | ⚠️ Priority 2 | Feature-gated: calendar (pure logic) |
| `integrations/calendar/providers/**/*.rs` | `infra` | `infra/src/integrations/calendar/providers/` | ⚠️ Priority 3 | Feature-gated: calendar |
| `integrations/calendar/sync.rs` | `infra` | `infra/src/integrations/calendar/sync.rs` | ⚠️ Priority 3 | Feature-gated: calendar |
| `integrations/calendar/scheduler.rs` | `infra` | `infra/src/integrations/calendar/scheduler.rs` | ⚠️ Priority 3 | Feature-gated: calendar |
| `integrations/calendar/types.rs` | `domain` | `domain/src/types/calendar.rs` | ⚠️ Priority 1 | Feature-gated: calendar |
| `integrations/calendar/config.rs` | `domain` | `domain/src/config/calendar_config.rs` | ⚠️ Priority 1 | Feature-gated: calendar |
| `integrations/sap/client.rs` | `infra` | `infra/src/integrations/sap/client.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/auth_commands.rs` | `infra` | `infra/src/integrations/sap/auth.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/cache.rs` | `infra` | `infra/src/integrations/sap/cache.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/forwarder.rs` | `infra` | `infra/src/integrations/sap/forwarder.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/health_monitor.rs` | `infra` | `infra/src/integrations/sap/health.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/scheduler.rs` | `infra` | `infra/src/integrations/sap/scheduler.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/models.rs` | `domain` | `domain/src/types/sap.rs` | ⚠️ Priority 1 | Feature-gated: sap |
| `integrations/sap/errors.rs` | ❌ **BLOCKED** | `infra/src/integrations/sap/errors.rs` | ⚠️ Reclassify | Wraps `reqwest::Error` directly → transport-specific, belongs in infra |
| `integrations/sap/validation.rs` | ❌ **BLOCKED** | `infra/src/integrations/sap/validation.rs` | ⚠️ Reclassify | Uses `DbManager` + `WbsCache` (DB access) → belongs in infra |
| **HTTP** |
| `http/client.rs` | `infra` | `infra/src/http/client.rs` | ✅ Priority 2 | HTTP client implementation |
| `http/graphql.rs` | `infra` | `infra/src/http/graphql.rs` | ⚠️ Priority 3 | Feature-gated: graphql |
| **Domain / API Integration** |
| `domain/api/client.rs` | `infra` | `infra/src/api/client.rs` | ✅ Priority 3 | Main API client |
| `domain/api/auth.rs` | `infra` | `infra/src/api/auth.rs` | ✅ Priority 3 | API authentication |
| `domain/api/commands.rs` | `infra` | `infra/src/api/commands.rs` | ✅ Priority 3 | API commands |
| `domain/api/forwarder.rs` | `infra` | `infra/src/api/forwarder.rs` | ✅ Priority 3 | API forwarder |
| `domain/api/scheduler.rs` | `infra` | `infra/src/api/scheduler.rs` | ✅ Priority 3 | API scheduler |
| `domain/api/models.rs` | `domain` | `domain/src/types/api.rs` | ✅ Priority 1 | API types |
| `domain/user_profile.rs` | `domain` | `domain/src/types/user_profile.rs` | ✅ Priority 1 | User profile types |
| **Sync** |
| `sync/outbox_worker.rs` | `infra` | `infra/src/sync/outbox_worker.rs` | ✅ Priority 3 | Outbox worker |
| `sync/neon_client.rs` | `infra` | `infra/src/sync/neon_client.rs` | ✅ Priority 3 | Neon database client |
| `sync/scheduler.rs` | `infra` | `infra/src/sync/scheduler.rs` | ✅ Priority 3 | Sync scheduler |
| `sync/retry.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::resilience::retry` instead |
| `sync/cost_tracker.rs` | `infra` | `infra/src/sync/cost_tracker.rs` | ✅ Priority 3 | Cost tracking |
| `sync/cleanup.rs` | `infra` | `infra/src/sync/cleanup.rs` | ✅ Priority 3 | Cleanup logic |
| **Observability** |
| `observability/metrics/**/*.rs` | `infra` | `infra/src/observability/metrics/` | ✅ Priority 3 | Metrics collection |
| `observability/errors/app.rs` | ❌ **BLOCKED** | **SPLIT REQUIRED** | ⚠️ Refactor | Error **types** → domain; `From<rusqlite>`, `From<reqwest>`, `From<keyring>` → infra conversions |
| `observability/datadog.rs` | `infra` | `infra/src/observability/datadog.rs` | ❌ Priority 4 | External observability (optional) |
| **Commands (API Layer)** |
| `commands/blocks.rs` | `api` | `api/src/commands/blocks.rs` | ✅ Priority 4 | Tauri command handlers |
| `commands/calendar.rs` | `api` | `api/src/commands/calendar.rs` | ⚠️ Priority 4 | Feature-gated: calendar |
| `commands/database.rs` | `api` | `api/src/commands/database.rs` | ✅ Priority 4 | Database commands |
| `commands/idle.rs` | `api` | `api/src/commands/idle.rs` | ✅ Priority 4 | Idle commands |
| `commands/idle_sync.rs` | `api` | `api/src/commands/idle_sync.rs` | ✅ Priority 4 | Idle sync commands |
| `commands/ml_training.rs` | `api` | `api/src/commands/ml_training.rs` | ⚠️ Priority 4 | Feature-gated: ml |
| `commands/monitoring.rs` | `api` | `api/src/commands/monitoring.rs` | ✅ Priority 4 | Monitoring commands |
| `commands/user_profile.rs` | `api` | `api/src/commands/user_profile.rs` | ✅ Priority 4 | User profile commands |
| `commands/window.rs` | `api` | `api/src/commands/window.rs` | ✅ Priority 4 | Window commands |
| `commands/seed_snapshots.rs` | ❌ **EXCLUDED** | N/A | ❌ Excluded | Test/seed data utility |
| **Utilities** |
| `utils/patterns.rs` | `core` | `core/src/utils/patterns.rs` | ✅ Priority 2 | Domain-specific extraction rules (uses PatternExtractor) |
| `utils/title.rs` | `domain` | `domain/src/utils/title.rs` | ✅ Priority 1 | Pure string helpers (delimiter splitting, truncation) |
| **Tooling** |
| `tooling/macros/status_enum.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::impl_status_conversions!` macro |

---

## Refactoring Requirements (Pre-Migration)

The following modules **must be refactored** before they can migrate to their target crates. Each contains side effects that violate layer separation rules.

### 1. `shared/config.rs` → Split into Domain + Infra

**Current Issues:**
- Reads environment variables (`std::env::var()`)
- Reads filesystem (`std::fs::read_to_string()`)
- Probes executable paths (`std::env::current_exe()`, `std::env::current_dir()`)

**Refactoring Strategy:**
```rust
// domain/src/config/app_config.rs (Pure data structures)
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

### 2. `observability/errors/app.rs` → Split Error Types + Conversions

**Current Issues:**
- Implements `From<rusqlite::Error>` (infra dependency)
- Implements `From<reqwest::Error>` (infra dependency)
- Implements `From<keyring::Error>` (infra dependency)

**Refactoring Strategy:**
```rust
// domain/src/errors/mod.rs (Pure error types)
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

### 3. `preprocess/segmenter.rs` → Add Repository Port

**Current Issues:**
- Direct imports: `crate::db::activity::SegmentOperations`
- Uses `LocalDatabase` directly
- Contains raw rusqlite calls

**Refactoring Strategy:**
```rust
// core/src/tracking/segmenter.rs (Business logic)
pub struct Segmenter<R: SegmentRepository> {
    repository: R,
}

// infra/src/database/segment_repository.rs (Port implementation)
impl SegmentRepository for SqliteSegmentRepository {
    async fn save_segment(&self, segment: &ActivitySegment) -> Result<()> {
        // Raw DB calls here
    }
}
```

### 4. `inference/batch_classifier.rs` → Move to Infra

**Current Issues:**
- Uses `DbManager` directly
- Uses `tauri::Emitter` (presentation layer concern)

**Resolution:** Already classified as infra in table above. No split needed—entire module belongs in infra.

### 5. `integrations/sap/errors.rs` → Move to Infra

**Current Issues:**
- `from_reqwest()` method wraps `reqwest::Error`
- Transport-specific error handling

**Resolution:** Reclassify to `infra/src/integrations/sap/errors.rs`. No split needed.

### 6. `integrations/sap/validation.rs` → Move to Infra

**Current Issues:**
- Uses `DbManager` directly
- Uses `WbsCache` (DB-backed cache)

**Resolution:** Reclassify to `infra/src/integrations/sap/validation.rs`. No split needed.

---

## Feature Flag Alignment

### Current State (Cargo.toml)
```toml
[features]
default = ["tree-classifier"]
tree-classifier = ["dep:linfa", "dep:linfa-trees", "dep:linfa-logistic", "dep:ndarray"]
graphql = ["dep:graphql_client"]
```

### Documented Features (This Inventory)
- `calendar` (not in Cargo.toml)
- `sap` (not in Cargo.toml)
- `ml` (not in Cargo.toml)
- `tree-classifier` ✅
- `graphql` ✅

### Required Action
**Option A:** Add missing features to `Cargo.toml`
```toml
[features]
calendar = []
sap = []
ml = ["tree-classifier"]  # Alias for ML features
```

**Option B:** Update inventory to match existing features
- Replace `calendar` → document as "future feature"
- Replace `sap` → document as "future feature"
- Replace `ml` → use `tree-classifier` instead

**Recommendation:** Option A (add features to Cargo) for explicit gating.

---

## Priority Legend

- **Priority 1**: Domain types and configuration (Week 1)
- **Priority 2**: Core business logic and infra adapters (Week 2-4)
- **Priority 3**: Integration adapters and schedulers (Week 4-5)
- **Priority 4**: API layer commands (Week 5)

---

## Exclusion Rationale

### Excluded from Migration

1. **`shared/cache.rs`** → Use `pulsearc_common::cache` instead
2. **`sync/retry.rs`** → Use `pulsearc_common::resilience::retry` instead
3. **`inference/timezone_utils.rs`** → Use `pulsearc_common::time` instead
4. **`tooling/macros/status_enum.rs`** → Use `pulsearc_common::impl_status_conversions!` macro
5. **`commands/seed_snapshots.rs`** → Test utility, not production code
6. **`observability/datadog.rs`** → External observability (optional, low priority)

### Feature-Gated Modules

Modules marked with ⚠️ require feature flags:

**`calendar` feature**:
- All `integrations/calendar/` modules
- Calendar-related database operations
- Calendar commands

**`sap` feature**:
- All `integrations/sap/` modules
- SAP-related types

**`ml` feature**:
- ML classifier modules
- Training pipeline
- Linfa integration

**`graphql` feature**:
- `http/graphql.rs`

---

## Port Trait Definitions (Core)

The following traits need to be defined in `core` layer:

### Tracking Ports
```rust
// core/src/tracking/ports.rs
pub trait ActivityProvider: Send + Sync {
    async fn capture_activity(&self) -> Result<ActivitySnapshot>;
    async fn pause(&self) -> Result<()>;
    async fn resume(&self) -> Result<()>;
    fn is_paused(&self) -> bool;
}

pub trait ActivityRepository: Send + Sync {
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()>;
    async fn find_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ActivitySnapshot>>;
}

pub trait SegmentRepository: Send + Sync {
    async fn save_segment(&self, segment: &ActivitySegment) -> Result<()>;
    async fn find_segments_by_date(&self, date: NaiveDate) -> Result<Vec<ActivitySegment>>;
}
```

### Classification Ports
```rust
// core/src/classification/ports.rs
pub trait Classifier: Send + Sync {
    /// Classify activity snapshots into proposed time blocks
    async fn classify(&self, snapshots: Vec<ActivitySnapshot>) -> Result<Vec<ProposedBlock>>;

    /// Health check for classifier availability (e.g., ML model validation, API connectivity)
    async fn health_check(&self) -> Result<()>;
}

pub trait BlockRepository: Send + Sync {
    async fn save_proposed_block(&self, block: &ProposedBlock) -> Result<()>;
    async fn get_proposed_blocks(&self, date: NaiveDate) -> Result<Vec<ProposedBlock>>;
}

pub trait ProjectMatcher: Send + Sync {
    async fn match_project(&self, signals: &ContextSignals) -> Result<Option<ProjectMatch>>;
}
```

### Integration Ports
```rust
// core/src/integrations/ports.rs
#[cfg(feature = "calendar")]
pub trait CalendarProvider: Send + Sync {
    async fn fetch_events(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<CalendarEvent>>;
    async fn sync(&self) -> Result<SyncStatus>;
}

#[cfg(feature = "sap")]
pub trait SapClient: Send + Sync {
    async fn forward_entry(&self, entry: &TimeEntry) -> Result<SapEntryId>;
    async fn validate_wbs(&self, wbs_code: &str) -> Result<bool>;
}
```

### Sync Ports
```rust
// core/src/sync/ports.rs
pub trait OutboxQueue: Send + Sync {
    async fn enqueue(&self, entry: &TimeEntryOutbox) -> Result<()>;
    async fn dequeue_batch(&self, limit: usize) -> Result<Vec<TimeEntryOutbox>>;
    async fn mark_sent(&self, id: &str) -> Result<()>;
    async fn mark_failed(&self, id: &str, error: &str) -> Result<()>;
}
```

---

## Dependency Analysis

### External Dependencies by Crate

**Domain**:
- serde, serde_json
- chrono, chrono-tz
- uuid (v7 feature)
- thiserror

**Core**:
- domain
- common (foundation/runtime)
- async-trait
- NO: rusqlite, reqwest, oauth2, objc2

**Infra**:
- core, domain, common (runtime/platform)
- rusqlite (feature: database)
- reqwest (feature: http)
- oauth2 (features: calendar, sap)
- objc2, objc2-app-kit, cocoa (feature: platform)
- linfa, linfa-trees (feature: ml)
- graphql_client (feature: graphql)

**API**:
- infra, core, domain, common
- tauri, tauri-plugin-shell
- serde, serde_json

---

## Migration Sequencing Strategy

### Phase 0: Pre-Migration Refactoring (Week 0)
**Goal**: Resolve all blockers before Phase 1

1. ✅ Split `shared/config.rs` → domain structs + infra loader
2. ✅ Split `observability/errors/app.rs` → domain types + infra conversions
3. ✅ Refactor `preprocess/segmenter.rs` → add `SegmentRepository` port
4. ✅ Reclassify `inference/batch_classifier.rs` → infra (no changes needed)
5. ✅ Reclassify `integrations/sap/errors.rs` → infra (no changes needed)
6. ✅ Reclassify `integrations/sap/validation.rs` → infra (no changes needed)
7. ✅ Add missing features to `Cargo.toml` (`calendar`, `sap`, `ml`)

**Validation**: All blocked modules resolved

### Phase 1: Foundation (Week 1)
**Goal**: Establish domain types and core ports

1. Move all `db/models.rs` types → `domain/src/types/`
2. Move `shared/types/` → `domain/src/types/`
3. Move `shared/config.rs` **structs** → `domain/src/config/app_config.rs` (after split)
4. Move `shared/constants/` → `domain/src/constants.rs`
5. Move `shared/extractors/pattern.rs` → `domain/src/utils/pattern_extractor.rs`
6. Move `utils/title.rs` → `domain/src/utils/title.rs`
7. Move `inference/types.rs` → `domain/src/types/classification.rs`
8. Move `observability/errors/app.rs` **types** → `domain/src/errors/mod.rs` (after split)
9. Define all port traits in `core/src/*/ports.rs`

**Validation**: `cargo check --package pulsearc-domain` passes with zero infra deps

### Phase 2: Core Business Logic (Week 2)
**Goal**: Migrate pure business logic

1. Move `utils/patterns.rs` → `core/src/utils/patterns.rs` (domain-specific extraction)
2. Move `tracker/core.rs` → `core/src/tracking/service.rs`
3. Move `preprocess/segmenter.rs` → `core/src/tracking/segmenter.rs`
4. Move `inference/block_builder.rs` → `core/src/classification/block_builder.rs`
5. Move `inference/signals.rs` → `core/src/classification/signals.rs`
6. Move `inference/evidence_extractor.rs` → `core/src/classification/evidence.rs`

**Validation**: Core tests pass with mock port implementations

### Phase 3: Infrastructure Adapters (Week 3-4)
**Goal**: Implement all port adapters

1. Database repositories (`db/activity/`, `db/blocks/`, `db/outbox/`)
2. Platform providers (`tracker/providers/macos.rs`, `tracker/os_events/`)
3. OpenAI adapter (`inference/openai_types.rs` → `infra/src/integrations/openai/`)
4. Integration adapters (calendar, SAP) behind feature flags
5. ML adapters (linfa, training) behind feature flags

**Validation**: Integration tests with real adapters pass

### Phase 4: API Layer (Week 5)
**Goal**: Migrate Tauri commands and wire everything

1. Move `commands/*.rs` → `api/src/commands/`
2. Build `api/src/context.rs` (DI container)
3. Create `api/src/mapping/` (domain ↔ frontend types)

**Validation**: Full application smoke tests pass

---

## Risk Assessment

### High Risk Areas

1. **Database Migrations**: `db/migrations.rs` contains SQLCipher setup
   - **Mitigation**: Test migration path with backup/restore

2. **macOS Platform Code**: Heavy use of `objc2`, Accessibility API
   - **Mitigation**: Careful refactoring, extensive manual testing

3. **Feature Flag Complexity**: Many modules gated behind features
   - **Mitigation**: Feature matrix testing in CI

4. **Circular Dependencies**: Some modules currently have circular refs
   - **Mitigation**: Identify and break with port pattern

### Low Risk Areas

1. **Domain Types**: Pure data structures, easy to move
2. **Pure Business Logic**: No side effects, straightforward migration
3. **Commands**: Already well-isolated, minimal refactoring needed

---

## Design Decisions (Resolved)

### 1. Pattern Module Split ✅
**Decision**: Split pattern utilities by abstraction level
- **`shared/extractors/pattern.rs`** → `domain` (generic utility builder)
- **`utils/title.rs`** → `domain` (pure string helpers)
- **`utils/patterns.rs`** → `core` (domain-specific business rules)

**Rationale**:
- `PatternExtractor` is a generic builder with zero business logic (pure abstraction)
- `title.rs` contains pure functions for string manipulation (no business rules)
- `patterns.rs` contains **domain knowledge** (how to extract Slack channels, GitHub PRs, Stack Overflow topics, with specific delimiters and filters for each platform)
- Core uses domain utilities to implement business logic
- **Key distinction**: Domain utilities are *reusable across any domain*, while core utilities encode *PulseArc-specific business rules*

### 2. OpenAI Types Placement ✅
**Decision**: Move to `infra/src/integrations/openai/types.rs`

**Rationale**:
- `BlockClassificationResponse` is OpenAI adapter-specific
- Core `Classifier` trait should return domain types (`Vec<ProposedBlock>`)
- Adapters map from `OpenAiBlockClassification` → `ProposedBlock`
- Keeps OpenAI-specific schema details in infrastructure layer

### 3. Detection Packs ✅
**Decision**: Start in `core`, move to feature flags if they grow large

**Rationale**: Industry-specific detection logic is business logic, even if domain-specific

### 4. Scheduler Placement ✅
**Decision**: `infra` (they're adapters for cron-like functionality)

### 5. Error Hierarchy ✅
**Decision**: Follow pattern in Common Crates Guide (module errors compose via `#[from]`)

### 6. Test Migration ✅
**Decision**: Unit tests move with code, add integration tests in `api/tests/`

---

## Success Criteria

- [ ] All modules classified with target crate
- [ ] Zero forbidden dependency edges (domain→core, core→infra, etc.)
- [ ] All port traits defined in core
- [ ] Feature flags properly gated in infra
- [ ] No use of `unwrap`/`expect` outside tests
- [ ] All `FEATURE`/`PHASE` comments documented
- [ ] Test coverage ≥80% in core/domain

---

## Next Steps

### Immediate Actions (Before Phase 1)

**🎫 GitHub Issue Created**: See [Phase 0 Blockers Tracking](issues/PHASE-0-BLOCKERS-TRACKING.md) for detailed task breakdown.

**To Create GitHub Issue**: Copy content from [GITHUB-ISSUE-PHASE-0.md](issues/GITHUB-ISSUE-PHASE-0.md)

1. ❌ **BLOCKED: Resolve Critical Issues** (see Phase 0 refactoring)
   - Split `shared/config.rs` into pure types + infra loader (2 days)
   - Split `observability/errors/app.rs` into pure types + infra conversions (2 days)
   - Add `SegmentRepository` port for `preprocess/segmenter.rs` (3-4 days)
   - Reclassify 3 modules to infra (<1 day)
   - Add missing feature flags to Cargo.toml (<1 day)

2. **Create Refactoring PRs**: Small PRs for each blocked module
   - See [Phase 0 Blockers Tracking](issues/PHASE-0-BLOCKERS-TRACKING.md) for PR checklists
3. **Update Feature Flags**: Align Cargo.toml with documented features
4. **Verify Zero Side Effects**: Run dependency checks on domain crate after splits

**Detailed Documentation**:
- **Task Tracking**: [issues/PHASE-0-BLOCKERS-TRACKING.md](issues/PHASE-0-BLOCKERS-TRACKING.md)
- **Full Specification**: [.github/ISSUE_TEMPLATE/phase-0-migration-blockers.md](../.github/ISSUE_TEMPLATE/phase-0-migration-blockers.md)

### Post-Refactoring Actions
1. ✅ **Review & Approval**: Classification reviewed and blockers identified
2. **Create Port Traits**: Define all traits in `core` before migration
3. **Week-by-Week PRs**: Small, incremental migrations with tests
4. **CI Updates**: Add dependency graph validation
5. **Documentation**: Update ADR-002 with migration notes

---

## Review Summary

### Initial Review (October 30, 2025)
**Changes Made**:
1. Reclassified `shared/extractors/pattern.rs` from `core` → `domain` (pure utility)
2. Reclassified `utils/title.rs` from `core` → `domain` (pure string helpers)
3. Reclassified `inference/openai_types.rs` from `domain` → `infra` (adapter DTOs)
4. Added `health_check()` method to `Classifier` trait
5. Documented rationale for pattern module split (domain utilities vs. business logic)
6. Resolved all open questions into concrete design decisions

**Validation**: All module classifications verified against source code

---

### Critical Review (October 30, 2025 - Post-Feedback)
**Critical Issues Identified:**
1. ❌ `shared/config.rs` reads env vars + filesystem → **MUST SPLIT**
2. ❌ `observability/errors/app.rs` has infra conversions (`From<rusqlite>`) → **MUST SPLIT**
3. ❌ `integrations/sap/errors.rs` wraps `reqwest::Error` → **RECLASSIFY TO INFRA**
4. ❌ `integrations/sap/validation.rs` uses `DbManager` → **RECLASSIFY TO INFRA**
5. ❌ `preprocess/segmenter.rs` has raw DB calls → **ADD REPOSITORY PORT**
6. ❌ `inference/batch_classifier.rs` uses `DbManager` + `tauri::Emitter` → **RECLASSIFY TO INFRA**
7. ❌ Feature flag mismatch (doc vs. Cargo.toml) → **ADD MISSING FEATURES**

**Total Blocked Modules**: 6 modules require refactoring or reclassification
**Estimated Refactoring Time**: 1 week (Phase 0)

**Validation**: All blockers verified by reading source code (lines 27-105, 363-447, 58-78, 4-29, 5-421)

---

**Document Status**: ⚠️ BLOCKED - Phase 0 refactoring required before Phase 1 can begin

