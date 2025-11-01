# PulseArc API

Tauri application layer providing the bridge between frontend (TypeScript) and backend (Rust) services.

---

## 🚧 Migration Status

**This crate is actively being migrated from `legacy/api/` as part of Phase 4: API Rewiring**

**Current Status:** ⏸️ Ready to Start (0/9 commands migrated)

**Migration Plan:** See [PHASE-4-NEW-CRATE-MIGRATION.md](../../docs/active-issue/PHASE-4-NEW-CRATE-MIGRATION.md) for the detailed migration plan.

### What's Changing

**Goal:** Migrate 9 Tauri commands (~3,385 LOC) from `legacy/api/` to this crate using database-persisted feature flags for safe rollout and rollback.

**Timeline:**
- **Weeks 1-2:** Migration (9 commands, one at a time)
- **Weeks 3-4:** Validation period (staged rollout with monitoring)
- **Week 5:** Cleanup (remove feature flags, archive legacy crate)

**Feature Flags:** All commands wrapped with fail-safe defaults
- Pattern: `new_[command]_commands` (e.g., `new_database_commands`)
- Default: `unwrap_or(false)` - any flag lookup error falls back to legacy path
- Stored: Database-persisted (not environment variables)
- Rollback: Toggle flag to `false`, restart app (<2 minutes)

### Commands to Migrate

| Priority | Command File | LOC | Status | Notes |
|----------|--------------|-----|--------|-------|
| P1 | database.rs | 512 | ⏸️ Pending | Database stats, health |
| P1 | user_profile.rs | 49 | ⏸️ Pending | User profile CRUD |
| P1 | window.rs | 61 | ⏸️ Pending | UI-only, low risk |
| P1 | blocks.rs | 632 | ⏸️ Pending | ⚠️ High complexity |
| P1 | calendar.rs | 946 | ⏸️ Pending | ⚠️ Highest risk (OAuth) |
| P1 | idle.rs | 193 | ⏸️ Pending | Idle period management |
| P2 | monitoring.rs | 741 | ⏸️ Pending | Sync stats, outbox |
| P2 | idle_sync.rs | 58 | ⏸️ Pending | Telemetry |
| P3 | seed_snapshots.rs | 193 | ⏸️ Pending | Dev tool (debug only) |

**Total:** 3,385 LOC | **Migrated:** 0/9 commands | **Progress:** 0%

**Note:** Feature flags command (107 LOC) already migrated; ML training command (242 LOC) skipped.

### Why This Approach?

1. **Clean Architecture:** Hexagonal design (ADR-003) with ports + adapters
2. **Gradual Migration:** Feature flags allow command-by-command rollout
3. **Safe Rollback:** Toggle flags in database, restart app (<2 minutes)
4. **Future-Proof:** Legacy crate will be archived after validation
5. **Minimal Risk:** Fail-safe defaults, extended validation for high-risk commands

---

## Overview

The `pulsearc-app` crate (API layer) serves as the entry point for the PulseArc desktop application. It orchestrates all backend components, exposes Tauri commands to the frontend, and manages application lifecycle and dependency injection.

## Architecture Role

The API crate sits at the **top of the dependency chain**:

```
┌─────────────────────────────────────┐
│        Frontend (TypeScript)        │
└──────────────┬──────────────────────┘
               │ Tauri IPC
┌──────────────▼──────────────────────┐
│         API (pulsearc-app)          │ ← You are here
│  • Tauri Commands                   │
│  • Application Context (DI)         │
│  • Main Entry Point                 │
└──────────────┬──────────────────────┘
               │
        ┌──────┴──────┬───────────┐
        ▼             ▼           ▼
  ┌─────────┐   ┌─────────┐  ┌─────────┐
  │  Infra  │   │  Core   │  │ Domain  │
  └─────────┘   └─────────┘  └─────────┘
```

**Key Responsibilities:**
- **Tauri Commands**: Expose backend functionality to the frontend via IPC
- **Dependency Injection**: Wire up all infrastructure implementations with core ports
- **Application Lifecycle**: Initialize, manage, and gracefully shut down all services
- **Configuration**: Load and validate application settings
- **Error Translation**: Convert backend errors to frontend-friendly responses

## Directory Structure

```
crates/api/
├── src/
│   ├── commands/               # Tauri command handlers
│   │   ├── calendar.rs        # Calendar integration commands
│   │   ├── classification.rs  # Classification & suggestion commands
│   │   ├── feature_flags.rs   # Feature flag management commands
│   │   ├── mod.rs             # Command module exports
│   │   ├── projects.rs        # Project CRUD commands
│   │   ├── suggestions.rs     # Time entry suggestions
│   │   ├── tracking.rs        # Activity tracking commands
│   │   └── user.rs            # User profile commands
│   ├── context/               # Application context (DI container)
│   │   └── mod.rs             # AppContext with all services
│   ├── lib.rs                 # Library exports
│   └── main.rs                # Binary entry point
├── Cargo.toml
└── README.md                   # This file
```

## Tauri Commands

All commands are exposed via the `#[tauri::command]` attribute and follow a consistent pattern:

### Tracking Commands ([`commands/tracking.rs`](src/commands/tracking.rs))

```rust
// Start activity tracking
start_tracking(context: State<AppContext>) -> Result<(), String>

// Stop activity tracking
stop_tracking(context: State<AppContext>) -> Result<(), String>

// Get current activity snapshot
get_current_activity(context: State<AppContext>) -> Result<ActivitySnapshot, String>
```

### Classification Commands ([`commands/classification.rs`](src/commands/classification.rs))

```rust
// Classify a time segment
classify_segment(context: State<AppContext>, segment_id: String) -> Result<Classification, String>

// Get classification suggestions for a time range
get_suggestions(context: State<AppContext>, start: i64, end: i64) -> Result<Vec<Suggestion>, String>
```

### Database Commands ([`commands/database.rs`](src/commands/database.rs))

```rust
// Get database statistics
get_database_stats(context: State<AppContext>) -> Result<DatabaseStats, String>

// Export database to file
export_database(context: State<AppContext>, path: String) -> Result<(), String>
```

### Calendar Commands ([`commands/calendar.rs`](src/commands/calendar.rs))
*Feature gated: `calendar`*

```rust
// Fetch calendar events
fetch_calendar_events(context: State<AppContext>, start: i64, end: i64) -> Result<Vec<CalendarEvent>, String>
```

### SAP Commands ([`commands/sap.rs`](src/commands/sap.rs))
*Feature gated: `sap`*

```rust
// Submit time entries to SAP
submit_to_sap(context: State<AppContext>, entries: Vec<TimeEntry>) -> Result<(), String>
```

### Feature Flag Commands ([`commands/feature_flags.rs`](src/commands/feature_flags.rs))

```rust
// List all feature flags
list_feature_flags(context: State<AppContext>) -> Result<Vec<FeatureFlag>, String>

// Check if a feature is enabled
is_feature_enabled(context: State<AppContext>, flag: String) -> Result<bool, String>

// Toggle a feature flag
toggle_feature_flag(context: State<AppContext>, flag: String, enabled: bool) -> Result<(), String>
```

### User Commands ([`commands/user.rs`](src/commands/user.rs))

```rust
// Get user profile
get_user_profile(context: State<AppContext>) -> Result<UserProfile, String>

// Update user settings
update_user_settings(context: State<AppContext>, settings: UserSettings) -> Result<(), String>
```

## Application Context ([`context/mod.rs`](src/context/mod.rs))

The `AppContext` struct acts as a **dependency injection container**, holding references to all services and infrastructure components:

```rust
pub struct AppContext {
    // Core services
    pub tracking_service: Arc<TrackingService>,
    pub classification_service: Arc<ClassificationService>,

    // Infrastructure
    pub db_manager: Arc<DbManager>,
    pub api_client: Arc<ApiClient>,

    // Schedulers
    pub block_scheduler: Arc<BlockScheduler>,
    pub classification_scheduler: Arc<ClassificationScheduler>,
    pub sync_scheduler: Arc<SyncScheduler>,

    // Optional integrations (feature-gated)
    #[cfg(feature = "calendar")]
    pub calendar_scheduler: Arc<CalendarScheduler>,

    #[cfg(feature = "sap")]
    pub sap_scheduler: Arc<SapScheduler>,
}
```

**AppContext Lifecycle:**
1. **Initialization** (`AppContext::new()`): Load config, create services, start schedulers
2. **Runtime**: Passed to Tauri commands via `State<AppContext>`
3. **Shutdown** (`AppContext::shutdown()`): Gracefully stop all schedulers and services

## Main Entry Point ([`main.rs`](src/main.rs))

The `main.rs` file:
1. Loads environment variables and configuration
2. Initializes logging and observability
3. Creates the `AppContext` (dependency injection)
4. Registers all Tauri commands
5. Launches the Tauri application
6. Handles graceful shutdown on exit

**Key Features:**
- **Single Instance Lock**: Prevents multiple instances of the app
- **System Tray**: macOS menu bar integration
- **Global Shortcuts**: Keyboard shortcuts for quick actions
- **Error Recovery**: Graceful degradation on initialization failures

## Dependencies

```toml
[dependencies]
# Internal crates (hexagonal architecture)
pulsearc-common = { workspace = true, features = ["platform", "observability"] }
pulsearc-domain = { workspace = true }
pulsearc-core = { workspace = true }
pulsearc-infra = { workspace = true }

# Tauri framework
tauri = { version = "2.9", features = ["macos-private-api", "tray-icon"] }
tauri-plugin-shell = "2.0"
tauri-plugin-global-shortcut = "2"
tauri-plugin-opener = "2"

# Async runtime
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization (Tauri IPC)
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
anyhow = { workspace = true }
thiserror = { workspace = true }

# Observability
tracing = { workspace = true }
log = { workspace = true }
```

## Feature Flags

The API crate propagates feature flags to underlying layers:

| Feature | Description | Default |
|---------|-------------|---------|
| `sqlcipher` | SQLCipher encrypted database support | ✅ |
| `calendar` | Calendar integration (macOS Calendar, Google Calendar) | ❌ |
| `sap` | SAP time entry submission | ❌ |
| `tree-classifier` | Decision tree-based classification | ❌ |
| `ml` | Machine learning features (includes `tree-classifier`) | ❌ |
| `graphql` | GraphQL API support | ❌ |
| `ts-gen` | TypeScript type generation for frontend | ❌ |
| `custom-protocol` | Tauri custom protocol for production builds | ❌ |

**Example:**
```toml
pulsearc-app = { workspace = true, features = ["calendar", "sap"] }
```

## Error Handling

All Tauri commands follow a consistent error handling pattern:

1. **Internal Errors**: Use `thiserror` for typed errors within the crate
2. **Command Results**: Return `Result<T, String>` for Tauri IPC (frontend-friendly)
3. **Error Logging**: Log errors with `tracing::error!` before converting to strings
4. **User Context**: Include actionable error messages for end users

**Example:**
```rust
#[tauri::command]
async fn my_command(context: State<'_, AppContext>) -> Result<Data, String> {
    context.service
        .do_something()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to do something");
            format!("Failed to do something: {}", e)
        })
}
```

## Testing

```bash
# Run all tests
cargo test -p pulsearc-app

# Test with features
cargo test -p pulsearc-app --features calendar,sap

# Integration tests
cargo test -p pulsearc-app --test integration_tests
```

## Development

### Running the Application

```bash
# Development mode (hot reload)
make dev
# or
pnpm tauri dev

# Production build
make build
# or
pnpm tauri build
```

### Adding a New Tauri Command

1. Define the command handler in `src/commands/<module>.rs`:
   ```rust
   #[tauri::command]
   pub async fn my_command(context: State<'_, AppContext>) -> Result<Data, String> {
       // Implementation
   }
   ```

2. Export it in `src/commands/mod.rs`:
   ```rust
   pub use my_module::my_command;
   ```

3. Register it in `src/main.rs`:
   ```rust
   .invoke_handler(tauri::generate_handler![
       // ... existing commands
       my_command,
   ])
   ```

4. Call it from the frontend (TypeScript):
   ```typescript
   import { invoke } from '@tauri-apps/api/core';

   const result = await invoke<Data>('my_command');
   ```

## Platform Support

- **macOS**: Full support (primary platform)
- **Linux**: Not supported (Tauri app is macOS-only per `CLAUDE.md`)
- **Windows**: Not supported

## Security

- **SQLCipher**: Database encryption at rest
- **Keychain Integration**: OAuth tokens stored in macOS Keychain
- **Single Instance Lock**: Prevents concurrent app instances
- **CSRF Protection**: OAuth state validation with constant-time comparison

## See Also

- [Core Layer](../core/README.md) - Business logic and domain services
- [Infra Layer](../infra/README.md) - Infrastructure implementations
- [Domain Layer](../domain/README.md) - Domain models and types
- [Common Layer](../common/README.md) - Shared utilities and patterns
- [CLAUDE.md](../../CLAUDE.md) - Project-wide development rules