# PulseArc Crates - Comprehensive Architecture Map

## Overview
PulseArc is organized into 5 core crates following hexagonal architecture:
- **domain**: Pure domain models and types (no dependencies)
- **core**: Business logic (ports/traits) - depends only on domain
- **common**: Shared utilities (features, observability, security)
- **infra**: Infrastructure implementations (I/O, database, APIs)
- **api**: Tauri application layer (commands and DI container)

---

## CRATE 1: pulsearc-domain
**Purpose**: Pure domain models - no external business logic

### Types Implemented
- `TimeEntry` - Classified work entry with full metadata
- `ActivityContext` - Captured OS activity with enrichment (13+ nested fields)
- `ActivitySnapshot` - Persisted activity snapshot
- `ProposedBlock` - 30+ min consolidated activity block
- `ActivityMetadata`, `WindowContext`, `CalendarEventContext`
- `WorkType` enum (Modeling, DocReview, Research, Email, Meeting, etc.)
- `ActivityCategory` enum (ClientWork, Research, Communication, etc.)

### Database Types
- `TimeRange`, `IdMapping`, `Project`, `ProjectWithWbs`
- `CalendarEventRow`, `CalendarSyncSettingsRow`
- `BatchQueue`, `BatchStatus`, `BatchStats`
- `DlqBatch` (Dead Letter Queue)
- `TimeEntryOutbox`, `OutboxStatus`
- `ActivitySegment`, `ActivitySnapshot` (with SnapshotMetadata)
- `IdlePeriod`, `IdleSummary` (FEATURE-028)
- `UserProfile`

### SAP Integration Types
- `WbsElement` (Work Breakdown Structure)
- `SapSyncSettings`, `OutboxStatusSummary`

### Statistics Types
- `TokenUsage`, `TokenVariance`, `UserCostSummary`
- `DatabaseStats`, `SyncStats`, `OutboxStats`
- `ClassificationMode`

### Config & Constants
- `Config` - Application configuration
- Domain-specific macros and utilities

---

## CRATE 2: pulsearc-core
**Purpose**: Business logic via port interfaces (no infrastructure code)

### Port Traits (Boundaries)

#### Tracking Ports
**ActivityProvider** (trait)
- `get_activity()` → ActivityContext
- `is_paused()`, `pause()`, `resume()`

**ActivityRepository** (trait)
- `save_snapshot(snapshot)` → Result
- `get_snapshots(start, end)` → Vec<ActivitySnapshot>
- `delete_old_snapshots(before)`

**ActivityEnricher** (trait)
- `enrich(&mut context)` → enriches activity with metadata

**SegmentRepository** (trait - sync)
- `save_segment(segment)`
- `find_segments_by_date(date)` → Vec<ActivitySegment>
- `find_unprocessed_segments(limit)`
- `mark_processed(segment_id)`

**SnapshotRepository** (trait - sync)
- `find_snapshots_by_time_range(start, end)`
- `count_snapshots_by_date(date)`

**CalendarEventRepository** (trait)
- `find_event_by_timestamp(timestamp, window_secs)`
- `insert_calendar_event(params)`
- `get_calendar_events_by_time_range(email, start_ts, end_ts)`
- `get_today_calendar_events()`
- `delete_calendar_events_older_than(days)`

**IdlePeriodsRepository** (trait)
- `save_idle_period(period)`, `get_idle_period(id)`
- `get_idle_periods_in_range(start_ts, end_ts)`
- `get_pending_idle_periods()`
- `update_idle_period_action(id, action, notes)`
- `delete_idle_periods_before(before_ts)`

#### Classification Ports
**Classifier** (trait)
- `classify(snapshots: Vec<ActivitySnapshot>)` → TimeEntry

**TimeEntryRepository** (trait)
- `save_entry(entry)`, `get_entries(start, end)`
- `update_entry(entry)`, `delete_entry(id)`

**BlockRepository** (trait)
- `save_proposed_block(block)`
- `get_proposed_blocks(date)` → Vec<ProposedBlock>

**ProjectMatcher** (trait)
- `match_project(signals)` → Option<ProjectMatch>

**WbsRepository** (trait)
- `count_active_wbs()` → i64
- `get_last_sync_timestamp()` → Option<i64>
- `load_common_projects(limit)` → Vec<WbsElement>
- `fts5_search_keyword(keyword, limit)` → Vec<WbsElement>
- `get_wbs_by_project_def(project_def)` → Option<WbsElement>
- `get_wbs_by_wbs_code(wbs_code)` → Option<WbsElement>

#### Sync Ports
**OutboxQueue** (trait)
- `enqueue(entry)`, `dequeue_batch(limit)`
- `mark_sent(id)`, `mark_failed(id, error)`

**IdMappingRepository** (trait)
- `create_id_mapping(mapping)`, `get_id_mapping_by_local_uuid(uuid)`
- `get_backend_cuid_by_local_uuid(uuid)`, `get_local_uuid_by_backend_cuid(cuid)`
- `get_id_mappings_by_entity_type(type)` → Vec<IdMapping>

**TokenUsageRepository** (trait)
- `record_token_usage(usage)`, `get_token_usage_by_batch(batch_id)`
- `record_estimated_usage(usage)`, `record_actual_usage(usage)`
- `delete_token_usage_by_batch(batch_id)`

#### Batch Ports
**BatchRepository** (trait)
- `save_batch(batch)`, `get_batch(batch_id)`, `update_batch_status(id, status)`
- `acquire_batch_lease(id, worker, duration)`, `renew_batch_lease(...)`
- `get_stale_leases(ttl_secs)`, `recover_stale_leases()`
- `create_batch_from_unprocessed(max_snapshots, worker_id, lease_duration)`
- `complete_batch(id)`, `mark_batch_failed(id, error)`
- `get_batches_by_status(status)`, `get_batch_stats()`, `get_pending_batches()`
- `cleanup_old_batches(older_than_seconds)`, `delete_batch(id)`

**DlqRepository** (trait)
- `move_batch_to_dlq(batch_id, error)`
- `get_dlq_batches()`, `get_dlq_batches_with_details()`
- `reset_batch_for_retry(batch_id)`, `retry_failed_batch(batch_id)`

#### User Ports
**UserProfileRepository** (trait)
- `get_by_id(id)`, `get_by_auth0_id(auth0_id)`, `get_by_email(email)`
- `create(profile)`, `update(profile)`, `delete(id)`

#### Feature Flags Ports
**FeatureFlagsPort** (trait)
- `is_enabled(flag_name)` → bool
- `get_flag(flag_name)` → Option<FeatureFlag>
- `set_enabled(flag_name, enabled)`
- `list_all()` → Vec<FeatureFlag>

### Services (Business Logic)

**TrackingService**
- `capture_activity()` → ActivityContext
- `get_snapshots(start, end)` → Vec<ActivitySnapshot>
- `is_paused()` → bool
- Supports enrichers and persistence config
- Uses ActivityProvider + ActivityRepository

**ClassificationService**
- `classify_and_save(snapshots)` → TimeEntry
- `get_entries(start, end)` → Vec<TimeEntry>
- `update_entry(entry)`, `delete_entry(id)`
- Uses Classifier + TimeEntryRepository

### Feature-Gated Ports
- `calendar_ports` (feature = "calendar")
- `sap_ports` (feature = "sap")

---

## CRATE 3: pulsearc-common
**Purpose**: Shared infrastructure utilities

### Feature Tiers
```
foundation: error, validation, utils, collections, privacy
runtime: cache, time, resilience, sync, lifecycle, observability, crypto
platform: auth, security, storage (SQLCipher integration)
test-utils: testing utilities
```

### Key Modules

**error/** - CommonError, ErrorClassification, ErrorContext, ErrorSeverity
**validation/** - FieldValidator, RuleBuilder, RuleSet, Validator
**collections/** - BloomFilter, BoundedQueue, LRU, Trie, RingBuffer
**utils/** - Macros, serde helpers (duration_millis), etc.

**cache/** - Thread-safe cache with TTL, eviction policies
**crypto/** - AES-256-GCM encryption, EncryptedData, EncryptionService
**privacy/** - Data hashing, pattern detection
**time/** - Duration formatting, intervals, timers, cron support

**resilience/** - Generic circuit breaker, retry with backoff
**sync/** - Domain-specific sync queue, integrated retry
**lifecycle/** - Component lifecycle management (AsyncManager, ManagedState)
**observability/** - Metrics, tracing, error reporting

**auth/** - OAuth client, token management, PKCE flow
**security/** - Key management, keychain provider, RBAC
**storage/** - SQLCipher integration, encrypted storage
**compliance/** - Audit logging, feature flags

**testing/** - Mock clocks, builders, matchers, temp files

---

## CRATE 4: pulsearc-infra
**Purpose**: Infrastructure implementations and I/O

### Database Module (14 repositories)

**Activity Tracking**
- `SqlCipherActivityRepository` - implements ActivityRepository
- `SqlCipherSegmentRepository` - implements SegmentRepository (sync)
- `SqlCipherSnapshotRepository` - implements SnapshotRepository (sync)

**Calendar**
- `SqlCipherCalendarEventRepository` - implements CalendarEventRepository
- `SqlCipherIdlePeriodsRepository` - implements IdlePeriodsRepository

**Classification**
- `SqlCipherTimeEntryRepository` - implements TimeEntryRepository
- `SqlCipherBlockRepository` - implements BlockRepository
- `SqlCipherWbsRepository` - implements WbsRepository (FTS5 search)

**Sync & Batch**
- `SqlCipherOutboxRepository` - implements OutboxQueue
- `SqlCipherBatchRepository` - implements BatchRepository (lease mgmt)
- `SqlCipherDlqRepository` - implements DlqRepository

**IDs & Tokens**
- `SqlCipherIdMappingRepository` - implements IdMappingRepository
- `SqlCipherTokenUsageRepository` - implements TokenUsageRepository

**Users & Feature Flags**
- `SqlCipherUserProfileRepository` - implements UserProfileRepository
- `SqlCipherFeatureFlagsRepository` - implements FeatureFlagsPort

**Database Infrastructure**
- `DbManager` - Manages SQLCipher pool and connections
- `SqlCipherPool` - Connection pool wrapper
- `DatabaseRepository` - Base trait for common operations
- `schema.sql` - Full database schema with FTS5 tables

### Platform Module

**macOS (target_os = "macos")**
- `MacOsActivityProvider` - implements ActivityProvider via Accessibility APIs
- `MacOsEventListener` - Monitors window focus changes
- Enrichers: Browser, Office, AppleScript helpers, URL extraction
- Error helpers, AX helpers for Accessibility API

**Fallback (non-macOS)**
- `FallbackActivityProvider` - Returns platform error

### API Client (4 sub-modules)

**Client Infrastructure**
- `ApiClient` - HTTP client for domain sync
- `ApiClientConfig` - Configuration
- `HttpClient` - Wrapper around reqwest with timeouts

**API Integration**
- `ApiAuthService` - OAuth authentication
- `AccessTokenProvider` - Token management
- `ApiCommands` - Command endpoints
- `ApiForwarder` - Batch submission with retry
- `ApiScheduler` - Background sync scheduling

**Error Handling**
- `ApiError`, `ApiErrorCategory` - Structured error types

### Schedulers (4 implementations)

**Block Scheduler**
- `BlockScheduler` - Generates inference blocks
- `BlockJob` - Job type

**Classification Scheduler**
- `ClassificationScheduler` - Periodic classification
- `ClassificationJob` - Job type

**Sync Scheduler**
- `SyncScheduler` - API outbox processing (always enabled)
- Implements periodic batch forwarding

**SAP Scheduler** (feature = "sap")
- `SapScheduler` - Batch forwarding for SAP

**Calendar Scheduler** (feature = "calendar")
- `CalendarScheduler` - Calendar sync

All schedulers support:
- Cron-based scheduling
- Lifecycle management (start/stop)
- Join handle tracking
- Cancellation tokens
- Timeout wrapping on async operations

### Sync Module (4 services)

**OutboxWorker**
- Processes time entry batches
- Retry logic for failed entries
- Partial success handling
- Neon API forwarding

**NeonClient**
- Postgres database sync to remote
- Configuration via NeonClientConfig

**CostTracker**
- API usage tracking and cost monitoring
- Daily cost aggregation

**CleanupService**
- Periodic cleanup of stale data
- Configurable retention policies

### Integrations

**Calendar Integration** (feature = "calendar")
- Google Calendar provider
- Microsoft Calendar provider
- OAuth handling
- Event parsing and sync
- Trait-based provider system

**SAP Integration** (feature = "sap")
- Client for SAP ERP API
- Authentication and token refresh
- WBS caching (FTS5 full-text search)
- Batch forwarding with health checks
- Validation and error handling

**OpenAI Integration**
- Client wrapper
- Type definitions for API responses

### Services

**FeatureFlagService**
- Wraps FeatureFlagsRepository
- Caching and refresh logic
- Used by API commands

### Other Infrastructure

**KeyManager**
- Manages encryption keys
- Get or create key from keychain

**InstanceLock**
- Prevents multiple app instances
- Uses PID file in temp directory

**Config Module**
- Configuration loading
- Database paths, pool sizes
- Feature settings

**HTTP Module**
- HttpClient with timeouts
- Configured with retry/circuit breaker

**Observability**
- Metrics collection (db, cache, calls, performance)
- Datadog exporter integration
- Observer patterns for instrumentation

---

## CRATE 5: pulsearc-lib (API)
**Purpose**: Tauri application layer - frontend bridge

### Application Context (DI Container)
```rust
pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<FeatureFlagService>,
    _instance_lock: InstanceLock,
}
```

### Commands Implemented (Tauri Bridge)

**Activity Tracking**
- `get_activity()` → ActivityContext (from tracking service)
- `pause_tracker()` → () — calls `TrackingService::pause()` and records command metrics
- `resume_tracker()` → () — resumes tracking via `TrackingService::resume()` with the same observability hooks

**Projects**
- `get_user_projects()` → Vec<Project> — fetches the latest SAP projects from `wbs_cache` (max 500 results)

**Suggestions & Blocks**
- `get_dismissed_suggestions()` → Vec<TimeEntryOutbox> — returns dismissed outbox rows via SqlCipherConnection
- `get_proposed_blocks(day_epoch, status?)` → Vec<ProposedBlock> — pulls proposed blocks for the given day with optional status filtering
- `get_outbox_status()` → Vec<TimeEntryOutbox> — legacy outbox view without status filtering

**Calendar**
- `get_calendar_events_for_timeline(start, end)` → Vec<TimelineCalendarEvent> — gated by the `new_calendar_commands` flag; queries encrypted calendar storage when enabled, otherwise returns an empty list

**Feature Flags** (Phase 4)
- `is_feature_enabled(flag, default)` → bool
- `toggle_feature_flag(flag, enabled)` → ()
- `list_feature_flags()` → Vec<FeatureFlag>

### Entry Point
- `run()` - Initializes Tauri builder with:
  - AppContext setup
  - Window blur effects (macOS native)
  - Encryption key resolution
  - Database migrations
  - Command registration

---

## Database Schema (SQLCipher)

### Core Tables
- `activity_snapshots` - Full activity captures
- `activity_segments` - Processed segments
- `time_entries` - Classified work entries
- `proposed_blocks` - Generated time blocks
- `calendar_events` - Calendar integrations
- `idle_periods` - Idle detection tracking (FEATURE-028)

### SAP/Project Tables
- `wbs_cache` - FTS5 full-text search table
- `projects` - Project definitions

### Sync Tables
- `time_entry_outbox` - Queue for remote sync
- `id_mappings` - Local ↔ Backend CUID mappings
- `token_usage` - AI token usage tracking
- `batches` - Batch processing queue with leases
- `dlq` - Dead letter queue for failed batches

### User Tables
- `users` - User profiles
- `user_preferences` - Settings and preferences

### Feature Management
- `feature_flags` - Toggle flags for Phase 4 rollback

### Full Schema
- Tables support: timestamps, encryption, soft deletes, status tracking
- Indexes on frequently queried fields
- Lease management with TTL for batch processing

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                  pulsearc-lib (API)                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ AppContext (DI): TrackingService, FeatureFlagService │   │
│  │ Tauri Commands: tracking, projects, suggestions, ... │   │
│  └────────────────────┬─────────────────────────────────┘   │
└─────────────────────┬──────────────────────────────────────┘
                      │
        ┌─────────────┴─────────────┐
        ▼                           ▼
   pulsearc-core              pulsearc-infra
   (Business Logic)           (Infrastructure)
   ┌──────────────┐           ┌─��────────────────┐
   │ Ports/Traits │◄──────────│ Implementations  │
   │ - Tracking   │           │ - Repositories   │
   │ - Classify   │           │ - Services       │
   │ - Sync       │           │ - Schedulers     │
   │ - Batch      │           │ - Platform       │
   │ - User       │           │ - Integrations   │
   └──────────────┘           └──────────────────┘
        ▲                           │
        │                    Uses: pulsearc-common
        │                           │
        └──────────────────────────┬──────────┐
                    ┌──────────────┴──────────┴────────┐
                    ▼                                   ▼
              pulsearc-domain                   pulsearc-common
              (Domain Models)                   (Shared Utils)
              ┌──────────────┐                 ┌──────────────┐
              │ Types: *     │                 │ foundation   │
              │ TimeEntry    │                 │ runtime      │
              │ Activity*    │                 │ platform     │
              │ SAP, etc     │                 │ observability│
              └──────────────┘                 └──────────────┘
```

---

## Dependency Flow

```
api
├─ common (features: platform, observability)
├─ domain
├─ core
└─ infra

infra
├─ common (all features)
├─ domain
└─ core

core
├─ common
└─ domain

domain
└─ (no pulsearc dependencies)

common
└─ (no pulsearc dependencies)
```

---

## Statistics

### Code Size (lines)
- infra/database: 5,724 lines across 16 repositories
- Largest repos: batch (676), outbox (548), calendar (483)
- core: ~1,500 lines (ports + services)
- common: ~3,500 lines (multi-module utilities)
- domain: ~1,000 lines (types only)

### Implementation Status
- ✅ Complete: Port interfaces (25+), domain types, repositories (14)
- ✅ Partial: API commands (tracking working, others TODO)
- ✅ Complete: Database schema, migrations
- ✅ Complete: Core services (tracking, classification)
- ⚠️ TODO: Pause/resume tracking, projects fetching, suggestions

### Feature Gates
- `calendar` - Calendar integration (Google, Microsoft)
- `sap` - SAP ERP integration
- `platform` - OAuth, keychain, RBAC
- `observability` - Metrics, tracing

---

## Key Design Patterns

1. **Hexagonal Architecture**: All logic in core via traits, all implementations in infra
2. **Repository Pattern**: 14 dedicated repositories for each domain entity
3. **Port/Adapter**: Clean boundaries between core and infrastructure
4. **Feature Gates**: Compile-time feature toggling for optional integrations
5. **Sync Strategy**: OutboxQueue + BatchRepository with lease management for reliable processing
6. **FTS5 Search**: Hybrid HashMap (hot) + FTS5 (cold) for project matching
7. **Lifecycle Management**: AsyncManager for component initialization/shutdown
8. **Error Handling**: CommonError with classification (Validation, NotFound, etc.)
