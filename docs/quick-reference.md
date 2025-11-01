# PulseArc Crates - Quick Reference Index

## File Locations

### Core Port Traits
- Activity Tracking: `/crates/core/src/tracking/ports.rs`
- Classification: `/crates/core/src/classification/ports.rs`
- Batch Processing: `/crates/core/src/batch/ports.rs`
- Sync Operations: `/crates/core/src/sync/ports.rs`
- User Management: `/crates/core/src/user/ports.rs`
- Feature Flags: `/crates/core/src/feature_flags_ports.rs`

### Domain Types
- Main Types: `/crates/domain/src/types/mod.rs`
- Classification: `/crates/domain/src/types/classification.rs`
- Database: `/crates/domain/src/types/database.rs`
- SAP: `/crates/domain/src/types/sap.rs`
- Stats: `/crates/domain/src/types/stats.rs`
- User: `/crates/domain/src/types/user.rs`
- Idle: `/crates/domain/src/types/idle.rs`

### Services (Core)
- Tracking: `/crates/core/src/tracking/service.rs`
- Classification: `/crates/core/src/classification/service.rs`

### Database Repositories (Infra)
- Activity: `/crates/infra/src/database/activity_repository.rs`
- Segment: `/crates/infra/src/database/segment_repository.rs`
- Block: `/crates/infra/src/database/block_repository.rs`
- Calendar: `/crates/infra/src/database/calendar_event_repository.rs`
- Idle Periods: `/crates/infra/src/database/idle_periods_repository.rs`
- Time Entry: `/crates/infra/src/database/time_entry_repository.rs` (NOT YET - uses TimeEntryRepository)
- WBS: `/crates/infra/src/database/wbs_repository.rs` (NOT YET - uses WbsRepository)
- Outbox: `/crates/infra/src/database/outbox_repository.rs`
- Batch: `/crates/infra/src/database/batch_repository.rs`
- DLQ: `/crates/infra/src/database/dlq_repository.rs`
- ID Mapping: `/crates/infra/src/database/id_mapping_repository.rs`
- Token Usage: `/crates/infra/src/database/token_usage_repository.rs`
- User Profile: `/crates/infra/src/database/user_profile_repository.rs`
- Feature Flags: `/crates/infra/src/database/feature_flags_repository.rs`

### Database Schema
- Schema: `/crates/infra/src/database/schema.sql`
- Manager: `/crates/infra/src/database/manager.rs`
- Pool: `/crates/infra/src/database/sqlcipher_pool.rs`

### Platform Code
- macOS Provider: `/crates/infra/src/platform/macos/activity_provider.rs`
- macOS Enrichers: `/crates/infra/src/platform/macos/enrichers/`
- Event Listener: `/crates/infra/src/platform/macos/event_listener.rs`

### API Integration
- Client: `/crates/infra/src/api/client.rs`
- Auth: `/crates/infra/src/api/auth.rs`
- Forwarder: `/crates/infra/src/api/forwarder.rs`
- Scheduler: `/crates/infra/src/api/scheduler.rs`

### Schedulers
- Block: `/crates/infra/src/scheduling/block_scheduler.rs`
- Classification: `/crates/infra/src/scheduling/classification_scheduler.rs`
- Sync: `/crates/infra/src/scheduling/sync_scheduler.rs`
- SAP: `/crates/infra/src/scheduling/sap_scheduler.rs` (feature = "sap")
- Calendar: `/crates/infra/src/scheduling/calendar_scheduler.rs` (feature = "calendar")

### Sync Services
- Outbox Worker: `/crates/infra/src/sync/outbox_worker.rs`
- Neon Client: `/crates/infra/src/sync/neon_client.rs`
- Cost Tracker: `/crates/infra/src/sync/cost_tracker.rs`
- Cleanup: `/crates/infra/src/sync/cleanup.rs`

### Integrations
- Calendar (feature = "calendar"): `/crates/infra/src/integrations/calendar/`
- SAP (feature = "sap"): `/crates/infra/src/integrations/sap/`
- OpenAI: `/crates/infra/src/integrations/openai/`

### API Commands
- Tracking: `/crates/api/src/commands/tracking.rs`
- Suggestions: `/crates/api/src/commands/suggestions.rs`
- Feature Flags: `/crates/api/src/commands/feature_flags.rs`
- Calendar: `/crates/api/src/commands/calendar.rs`
- Projects: `/crates/api/src/commands/projects.rs`

### Application Entry
- Context/DI: `/crates/api/src/context/mod.rs`
- Main: `/crates/api/src/main.rs`
- Commands: `/crates/api/src/commands/mod.rs`

### Common Utilities
- Error: `/crates/common/src/error/mod.rs`
- Validation: `/crates/common/src/validation/mod.rs`
- Cache: `/crates/common/src/cache/mod.rs`
- Crypto: `/crates/common/src/crypto/mod.rs`
- Privacy: `/crates/common/src/privacy/mod.rs`
- Resilience: `/crates/common/src/resilience/mod.rs`
- Auth: `/crates/common/src/auth/mod.rs`
- Security: `/crates/common/src/security/mod.rs`
- Storage: `/crates/common/src/storage/mod.rs`
- Lifecycle: `/crates/common/src/lifecycle/mod.rs`
- Time: `/crates/common/src/time/mod.rs`
- Sync Queue: `/crates/common/src/sync/mod.rs`
- Collections: `/crates/common/src/collections/mod.rs`
- Observability: `/crates/common/src/observability/mod.rs`
- Compliance: `/crates/common/src/compliance/mod.rs`
- Testing: `/crates/common/src/testing/mod.rs`

---

## Type Hierarchy

### Core Activity Types
```
ActivityContext (from OS, enriched)
├─ active_app: WindowContext
├─ recent_apps: Vec<WindowContext>
├─ detected_activity: String
├─ work_type: Option<WorkType>
├─ activity_category: ActivityCategory
├─ extracted_metadata: ActivityMetadata
├─ calendar_event: Option<CalendarEventContext>
├─ location: Option<LocationContext>
├─ temporal_context: Option<TemporalContext>
└─ classification: Option<ClassificationContext>

ActivitySnapshot (persisted)
├─ id: String
├─ timestamp: i64
├─ app_name: String
├─ window_title: String
├─ active: bool
├─ metadata: SnapshotMetadata
└─ [context fields]

TimeEntry (classified)
├─ id: Uuid
├─ start_time: DateTime<Utc>
├─ end_time: Option<DateTime<Utc>>
├─ duration_seconds: Option<i64>
├─ project_id: Option<String>
├─ wbs_code: Option<String>
├─ description: String
└─ [other metadata]

ProposedBlock
├─ id: String
├─ day_epoch: i64
├─ start_time: i64
├─ end_time: i64
└─ status: String
```

### Enum Types
```
WorkType: Modeling, DocReview, Research, Email, Meeting, DMS, DataRoom, 
          AccountingSuite, Documentation, Unknown

ActivityCategory: ClientWork, Research, Communication, Administrative, 
                 Meeting, Documentation, Internal

BatchStatus: Pending, Processing, Completed, Failed, DLQ

ClassificationMode: [values from stats]
```

---

## Key Implementations to Compare Against Legacy

### Must Migrate From Legacy
1. **TimeEntry classification logic** - Check legacy classifier algorithm
2. **ActivityContext enrichment** - Compare legacy enrichers (browser, office, calendar)
3. **Project matching logic** - FTS5 search vs legacy regex/HashMap
4. **Idle period detection** - Algorithm and thresholds (FEATURE-028)
5. **WBS caching strategy** - Hybrid HashMap + FTS5
6. **Sync protocol** - Outbox format and retry logic
7. **Calendar sync** - Google/Microsoft OAuth and event parsing
8. **SAP integration** - Auth, batch forwarding, error handling

### Already Implemented
- Port interfaces (25+)
- Domain type definitions
- Database schema and 14 repositories
- Core services (tracking, classification)
- macOS Accessibility API integration
- Feature flag system (Phase 4)
- API client infrastructure
- Scheduler framework

---

## Repository Method Reference

### ActivityRepository (async)
```rust
save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()>
get_snapshots(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ActivitySnapshot>>
delete_old_snapshots(&self, before: DateTime<Utc>) -> Result<usize>
```

### SegmentRepository (sync)
```rust
save_segment(&self, segment: &ActivitySegment) -> CommonResult<()>
find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>>
find_unprocessed_segments(&self, limit: usize) -> CommonResult<Vec<ActivitySegment>>
mark_processed(&self, segment_id: &str) -> CommonResult<()>
```

### TimeEntryRepository (async)
```rust
save_entry(&self, entry: TimeEntry) -> Result<()>
get_entries(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<TimeEntry>>
update_entry(&self, entry: TimeEntry) -> Result<()>
delete_entry(&self, id: Uuid) -> Result<()>
```

### BlockRepository (async)
```rust
save_proposed_block(&self, block: &ProposedBlock) -> Result<()>
get_proposed_blocks(&self, date: NaiveDate) -> Result<Vec<ProposedBlock>>
```

### WbsRepository (sync)
```rust
count_active_wbs(&self) -> Result<i64>
get_last_sync_timestamp(&self) -> Result<Option<i64>>
load_common_projects(&self, limit: usize) -> Result<Vec<WbsElement>>
fts5_search_keyword(&self, keyword: &str, limit: usize) -> Result<Vec<WbsElement>>
get_wbs_by_project_def(&self, project_def: &str) -> Result<Option<WbsElement>>
get_wbs_by_wbs_code(&self, wbs_code: &str) -> Result<Option<WbsElement>>
```

### BatchRepository (async)
```rust
save_batch(&self, batch: &BatchQueue) -> Result<()>
get_batch(&self, batch_id: &str) -> Result<BatchQueue>
update_batch_status(&self, batch_id: &str, status: BatchStatus) -> Result<()>
acquire_batch_lease(&self, batch_id: &str, worker_id: &str, duration: Duration) -> Result<()>
renew_batch_lease(&self, batch_id: &str, worker_id: &str, duration: Duration) -> Result<()>
create_batch_from_unprocessed(&self, max_snapshots: usize, worker_id: &str, lease_duration_secs: i64) -> Result<Option<(String, Vec<String>)>>
complete_batch(&self, batch_id: &str) -> Result<()>
mark_batch_failed(&self, batch_id: &str, error: &str) -> Result<()>
get_batches_by_status(&self, status: BatchStatus) -> Result<Vec<BatchQueue>>
get_batch_stats(&self) -> Result<BatchStats>
get_pending_batches(&self) -> Result<Vec<BatchQueue>>
cleanup_old_batches(&self, older_than_seconds: i64) -> Result<usize>
delete_batch(&self, batch_id: &str) -> Result<()>
```

### CalendarEventRepository (async)
```rust
find_event_by_timestamp(&self, timestamp: i64, window_secs: i64) -> Result<Option<CalendarEventRow>>
insert_calendar_event(&self, params: CalendarEventParams) -> Result<()>
get_calendar_events_by_time_range(&self, user_email: &str, start_ts: i64, end_ts: i64) -> Result<Vec<CalendarEventRow>>
get_today_calendar_events(&self) -> Result<Vec<CalendarEventRow>>
delete_calendar_events_older_than(&self, days: i64) -> Result<usize>
```

### IdlePeriodsRepository (async)
```rust
save_idle_period(&self, period: IdlePeriod) -> Result<()>
get_idle_period(&self, id: &str) -> Result<Option<IdlePeriod>>
get_idle_periods_in_range(&self, start_ts: i64, end_ts: i64) -> Result<Vec<IdlePeriod>>
get_pending_idle_periods(&self) -> Result<Vec<IdlePeriod>>
update_idle_period_action(&self, id: &str, user_action: &str, notes: Option<String>) -> Result<()>
delete_idle_periods_before(&self, before_ts: i64) -> Result<usize>
```

---

## Configuration

### AppContext (DI Container)
```rust
pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<FeatureFlagService>,
    _instance_lock: InstanceLock,
}
```

### TrackingService Construction
```rust
let provider = Arc::new(MacOsActivityProvider::new());
let repository = Arc::new(SqlCipherActivityRepository::new(db.clone()));
let tracking_service = Arc::new(TrackingService::new(provider, repository));
```

---

## Test Coverage

Tests exist in:
- `/crates/core/tests/` - Service integration tests
- `/crates/common/tests/` - Utility tests
- `/crates/infra/tests/` - Integration tests (date query perf, config loader, outbox, etc.)
- `/crates/infra/examples/` - MDM remote config example

---

## Features & Compilation

### Enable All Features
```bash
cargo build --all-features
```

### Feature Combinations
```bash
# Core only (no integrations)
cargo build

# With calendar
cargo build --features calendar

# With SAP
cargo build --features sap

# With observability
cargo build --features observability

# All integrations
cargo build --all-features
```

---

## Common Patterns

### Using Repositories
```rust
// Async
let entries = repository.get_entries(start, end).await?;
repository.save_entry(entry).await?;

// Sync (SegmentRepository, WbsRepository, SnapshotRepository)
let segments = repository.find_segments_by_date(date)?;
repository.save_segment(segment)?;
```

### Error Handling
```rust
// Domain errors (use Result<T> from domain)
use pulsearc_domain::Result;

// Common errors (use CommonResult<T> from common)
use pulsearc_common::CommonResult;

// Convert: CommonError -> DomainError
error.map_err(|e| PulseArcError::from(e))?
```

### Async Service Usage
```rust
// Injected via DI
let ctx = State<Arc<AppContext>>;
let activity = ctx.tracking_service.capture_activity().await?;
```

---

## Next Steps for Comparison

1. **Load legacy code** - Extract classifiers, enrichers, matchers
2. **Compare type definitions** - Map legacy DTO → new domain types
3. **Port business logic** - Migrate classifier algorithms to infra implementations
4. **Test equivalence** - Ensure new implementation matches legacy behavior
5. **Migrate enrichers** - Browser, office, AppleScript detection
6. **Implement missing repos** - TimeEntryRepository, WbsRepository implementations
7. **Complete API commands** - Fill in TODO placeholders
8. **Sync protocol** - Compare outbox format and retry strategy