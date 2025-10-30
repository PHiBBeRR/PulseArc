# PulseArc Refactoring: File Mapping Reference

**Created:** Oct 30, 2025
**Status:** Reference Document

---

## Table of Contents

1. [Current Structure](#current-structure)
2. [Proposed Structure](#proposed-structure)
3. [Complete File Mapping](#complete-file-mapping)
4. [Migration Commands](#migration-commands)
5. [Key Transformations](#key-transformations)

---

## Current Structure

```
PulseArc/
├── Cargo.toml                           # Single crate
├── build.rs
├── src/
│   ├── main.rs                          # 1,917 lines (TOO BIG)
│   ├── lib.rs                           # Global singletons
│   │
│   ├── commands/                        # Tauri commands
│   │   ├── mod.rs
│   │   ├── blocks.rs
│   │   ├── calendar.rs
│   │   ├── database.rs
│   │   ├── idle.rs
│   │   ├── idle_sync.rs
│   │   ├── ml_training.rs
│   │   ├── monitoring.rs
│   │   ├── seed_snapshots.rs
│   │   ├── user_profile.rs
│   │   └── window.rs
│   │
│   ├── db/                              # Database (SQLCipher)
│   │   ├── mod.rs
│   │   ├── manager.rs                   # DbManager + connection pool
│   │   ├── migrations.rs
│   │   ├── models.rs
│   │   ├── models_idle.rs
│   │   ├── local.rs                     # ❌ Singleton pattern (DELETE)
│   │   ├── helpers.rs
│   │   ├── params.rs
│   │   │
│   │   ├── activity/
│   │   │   ├── mod.rs
│   │   │   ├── snapshots.rs
│   │   │   └── segments.rs
│   │   │
│   │   ├── blocks/
│   │   │   ├── mod.rs
│   │   │   └── operations.rs
│   │   │
│   │   ├── outbox/
│   │   │   ├── mod.rs
│   │   │   ├── outbox.rs
│   │   │   ├── id_mappings.rs
│   │   │   └── token_usage.rs
│   │   │
│   │   ├── calendar/
│   │   │   ├── mod.rs
│   │   │   ├── events.rs
│   │   │   ├── tokens.rs
│   │   │   ├── sync_settings.rs
│   │   │   └── suggestions.rs
│   │   │
│   │   ├── batch/
│   │   │   ├── mod.rs
│   │   │   ├── operations.rs
│   │   │   └── dlq.rs
│   │   │
│   │   └── utils/
│   │       ├── mod.rs
│   │       ├── stats.rs
│   │       └── raw_queries.rs
│   │
│   ├── tracker/                         # Activity tracking
│   │   ├── mod.rs
│   │   ├── core.rs                      # Tracker orchestration
│   │   ├── provider.rs                  # ActivityProvider trait
│   │   │
│   │   ├── providers/
│   │   │   └── macos.rs                 # macOS AX API implementation
│   │   │
│   │   ├── idle/
│   │   │   ├── mod.rs
│   │   │   ├── detector.rs
│   │   │   ├── period_tracker.rs
│   │   │   └── sleep_wake.rs
│   │   │
│   │   └── os_events/
│   │       ├── mod.rs
│   │       └── macos.rs                 # NSWorkspace events
│   │
│   ├── detection/                       # Activity detection
│   │   ├── mod.rs                       # Engine + Detector trait
│   │   ├── default.rs
│   │   │
│   │   ├── enrichers/
│   │   │   ├── mod.rs
│   │   │   ├── browser.rs               # Browser URL extraction
│   │   │   └── office.rs                # Office metadata
│   │   │
│   │   └── packs/
│   │       ├── mod.rs
│   │       │
│   │       ├── technology/
│   │       │   ├── mod.rs
│   │       │   ├── ide.rs
│   │       │   ├── terminal.rs
│   │       │   ├── email.rs
│   │       │   ├── comms.rs
│   │       │   ├── design.rs
│   │       │   └── browser/
│   │       │       ├── mod.rs
│   │       │       ├── github.rs
│   │       │       ├── docs.rs
│   │       │       ├── gworkspace.rs
│   │       │       ├── meeting.rs
│   │       │       ├── productivity.rs
│   │       │       └── stackoverflow.rs
│   │       │
│   │       ├── deals/
│   │       │   ├── mod.rs
│   │       │   ├── vdr.rs
│   │       │   ├── tax_research.rs
│   │       │   ├── tax_software.rs
│   │       │   ├── deal_docs.rs
│   │       │   ├── client_comms.rs
│   │       │   └── practice_mgmt.rs
│   │       │
│   │       ├── consulting/
│   │       │   ├── mod.rs
│   │       │   ├── consulting_deliverables.rs
│   │       │   ├── data_viz.rs
│   │       │   ├── etl_analytics.rs
│   │       │   ├── survey_tools.rs
│   │       │   └── whiteboarding.rs
│   │       │
│   │       ├── finance/
│   │       │   ├── mod.rs
│   │       │   ├── accounting_software.rs
│   │       │   ├── audit_tools.rs
│   │       │   ├── consolidation.rs
│   │       │   ├── erp_finance.rs
│   │       │   ├── expense_mgmt.rs
│   │       │   ├── financial_spreadsheets.rs
│   │       │   └── fpa_tools.rs
│   │       │
│   │       ├── legal/
│   │       │   ├── mod.rs
│   │       │   ├── contract_mgmt.rs
│   │       │   ├── document_review.rs
│   │       │   ├── legal_drafting.rs
│   │       │   ├── legal_research.rs
│   │       │   └── practice_mgmt.rs
│   │       │
│   │       └── sales/
│   │           ├── mod.rs
│   │           ├── crm.rs
│   │           ├── customer_success.rs
│   │           ├── proposals.rs
│   │           └── sales_engagement.rs
│   │
│   ├── preprocess/                      # Data preprocessing
│   │   ├── mod.rs
│   │   ├── redact.rs                    # PII redaction
│   │   ├── segmenter.rs                 # Activity segmentation
│   │   └── trigger.rs                   # Segmentation trigger
│   │
│   ├── inference/                       # ML classification
│   │   ├── mod.rs
│   │   ├── block_builder.rs             # Block building logic
│   │   ├── scheduler.rs                 # Block scheduler
│   │   ├── tree_classifier.rs           # Decision tree
│   │   ├── logistic_classifier.rs       # Logistic regression
│   │   ├── rules_classifier.rs          # Rule-based fallback
│   │   └── metrics.rs                   # Classification metrics
│   │
│   ├── sync/                            # Backend sync
│   │   ├── mod.rs
│   │   ├── neon_client.rs               # Backend GraphQL client
│   │   ├── outbox_worker.rs             # Outbox processor
│   │   ├── scheduler.rs                 # Sync scheduler
│   │   ├── retry.rs                     # Retry logic
│   │   ├── cost_tracker.rs              # Token usage
│   │   └── cleanup.rs                   # Storage cleanup
│   │
│   ├── integrations/                    # External integrations
│   │   │
│   │   ├── calendar/
│   │   │   ├── mod.rs
│   │   │   ├── oauth.rs
│   │   │   ├── client.rs
│   │   │   ├── sync.rs
│   │   │   ├── scheduler.rs
│   │   │   └── providers/
│   │   │       ├── mod.rs
│   │   │       ├── google.rs
│   │   │       └── microsoft.rs
│   │   │
│   │   └── sap/
│   │       ├── mod.rs
│   │       ├── client.rs
│   │       ├── forwarder.rs
│   │       ├── scheduler.rs
│   │       ├── cache.rs
│   │       ├── validation.rs
│   │       ├── bulk_lookup.rs
│   │       └── auth/
│   │           ├── mod.rs
│   │           └── service.rs
│   │
│   ├── domain/                          # Domain logic
│   │   ├── mod.rs
│   │   ├── user_profile.rs
│   │   └── api/                         # Main API integration
│   │       ├── mod.rs
│   │       ├── auth.rs
│   │       ├── client.rs
│   │       ├── forwarder.rs
│   │       ├── scheduler.rs
│   │       ├── commands.rs
│   │       └── models.rs
│   │
│   ├── http/                            # HTTP client
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── graphql.rs
│   │
│   ├── shared/                          # Shared utilities
│   │   ├── mod.rs
│   │   ├── cache.rs                     # Startup cache
│   │   ├── config.rs                    # Config management
│   │   │
│   │   ├── types/
│   │   │   ├── mod.rs
│   │   │   ├── activity.rs
│   │   │   └── time_entry.rs
│   │   │
│   │   ├── auth/
│   │   │   ├── mod.rs
│   │   │   ├── oauth_service.rs
│   │   │   └── pkce.rs
│   │   │
│   │   ├── constants/
│   │   │   ├── mod.rs
│   │   │   └── apps.rs
│   │   │
│   │   └── extractors/
│   │       ├── mod.rs
│   │       └── patterns.rs
│   │
│   ├── observability/                   # Metrics & errors
│   │   ├── mod.rs
│   │   ├── datadog.rs
│   │   │
│   │   ├── metrics/
│   │   │   ├── mod.rs
│   │   │   └── counters.rs
│   │   │
│   │   └── errors/
│   │       ├── mod.rs
│   │       └── types.rs
│   │
│   ├── tooling/                         # Dev tools
│   │   ├── mod.rs
│   │   └── macros/
│   │       └── mod.rs
│   │
│   └── utils/                           # Generic utils
│       ├── mod.rs
│       └── time.rs
│
└── frontend/                            # React frontend (unchanged)
    └── src/
```

---

## Proposed Structure

```
PulseArc/
├── Cargo.toml                           # Workspace root
├── build.rs
│
├── crates/
│   │
│   ├── shared/                          # ✨ Foundation layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs                # NEW: Unified config
│   │       ├── error.rs                 # NEW: Unified errors
│   │       │
│   │       ├── types/
│   │       │   ├── mod.rs
│   │       │   ├── activity.rs
│   │       │   ├── time_entry.rs
│   │       │   ├── classification.rs
│   │       │   └── sync.rs
│   │       │
│   │       ├── constants/
│   │       │   ├── mod.rs
│   │       │   └── apps.rs
│   │       │
│   │       ├── extractors/
│   │       │   ├── mod.rs
│   │       │   └── patterns.rs
│   │       │
│   │       └── utils/
│   │           ├── mod.rs
│   │           └── time.rs
│   │
│   ├── core/                            # ✨ Business logic (pure)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       │
│   │       ├── tracking/
│   │       │   ├── mod.rs
│   │       │   ├── ports.rs             # NEW: Traits
│   │       │   └── service.rs           # NEW: Business logic
│   │       │
│   │       ├── detection/
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs
│   │       │   ├── default.rs
│   │       │   └── packs/
│   │       │       ├── mod.rs
│   │       │       ├── technology/
│   │       │       ├── deals/
│   │       │       ├── consulting/
│   │       │       ├── finance/
│   │       │       ├── legal/
│   │       │       └── sales/
│   │       │
│   │       ├── preprocessing/
│   │       │   ├── mod.rs
│   │       │   ├── redact.rs
│   │       │   └── segmenter.rs
│   │       │
│   │       ├── classification/
│   │       │   ├── mod.rs
│   │       │   ├── ports.rs             # NEW: Traits
│   │       │   ├── hybrid.rs            # Tree + logistic
│   │       │   ├── rules.rs
│   │       │   └── block_builder.rs
│   │       │
│   │       └── sync/
│   │           ├── mod.rs
│   │           └── ports.rs             # NEW: Traits
│   │
│   ├── infra/                           # ✨ Infrastructure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       │
│   │       ├── database/
│   │       │   ├── mod.rs
│   │       │   ├── manager.rs           # DbManager + pool
│   │       │   ├── migrations.rs
│   │       │   ├── models.rs
│   │       │   │
│   │       │   ├── repositories/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── activity.rs      # Implements ActivityRepository
│   │       │   │   ├── blocks.rs
│   │       │   │   ├── outbox.rs
│   │       │   │   ├── calendar.rs
│   │       │   │   └── batch.rs
│   │       │   │
│   │       │   └── utils/
│   │       │       ├── mod.rs
│   │       │       ├── stats.rs
│   │       │       └── queries.rs
│   │       │
│   │       ├── http/
│   │       │   ├── mod.rs
│   │       │   ├── client.rs
│   │       │   └── graphql.rs
│   │       │
│   │       ├── platform/
│   │       │   └── macos/
│   │       │       ├── mod.rs
│   │       │       ├── provider.rs      # Implements ActivityProvider
│   │       │       ├── events.rs        # NSWorkspace
│   │       │       ├── idle.rs
│   │       │       └── enrichers/
│   │       │           ├── mod.rs
│   │       │           ├── browser.rs
│   │       │           └── office.rs
│   │       │
│   │       ├── integrations/
│   │       │   ├── mod.rs
│   │       │   ├── oauth.rs             # Shared OAuth
│   │       │   │
│   │       │   ├── calendar/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── client.rs
│   │       │   │   ├── sync.rs
│   │       │   │   └── providers/
│   │       │   │       ├── google.rs
│   │       │   │       └── microsoft.rs
│   │       │   │
│   │       │   ├── sap/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── client.rs
│   │       │   │   ├── auth.rs
│   │       │   │   ├── cache.rs
│   │       │   │   └── validation.rs
│   │       │   │
│   │       │   └── backend_api/
│   │       │       ├── mod.rs
│   │       │       ├── client.rs
│   │       │       ├── auth.rs
│   │       │       └── models.rs
│   │       │
│   │       ├── sync/
│   │       │   ├── mod.rs
│   │       │   ├── outbox_worker.rs
│   │       │   ├── retry.rs
│   │       │   ├── cost_tracker.rs
│   │       │   └── cleanup.rs
│   │       │
│   │       └── observability/
│   │           ├── mod.rs
│   │           ├── datadog.rs
│   │           └── metrics/
│   │               └── counters.rs
│   │
│   └── api/                             # ✨ Application layer
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                  # <100 lines!
│           ├── lib.rs                   # Re-exports only
│           │
│           ├── context.rs               # NEW: AppContext
│           ├── builder.rs               # NEW: AppContextBuilder
│           ├── schedulers.rs            # NEW: Scheduler registry
│           │
│           └── commands/
│               ├── mod.rs
│               ├── tracking.rs
│               ├── blocks.rs
│               ├── calendar.rs
│               ├── database.rs
│               ├── idle.rs
│               ├── monitoring.rs
│               └── user_profile.rs
│
└── frontend/                            # React (unchanged)
    └── src/
```

---

## Complete File Mapping

### Phase 1: Extract Shared (Week 2)

| Current Location | New Location | Action |
|-----------------|--------------|--------|
| `src/shared/types/` | `crates/shared/src/types/` | Move |
| `src/shared/constants/` | `crates/shared/src/constants/` | Move |
| `src/shared/extractors/` | `crates/shared/src/extractors/` | Move |
| `src/observability/errors/` | `crates/shared/src/error.rs` | Consolidate |
| `src/utils/` (generic only) | `crates/shared/src/utils/` | Move |
| N/A | `crates/shared/src/config.rs` | **Create new** |

**Create crates/shared/src/config.rs:**
```rust
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub webapi_url: String,
    pub webapi_client_id: String,
    pub webapi_client_secret: String,
    pub sap_graphql_url: String,
    pub sap_client_id: String,
    pub sap_client_secret: String,
    pub sync_interval_secs: u64,
    pub enable_snapshot_persistence: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL not set")?,
            webapi_url: env::var("WEBAPI_URL")
                .unwrap_or_else(|_| "https://api.pulsearc.ai".to_string()),
            // ... load all env vars
        })
    }
}
```

---

### Phase 2: Extract Infra (Week 3-4)

#### Database → crates/infra/src/database/

| Current Location | New Location |
|-----------------|--------------|
| `src/db/manager.rs` | `crates/infra/src/database/manager.rs` |
| `src/db/migrations.rs` | `crates/infra/src/database/migrations.rs` |
| `src/db/models.rs` | `crates/infra/src/database/models.rs` |
| `src/db/models_idle.rs` | `crates/infra/src/database/models.rs` (merge) |
| `src/db/helpers.rs` | `crates/infra/src/database/utils/helpers.rs` |
| `src/db/params.rs` | `crates/infra/src/database/utils/params.rs` |
| `src/db/local.rs` | **DELETE** (use manager only) |
| | |
| `src/db/activity/snapshots.rs` | `crates/infra/src/database/repositories/activity.rs` |
| `src/db/activity/segments.rs` | `crates/infra/src/database/repositories/activity.rs` |
| `src/db/blocks/operations.rs` | `crates/infra/src/database/repositories/blocks.rs` |
| `src/db/outbox/outbox.rs` | `crates/infra/src/database/repositories/outbox.rs` |
| `src/db/outbox/id_mappings.rs` | `crates/infra/src/database/repositories/outbox.rs` |
| `src/db/outbox/token_usage.rs` | `crates/infra/src/database/repositories/outbox.rs` |
| `src/db/calendar/events.rs` | `crates/infra/src/database/repositories/calendar.rs` |
| `src/db/calendar/tokens.rs` | `crates/infra/src/database/repositories/calendar.rs` |
| `src/db/calendar/sync_settings.rs` | `crates/infra/src/database/repositories/calendar.rs` |
| `src/db/calendar/suggestions.rs` | `crates/infra/src/database/repositories/calendar.rs` |
| `src/db/batch/operations.rs` | `crates/infra/src/database/repositories/batch.rs` |
| `src/db/batch/dlq.rs` | `crates/infra/src/database/repositories/batch.rs` |
| | |
| `src/db/utils/stats.rs` | `crates/infra/src/database/utils/stats.rs` |
| `src/db/utils/raw_queries.rs` | `crates/infra/src/database/utils/queries.rs` |

#### HTTP → crates/infra/src/http/

| Current Location | New Location |
|-----------------|--------------|
| `src/http/client.rs` | `crates/infra/src/http/client.rs` |
| `src/http/graphql.rs` | `crates/infra/src/http/graphql.rs` |

#### Platform → crates/infra/src/platform/macos/

| Current Location | New Location |
|-----------------|--------------|
| `src/tracker/providers/macos.rs` | `crates/infra/src/platform/macos/provider.rs` |
| `src/tracker/os_events/macos.rs` | `crates/infra/src/platform/macos/events.rs` |
| `src/tracker/idle/detector.rs` | `crates/infra/src/platform/macos/idle/detector.rs` |
| `src/tracker/idle/period_tracker.rs` | `crates/infra/src/platform/macos/idle/period_tracker.rs` |
| `src/tracker/idle/sleep_wake.rs` | `crates/infra/src/platform/macos/idle/sleep_wake.rs` |
| `src/detection/enrichers/browser.rs` | `crates/infra/src/platform/macos/enrichers/browser.rs` |
| `src/detection/enrichers/office.rs` | `crates/infra/src/platform/macos/enrichers/office.rs` |

#### Integrations → crates/infra/src/integrations/

| Current Location | New Location |
|-----------------|--------------|
| `src/shared/auth/oauth_service.rs` | `crates/infra/src/integrations/oauth.rs` |
| `src/shared/auth/pkce.rs` | `crates/infra/src/integrations/oauth.rs` (merge) |
| | |
| `src/integrations/calendar/*` | `crates/infra/src/integrations/calendar/*` |
| `src/integrations/sap/*` | `crates/infra/src/integrations/sap/*` |
| `src/domain/api/*` | `crates/infra/src/integrations/backend_api/*` |

#### Sync → crates/infra/src/sync/

| Current Location | New Location |
|-----------------|--------------|
| `src/sync/neon_client.rs` | `crates/infra/src/integrations/backend_api/client.rs` |
| `src/sync/outbox_worker.rs` | `crates/infra/src/sync/outbox_worker.rs` |
| `src/sync/retry.rs` | `crates/infra/src/sync/retry.rs` |
| `src/sync/cost_tracker.rs` | `crates/infra/src/sync/cost_tracker.rs` |
| `src/sync/cleanup.rs` | `crates/infra/src/sync/cleanup.rs` |
| `src/sync/scheduler.rs` | Move to **api crate** |

#### Observability → crates/infra/src/observability/

| Current Location | New Location |
|-----------------|--------------|
| `src/observability/datadog.rs` | `crates/infra/src/observability/datadog.rs` |
| `src/observability/metrics/*` | `crates/infra/src/observability/metrics/*` |

---

### Phase 3: Extract Core (Week 5-6)

#### Tracking → crates/core/src/tracking/

| Current Location | New Location | Notes |
|-----------------|--------------|-------|
| N/A | `crates/core/src/tracking/ports.rs` | **Create new** (traits) |
| `src/tracker/core.rs` | `crates/core/src/tracking/service.rs` | Extract logic only |
| `src/tracker/provider.rs` | `crates/core/src/tracking/ports.rs` | Move trait definition |

**Create crates/core/src/tracking/ports.rs:**
```rust
use shared::types::ActivityContext;

/// Trait for platform-specific activity providers
pub trait ActivityProvider: Send + Sync {
    fn get_activity(&self) -> Result<ActivityContext, Error>;
}

/// Trait for activity storage
pub trait ActivityRepository: Send + Sync {
    fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<(), Error>;
    fn get_recent(&self, limit: usize) -> Result<Vec<ActivitySnapshot>, Error>;
}
```

#### Detection → crates/core/src/detection/

| Current Location | New Location |
|-----------------|--------------|
| `src/detection/mod.rs` | `crates/core/src/detection/engine.rs` |
| `src/detection/default.rs` | `crates/core/src/detection/default.rs` |
| `src/detection/packs/**/*` | `crates/core/src/detection/packs/**/*` |
| `src/detection/enrichers/*` | Stay in **infra** (platform-specific) |

#### Preprocessing → crates/core/src/preprocessing/

| Current Location | New Location |
|-----------------|--------------|
| `src/preprocess/redact.rs` | `crates/core/src/preprocessing/redact.rs` |
| `src/preprocess/segmenter.rs` | `crates/core/src/preprocessing/segmenter.rs` |
| `src/preprocess/trigger.rs` | Move to **api** (orchestration) |

#### Classification → crates/core/src/classification/

| Current Location | New Location |
|-----------------|--------------|
| N/A | `crates/core/src/classification/ports.rs` | **Create new** |
| `src/inference/tree_classifier.rs` | `crates/core/src/classification/tree.rs` |
| `src/inference/logistic_classifier.rs` | `crates/core/src/classification/logistic.rs` |
| `src/inference/rules_classifier.rs` | `crates/core/src/classification/rules.rs` |
| `src/inference/block_builder.rs` | `crates/core/src/classification/block_builder.rs` |
| `src/inference/metrics.rs` | `crates/core/src/classification/metrics.rs` |
| `src/inference/scheduler.rs` | Move to **api** (orchestration) |

#### Sync → crates/core/src/sync/

| Current Location | New Location |
|-----------------|--------------|
| N/A | `crates/core/src/sync/ports.rs` | **Create new** (traits) |

**Create crates/core/src/sync/ports.rs:**
```rust
pub trait SyncRepository: Send + Sync {
    fn get_pending_entries(&self) -> Result<Vec<OutboxEntry>, Error>;
    fn mark_sent(&self, id: &str) -> Result<(), Error>;
}

pub trait BackendClient: Send + Sync {
    async fn create_time_entry(&self, entry: TimeEntry) -> Result<String, Error>;
}
```

---

### Phase 4: Create API Crate (Week 7-8)

#### Main Structure → crates/api/src/

| Current Location | New Location | Notes |
|-----------------|--------------|-------|
| `src/main.rs` | `crates/api/src/main.rs` | Simplify to <100 lines |
| `src/lib.rs` | `crates/api/src/lib.rs` | Minimal re-exports |
| N/A | `crates/api/src/context.rs` | **Create new** |
| N/A | `crates/api/src/builder.rs` | **Create new** |
| N/A | `crates/api/src/schedulers.rs` | **Create new** |

**Create crates/api/src/context.rs:**
```rust
use std::sync::Arc;
use infra::database::DbManager;
use core::tracking::service::TrackingService;
use shared::Config;

/// Application context - replaces all global singletons
pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracker: Arc<TrackingService>,
    pub schedulers: SchedulerRegistry,
}

impl AppContext {
    pub fn builder() -> AppContextBuilder {
        AppContextBuilder::new()
    }
}
```

**Create crates/api/src/builder.rs:**
```rust
pub struct AppContextBuilder {
    app_handle: Option<AppHandle>,
}

impl AppContextBuilder {
    pub fn new() -> Self {
        Self { app_handle: None }
    }

    pub fn with_app_handle(mut self, handle: AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
    }

    pub fn build(self) -> Result<AppContext, Error> {
        // All 10-phase initialization logic here
        let config = Config::from_env()?;
        let db = DbManager::new(&config)?;
        let provider = MacOsProvider::new();
        let tracker = TrackingService::new(provider, db.clone());
        // ... initialize all services

        Ok(AppContext {
            config,
            db: Arc::new(db),
            tracker: Arc::new(tracker),
            schedulers: SchedulerRegistry::new(),
        })
    }
}
```

**Create crates/api/src/schedulers.rs:**
```rust
use std::sync::Arc;

pub struct SchedulerRegistry {
    pub sync: Arc<SyncScheduler>,
    pub blocks: Arc<BlockScheduler>,
    pub calendar: Arc<CalendarSyncScheduler>,
    pub cleanup: Arc<CleanupScheduler>,
}

impl SchedulerRegistry {
    pub fn start_all(&self) {
        self.sync.start();
        self.blocks.start();
        self.calendar.start();
        self.cleanup.start();
    }

    pub fn stop_all(&self) {
        self.sync.stop();
        self.blocks.stop();
        self.calendar.stop();
        self.cleanup.stop();
    }
}
```

#### Commands → crates/api/src/commands/

| Current Location | New Location | Changes |
|-----------------|--------------|---------|
| `src/commands/blocks.rs` | `crates/api/src/commands/blocks.rs` | Update to use `State<Arc<AppContext>>` |
| `src/commands/calendar.rs` | `crates/api/src/commands/calendar.rs` | Update to use `State<Arc<AppContext>>` |
| `src/commands/database.rs` | `crates/api/src/commands/database.rs` | Update to use `State<Arc<AppContext>>` |
| `src/commands/idle.rs` | `crates/api/src/commands/idle.rs` | Update to use `State<Arc<AppContext>>` |
| `src/commands/monitoring.rs` | `crates/api/src/commands/monitoring.rs` | Update to use `State<Arc<AppContext>>` |
| `src/commands/user_profile.rs` | `crates/api/src/commands/user_profile.rs` | Update to use `State<Arc<AppContext>>` |

**Command signature change:**
```rust
// Before
#[tauri::command]
fn pause_tracker() -> Result<()> {
    let tracker = &*TRACKER;  // Global singleton
    tracker.pause()
}

// After
#[tauri::command]
fn pause_tracker(ctx: State<Arc<AppContext>>) -> Result<()> {
    ctx.tracker.pause()  // Dependency injection
}
```

---

## Migration Commands

### Phase 0: Setup Workspace (15 minutes)

```bash
# 1. Create workspace structure
mkdir -p crates/api
mv src crates/api/
mv Cargo.toml crates/api/

# 2. Create root Cargo.toml
cat > Cargo.toml <<'EOF'
[workspace]
members = ["crates/api"]
resolver = "2"
EOF

# 3. Test - should work exactly as before
cargo build --workspace
cargo run

# ✅ Checkpoint: App still works
```

---

### Phase 1: Extract Shared (Week 2)

```bash
# 1. Create shared crate
cargo new --lib crates/shared

# 2. Create directory structure
mkdir -p crates/shared/src/{types,constants,extractors,utils}

# 3. Copy files
cp -r crates/api/src/shared/types/* crates/shared/src/types/
cp -r crates/api/src/shared/constants/* crates/shared/src/constants/
cp -r crates/api/src/shared/extractors/* crates/shared/src/extractors/
cp crates/api/src/observability/errors/mod.rs crates/shared/src/error.rs

# 4. Create config module
cat > crates/shared/src/config.rs <<'EOF'
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub webapi_url: String,
    // ... add all config fields
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        // Load from environment
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL not set")?,
            webapi_url: env::var("WEBAPI_URL")
                .unwrap_or_else(|_| "https://api.pulsearc.ai".to_string()),
            // ... load all env vars
        })
    }
}
EOF

# 5. Update crates/shared/src/lib.rs
cat > crates/shared/src/lib.rs <<'EOF'
pub mod config;
pub mod error;
pub mod types;
pub mod constants;
pub mod extractors;
pub mod utils;

// Re-export commonly used items
pub use config::Config;
pub use error::Error;
EOF

# 6. Update crates/shared/Cargo.toml
cat >> crates/shared/Cargo.toml <<'EOF'

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
EOF

# 7. Update workspace Cargo.toml
cat > Cargo.toml <<'EOF'
[workspace]
members = ["crates/shared", "crates/api"]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
EOF

# 8. Update api Cargo.toml to depend on shared
echo 'shared = { path = "../shared" }' >> crates/api/Cargo.toml

# 9. Update imports in api crate
# Find and replace: use crate::shared → use shared
find crates/api/src -type f -name "*.rs" -exec sed -i '' 's/use crate::shared/use shared/g' {} +

# 10. Build and test
cargo build --workspace
cargo test --workspace

# ✅ Checkpoint: Shared crate extracted, app still works
```

---

### Phase 2: Extract Infra (Week 3-4)

```bash
# 1. Create infra crate
cargo new --lib crates/infra

# 2. Create directory structure
mkdir -p crates/infra/src/{database/{repositories,utils},http,platform/macos/{enrichers,idle},integrations/{calendar,sap,backend_api},sync,observability/metrics}

# 3. Copy database files
cp crates/api/src/db/manager.rs crates/infra/src/database/
cp crates/api/src/db/migrations.rs crates/infra/src/database/
cp crates/api/src/db/models.rs crates/infra/src/database/

# Consolidate into repositories
cat crates/api/src/db/activity/*.rs > crates/infra/src/database/repositories/activity.rs
cat crates/api/src/db/blocks/*.rs > crates/infra/src/database/repositories/blocks.rs
cat crates/api/src/db/outbox/*.rs > crates/infra/src/database/repositories/outbox.rs
cat crates/api/src/db/calendar/*.rs > crates/infra/src/database/repositories/calendar.rs
cat crates/api/src/db/batch/*.rs > crates/infra/src/database/repositories/batch.rs

# 4. Copy HTTP files
cp -r crates/api/src/http/* crates/infra/src/http/

# 5. Copy platform files
cp crates/api/src/tracker/providers/macos.rs crates/infra/src/platform/macos/provider.rs
cp crates/api/src/tracker/os_events/macos.rs crates/infra/src/platform/macos/events.rs
cp -r crates/api/src/tracker/idle/* crates/infra/src/platform/macos/idle/
cp crates/api/src/detection/enrichers/browser.rs crates/infra/src/platform/macos/enrichers/
cp crates/api/src/detection/enrichers/office.rs crates/infra/src/platform/macos/enrichers/

# 6. Copy integration files
cp -r crates/api/src/integrations/calendar/* crates/infra/src/integrations/calendar/
cp -r crates/api/src/integrations/sap/* crates/infra/src/integrations/sap/
cp -r crates/api/src/domain/api/* crates/infra/src/integrations/backend_api/

# 7. Copy sync files (except scheduler)
cp crates/api/src/sync/outbox_worker.rs crates/infra/src/sync/
cp crates/api/src/sync/retry.rs crates/infra/src/sync/
cp crates/api/src/sync/cost_tracker.rs crates/infra/src/sync/
cp crates/api/src/sync/cleanup.rs crates/infra/src/sync/

# 8. Update infra Cargo.toml
cat >> crates/infra/Cargo.toml <<'EOF'

[dependencies]
shared = { path = "../shared" }

# Database
rusqlite = { version = "0.37", features = ["bundled-sqlcipher-vendored-openssl"] }
r2d2 = "0.8"
r2d2_sqlite = "0.31"

# HTTP
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Async
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# OAuth
oauth2 = "5.0"
keyring = "2.3"

# macOS platform
objc2 = "0.5"
objc2-foundation = { version = "0.2", features = ["all"] }
objc2-app-kit = { version = "0.2", features = ["all"] }
core-graphics = "0.24"

# Error handling
thiserror = "2.0"
anyhow = "1.0"
EOF

# 9. Update workspace and api Cargo.toml
cat > Cargo.toml <<'EOF'
[workspace]
members = ["crates/shared", "crates/infra", "crates/api"]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
EOF

echo 'infra = { path = "../infra" }' >> crates/api/Cargo.toml

# 10. Update imports in api crate
find crates/api/src -type f -name "*.rs" -exec sed -i '' 's/use crate::db/use infra::database/g' {} +
find crates/api/src -type f -name "*.rs" -exec sed -i '' 's/use crate::http/use infra::http/g' {} +

# 11. Build and test
cargo build --workspace
cargo test --workspace

# ✅ Checkpoint: Infra crate extracted, app still works
```

---

### Phase 3: Extract Core (Week 5-6)

```bash
# 1. Create core crate
cargo new --lib crates/core

# 2. Create directory structure
mkdir -p crates/core/src/{tracking,detection/packs,preprocessing,classification,sync}

# 3. Create trait definitions
cat > crates/core/src/tracking/ports.rs <<'EOF'
use shared::types::ActivityContext;
use shared::error::Error;

pub trait ActivityProvider: Send + Sync {
    fn get_activity(&self) -> Result<ActivityContext, Error>;
}

pub trait ActivityRepository: Send + Sync {
    fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<(), Error>;
    fn get_recent(&self, limit: usize) -> Result<Vec<ActivitySnapshot>, Error>;
}
EOF

cat > crates/core/src/classification/ports.rs <<'EOF'
pub trait Classifier: Send + Sync {
    fn classify(&self, block: &ProposedBlock) -> Result<ClassificationResult, Error>;
}
EOF

cat > crates/core/src/sync/ports.rs <<'EOF'
pub trait SyncRepository: Send + Sync {
    fn get_pending_entries(&self) -> Result<Vec<OutboxEntry>, Error>;
}

pub trait BackendClient: Send + Sync {
    async fn create_time_entry(&self, entry: TimeEntry) -> Result<String, Error>;
}
EOF

# 4. Copy detection logic (not enrichers)
cp crates/api/src/detection/mod.rs crates/core/src/detection/engine.rs
cp crates/api/src/detection/default.rs crates/core/src/detection/
cp -r crates/api/src/detection/packs/* crates/core/src/detection/packs/

# 5. Copy preprocessing logic
cp crates/api/src/preprocess/redact.rs crates/core/src/preprocessing/
cp crates/api/src/preprocess/segmenter.rs crates/core/src/preprocessing/

# 6. Copy classification logic
cp crates/api/src/inference/tree_classifier.rs crates/core/src/classification/tree.rs
cp crates/api/src/inference/logistic_classifier.rs crates/core/src/classification/logistic.rs
cp crates/api/src/inference/rules_classifier.rs crates/core/src/classification/rules.rs
cp crates/api/src/inference/block_builder.rs crates/core/src/classification/
cp crates/api/src/inference/metrics.rs crates/core/src/classification/

# 7. Create tracking service
cat > crates/core/src/tracking/service.rs <<'EOF'
use super::ports::{ActivityProvider, ActivityRepository};
use shared::types::ActivityContext;
use std::sync::Arc;

pub struct TrackingService {
    provider: Arc<dyn ActivityProvider>,
    repository: Arc<dyn ActivityRepository>,
}

impl TrackingService {
    pub fn new(
        provider: Arc<dyn ActivityProvider>,
        repository: Arc<dyn ActivityRepository>,
    ) -> Self {
        Self { provider, repository }
    }

    pub fn pause(&self) -> Result<(), Error> {
        // Business logic for pausing
    }

    pub fn resume(&self) -> Result<(), Error> {
        // Business logic for resuming
    }
}
EOF

# 8. Update core Cargo.toml
cat >> crates/core/Cargo.toml <<'EOF'

[dependencies]
shared = { path = "../shared" }

# ML (optional)
linfa = { version = "0.7", optional = true }
linfa-trees = { version = "0.7", optional = true }
linfa-logistic = { version = "0.7", optional = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Async
tokio = { workspace = true }

[features]
default = ["ml"]
ml = ["dep:linfa", "dep:linfa-trees", "dep:linfa-logistic"]
EOF

# 9. Update workspace
cat > Cargo.toml <<'EOF'
[workspace]
members = ["crates/shared", "crates/core", "crates/infra", "crates/api"]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
EOF

echo 'core = { path = "../core" }' >> crates/api/Cargo.toml

# 10. Implement traits in infra
# Edit crates/infra/src/platform/macos/provider.rs
# Add: impl ActivityProvider for MacOsProvider { ... }

# 11. Build and test
cargo build --workspace
cargo test --workspace

# ✅ Checkpoint: Core crate extracted with trait-based architecture
```

---

### Phase 4: Create AppContext (Week 7-8)

```bash
# 1. Create context module
cat > crates/api/src/context.rs <<'EOF'
use std::sync::Arc;
use infra::database::DbManager;
use core::tracking::service::TrackingService;
use shared::Config;

pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracker: Arc<TrackingService>,
    pub schedulers: SchedulerRegistry,
}

impl AppContext {
    pub fn builder() -> AppContextBuilder {
        AppContextBuilder::new()
    }
}
EOF

# 2. Create builder module
cat > crates/api/src/builder.rs <<'EOF'
use super::context::AppContext;
use tauri::AppHandle;

pub struct AppContextBuilder {
    app_handle: Option<AppHandle>,
}

impl AppContextBuilder {
    pub fn new() -> Self {
        Self { app_handle: None }
    }

    pub fn with_app_handle(mut self, handle: AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
    }

    pub fn build(self) -> Result<AppContext, Box<dyn std::error::Error>> {
        // Load config
        let config = Config::from_env()?;

        // Initialize database
        let db = DbManager::new(&config)?;
        let db = Arc::new(db);

        // Initialize platform provider
        let provider = Arc::new(MacOsProvider::new());

        // Initialize tracking service
        let tracker = Arc::new(TrackingService::new(provider, db.clone()));

        // Initialize schedulers
        let schedulers = SchedulerRegistry::new(
            self.app_handle.expect("app_handle required"),
            db.clone(),
            &config,
        );

        Ok(AppContext {
            config,
            db,
            tracker,
            schedulers,
        })
    }
}
EOF

# 3. Create schedulers module
cat > crates/api/src/schedulers.rs <<'EOF'
use std::sync::Arc;
use infra::sync::SyncScheduler;
use infra::integrations::calendar::scheduler::CalendarSyncScheduler;

pub struct SchedulerRegistry {
    sync: Arc<SyncScheduler>,
    calendar: Arc<CalendarSyncScheduler>,
    // ... other schedulers
}

impl SchedulerRegistry {
    pub fn new(app_handle: AppHandle, db: Arc<DbManager>, config: &Config) -> Self {
        // Initialize all schedulers
        Self {
            sync: Arc::new(SyncScheduler::new(/* ... */)),
            calendar: Arc::new(CalendarSyncScheduler::new(/* ... */)),
        }
    }

    pub fn start_all(&self) {
        self.sync.start();
        self.calendar.start();
    }

    pub fn stop_all(&self) {
        self.sync.stop();
        self.calendar.stop();
    }
}
EOF

# 4. Simplify main.rs
cat > crates/api/src/main.rs <<'EOF'
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use std::sync::Arc;

mod context;
mod builder;
mod schedulers;
mod commands;

use context::AppContext;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Build context
            let ctx = AppContext::builder()
                .with_app_handle(app.handle())
                .build()?;

            // Start schedulers
            ctx.schedulers.start_all();

            // Manage state
            app.manage(Arc::new(ctx));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pause_tracker,
            commands::resume_tracker,
            // ... all commands
        ])
        .run(tauri::generate_context!())
        .expect("error running app");
}
EOF

# 5. Update all commands to use AppContext
# Example for one command:
cat > crates/api/src/commands/tracking.rs <<'EOF'
use tauri::State;
use std::sync::Arc;
use crate::context::AppContext;

#[tauri::command]
pub fn pause_tracker(ctx: State<Arc<AppContext>>) -> Result<(), String> {
    ctx.tracker.pause()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resume_tracker(ctx: State<Arc<AppContext>>) -> Result<(), String> {
    ctx.tracker.resume()
        .map_err(|e| e.to_string())
}
EOF

# 6. Delete old singleton code
rm crates/api/src/lib.rs  # Remove global singletons

# 7. Build and test
cargo build --workspace
cargo run

# ✅ Checkpoint: Zero global singletons, all dependency injection
```

---

## Key Transformations

### 1. Global Singletons → AppContext

**Before:**
```rust
// lib.rs
pub static TRACKER: Lazy<Arc<Tracker>> = Lazy::new(|| {
    Arc::new(Tracker::new())
});

pub static DB: Lazy<Arc<Mutex<Option<LocalDatabase>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

// command
#[tauri::command]
fn pause_tracker() -> Result<()> {
    let tracker = &*TRACKER;  // Hidden dependency
    tracker.pause()
}
```

**After:**
```rust
// context.rs
pub struct AppContext {
    pub tracker: Arc<TrackingService>,
    pub db: Arc<DbManager>,
}

// command
#[tauri::command]
fn pause_tracker(ctx: State<Arc<AppContext>>) -> Result<()> {
    ctx.tracker.pause()  // Explicit dependency
}
```

---

### 2. Concrete Implementation → Trait-Based

**Before:**
```rust
// tracker/core.rs
pub struct Tracker {
    provider: MacOsProvider,  // Hardcoded to macOS
    db: Arc<LocalDatabase>,
}

impl Tracker {
    pub fn get_activity(&self) -> ActivityContext {
        self.provider.fetch()  // Direct call
    }
}
```

**After:**
```rust
// core/tracking/ports.rs
pub trait ActivityProvider: Send + Sync {
    fn get_activity(&self) -> Result<ActivityContext>;
}

// core/tracking/service.rs
pub struct TrackingService {
    provider: Arc<dyn ActivityProvider>,  // Trait object
    repository: Arc<dyn ActivityRepository>,
}

impl TrackingService {
    pub fn get_activity(&self) -> Result<ActivityContext> {
        self.provider.get_activity()  // Polymorphic call
    }
}

// infra/platform/macos/provider.rs
impl ActivityProvider for MacOsProvider {
    fn get_activity(&self) -> Result<ActivityContext> {
        // macOS-specific implementation
    }
}
```

---

### 3. Direct DB Access → Repository Pattern

**Before:**
```rust
// Scattered everywhere
let conn = get_database()?.get_connection()?;
conn.execute("INSERT INTO ...", params![])?;
```

**After:**
```rust
// core/tracking/ports.rs
pub trait ActivityRepository: Send + Sync {
    fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()>;
}

// infra/database/repositories/activity.rs
pub struct SqliteActivityRepository {
    db: Arc<DbManager>,
}

impl ActivityRepository for SqliteActivityRepository {
    fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()> {
        let conn = self.db.get_connection()?;
        conn.execute("INSERT INTO ...", params![])?;
        Ok(())
    }
}

// Usage in service
service.repository.save_snapshot(snapshot)?;
```

---

### 4. Monolithic main.rs → Builder Pattern

**Before (1,917 lines):**
```rust
// main.rs
fn main() {
    // Phase 1: Initialize database (100 lines)
    let db = initialize_database().unwrap();

    // Phase 2: Initialize tracker (150 lines)
    let tracker = initialize_tracker().unwrap();

    // Phase 3: Initialize schedulers (200 lines)
    let sync_scheduler = initialize_sync_scheduler().unwrap();

    // ... 7 more phases ...

    tauri::Builder::default()
        .setup(|app| {
            // ... setup logic ...
        })
        .run(...)
}
```

**After (<100 lines):**
```rust
// main.rs
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let ctx = AppContext::builder()
                .with_app_handle(app.handle())
                .build()?;
            ctx.schedulers.start_all();
            app.manage(Arc::new(ctx));
            Ok(())
        })
        .run(...)
}

// builder.rs (all initialization logic)
impl AppContextBuilder {
    pub fn build(self) -> Result<AppContext> {
        // All 10 phases here
    }
}
```

---

### 5. Multiple DB Patterns → Single Pool

**Before (3 patterns):**
```rust
// Pattern 1: Singleton
let db = &*DB.lock().unwrap();

// Pattern 2: Direct manager
let manager = DbManager::new()?;

// Pattern 3: get_database()
let db = get_database()?;
```

**After (1 pattern):**
```rust
// Always through AppContext
ctx.db.get_connection()?  // Connection pool
```

---

## Benefits Summary

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| **Compile time** | 2-3 min | 30-60 sec | **50-70%** ⚡ |
| **Global singletons** | 5+ | 0 | **100%** ✅ |
| **main.rs size** | 1,917 lines | <100 lines | **95%** 📉 |
| **Crates** | 1 monolith | 4 focused | **4x** 🏗️ |
| **Testability** | Hard | Easy | **Mockable** 🧪 |
| **DB patterns** | 3 | 1 | **Unified** 🎯 |
| **Dependencies** | Circular | Clean DAG | **No cycles** ♻️ |

---

## Dependency Graph

```
┌─────────────────────────────────────┐
│          crates/api                 │
│    (Tauri app + commands)           │
│    - main.rs (<100 lines)           │
│    - context.rs (AppContext)        │
│    - builder.rs (initialization)    │
│    - commands/ (Tauri commands)     │
└───────┬─────────────┬───────────────┘
        │             │
        │             │
   ┌────▼────┐   ┌────▼────┐
   │  core   │   │  infra  │
   │         │   │         │
   └────┬────┘   └────┬────┘
        │             │
        └──────┬──────┘
               │
          ┌────▼────┐
          │ shared  │
          │         │
          └─────────┘

Dependency Rules:
✅ api → {core, infra, shared}
✅ infra → {core, shared}
✅ core → {shared}
✅ shared → {}
❌ No circular dependencies
```

---

## Next Steps

1. **Phase 0** (15 min): Create workspace - zero risk
2. **Phase 1** (Week 2): Extract shared crate
3. **Phase 2** (Week 3-4): Extract infra crate
4. **Phase 3** (Week 5-6): Extract core crate with traits
5. **Phase 4** (Week 7-8): Create AppContext and kill singletons
6. **Phase 5** (Week 9): Clean up main.rs
7. **Phase 6** (Week 10): Unify DB access patterns

**Total Timeline:** 10 weeks (incremental, safe)

---

**Document Status:** Complete Reference
**Created:** Oct 30, 2025
**Purpose:** Step-by-step guide for refactoring PulseArc to workspace architecture
