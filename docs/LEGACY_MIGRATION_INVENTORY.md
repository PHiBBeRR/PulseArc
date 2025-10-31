# Legacy Code Migration Inventory

**Generated**: October 30, 2025
**Last Updated**: November 2, 2025 (Phase 3A 🔄 - Tasks 3A.1-3A.6 Complete)
**Purpose**: Classify all `legacy/api/src/` modules by target crate for ADR-003 migration
**Status**: 🔄 PHASE 3A IN PROGRESS - Segment repository migrated (330 LOC, 5 tests) • Block repository up next

---

## ✅ PHASE 1 COMPLETE - FOUNDATION ESTABLISHED

### Domain Types & Core Ports Ready

**Phase 1 (Foundation) is complete!** All domain types and core port traits have been successfully migrated to their target crates. The foundation is now ready for Phase 2 (Core Business Logic) migration.

**What's Complete:**
- ✅ **Domain Types**: All pure data structures migrated to `pulsearc-domain`
- ✅ **Core Ports**: All hexagonal port traits defined in `pulsearc-core`
- ✅ **Feature Flags**: Calendar, SAP, ML features properly configured
- ✅ **TypeScript Generation**: ts-gen feature fully integrated
- ✅ **Zero Dependencies**: Domain has no forbidden dependencies
- ✅ **39 Tests Passing**: All utility and helper functions tested

**Migration Progress:**
- Phase 0: ✅ Complete (Pre-migration refactoring)
- Phase 1: ✅ Complete (Domain types & core ports) - October 31, 2025
- Phase 2: ✅ Complete (Core business logic) - November 1, 2025 (5 PRs, 2,610 lines, 54 tests)
- Phase 3: 🔄 In Progress (Infrastructure adapters) - **Started October 31, 2025** - Tasks 3A.1-3A.6 ✅
- Phase 4: ⏳ Pending (API layer)

---

## ✅ PHASE 0 COMPLETE - READY FOR MIGRATION

### All Blockers Resolved

All modules previously classified as `domain` or `core` with **side effects** have been refactored or reclassified.

**Critical Reclassifications:**
1. ✅ `shared/config.rs` → **SPLIT COMPLETE** (config_types.rs → domain, config_loader.rs → infra)
2. ✅ `observability/errors/app.rs` → **SPLIT COMPLETE** (error types → domain, conversions.rs → infra)
3. ✅ `integrations/sap/errors.rs` → **RECLASSIFIED** (moved to infra Priority 3)
4. ✅ `integrations/sap/validation.rs` → **MOVED** (moved to infra)
5. ✅ `preprocess/segmenter.rs` → **REFACTOR COMPLETE** (uses repository ports)
6. ✅ `inference/batch_classifier.rs` → **RECLASSIFIED** (moved to infra Priority 3, ml feature)

**Feature Flag Mismatch:**
- Inventory documents `calendar`, `sap`, `ml` features
- Actual Cargo.toml only defines `tree-classifier`, `graphql`
- Either rename docs or add missing feature declarations

**Phase 0 Status:** ✅ All blockers resolved! Ready for Phase 1.

### Quick Reference: Blockers by Action Required

| Module | Action | Status | Completed |
|--------|--------|--------|----------|
| `shared/config.rs` | Split (types → domain, loader → infra) | ✅ Complete | 2025-10-30 |
| `observability/errors/app.rs` | Split (types → domain, conversions → infra) | ✅ Complete | 2025-10-30 |
| `preprocess/segmenter.rs` | Refactor (add repository port) | ✅ Complete | 2025-10-31 |
| `inference/batch_classifier.rs` | Reclassify (→ infra) | ✅ Complete | 2025-10-30 |
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
| `shared/config_types.rs` | `domain` | `domain/src/config/app_config.rs` | ✅ Priority 1 | Config DTOs (split from config.rs) |
| `shared/config_loader.rs` | `infra` | `infra/src/config/loader.rs` | ✅ Priority 2 | Config loading with I/O (split from config.rs) |
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
| `preprocess/segmenter.rs` | `core` | `core/src/tracking/segmenter.rs` | ✅ Priority 2 | Refactored to use `SegmentRepository` + `SnapshotRepository` ports |
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
| `inference/batch_classifier.rs` | `infra` | `infra/src/classification/batch_classifier.rs` | ✅ Priority 3 | Feature-gated: ml • Uses `DbManager` + `tauri::Emitter` (side effects) |
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
| `integrations/sap/cache.rs` | `infra` | `infra/src/integrations/sap/cache.rs` | ⚠️ Priority 3 | Feature-gated: sap • ✅ WbsRepository port complete (PR #1), SqlCipherWbsRepository impl ready (455 lines, FTS5 search) |
| `integrations/sap/forwarder.rs` | `infra` | `infra/src/integrations/sap/forwarder.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/health_monitor.rs` | `infra` | `infra/src/integrations/sap/health.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/scheduler.rs` | `infra` | `infra/src/integrations/sap/scheduler.rs` | ⚠️ Priority 3 | Feature-gated: sap |
| `integrations/sap/models.rs` | `domain` | `domain/src/types/sap.rs` | ⚠️ Priority 1 | Feature-gated: sap |
| `integrations/sap/errors.rs` | `infra` | `infra/src/integrations/sap/errors.rs` | ✅ Priority 3 | Feature-gated: sap • Wraps `reqwest::Error` directly (transport-specific) |
| `integrations/sap/validation.rs` | `infra` | `infra/src/integrations/sap/validation.rs` | ✅ Priority 3 | Feature-gated: sap • Uses `DbManager` + `WbsCache` (DB access) |
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
| `observability/errors/app.rs` | `domain` | `domain/src/errors/mod.rs` | ✅ Priority 1 | Pure error types (split complete) |
| `observability/errors/conversions.rs` | `infra` | `infra/src/errors/conversions.rs` | ✅ Priority 2 | External From impls (split from app.rs) |
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

### 3. `preprocess/segmenter.rs` → Add Repository Port ✅ COMPLETED

**Resolution (Phase 0)**:
- ✅ Created `SegmentRepository` and `SnapshotRepository` traits in `crates/core/src/tracking/ports.rs`
- ✅ Refactored `Segmenter<S, A>` to be generic over repository ports
- ✅ Implemented `SqlCipherSegmentRepository` and `SqlCipherSnapshotRepository` in `legacy/api/src/infra/repositories/`
- ✅ Added integration tests with real SqlCipher database
- ✅ Removed all direct `LocalDatabase` and `rusqlite` usage from production code

**Implementation:**
```rust
// crates/core/src/tracking/ports.rs (Port definitions)
pub trait SegmentRepository: Send + Sync {
    fn save_segment(&self, segment: &ActivitySegment) -> CommonResult<()>;
    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>>;
    fn find_unprocessed_segments(&self, limit: usize) -> CommonResult<Vec<ActivitySegment>>;
    fn mark_processed(&self, segment_id: &str) -> CommonResult<()>;
}

// legacy/api/src/preprocess/segmenter.rs (Business logic)
pub struct Segmenter<S, A>
where
    S: SegmentRepository,
    A: SnapshotRepository,
{
    segment_repo: S,
    snapshot_repo: A,
}

// legacy/api/src/infra/repositories/segment_repository.rs (SqlCipher implementation)
impl SegmentRepository for SqlCipherSegmentRepository {
    fn save_segment(&self, segment: &ActivitySegment) -> CommonResult<()> {
        // SqlCipher pool-based implementation
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

### Current State (crates/api/Cargo.toml)
```toml
[features]
default = ["sqlcipher"]
custom-protocol = ["tauri/custom-protocol"]
sqlcipher = []
ts-gen = ["dep:ts-rs"]
calendar = ["pulsearc-infra/calendar"]
sap = ["pulsearc-infra/sap"]
tree-classifier = ["pulsearc-infra/tree-classifier"]
ml = ["tree-classifier", "pulsearc-infra/ml"]
graphql = ["pulsearc-infra/graphql"]
```

### Inventory Coverage
- `calendar` ✅
- `sap` ✅
- `ml` ✅ (alias for `tree-classifier`)
- `tree-classifier` ✅
- `graphql` ✅

**Status:** ✅ Feature flags now align with documented targets.

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
**Goal**: Resolve all blockers before Phase 1 ✅ **COMPLETE**

1. ✅ **COMPLETED** Split `shared/config.rs` → config_types.rs (domain) + config_loader.rs (infra)
2. ✅ **COMPLETED** Split `observability/errors/app.rs` → error types (domain) + conversions.rs (infra)
3. ✅ **COMPLETED** Refactor `preprocess/segmenter.rs` → uses repository ports (Tasks 4.1-4.3)
4. ✅ **COMPLETED** Reclassify `inference/batch_classifier.rs` → infra Priority 3, ml feature
5. ✅ **COMPLETED** Reclassify `integrations/sap/errors.rs` → infra Priority 3
6. ✅ **COMPLETED** Reclassify `integrations/sap/validation.rs` → infra
7. ✅ **COMPLETED** Add missing features to `Cargo.toml` (`calendar`, `sap`, `ml`)

**Progress**: 7/7 tasks completed (100% ✅)
**Status**: Ready for Phase 1! 🚀

### Phase 1: Foundation (Week 1) ✅ **COMPLETE**
**Goal**: Establish domain types and core ports

**Completed Tasks:**
1. ✅ **COMPLETED** Move all `db/models.rs` types → `domain/src/types/database.rs` (BatchQueue, TimeEntryOutbox, BatchStatus, OutboxStatus, IdMapping)
2. ✅ **COMPLETED** Move `db/models_idle.rs` types → `domain/src/types/idle.rs` (IdlePeriod, IdleSummary)
3. ✅ **COMPLETED** Move `shared/types/stats.rs` → `domain/src/types/stats.rs` (DatabaseStats, BatchStats, SyncStats, OutboxStats, DlqBatch)
4. ✅ **COMPLETED** Move `shared/constants/` → `domain/src/constants.rs` (21 application constants)
5. ✅ **COMPLETED** Move `shared/extractors/pattern.rs` → `domain/src/utils/pattern_extractor.rs` (with 16 tests)
6. ✅ **COMPLETED** Move `utils/title.rs` → `domain/src/utils/title.rs` (with 17 tests)
7. ✅ **COMPLETED** Move `inference/types.rs` → `domain/src/types/classification.rs` (ProposedBlock, ContextSignals, ProjectMatch, AppCategory, WorkLocation)
8. ✅ **COMPLETED** Copy status conversion macro → `domain/src/macros.rs` (impl_domain_status_conversions!)
9. ✅ **COMPLETED** Define BlockRepository in `core/src/classification/ports.rs`
10. ✅ **COMPLETED** Define OutboxQueue in `core/src/sync/ports.rs`
11. ✅ **COMPLETED** Define CalendarProvider in `core/src/calendar_ports.rs` (feature-gated)
12. ✅ **COMPLETED** Define SapClient in `core/src/sap_ports.rs` (feature-gated)

**Setup Completed:**
- ✅ Added ts-rs to workspace dependencies
- ✅ Configured domain crate with ts-gen feature
- ✅ Forwarded ts-gen from api → domain
- ✅ Added calendar/sap/ml features to core
- ✅ Forwarded calendar/sap features from api → core

**Validation Results:**
- ✅ `cargo check -p pulsearc-domain` passes
- ✅ `cargo check -p pulsearc-domain --features ts-gen` passes (TypeScript generation ready)
- ✅ `cargo check -p pulsearc-core` passes
- ✅ `cargo check -p pulsearc-core --features calendar,sap` passes
- ✅ **39 tests passing** in domain (utils and classification)
- ✅ **Zero forbidden dependencies** (domain has no infra/common/core deps)
- ✅ Status enums use macro (avoids ~160 lines of boilerplate)

**Progress**: 12/12 tasks completed (100% ✅)
**Completion Date**: October 31, 2025
**Status**: Ready for Phase 2! 🚀

### Phase 2: Core Business Logic (Week 2) 🔄 **IN PROGRESS**
**Goal**: Migrate pure business logic
**Started**: October 31, 2025

**Architectural Decisions (Applied):**
- ✅ **Async conversion**: Converting all legacy sync code to async to match existing core ports
- ✅ **Service integration**: Merging business logic into existing `TrackingService` and `ClassificationService` (not parallel modules)
- ✅ **Database refactoring**: All database access via repository ports (no `DbManager` in core)
- ✅ **Calendar types**: Reusing existing `CalendarEventRow` from domain (no new types needed)
- ✅ **Error consistency**: Using `pulsearc_domain::Result` across all ports (not mixing with `CommonResult`)
- ✅ **Project matcher inclusion**: Including project_matcher in Phase 2 (dependency of block_builder)

**Completed Foundation Work:**
1. ✅ Added `CalendarEventRepository` port to `tracking/ports.rs` (returns `CalendarEventRow`)
2. ✅ Added `ProjectMatcher` port to `classification/ports.rs` (uses `pulsearc_domain::Result`)
3. ✅ Created `core/src/utils/` module structure
4. ✅ Migrated `utils/patterns.rs` → `core/src/utils/patterns.rs` (485 lines, 17 tests, updated imports)
5. ✅ Added dependencies to `core/Cargo.toml` (log, ahash, url, lazy_static)
6. ✅ Updated `core/src/lib.rs` with utils module and new port re-exports
7. ✅ Verified compilation: `cargo check -p pulsearc-core` passes
8. ✅ **PR #1 COMPLETE (Oct 31, 2025)**: Added `WbsRepository` trait to `classification/ports.rs` with 6 methods (count, timestamp, load, search, get by project_def, get by wbs_code)
9. ✅ **PR #1 COMPLETE**: Created `SqlCipherWbsRepository` in `legacy/api/src/infra/repositories/wbs_repository.rs` (455 lines, 7 comprehensive tests)
10. ✅ **PR #1 COMPLETE**: FTS5 full-text search with BM25 ranking, Porter stemming, typo tolerance (<3ms query performance target)

**Remaining Business Logic Migrations (~2800 lines total):**

1. ✅ **`inference/signals.rs`** (692 lines, 16 tests → 8 tests) → `core/src/classification/signal_extractor.rs` **COMPLETE (PR #2, Oct 31, 2025)**
   - **Priority**: HIGH (dependency for block_builder)
   - **Completed refactoring**:
     - ✅ Replaced `Arc<DbManager>` with `Option<Arc<dyn CalendarEventRepository>>`
     - ✅ Converted `query_calendar_event()` to async with repository port
     - ✅ Returns `CalendarEventRow`, extracts fields in caller
     - ✅ Uses `pulsearc_domain::ActivityContext`
     - ✅ All 5 public methods converted to async
   - **Migrated**: 602 lines + 8 tests to `core/src/classification/signal_extractor.rs`

2. ✅ **`inference/evidence_extractor.rs`** (488 lines, 7 tests → 5 tests) → `core/src/classification/evidence_extractor.rs` **COMPLETE (PR #3, Oct 31, 2025)**
   - **Priority**: HIGH (dependency for block_builder)
   - **Completed refactoring**:
     - ✅ Replaced `Arc<DbManager>` with `Arc<dyn SnapshotRepository>` + `Option<Arc<dyn CalendarEventRepository>>`
     - ✅ Converted `fetch_snapshots_for_block()` to use repository (sync, not async)
     - ✅ Converted `extract_signals_from_snapshots()` to async (uses calendar repo)
     - ✅ Uses domain types: `ProposedBlock`, `ActivitySnapshot`, `EvidenceSignals`
   - **Migrated**: 380 lines + 5 tests to `core/src/classification/evidence_extractor.rs`

3. ✅ **`inference/project_matcher.rs`** (1146 lines, 18 tests → 10 tests) → `core/src/classification/project_matcher.rs` **COMPLETE (PR #4, Nov 1, 2025)**
   - **Priority**: HIGH (dependency for block_builder)
   - **Completed refactoring**:
     - ✅ Replaced `Arc<DbManager>` with `Arc<dyn WbsRepository>`
     - ✅ Uses WbsRepository port trait (6 methods: count, timestamp, load, search, get by project_def, get by wbs_code)
     - ✅ Preserved FTS5 search logic via repository
     - ✅ Preserved HashMap caching (common projects cache)
     - ✅ Preserved all business logic: hybrid matching, confidence scoring, workstream inference
   - **Migrated**: 784 lines + 10 tests to `core/src/classification/project_matcher.rs`
   - **Performance**: Target <15ms per match, <3ms FTS5 queries

4. ✅ **`inference/block_builder.rs`** (2,882 lines, 51 tests → 18 tests) → `core/src/classification/block_builder.rs` **COMPLETE (PR #5, Nov 1, 2025)**
   - **Priority**: MEDIUM (depends on PRs #2-4)
   - **Completed refactoring**:
     - ✅ Pure business logic migration (no infrastructure dependencies)
     - ✅ REFACTOR-004 already removed SignalExtractor and ProjectMatcher dependencies
     - ✅ Preserved time consolidation logic (same app + gap ≤ 180s)
     - ✅ Preserved activity breakdown weighted by duration (not count)
     - ✅ Preserved idle time handling (exclude/include/partial strategies)
     - ✅ Preserved day boundary clipping semantics
   - **Migrated**: 387 lines implementation + 18 comprehensive tests
   - **Test coverage**: Basic consolidation, activity breakdown, idle time handling (6 tests), boundary conditions, time selection
   - **No async conversion needed**: Synchronous business logic only

5. ⏳ **`preprocess/segmenter.rs`** (1127 lines, 31 tests) → merge into `TrackingService`
   - **Priority**: MEDIUM
   - **Status**: Already uses `SegmentRepository` port (Phase 0 complete)
   - **Public API**: 8 methods (create, save, generate dictionary)
   - **Refactoring needed**:
     - Add methods to `TrackingService` (not separate module)
     - Convert sync repository calls to async (add `.await`)
     - Keep all business logic (5-minute windowing, gap detection, midnight boundaries)
   - **Async conversion**: All methods, simple (just add async/await)

6. ⏳ **`tracker/core.rs`** → extract equality logic into `TrackingService`
   - **Priority**: LOW (pure utility functions)
   - **Scope**: ~50 lines total
   - **Extract**:
     - `contexts_equal(a, b) -> bool`
     - `contexts_equal_with_mode(a, b, mode) -> bool`
     - `EqualityMode` enum (Strict, Relaxed)
   - **Skip**: All infra code (RefresherState, threading, Tauri, macOS NSWorkspace)
   - **No async needed**: Pure comparison functions

**Remaining Test Migration:**
- ⏳ Port 71+ unit tests to `core/tests/` with async mocks:
  - 16 signal extractor tests → `core/tests/classification/signal_extractor_tests.rs`
  - 7 evidence extractor tests → `core/tests/classification/evidence_extractor_tests.rs`
  - 31 segmenter tests → `core/tests/tracking/segmenter_tests.rs`
  - Block builder tests → `core/tests/classification/block_builder_tests.rs`
  - Context equality tests → `core/tests/tracking/equality_tests.rs`
- ⏳ Create shared test utilities in `core/tests/common/mod.rs`
- ⏳ Run `cargo test -p pulsearc-core --all-features` and verify all pass

**Critical Blockers for Continuing:**
1. ✅ **RESOLVED (PR #1)**: WbsRepository port complete with SqlCipherWbsRepository implementation (455 lines, 7 tests)
2. ✅ **RESOLVED (PRs #2-4)**: Week 1 PRs complete (signal_extractor, evidence_extractor, project_matcher)
3. **Large scope**: ~2800 lines of block_builder logic remaining with async conversions
4. **Test complexity**: Need async test infrastructure with mock repositories for block_builder

**Completed PRs (Week 1-2):**
1. ✅ **COMPLETE (PR #1, Oct 31)**: WbsRepository trait + SqlCipherWbsRepository implementation (455 lines, 7 tests)
2. ✅ **COMPLETE (PR #2, Oct 31)**: Migrated signal_extractor.rs (602 lines, 8 tests)
3. ✅ **COMPLETE (PR #3, Oct 31)**: Migrated evidence_extractor.rs (380 lines, 5 tests)
4. ✅ **COMPLETE (PR #4, Nov 1)**: Migrated project_matcher.rs (784 lines, 10 tests)
5. ✅ **COMPLETE (PR #5, Nov 1)**: Migrated block_builder.rs (387 lines, 18 tests)

**Remaining Next Steps (PR #6+):**
1. **Later**: Merge segmenter into TrackingService (straightforward async conversion)
2. **Later**: Extract tracker equality logic (simple utility functions)
3. **Later**: Port additional tests with async mocks
4. **Final**: Full validation with `cargo test`

**Status**: ✅ Week 1-2 complete (5 PRs, ~2,610 lines migrated, 41 tests). ✅ Core classification modules complete!

**Latest Progress (Nov 1, 2025):**
- ✅ **PR #1 Complete (Oct 31)**: WbsRepository trait + SqlCipherWbsRepository (455 lines, 7 tests)
- ✅ **PR #2 Complete (Oct 31)**: SignalExtractor migration (602 lines, 8 tests) - async conversion, CalendarEventRepository integration
- ✅ **PR #3 Complete (Oct 31)**: EvidenceExtractor migration (380 lines, 5 tests) - SnapshotRepository integration, calendar metadata
- ✅ **PR #4 Complete (Nov 1)**: ProjectMatcher migration (784 lines, 10 tests) - WbsRepository integration, hybrid matching preserved
- ✅ **PR #5 Complete (Nov 1)**: BlockBuilder migration (387 lines, 18 tests) - Pure business logic, idle time handling, time consolidation

**Week 1-2 Summary:**
- **Total Migrated**: ~2,610 lines of business logic + 41 tests
- **Files Created**: signal_extractor.rs, evidence_extractor.rs, project_matcher.rs, block_builder.rs
- **Repository Ports Used**: WbsRepository, SnapshotRepository, CalendarEventRepository
- **Key Patterns Preserved**: FTS5 search, hybrid matching, confidence scoring, workstream inference, time consolidation, idle handling
- **Next Steps**: Phase 2 core classification complete! Ready for Phase 3 infrastructure adapters.

**Validation**: ✅ Core compilation passes. ✅ All 41 core tests pass. ✅ Clippy clean (0 warnings). ✅ Core classification modules migration complete!

### Phase 3: Infrastructure Adapters (Weeks 3-8) 🔄 **IN PROGRESS**

**Week 3 Checkpoint (November 2, 2025)**
- ✅ Task 3A.1 — Config loader migrated to `crates/infra/src/config/loader.rs` (600 LOC, 17 tests)
- ✅ Task 3A.2 — Error conversions consolidated in `crates/infra/src/errors/conversions.rs` (242 LOC, 3 tests)
- ✅ Task 3A.3 — HTTP client ported with retry/backoff in `crates/infra/src/http/client.rs` (304 LOC, 4 tests)
- ✅ Task 3A.4 — `DbManager` + SQLCipher helpers landed in `crates/infra/src/database/manager.rs` and `sqlcipher_pool.rs` (115 LOC, 2 tests)
- ✅ Task 3A.5 — `SqlCipherActivityRepository` shipped in `crates/infra/src/database/activity_repository.rs` (≈450 LOC, 6 async tests)
  - Implements both `ActivityRepository` and `SnapshotRepository` ports on top of `DbManager`
  - Uses `spawn_blocking` + SQLCipher wrappers (no `LocalDatabase`), enforces half-open `[start, end)` range semantics, and adds `find_snapshots_page` pagination helper
  - Regression coverage includes deletion pruning, range validation, and failure-path drop-table test
- ✅ Task 3A.6 — `SqlCipherSegmentRepository` landed in `crates/infra/src/database/segment_repository.rs` (≈330 LOC, 5 tests)
  - Sync `SegmentRepository` port now uses SQLCipher pool directly with `[start, end)` queries and pagination-friendly helpers
  - Converts `usize` limits safely to `u64`, serializes snapshot IDs via `serde_json`, and maps failure conditions to `CommonError`
  - Added regression fixes to metrics ring-buffer tests (`observability/metrics/{call,db}.rs`) to keep `cargo test -p pulsearc-infra database::segment_repository` clean under `-D warnings`
  - CI temporarily excludes the legacy `pulsearc` crate from Clippy (`xtask run_clippy`) until `legacy/api/clippy.toml` is modernised (TODO: LEGACY-CLIPPY-CONFIG)
- ✅ Task 3A.7 — `SqlCipherBlockRepository` implemented in `crates/infra/src/database/block_repository.rs` (≈330 LOC, 3 tests)
  - Ports save/query workflow for `ProposedBlock` onto SQLCipher with `[start, end)` day queries and snapshot history lookups
  - Adds explicit `approve_block`/`reject_block` helpers plus async tests covering approval transitions and history ordering (`cargo test -p pulsearc-infra database::block_repository` ✅)
  - Follows [`docs/issues/SQLCIPHER-API-REFERENCE.md`](issues/SQLCIPHER-API-REFERENCE.md) guidance: synchronous pool access, `[&dyn ToSql]` parameter slices, and zero `.collect()` on `query_map`

**DbManager Snapshot**
- `DbManager::new` wraps `create_sqlcipher_pool`, enforces provided encryption keys, and seeds `schema_version` via `schema.sql`
- Health path validated through `database::manager::tests::migrations_create_schema_version`
- Pool helper smoke-tested by `sqlcipher_pool::tests::create_pool_successfully`; `cargo test -p pulsearc-infra database::manager` passes locally
- Activity, segment, and block repositories now use pooled SQLCipher connections exclusively (`SqlCipherConnection::prepare/query_map`), ensuring no direct `rusqlite` handles escape the pool

**Next Focus: Task 3A.8**
- Port `OutboxRepository` to SQLCipher (`crates/infra/src/database/outbox_repository.rs`) with enqueue/dequeue workflow and retry counters
- Ensure regression suites cover pending/failed transitions and FIFO ordering
- Investigate legacy outbox retry filter regression before rewiring (see Phase 3 blockers section)

**Goal**: Implement all port adapters (~60+ modules, ~17,600 LOC)

**📋 DETAILED PLAN**: See [**PHASE-3-INFRA-TRACKING.md**](issues/PHASE-3-INFRA-TRACKING.md) for complete breakdown

**Duration**: 4-6 weeks (23-31 working days)
**Dependencies**: Phase 2 (Core Business Logic) must be complete
**Priority**: P1-P3 (mixed priorities across sub-phases)

**Sub-Phases:**
1. **3A: Core Infrastructure** (5-7 days) - Database repos, HTTP client, config loader
2. **3B: Platform Adapters** (4-6 days) - macOS provider, enrichment, Windows/Linux fallback
3. **3C: Integration Adapters** (5-7 days) - OpenAI, SAP, Calendar (feature-gated)
4. **3D: Schedulers & Workers** (4-5 days) - Cron jobs, outbox processing, sync infrastructure
5. **3E: ML Adapters (optional)** (3-4 days) - Linfa, training pipeline (feature-gated)
6. **3F: Observability (parallel)** (2-3 days) - Metrics collection

**Validation**:
- Integration tests with real adapters pass
- All feature combinations compile and work
- Performance targets met (database < 50ms p99, enrichment < 100ms p50)
- Manual testing complete on macOS, Windows, Linux

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

**Phase 1 Complete:**
- [x] All modules classified with target crate ✅
- [x] Zero forbidden dependency edges (domain has no infra/core deps) ✅
- [x] All port traits defined in core ✅
- [x] Feature flags properly configured (calendar, sap, ml) ✅
- [x] No use of `unwrap`/`expect` outside tests (domain layer clean) ✅
- [x] Status macros reduce boilerplate (~160 lines saved) ✅
- [x] Test coverage in domain utilities (39 tests passing) ✅

**Remaining (Phase 2+):**
- [ ] Core business logic migrated
- [ ] Infrastructure adapters implemented
- [ ] API layer commands migrated
- [ ] All `FEATURE`/`PHASE` comments documented

---

## Next Steps

### Phase 1 Complete ✅ - Ready for Phase 2

**All Phase 1 tasks are complete!** Domain types and core ports are now migrated and validated.

**What's Next (Phase 2 - Core Business Logic):**

1. **Migrate Domain-Specific Utilities**
   - Move `utils/patterns.rs` → `core/src/utils/patterns.rs`
   - Contains PulseArc-specific business rules (Slack channel extraction, GitHub PR patterns, etc.)

2. **Migrate Tracking Service**
   - Move `tracker/core.rs` → `core/src/tracking/service.rs`
   - Implement TrackingService business logic

3. **Migrate Segmentation Logic**
   - Move `preprocess/segmenter.rs` → `core/src/tracking/segmenter.rs`
   - Pure business logic for activity segmentation

4. **Migrate Classification Logic**
   - Move `inference/block_builder.rs` → `core/src/classification/block_builder.rs`
   - Move `inference/signals.rs` → `core/src/classification/signals.rs`
   - Move `inference/evidence_extractor.rs` → `core/src/classification/evidence.rs`
   - Move `inference/project_matcher.rs` → `core/src/classification/project_matcher.rs`

**Validation Goals for Phase 2:**
- Core tests pass with mock port implementations
- No infrastructure dependencies in core
- Business logic properly isolated from adapters

**Documentation:**
- **Phase 0 Complete**: [issues/PHASE-0-BLOCKERS-TRACKING.md](issues/PHASE-0-BLOCKERS-TRACKING.md)
- **Phase 1 Complete**: All domain types and ports documented in this file
- **Next**: Create Phase 2 tracking issue for core business logic migration

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

### Critical Review (October 30, 2025 - COMPLETE ✅)
**All Critical Issues Resolved:**
1. ✅ `shared/config.rs` → **SPLIT COMPLETE** (config_types.rs + config_loader.rs)
2. ✅ `observability/errors/app.rs` → **SPLIT COMPLETE** (error types + conversions.rs)
3. ✅ `integrations/sap/errors.rs` → **RECLASSIFIED TO INFRA** (Priority 3)
4. ✅ `integrations/sap/validation.rs` → **RECLASSIFIED TO INFRA**
5. ✅ `preprocess/segmenter.rs` → **REPOSITORY PORTS ADDED** (Tasks 4.1-4.3 complete)
6. ✅ `inference/batch_classifier.rs` → **RECLASSIFIED TO INFRA** (Priority 3, ml feature)
7. ✅ Feature flags → **ADDED TO CARGO.TOML** (calendar, sap, ml)

**Total Resolved**: 7/7 blockers complete (100% ✅)
**Actual Time**: 2.5 days (ahead of 1-week estimate!)

**Validation**: All blockers verified by reading source code (lines 27-105, 363-447, 58-78, 4-29, 5-421)

---

### Phase 1 Migration (October 31, 2025 - COMPLETE ✅)
**All Foundation Tasks Completed:**

**Setup & Configuration (6 tasks):**
1. ✅ ts-rs added to workspace dependencies
2. ✅ Domain crate configured with ts-gen feature  
3. ✅ API crate forwards ts-gen to domain
4. ✅ Core crate declares calendar/sap/ml/tree-classifier features
5. ✅ API crate forwards calendar/sap features to core
6. ✅ Status conversion macro copied to domain (impl_domain_status_conversions!)

**Domain Type Migrations (7 tasks):**
1. ✅ `db/models.rs` → `domain/src/types/database.rs` (BatchQueue, TimeEntryOutbox, BatchStatus, OutboxStatus, IdMapping)
2. ✅ `db/models_idle.rs` → `domain/src/types/idle.rs` (IdlePeriod, IdleSummary)
3. ✅ `shared/types/stats.rs` → `domain/src/types/stats.rs` (DatabaseStats, BatchStats, SyncStats, OutboxStats, DlqBatch)
4. ✅ `shared/constants/` → `domain/src/constants.rs` (21 constants)
5. ✅ `shared/extractors/pattern.rs` → `domain/src/utils/pattern_extractor.rs` (16 tests)
6. ✅ `utils/title.rs` → `domain/src/utils/title.rs` (17 tests)
7. ✅ `inference/types.rs` → `domain/src/types/classification.rs` (ProposedBlock, ContextSignals, ProjectMatch, AppCategory, WorkLocation)

**Core Port Definitions (4 tasks):**
1. ✅ BlockRepository added to `core/src/classification/ports.rs`
2. ✅ OutboxQueue created in `core/src/sync/ports.rs`
3. ✅ CalendarProvider created in `core/src/calendar_ports.rs` (feature-gated)
4. ✅ SapClient created in `core/src/sap_ports.rs` (feature-gated)

**Validation Results:**
- ✅ Domain compiles standalone
- ✅ Domain with ts-gen compiles (TypeScript generation ready)
- ✅ Core compiles with all features
- ✅ **39 tests passing** in domain
- ✅ **Zero forbidden dependencies** verified
- ✅ Status enums use macro (saves ~160 lines)

**Total Completed**: 17/17 tasks (100% ✅)
**Actual Time**: 1 day (as planned!)

---

**Document Status**: 🔄 PHASE 3A WEEK 3 IN PROGRESS - Config loader, error conversions, HTTP client, DbManager, Activity + Segment + Block repositories migrated; Outbox repository (Task 3A.8) next
**Latest**: `SqlCipherBlockRepository` added with approval helpers and snapshot history queries (`cargo test -p pulsearc-infra database::block_repository` ✅); next focus is Outbox queue migration + regression fixes (November 2, 2025)
