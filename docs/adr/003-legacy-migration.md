I'll help you draft a comprehensive ADR for porting the legacy code into the layered workspace architecture. Let me first examine the current codebase structure to understand what we're working with.

[7 tools called]

Now I have enough context. Let me draft the comprehensive ADR and migration plan for you.

---

## Key Principles from COMMON_CRATES_GUIDE.md

Based on the attached guide, here are the critical rules that directly impact the migration:

1. **Tiered Feature System**: Use opt-in feature flags (foundation/runtime/platform) with no defaults. Pay only for what you use. Apply this to `infra` adapters.

2. **Safety First**: `#![forbid(unsafe_code)]` across all crates. No exceptions outside of `platform` if absolutely necessary with documentation.

3. **Error Composition**: Domain/core use module-specific errors that compose with `CommonError` via `#[from]`. Standard patterns (timeouts, rate limits) use `CommonError`. Implement `ErrorClassification` for resilience.

4. **Observability Boundaries**: Domain/core define errors but avoid direct logging. Infra/api use `tracing` spans/fields around IO boundaries. Structured logging with fields, JSON output in production.

5. **Pure Domain Layer**: Domain crate has zero infrastructure dependencies. Only foundational crates (serde, chrono, uuid, thiserror). Forms ubiquitous language.

6. **Hexagonal Architecture**: Core defines ports (traits), infra provides adapters (implementations). Business logic has zero platform-specific code. Testability via mock implementations.

7. **No `unwrap`/`expect`/`panic`**: Except in tests. Explicit error propagation. Use `thiserror` in libraries, `anyhow` at application boundaries.

8. **Dependency Direction**: Common ← Domain ← Core ← Infra ← API. No back-edges. Domain depends only on external foundational crates, not other PulseArc crates.

---

## A) Architecture Decision Record (ADR)

```markdown
# ADR-003: Port `legacy/` and `api/src/` into `core`/`domain`/`infra`/`api` Layered Workspace

## Status
**Proposed**

**Date:** October 31, 2025

**Supersedes:** N/A (additive migration from legacy structure)

**Related:** [ADR-002: Production Architecture](./002-production-architecture.md), [Common Crates Guide](../../crates/common/docs/COMMON_CRATES_GUIDE.md)

---

## Context

### Current State

PulseArc currently has two distinct code structures:

1. **Active workspace crates** (`crates/`):
   - `api/` - Minimal Tauri commands (4 modules: calendar, projects, suggestions, tracking)
   - `core/` - Business logic with hexagonal ports (classification, tracking services)
   - `domain/` - Pure domain types (config, errors, types)
   - `infra/` - Infrastructure adapters (database, http, platform, integrations)
   - `common/` - Shared infrastructure with tiered features

2. **Legacy monolith** (`legacy/api/`):
   - ~150+ modules spanning all architectural layers
   - Mixed concerns: commands, database, domain logic, integrations, ML inference
   - No clear separation between business logic and infrastructure
   - Direct dependencies on platform APIs, databases, HTTP clients throughout
   - Difficult to test in isolation

### Problems with Current Structure

1. **Architectural Drift**: Legacy code violates hexagonal architecture principles:
   - Business logic directly coupled to SQLite, HTTP clients, platform APIs
   - No trait boundaries enabling mock implementations
   - Commands contain both presentation and domain logic

2. **Testability**: Impossible to unit test domain logic without real databases, HTTP servers, or macOS APIs

3. **Compilation Times**: Monolithic structure forces recompilation of unrelated code

4. **Maintenance Burden**: Two parallel implementations (active workspace vs legacy) with unclear ownership

5. **Security/Compliance**: Mixed abstraction levels make audit logging, encryption, and PII handling inconsistent

6. **Onboarding**: New developers struggle to understand boundaries and where new code belongs

### Principles from Common Crates Guide

The migration must align with established patterns in `pulsearc-common`:

- **Tiered dependencies**: Foundation (no async/logging) → Runtime (async infrastructure) → Platform (system integrations)
- **Opt-in features**: Feature flags control compilation of optional components
- **Error classification**: Unified error handling with `CommonError` + domain-specific errors implementing `ErrorClassification`
- **Observability**: Structured `tracing` spans at IO boundaries, not in domain logic
- **Safety**: `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`panic` outside tests
- **Hexagonal ports**: Traits define contracts, implementations are swappable

---

## Decision

**Adopt a strict layered crate architecture and migrate all code from `legacy/api/` into `core`/`domain`/`infra`/`api` according to architectural boundaries.**

Key architectural principles:

1. **Unidirectional dependencies**: `api` → `infra` → `core` → `domain` → `common`
2. **Pure domain layer**: Domain contains only data types, errors, and constants. Zero infrastructure.
3. **Hexagonal boundaries**: Core defines business logic and ports (traits). Infra implements adapters.
4. **Presentation isolation**: API layer handles Tauri commands, mapping, validation, and dependency wiring.
5. **Feature flags**: Infra uses additive feature flags for optional adapters (ML, GraphQL, SAP, calendar providers).

---

## Options Considered

### Option 1: Keep Current Layout (Rejected)

**Description**: Maintain parallel `legacy/` and `crates/` structures indefinitely.

**Pros**:
- Zero migration effort
- No risk of breaking existing functionality

**Cons**:
- Continued architectural drift
- Duplicate implementations
- Technical debt accumulation
- Poor testability
- Confusion about where new code belongs

### Option 2: Partial Refactor (Rejected)

**Description**: Move only "easy" modules to workspace, leave complex areas in legacy.

**Pros**:
- Lower initial effort
- Incremental risk

**Cons**:
- Unclear completion criteria
- Perpetual mixed state
- Doesn't solve fundamental testability/boundary issues
- Still maintains two code paths

### Option 3: Full Layered Refactor (CHOSEN)

**Description**: Systematically migrate all code to layered architecture with clear boundaries.

**Pros**:
- Clear architectural boundaries
- Full testability (mock implementations)
- Faster compilation (incremental builds)
- Enables parallel development
- Easier onboarding
- Audit trail and compliance infrastructure

**Cons**:
- Significant upfront effort (~4-8 weeks)
- Risk of breaking existing functionality
- Requires comprehensive test coverage

**Decision Rationale**: Option 3 chosen because it's the only option that addresses root causes and aligns with production architecture (ADR-002). The benefits (testability, maintainability, clarity) far outweigh the one-time migration cost.

---

## Decision Drivers

### 1. Testability
- **Current**: Cannot unit test business logic without real databases/APIs
- **Target**: Mock implementations enable fast, isolated tests

### 2. Maintainability
- **Current**: Changes ripple across layers unpredictably
- **Target**: Changes isolated to single crate with clear interfaces

### 3. Compile Times
- **Current**: Legacy monolith forces full recompilation
- **Target**: Layered crates enable incremental compilation

### 4. Clarity
- **Current**: Unclear where new code belongs
- **Target**: Unambiguous placement rules

### 5. Security/Compliance
- **Current**: Mixed encryption, audit logging, PII handling
- **Target**: Consistent application of security patterns at boundaries

### 6. Migration Safety
- **Current**: N/A
- **Target**: Incremental migration with deprecation path, temporary shims

---

## Detailed Design

### 1. Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                          api                                │
│  (Tauri commands, mapping, validation, DI wiring)          │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                         infra                               │
│  (Adapters: DB, HTTP, platform APIs, integrations)         │
│  Features: sap, calendar, ml, graphql                       │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                          core                               │
│  (Business logic, use cases, ports/traits)                 │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                         domain                              │
│  (Pure types, errors, config - no infra deps)              │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                         common                              │
│  (Tiered shared infrastructure)                            │
└─────────────────────────────────────────────────────────────┘
```

**Allowed edges**:
- api → {infra, core, domain, common}
- infra → {core, domain, common}
- core → {domain, common}
- domain → {common} (foundation tier only)
- common → {external deps only}

**FORBIDDEN edges** (must be rejected by CI):
- domain → core
- domain → infra
- domain → api
- core → infra
- core → api
- infra → api

### 2. Module Placement Rules

#### Domain Crate (`pulsearc-domain`)

**Criteria**: Pure data types, zero side effects, no infrastructure dependencies.

**Allowed**:
- Data structures (Activity, TimeEntry, Project, WbsCode)
- Configuration types (AppConfig, DatabaseConfig, TrackingConfig)
- Domain errors (custom error types with `thiserror`)
- Domain constants and enums
- Value objects (validated types like Email, ProjectId)

**Examples**:
- `legacy/api/src/db/models.rs` → `domain/src/types/time_entry.rs`
- `legacy/api/src/shared/types/stats.rs` → `domain/src/types/stats.rs`
- `legacy/api/src/shared/config.rs` → `domain/src/config/app_config.rs`
- `legacy/api/src/inference/types.rs` → `domain/src/types/classification.rs`

**Dependencies**: serde, chrono, uuid, thiserror ONLY. NO tokio, rusqlite, reqwest, objc2.

**Forbidden**:
- Database queries or connections
- HTTP clients
- File I/O
- Platform APIs (macOS, keychain)
- Async runtime assumptions

#### Core Crate (`pulsearc-core`)

**Criteria**: Business logic, use cases, hexagonal ports (traits). No concrete I/O.

**Allowed**:
- Service implementations (TrackingService, ClassificationService)
- Port trait definitions (ActivityProvider, ActivityRepository, TimeEntryRepository)
- Use case orchestration
- Business rule validation
- Domain event handling

**Examples**:
- `legacy/api/src/tracker/core.rs` → `core/src/tracking/service.rs`
- `legacy/api/src/inference/hybrid_classifier.rs` → `core/src/classification/hybrid_service.rs`
- `legacy/api/src/preprocess/segmenter.rs` → `core/src/tracking/segmenter.rs` (if pure logic)
- Define traits: `ActivityProvider`, `CalendarProvider`, `SapClient`, `ClassificationEngine`

**Dependencies**: domain, common (foundation/runtime), async-trait

**Forbidden**:
- Concrete database implementations (no rusqlite, no `Connection`)
- Concrete HTTP clients (no reqwest)
- Platform-specific code (no objc2)
- Direct file system access

**Pattern**: Services accept `Arc<dyn PortTrait>` for dependencies.

#### Infra Crate (`pulsearc-infra`)

**Criteria**: Implements core ports. All side effects. Feature flags for optional components.

**Allowed**:
- Database implementations (SqliteActivityRepository, SqliteTimeEntryRepository)
- HTTP client adapters (ReqwestSapClient, ReqwestCalendarClient)
- Platform providers (MacOsActivityProvider via Accessibility API)
- External service integrations (calendar OAuth, SAP authentication)
- Encryption, keychain, file I/O
- Migrations

**Examples**:
- `legacy/api/src/db/activity/snapshots.rs` → `infra/src/database/activity_repository.rs`
- `legacy/api/src/integrations/sap/client.rs` → `infra/src/integrations/sap/client.rs`
- `legacy/api/src/integrations/calendar/providers/*.rs` → `infra/src/integrations/calendar/providers/*.rs`
- `legacy/api/src/tracker/os_events/macos_ax.rs` → `infra/src/platform/macos/activity_provider.rs`
- `legacy/api/src/http/client.rs` → `infra/src/http/client.rs`
- `legacy/api/src/inference/openai_types.rs` + `openai_client` → `infra/src/ml/openai_client.rs`

**Feature flags** (in `Cargo.toml`):
```toml
[features]
default = ["database", "platform"]
database = ["dep:rusqlite", "dep:r2d2"]
platform = ["dep:objc2", "dep:objc2-app-kit", "dep:cocoa"]
ml = ["dep:linfa", "dep:linfa-trees"]
sap = ["dep:oauth2"]
calendar = ["dep:oauth2"]
graphql = ["dep:graphql_client"]
```

**Dependencies**: core, domain, common (runtime/platform), rusqlite, reqwest, oauth2, keyring, objc2

**Pattern**: Implement `#[async_trait] impl PortTrait for ConcreteAdapter { ... }`

#### API Crate (`pulsearc-api`)

**Criteria**: Tauri commands, mapping, validation, dependency injection wiring.

**Allowed**:
- Tauri command handlers (`#[tauri::command]`)
- AppContext (DI container)
- Request/response mapping (domain types ↔ frontend types)
- Input validation
- Error mapping (domain errors → frontend errors)
- Service wiring (connect infra adapters to core services)

**Examples**:
- `legacy/api/src/commands/*.rs` → `api/src/commands/*.rs`
- `legacy/api/src/main.rs` → `api/src/main.rs` (Tauri setup)
- Create `api/src/context.rs` - AppContext with all services
- Create `api/src/mapping/` for type conversions

**Dependencies**: infra, core, domain, common, tauri, serde_json

**Pattern**:
```rust
#[tauri::command]
pub async fn get_timeline(
    ctx: State<'_, Arc<AppContext>>,
    start: String,
    end: String,
) -> Result<Vec<ActivitySnapshot>, String> {
    let start = parse_datetime(&start).map_err(|e| e.to_string())?;
    let end = parse_datetime(&end).map_err(|e| e.to_string())?;
    
    ctx.tracking_service
        .get_timeline(start, end)
        .await
        .map_err(|e| e.to_string())
}
```

**Forbidden**:
- Business logic (belongs in core)
- Direct database queries (use core services)
- Direct platform API calls (use infra providers)

### 3. Ports Pattern (Hexagonal Architecture)

**Define ports in `core`**, implement adapters in `infra`.

Example: Activity tracking

**Core** (`core/src/tracking/ports.rs`):
```rust
use pulsearc_domain::{ActivitySnapshot, ActivityContext, Result};

#[async_trait]
pub trait ActivityProvider: Send + Sync {
    async fn capture_activity(&self) -> Result<ActivitySnapshot>;
    async fn pause(&self) -> Result<()>;
    async fn resume(&self) -> Result<()>;
    fn is_paused(&self) -> bool;
}

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()>;
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySnapshot>>;
}
```

**Infra** (`infra/src/platform/macos/activity_provider.rs`):
```rust
use pulsearc_core::tracking::ActivityProvider;

pub struct MacOsActivityProvider {
    paused: AtomicBool,
}

#[async_trait]
impl ActivityProvider for MacOsActivityProvider {
    async fn capture_activity(&self) -> Result<ActivitySnapshot> {
        // macOS Accessibility API implementation
    }
    
    // ... other methods
}
```

**Infra** (`infra/src/database/activity_repository.rs`):
```rust
use pulsearc_core::tracking::ActivityRepository;

pub struct SqliteActivityRepository {
    pool: Arc<SqlCipherPool>,
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()> {
        // SQLite implementation
    }
}
```

**API** (`api/src/context.rs`):
```rust
pub struct AppContext {
    pub tracking_service: Arc<TrackingService>,
}

impl AppContext {
    pub async fn new() -> Result<Self> {
        let activity_provider: Arc<dyn ActivityProvider> = 
            Arc::new(MacOsActivityProvider::new());
        
        let activity_repository: Arc<dyn ActivityRepository> = 
            Arc::new(SqliteActivityRepository::new(pool));
        
        let tracking_service = Arc::new(TrackingService::new(
            activity_provider,
            activity_repository,
        ));
        
        Ok(Self { tracking_service })
    }
}
```

### 4. Error Mapping and Observability

#### Domain Layer
```rust
// domain/src/errors.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PulseArcError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Classification failed: {0}")]
    Classification(String),
    
    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },
    
    #[error(transparent)]
    Common(#[from] pulsearc_common::error::CommonError),
}

pub type Result<T> = std::result::Result<T, PulseArcError>;
```

#### Core Layer
```rust
// core/src/tracking/service.rs
use pulsearc_domain::{Result, PulseArcError};

impl TrackingService {
    pub async fn capture_and_save(&self) -> Result<ActivitySnapshot> {
        let snapshot = self.provider
            .capture_activity()
            .await
            .map_err(|e| PulseArcError::Platform(e.to_string()))?;
        
        self.repository
            .save(&snapshot)
            .await?;
        
        Ok(snapshot)
    }
}
```

#### Infra Layer
```rust
// infra/src/database/activity_repository.rs
use tracing::{instrument, warn};

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    #[instrument(skip(self, snapshot), fields(snapshot_id = %snapshot.id))]
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()> {
        let conn = self.pool.get()
            .map_err(|e| {
                warn!("Failed to get DB connection: {}", e);
                PulseArcError::Database(e.to_string())
            })?;
        
        conn.execute(/* ... */)
            .map_err(|e| PulseArcError::Database(e.to_string()))?;
        
        Ok(())
    }
}
```

#### API Layer
```rust
// api/src/commands/tracking.rs
use tracing::{error, info};

#[tauri::command]
pub async fn get_activity(
    ctx: State<'_, Arc<AppContext>>
) -> Result<ActivitySnapshot, String> {
    match ctx.tracking_service.capture_and_save().await {
        Ok(snapshot) => {
            info!("Captured activity: {}", snapshot.id);
            Ok(snapshot)
        }
        Err(e) => {
            error!("Failed to capture activity: {}", e);
            Err(e.to_string())
        }
    }
}
```

### 5. Feature Flag Strategy

**Infra Cargo.toml**:
```toml
[features]
default = ["database", "platform"]

# Core features (always needed)
database = ["dep:rusqlite", "dep:r2d2", "pulsearc-common/storage"]
platform = ["dep:objc2", "dep:objc2-app-kit", "dep:cocoa"]

# Optional integrations
ml = ["dep:linfa", "dep:linfa-trees", "dep:ndarray"]
sap = ["dep:oauth2", "pulsearc-common/auth"]
calendar = ["dep:oauth2", "pulsearc-common/auth"]
graphql = ["dep:graphql_client"]

# Feature groups
integrations = ["sap", "calendar"]
all = ["ml", "integrations", "graphql"]
```

**Module gating**:
```rust
// infra/src/lib.rs
pub mod database;
pub mod platform;

#[cfg(feature = "ml")]
pub mod ml;

#[cfg(feature = "sap")]
pub mod integrations {
    pub mod sap;
}

#[cfg(feature = "calendar")]
pub mod integrations {
    pub mod calendar;
}
```

---

## Migration Plan

### Phase 1: Preparation (Week 1)

1. **Inventory Legacy Modules**
   - Classify each `legacy/api/src/` module by target crate
   - Identify shared types used across boundaries
   - Map external dependencies to crates
   
2. **Extract Shared Domain Types**
   - Move `legacy/api/src/db/models.rs` → `domain/src/types/`
   - Move `legacy/api/src/shared/types/` → `domain/src/types/`
   - Move `legacy/api/src/shared/config.rs` → `domain/src/config/`
   - Move `legacy/api/src/inference/types.rs` → `domain/src/types/classification.rs`
   
3. **Define Core Ports**
   - Create `core/src/tracking/ports.rs` (ActivityProvider, ActivityRepository)
   - Create `core/src/classification/ports.rs` (Classifier, TimeEntryRepository)
   - Create `core/src/sync/ports.rs` (OutboxQueue, SyncClient)
   - Create `core/src/integrations/ports.rs` (CalendarProvider, SapClient)

### Phase 2: Core Services (Week 2)

1. **Tracking Service**
   - Move `legacy/api/src/tracker/core.rs` → `core/src/tracking/service.rs`
   - Move `legacy/api/src/preprocess/segmenter.rs` → `core/src/tracking/segmenter.rs`
   - Refactor to depend on ports, not concrete implementations
   
2. **Classification Service**
   - Move `legacy/api/src/inference/hybrid_classifier.rs` → `core/src/classification/hybrid.rs`
   - Move `legacy/api/src/inference/rules_classifier.rs` → `core/src/classification/rules.rs`
   - Move `legacy/api/src/inference/evidence_extractor.rs` → `core/src/classification/evidence.rs`
   - Extract port trait for classification engines

### Phase 3: Infra Adapters (Week 3-4)

1. **Database Adapters**
   - Move `legacy/api/src/db/activity/` → `infra/src/database/activity_repository.rs`
   - Move `legacy/api/src/db/blocks/` → `infra/src/database/block_repository.rs`
   - Move `legacy/api/src/db/calendar/` → `infra/src/database/calendar_repository.rs`
   - Move `legacy/api/src/db/outbox/` → `infra/src/database/outbox_repository.rs`
   - Move `legacy/api/src/db/migrations.rs` → `infra/src/database/migrations.rs`
   
2. **Platform Adapters**
   - Move `legacy/api/src/tracker/os_events/macos_ax.rs` → `infra/src/platform/macos/activity_provider.rs`
   - Move `legacy/api/src/detection/enrichers/` → `infra/src/platform/enrichers/`
   
3. **Integration Adapters** (behind feature flags)
   - Move `legacy/api/src/integrations/sap/` → `infra/src/integrations/sap/` (feature = "sap")
   - Move `legacy/api/src/integrations/calendar/` → `infra/src/integrations/calendar/` (feature = "calendar")
   - Move `legacy/api/src/inference/linfa_integration.rs` → `infra/src/ml/linfa_classifier.rs` (feature = "ml")
   - Move `legacy/api/src/http/` → `infra/src/http/`

### Phase 4: API Layer (Week 5)

1. **Commands Migration**
   - Move `legacy/api/src/commands/` → `api/src/commands/`
   - Refactor to use AppContext services instead of direct DB/HTTP calls
   
2. **Context Wiring**
   - Build comprehensive `api/src/context.rs` with DI container
   - Wire infra adapters → core services
   
3. **Mapping Layer**
   - Create `api/src/mapping/` for domain ↔ frontend type conversions

### Phase 5: Testing & Verification (Week 6)

1. **Unit Tests**
   - Core tests with mock ports (`core/tests/` with mock implementations)
   - Infra tests with in-memory adapters or test fixtures
   
2. **Integration Tests**
   - `api/tests/` with real infra adapters in test mode
   - End-to-end command tests
   
3. **Manual Testing**
   - Full application smoke tests
   - Performance comparison (legacy vs migrated paths)

### Phase 6: Deprecation & Cleanup (Week 7-8)

1. **Mark Legacy Deprecated**
   - Add `#[deprecated]` attributes to legacy modules
   - Add `legacy` feature flag (default = false)
   - Update documentation to point to new paths
   
2. **Gradual Removal**
   - Remove `legacy/` directory after 2-week deprecation period
   - Clean up duplicate code in `api/`
   - Final audit of dependency graph

---

## Classification & Mapping Table

| Legacy Path | Target Crate | Target Path | Rationale |
|-------------|--------------|-------------|-----------|
| `legacy/api/src/db/models.rs` | `domain` | `domain/src/types/time_entry.rs` | Pure data types |
| `legacy/api/src/shared/config.rs` | `domain` | `domain/src/config/app_config.rs` | Configuration types |
| `legacy/api/src/inference/types.rs` | `domain` | `domain/src/types/classification.rs` | Domain types |
| `legacy/api/src/tracker/core.rs` | `core` | `core/src/tracking/service.rs` | Business logic |
| `legacy/api/src/inference/hybrid_classifier.rs` | `core` | `core/src/classification/hybrid.rs` | Business logic (port-based) |
| `legacy/api/src/preprocess/segmenter.rs` | `core` | `core/src/tracking/segmenter.rs` | Pure logic (if no IO) |
| `legacy/api/src/db/activity/snapshots.rs` | `infra` | `infra/src/database/activity_repository.rs` | Database adapter |
| `legacy/api/src/tracker/os_events/macos_ax.rs` | `infra` | `infra/src/platform/macos/activity_provider.rs` | Platform adapter |
| `legacy/api/src/integrations/sap/client.rs` | `infra` | `infra/src/integrations/sap/client.rs` | External service adapter |
| `legacy/api/src/integrations/calendar/providers/` | `infra` | `infra/src/integrations/calendar/providers/` | External service adapter |
| `legacy/api/src/inference/linfa_integration.rs` | `infra` | `infra/src/ml/linfa_classifier.rs` | ML adapter (feature gated) |
| `legacy/api/src/http/client.rs` | `infra` | `infra/src/http/client.rs` | HTTP adapter |
| `legacy/api/src/commands/*.rs` | `api` | `api/src/commands/*.rs` | Tauri commands |
| `legacy/api/src/main.rs` | `api` | `api/src/main.rs` | Application entry point |

---

## Stepwise Migration Process

For each module to migrate:

1. **Classify**: Determine target crate based on placement rules
2. **Extract Types**: Move shared types to `domain` first
3. **Define Port**: If in `core`, define trait in `core/src/*/ports.rs`
4. **Implement Adapter**: If in `infra`, implement port trait
5. **Wire in API**: Update `api/src/context.rs` to instantiate and inject
6. **Move Tests**: Migrate tests alongside code, update imports
7. **Add Integration Test**: Add end-to-end test in `api/tests/`
8. **Deprecate Legacy**: Mark original with `#[deprecated]`
9. **Verify CI**: Ensure all checks pass

### Temporary Shims

During migration, use re-exports to avoid breaking all callsites:

```rust
// legacy/api/src/tracker/core.rs
#[deprecated(since = "0.2.0", note = "Use pulsearc_core::tracking::TrackingService")]
pub use pulsearc_core::tracking::TrackingService;
```

Enable via feature flag:
```toml
[features]
legacy = []  # Enable deprecated re-exports
```

### Sunset Plan

- **Week 1-5**: Migration with shims enabled
- **Week 6**: Deprecation warnings in logs/docs
- **Week 7-8**: Remove shims, force breakage (with migration guide)
- **Week 9**: Delete `legacy/` directory

---

## CI/Policy Enforcement

### 1. Dependency Graph Validation

Create `xtask/src/check_deps.rs`:

```rust
use std::collections::HashSet;

const FORBIDDEN_EDGES: &[(&str, &str)] = &[
    ("pulsearc-domain", "pulsearc-core"),
    ("pulsearc-domain", "pulsearc-infra"),
    ("pulsearc-domain", "pulsearc-api"),
    ("pulsearc-core", "pulsearc-infra"),
    ("pulsearc-core", "pulsearc-api"),
    ("pulsearc-infra", "pulsearc-api"),
];

fn main() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .unwrap();
    
    let mut violations = Vec::new();
    
    for pkg in &metadata.packages {
        for dep in &pkg.dependencies {
            let edge = (pkg.name.as_str(), dep.name.as_str());
            if FORBIDDEN_EDGES.contains(&edge) {
                violations.push(format!(
                    "{} depends on {} (FORBIDDEN)",
                    pkg.name, dep.name
                ));
            }
        }
    }
    
    if !violations.is_empty() {
        eprintln!("Dependency violations detected:");
        for v in violations {
            eprintln!("  - {}", v);
        }
        std::process::exit(1);
    }
}
```

Add to CI:
```yaml
- name: Check dependency graph
  run: cargo xtask check-deps
```

### 2. Import Deny Lists

In each crate's `src/lib.rs`:

```rust
// domain/src/lib.rs
#![forbid(unsafe_code)]
#![deny(
    // No async runtime in domain
    clippy::await_holding_lock,
)]

// Ensure we don't accidentally import infra
#[cfg(not(test))]
compile_error_if_exists!("pulsearc_infra");
#[cfg(not(test))]
compile_error_if_exists!("rusqlite");
#[cfg(not(test))]
compile_error_if_exists!("reqwest");
```

### 3. Clippy Policy

```toml
# .cargo/config.toml
[target.'cfg(all())']
rustflags = [
    "-D", "warnings",
    "-D", "clippy::all",
    "-D", "clippy::pedantic",
]
```

CI check:
```yaml
- name: Clippy
  run: cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 4. Unsafe Code Audit

```yaml
- name: Check for unsafe code
  run: |
    if grep -r "unsafe" crates/domain crates/core --include="*.rs" | grep -v "test"; then
      echo "Unsafe code found in domain/core (forbidden)"
      exit 1
    fi
```

---

## Rollback Plan

### Immediate Rollback (Week 1-5)

If critical issues arise during migration:

1. **Feature flag off**: Disable new crates via feature flags
2. **Revert PR**: Git revert specific migration commits
3. **Restore legacy**: Re-enable legacy paths via shims
4. **Rollback window**: ≤ 2 hours to production

### Partial Rollback (Week 6-8)

After deprecation period:

1. **Identify broken areas**: User reports or integration test failures
2. **Re-enable shims**: Temporarily restore deprecated re-exports
3. **Hotfix specific modules**: Fix and redeploy broken areas
4. **Extended timeline**: Give additional 1 week for fixes

### Full Rollback (Unlikely)

If migration fundamentally flawed:

1. **Abandon new structure**: Keep legacy as primary
2. **Archive migration work**: Branch preservation for future attempt
3. **Post-mortem**: Document what went wrong
4. **Timeline**: 1-2 sprints to stabilize legacy

---

## Security/Privacy Impact

### Positive Impacts

1. **Audit Trail**: Centralized observability at infra boundaries enables comprehensive audit logging
2. **Encryption Boundaries**: Clear separation makes it obvious where encryption should be applied
3. **PII Detection**: Shared `privacy` module from `common` can be applied consistently at infra layer
4. **Keychain Integration**: Centralized in `infra/src/key_manager.rs` rather than scattered

### Migration Risks

1. **Temporary Security Gaps**: During migration, some modules may have inconsistent security posture
   - **Mitigation**: Security audit checklist as part of each PR
   
2. **Secret Exposure**: Refactoring may accidentally log secrets
   - **Mitigation**: Pre-commit hooks scanning for common patterns (API keys, tokens)

### Post-Migration Security Improvements

1. **RBAC Integration**: Use `pulsearc_common::security::RBACManager` for authorization
2. **Secret Management**: Migrate all secrets to `pulsearc_common::security::KeychainProvider`
3. **Audit Logging**: Use `pulsearc_common::compliance::GlobalAuditLogger` at API boundaries

---

## Performance Impact

### Expected Improvements

1. **Compile Times**: 
   - Current: Full legacy rebuild on any change (~45s)
   - Target: Incremental builds with layered crates (~8s for typical change)
   
2. **Runtime Performance**:
   - Hexagonal architecture enables caching at port boundaries
   - Feature flags reduce binary size (only compile what's needed)

### Potential Regressions

1. **Dynamic Dispatch Overhead**: `Arc<dyn Trait>` has small vtable cost
   - **Mitigation**: Measure with benchmarks, accept <5% overhead for testability
   
2. **Memory Overhead**: Arc wrappers add pointer indirection
   - **Mitigation**: Profile with heaptrack, optimize hot paths

### Performance Testing

- Criterion benchmarks for core services (before/after)
- Integration test performance comparison
- Memory profiling with jemalloc/heaptrack

---

## Operational Impact

### Deployment Changes

- **Binary Size**: May increase slightly due to multiple crate overhead
- **Startup Time**: No expected change (lazy initialization patterns maintained)
- **Hot Reload**: Better development experience with granular compilation

### Monitoring/Observability

- **Metrics**: Structured `tracing` spans enable better telemetry
- **Dashboards**: Update metrics queries to reflect new module paths
- **Alerting**: No change to alert logic, just metric paths

### Documentation Updates

- Update architecture diagrams (ADR-002)
- Update onboarding guides
- Add migration guide for external contributors
- Update API documentation

---

## Open Questions & Follow-ups

### Questions

1. **ML Feature Flag**: Should `ml` be default-enabled or opt-in? (Impacts binary size)
   - **Recommendation**: Opt-in initially, promote to default after validation
   
2. **Legacy Sunset Timeline**: Is 8 weeks sufficient for deprecation?
   - **Risk**: External consumers may need longer notice
   - **Mitigation**: Survey external users, extend if needed
   
3. **Test Coverage Target**: What's acceptable threshold before removing legacy?
   - **Recommendation**: 80% line coverage in core/domain, 70% in infra

### Follow-up Work

1. **ADR-004**: Document observability patterns (tracing spans, metrics)
2. **ADR-005**: Document testing strategies (mock patterns, integration test structure)
3. **Performance Baseline**: Establish benchmarks for key operations
4. **Security Audit**: Comprehensive review of port boundaries for secret exposure

---

## Consequences

### Positive

1. **Testability**: Can unit test business logic with mock implementations
2. **Maintainability**: Clear boundaries reduce cognitive load
3. **Compile Times**: Incremental builds improve developer experience
4. **Onboarding**: New developers have unambiguous placement rules
5. **Security**: Centralized application of encryption/audit patterns
6. **Flexibility**: Easy to swap implementations (e.g., PostgreSQL instead of SQLite)

### Negative

1. **Migration Effort**: 6-8 weeks of focused work
2. **Boilerplate**: Trait definitions add code volume
3. **Learning Curve**: Team needs to understand hexagonal architecture
4. **Dynamic Dispatch**: Small runtime overhead for `dyn Trait`

### Neutral

1. **Binary Size**: Slight increase from crate overhead
2. **API Surface**: Public APIs become more explicit (pro and con)

---

## Appendix: Glossary

- **Port**: Trait definition in `core` representing an external dependency (e.g., `ActivityProvider`)
- **Adapter**: Concrete implementation in `infra` of a port trait (e.g., `MacOsActivityProvider`)
- **Hexagonal Architecture**: Pattern where business logic depends on abstract ports, not concrete implementations
- **Feature Flag**: Cargo feature controlling compilation of optional code paths
- **Shim**: Temporary re-export to avoid breaking callsites during migration
- **DI Container**: Dependency injection container (`AppContext`) that wires adapters to services

---

## References

- [ADR-002: Production Architecture](./002-production-architecture.md)
- [Common Crates Guide](../../crates/common/docs/COMMON_CRATES_GUIDE.md)
- [CLAUDE.md](../../CLAUDE.md) - Rust standards
- [Hexagonal Architecture (Alistair Cockburn)](https://alistair.cockburn.us/hexagonal-architecture/)

---

**End of ADR-003**
```

---

## B) Migration Work Plan (Checklist)

```markdown
# Migration Work Plan: Legacy → Layered Workspace

**Goal**: Migrate all code from `legacy/api/` into `core`/`domain`/`infra`/`api` with clear architectural boundaries.

**Timeline**: 8 weeks (6 weeks migration + 2 weeks deprecation/cleanup)

**Definition of Done**:
- [ ] All legacy code removed or deprecated
- [ ] CI enforces dependency graph rules
- [ ] 80% test coverage in core/domain
- [ ] Integration tests pass with new structure
- [ ] Performance benchmarks show <5% regression
- [ ] Documentation updated

---

## Week 1: Preparation & Domain Types

### Inventory
- [ ] Audit all `legacy/api/src/` modules and classify by target crate
- [ ] Create migration tracking spreadsheet (module → target crate → status)
- [ ] Identify shared types used across multiple modules

### Domain Migration
- [ ] Move `legacy/api/src/db/models.rs` → `domain/src/types/time_entry.rs`
- [ ] Move `legacy/api/src/db/models_idle.rs` → `domain/src/types/idle.rs`
- [ ] Move `legacy/api/src/shared/types/stats.rs` → `domain/src/types/stats.rs`
- [ ] Move `legacy/api/src/shared/config.rs` → `domain/src/config/app_config.rs`
- [ ] Move `legacy/api/src/inference/types.rs` → `domain/src/types/classification.rs`
- [ ] Move `legacy/api/src/integrations/sap/models.rs` → `domain/src/types/sap.rs`
- [ ] Move `legacy/api/src/integrations/calendar/types.rs` → `domain/src/types/calendar.rs`

### Port Definitions (Core)
- [ ] Define `ActivityProvider` trait in `core/src/tracking/ports.rs`
- [ ] Define `ActivityRepository` trait in `core/src/tracking/ports.rs`
- [ ] Define `TimeEntryRepository` trait in `core/src/classification/ports.rs`
- [ ] Define `Classifier` trait in `core/src/classification/ports.rs`
- [ ] Define `OutboxQueue` trait in `core/src/sync/ports.rs`
- [ ] Define `CalendarProvider` trait in `core/src/integrations/ports.rs`
- [ ] Define `SapClient` trait in `core/src/integrations/ports.rs`

### Testing
- [ ] Verify domain crate has zero infra dependencies (`cargo tree`)
- [ ] Update existing tests for moved types

---

## Week 2: Core Services

### Tracking Service
- [ ] Move `legacy/api/src/tracker/core.rs` → `core/src/tracking/service.rs`
- [ ] Refactor to use `ActivityProvider` port instead of concrete impl
- [ ] Move `legacy/api/src/preprocess/segmenter.rs` → `core/src/tracking/segmenter.rs`
- [ ] Move `legacy/api/src/preprocess/trigger.rs` → `core/src/tracking/trigger.rs`
- [ ] Add unit tests with mock `ActivityProvider`

### Classification Service
- [ ] Move `legacy/api/src/inference/hybrid_classifier.rs` → `core/src/classification/hybrid.rs`
- [ ] Move `legacy/api/src/inference/rules_classifier.rs` → `core/src/classification/rules.rs`
- [ ] Move `legacy/api/src/inference/evidence_extractor.rs` → `core/src/classification/evidence.rs`
- [ ] Move `legacy/api/src/inference/project_matcher.rs` → `core/src/classification/project_matcher.rs`
- [ ] Extract `ClassificationEngine` trait
- [ ] Add unit tests with mock implementations

### Idle Detection Service
- [ ] Move `legacy/api/src/tracker/idle/detector.rs` → `core/src/idle/detector.rs`
- [ ] Move `legacy/api/src/tracker/idle/period_tracker.rs` → `core/src/idle/period_tracker.rs`
- [ ] Move `legacy/api/src/tracker/idle/recovery.rs` → `core/src/idle/recovery.rs`

### Testing
- [ ] Verify core has no concrete DB/HTTP/platform dependencies
- [ ] All core tests pass with mock ports
- [ ] Code coverage ≥80% for core services

---

## Week 3: Database Adapters (Infra)

### Activity & Blocks
- [ ] Move `legacy/api/src/db/activity/snapshots.rs` → `infra/src/database/activity_repository.rs`
- [ ] Implement `ActivityRepository` trait
- [ ] Move `legacy/api/src/db/activity/segments.rs` → `infra/src/database/segment_repository.rs`
- [ ] Move `legacy/api/src/db/blocks/operations.rs` → `infra/src/database/block_repository.rs`

### Time Entries & Outbox
- [ ] Extract time entry operations → `infra/src/database/time_entry_repository.rs`
- [ ] Implement `TimeEntryRepository` trait
- [ ] Move `legacy/api/src/db/outbox/outbox.rs` → `infra/src/database/outbox_repository.rs`
- [ ] Move `legacy/api/src/db/outbox/id_mappings.rs` → `infra/src/database/id_mapping_repository.rs`
- [ ] Implement `OutboxQueue` trait

### Calendar & Migrations
- [ ] Move `legacy/api/src/db/calendar/` → `infra/src/database/calendar_repository.rs`
- [ ] Move `legacy/api/src/db/migrations.rs` → `infra/src/database/migrations.rs`
- [ ] Move `legacy/api/src/db/manager.rs` → `infra/src/database/manager.rs`

### Testing
- [ ] Integration tests with real SQLite (in-memory)
- [ ] Migration tests (schema validation)

---

## Week 4: Platform & Integration Adapters (Infra)

### macOS Platform
- [ ] Move `legacy/api/src/tracker/os_events/macos_ax.rs` → `infra/src/platform/macos/activity_provider.rs`
- [ ] Implement `ActivityProvider` trait
- [ ] Move `legacy/api/src/tracker/os_events/macos.rs` → `infra/src/platform/macos/event_monitor.rs`
- [ ] Move `legacy/api/src/detection/enrichers/` → `infra/src/platform/enrichers/`

### SAP Integration (Feature Gated)
- [ ] Move `legacy/api/src/integrations/sap/client.rs` → `infra/src/integrations/sap/client.rs`
- [ ] Implement `SapClient` trait
- [ ] Move `legacy/api/src/integrations/sap/auth_commands.rs` → `infra/src/integrations/sap/auth.rs`
- [ ] Move `legacy/api/src/integrations/sap/cache.rs` → `infra/src/integrations/sap/cache.rs`
- [ ] Add `sap` feature flag to `Cargo.toml`

### Calendar Integration (Feature Gated)
- [ ] Move `legacy/api/src/integrations/calendar/` → `infra/src/integrations/calendar/`
- [ ] Implement `CalendarProvider` trait
- [ ] Add `calendar` feature flag to `Cargo.toml`

### HTTP & Observability
- [ ] Move `legacy/api/src/http/client.rs` → `infra/src/http/client.rs`
- [ ] Move `legacy/api/src/http/graphql.rs` → `infra/src/http/graphql.rs` (feature gated)
- [ ] Move `legacy/api/src/observability/metrics/` → `infra/src/observability/metrics/`

### ML Adapters (Feature Gated)
- [ ] Move `legacy/api/src/inference/linfa_integration.rs` → `infra/src/ml/linfa_classifier.rs`
- [ ] Move `legacy/api/src/inference/tree_classifier.rs` → `infra/src/ml/tree_classifier.rs`
- [ ] Add `ml` feature flag to `Cargo.toml`

### Testing
- [ ] Unit tests for each adapter (mocked external deps)
- [ ] Feature flag compilation tests (ensure optional features work)

---

## Week 5: API Layer & Wiring

### Commands Migration
- [ ] Move `legacy/api/src/commands/blocks.rs` → `api/src/commands/blocks.rs`
- [ ] Move `legacy/api/src/commands/calendar.rs` → `api/src/commands/calendar.rs`
- [ ] Move `legacy/api/src/commands/database.rs` → `api/src/commands/database.rs`
- [ ] Move `legacy/api/src/commands/idle.rs` → `api/src/commands/idle.rs`
- [ ] Move `legacy/api/src/commands/ml_training.rs` → `api/src/commands/ml_training.rs`
- [ ] Move `legacy/api/src/commands/monitoring.rs` → `api/src/commands/monitoring.rs`
- [ ] Move `legacy/api/src/commands/user_profile.rs` → `api/src/commands/user_profile.rs`

### Context & DI Container
- [ ] Build comprehensive `api/src/context.rs`:
  - [ ] Initialize database pool
  - [ ] Create repository adapters
  - [ ] Create platform providers
  - [ ] Wire services with ports
  - [ ] Feature-gated integration clients
- [ ] Add lifecycle management (startup/shutdown)

### Mapping Layer
- [ ] Create `api/src/mapping/activity.rs` (domain ↔ frontend types)
- [ ] Create `api/src/mapping/classification.rs`
- [ ] Create `api/src/mapping/calendar.rs`
- [ ] Create `api/src/mapping/sap.rs`

### Main Entry Point
- [ ] Update `api/src/main.rs` with new context initialization
- [ ] Add tracing initialization
- [ ] Add instance lock
- [ ] Platform-specific window setup (macOS)

### Testing
- [ ] End-to-end integration tests in `api/tests/`
- [ ] Command tests with mocked context
- [ ] Manual smoke testing

---

## Week 6: Testing & CI

### Test Coverage
- [ ] Achieve 80% line coverage in `core`
- [ ] Achieve 80% line coverage in `domain`
- [ ] Achieve 70% line coverage in `infra`
- [ ] Add integration tests for critical paths

### CI Enforcement
- [ ] Implement `xtask check-deps` (dependency graph validation)
- [ ] Add CI job: `cargo xtask check-deps`
- [ ] Add CI job: Check for unsafe code in domain/core
- [ ] Add CI job: Feature flag compilation matrix
- [ ] Update clippy config with strict lints

### Performance Validation
- [ ] Criterion benchmarks for core services
- [ ] Compare legacy vs migrated performance
- [ ] Memory profiling (heaptrack)
- [ ] Ensure <5% regression

### Documentation
- [ ] Update ADR-002 with migration notes
- [ ] Update architecture diagrams
- [ ] Create migration guide for contributors
- [ ] Update CLAUDE.md with new crate structure

---

## Week 7-8: Deprecation & Cleanup

### Mark Legacy Deprecated
- [ ] Add `#[deprecated]` attributes to all legacy modules
- [ ] Add `legacy` feature flag (default = false) to `Cargo.toml`
- [ ] Update imports to use new paths
- [ ] Add deprecation notices to logs

### Shim Removal Timeline
- [ ] Week 7 Day 1-3: Monitor for issues with deprecation warnings
- [ ] Week 7 Day 4-7: Fix any reported issues
- [ ] Week 8 Day 1-3: Remove shims (force breakage)
- [ ] Week 8 Day 4-7: Delete `legacy/` directory

### Final Verification
- [ ] All tests pass without legacy code
- [ ] CI passes with new structure
- [ ] Performance benchmarks meet targets
- [ ] Documentation complete
- [ ] Security audit complete

---

## Acceptance Criteria

### Functional
- [ ] All Tauri commands work with new structure
- [ ] Activity tracking functions correctly
- [ ] Classification produces same results
- [ ] Calendar/SAP integrations work (if enabled)
- [ ] Idle detection works

### Non-Functional
- [ ] Compile times improved (target: 30-40s → 15-20s for full build)
- [ ] Test execution time <2 minutes
- [ ] Binary size increase <10%
- [ ] Memory usage unchanged (±5%)

### Quality
- [ ] Test coverage: core ≥80%, domain ≥80%, infra ≥70%
- [ ] Zero clippy warnings
- [ ] CI dependency graph check passes
- [ ] Documentation complete and accurate

### Operational
- [ ] Monitoring dashboards updated
- [ ] Deployment process unchanged
- [ ] Rollback plan tested
- [ ] Team trained on new structure

---

## Milestones

1. **Week 1 Complete**: Domain types migrated, ports defined
2. **Week 2 Complete**: Core services migrated and tested
3. **Week 3 Complete**: Database adapters implemented
4. **Week 4 Complete**: Platform/integration adapters implemented
5. **Week 5 Complete**: API layer migrated and wired
6. **Week 6 Complete**: Testing complete, CI enforcing boundaries
7. **Week 7 Complete**: Legacy deprecated, warnings in place
8. **Week 8 Complete**: Legacy deleted, migration complete

---

## Risk Mitigation

- **Blocker identified**: Stop migration, assess rollback vs continuation
- **Test coverage insufficient**: Extend testing phase before continuing
- **Performance regression >5%**: Profile and optimize before proceeding
- **Team velocity slower than expected**: Extend timeline, adjust milestones

---

**Status Tracking**: Update this checklist daily during migration. Mark items complete as PRs merge.
```

---

This comprehensive ADR and work plan provides clear guidance for migrating the legacy codebase into a clean, layered architecture aligned with the principles from your Common Crates Guide and Production Architecture ADR. The plan is specific, actionable, and includes concrete examples, CI enforcement strategies, and risk mitigation approaches.