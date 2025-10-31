# ADR-002: PulseArc Production Architecture (2025)

## Status
**Accepted** (Current Implementation)

**Supersedes:** [ADR-001](./001-architecture-overview.md)

**Last Updated:** October 30, 2025

---

## Context

This ADR supersedes [ADR-001](./001-architecture-overview.md) to reflect the **production-ready architecture** as implemented and battle-tested in 2025. While ADR-001 established the foundational architectural patterns, this document captures:

- **Evolved component organization** with tiered feature system
- **Production-hardened infrastructure** (resilience, observability, security)
- **Comprehensive testing strategies** and quality gates
- **Developer tooling ecosystem** (Makefile, xtask, CI/CD)
- **Platform-specific optimizations** for macOS
- **Operational considerations** (monitoring, deployment, maintenance)

### Why a New ADR?

ADR-001 served as the initial architectural blueprint, but significant evolution has occurred:

1. **Common crate reorganization** → Tiered feature system (foundation/runtime/platform)
2. **Enhanced resilience patterns** → Circuit breakers, retry logic with exponential backoff
3. **Security hardening** → Encrypted storage, RBAC, audit logging, compliance infrastructure
4. **Observability maturity** → Structured metrics, distributed tracing, error classification
5. **Developer experience** → Comprehensive tooling, automated CI, pre-commit hooks
6. **Documentation depth** → API guides, troubleshooting, migration paths

This ADR provides the **definitive architectural reference** for the current production system.

---

## Decision

PulseArc implements a **layered, hexagonal architecture** with the following key architectural patterns:

1. **Tauri 2.0 Desktop Framework** - Cross-language (Rust + TypeScript) desktop development
2. **Hexagonal Architecture (Ports & Adapters)** - Business logic isolation with pluggable implementations
3. **Domain-Driven Design (DDD)** - Pure domain models with ubiquitous language
4. **Feature-Based Frontend** - Vertical slices with high cohesion, low coupling
5. **Tiered Infrastructure** - Opt-in feature system (foundation/runtime/platform)
6. **Zero-Trust Security** - Encryption at rest, secure credential management, RBAC

---

## Architecture Overview

### System Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                   Frontend (React 19 + TypeScript 5.9)          │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Features: Timer, Entries, Settings, Analytics, Timeline  │  │
│  │  (Vertical slices with services, stores, hooks, types)    │  │
│  └───────────────────┬───────────────────────────────────────┘  │
│  ┌───────────────────▼───────────────────────────────────────┐  │
│  │  Shared Infrastructure: IPC Client, State, Events, UI     │  │
│  └───────────────────────────────────────────────────────────┘  │
└───────────────────────┬─────────────────────────────────────────┘
                        │ Tauri IPC (Commands + Events)
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Backend (Rust 1.77)                          │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  API Layer (pulsearc-api): Tauri Commands + DI Container  │  │
│  └─────────────────────┬─────────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────────┐  │
│  │  Core Layer (pulsearc-core): Business Logic + Ports       │  │
│  └─────────────────────┬─────────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────────┐  │
│  │  Infrastructure (pulsearc-infra): Port Implementations    │  │
│  │  • Database (SQLCipher)    • Platform (macOS APIs)        │  │
│  │  • HTTP Clients            • Key Management (Keychain)    │  │
│  └─────────────────────┬─────────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────────┐  │
│  │  Domain Layer (pulsearc-domain): Pure Domain Models       │  │
│  │  • Zero infrastructure dependencies                       │  │
│  │  • Ubiquitous language types                              │  │
│  └─────────────────────┬─────────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────────┐  │
│  │  Common Layer (pulsearc-common): Shared Infrastructure    │  │
│  │  Tiers: Foundation → Runtime → Platform                   │  │
│  │  • Error, Validation, Collections (foundation)            │  │
│  │  • Cache, Resilience, Observability (runtime)             │  │
│  │  • Auth, Security, Storage, Compliance (platform)         │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Backend Architecture (Rust Workspace)

### Cargo Workspace Structure

```
crates/
├── common/       # Tiered shared infrastructure (feature flags)
├── domain/       # Pure domain models (zero infrastructure deps)
├── core/         # Business logic + hexagonal ports
├── infra/        # Infrastructure adapters (database, platform, HTTP)
└── api/          # Tauri application entry point + commands

xtask/            # Development automation (cargo xtask ci/fmt/clippy/test)
```

---

## 1. `pulsearc-common` - Tiered Shared Infrastructure

### Architecture: Feature Tier System

**Design Philosophy:** Opt-in features with no defaults. Pay only for what you use.

```rust
// Foundation tier: Core utilities (no async, no logging)
pulsearc-common = { workspace = true, features = ["foundation"] }

// Runtime tier: Async infrastructure + observability
pulsearc-common = { workspace = true, features = ["runtime"] }

// Platform tier: Full application features (auth, storage, security)
pulsearc-common = { workspace = true, features = ["platform"] }
```

### Foundation Tier (`foundation`)

**Purpose:** Core utilities without side effects
**Dependencies:** Minimal (no tokio, no tracing)
**Use When:** Libraries, minimal environments, embedded contexts

**Key Modules:**

#### Error Handling (`error/`)

```rust
use pulsearc_common::error::{CommonError, CommonResult, ErrorClassification, ErrorSeverity};

// Unified error type with classification
pub enum CommonError {
    Timeout { duration: Duration },
    RateLimitExceeded { retry_after: Option<Duration> },
    CircuitBreakerOpen { name: String },
    Lock { resource: String },
    Serialization { format: String, details: String },
    Validation { field: String, message: String },
    Config { key: String, message: String },
    Storage { operation: String, details: String },
    Backend { service: String, details: String },
    Unauthorized { reason: String },
    NotFound { resource: String },
    Internal { context: String },
}

// Error classification for resilience patterns
impl ErrorClassification for CommonError {
    fn is_retryable(&self) -> bool { /* ... */ }
    fn severity(&self) -> ErrorSeverity { /* ... */ }
    fn is_critical(&self) -> bool { /* ... */ }
    fn retry_after(&self) -> Option<Duration> { /* ... */ }
}

// Standard error severity levels
pub enum ErrorSeverity {
    Info,      // Expected, informational
    Warning,   // Degraded but operational
    Error,     // Failure requiring attention
    Critical,  // System integrity at risk
}
```

**Design Pattern:**
- Module-specific errors **compose** with `CommonError` via `#[from]`
- Standard patterns (timeouts, rate limits) use `CommonError`
- Custom domain logic uses module-specific error types
- All errors implement `ErrorClassification` for resilience patterns

#### Validation Framework (`validation/`)

```rust
use pulsearc_common::validation::{Validator, ValidationError, RuleSet};

// Enterprise-grade validation with field-level errors
let validator = Validator::new()
    .add_rule("email", EmailValidator::default())
    .add_rule("age", RangeValidator::new(0, 150))
    .add_rule("url", UrlValidator::new())
    .add_custom("custom_logic", |value| {
        // Custom validation logic
        Ok(())
    });

// Validate with detailed field-level errors
match validator.validate(&data) {
    Ok(_) => println!("Valid!"),
    Err(ValidationError::FieldErrors(errors)) => {
        for (field, message) in errors {
            println!("{}: {}", field, message);
        }
    }
}

// Rule sets for complex validation
let ruleset = RuleSet::builder()
    .add_string("name", StringValidator::min_length(3))
    .add_range("score", 0.0..=100.0)
    .add_collection("items", CollectionValidator::min_size(1))
    .build();
```

**Pre-built Validators:**
- `StringValidator` - Length, regex, format constraints
- `RangeValidator` - Numeric range validation
- `CollectionValidator` - Size, uniqueness, item validation
- `EmailValidator` - RFC 5322 compliant email validation
- `UrlValidator` - URL format and protocol validation
- `IpValidator` - IPv4/IPv6 validation
- `CustomValidator` - Lambda-based custom logic

#### Collections (`collections/`)

**Specialized data structures:**

```rust
// Bloom filter for membership testing
let bloom = BloomFilter::new(1000, 0.01); // capacity, false positive rate
bloom.insert("key");
assert!(bloom.contains("key"));

// Bounded queue with overflow handling
let queue = BoundedQueue::new(100); // max capacity
queue.push(item)?; // Err if full

// LRU cache (foundation tier - sync only)
let lru = LruCache::new(100);
lru.insert("key", "value");

// Trie for prefix matching
let trie = Trie::new();
trie.insert("apple");
assert!(trie.starts_with("app"));

// Ring buffer for fixed-size circular storage
let ring = RingBuffer::new(10);
ring.push(item); // Overwrites oldest when full
```

### Runtime Tier (`runtime`)

**Purpose:** Async infrastructure with observability
**Dependencies:** Foundation + `tokio` + `tracing` + `moka`
**Use When:** Building async services, background workers, APIs

**Key Modules:**

#### Cache (`cache/`)

```rust
use pulsearc_common::cache::{Cache, AsyncCache, CacheConfig, CacheStats};

// Synchronous cache with TTL and size limits
let cache = Cache::new(CacheConfig::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(300))
    .build());

cache.insert("key", "value");
let value = cache.get("key"); // Option<&str>

// Async cache for concurrent workloads
let async_cache = AsyncCache::new(CacheConfig::default());
async_cache.insert("key", "value").await;
let value = async_cache.get("key").await;

// Cache statistics (hit rate, evictions, size)
let stats = cache.stats();
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
```

**Features:**
- TTL-based eviction (time-to-live)
- Size-based eviction (LRU)
- Thread-safe concurrent access
- Hit/miss/eviction metrics
- Builder pattern configuration

#### Resilience (`resilience/`)

**Circuit Breaker:**

```rust
use pulsearc_common::resilience::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

let breaker = CircuitBreaker::new(CircuitBreakerConfig::builder()
    .failure_threshold(5)           // Open after 5 failures
    .success_threshold(2)           // Close after 2 successes in half-open
    .timeout(Duration::from_secs(60)) // Stay open for 60s
    .build());

// Call with automatic failure tracking
let result = breaker.call(|| async {
    external_api_call().await
}).await;

// Check breaker state
match breaker.state() {
    CircuitState::Closed => println!("Normal operation"),
    CircuitState::Open => println!("Too many failures, rejecting calls"),
    CircuitState::HalfOpen => println!("Testing if service recovered"),
}

// Metrics
let metrics = breaker.metrics();
println!("Failure rate: {:.2}%", metrics.failure_rate() * 100.0);
```

**Retry Logic:**

```rust
use pulsearc_common::resilience::{
    RetryExecutor, RetryPolicy, BackoffStrategy, Jitter, retry
};

// Simple retry with defaults
let result = retry(|| async {
    fallible_operation().await
}).await?;

// Advanced retry with custom policy
let executor = RetryExecutor::builder()
    .max_attempts(5)
    .backoff(BackoffStrategy::Exponential {
        initial: Duration::from_millis(100),
        max: Duration::from_secs(10),
        multiplier: 2.0,
    })
    .jitter(Jitter::Full) // Full jitter to prevent thundering herd
    .build();

let result = executor.execute(|| async {
    network_call().await
}).await?;

// Custom retry policy
let policy = RetryPolicy::new(|error: &MyError| {
    error.is_retryable() && error.severity() != ErrorSeverity::Critical
});

let result = executor.execute_with_policy(|| async {
    custom_operation().await
}, policy).await?;
```

**Backoff Strategies:**
- `Exponential` - 100ms, 200ms, 400ms, 800ms... (with max cap)
- `Linear` - 100ms, 200ms, 300ms, 400ms...
- `Constant` - 100ms, 100ms, 100ms, 100ms...

**Jitter Options:**
- `None` - No randomization
- `Full` - Random between 0 and computed delay
- `Decorrelated` - Decorrelated jitter (AWS recommendation)

#### Time (`time/`)

```rust
use pulsearc_common::time::{format_duration, Interval, Timer, CronSchedule};

// Duration formatting
let duration = Duration::from_secs(3661);
assert_eq!(format_duration(duration), "1h 1m 1s");

// Interval for periodic tasks
let mut interval = Interval::new(Duration::from_secs(60));
loop {
    interval.tick().await;
    perform_task().await;
}

// Timer for one-shot delays
let timer = Timer::after(Duration::from_secs(30));
timer.await;

// Cron scheduling (for background jobs)
let schedule = CronSchedule::parse("0 0 * * *")?; // Daily at midnight
let next_run = schedule.next_occurrence();
```

#### Lifecycle (`lifecycle/`)

```rust
use pulsearc_common::lifecycle::{AsyncManager, ManagerStatus, ManagerHealth};

#[async_trait]
impl AsyncManager for MyService {
    async fn start(&mut self) -> Result<()> {
        // Initialize resources
        self.db_pool = create_pool().await?;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // Graceful shutdown
        self.db_pool.close().await?;
        Ok(())
    }

    async fn health_check(&self) -> ManagerHealth {
        // Health status for monitoring
        ManagerHealth::Healthy
    }

    fn status(&self) -> ManagerStatus {
        // Current operational status
        ManagerStatus::Running
    }
}

// Lifecycle management
let mut service = MyService::new();
service.start().await?;

// Graceful shutdown
service.stop().await?;
```

#### Observability (`observability/`)

```rust
use pulsearc_common::observability::{
    AppError, ErrorCode, ActionHint, MetricsTracker
};

// Unified application errors
let error = AppError::ai_api_key_missing("OPENAI_API_KEY");
println!("Error code: {}", error.code()); // "AI_API_KEY_MISSING"
println!("Hint: {}", error.hint()); // "Set OPENAI_API_KEY environment variable"

// Metrics tracking
let tracker = MetricsTracker::new();
tracker.track_operation_start("classification");
// ... perform operation
tracker.track_operation_end("classification", success);

// Classification metrics
tracker.track_classification_coverage(0.85); // 85% coverage
let metrics = tracker.snapshot();
```

### Platform Tier (`platform`)

**Purpose:** Platform integrations (keychain, OAuth, storage, compliance)
**Dependencies:** Runtime + platform-specific libs (keyring, rusqlite, oauth2)
**Use When:** Building full applications with auth, storage, security

**Key Modules:**

#### Authentication (`auth/`)

**OAuth 2.0 + PKCE:**

```rust
use pulsearc_common::auth::{
    OAuthService, OAuthConfig, TokenManager, PKCEChallenge
};

// OAuth configuration
let config = OAuthConfig {
    client_id: "your-client-id".to_string(),
    client_secret: Some("your-secret".to_string()),
    authorization_endpoint: "https://auth.example.com/authorize".parse()?,
    token_endpoint: "https://auth.example.com/token".parse()?,
    redirect_uri: "http://localhost:8080/callback".parse()?,
    scopes: vec!["openid".to_string(), "profile".to_string()],
};

// OAuth service with automatic token refresh
let service = OAuthService::new(config).await?;

// Generate authorization URL with PKCE
let (auth_url, state) = service.authorization_url().await?;
println!("Visit: {}", auth_url);

// Exchange authorization code for tokens
let tokens = service.exchange_code(&code, &state).await?;

// Automatic token refresh (background task)
let token_manager = service.token_manager();
token_manager.enable_auto_refresh(Duration::from_secs(60)).await;

// Get current valid token (auto-refreshed if needed)
let access_token = token_manager.get_valid_token().await?;
```

**Features:**
- **PKCE (RFC 7636)** - Proof Key for Code Exchange for public clients
- **State validation** - CSRF protection with constant-time comparison
- **Automatic token refresh** - Background refresh before expiration
- **Keychain storage** - Secure token storage in platform keychain
- **OpenID Connect** - ID token and user claims support
- **Multi-provider** - Auth0, Google, Microsoft, custom OAuth servers

#### Security (`security/`)

**RBAC (Role-Based Access Control):**

```rust
use pulsearc_common::security::{
    RBACManager, Role, Permission, RBACPolicy, PolicyCondition
};

let mut rbac = RBACManager::new();

// Define roles and permissions
let admin = Role::new("admin")
    .with_permission(Permission::new("users:read"))
    .with_permission(Permission::new("users:write"))
    .with_permission(Permission::new("users:delete"));

let viewer = Role::new("viewer")
    .with_permission(Permission::new("users:read"));

rbac.add_role(admin);
rbac.add_role(viewer);

// Check permissions
assert!(rbac.has_permission("admin", "users:delete").await);
assert!(!rbac.has_permission("viewer", "users:delete").await);

// Dynamic policies with conditions
let policy = RBACPolicy::new("business_hours_only")
    .with_condition(PolicyCondition::Time {
        start: "09:00".parse()?,
        end: "17:00".parse()?,
    })
    .with_condition(PolicyCondition::IpRange {
        allowed: vec!["10.0.0.0/8".parse()?],
    });

rbac.add_policy(policy);
```

**Keychain Integration:**

```rust
use pulsearc_common::security::keychain::KeychainProvider;

let keychain = KeychainProvider::new("com.pulsearc.app");

// Store credentials securely
keychain.store("database_key", "super-secret-key").await?;

// Retrieve credentials
let key = keychain.retrieve("database_key").await?;

// Delete credentials
keychain.delete("database_key").await?;
```

#### Storage (`storage/`)

**Encrypted Database (SQLCipher):**

```rust
use pulsearc_common::storage::{
    SqlCipherPool, SqlCipherPoolConfig, StorageKeyManager
};

// Key management with automatic rotation
let key_manager = StorageKeyManager::new("com.pulsearc.db-keys");
let encryption_key = key_manager.get_or_create_key("main").await?;

// Connection pool configuration
let pool = SqlCipherPool::new(SqlCipherPoolConfig::builder()
    .database_path("pulsearc.db")
    .encryption_key(encryption_key)
    .pool_size(10)
    .connection_timeout(Duration::from_secs(30))
    .pragma("journal_mode", "WAL")
    .pragma("synchronous", "NORMAL")
    .build()).await?;

// Get connection from pool
let conn = pool.get().await?;

// Execute queries (fully encrypted at rest)
conn.execute("INSERT INTO users (name) VALUES (?1)", params!["Alice"])?;

// Key rotation (automatic, scheduled)
key_manager.rotate_key("main", &pool).await?;
```

**Features:**
- **AES-256-GCM encryption** at rest
- **Keychain-backed key storage** (platform-secure)
- **Automatic key rotation** with configurable schedules
- **Key caching** for performance
- **Connection pooling** (r2d2)
- **Pragma management** for SQLite optimization

#### Compliance (`compliance/`)

**Audit Logging:**

```rust
use pulsearc_common::compliance::{GlobalAuditLogger, AuditEvent, AuditSeverity};

let logger = GlobalAuditLogger::new();

// Log audit events
logger.log(AuditEvent {
    timestamp: Utc::now(),
    user_id: Some("user123".to_string()),
    action: "users:delete".to_string(),
    resource: "user456".to_string(),
    severity: AuditSeverity::Critical,
    context: serde_json::json!({
        "ip_address": "203.0.113.1",
        "user_agent": "PulseArc/1.0",
    }),
}).await?;

// Query audit logs
let events = logger.query()
    .user_id("user123")
    .action("users:delete")
    .since(yesterday)
    .execute().await?;
```

**Feature Flags:**

```rust
use pulsearc_common::compliance::FeatureFlagManager;

let flags = FeatureFlagManager::new();

// Define feature flags
flags.register("ai_suggestions", false).await; // Default off
flags.register("dark_mode", true).await;       // Default on

// Check feature flag
if flags.is_enabled("ai_suggestions").await {
    show_ai_suggestions();
}

// Dynamic enable/disable (no deployment required)
flags.set("ai_suggestions", true).await;
```

### Testing Utilities (`test-utils` feature)

```rust
use pulsearc_common::testing::{
    MockClock, TestBuilder, MatcherExt, TempFile, fixture
};

#[tokio::test]
async fn test_with_utilities() {
    // Mock clock for time-based tests
    let clock = MockClock::new();
    clock.set_time(Utc::now());
    clock.advance(Duration::from_secs(60));

    // Test builders
    let user = TestBuilder::user()
        .with_id("user123")
        .with_email("test@example.com")
        .build();

    // Temporary files
    let temp = TempFile::new("test.db")?;
    // ... use temp file, automatically cleaned up on drop

    // Fixtures
    let sample_data = fixture("users.json")?;
}
```

---

## 2. `pulsearc-domain` - Pure Domain Layer

### Design Philosophy

**Zero Infrastructure Dependencies**

```toml
[dependencies]
# Only foundational crates - NO infrastructure
serde = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
```

### Core Domain Types

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Core activity representation - what the user is doing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityContext {
    pub app_name: String,
    pub window_title: Option<String>,
    pub url: Option<String>,
    pub document_path: Option<String>,
    pub captured_at: DateTime<Utc>,
}

/// Timestamped activity snapshot - point-in-time capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub id: Uuid,
    pub context: ActivityContext,
    pub timestamp: DateTime<Utc>,
    pub idle_since: Option<DateTime<Utc>>,
}

/// Classified work period - user-categorized time entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub wbs_code: Option<String>,      // SAP Work Breakdown Structure
    pub is_billable: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub sync: SyncConfig,
    pub tracking: TrackingConfig,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub pool_size: u32,
    pub encryption_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub capture_interval: Duration,
    pub idle_threshold: Duration,
    pub privacy_mode: bool,
}
```

### Domain Errors

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PulseArcError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Internal error: {context}")]
    Internal { context: String },
}

pub type Result<T> = std::result::Result<T, PulseArcError>;
```

**Philosophy:**
- Domain types are **pure data structures**
- No dependencies on other PulseArc crates
- Can be understood in **complete isolation**
- Forms the **ubiquitous language** of the system
- Only foundational external crates (serde, chrono, uuid, thiserror)

---

## 3. `pulsearc-core` - Business Logic Layer

### Hexagonal Architecture (Ports & Adapters)

**Core Principle:** Business logic defines **ports (traits)**, infrastructure provides **adapters (implementations)**

### Tracking Module

**Port Definitions:**

```rust
use async_trait::async_trait;
use pulsearc_domain::{ActivitySnapshot, ActivityContext, Result};

#[async_trait]
pub trait ActivityProvider: Send + Sync {
    /// Capture current activity from OS
    async fn capture_activity(&self) -> Result<ActivitySnapshot>;

    /// Pause activity tracking
    async fn pause(&self) -> Result<()>;

    /// Resume activity tracking
    async fn resume(&self) -> Result<()>;

    /// Check if tracking is paused
    fn is_paused(&self) -> bool;
}

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    /// Save activity snapshot
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()>;

    /// Find activities by time range
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySnapshot>>;

    /// Get most recent activity
    async fn get_latest(&self) -> Result<Option<ActivitySnapshot>>;
}

#[async_trait]
pub trait ActivityEnricher: Send + Sync {
    /// Enrich activity context with additional data
    async fn enrich(&self, context: ActivityContext) -> Result<ActivityContext>;
}
```

**Service Implementation:**

```rust
pub struct TrackingService {
    provider: Arc<dyn ActivityProvider>,
    repository: Arc<dyn ActivityRepository>,
    enrichers: Vec<Arc<dyn ActivityEnricher>>,
}

impl TrackingService {
    pub fn new(
        provider: Arc<dyn ActivityProvider>,
        repository: Arc<dyn ActivityRepository>,
        enrichers: Vec<Arc<dyn ActivityEnricher>>,
    ) -> Self {
        Self {
            provider,
            repository,
            enrichers,
        }
    }

    /// Capture current activity and save to repository
    pub async fn capture_and_save(&self) -> Result<ActivitySnapshot> {
        // 1. Capture from OS
        let mut snapshot = self.provider.capture_activity().await?;

        // 2. Enrich with additional context
        for enricher in &self.enrichers {
            snapshot.context = enricher.enrich(snapshot.context).await?;
        }

        // 3. Persist to repository
        self.repository.save(&snapshot).await?;

        Ok(snapshot)
    }

    /// Get activity timeline for date range
    pub async fn get_timeline(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySnapshot>> {
        self.repository.find_by_time_range(start, end).await
    }
}
```

### Classification Module

**Port Definitions:**

```rust
#[async_trait]
pub trait Classifier: Send + Sync {
    /// Classify activity snapshots into time entries
    async fn classify(
        &self,
        snapshots: Vec<ActivitySnapshot>,
    ) -> Result<Vec<TimeEntry>>;
}

#[async_trait]
pub trait TimeEntryRepository: Send + Sync {
    /// Save time entry
    async fn save(&self, entry: &TimeEntry) -> Result<()>;

    /// Find entries by date
    async fn find_by_date(&self, date: NaiveDate) -> Result<Vec<TimeEntry>>;

    /// Update existing entry
    async fn update(&self, entry: &TimeEntry) -> Result<()>;

    /// Delete entry by ID
    async fn delete(&self, id: Uuid) -> Result<()>;
}
```

**Service Implementation:**

```rust
pub struct ClassificationService {
    classifier: Arc<dyn Classifier>,
    repository: Arc<dyn TimeEntryRepository>,
}

impl ClassificationService {
    pub async fn classify_activities(
        &self,
        snapshots: Vec<ActivitySnapshot>,
    ) -> Result<Vec<TimeEntry>> {
        // 1. Classify snapshots into entries
        let entries = self.classifier.classify(snapshots).await?;

        // 2. Persist entries
        for entry in &entries {
            self.repository.save(entry).await?;
        }

        Ok(entries)
    }

    pub async fn get_day_entries(&self, date: NaiveDate) -> Result<Vec<TimeEntry>> {
        self.repository.find_by_date(date).await
    }
}
```

**Benefits of Hexagonal Architecture:**

| Benefit | Description |
|---------|-------------|
| **Testability** | Mock implementations for unit tests |
| **Flexibility** | Swap implementations without changing business logic |
| **Platform Independence** | Core logic has zero platform-specific code |
| **Clear Boundaries** | Traits define explicit contracts |
| **Parallel Development** | Teams can work on ports/adapters independently |

---

## 4. `pulsearc-infra` - Infrastructure Layer

### Purpose: Concrete Implementations of Core Ports

### Database Module (`database/`)

**SQLCipher Integration:**

```rust
use pulsearc_common::storage::{SqlCipherPool, SqlCipherPoolConfig};
use r2d2::{Pool, PooledConnection};
use rusqlite::Connection;

pub struct DbManager {
    pool: SqlCipherPool,
}

impl DbManager {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let encryption_key = Self::get_or_create_encryption_key()?;

        let pool = SqlCipherPool::new(SqlCipherPoolConfig::builder()
            .database_path(&config.path)
            .encryption_key(encryption_key)
            .pool_size(config.pool_size)
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("cache_size", "-64000") // 64MB cache
            .pragma("temp_store", "MEMORY")
            .build()).await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        let conn = self.pool.get().await?;

        // Embedded SQL migrations
        conn.execute_batch(include_str!("../migrations/001_initial_schema.sql"))?;
        conn.execute_batch(include_str!("../migrations/002_add_time_entries.sql"))?;
        // ... more migrations

        Ok(())
    }
}
```

**Repository Implementation:**

```rust
pub struct SqliteActivityRepository {
    db: Arc<DbManager>,
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()> {
        let conn = self.db.pool.get().await?;

        conn.execute(
            "INSERT INTO activity_snapshots (
                id, app_name, window_title, url, document_path,
                timestamp, idle_since
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                snapshot.id.to_string(),
                snapshot.context.app_name,
                snapshot.context.window_title,
                snapshot.context.url,
                snapshot.context.document_path,
                snapshot.timestamp.to_rfc3339(),
                snapshot.idle_since.map(|t| t.to_rfc3339()),
            ],
        )?;

        Ok(())
    }

    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySnapshot>> {
        let conn = self.db.pool.get().await?;

        let mut stmt = conn.prepare(
            "SELECT id, app_name, window_title, url, document_path,
                    timestamp, idle_since
             FROM activity_snapshots
             WHERE timestamp BETWEEN ?1 AND ?2
             ORDER BY timestamp ASC"
        )?;

        let rows = stmt.query_map(
            params![start.to_rfc3339(), end.to_rfc3339()],
            |row| {
                Ok(ActivitySnapshot {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    context: ActivityContext {
                        app_name: row.get(1)?,
                        window_title: row.get(2)?,
                        url: row.get(3)?,
                        document_path: row.get(4)?,
                        captured_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                            .unwrap()
                            .with_timezone(&Utc),
                    },
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    idle_since: row.get::<_, Option<String>>(6)?
                        .map(|s| DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)),
                })
            },
        )?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| PulseArcError::Database(e.to_string()))
    }
}
```

### Platform Module (`platform/`)

**macOS Activity Provider:**

```rust
use objc2::runtime::NSObject;
use objc2_foundation::NSString;
use objc2_app_kit::NSWorkspace;
use core_foundation::{runloop::CFRunLoop, string::CFString};

pub struct MacOsActivityProvider {
    paused: AtomicBool,
}

#[async_trait]
impl ActivityProvider for MacOsActivityProvider {
    async fn capture_activity(&self) -> Result<ActivitySnapshot> {
        if self.is_paused() {
            return Err(PulseArcError::Platform("Tracking paused".into()));
        }

        // Get frontmost application
        let workspace = unsafe { NSWorkspace::sharedWorkspace() };
        let active_app = unsafe { workspace.frontmostApplication() };

        let app_name = unsafe {
            active_app.localizedName()
                .as_ptr()
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        };

        // Capture window title via Accessibility API
        let window_title = self.capture_window_title(&app_name)?;

        // Capture browser URL (Chrome, Safari, Firefox)
        let url = self.capture_browser_url(&app_name)?;

        // Detect idle time (system-level)
        let idle_since = self.detect_idle_time()?;

        Ok(ActivitySnapshot {
            id: Uuid::new_v7(),
            context: ActivityContext {
                app_name,
                window_title,
                url,
                document_path: None,
                captured_at: Utc::now(),
            },
            timestamp: Utc::now(),
            idle_since,
        })
    }

    async fn pause(&self) -> Result<()> {
        self.paused.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn resume(&self) -> Result<()> {
        self.paused.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }
}

impl MacOsActivityProvider {
    fn capture_window_title(&self, app_name: &str) -> Result<Option<String>> {
        // Accessibility API (requires permissions)
        // Implementation details...
        Ok(None)
    }

    fn capture_browser_url(&self, app_name: &str) -> Result<Option<String>> {
        // AppleScript execution for browser URL extraction
        match app_name {
            "Google Chrome" | "Brave Browser" => {
                self.run_applescript(r#"
                    tell application "Google Chrome"
                        get URL of active tab of front window
                    end tell
                "#)
            }
            "Safari" => {
                self.run_applescript(r#"
                    tell application "Safari"
                        get URL of front document
                    end tell
                "#)
            }
            _ => Ok(None),
        }
    }

    fn detect_idle_time(&self) -> Result<Option<DateTime<Utc>>> {
        // Use IOKit to get system idle time
        // Implementation details...
        Ok(None)
    }
}
```

**Platform-Specific Dependencies:**

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = { workspace = true }
objc2-foundation = { workspace = true }
objc2-app-kit = { workspace = true }
cocoa = "0.25"
core-foundation = "0.10"
core-graphics = "0.24"
io-kit-sys = "0.4"
```

### Key Manager (`key_manager.rs`)

```rust
use keyring::Entry;
use pulsearc_common::security::keychain::KeychainProvider;

pub struct KeyManager {
    keychain: KeychainProvider,
}

impl KeyManager {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            keychain: KeychainProvider::new(service_name),
        }
    }

    pub async fn store_key(&self, key_name: &str, value: &str) -> Result<()> {
        self.keychain.store(key_name, value).await
    }

    pub async fn retrieve_key(&self, key_name: &str) -> Result<String> {
        self.keychain.retrieve(key_name).await
    }

    pub async fn delete_key(&self, key_name: &str) -> Result<()> {
        self.keychain.delete(key_name).await
    }
}
```

### Instance Lock (`instance_lock.rs`)

```rust
use std::fs;
use std::io::Write;

pub struct InstanceLock {
    lock_file: PathBuf,
}

impl InstanceLock {
    pub fn acquire(app_name: &str) -> Result<Self> {
        let lock_file = std::env::temp_dir().join(format!("{}.lock", app_name));

        if lock_file.exists() {
            let pid_str = fs::read_to_string(&lock_file)?;
            let pid: i32 = pid_str.trim().parse()
                .map_err(|e| PulseArcError::Internal(format!("Invalid PID: {}", e)))?;

            if Self::is_process_running(pid) {
                return Err(PulseArcError::Internal(
                    format!("Another instance is running (PID: {})", pid)
                ));
            }

            fs::remove_file(&lock_file)?;
        }

        let mut file = fs::File::create(&lock_file)?;
        writeln!(file, "{}", std::process::id())?;

        Ok(Self { lock_file })
    }

    fn is_process_running(pid: i32) -> bool {
        #[cfg(target_os = "macos")]
        {
            use libc::{kill, ESRCH};
            unsafe {
                kill(pid, 0) == 0 || *libc::__error() != ESRCH
            }
        }

        #[cfg(not(target_os = "macos"))]
        false
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_file);
    }
}
```

---

## 5. `pulsearc` (API / Application Layer)

### Purpose: Tauri Application Entry Point + Commands

**Main Entry Point (`main.rs`):**

```rust
use tauri::{Manager, State};
use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

mod commands;
mod context;

use context::AppContext;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize application context (DI container)
    let app_context = Arc::new(AppContext::new().await?);

    tauri::Builder::default()
        .manage(app_context)
        .setup(|app| {
            #[cfg(target_os = "macos")]
            apply_macos_window_effects(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::tracking::get_activity,
            commands::tracking::pause_tracker,
            commands::tracking::resume_tracker,
            commands::tracking::get_timeline,
            commands::projects::get_user_projects,
            commands::projects::create_project,
            commands::entries::get_day_entries,
            commands::entries::save_entry,
            commands::entries::update_entry,
            commands::entries::delete_entry,
            commands::suggestions::get_suggestions,
            commands::calendar::get_calendar_events,
            commands::sync::get_outbox_status,
            commands::analytics::get_productivity_metrics,
            commands::settings::get_settings,
            commands::settings::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_macos_window_effects(app: &mut tauri::App) -> Result<()> {
    use cocoa::appkit::{NSWindow, NSWindowStyleMask};

    let window = app.get_window("main").unwrap();
    let ns_window = window.ns_window().unwrap() as cocoa::base::id;

    unsafe {
        // Native blur/vibrancy effects
        ns_window.setTitlebarAppearsTransparent_(cocoa::base::YES);
        ns_window.setStyleMask_(
            NSWindowStyleMask::NSFullSizeContentViewWindowMask
        );
    }

    Ok(())
}
```

**Dependency Injection Container (`context/mod.rs`):**

```rust
use pulsearc_common::cache::{Cache, CacheConfig};
use pulsearc_infra::database::DbManager;
use pulsearc_infra::platform::MacOsActivityProvider;
use pulsearc_infra::database::{SqliteActivityRepository, SqliteTimeEntryRepository};
use pulsearc_core::tracking::TrackingService;
use pulsearc_core::classification::ClassificationService;

pub struct AppContext {
    pub config: AppConfig,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub classification_service: Arc<ClassificationService>,
    pub cache: Arc<Cache<String, serde_json::Value>>,
    _instance_lock: InstanceLock,
}

impl AppContext {
    pub async fn new() -> Result<Self> {
        // Load configuration
        let config = AppConfig::load_from_env()?;

        // Acquire instance lock
        let instance_lock = InstanceLock::acquire("pulsearc")?;

        // Initialize database
        let db = Arc::new(DbManager::new(&config.database).await?);
        db.run_migrations().await?;

        // Initialize cache
        let cache = Arc::new(Cache::new(CacheConfig::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build()));

        // Wire up tracking service (hexagonal architecture)
        let activity_provider = Arc::new(MacOsActivityProvider::new());
        let activity_repository = Arc::new(
            SqliteActivityRepository::new(Arc::clone(&db))
        );

        let tracking_service = Arc::new(TrackingService::new(
            activity_provider,
            activity_repository,
            vec![], // enrichers
        ));

        // Wire up classification service
        let time_entry_repository = Arc::new(
            SqliteTimeEntryRepository::new(Arc::clone(&db))
        );

        let classification_service = Arc::new(ClassificationService::new(
            // Classifier implementation
            time_entry_repository,
        ));

        Ok(Self {
            config,
            db,
            tracking_service,
            classification_service,
            cache,
            _instance_lock: instance_lock,
        })
    }
}
```

**Tauri Commands (`commands/tracking.rs`):**

```rust
use tauri::State;
use pulsearc_domain::{ActivitySnapshot, Result};

#[tauri::command]
pub async fn get_activity(
    ctx: State<'_, Arc<AppContext>>
) -> Result<ActivitySnapshot> {
    ctx.tracking_service.capture_and_save().await
}

#[tauri::command]
pub async fn pause_tracker(
    ctx: State<'_, Arc<AppContext>>
) -> Result<()> {
    ctx.tracking_service.pause().await
}

#[tauri::command]
pub async fn resume_tracker(
    ctx: State<'_, Arc<AppContext>>
) -> Result<()> {
    ctx.tracking_service.resume().await
}

#[tauri::command]
pub async fn get_timeline(
    ctx: State<'_, Arc<AppContext>>,
    start: String,
    end: String,
) -> Result<Vec<ActivitySnapshot>> {
    let start = DateTime::parse_from_rfc3339(&start)?.with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339(&end)?.with_timezone(&Utc);

    ctx.tracking_service.get_timeline(start, end).await
}
```

---

## Frontend Architecture (React + TypeScript)

### Structure

```
frontend/
├── App.tsx                    # Main entry, view routing
├── main.tsx                   # React bootstrap
├── globals.css                # TailwindCSS globals
├── components/                # Shared UI components
│   └── ui/                   # shadcn/ui components (40+)
├── features/                  # Feature modules (vertical slices)
│   ├── timer/                # Main timer widget
│   ├── time-entry/           # Time entry management
│   ├── timeline/             # Visual activity timeline
│   ├── analytics/            # Productivity analytics
│   ├── settings/             # Application settings
│   ├── project/              # Project management
│   ├── activity-tracker/     # AI activity suggestions
│   ├── build-my-day/         # Day planning
│   └── idle-detection/       # Idle time tracking
└── shared/                    # Shared infrastructure
    ├── components/           # Reusable UI components
    ├── services/             # Core services (IPC, audio, cache)
    ├── state/                # State derivation
    ├── events/               # Event bus
    ├── hooks/                # Custom hooks
    ├── types/                # Shared types
    ├── utils/                # Utilities
    └── test/                 # Test utilities
```

### Feature-Based Organization

**Each feature is a vertical slice:**

```
feature/
├── components/        # React components
├── services/         # Backend integration (Tauri commands)
├── stores/           # State management (Zustand)
├── hooks/            # Custom React hooks
├── types/            # TypeScript type definitions
└── index.ts          # Barrel export (public API)
```

**Benefits:**
- **High cohesion** - Related code stays together
- **Low coupling** - Features are independent
- **Scalability** - Add features without touching existing code
- **Team autonomy** - Teams can own entire features
- **Easier testing** - Test entire feature in isolation

### Shared Infrastructure

#### IPC Client (`shared/services/ipc/`)

```typescript
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

export class TauriAPI {
    private static window = getCurrentWindow();

    static async setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
        await this.window.setAlwaysOnTop(alwaysOnTop);
    }

    static async setSize(width: number, height: number): Promise<void> {
        await this.window.setSize({ width, height });
    }

    static async center(): Promise<void> {
        await this.window.center();
    }

    static async invokeCommand<T>(
        command: string,
        args?: Record<string, unknown>
    ): Promise<T> {
        return invoke<T>(command, args);
    }
}

// Usage in services
export async function getActivity(): Promise<ActivitySnapshot> {
    return TauriAPI.invokeCommand('get_activity');
}
```

#### State Management (Zustand)

```typescript
import { create } from 'zustand';

interface ProjectStore {
    projects: Project[];
    currentProject: Project | null;
    setProjects: (projects: Project[]) => void;
    setCurrentProject: (project: Project | null) => void;
}

export const useProjectStore = create<ProjectStore>((set) => ({
    projects: [],
    currentProject: null,
    setProjects: (projects) => set({ projects }),
    setCurrentProject: (project) => set({ currentProject: project }),
}));
```

#### Event System (`shared/events/`)

```typescript
export class TimerEvents {
    private static listeners = new Map<string, Set<Function>>();

    static on(event: string, callback: Function): () => void {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, new Set());
        }
        this.listeners.get(event)!.add(callback);

        return () => {
            this.listeners.get(event)?.delete(callback);
        };
    }

    static emit(event: string, data?: unknown): void {
        this.listeners.get(event)?.forEach((callback) => {
            callback(data);
        });
    }
}
```

---

## Technology Stack

### Backend (Rust)

| Category | Technologies | Version |
|----------|-------------|---------|
| **Framework** | Tauri | 2.9 |
| **Runtime** | Tokio (multi-thread) | 1.x |
| **Language** | Rust | 1.77 (stable) |
| **Database** | rusqlite + SQLCipher | 0.37 |
| **Connection Pool** | r2d2 | 0.8 |
| **Security** | blake3, keyring, aes-gcm | 1.5, 3.6, 0.10 |
| **HTTP/OAuth** | reqwest (rustls), oauth2, axum | 0.12, 5.0, 0.8 |
| **Observability** | tracing, metrics, prometheus | 0.1, 0.23, 0.14 |
| **Serialization** | serde, serde_json | 1.0 |
| **Time** | chrono, chrono-tz | 0.4, 0.10 |
| **Caching** | moka (TTL + LRU) | 0.12 |
| **Error Handling** | thiserror, anyhow | 2.0, 1.0 |

### Frontend (React/TypeScript)

| Category | Technologies | Version |
|----------|-------------|---------|
| **Framework** | React | 19.2 |
| **Language** | TypeScript | 5.9 |
| **Build Tool** | Vite | 7.1 |
| **UI Components** | Radix UI, shadcn/ui | Various |
| **Styling** | TailwindCSS | 4.1 |
| **Animation** | Framer Motion | 12.x |
| **Icons** | Lucide React | 0.548 |
| **State** | Zustand, React Hook Form | 5.0, 7.65 |
| **Charts** | Recharts | 3.3 |
| **Notifications** | sonner | 2.0 |
| **Testing** | Vitest, Testing Library | 4.0, 16.3 |

### Development Tools

| Category | Tools |
|----------|-------|
| **Rust** | cargo-audit, cargo-deny, cargo +nightly fmt |
| **Frontend** | pnpm, Prettier, ESLint |
| **Automation** | Makefile (45+ commands), xtask (Rust CLI) |
| **CI/CD** | GitHub Actions (macOS self-hosted) |
| **Version Control** | Git, Conventional Commits |

---

## Design Principles & Standards

### 1. Rust Standards (CLAUDE.md)

**Toolchain:**
- Rust **1.77** (stable, pinned)
- **Nightly rustfmt** for formatting
- **No unsafe code** (`#![forbid(unsafe_code)]`)

**Logging:**
- **`tracing` exclusively** (no `println!`, no `log` macros)
- **Structured logging** with fields
- **JSON output** in production

**Error Handling:**
- `thiserror` in libraries
- `anyhow` at application boundaries
- **NO `unwrap()`, `expect()`, `panic!()`** (except tests)
- Explicit error propagation

**Async:**
- Tokio multi-thread runtime
- No blocking in async contexts
- `spawn_blocking` for CPU-heavy work
- Timeouts and cancellation for external calls

**Lints:**
- `cargo clippy -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery`
- `unsafe_code = "deny"`

### 2. CI Pipeline

**Required Checks:**
1. `cargo +nightly fmt --all -- --check`
2. `cargo clippy --workspace --exclude xtask --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-features`
4. `cargo deny check`
5. `cargo audit`

**Quick Local Check:**
```bash
cargo ci  # or: cargo xtask ci
```

### 3. Git Hygiene

**Commits:**
- Conventional Commits (`feat:`, `fix:`, `perf:`, `refactor:`)
- Clear, descriptive messages

**PRs:**
- Small, focused changes
- Risk assessment + rollback plan
- "How I tested this" section

---

## Consequences

### Positive

1. **Production-Grade Reliability**
   - Circuit breakers prevent cascading failures
   - Retry logic with exponential backoff
   - Comprehensive error classification
   - Health checks and graceful degradation

2. **Security Hardening**
   - AES-256-GCM encryption at rest
   - Platform keychain integration
   - RBAC for authorization
   - Audit logging for compliance
   - Supply chain security (cargo-deny, cargo-audit)

3. **Developer Productivity**
   - Single command CI (`cargo ci`)
   - Comprehensive tooling (Makefile, xtask)
   - Fast builds (incremental compilation, Vite HMR)
   - Strong type safety (Rust + TypeScript)
   - Excellent IDE support

4. **Operational Excellence**
   - Structured metrics and tracing
   - Error classification for alerting
   - Health monitoring infrastructure
   - Graceful shutdown and lifecycle management

5. **Architectural Flexibility**
   - Hexagonal architecture enables easy mocking
   - Tiered feature system minimizes dependencies
   - Clear separation of concerns
   - Platform-agnostic business logic

### Negative

1. **Complexity**
   - Multi-layer architecture requires pattern understanding
   - Longer onboarding time
   - More boilerplate (traits, implementations)

2. **Platform Lock-in**
   - macOS-only (no Linux/Windows support)
   - Heavy reliance on macOS-specific APIs
   - Porting requires significant refactoring

3. **Build Times**
   - Rust compilation can be slow
   - Large dependency tree

### Mitigations

1. **Documentation**
   - Comprehensive ADRs
   - API guides with examples
   - Inline code comments
   - Architecture diagrams

2. **Tooling**
   - Automated CI pipeline
   - Developer CLI tools
   - Pre-commit hooks

3. **Training**
   - Onboarding guides
   - Architecture walkthroughs
   - Code review guidelines

---

## Future Considerations

### 1. Cross-Platform Support

**Options:**
- Abstract platform layer with trait-based providers
- Use cross-platform libraries (accesskit, rdev)

### 2. Cloud Sync

**Design:**
- `SyncService` in core
- `CloudSyncProvider` trait
- End-to-end encryption
- Conflict resolution (CRDTs)

### 3. Plugin System

**Design:**
- WebAssembly (WASM) plugins
- Sandboxed execution
- Plugin API via trait objects

### 4. Distributed Tracing

**Implementation:**
- OpenTelemetry integration
- Jaeger/Tempo backend
- Span correlation (frontend → backend)

---

## Related Documents

- [ADR-001: Architecture Overview](./001-architecture-overview.md) (Superseded)
- [Common Crate API Guide](../../crates/common/docs/API_GUIDE.md)
- [Common Crate Guide](../../crates/common/docs/COMMON_CRATES_GUIDE.md)
- [CLAUDE.md](../../CLAUDE.md) - Development standards and rules
- [Tracker Refactor Plan](../TRACKER_REFACTOR_PLAN.md)

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-30 | Lewis Catapang | Production architecture (supersedes ADR-001) |

---

## Appendix: Key Architectural Decisions

### A. Tiered Feature System (Common Crate)

**Decision:** Opt-in feature tiers (foundation/runtime/platform)

**Rationale:**
- Minimize dependencies for library consumers
- Pay-only-for-what-you-use compilation times
- Clear dependency boundaries

**Trade-offs:**
- More complex Cargo.toml configuration
- Requires understanding of feature tiers

### B. Hexagonal Architecture

**Decision:** Ports (traits) + Adapters (implementations)

**Rationale:**
- Testability (mock implementations)
- Platform independence (core has zero platform code)
- Flexibility (swap implementations)

**Trade-offs:**
- More boilerplate (trait definitions)
- Learning curve for new developers

### C. Feature-Based Frontend

**Decision:** Vertical slices with all layers

**Rationale:**
- High cohesion, low coupling
- Team autonomy
- Scalability

**Trade-offs:**
- Potential code duplication
- Harder to enforce cross-cutting concerns

### D. macOS-Only

**Decision:** Target macOS exclusively (initial release)

**Rationale:**
- Faster time-to-market
- Deep platform integration
- Target audience primarily uses macOS

**Trade-offs:**
- Limits market reach
- Porting requires significant effort

---

**End of ADR-002**
