# PulseArc Domain

Pure domain models and types with zero dependencies on other PulseArc crates.

## Overview

The `pulsearc-domain` crate defines the **ubiquitous language** of the PulseArc application. It contains all domain models, value objects, configuration types, and domain-specific errors. This crate has **no dependencies** on other PulseArc crates and minimal external dependencies.

## Architecture Role

The Domain crate sits at the **bottom of the dependency hierarchy**:

```
     ┌─────────────────────────────────────┐
     │          API Layer                  │
     └──────────────┬──────────────────────┘
                    │
        ┌────┴──────┬───────────┐
        ▼           ▼           ▼
  ┌─────────┐   ┌─────────┐  ┌─────────┐
  │  Infra  │   │  Core   │  │ Common  │
  └────┬────┘   └────┬────┘  └────┬────┘
       │             │            │
       └─────────────┴────────────┘
                     │
              ┌──────▼──────┐
              │   DOMAIN    │ ← You are here (no dependencies)
              │  • Types    │
              │  • Errors   │
              │  • Config   │
              └─────────────┘
```

**Key Principles:**
- **Zero Internal Dependencies**: Does not depend on `api`, `core`, `infra`, or `common` crates
- **Minimal External Dependencies**: Only essential libraries (serde, chrono, uuid, thiserror)
- **Ubiquitous Language**: Models map directly to business domain concepts
- **Value Objects**: Immutable, self-validating domain primitives
- **Serialization Ready**: All types are `Serialize` + `Deserialize` for IPC and storage

## Directory Structure

```
crates/domain/
├── src/
│   ├── types/                  # Domain models
│   │   ├── activity.rs        # Activity, ActivityContext, ActivitySnapshot
│   │   ├── classification.rs  # Block, TimeEntry, Classification, Suggestion
│   │   ├── database.rs        # DatabaseStats, DatabaseMetadata
│   │   ├── idle.rs            # IdleEvent, IdleState
│   │   ├── mod.rs             # Type exports
│   │   ├── sap.rs             # SAPEntry, SAPResponse (feature-gated)
│   │   ├── stats.rs           # Statistics types
│   │   └── user.rs            # UserProfile, UserSettings
│   ├── utils/                  # Domain utilities
│   │   └── mod.rs             # Utility functions
│   ├── config.rs               # Configuration types
│   ├── constants.rs            # Domain constants
│   ├── errors.rs               # Domain error types
│   ├── lib.rs                  # Module exports
│   └── macros.rs               # Domain-specific macros
├── Cargo.toml
└── README.md                    # This file
```

## Domain Types

### Activity Types ([`types/activity.rs`](src/types/activity.rs))

**`ActivityContext`**
Represents the current application context captured from the macOS Accessibility API.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityContext {
    pub app_name: String,              // "Google Chrome", "Visual Studio Code"
    pub window_title: String,          // "PulseArc - GitHub"
    pub url: Option<String>,           // Browser URL (if applicable)
    pub local_path: Option<String>,    // Local file path (if applicable)
    pub bundle_id: Option<String>,     // macOS bundle ID
    pub timestamp: DateTime<Utc>,      // When the activity was captured
}
```

**`ActivitySnapshot`**
A point-in-time snapshot of user activity, captured at regular intervals.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub id: Uuid,
    pub context: ActivityContext,
    pub captured_at: DateTime<Utc>,
    pub idle_duration_secs: u64,       // Seconds user has been idle
}
```

**`Activity`**
A persisted activity record with additional metadata.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub context: ActivityContext,
    pub created_at: DateTime<Utc>,
    pub segment_id: Option<Uuid>,      // Associated time segment
}
```

### Classification Types ([`types/classification.rs`](src/types/classification.rs))

**`Block`**
A contiguous time period representing focused work on a specific task or project.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub activities: Vec<Activity>,     // Activities within this block
    pub signals: Vec<Signal>,          // Extracted classification signals
    pub evidence: Vec<Evidence>,       // Human-readable evidence
    pub project_id: Option<String>,    // Matched project/WBS code
    pub confidence: f64,               // Classification confidence (0.0-1.0)
}
```

**`TimeEntry`**
A billable time entry ready for submission to external systems (SAP, Jira, etc.).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: Uuid,
    pub user_id: String,
    pub project_code: String,          // WBS or project identifier
    pub task_description: String,      // Human-readable task description
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_hours: f64,           // Billable hours
    pub evidence: Vec<Evidence>,       // Supporting evidence
    pub submitted_at: Option<DateTime<Utc>>,
    pub external_id: Option<String>,   // ID in external system (SAP, etc.)
}
```

**`Suggestion`**
A suggested time entry classification presented to the user for approval.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub block: Block,
    pub suggested_project: String,     // Suggested project code
    pub suggested_description: String, // Suggested task description
    pub confidence: f64,               // Confidence score (0.0-1.0)
    pub reasoning: String,             // Explanation for the suggestion
}
```

**`Signal`**
A classification signal extracted from activity context (URLs, app names, etc.).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Signal {
    UrlDomain(String),                 // "github.com", "jira.example.com"
    UrlPath(String),                   // "/project-alpha/issues/123"
    AppName(String),                   // "Visual Studio Code"
    WindowTitle(String),               // "PROJ-123: Fix login bug"
    FilePath(String),                  // "/Users/me/projects/alpha/..."
    BundleId(String),                  // "com.microsoft.VSCode"
}
```

**`Evidence`**
Human-readable evidence supporting a classification decision.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub timestamp: DateTime<Utc>,
    pub description: String,           // "Worked in VS Code on auth module"
    pub source: EvidenceSource,        // Where the evidence came from
    pub confidence: f64,               // Reliability of this evidence (0.0-1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceSource {
    Activity,                          // Derived from activity snapshot
    Calendar,                          // Derived from calendar event
    Manual,                            // User-provided
    System,                            // System-generated
}
```

### User Types ([`types/user.rs`](src/types/user.rs))

**`UserProfile`**
User profile information and preferences.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub name: String,
    pub settings: UserSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**`UserSettings`**
User-configurable application settings.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub tracking_interval_secs: u64,   // How often to capture snapshots (default: 60)
    pub idle_threshold_secs: u64,      // Idle detection threshold (default: 300)
    pub auto_classify: bool,           // Auto-classify time blocks (default: true)
    pub show_notifications: bool,      // Show system notifications (default: true)
}
```

### Database Types ([`types/database.rs`](src/types/database.rs))

**`DatabaseStats`**
Database statistics exposed to the UI.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_activities: u64,
    pub total_segments: u64,
    pub total_blocks: u64,
    pub total_time_entries: u64,
    pub db_size_bytes: u64,
    pub first_activity_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
}
```

### Idle Types ([`types/idle.rs`](src/types/idle.rs))

**`IdleEvent`**
Represents a period of user inactivity.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleEvent {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_secs: u64,
}
```

**`IdleState`**
Current idle state of the user.

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IdleState {
    Active,
    Idle { since: DateTime<Utc> },
}
```

### SAP Types ([`types/sap.rs`](src/types/sap.rs))
*Feature gated: `sap`*

**`SAPEntry`**
Time entry formatted for SAP submission.

```rust
#[cfg(feature = "sap")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SAPEntry {
    pub employee_id: String,
    pub wbs_element: String,            // Work Breakdown Structure code
    pub activity_type: String,          // SAP activity type code
    pub start_date: NaiveDate,
    pub hours: f64,
    pub description: String,
}
```

### Statistics Types ([`types/stats.rs`](src/types/stats.rs))

**`DailyStats`**
Aggregated statistics for a single day.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: NaiveDate,
    pub total_tracked_hours: f64,
    pub total_billable_hours: f64,
    pub total_idle_hours: f64,
    pub activities_count: u64,
    pub blocks_count: u64,
    pub time_entries_count: u64,
}
```

## Configuration Types ([`config.rs`](src/config.rs))

**`AppConfig`**
Application-wide configuration loaded from environment or config files.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub api_base_url: String,
    pub oauth_client_id: String,
    pub tracking_interval_secs: u64,
    pub idle_threshold_secs: u64,
    pub log_level: String,
}
```

**`SchedulerConfig`**
Configuration for background job schedulers.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub block_scheduler_cron: String,         // e.g., "0 */5 * * * *" (every 5 min)
    pub classification_scheduler_cron: String, // e.g., "0 0 * * * *" (hourly)
    pub sync_scheduler_cron: String,          // e.g., "0 */15 * * * *" (every 15 min)
}
```

## Error Types ([`errors.rs`](src/errors.rs))

**`DomainError`**
Domain-level errors (validation, business rule violations).

```rust
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Invalid time range: {0}")]
    InvalidTimeRange(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Business rule violation: {0}")]
    BusinessRule(String),
}
```

## Constants ([`constants.rs`](src/constants.rs))

Domain-specific constants and defaults:

```rust
// Default tracking interval (60 seconds)
pub const DEFAULT_TRACKING_INTERVAL_SECS: u64 = 60;

// Default idle threshold (5 minutes)
pub const DEFAULT_IDLE_THRESHOLD_SECS: u64 = 300;

// Minimum segment duration (1 second)
pub const MIN_SEGMENT_DURATION_SECS: u64 = 1;

// Maximum block duration (8 hours)
pub const MAX_BLOCK_DURATION_HOURS: u64 = 8;

// Classification confidence threshold (70%)
pub const CLASSIFICATION_CONFIDENCE_THRESHOLD: f64 = 0.7;
```

## Macros ([`macros.rs`](src/macros.rs))

Domain-specific utility macros:

```rust
/// Create a new Activity with default values
macro_rules! activity {
    ($app_name:expr, $window_title:expr) => {
        Activity {
            id: Uuid::new_v4(),
            context: ActivityContext {
                app_name: $app_name.to_string(),
                window_title: $window_title.to_string(),
                url: None,
                local_path: None,
                bundle_id: None,
                timestamp: Utc::now(),
            },
            created_at: Utc::now(),
            segment_id: None,
        }
    };
}
```

## Dependencies

```toml
[dependencies]
# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }
anyhow = { workspace = true }

# Time and dates
chrono = { workspace = true }

# UUIDs
uuid = { workspace = true }

# TypeScript generation (optional)
ts-rs = { workspace = true, optional = true }
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `ts-gen` | Generate TypeScript type definitions for frontend | ❌ |

**Example:**
```toml
pulsearc-domain = { workspace = true, features = ["ts-gen"] }
```

## TypeScript Generation

When the `ts-gen` feature is enabled, TypeScript type definitions are automatically generated for all domain types marked with `#[ts(export)]`.

**Workflow:**

```bash
# Generate and sync types to frontend (recommended)
cargo xtask codegen
# or
make codegen
```

**What happens:**
1. Runs tests with `ts-gen` feature enabled
2. ts-rs generates `.ts` files to `crates/domain/bindings/`
3. Files are synced to `frontend/shared/types/generated/`
4. An `index.ts` is generated with all exports

**Output locations:**
- **Temporary:** `crates/domain/bindings/` (gitignored)
- **Committed:** `frontend/shared/types/generated/` (tracked in git)

**Example generated types:**

```typescript
// Generated TypeScript types
export interface ActivityContext {
    app_name: string;
    window_title: string;
    url: string | null;
    local_path: string | null;
    bundle_id: string | null;
    timestamp: string;  // ISO 8601 datetime
}

export interface TimeEntry {
    id: string;  // UUID
    user_id: string;
    project_code: string;
    task_description: string;
    start_time: string;  // ISO 8601 datetime
    end_time: string;    // ISO 8601 datetime
    duration_hours: number;
    evidence: Evidence[];
    submitted_at: string | null;
    external_id: string | null;
}
```

## Validation

Domain types should self-validate where possible:

```rust
impl TimeEntry {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.end_time <= self.start_time {
            return Err(DomainError::InvalidTimeRange(
                "End time must be after start time".to_string()
            ));
        }

        if self.duration_hours <= 0.0 {
            return Err(DomainError::Validation(
                "Duration must be positive".to_string()
            ));
        }

        if self.project_code.is_empty() {
            return Err(DomainError::Validation(
                "Project code is required".to_string()
            ));
        }

        Ok(())
    }
}
```

## Testing

```bash
# Run all domain tests
cargo test -p pulsearc-domain

# Test with TypeScript generation
cargo test -p pulsearc-domain --features ts-gen

# Check serialization round-trips
cargo test -p pulsearc-domain --lib types::tests::test_serialization
```

## Design Principles

1. **Immutability**: Prefer immutable types; use builder pattern for complex construction
2. **Value Objects**: Types should be self-contained and self-validating
3. **Serialization**: All types must be `Serialize` + `Deserialize` for IPC and storage
4. **Documentation**: All public types must have `///` doc comments with examples
5. **No Business Logic**: Domain types are data containers; business logic lives in Core
6. **Minimal Dependencies**: Only add dependencies that are truly essential

## Best Practices

- **Use `Uuid::new_v4()` for IDs** instead of auto-incrementing integers
- **Use `DateTime<Utc>` for all timestamps** (never local time)
- **Use `chrono::Duration` for time periods** instead of raw seconds
- **Derive `Debug, Clone, Serialize, Deserialize`** on all domain types
- **Add validation methods** to domain types that enforce invariants
- **Use newtype pattern** for domain-specific primitives (e.g., `ProjectCode(String)`)

## See Also

- [Core Layer](../core/README.md) - Business logic using these domain types
- [API Layer](../api/README.md) - Tauri IPC using domain types for serialization
- [Infra Layer](../infra/README.md) - Database persistence of domain types
- [Common Layer](../common/README.md) - Shared utilities and error handling
- [CLAUDE.md](../../CLAUDE.md) - Project-wide development rules