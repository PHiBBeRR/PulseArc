# PulseArc Infrastructure

Infrastructure implementations of core domain ports - databases, HTTP clients, platform APIs, and external integrations.

## Overview

The `pulsearc-infra` crate provides **concrete implementations** of all port interfaces defined in the Core layer. It contains all "impure" code that interacts with the outside world: databases, HTTP APIs, file systems, and platform-specific APIs (macOS Accessibility).

## Architecture Role

The Infra crate implements **adapters** for the hexagonal architecture:

```
┌─────────────────────────────────────┐
│          CORE LAYER                 │
│  ┌────────────────────────────┐     │
│  │   Business Logic           │     │
│  │  • Services                │     │
│  │  • Use Cases               │     │
│  └────────────────────────────┘     │
│  ┌────────────────────────────┐     │
│  │   Ports (Interfaces)       │ ◄───┼───┐
│  │  • Repository Traits       │     │   │
│  │  • Service Traits          │     │   │
│  │  • Provider Traits         │     │   │
│  └────────────────────────────┘     │   │
└─────────────────────────────────────┘   │
                                          │
┌──────────────────────────────────────┐  ▼
│          INFRA LAYER (Adapters)      │ ← You are here
│  ┌────────────────────────────────┐  │
│  │   Database (SQLCipher)         │  │
│  │  • Repository implementations  │  │
│  │  • Schema management           │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │   HTTP Clients                 │  │
│  │  • API client (backend)        │  │
│  │  • OAuth client                │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │   Platform (macOS)             │  │
│  │  • Accessibility API           │  │
│  │  • Event listener              │  │
│  │  • Activity enrichers          │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │   Schedulers (Background Jobs) │  │
│  │  • Block scheduler             │  │
│  │  • Classification scheduler    │  │
│  │  • Sync scheduler              │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │   Integrations                 │  │
│  │  • Calendar (feature-gated)    │  │
│  │  • SAP (feature-gated)         │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

**Key Responsibilities:**
- **Database**: SQLCipher encrypted database with schema management
- **HTTP**: HTTP client implementations for backend APIs
- **Platform**: macOS Accessibility API integration for activity tracking
- **Schedulers**: Background job schedulers using cron expressions
- **Integrations**: External service integrations (Calendar, SAP)
- **Sync**: Data synchronization with outbox pattern and cleanup

## Directory Structure

```
crates/infra/
├── src/
│   ├── api/                        # API client implementations
│   │   ├── auth.rs                # OAuth authentication wrapper
│   │   ├── client.rs              # HTTP client for backend API
│   │   ├── commands.rs            # API command handlers
│   │   ├── errors.rs              # API-specific errors
│   │   ├── forwarder.rs           # Request forwarding middleware
│   │   ├── mod.rs                 # Module exports
│   │   └── scheduler.rs           # API request scheduler
│   ├── config/                     # Configuration loading
│   │   ├── mod.rs                 # AppConfig loader
│   │   └── settings.rs            # Settings management
│   ├── database/                   # Database layer (SQLCipher)
│   │   ├── activity_repository.rs    # ActivityRepository impl
│   │   ├── batch_repository.rs       # BatchRepository impl
│   │   ├── block_repository.rs       # BlockRepository impl
│   │   ├── calendar_event_repository.rs # CalendarEventRepository impl
│   │   ├── dlq_repository.rs         # DlqRepository impl
│   │   ├── feature_flags_repository.rs # FeatureFlagsPort impl
│   │   ├── id_mapping_repository.rs  # IdMappingRepository impl
│   │   ├── idle_periods_repository.rs # Idle tracking repository
│   │   ├── manager.rs                # DbManager (connection pool)
│   │   ├── mod.rs                    # Database module exports
│   │   ├── outbox_repository.rs      # OutboxQueue impl
│   │   ├── repository.rs             # Base repository utilities
│   │   ├── schema.sql                # Database schema (migrations)
│   │   ├── segment_repository.rs     # SegmentRepository impl
│   │   ├── sqlcipher_pool.rs         # SQLCipher connection pool
│   │   ├── token_usage_repository.rs # TokenUsageRepository impl
│   │   └── user_profile_repository.rs # UserProfileRepository impl
│   ├── errors/                     # Infrastructure errors
│   │   ├── api.rs                 # API errors
│   │   └── mod.rs                 # Error exports
│   ├── http/                       # HTTP client infrastructure
│   │   ├── client.rs              # Generic HTTP client
│   │   ├── mod.rs                 # HTTP module exports
│   │   └── retry.rs               # HTTP retry logic
│   ├── integrations/               # External integrations
│   │   ├── calendar/              # Calendar integration (feature-gated)
│   │   │   ├── client.rs          # Calendar API client
│   │   │   ├── mod.rs             # Calendar module exports
│   │   │   ├── provider.rs        # Calendar provider implementation
│   │   │   └── README.md          # Calendar documentation
│   │   └── sap/                   # SAP integration (feature-gated)
│   │       ├── client.rs          # SAP API client
│   │       ├── mod.rs             # SAP module exports
│   │       └── provider.rs        # SAP provider implementation
│   ├── mdm/                        # Mobile Device Management
│   │   ��── mod.rs                 # MDM module exports
│   │   └── README.md              # MDM documentation
│   ├── observability/              # Metrics and monitoring
│   │   ├── metrics.rs             # Metrics collection
│   │   ├── mod.rs                 # Observability exports
│   │   └── tracing.rs             # Distributed tracing
│   ├── platform/                   # Platform-specific code (macOS)
│   │   └── macos/                 # macOS implementations
│   │       ├── enrichers/         # Activity enrichment
│   │       │   ├── applescript_helpers.rs # AppleScript utilities
│   │       │   ├── browser.rs     # Browser URL extraction
│   │       │   ├── cache.rs       # Enrichment caching
│   │       │   ├── mod.rs         # Enricher exports
│   │       │   └── office.rs      # Office app enrichment
│   │       ├── accessibility.rs   # macOS Accessibility API
│   │       ├── event_listener.rs  # Activity event listener
│   │       └── mod.rs             # macOS platform exports
│   ├── scheduling/                 # Background job schedulers
│   │   ├── block_scheduler.rs     # Time block creation scheduler
│   │   ├── calendar_scheduler.rs  # Calendar sync scheduler (feature-gated)
│   │   ├── classification_scheduler.rs # Classification scheduler
│   │   ├── error.rs               # Scheduler errors
│   │   ├── mod.rs                 # Scheduler module exports
│   │   ├── sap_scheduler.rs       # SAP sync scheduler (feature-gated)
│   │   └── sync_scheduler.rs      # Data sync scheduler
│   ├── services/                   # Infrastructure services
│   │   ├── feature_flags.rs       # Feature flag service
│   │   └── mod.rs                 # Service exports
│   ├── sync/                       # Data synchronization
│   │   ├── cleanup.rs             # Cleanup service (old data)
│   │   ├── cost_tracker.rs        # API cost tracking
│   │   ├── mod.rs                 # Sync module exports
│   │   ├── neon_client.rs         # Neon PostgreSQL client
│   │   └── outbox_worker.rs       # Outbox pattern worker
│   ├── errors.rs                   # Infra-level errors
│   ├── instance_lock.rs            # Single instance lock
│   ├── key_manager.rs              # Encryption key manager
│   └── lib.rs                      # Library exports
├── Cargo.toml
└── README.md                        # This file
```

## Database Layer ([`database/`](src/database/))

### DbManager ([`database/manager.rs`](src/database/manager.rs))

Central database connection manager using SQLCipher connection pooling.

```rust
use pulsearc_infra::DbManager;

let db_manager = DbManager::new("./data/pulsearc.db", "encryption_key").await?;

// Get a connection from the pool
let conn = db_manager.get_connection().await?;
```

**Features:**
- **SQLCipher encryption**: Database encrypted at rest with AES-256
- **Connection pooling**: Efficient connection reuse with `r2d2`
- **Schema migrations**: Automatic schema creation and versioning
- **WAL mode**: Write-Ahead Logging for concurrent reads/writes
- **Pragma optimization**: Performance-tuned SQLite settings

### Repository Implementations

All Core ports are implemented as database repositories:

| Port (Core) | Implementation (Infra) | File |
|-------------|------------------------|------|
| `ActivityRepository` | `SqlActivityRepository` | `activity_repository.rs` |
| `SegmentRepository` | `SqlSegmentRepository` | `segment_repository.rs` |
| `SnapshotRepository` | *(part of activity repo)* | `activity_repository.rs` |
| `BlockRepository` | `SqlBlockRepository` | `block_repository.rs` |
| `TimeEntryRepository` | *(part of block repo)* | `block_repository.rs` |
| `WbsRepository` | *(placeholder)* | TBD |
| `BatchRepository` | `SqlBatchRepository` | `batch_repository.rs` |
| `DlqRepository` | `SqlDlqRepository` | `dlq_repository.rs` |
| `OutboxQueue` | `SqlOutboxRepository` | `outbox_repository.rs` |
| `IdMappingRepository` | `SqlIdMappingRepository` | `id_mapping_repository.rs` |
| `TokenUsageRepository` | `SqlTokenUsageRepository` | `token_usage_repository.rs` |
| `UserProfileRepository` | `SqlUserProfileRepository` | `user_profile_repository.rs` |
| `CalendarEventRepository` | `SqlCalendarEventRepository` | `calendar_event_repository.rs` |
| `FeatureFlagsPort` | `SqlFeatureFlagsRepository` | `feature_flags_repository.rs` |

**Example Usage:**
```rust
use pulsearc_infra::database::{DbManager, SqlActivityRepository};
use pulsearc_core::ActivityRepository;

let db_manager = DbManager::new("./data.db", "key").await?;
let repo = SqlActivityRepository::new(db_manager.clone());

// Save an activity
repo.save(&activity).await?;

// Query activities by time range
let activities = repo.find_by_time_range(start, end).await?;
```

### Schema ([`database/schema.sql`](src/database/schema.sql))

The database schema includes:
- **activities** - Raw activity events
- **segments** - Time segments (contiguous periods)
- **snapshots** - Point-in-time activity snapshots
- **blocks** - Classified time blocks
- **time_entries** - Billable time entries
- **batches** - Batch processing metadata
- **dlq** - Dead-letter queue for failed items
- **outbox** - Outbox pattern for eventual sync
- **id_mappings** - Local-to-remote ID mappings
- **token_usage** - API token usage tracking
- **user_profiles** - User settings and preferences
- **calendar_events** - Calendar event cache
- **feature_flags** - Feature flag storage
- **idle_periods** - Idle time tracking

## Platform Layer ([`platform/macos/`](src/platform/macos/))

### Activity Provider ([`platform/macos/accessibility.rs`](src/platform/macos/accessibility.rs))

Implements `ActivityProvider` port using macOS Accessibility API.

```rust
use pulsearc_infra::platform::macos::MacOSActivityProvider;
use pulsearc_core::ActivityProvider;

let provider = MacOSActivityProvider::new();

// Get current activity
let activity = provider.get_current_activity().await?;
println!("App: {}, Window: {}", activity.app_name, activity.window_title);
```

**Capabilities:**
- Query frontmost application
- Extract window titles
- Get bundle IDs
- Detect idle time

**Permissions Required:**
- macOS Accessibility permission (user approval)

### Activity Enrichers ([`platform/macos/enrichers/`](src/platform/macos/enrichers/))

Enrichers add additional context to activities:

| Enricher | Purpose | Implementation |
|----------|---------|----------------|
| **Browser Enricher** | Extract URLs from browser tabs | `browser.rs` |
| **Office Enricher** | Extract file paths from Office apps | `office.rs` |
| **Cache Enricher** | Cache enriched data with TTL | `cache.rs` |

**Browser Enricher Example:**
```rust
use pulsearc_infra::platform::macos::BrowserEnricher;
use pulsearc_core::ActivityEnricher;

let enricher = BrowserEnricher::new();

let mut activity = ActivityContext {
    app_name: "Google Chrome".to_string(),
    window_title: "PulseArc - GitHub".to_string(),
    ..Default::default()
};

// Enrich with URL
enricher.enrich(&mut activity).await?;
assert!(activity.url.is_some());
```

**AppleScript Helpers ([`applescript_helpers.rs`](src/platform/macos/enrichers/applescript_helpers.rs)):**
- Execute AppleScript with timeout protection
- Extract browser URLs from Chrome, Safari, Firefox, Edge
- Extract file paths from Microsoft Office apps
- Process management and error handling

### Event Listener ([`platform/macos/event_listener.rs`](src/platform/macos/event_listener.rs))

Background service that listens for activity changes and triggers tracking.

```rust
use pulsearc_infra::platform::macos::EventListener;

let listener = EventListener::new(db_manager);
listener.start().await?;

// Automatically captures activities at configured intervals
```

## API Client ([`api/`](src/api/))

### HTTP Client ([`api/client.rs`](src/api/client.rs))

HTTP client for communicating with the PulseArc backend API.

```rust
use pulsearc_infra::ApiClient;

let client = ApiClient::new("https://api.pulsearc.com", "oauth_token")?;

// Sync time entries
client.sync_time_entries(&entries).await?;

// Fetch project list
let projects = client.fetch_projects().await?;
```

**Features:**
- OAuth token injection
- Automatic retry with exponential backoff
- Circuit breaker protection
- Request/response logging
- Error handling and classification

### API Commands ([`api/commands.rs`](src/api/commands.rs))

Command pattern for API operations:

```rust
pub trait ApiCommand {
    type Response;
    async fn execute(&self) -> Result<Self::Response, ApiError>;
}
```

### API Scheduler ([`api/scheduler.rs`](src/api/scheduler.rs))

Schedule API requests with rate limiting and retry logic.

## Schedulers ([`scheduling/`](src/scheduling/))

Background job schedulers using `tokio-cron-scheduler`:

### Block Scheduler ([`scheduling/block_scheduler.rs`](src/scheduling/block_scheduler.rs))

**Job:** Create time blocks from raw activity segments
**Schedule:** Every 5 minutes (configurable via cron)

```rust
use pulsearc_infra::BlockScheduler;

let scheduler = BlockScheduler::new(config, db_manager, tracking_service);
scheduler.start().await?;

// Runs automatically on schedule
```

### Classification Scheduler ([`scheduling/classification_scheduler.rs`](src/scheduling/classification_scheduler.rs))

**Job:** Classify unclassified time blocks
**Schedule:** Every hour (configurable via cron)

### Sync Scheduler ([`scheduling/sync_scheduler.rs`](src/scheduling/sync_scheduler.rs))

**Job:** Process outbox queue and sync data to remote API
**Schedule:** Every 15 minutes (configurable via cron)

### Calendar Scheduler ([`scheduling/calendar_scheduler.rs`](src/scheduling/calendar_scheduler.rs))
*Feature gated: `calendar`*

**Job:** Fetch calendar events and store for context enrichment
**Schedule:** Every 30 minutes (configurable via cron)

### SAP Scheduler ([`scheduling/sap_scheduler.rs`](src/scheduling/sap_scheduler.rs))
*Feature gated: `sap`*

**Job:** Submit approved time entries to SAP
**Schedule:** Daily at configured time (configurable via cron)

## Sync Layer ([`sync/`](src/sync/))

### Outbox Worker ([`sync/outbox_worker.rs`](src/sync/outbox_worker.rs))

Implements the **Transactional Outbox Pattern** for reliable data synchronization.

```rust
use pulsearc_infra::OutboxWorker;

let worker = OutboxWorker::new(config, db_manager, api_client);
worker.start().await?;

// Processes outbox items in background
```

**Features:**
- Automatic retry with exponential backoff
- Dead-letter queue for permanent failures
- At-least-once delivery guarantee
- Idempotent message processing

### Cleanup Service ([`sync/cleanup.rs`](src/sync/cleanup.rs))

Periodically removes old data to prevent database bloat.

```rust
use pulsearc_infra::CleanupService;

let cleanup = CleanupService::new(db_manager, retention_days: 90);
cleanup.run().await?;
```

**Cleanup Policies:**
- Activities older than 90 days (configurable)
- Processed outbox items older than 30 days
- DLQ items reviewed and archived

### Cost Tracker ([`sync/cost_tracker.rs`](src/sync/cost_tracker.rs))

Track API usage and costs for billing/budgeting.

```rust
use pulsearc_infra::CostTracker;

let tracker = CostTracker::new(db_manager);
tracker.record_usage("openai_api", tokens_used, cost_usd).await?;

let total_cost = tracker.get_total_cost_this_month().await?;
```

### Neon Client ([`sync/neon_client.rs`](src/sync/neon_client.rs))

PostgreSQL client for Neon (cloud backend database).

## Integrations ([`integrations/`](src/integrations/))

### Calendar Integration ([`integrations/calendar/`](src/integrations/calendar/))
*Feature gated: `calendar`*

**Providers:**
- macOS Calendar
- Google Calendar (OAuth)
- Microsoft Outlook (OAuth)

**Usage:**
```rust
use pulsearc_infra::integrations::calendar::CalendarProvider;

let provider = CalendarProvider::new_macos();
let events = provider.fetch_events(start, end).await?;
```

**See:** [`integrations/calendar/README.md`](src/integrations/calendar/README.md)

### SAP Integration ([`integrations/sap/`](src/integrations/sap/))
*Feature gated: `sap`*

**Capabilities:**
- Submit time entries to SAP
- Validate WBS codes
- Retrieve project metadata

```rust
use pulsearc_infra::integrations::sap::SapClient;

let client = SapClient::new(sap_config);
client.submit_time_entries(&entries).await?;
```

## HTTP Client ([`http/`](src/http/))

### HttpClient ([`http/client.rs`](src/http/client.rs))

Generic HTTP client with retry and circuit breaker support.

```rust
use pulsearc_infra::HttpClient;

let client = HttpClient::builder()
    .base_url("https://api.example.com")
    .timeout(Duration::from_secs(30))
    .retry_policy(RetryPolicy::default())
    .build()?;

let response: MyData = client.get("/endpoint").send().await?;
```

**Features:**
- Automatic retries on transient failures
- Circuit breaker protection
- Request/response logging with `tracing`
- JSON serialization/deserialization
- Bearer token authentication

## Configuration ([`config/`](src/config/))

Load configuration from environment variables and config files:

```rust
use pulsearc_infra::load_config;

let config = load_config()?;
println!("Database path: {:?}", config.database_path);
```

**Configuration Sources:**
1. `.env` file (development)
2. Environment variables (production)
3. Config file (`config.toml`)

## Error Handling ([`errors/`](src/errors/))

Infrastructure-specific error types:

```rust
use pulsearc_infra::InfraError;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Platform API error: {0}")]
    Platform(String),

    #[error(transparent)]
    Common(#[from] CommonError),
}
```

## Dependencies

```toml
[dependencies]
# Internal crates
pulsearc-common = { workspace = true, features = ["platform", "observability"] }
pulsearc-domain = { workspace = true }
pulsearc-core = { workspace = true }

# Database
rusqlite = { workspace = true }
r2d2 = { workspace = true }
r2d2_sqlite = { workspace = true }

# HTTP
reqwest = { workspace = true }

# OAuth
oauth2 = { workspace = true }
axum = { workspace = true }

# Async runtime
tokio = { workspace = true }
async-trait = { workspace = true }

# Scheduling
tokio-cron-scheduler = { workspace = true }

# Caching (enrichment cache)
moka = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
anyhow = { workspace = true }
thiserror = { workspace = true }

# Observability
tracing = { workspace = true }
log = { workspace = true }

# macOS Platform (only on macOS)
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = { workspace = true }
objc2-foundation = { workspace = true }
objc2-app-kit = { workspace = true }
block2 = { workspace = true }
cocoa = "0.26"
core-foundation = "0.10"
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `calendar` | Calendar integration | ❌ |
| `sap` | SAP time entry submission | ❌ |
| `openai` | OpenAI API integration | ❌ |
| `tree-classifier` | Decision tree classifier | ❌ |
| `ml` | Machine learning features | ❌ |
| `graphql` | GraphQL API support | ❌ |
| `audit-compliance` | Audit logging and compliance | ❌ |
| `test-utils` | Testing utilities | ❌ |
| `ts-gen` | TypeScript type generation | ❌ |

## Testing

```bash
# Run all tests
cargo test -p pulsearc-infra

# Test with features
cargo test -p pulsearc-infra --features calendar,sap

# Integration tests (requires database)
cargo test -p pulsearc-infra --test integration_tests

# Test specific module
cargo test -p pulsearc-infra --lib database::tests
```

**Test Database:**
Integration tests use a temporary SQLCipher database that is created and destroyed for each test.

## Platform Support

- **macOS**: Full support (primary platform)
- **Linux**: Not supported (macOS-only Tauri app)
- **Windows**: Not supported

## Security

- **SQLCipher**: Database encryption with AES-256
- **Keychain Integration**: OAuth tokens stored in macOS Keychain
- **Key Management**: Encryption key rotation support
- **HTTPS Only**: All HTTP clients enforce HTTPS
- **Token Validation**: OAuth token validation and refresh

## Performance Considerations

### Database Optimization
- **Connection pooling**: Reuse connections with `r2d2`
- **WAL mode**: Concurrent reads/writes
- **Prepared statements**: Reduce parsing overhead
- **Batch inserts**: Group multiple inserts into transactions

### HTTP Client
- **Connection pooling**: Reuse HTTP connections (via `reqwest`)
- **Timeouts**: Prevent hanging requests
- **Circuit breakers**: Fail fast on repeated errors
- **Retry backoff**: Exponential backoff to reduce load

### Platform API (macOS)
- **AppleScript timeout**: 5-second timeout on AppleScript execution
- **Enrichment caching**: Cache enriched data with 5-minute TTL
- **Background processing**: Offload enrichment to background tasks

## Best Practices

1. **Use SqlCipherConnection**: Always use `SqlCipherConnection` from `pulsearc-common` (not `LocalDatabase`)
2. **Connection Pooling**: Always get connections from `DbManager` pool
3. **Async/Await**: All I/O operations must be async
4. **Error Handling**: Use `InfraError` and compose with `CommonError`
5. **Logging**: Use `tracing` for structured logging (not `println!`)
6. **Resource Cleanup**: Ensure connections, files, and handles are properly closed
7. **Testing**: Mock all external dependencies (HTTP, database, file system)

## See Also

- [Core Layer](../core/README.md) - Port interfaces implemented by this layer
- [API Layer](../api/README.md) - Dependency injection and orchestration
- [Domain Layer](../domain/README.md) - Domain models persisted by repositories
- [Common Layer](../common/README.md) - Shared utilities (SqlCipherConnection, error handling, resilience)
- [CLAUDE.md](../../CLAUDE.md) - Project-wide development rules
- [SQLCIPHER-API-REFERENCE.md](../../docs/issues/SQLCIPHER-API-REFERENCE.md) - SqlCipher API usage guide