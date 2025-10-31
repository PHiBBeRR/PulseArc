# Legacy Code Migration Inventory

**Generated**: October 31, 2025  
**Purpose**: Classify all `legacy/api/src/` modules by target crate for ADR-003 migration  
**Status**: DRAFT - Awaiting Review

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

### Core (`pulsearc-core`)
**Criteria**: Business logic, hexagonal ports (traits), use cases
- Service implementations
- Port trait definitions
- Business rule validation
- Domain event handling

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
| `shared/config.rs` | `domain` | `domain/src/config/app_config.rs` | ✅ Priority 1 | AppConfig, DatabaseConfig, TrackingConfig |
| `shared/constants/mod.rs` | `domain` | `domain/src/constants.rs` | ✅ Priority 1 | Application constants |
| `shared/auth/` | `infra` | `infra/src/auth/` | ⚠️ Priority 3 | OAuth implementation (feature-gated) |
| `shared/cache.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::cache` instead |
| `shared/extractors/pattern.rs` | `core` | `core/src/utils/pattern.rs` | ✅ Priority 2 | Pure pattern matching logic |
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
| `preprocess/segmenter.rs` | `core` | `core/src/tracking/segmenter.rs` | ✅ Priority 2 | Activity segmentation logic (if pure) |
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
| `inference/batch_classifier.rs` | `core` | `core/src/classification/batch_classifier.rs` | ✅ Priority 2 | Batch classification orchestration |
| `inference/scheduler.rs` | `infra` | `infra/src/scheduling/block_scheduler.rs` | ✅ Priority 3 | Scheduler implementation |
| `inference/classification_scheduler.rs` | `infra` | `infra/src/scheduling/classification_scheduler.rs` | ✅ Priority 3 | Classification scheduler |
| `inference/timezone_utils.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::time` instead |
| `inference/openai_types.rs` | `domain` | `domain/src/types/openai.rs` | ✅ Priority 2 | OpenAI API types |
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
| `integrations/sap/errors.rs` | `domain` | `domain/src/errors/sap_error.rs` | ⚠️ Priority 1 | Feature-gated: sap |
| `integrations/sap/validation.rs` | `core` | `core/src/integrations/sap_validation.rs` | ⚠️ Priority 2 | Feature-gated: sap |
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
| `observability/errors/app.rs` | `domain` | `domain/src/errors/app_error.rs` | ✅ Priority 1 | Application errors |
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
| `utils/patterns.rs` | `core` | `core/src/utils/patterns.rs` | ✅ Priority 2 | Pure pattern utilities |
| `utils/title.rs` | `core` | `core/src/utils/title.rs` | ✅ Priority 2 | Title parsing |
| **Tooling** |
| `tooling/macros/status_enum.rs` | **`common`** | N/A | ❌ Excluded | Use `pulsearc_common::impl_status_conversions!` macro |

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
    async fn classify(&self, snapshots: Vec<ActivitySnapshot>) -> Result<Vec<ProposedBlock>>;
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

### Phase 1: Foundation (Week 1)
**Goal**: Establish domain types and core ports

1. Move all `db/models.rs` types → `domain/src/types/`
2. Move `shared/types/` → `domain/src/types/`
3. Move `shared/config.rs` → `domain/src/config/`
4. Move `inference/types.rs` → `domain/src/types/classification.rs`
5. Define all port traits in `core/src/*/ports.rs`

**Validation**: `cargo check --package pulsearc-domain` passes with zero infra deps

### Phase 2: Core Business Logic (Week 2)
**Goal**: Migrate pure business logic

1. Move `tracker/core.rs` → `core/src/tracking/service.rs`
2. Move `preprocess/segmenter.rs` → `core/src/tracking/segmenter.rs`
3. Move `inference/block_builder.rs` → `core/src/classification/block_builder.rs`
4. Move `inference/signals.rs` → `core/src/classification/signals.rs`
5. Move `inference/evidence_extractor.rs` → `core/src/classification/evidence.rs`

**Validation**: Core tests pass with mock port implementations

### Phase 3: Infrastructure Adapters (Week 3-4)
**Goal**: Implement all port adapters

1. Database repositories (`db/activity/`, `db/blocks/`, `db/outbox/`)
2. Platform providers (`tracker/providers/macos.rs`, `tracker/os_events/`)
3. Integration adapters (calendar, SAP) behind feature flags
4. ML adapters (linfa, training) behind feature flags

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

## Open Questions

1. **Detection Packs**: Should industry-specific packs (consulting, deals, finance, etc.) be in `core` or feature-gated in `infra`?
   - **Recommendation**: Start in `core`, move to feature flags if they grow large

2. **Scheduler Placement**: Should schedulers be in `infra` or `api`?
   - **Recommendation**: `infra` (they're adapters for cron-like functionality)

3. **Error Hierarchy**: How should domain errors compose with `CommonError`?
   - **Recommendation**: Follow pattern in Common Crates Guide (module errors compose via `#[from]`)

4. **Test Migration**: Should tests move with code or stay in integration tests?
   - **Recommendation**: Unit tests move with code, add integration tests in `api/tests/`

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

1. **Review & Approval**: Stakeholder review of this inventory
2. **Create Port Traits**: Define all traits in `core` before migration
3. **Week-by-Week PRs**: Small, incremental migrations with tests
4. **CI Updates**: Add dependency graph validation
5. **Documentation**: Update ADR-002 with migration notes

---

**Document Status**: DRAFT - Awaiting confirmation before proceeding with migration

