# PulseArc Core

Pure business logic layer implementing domain-driven design with hexagonal architecture.

## Overview

The `pulsearc-core` crate contains the **heart of PulseArc's business logic** - all use cases, domain services, and port interfaces. It is completely independent of infrastructure concerns (databases, HTTP, UI, etc.) and only depends on domain models and common utilities.

## Architecture Role

The Core crate implements **hexagonal architecture** (ports & adapters pattern):

```
┌─────────────────────────────────────┐
│              API Layer              │
│         (Tauri Commands)            │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│          CORE LAYER                 │ ← You are here
│  ┌────────────────────────────┐     │
│  │   Business Logic           │     │
│  │  • Services                │     │
│  │  • Use Cases               │     │
│  │  • Business Rules          │     │
│  └────────────────────────────┘     │
│  ┌────────────────────────────┐     │
│  │   Ports (Interfaces)       │     │
│  │  • Repository Traits       │     │
│  │  • Service Traits          │     │
│  │  • Provider Traits         │     │
│  └────────────────────────────┘     │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│          INFRA LAYER                │
│   (Port Implementations)            │
│  • Database Repositories            │
│  • HTTP Clients                     │
│  • Platform APIs                    │
└─────────────────────────────────────┘
```

**Key Principles:**
- **Pure Business Logic**: No database, HTTP, or platform code
- **Port/Adapter Pattern**: All external dependencies defined as traits
- **Testability**: Mock all ports for unit testing
- **Domain-Driven Design**: Rich domain models with encapsulated behavior
- **Technology Independence**: Swap infrastructure without changing business logic

## Directory Structure

```
crates/core/
├── src/
│   ├── batch/                     # Batch processing domain
│   │   ├── mod.rs                 # Batch service and logic
│   │   └── ports.rs               # BatchRepository, DlqRepository
│   ├── classification/            # Time entry classification
│   │   ├── block_builder.rs       # Build time blocks from segments
│   │   ├── evidence_extractor.rs  # Extract evidence from activities
│   │   ├── mod.rs                 # ClassificationService
│   │   ├── ports.rs               # Classification ports
│   │   ├── project_matcher.rs     # Match activities to projects
│   │   ├── service.rs             # Classification use cases
│   │   └── signal_extractor.rs    # Extract signals from activities
│   ├── sync/                      # Data synchronization domain
│   │   ├── mod.rs                 # Sync logic
│   │   └── ports.rs               # OutboxQueue, IdMappingRepository, TokenUsageRepository
│   ├── tracking/                  # Activity tracking domain
│   │   ├── mod.rs                 # TrackingService exports
│   │   ├── ports.rs               # Tracking ports
│   │   └── service.rs             # Tracking use cases
│   ├── user/                      # User profile domain
│   │   ├── mod.rs                 # User service
│   │   └── ports.rs               # UserProfileRepository
│   ├── utils/                     # Core utilities
│   │   └── patterns.rs            # Signal extraction patterns
│   ├── calendar_ports.rs          # Calendar integration ports (feature-gated)
│   ├── feature_flags_ports.rs     # Feature flag management ports
│   ├── lib.rs                     # Module exports
│   └── sap_ports.rs               # SAP integration ports (feature-gated)
├── Cargo.toml
└── README.md                      # This file
```

## Core Domains

### 1. Tracking ([`tracking/`](src/tracking/))

**Responsibility:** Capture and manage user activity data from macOS Accessibility API.

**Ports:**
- `ActivityProvider` - Fetch raw activity events from the platform
- `ActivityEnricher` - Enrich activities with additional context (browser tabs, app titles)
- `ActivityRepository` - Persist activity events
- `SegmentRepository` - Store time segments (contiguous periods of activity)
- `SnapshotRepository` - Store point-in-time activity snapshots
- `CalendarEventRepository` - Store calendar event context

**Service:** `TrackingService`

**Use Cases:**
- Start/stop activity tracking
- Capture periodic snapshots of current activity
- Create time segments from activity streams
- Enrich activities with application-specific context

**Example:**
```rust
use pulsearc_core::{TrackingService, ActivityProvider};

let service = TrackingService::new(
    activity_provider,
    activity_repository,
    segment_repository,
    snapshot_repository,
);

// Start tracking
service.start_tracking().await?;

// Get current activity
let snapshot = service.get_current_snapshot().await?;

// Stop tracking
service.stop_tracking().await?;
```

### 2. Classification ([`classification/`](src/classification/))

**Responsibility:** Classify time segments into billable time entries with project codes and evidence.

**Ports:**
- `BlockRepository` - Persist time blocks (classified segments)
- `TimeEntryRepository` - Store finalized time entries
- `WbsRepository` - Access WBS (Work Breakdown Structure) / project codes
- `Classifier` - ML-based classification engine (feature-gated)
- `ProjectMatcher` - Match activities to projects using signals

**Service:** `ClassificationService`

**Key Components:**
- **`BlockBuilder`** - Aggregates activity segments into logical time blocks
- **`SignalExtractor`** - Extracts classification signals (URLs, app names, window titles)
- **`EvidenceExtractor`** - Builds human-readable evidence for classifications
- **`ProjectMatcher`** - Uses signals to match activities to project codes

**Use Cases:**
- Build time blocks from raw activity segments
- Extract signals and evidence from activities
- Classify time blocks to project codes
- Generate time entry suggestions

**Example:**
```rust
use pulsearc_core::ClassificationService;

let service = ClassificationService::new(
    block_repository,
    time_entry_repository,
    wbs_repository,
    project_matcher,
);

// Classify a time segment
let classification = service.classify_segment(segment_id).await?;

// Get suggestions for a time range
let suggestions = service.get_suggestions(start, end).await?;
```

**Signal Extraction:**

The classification engine extracts **signals** from activities to determine project context:

| Signal Type | Examples | Use Case |
|-------------|----------|----------|
| URL domains | `github.com`, `jira.example.com` | Identify project repos, issue trackers |
| URL paths | `/project-alpha/issues/123` | Extract project identifiers from URLs |
| App names | `Visual Studio Code`, `Slack`, `Zoom` | Categorize work type |
| Window titles | `PROJ-123: Fix login bug` | Extract issue IDs and project codes |
| File paths | `/Users/me/projects/alpha/src/...` | Identify local project context |

### 3. Sync ([`sync/`](src/sync/))

**Responsibility:** Manage data synchronization with remote APIs (cloud backend, SAP, etc.).

**Ports:**
- `OutboxQueue` - Queue items for eventual delivery to external systems
- `IdMappingRepository` - Map local IDs to remote IDs for synced entities
- `TokenUsageRepository` - Track API token usage and costs

**Use Cases:**
- Queue time entries for sync to remote systems
- Track token usage and API costs
- Map local entities to remote entities
- Handle sync conflicts and retries

### 4. Batch Processing ([`batch/`](src/batch/))

**Responsibility:** Process time entries in batches with dead-letter queue (DLQ) support.

**Ports:**
- `BatchRepository` - Persist batch metadata and status
- `DlqRepository` - Store failed items for manual review

**Use Cases:**
- Create and manage batches of time entries
- Process batches with error handling
- Move failed items to DLQ for investigation

### 5. User Profile ([`user/`](src/user/))

**Responsibility:** Manage user settings, preferences, and profile data.

**Ports:**
- `UserProfileRepository` - Store and retrieve user profile data

**Use Cases:**
- Load user configuration
- Update user preferences
- Manage OAuth credentials

## Port Interfaces (Traits)

All external dependencies are defined as **traits** (ports), allowing infrastructure to be swapped without changing business logic.

### Repository Pattern

```rust
#[async_trait]
pub trait SegmentRepository: Send + Sync {
    async fn save(&self, segment: &Segment) -> Result<(), CoreError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Segment>, CoreError>;
    async fn find_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Segment>, CoreError>;
}
```

### Service Pattern

```rust
#[async_trait]
pub trait ActivityProvider: Send + Sync {
    async fn get_current_activity(&self) -> Result<ActivityContext, CoreError>;
    async fn start_monitoring(&self) -> Result<(), CoreError>;
    async fn stop_monitoring(&self) -> Result<(), CoreError>;
}
```

### Provider Pattern

```rust
#[async_trait]
pub trait ActivityEnricher: Send + Sync {
    async fn enrich(&self, activity: &mut ActivityContext) -> Result<(), CoreError>;
}
```

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `calendar` | Calendar integration ports | Enables `calendar_ports.rs` |
| `sap` | SAP time entry submission ports | Enables `sap_ports.rs` |
| `ml` | Machine learning classification | Enables `Classifier` port |
| `tree-classifier` | Decision tree classifier | Part of `ml` feature |

**Example:**
```toml
pulsearc-core = { workspace = true, features = ["calendar", "sap"] }
```

## Dependencies

```toml
[dependencies]
# Internal dependencies (domain models only)
pulsearc-common = { workspace = true, features = ["platform", "observability"] }
pulsearc-domain = { workspace = true }

# Async support
async-trait = "0.1"
tokio = { workspace = true }

# Error handling
anyhow = { workspace = true }
thiserror = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Utilities
uuid = { workspace = true }
chrono = { workspace = true }
url = "2.5"  # For URL parsing in signal extraction
ahash = "0.8"  # For fast hashing in block_builder

# Observability
log = { workspace = true }
tracing = { workspace = true }
```

## Testing

The Core layer is **highly testable** because all dependencies are trait-based:

```rust
use pulsearc_core::{TrackingService, ActivityProvider};
use async_trait::async_trait;

// Mock implementation
struct MockActivityProvider {
    activity: ActivityContext,
}

#[async_trait]
impl ActivityProvider for MockActivityProvider {
    async fn get_current_activity(&self) -> Result<ActivityContext, CoreError> {
        Ok(self.activity.clone())
    }
}

#[tokio::test]
async fn test_tracking_service() {
    let mock_provider = MockActivityProvider {
        activity: ActivityContext::default(),
    };

    let service = TrackingService::new(/* inject mocks */);
    let snapshot = service.get_current_snapshot().await.unwrap();

    assert_eq!(snapshot.app_name, "Expected App");
}
```

**Run tests:**
```bash
# All tests
cargo test -p pulsearc-core

# With features
cargo test -p pulsearc-core --features calendar,sap

# Specific module
cargo test -p pulsearc-core --lib classification
```

## Business Rules

### Time Segmentation
- Segments are created when activity context changes (app, window title, URL)
- Minimum segment duration: 1 second
- Segments are merged if context is identical within a threshold

### Classification
- Blocks are built from 1+ segments with similar project context
- Classification requires at least one valid signal (URL, app, title)
- Evidence is generated from all activities within a block
- Confidence scores are calculated based on signal strength

### Sync
- Items are queued in the outbox for eventual delivery
- Failed sync attempts move to DLQ after 3 retries
- Token usage is tracked per API endpoint

## Error Handling

The Core layer uses domain-specific error types that compose with `CommonError`:

```rust
use pulsearc_common::error::{CommonError, ErrorClassification};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackingError {
    #[error("Tracking not started")]
    NotStarted,

    #[error("Segment not found: {0}")]
    SegmentNotFound(String),

    #[error(transparent)]
    Common(#[from] CommonError),
}
```

## Design Patterns

### Repository Pattern
- Abstract data access behind traits
- Infrastructure layer provides implementations
- Business logic is database-agnostic

### Service Pattern
- Encapsulate use cases in service structs
- Services coordinate between repositories and domain models
- Services are stateless (dependencies injected via constructor)

### Port/Adapter Pattern (Hexagonal Architecture)
- Core defines ports (traits)
- Infrastructure provides adapters (implementations)
- Core never depends on infrastructure

## Best Practices

1. **Pure Functions**: Prefer pure functions in domain logic; use services for stateful operations
2. **No I/O**: Core should never do I/O directly (database, HTTP, file system)
3. **Trait Bounds**: Use `Send + Sync` for all ports to enable async execution
4. **Error Propagation**: Use `?` operator and compose errors with `thiserror`
5. **Documentation**: Document all public APIs with `///` comments and examples
6. **Testing**: Write unit tests with mocked ports for all use cases

## See Also

- [API Layer](../api/README.md) - Tauri commands and dependency injection
- [Infra Layer](../infra/README.md) - Port implementations (databases, HTTP, platform APIs)
- [Domain Layer](../domain/README.md) - Domain models and types
- [Common Layer](../common/README.md) - Shared utilities and error handling
- [CLAUDE.md](../../CLAUDE.md) - Project-wide development rules