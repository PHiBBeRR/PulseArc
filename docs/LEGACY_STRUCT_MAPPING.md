# Legacy Struct & Type Mapping

**Generated**: October 31, 2025
**Purpose**: Comprehensive mapping of all data structures in `legacy/api/src/` for migration planning
**Related**: LEGACY_MIGRATION_INVENTORY.md

---

## Executive Summary

This document catalogs **200+ structs and 50+ enums** across the legacy codebase, identifying:
- **Core Domain Types**: ActivitySnapshot, ActivitySegment, ProposedBlock, TimeEntryOutbox
- **Shared Types**: Used across multiple modules (high migration priority)
- **Feature-Gated Types**: Calendar, SAP, ML-specific structures
- **Infrastructure Types**: HTTP clients, OAuth, schedulers, error types

**Key Findings**:
- 15+ types are **heavily shared** across modules (need domain layer)
- 30+ types are **feature-specific** (require feature flags)
- 40+ types are **database-bound** (rusqlite conversions needed)
- 25+ types are **TS-exported** for frontend integration

---

## Table of Contents

1. [Core Domain Models](#core-domain-models)
2. [Shared Types Analysis](#shared-types-analysis)
3. [Complete Struct Inventory](#complete-struct-inventory)
4. [Enum Inventory](#enum-inventory)
5. [Type Dependencies](#type-dependencies)
6. [Migration Priorities](#migration-priorities)

---

## Core Domain Models

### Primary Data Structures (Priority 1)

These types form the **backbone of the domain model** and are referenced across all layers:

| Struct | Location | Target Crate | Used By | Exported to TS? |
|--------|----------|--------------|---------|-----------------|
| **`ActivitySnapshot`** | `db/models.rs` | `domain/types/database.rs` | tracker, preprocess, inference, commands | ✅ Yes |
| **`ActivitySegment`** | `db/models.rs` | `domain/types/database.rs` | segmenter, block_builder, inference | ✅ Yes |
| **`ActivityContext`** | `shared/types/mod.rs` | `domain/types/activity.rs` | tracker, preprocess, signals | ✅ Yes |
| **`WindowContext`** | `shared/types/mod.rs` | `domain/types/activity.rs` | tracker, enrichers, signals | ✅ Yes |
| **`ProposedBlock`** | `inference/types.rs` | `domain/types/classification.rs` | block_builder, batch_classifier, commands | ✅ Yes |
| **`ContextSignals`** | `inference/types.rs` | `domain/types/classification.rs` | signals, project_matcher, block_builder | ❌ No (internal) |
| **`ProjectMatch`** | `inference/types.rs` | `domain/types/classification.rs` | project_matcher, block_builder | ❌ No (internal) |
| **`TimeEntryOutbox`** | `db/models.rs` | `domain/types/database.rs` | outbox_worker, sync, commands | ✅ Yes |
| **`BatchQueue`** | `db/models.rs` | `domain/types/database.rs` | batch ops, scheduler, commands | ✅ Yes |
| **`IdlePeriod`** | `db/models.rs` | `domain/types/idle.rs` | idle detector, period tracker, commands | ✅ Yes |
| **`IdleSummary`** | `db/models.rs` | `domain/types/idle.rs` | idle detector, commands | ✅ Yes |

### Configuration Types (Priority 1)

| Struct | Location | Target Crate | Purpose |
|--------|----------|--------------|---------|
| **`Config`** | `shared/config.rs` | `domain/config/app_config.rs` | App-wide configuration (cache, debug, packs) |
| **`PackConfig`** | `shared/config.rs` | `domain/config/app_config.rs` | Detector pack configuration |
| **`IdleConfig`** | `tracker/idle/config.rs` | `domain/config/idle_config.rs` | Idle detection thresholds |
| **`RecoveryConfig`** | `tracker/idle/config.rs` | `domain/config/idle_config.rs` | Circuit breaker settings |
| **`BlockConfig`** | `inference/types.rs` | `domain/config/block_config.rs` | Block building parameters |

---

## Shared Types Analysis

### Heavily Shared Types (Used in 3+ Modules)

These types are **referenced across multiple layers** and require careful migration coordination:

#### 1. `ActivityContext` (9 references)
**Used by**:
- `tracker/core.rs` (creation)
- `tracker/providers/macos.rs` (platform capture)
- `preprocess/segmenter.rs` (processing)
- `preprocess/redact.rs` (PII redaction)
- `inference/signals.rs` (signal extraction)
- `inference/block_builder.rs` (block construction)
- `db/activity/snapshots.rs` (persistence)
- `commands/blocks.rs` (API layer)
- `commands/database.rs` (stats)

**Fields**: 13 total
- `active_app`: WindowContext
- `recent_apps`: Vec<WindowContext>
- `detected_activity`: String
- `work_type`: Option<WorkType>
- `activity_category`: ActivityCategory
- `billable_confidence`: f32
- `suggested_client`, `suggested_matter`, `suggested_task_code`: Option<String>
- `extracted_metadata`: ActivityMetadata
- `evidence`: ConfidenceEvidence
- `calendar_event`: Option<CalendarEventContext>
- `location`: Option<LocationContext>
- `temporal_context`: Option<TemporalContext>
- `classification`: Option<ClassificationContext>

**Migration Impact**: HIGH - Core type serialized in ActivitySnapshot

---

#### 2. `ActivitySegment` (7 references)
**Used by**:
- `preprocess/segmenter.rs` (creation)
- `db/activity/segments.rs` (persistence)
- `inference/signals.rs` (signal extraction)
- `inference/block_builder.rs` (aggregation)
- `inference/batch_classifier.rs` (batch processing)
- `commands/blocks.rs` (retrieval)
- `commands/database.rs` (stats)

**Fields**: 14 total (includes REFACTOR-003 and FEATURE-028 additions)
- Core: `id`, `start_ts`, `end_ts`, `primary_app`, `normalized_label`, `sample_count`
- Traceability: `snapshot_ids`, `dictionary_keys`, `created_at`, `processed`
- Classification: `work_type`, `activity_category`, `detected_activity`
- Inference: `extracted_signals_json`, `project_match_json`
- Idle tracking: `idle_time_secs`, `active_time_secs`, `user_action`

**Migration Impact**: HIGH - Links snapshots to blocks, stores serialized signals

---

#### 3. `ProposedBlock` (6 references)
**Used by**:
- `inference/block_builder.rs` (creation)
- `inference/batch_classifier.rs` (classification)
- `inference/project_matcher.rs` (project assignment)
- `db/blocks/operations.rs` (persistence)
- `commands/blocks.rs` (API retrieval)
- `integrations/sap/forwarder.rs` (time entry conversion)

**Fields**: 23 total (see inference/types.rs)
- Extensive metadata for block classification, confidence, idle handling, location context
- Supports FEATURE-033 (location/context tracking)

**Migration Impact**: HIGH - Central to classification workflow

---

#### 4. `TimeEntryOutbox` (6 references)
**Used by**:
- `db/outbox/outbox.rs` (persistence)
- `sync/outbox_worker.rs` (background sync)
- `integrations/sap/forwarder.rs` (SAP sync)
- `domain/api/forwarder.rs` (Main API sync)
- `commands/idle_sync.rs` (API commands)
- `commands/database.rs` (stats)

**Fields**: 22 total (FEATURE-020 SAP fields)
- Multi-target support (`target`: 'sap' | 'main_api')
- SAP-specific: `wbs_code`, `correlation_id`, `sap_entry_id`
- Retry logic: `attempts`, `retry_after`, `next_attempt_at`, `error_code`

**Migration Impact**: HIGH - Dual-target outbox pattern

---

#### 5. `ContextSignals` (5 references)
**Used by**:
- `inference/signals.rs` (extraction)
- `inference/project_matcher.rs` (project scoring)
- `inference/block_builder.rs` (reasoning)
- `inference/hybrid_classifier.rs` (ML features)
- `db/activity/segments.rs` (serialization in `extracted_signals_json`)

**Fields**: 18 total
- Signal data: `title_keywords`, `url_domain`, `file_path`, `project_folder`
- Classification: `app_category`, `is_vdr_provider`, `project_id`
- Context: `calendar_event_id`, `attendee_domains`, `organizer_domain`
- Flags: `is_screen_locked`, `has_personal_event`, `is_internal_training`, `is_personal_browsing`, `has_external_meeting_attendees`

**Migration Impact**: MEDIUM - Internal type, but serialized to DB

---

#### 6. `WindowContext` (5 references)
**Used by**:
- `tracker/core.rs` (capture)
- `tracker/providers/macos.rs` (platform-specific extraction)
- `detection/enrichers/browser.rs` (URL extraction)
- `detection/enrichers/office.rs` (file path extraction)
- `shared/types/mod.rs` (embedded in ActivityContext)

**Fields**: 6 total
- `app_name`, `window_title`, `bundle_id`
- `url`, `url_host` (enrichment Phase 0)
- `document_name`, `file_path` (enrichment Phase 0)

**Migration Impact**: MEDIUM - Embedded in ActivityContext

---

#### 7. `BatchStats` (5 references)
**Used by**:
- `db/batch/operations.rs` (query)
- `commands/database.rs` (stats API)
- `sync/scheduler.rs` (monitoring)
- `shared/types/stats.rs` (definition)
- `db/models.rs` (re-export)

**Fields**: 4 total
- `pending`, `processing`, `completed`, `failed` (all i64)

**Migration Impact**: LOW - Simple stats aggregation

---

### Moderately Shared Types (Used in 2-3 Modules)

| Type | Locations | Migration Impact |
|------|-----------|------------------|
| `IdlePeriod` | idle detector, commands | Medium (FEATURE-028) |
| `CalendarEvent` | calendar sync, parser, commands | Medium (feature-gated) |
| `WbsElement` | SAP client, cache, commands | Medium (feature-gated) |
| `UserProfile` | domain/user_profile, commands | Low (simple CRUD) |
| `PrismaTimeEntryDto` | outbox, forwarder, commands | Medium (API contract) |
| `OutboxStats` | db/outbox, commands | Low (simple stats) |
| `DatabaseStats` | db/local, commands | Low (simple stats) |
| `ActivityMetadata` | shared/types, signals, redactor | Low (embedded type) |

---

## Complete Struct Inventory

### Database Models (`db/models.rs`)

| Struct | TS Export? | Purpose | Fields |
|--------|------------|---------|--------|
| `ActivitySnapshot` | ✅ | Raw 30s snapshots | 14 fields (id, timestamp, activity_context_json, denormalized fields, idle tracking) |
| `ActivitySegment` | ✅ | 5-min aggregated segments | 14 fields (includes signals_json, project_match_json) |
| `BatchQueue` | ✅ | Batch processing queue | 11 fields (status, lease, worker_id, costs) |
| `TimeEntryOutbox` | ✅ | Outbox pattern for sync | 22 fields (multi-target, SAP fields, retry logic) |
| `IdMapping` | ✅ | Local UUID → Backend CUID | 5 fields |
| `PrismaTimeEntryDto` | ✅ | GraphQL time entry DTO | 17 fields + frontend display fields |
| `ContextPart` | ✅ | Per-app activity breakdown | 3 fields (app, duration_sec, contribution) |
| `AcceptPatch` | ✅ | Partial update for editing | 5 optional fields |
| `Project` | ✅ | Minimal project info | 2 fields (id, name) |
| `ProjectWithWbs` | ✅ | Project with WBS code | 3 fields (id, name, wbs_code) |
| `CalendarTokenRow` | ❌ | Calendar OAuth tokens (metadata only) | 7 fields |
| `CalendarSyncSettingsRow` | ❌ | Calendar sync config | 12 fields |
| `CalendarEventRow` | ✅ | Calendar event database row | 21 fields (FEATURE-029, FEATURE-033) |
| `IdlePeriod` | ✅ | Idle time period | 10 fields (start_ts, end_ts, trigger, user_action) |
| `IdleSummary` | ✅ | Idle time aggregates | 6 fields |

**Enums in models.rs**:
- `BatchStatus`: Pending, Processing, Completed, Failed
- `OutboxStatus`: Pending, Sent, Failed, Dismissed

---

### Shared Types (`shared/types/mod.rs`, `shared/types/stats.rs`)

| Struct | TS Export? | Purpose | Fields |
|--------|------------|---------|--------|
| `ActivityContext` | ✅ | Core activity context | 13 fields (see "Heavily Shared Types") |
| `WindowContext` | ✅ | Window metadata | 6 fields |
| `CalendarEventContext` | ✅ | Calendar event metadata | 6 fields |
| `LocationContext` | ✅ | Location tracking (FEATURE-033) | 3 fields |
| `TemporalContext` | ✅ | Temporal metadata | 2 fields (is_weekend, is_after_hours) |
| `ClassificationContext` | ✅ | Classification metadata | 6 fields |
| `ActivityMetadata` | ✅ | Extracted metadata | 7 optional fields |
| `ConfidenceEvidence` | ✅ | Auditability reasons | 1 field (reasons: Vec<String>) |
| `DatabaseStats` | ✅ | Database statistics | 4 fields |
| `BatchStats` | ✅ | Batch queue stats | 4 fields |
| `SyncStats` | ✅ | Sync operation stats | 4 fields |
| `OutboxStats` | ✅ | Outbox queue stats | 3 fields |
| `DlqBatch` | ✅ | Dead letter queue batch | 7 fields |

**Enums in shared/types**:
- `WorkType`: Modeling, DocReview, Research, Email, Meeting, DMS, DataRoom, AccountingSuite, Documentation, Unknown
- `ActivityCategory`: ClientWork, Research, Communication, Administrative, Meeting, Documentation, Internal

---

### Inference Types (`inference/types.rs`)

| Struct | TS Export? | Purpose | Fields |
|--------|------------|---------|--------|
| `BlockConfig` | ✅ | Block building config | 4 duration fields |
| `ProposedBlock` | ✅ | Time block suggestion | 23 fields (see "Heavily Shared Types") |
| `ActivityBreakdown` | ✅ | Activity within block | 3 fields (name, duration_secs, percentage) |
| `ContextSignals` | ❌ | Extracted signals | 18 fields (see "Heavily Shared Types") |
| `ProjectMatch` | ❌ | Project match result | 6 fields (project_id, wbs_code, deal_name, workstream, confidence, reasons) |
| `SerializedSignals` | ❌ | Versioned wrapper for ContextSignals | 2 fields (version, data) |
| `SerializedProjectMatch` | ❌ | Versioned wrapper for ProjectMatch | 2 fields (version, data) |

**Enums in inference/types**:
- `WorkLocation`: Home, Office, Travel
- `AppCategory`: Excel, Word, PowerPoint, Browser, Email, Meeting, Terminal, IDE, Other

---

### Tracker Types (`tracker/`)

| Struct | Location | Purpose | Fields |
|--------|----------|---------|--------|
| `TrackerState` | `mod.rs` | Tracker orchestration state | Internal (pausable, snapshotable traits) |
| `ActivityError` | `provider.rs` | Activity provider error | 1 field (message) |
| `RefresherState` | `core.rs` | Snapshot refresh state | Internal state machine |
| `Tracker<P, C>` | `core.rs` | Main tracker orchestrator | Generic over provider + clock |
| `MacOsProvider` | `providers/macos.rs` | macOS activity provider | Internal (cache, enrichment queue) |
| `CacheEntry` | `providers/macos.rs` (private) | Cached window info | 3 fields (context, timestamp, enriched) |
| `EnrichmentJob` | `providers/macos.rs` (private) | Background enrichment | 3 fields (snapshot_id, context, timestamp) |
| `DummyProvider` | `providers/dummy.rs` | Test/fallback provider | Zero-sized type |
| `MacOsEventListener` | `os_events/macos.rs` | macOS event monitoring | Platform-specific (CFRunLoop, callbacks) |
| `FallbackEventListener` | `os_events/fallback.rs` | Fallback event provider | Zero-sized type |
| `MacOsIdleDetector` | `idle/detector.rs` | macOS idle detection | Platform-specific (CGEventSource) |
| `IdlePeriodTracker` | `idle/period_tracker.rs` | Idle period tracking | State machine for idle periods |
| `ErrorRecovery` | `idle/recovery.rs` | Circuit breaker | State: failures, last_error, circuit_state |
| `SleepWakeListener` | `idle/recovery.rs` | System sleep/wake monitoring | Platform-specific (IOKit) |
| `LockScreenListener` | `idle/lock_detection.rs` | Screen lock detection | Platform-specific (DistributedNotifications) |

**Enums in tracker/**:
- `IdleError`: PlatformApiError, CircuitBreakerOpen, ConfigError, InternalError, DatabaseError, StateError
- `ActivityEvent`: Mouse, Keyboard, SystemWake, SystemSleep, ScreenLock, ScreenUnlock
- `PauseReason`: Manual, Idle, Locked, Sleep
- `CircuitBreakerState` (private): Closed, Open, HalfOpen
- `RecoveryAction`: Retry, OpenCircuitBreaker, CloseCircuitBreaker, Skip

---

### Sync Types (`sync/`)

| Struct | Location | Purpose | Fields |
|--------|----------|---------|--------|
| `SyncScheduler` | `scheduler.rs` | Background sync orchestrator | Internal state |
| `DrainStats` | `scheduler.rs` | Drain operation stats | 3 fields (drained, failed, remaining) |
| `OutboxWorker` | `outbox_worker.rs` | Outbox background worker | Internal (db, client, stats) |
| `ProcessingStats` | `outbox_worker.rs` | Processing stats | 5 fields (total, succeeded, failed, rate_limited, dismissed) |
| `NeonClient` | `neon_client.rs` | Neon PostgreSQL client | Internal (pool, config) |
| `CostTracker` | `cost_tracker.rs` | OpenAI cost tracking | Internal (db, config) |
| `CostConfig` | `cost_tracker.rs` | Cost limits | 3 fields (daily_limit, warning_threshold, currency) |
| `TokenUsage` | `cost_tracker.rs` | Token usage record | 5 fields (user_id, tokens, cost, mode, timestamp) |
| `UserCostSummary` | `cost_tracker.rs` | User cost summary | 4 fields (user_id, total_cost, total_tokens, entry_count) |
| `TokenVariance` | `cost_tracker.rs` | Token variance stats | 2 fields (mean, stddev) |
| `CleanupService` | `cleanup.rs` | Data cleanup orchestrator | Internal (db, config) |
| `CleanupConfig` | `cleanup.rs` | Cleanup retention policy | 6 fields (retention days for various types) |
| `CleanupSummary` | `cleanup.rs` | Cleanup operation results | 7 fields (deleted counts per type) |
| `DryRunResult` | `cleanup.rs` | Dry-run preview | 2 fields (summary, would_delete) |
| `CleanupScheduler` | `cleanup.rs` | Scheduled cleanup | Internal (service, scheduler) |
| `RetryConfig` | `retry.rs` | Retry configuration | 5 fields (max_attempts, backoff, jitter, timeout, transient_errors) |
| `RetryConfigBuilder` | `retry.rs` | Builder for RetryConfig | Builder pattern |
| `RetryContext` | `retry.rs` | Retry attempt context | 5 fields (attempt, elapsed, last_error, backoff, operation_id) |
| `RetryExecutor<P>` | `retry.rs` | Generic retry executor | Generic over policy |
| `RetryPolicy` | `retry.rs` | Retry decision policy | Config + state |
| `RetryState` | `retry.rs` | Retry state tracking | 4 fields (attempts, first_attempt, last_attempt, total_wait) |

**Enums in sync/**:
- `RetryError<E>`: PermanentFailure, Exhausted, Timeout
- `RetryDecision`: Retry, Fail, Succeed
- `BackoffStrategy`: Constant, Linear, Exponential
- `Jitter`: None, Full, Equal, Decorrelated
- `RetryStrategy`: Immediate, Exponential, Custom
- `SyncError`: Database, Network, Validation, RateLimit, Unknown
- `NeonClientError`: Connection, Query, Serialization, Pool
- `ClassificationMode`: AI, Manual

---

### Integration Types

#### SAP Integration (`integrations/sap/`)

| Struct | Location | TS Export? | Purpose |
|--------|----------|------------|---------|
| `WbsElement` | `models.rs` | ✅ | WBS element with Neon enrichment (11 fields) |
| `OutboxStatusSummary` | `models.rs` | ✅ | Outbox status counts (3 fields) |
| `SapSyncSettings` | `models.rs` | ✅ | SAP sync configuration (4 fields) |
| `SapClient` | `client.rs` | ❌ | SAP GraphQL client |
| `SapError` | `errors.rs` | ❌ | SAP error with category (3 fields) |
| `SapHealthStatus` | `health_monitor.rs` | ❌ | Health check status (4 fields) |
| `SapHealthMonitor` | `health_monitor.rs` | ❌ | Health monitoring service |
| `OutboxForwarder` | `forwarder.rs` | ❌ | SAP outbox forwarder |
| `SyncScheduler` | `scheduler.rs` | ❌ | SAP sync scheduler |
| `WbsValidator` | `validation.rs` | ❌ | WBS code validation |
| `WbsCache` | `cache.rs` | ❌ | Local WBS cache |
| `NeonWbsCache` | `neon_cache.rs` | ❌ | Neon-backed WBS cache |
| `BulkLookupManager` | `bulk_lookup.rs` | ❌ | Bulk WBS lookup |
| `NetworkStatus` | `network_status.rs` | ❌ | Network connectivity status (2 fields) |
| `ConnectionHealthStatus` | `commands.rs` | ❌ | Connection health (4 fields) |
| `RetrySyncResult` | `commands.rs` | ❌ | Retry sync result (3 fields) |
| `BatchValidationResult` | `commands.rs` | ❌ | Batch validation (4 fields) |
| `TimeEntryInput` | `client.rs` | ❌ | SAP GraphQL input (8 fields) |
| `TimeEntryBatchResult` | `client.rs` | ❌ | Batch submission result (3 fields) |
| `TimeEntryError` | `client.rs` | ❌ | Time entry error (3 fields) |

**Enums in sap/**:
- `SapErrorCategory`: Network, Validation, Authentication, RateLimit, Server, Unknown
- `ValidationResult`: Valid, Invalid, Unknown
- `SyncStatus`: Idle, Running, Failed
- `TimeEntryStatus`: Draft, Submitted, Approved, Rejected
- `TimeEntryErrorCode`: InvalidWbs, InvalidDate, InvalidDuration, DuplicateEntry

---

#### Calendar Integration (`integrations/calendar/`)

| Struct | Location | TS Export? | Purpose |
|--------|----------|------------|---------|
| `CalendarConnectionStatus` | `types.rs` | ✅ | OAuth connection status (5 fields) |
| `CalendarSyncSettings` | `types.rs` | ✅ | Sync configuration (9 fields) |
| `CalendarEvent` | `types.rs` | ✅ | Calendar event (17 fields, FEATURE-029/033) |
| `TimelineCalendarEvent` | `types.rs` | ✅ | Timeline visualization (11 fields) |
| `ParsedEventTitle` | `types.rs` | ❌ | Parsed event components (4 fields) |
| `CalendarClient` | `client.rs` | ❌ | Calendar API client |
| `CalendarEventsResponse` | `client.rs` | ❌ | API response (2 fields) |
| `GoogleProvider` | `providers/google.rs` | ❌ | Google Calendar provider |
| `MicrosoftProvider` | `providers/microsoft.rs` | ❌ | Microsoft Calendar provider |
| `RawCalendarEvent` | `providers/traits.rs` | ❌ | Provider-agnostic event (8 fields) |
| `FetchEventsResponse` | `providers/traits.rs` | ❌ | Fetch response (2 fields) |
| `TokenResponse` | `providers/traits.rs` | ❌ | OAuth token response (4 fields) |
| `RefreshTokenResponse` | `providers/traits.rs` | ❌ | Refresh token response (3 fields) |
| `GoogleCalendarEvent` | `sync.rs` (private) | ❌ | Google API event shape (7 fields) |
| `CalendarSyncWorker` | `sync.rs` | ❌ | Background sync worker |
| `CalendarSyncScheduler` | `scheduler.rs` | ❌ | Calendar sync scheduler |
| `OAuthCallbackData` | `oauth.rs` | ❌ | OAuth callback data (3 fields) |
| `OAuthCallbackServer` | `oauth.rs` | ❌ | OAuth callback server |

---

#### Main API Integration (`domain/api/`)

| Struct | Location | TS Export? | Purpose |
|--------|----------|------------|---------|
| `CreateTimeEntryInput` | `models.rs` | ❌ | GraphQL input (12 fields) |
| `TimeEntryResponse` | `models.rs` | ❌ | GraphQL response (13 fields) |
| `GraphQLResponse<T>` | `models.rs` | ❌ | Generic GraphQL wrapper |
| `GraphQLError` | `models.rs` | ❌ | GraphQL error (3 fields) |

---

### OAuth & Auth Types (`shared/auth/`)

| Struct | Location | Purpose | Fields |
|--------|----------|---------|--------|
| `TokenSet` | `types.rs` | OAuth token set | 5 fields (access, refresh, expiry, scope, provider) |
| `TokenResponse` | `types.rs` | OAuth response | 5 fields |
| `OAuthConfig` | `types.rs` | OAuth configuration | 8 fields (client_id, auth_url, token_url, scopes, etc.) |
| `OAuthError` | `types.rs` | OAuth error | 2 fields (error, description) |
| `TokenManager` | `token_manager.rs` | Token lifecycle manager | Internal (storage, keychain) |
| `PKCEChallenge` | `pkce.rs` | PKCE challenge | 2 fields (verifier, challenge) |
| `OAuthService` | `oauth_service.rs` | OAuth orchestration | Internal (config, token_manager) |
| `OAuthCallbackData` | `oauth_service.rs` | OAuth callback | 3 fields (code, state, error) |
| `OAuthCallbackServer` | `oauth_service.rs` | Callback HTTP server | Internal (server, receiver) |
| `OAuthClient` | `oauth_pkce.rs` | PKCE OAuth client | Internal (config, http_client) |
| `KeychainStorage` | `keychain.rs` | Keychain wrapper | Platform-specific |

**Enums in auth/**:
- `TokenManagerError`: StorageError, ValidationError, NetworkError, ParseError, KeychainError, Expired
- `OAuthServiceError`: ConfigError, NetworkError, InvalidResponse, UserCancelled, CallbackTimeout, StateValidationFailed, KeychainError
- `OAuthClientError`: ConfigError, NetworkError, InvalidResponse, PkceError, TokenExchangeFailed
- `KeychainError`: ItemNotFound, AccessDenied, InvalidData, IoError, Other

---

### Configuration & Utilities

#### Shared Config (`shared/config.rs`, `shared/extractors/`)

| Struct | Location | Purpose |
|--------|----------|---------|
| `Config` | `config.rs` | App-wide config (6 fields) |
| `PackConfig` | `config.rs` | Detector pack config (2 fields) |
| `PatternExtractor` | `extractors/pattern.rs` | Generic pattern extractor |
| `PatternExtractorBuilder` | `extractors/pattern.rs` | Builder for PatternExtractor |

---

#### Preprocessing (`preprocess/`)

| Struct | Location | Purpose |
|--------|----------|---------|
| `Segmenter<DB>` | `segmenter.rs` | Activity segmentation orchestrator |
| `DictionaryEntry` | `segmenter.rs` | Dictionary-based compression (4 fields) |

---

#### Observability (`observability/`)

**Error Types** (`observability/errors/app.rs`):

| Enum | Variants | Purpose |
|------|----------|---------|
| `AppError` | 13 variants | Top-level application error |
| `ErrorCode` | 30+ codes | Machine-readable error codes |
| `ActionHint` | 10 hints | User-facing action suggestions |
| `DbError` | 9 variants | Database errors |
| `KeychainError` | 5 variants | Keychain errors |
| `PreprocessError` | 6 variants | Preprocessing errors |
| `AiError` | 7 variants | AI/OpenAI errors |
| `HttpError` | 6 variants | HTTP errors |
| `BatchError` | 5 variants | Batch processing errors |

**Metrics** (`observability/metrics/`):

| Struct | Location | Purpose |
|--------|----------|---------|
| `DbMetrics` | `db.rs` | Database operation metrics |
| `DbStats` | `db.rs` | Database statistics |
| `CallMetrics` | `call.rs` | Function call metrics |
| `CacheMetrics` | `cache.rs` | Cache hit/miss metrics |
| `EnrichmentMetrics` | `enrichment.rs` | Enrichment metrics |
| `EventMetrics` | `events.rs` | Event metrics |
| `FetchMetrics` | `fetch.rs` | Fetch operation metrics |
| `IdleSyncMetrics` | `idle_sync.rs` | Idle sync metrics |
| `ObserverStats` | `observer.rs` | Observer statistics |
| `ObserverMetrics` | `observer.rs` | Observer metrics |
| `PerformanceMetrics` | `performance.rs` | Performance tracking |
| `PollingMetrics` | `polling.rs` | Polling metrics |
| `TestMetrics` | `test_metrics.rs` | Test metrics |

---

## Enum Inventory

### Core Domain Enums

| Enum | Location | Variants | Target Crate |
|------|----------|----------|--------------|
| `WorkType` | `shared/types/mod.rs` | 10 variants | `domain/types/activity.rs` |
| `ActivityCategory` | `shared/types/mod.rs` | 7 variants | `domain/types/activity.rs` |
| `BatchStatus` | `db/models.rs` | 4 variants | `domain/types/database.rs` |
| `OutboxStatus` | `db/models.rs` | 4 variants | `domain/types/database.rs` |
| `WorkLocation` | `inference/types.rs` | 3 variants | `domain/types/classification.rs` |
| `AppCategory` | `inference/types.rs` | 9 variants | `domain/types/classification.rs` |
| `PauseReason` | `tracker/idle/types.rs` | 4 variants | `domain/types/idle.rs` |

### Error Enums (Many!)

**Application Errors** (`observability/errors/app.rs`):
- `AppError` → `domain/errors/app_error.rs`
- `ErrorCode` → `domain/errors/codes.rs`
- `ActionHint` → `domain/errors/hints.rs`
- `DbError` → `domain/errors/db_error.rs`
- `KeychainError` → `domain/errors/keychain_error.rs`
- `PreprocessError` → `domain/errors/preprocess_error.rs`
- `AiError` → `domain/errors/ai_error.rs`
- `HttpError` → `domain/errors/http_error.rs`
- `BatchError` → `domain/errors/batch_error.rs`

**Idle Errors** (`tracker/idle/types.rs`):
- `IdleError` → `domain/errors/idle_error.rs`
- `ActivityEvent` → `core/idle/events.rs`

**Sync Errors** (`sync/`):
- `RetryError<E>` → Common crates (use `pulsearc_common::resilience::retry`)
- `SyncError` → `domain/errors/sync_error.rs`
- `NeonClientError` → `infra/sync/neon_client.rs` (infra-specific)

**SAP Errors** (`integrations/sap/errors.rs`):
- `SapError` → `domain/errors/sap_error.rs` (feature-gated)
- `SapErrorCategory` → `domain/errors/sap_error.rs`

**Auth Errors** (`shared/auth/`):
- `TokenManagerError` → `infra/auth/token_manager.rs`
- `OAuthServiceError` → `infra/auth/oauth_service.rs`
- `OAuthClientError` → `infra/auth/oauth_client.rs`

### Strategy/Config Enums

**Retry Strategy** (`sync/retry.rs`):
- `RetryDecision` → Use `pulsearc_common::resilience::retry`
- `BackoffStrategy` → Use `pulsearc_common::resilience::retry`
- `Jitter` → Use `pulsearc_common::resilience::retry`
- `RetryStrategy` → Use `pulsearc_common::resilience::retry`

**Classification** (`sync/cost_tracker.rs`):
- `ClassificationMode`: AI, Manual → `domain/types/classification.rs`

**SAP** (`integrations/sap/`):
- `ValidationResult` → `core/integrations/sap_validation.rs`
- `SyncStatus` → `infra/integrations/sap/scheduler.rs`
- `TimeEntryStatus` → `domain/types/sap.rs`
- `TimeEntryErrorCode` → `domain/types/sap.rs`

---

## Type Dependencies

### Dependency Graph (Core Types)

```
ActivitySnapshot
  ├─→ ActivityContext (JSON serialized)
  │     ├─→ WindowContext (embedded)
  │     ├─→ WorkType (enum)
  │     ├─→ ActivityCategory (enum)
  │     ├─→ ActivityMetadata (embedded)
  │     ├─→ ConfidenceEvidence (embedded)
  │     ├─→ CalendarEventContext (optional)
  │     ├─→ LocationContext (optional)
  │     ├─→ TemporalContext (optional)
  │     └─→ ClassificationContext (optional)
  └─→ (denormalized: detected_activity, work_type, activity_category, primary_app)

ActivitySegment
  ├─→ extracted_signals_json (ContextSignals serialized)
  │     ├─→ AppCategory (enum)
  │     └─→ title_keywords, url_domain, project_folder (extracted)
  ├─→ project_match_json (ProjectMatch serialized)
  │     └─→ project_id, wbs_code, deal_name, workstream, confidence, reasons
  └─→ work_type, activity_category, detected_activity (copied from snapshots)

ProposedBlock
  ├─→ activities: Vec<ActivityBreakdown>
  ├─→ snapshot_ids: Vec<String>
  ├─→ segment_ids: Vec<String>
  ├─→ work_location: Option<WorkLocation> (enum)
  └─→ inferred_project_id, inferred_wbs_code, inferred_deal_name (from ProjectMatch)

TimeEntryOutbox
  ├─→ payload_json (PrismaTimeEntryDto serialized)
  │     ├─→ context_breakdown: Vec<ContextPart>
  │     └─→ org_id, project_id, task_id, user_id (CUIDs/IDs)
  ├─→ OutboxStatus (enum)
  └─→ wbs_code (for SAP target)

IdlePeriod
  ├─→ system_trigger: 'threshold' | 'lock_screen' | 'sleep' | 'manual'
  └─→ user_action: 'kept' | 'discarded' | 'pending' | 'auto_excluded'
```

### Shared Field Analysis

**`wbs_code` appears in**:
- `WbsElement` (primary key)
- `ProjectMatch` (optional)
- `ProposedBlock` (optional, via ProjectMatch)
- `TimeEntryOutbox` (optional, for SAP sync)
- `AcceptPatch` (optional, for editing)

**`project_id` appears in**:
- `PrismaTimeEntryDto` (required)
- `ProjectMatch` (optional)
- `ProposedBlock` (optional, as `inferred_project_id`)
- `ContextSignals` (optional)
- `Project`, `ProjectWithWbs` (primary key)

**`confidence` appears in**:
- `ProposedBlock` (classification confidence)
- `ProjectMatch` (match confidence)
- `ActivityContext` (billable_confidence)
- `PrismaTimeEntryDto` (display confidence)
- `CalendarEventRow` (parsing confidence)
- `ParsedEventTitle` (parsing confidence)

**`timestamp` fields**:
- Unix epoch i64: `ActivitySnapshot`, `ActivitySegment`, `ProposedBlock`, `IdlePeriod`, `TimeEntryOutbox`, `CalendarEventRow`, `UserProfile`
- Chrono types: Tracker internal, IdleDetector internal
- ISO 8601 strings: `PrismaTimeEntryDto`, `TimeEntryResponse`

---

## Migration Priorities

### Priority 1: Core Domain Types (Week 1)

**Move to `domain/src/types/`**:
1. `ActivitySnapshot`, `ActivitySegment`, `BatchQueue`, `IdlePeriod`, `IdleSummary` (from `db/models.rs`)
2. `ActivityContext`, `WindowContext`, all context structs (from `shared/types/mod.rs`)
3. `WorkType`, `ActivityCategory` enums (from `shared/types/mod.rs`)
4. `ProposedBlock`, `ActivityBreakdown`, `ContextSignals`, `ProjectMatch` (from `inference/types.rs`)
5. `WorkLocation`, `AppCategory` enums (from `inference/types.rs`)
6. `TimeEntryOutbox`, `OutboxStatus`, `PrismaTimeEntryDto`, `IdMapping` (from `db/models.rs`)

**Move to `domain/src/config/`**:
1. `Config`, `PackConfig` (from `shared/config.rs`)
2. `IdleConfig`, `RecoveryConfig` (from `tracker/idle/config.rs`)
3. `BlockConfig` (from `inference/types.rs`)

**Move to `domain/src/errors/`**:
1. `AppError`, `ErrorCode`, `ActionHint`, `DbError` (from `observability/errors/app.rs`)
2. `IdleError`, `PauseReason` (from `tracker/idle/types.rs`)

**Move to `domain/src/utils/`**:
1. `PatternExtractor` (from `shared/extractors/pattern.rs`) - pure utility
2. Title/string helpers (from `utils/title.rs`)

---

### Priority 2: Core Business Logic (Week 2)

**Traits to define in `core/src/*/ports.rs`**:
1. `ActivityProvider` (tracker)
2. `ActivityRepository`, `SegmentRepository`, `BlockRepository` (database)
3. `Classifier`, `ProjectMatcher` (classification)
4. `OutboxQueue` (sync)
5. `EventProvider` (os_events)

**Move business logic to `core/`**:
1. `Tracker`, `RefresherState` → `core/src/tracking/service.rs`
2. `Segmenter` → `core/src/tracking/segmenter.rs`
3. `SignalExtractor`, `EvidenceExtractor`, `BlockBuilder`, `ProjectMatcher` → `core/src/classification/`
4. `IdlePeriodTracker`, `ErrorRecovery` → `core/src/idle/`

---

### Priority 3: Infrastructure Adapters (Week 3-4)

**Move to `infra/src/database/`**:
- All `db/` repositories (ActivityRepository, SegmentRepository, BlockRepository, etc.)

**Move to `infra/src/platform/`**:
- `MacOsProvider`, `MacOsEventListener`, `MacOsIdleDetector`, `LockScreenListener`, `SleepWakeListener`
- Browser/Office enrichers

**Move to `infra/src/sync/`**:
- `OutboxWorker`, `NeonClient`, `CostTracker`, `CleanupService`, schedulers

**Move to `infra/src/integrations/`**:
- SAP client, calendar clients, Main API client (behind feature flags)

---

### Priority 4: API Layer (Week 5)

**Move to `api/src/commands/`**:
- All Tauri command handlers

**Create in `api/src/`**:
- `context.rs` (DI container)
- `mapping/` (domain ↔ frontend types)

---

## Identified Shared Types (High Priority for Domain Layer)

### Types Used in 5+ Modules

1. **`ActivityContext`** (9 modules)
   - Target: `domain/types/activity.rs`
   - Status: ✅ Serialized in DB, TS-exported
   - Migration: Week 1, Priority 1

2. **`ActivitySegment`** (7 modules)
   - Target: `domain/types/database.rs`
   - Status: ✅ Core database type, TS-exported
   - Migration: Week 1, Priority 1

3. **`ProposedBlock`** (6 modules)
   - Target: `domain/types/classification.rs`
   - Status: ✅ TS-exported, central to classification
   - Migration: Week 1, Priority 1

4. **`TimeEntryOutbox`** (6 modules)
   - Target: `domain/types/database.rs`
   - Status: ✅ Multi-target outbox, TS-exported
   - Migration: Week 1, Priority 1

5. **`ContextSignals`** (5 modules)
   - Target: `domain/types/classification.rs`
   - Status: ❌ Internal (not TS-exported), serialized to DB
   - Migration: Week 1, Priority 1

6. **`WindowContext`** (5 modules)
   - Target: `domain/types/activity.rs`
   - Status: ✅ Embedded in ActivityContext, TS-exported
   - Migration: Week 1, Priority 1

7. **`BatchStats`** (5 modules)
   - Target: `domain/types/stats.rs`
   - Status: ✅ Simple stats type, TS-exported
   - Migration: Week 1, Priority 1

---

## Next Steps

1. **Validate Struct Inventory**: Cross-reference with actual source files to ensure completeness
2. **Identify Missing Traits**: Any implicit traits used across modules?
3. **Analyze Circular Dependencies**: Map which types create circular refs
4. **Plan Serialization Strategy**: Ensure all DB-bound types have rusqlite conversions
5. **Feature Flag Matrix**: Document which types require which feature flags
6. **TS Export Validation**: Verify `#[ts(export)]` annotations for all frontend-facing types

---

**Document Status**: ✅ COMPLETE - Ready for Phase 1 migration planning

