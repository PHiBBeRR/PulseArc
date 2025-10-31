# PulseArc Common Crates Comprehensive Guide

> **Last Updated**: October 31, 2025  
> **Version**: 1.0  
> **Module**: `pulsearc-common`

## Table of Contents

- [Part 1: Introduction & Architecture](#part-1-introduction--architecture)
- [Part 2: Foundation Tier Modules](#part-2-foundation-tier-modules)
- [Part 3: Runtime Tier Modules](#part-3-runtime-tier-modules)
- [Part 4: Platform Tier Modules](#part-4-platform-tier-modules)
- [Part 5: Integration Patterns](#part-5-integration-patterns)
- [Part 6: Best Practices & Performance](#part-6-best-practices--performance)
- [Part 7: Reference](#part-7-reference)

---

## Part 1: Introduction & Architecture

### Overview

The PulseArc common crates provide a modular foundation of reusable utilities, infrastructure components, and patterns used throughout the PulseArc application. The module follows an **opt-in feature system** with no default features enabled, allowing you to include only what you need.

### Philosophy

**Core Principles:**
1. **Safety First**: `#![forbid(unsafe_code)]` - All code is memory-safe with no unsafe blocks
2. **Opt-In Features**: Choose only the components you need via cargo features
3. **Composability**: Components work together seamlessly through shared traits and types
4. **Observability**: Built-in metrics, health checks, and structured logging
5. **Production-Ready**: Battle-tested patterns with comprehensive error handling

### Feature Tier System

The common crates are organized into three feature tiers that build upon each other:

```
┌──────────────────────────────────────────────────────────┐
│                      Platform Tier                        │
│  (auth, security, storage, compliance, crypto, privacy)   │
│  Requires: runtime + foundation + platform dependencies   │
└──────────────────────────────────────────────────────────┘
                           ▲
                           │
┌──────────────────────────────────────────────────────────┐
│                      Runtime Tier                         │
│  (cache, lifecycle, time, sync, resilience, observability)│
│  Requires: foundation + tokio/async runtime               │
└──────────────────────────────────────────────────────────┘
                           ▲
                           │
┌──────────────────────────────────────────────────────────┐
│                    Foundation Tier                        │
│     (error, validation, collections, utils, privacy)      │
│     No side effects, no async, minimal dependencies       │
└──────────────────────────────────────────────────────────┘
```

**Foundation Tier** (`foundation` feature):
- Core utilities without side effects
- No logging, no async runtime, minimal dependencies
- Includes: `error`, `validation`, `collections`, `utils`, `privacy`
- Use for: Libraries, plugins, shared utilities

**Runtime Tier** (`runtime` feature):
- Async infrastructure with Tokio runtime
- Includes foundation + observability (tracing)
- Adds: `cache`, `time`, `resilience`, `sync`, `lifecycle`
- Use for: Services, background workers, async applications

**Platform Tier** (`platform` feature):
- Platform integrations and system-specific features
- Includes runtime + platform dependencies
- Adds: `auth`, `security`, `storage`, `compliance`
- Use for: Full applications with database, OAuth, encryption

**Observability** (`observability` feature):
- Opt-in tracing and structured logging
- NOT included by default even in runtime tier
- Required if you want logging from common utilities

### Adding to Your Project

```toml
[dependencies]
# Minimal: just foundation utilities (no logging, no async)
pulsearc-common = { workspace = true, features = ["foundation"] }

# Runtime: async infrastructure with logging
pulsearc-common = { workspace = true, features = ["runtime"] }

# Full platform: all features
pulsearc-common = { workspace = true, features = ["platform"] }

# Custom combination
pulsearc-common = { workspace = true, features = ["foundation", "cache", "resilience"] }
```

### Architecture Diagram

```
┌───────────────────────────────────────────────────────────────────┐
│                        PulseArc Application                        │
└───────────────────────────────────────────────────────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │   pulsearc-common       │
                    └────────────┬────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
  ┌─────▼─────┐          ┌──────▼──────┐        ┌───────▼──────┐
  │Foundation │          │   Runtime   │        │   Platform   │
  │   Tier    │          │    Tier     │        │     Tier     │
  └─────┬─────┘          └──────┬──────┘        └───────┬──────┘
        │                       │                        │
  ┌─────▼─────────┐    ┌───────▼────────┐      ┌───────▼───────┐
  │ • error       │    │ • cache        │      │ • auth        │
  │ • validation  │    │ • lifecycle    │      │ • security    │
  │ • collections │    │ • resilience   │      │ • storage     │
  │ • utils       │    │ • sync         │      │ • compliance  │
  └───────────────┘    │ • time         │      └───────────────┘
                       │ • observability│
                       │ • crypto       │
                       │ • privacy      │
                       └────────────────┘
```

### Module Dependencies

```
auth ──────────┐
              ▼
security ─────┼─► keychain
              ▼
storage ──────┼─► crypto ──► encryption
              ▼
compliance ───┼─► observability ──► error
              │
privacy ──────┤
              │
lifecycle ────┤
              │
sync ─────────┼─► resilience
              │   time
cache ────────┘   collections
                  validation
                  utils
```

### When to Use Which Module

| Need | Module | Tier | Example Use Case |
|------|--------|------|------------------|
| Error handling | `error` | Foundation | Unified error types across modules |
| Input validation | `validation` | Foundation | Config file validation |
| Data structures | `collections` | Foundation | LRU cache, bloom filters, tries |
| In-memory caching | `cache` | Runtime | API response caching |
| Fault tolerance | `resilience` | Runtime | Retry with backoff, circuit breakers |
| Lifecycle management | `lifecycle` | Runtime | Manager startup/shutdown |
| Time utilities | `time` | Runtime | Duration parsing, cron schedules |
| Synchronization | `sync` | Runtime | Message queues, retry budgets |
| Encryption | `crypto` | Runtime | AES-256-GCM encryption |
| PII detection | `privacy` | Runtime | Secure hashing, pattern matching |
| Observability | `observability` | Runtime | Error tracking, metrics |
| OAuth 2.0 | `auth` | Platform | User authentication |
| RBAC | `security` | Platform | Role-based access control |
| Database | `storage` | Platform | SQLCipher integration |
| Audit logging | `compliance` | Platform | Regulatory compliance |
| Test utilities | `testing` | Runtime | Mocks, fixtures, assertions |

---

## Part 2: Foundation Tier Modules

Foundation tier modules provide core utilities without side effects, async runtime, or heavy dependencies. They are safe to use in any context.

### Error Handling (`error/`)

The error module provides a unified error handling system with classification, severity levels, and recovery strategies.

#### CommonError

A comprehensive error enum covering standard error patterns across the application:

```rust
use pulsearc_common::error::{CommonError, CommonResult, ErrorClassification};
use std::time::Duration;

fn load_config() -> CommonResult<Config> {
    // Use CommonError for standard patterns
    let json = std::fs::read_to_string("config.json")
        .map_err(|e| CommonError::persistence(e.to_string()))?;
    
    serde_json::from_str(&json)
        .map_err(|e| CommonError::serialization_format("JSON", e.to_string()))
}

// Check error properties
fn handle_error(err: CommonError) {
    if err.is_retryable() {
        println!("This error can be retried");
        if let Some(delay) = err.retry_after() {
            println!("Retry after: {:?}", delay);
        }
    }
    
    if err.is_critical() {
        println!("Critical error! Severity: {:?}", err.severity());
    }
}
```

**CommonError Variants:**

| Variant | Use Case | Retryable | Severity |
|---------|----------|-----------|----------|
| `CircuitBreakerOpen` | Circuit breaker protection | Yes | Warning |
| `RateLimitExceeded` | Rate limiting | Yes | Warning |
| `Timeout` | Operation timeout | Yes | Warning |
| `Lock` | Lock acquisition failure | Yes | Warning |
| `Serialization` | Data serialization error | No | Error |
| `Validation` | Invalid input | No | Error |
| `Config` | Configuration error | No | Error |
| `Storage` | Database/storage error | Yes | Error |
| `Backend` | Backend service error | Yes | Warning |
| `Unauthorized` | Auth failure | No | Error |
| `NotFound` | Resource not found | No | Info |
| `Internal` | Internal bug | No | Critical |

#### ErrorClassification Trait

Implement `ErrorClassification` for domain-specific errors:

```rust
use pulsearc_common::error::{
    CommonError, ErrorClassification, ErrorSeverity
};
use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Module-specific: {0}")]
    Specific(String),
    
    #[error(transparent)]
    Common(#[from] CommonError),
}

impl ErrorClassification for MyError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Specific(_) => false,
            Self::Common(e) => e.is_retryable(),
        }
    }
    
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Specific(_) => ErrorSeverity::Error,
            Self::Common(e) => e.severity(),
        }
    }
    
    fn is_critical(&self) -> bool {
        match self {
            Self::Common(e) => e.is_critical(),
            _ => false,
        }
    }
    
    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Common(e) => e.retry_after(),
            _ => None,
        }
    }
}
```

#### Error Migration Guide

Migrating from ad-hoc errors to CommonError:

```rust
// BEFORE
#[derive(Debug, thiserror::Error)]
pub enum OldError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Config error: {0}")]
    ConfigError(String),
    
    #[error("Timeout")]
    Timeout,
}

// AFTER
use pulsearc_common::error::CommonError;

#[derive(Debug, thiserror::Error)]
pub enum NewError {
    #[error(transparent)]
    Common(#[from] CommonError),
    
    // Keep only module-specific errors
    #[error("Domain-specific: {0}")]
    DomainSpecific(String),
}

// Update call sites
fn old_way() -> Result<(), OldError> {
    Err(OldError::ConfigError("invalid".to_string()))
}

fn new_way() -> Result<(), NewError> {
    Err(CommonError::config("invalid").into())
}
```

### Collections (`collections/`)

Specialized data structures for high-performance applications.

#### Bounded Queue

Thread-safe bounded queue with backpressure:

```rust
use pulsearc_common::collections::BoundedQueue;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let queue = Arc::new(BoundedQueue::new(100));
    
    // Producer
    let producer = queue.clone();
    tokio::spawn(async move {
        for i in 0..1000 {
            producer.push(i).await.unwrap();
        }
    });
    
    // Consumer
    let consumer = queue.clone();
    tokio::spawn(async move {
        while let Ok(Some(item)) = consumer.pop().await {
            println!("Processing: {}", item);
        }
    });
}
```

#### Ring Buffer

Fixed-size circular buffer for streaming data:

```rust
use pulsearc_common::collections::RingBuffer;

let mut buffer = RingBuffer::new(5);

// Push items (overwrites oldest when full)
for i in 0..10 {
    buffer.push(i);
}

// Latest 5 items: [5, 6, 7, 8, 9]
assert_eq!(buffer.len(), 5);
assert_eq!(buffer.get(0), Some(&5));
```

#### LRU Cache

Least Recently Used cache with automatic eviction:

```rust
use pulsearc_common::collections::LruCache;
use std::num::NonZeroUsize;

let mut cache = LruCache::new(NonZeroUsize::new(100).unwrap());

cache.put("key1", "value1");
cache.put("key2", "value2");

// Get updates access time
assert_eq!(cache.get(&"key1"), Some(&"value1"));

// Peek doesn't update access time
assert_eq!(cache.peek(&"key2"), Some(&"value2"));
```

#### Trie

Prefix tree for efficient string searching:

```rust
use pulsearc_common::collections::Trie;

let mut trie = Trie::new();

// Build autocomplete dictionary
trie.insert("apple");
trie.insert("application");
trie.insert("apply");
trie.insert("banana");

// Find all words with prefix
let suggestions = trie.find_prefix("app");
// Returns: ["apple", "application", "apply"]
```

#### Bloom Filter

Probabilistic membership testing:

```rust
use pulsearc_common::collections::BloomFilter;

// 1000 expected items, 1% false positive rate
let mut filter = BloomFilter::new(1000, 0.01);

filter.insert("user123");
filter.insert("user456");

assert!(filter.contains("user123"));  // Definitely in set
assert!(!filter.contains("user999")); // Probably not in set
```

### Validation (`validation/`)

Enterprise-grade validation framework with field-level errors.

#### Basic Validation

```rust
use pulsearc_common::validation::{
    Validator, StringValidator, RangeValidator, EmailValidator
};

let mut validator = Validator::new();

// String validation
let username = "alice";
if let Err(e) = StringValidator::new()
    .min_length(3)
    .max_length(20)
    .pattern(r"^[a-zA-Z0-9_]+$")
    .unwrap()
    .validate(username)
{
    validator.add_error("username", &e.to_string());
}

// Email validation
if let Err(_) = EmailValidator::new().validate("invalid-email") {
    validator.add_error("email", "Invalid email format");
}

// Range validation
if let Err(_) = RangeValidator::new()
    .min(1)
    .max(100)
    .validate(&150)
{
    validator.add_error("age", "Age must be between 1 and 100");
}

// Check results
if validator.has_errors() {
    eprintln!("Validation failed: {:?}", validator.errors());
}
```

#### RuleSet Composition

```rust
use pulsearc_common::validation::{RuleSet, RuleBuilder};

let rules = RuleSet::new()
    .add_rule(RuleBuilder::new()
        .field("username")
        .required()
        .min_length(3)
        .max_length(50)
        .build())
    .add_rule(RuleBuilder::new()
        .field("email")
        .required()
        .email()
        .build())
    .add_rule(RuleBuilder::new()
        .field("age")
        .required()
        .range(18, 120)
        .build());

// Validate entire config
rules.validate(&config)?;
```

### Utils (`utils/`)

Common utilities including serialization helpers and macros.

#### Status Conversion Macro

Eliminate boilerplate for Display and FromStr:

```rust
use pulsearc_common::impl_status_conversions;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl_status_conversions!(Status {
    Pending => "pending",
    InProgress => "in_progress",
    Completed => "completed",
    Failed => "failed",
});

// Display (lowercase)
assert_eq!(Status::InProgress.to_string(), "in_progress");

// FromStr (case-insensitive)
assert_eq!(Status::from_str("COMPLETED").unwrap(), Status::Completed);
assert_eq!(Status::from_str("pending").unwrap(), Status::Pending);
```

#### Duration Serialization

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;
use pulsearc_common::utils::duration_millis;

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(with = "duration_millis")]
    timeout: Duration,
    
    #[serde(with = "duration_millis")]
    retry_delay: Duration,
}

let config = Config {
    timeout: Duration::from_secs(30),
    retry_delay: Duration::from_millis(1500),
};

// Serializes as: {"timeout":30000,"retry_delay":1500}
let json = serde_json::to_string(&config).unwrap();
```

---

## Part 3: Runtime Tier Modules

Runtime tier modules provide async infrastructure, resilience patterns, and observability. They require a Tokio runtime.

### Cache (`cache/`)

High-performance in-memory caching with TTL, eviction policies, and metrics.

#### Eviction Policies

```rust
use pulsearc_common::cache::{Cache, CacheConfig, EvictionPolicy};
use std::time::Duration;

// LRU (Least Recently Used) - default
let cache = Cache::<String, String>::new(
    CacheConfig::lru(1000)
);

// LFU (Least Frequently Used)
let cache = Cache::<String, String>::new(
    CacheConfig::builder()
        .max_size(1000)
        .eviction_policy(EvictionPolicy::LFU)
        .build()
);

// TTL-only (no size limit)
let cache = Cache::<String, String>::new(
    CacheConfig::ttl(Duration::from_secs(3600))
);

// Combined TTL + LRU
let cache = Cache::<String, String>::new(
    CacheConfig::ttl_lru(Duration::from_secs(600), 5000)
);
```

#### Cache Operations

```rust
use pulsearc_common::cache::{Cache, CacheConfig};

let cache = Cache::<String, i32>::new(CacheConfig::lru(128));

// Insert
cache.insert("answer".into(), 42);

// Get
assert_eq!(cache.get(&"answer".into()), Some(42));

// Get or insert with closure
let value = cache.get_or_insert_with("key".into(), || {
    expensive_computation()
});

// Cleanup expired entries
let removed = cache.cleanup_expired();

// Statistics
if let Some(stats) = cache.stats() {
    println!("Hit rate: {:.1}%", stats.hit_rate() * 100.0);
    println!("Evictions: {}", stats.evictions);
}
```

#### Async Cache

```rust
use pulsearc_common::cache::{AsyncCache, CacheConfig};

#[tokio::main]
async fn main() {
    let cache = AsyncCache::<String, String>::new(CacheConfig::lru(1000));
    
    // Async operations
    cache.insert("key".into(), "value".into()).await;
    
    let value = cache.get(&"key".into()).await;
    
    // Async get_or_insert
    let value = cache.get_or_insert_with_async(
        "user:123".into(),
        || async {
            fetch_user_from_db("123").await.unwrap()
        }
    ).await;
}
```

### Resilience (`resilience/`)

Fault tolerance patterns including circuit breakers and retry logic.

#### Circuit Breaker

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig
};
use std::time::Duration;

let breaker = CircuitBreaker::new(
    CircuitBreakerConfig::builder()
        .failure_threshold(5)      // Open after 5 failures
        .success_threshold(2)       // Close after 2 successes
        .timeout(Duration::from_secs(60))  // Recovery period
        .build()
        .unwrap()
);

// Use circuit breaker
let result = breaker.call(|| async {
    external_api_call().await
}).await;

// Check state
println!("Circuit state: {:?}", breaker.state());
```

#### Retry with Backoff

```rust
use pulsearc_common::resilience::{
    retry, RetryConfig, BackoffStrategy, Jitter
};
use std::time::Duration;

let config = RetryConfig::builder()
    .max_attempts(5)
    .initial_delay(Duration::from_millis(100))
    .max_delay(Duration::from_secs(10))
    .backoff_strategy(BackoffStrategy::Exponential)
    .jitter(Jitter::Full)
    .build()
    .unwrap();

let result = retry(config, || async {
    risky_operation().await
}).await;
```

### Lifecycle (`lifecycle/`)

Standardized lifecycle management for async components.

#### AsyncManager Trait

```rust
use pulsearc_common::lifecycle::{
    AsyncManager, ManagerHealth, ManagerStatus, SharedState
};

pub struct MyManager {
    status: SharedState<ManagerStatus>,
    config: MyConfig,
}

#[async_trait::async_trait]
impl AsyncManager for MyManager {
    type Error = MyError;
    type Config = MyConfig;
    
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            status: SharedState::new(ManagerStatus::Created, "my_manager"),
            config: MyConfig::default(),
        })
    }
    
    async fn initialize(&mut self) -> Result<(), Self::Error> {
        self.status.replace(ManagerStatus::Initializing).await;
        // Initialize resources
        self.status.replace(ManagerStatus::Running).await;
        Ok(())
    }
    
    async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
        Ok(ManagerHealth::healthy())
    }
    
    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.status.replace(ManagerStatus::ShuttingDown).await;
        // Cleanup resources
        self.status.replace(ManagerStatus::Shutdown).await;
        Ok(())
    }
    
    fn status(&self) -> ManagerStatus {
        self.status.try_read()
            .map(|guard| *guard)
            .unwrap_or(ManagerStatus::Error)
    }
}
```

#### Manager Controller

```rust
use pulsearc_common::lifecycle::ManagerController;

// Coordinate multiple managers
let mut controller = ManagerController::new();

controller.add_manager(DatabaseManager::new().await?);
controller.add_manager(CacheManager::new().await?);
controller.add_manager(ApiManager::new().await?);

// Initialize all in order
controller.initialize_all().await?;

// Shutdown all in reverse order
controller.shutdown_all().await?;
```

### Time (`time/`)

Time utilities including duration parsing, cron expressions, and mock clocks.

#### Duration Parsing

```rust
use pulsearc_common::time::parse_duration;
use std::time::Duration;

// Parse human-readable durations
assert_eq!(parse_duration("5s")?, Duration::from_secs(5));
assert_eq!(parse_duration("2h 30m")?, Duration::from_secs(9000));
assert_eq!(parse_duration("1d")?, Duration::from_secs(86400));
```

#### Cron Expressions

```rust
use pulsearc_common::time::{CronExpression, CronSchedule};
use chrono::{Utc, Weekday};

// Parse cron expression
let cron = CronExpression::parse("0 9 * * 1")?; // Every Monday at 9am

// Check if datetime matches
let dt = Utc::now();
if cron.matches(&dt) {
    println!("It's time!");
}

// Get next occurrence
let schedule = CronSchedule::new("0 * * * *")?; // Every hour
if let Some(next) = schedule.next() {
    println!("Next run: {}", next);
}
```

#### Mock Clock for Testing

```rust
use pulsearc_common::time::MockClock;
use std::time::Duration;

#[test]
fn test_with_controlled_time() {
    let clock = MockClock::new();
    let start = clock.now();
    
    // Advance time deterministically
    clock.advance(Duration::from_secs(5));
    
    let end = clock.now();
    assert_eq!(end.duration_since(start), Duration::from_secs(5));
}
```

### Sync (`sync/`)

Synchronization infrastructure with retry, circuit breakers, and message queues.

#### Retry Strategy

```rust
use pulsearc_common::sync::{RetryStrategy, RetryPolicies};
use std::time::Duration;

// Use predefined policy
let strategy = RetryPolicies::network_policy();

let result = strategy.execute(|| async {
    http_request().await
}).await;

// Custom strategy
let strategy = RetryStrategy::new()
    .with_max_attempts(5)?
    .with_base_delay(Duration::from_millis(100))?
    .with_jitter_factor(0.3);

let (result, metrics) = strategy
    .execute_with_metrics("api_call", || async {
        api_request().await
    })
    .await;

println!("Attempts: {}", metrics.attempts);
println!("Succeeded: {}", metrics.succeeded);
```

#### Retry Budget

```rust
use pulsearc_common::sync::RetryBudget;

// 100 tokens, refill 10 per second
let budget = RetryBudget::new(100, 10.0);

if budget.try_acquire_multiple(3) {
    // Have budget for up to 3 retries
    let result = retry_operation().await;
    
    // Return unused tokens
    if result.is_ok() {
        budget.return_tokens(2);
    }
}
```

#### Message Queue

```rust
use pulsearc_common::sync::queue::{SyncQueue, SyncItem, Priority};

let queue = SyncQueue::new()?;

// Push high-priority item
let item = SyncItem::new(
    serde_json::json!({"task": "process_payment"}),
    Priority::High
)
.with_max_retries(5);

queue.push(item).await?;

// Process items
while let Some(item) = queue.pop().await? {
    match process(&item).await {
        Ok(_) => queue.mark_completed(&item.id).await?,
        Err(e) => {
            queue.mark_failed(&item.id, Some(e.to_string())).await?;
        }
    }
}
```

### Crypto (`crypto/`)

Cryptographic primitives using AES-256-GCM.

#### Encryption Service

```rust
use pulsearc_common::crypto::EncryptionService;

// Generate random key
let key = EncryptionService::generate_key();
let service = EncryptionService::new(key)?;

// Encrypt data
let plaintext = b"sensitive data";
let encrypted = service.encrypt(plaintext)?;

// Decrypt data
let decrypted = service.decrypt(&encrypted)?;
assert_eq!(decrypted, plaintext);

// Password-based encryption
let service = EncryptionService::from_password("my-password")?;
let encrypted = service.encrypt(b"secret")?;
```

### Privacy (`privacy/`)

Privacy-preserving operations including hashing and PII detection.

#### Secure Hashing

```rust
use pulsearc_common::privacy::{SecureHasher, HashConfig, HashAlgorithm};

// Create hasher
let hasher = SecureHasher::new(HashConfig::default())?;

// Hash data
let hashed = hasher.hash_string("user@example.com")?;

// Verify
assert!(hasher.verify("user@example.com", &hashed)?);

// Batch hashing
let values = vec!["email1@test.com", "email2@test.com"];
let hashes = hasher.hash_batch(&values)?;
```

#### PII Detection

```rust
use pulsearc_common::privacy::{
    PatternMatcher, PiiDetectionConfig, PiiType, RedactionStrategy
};

// Configure detector
let config = PiiDetectionConfig::default()
    .enable_type(PiiType::Email)
    .enable_type(PiiType::PhoneNumber)
    .enable_type(PiiType::CreditCard)
    .set_redaction_strategy(RedactionStrategy::Partial);

let matcher = PatternMatcher::new(config)?;

// Detect PII
let text = "Contact john@example.com or call 555-1234";
let entities = matcher.detect(text)?;

for entity in entities {
    println!("Found: {:?} at position {}", 
        entity.pii_type, entity.location.start);
}

// Redact PII
let redacted = matcher.redact(text)?;
// Output: "Contact j***@example.com or call ***-1234"
```

### Observability (`observability/`)

Unified observability with error handling, metrics, and tracing.

#### AppError Hierarchy

```rust
use pulsearc_common::observability::{AppError, AiError, ActionHint};
use std::time::Duration;

// AI-specific errors
let err = AppError::Ai(AiError::RateLimited {
    retry_after: Some(Duration::from_secs(5)),
});

// Get error properties
let code = err.code();  // Stable error code for telemetry
let action = err.action();  // Recovery hint

match action {
    ActionHint::RetryAfter { duration } => {
        tokio::time::sleep(duration).await;
        // Retry operation
    }
    ActionHint::CheckOpenAiKey => {
        eprintln!("Please verify your OpenAI API key");
    }
    _ => {}
}

// Convert for frontend
let ui_error = err.to_ui();
```

#### Trait Abstractions

```rust
use pulsearc_common::observability::{
    MetricsCollector, AuditLogger, AuditLogEntry, AuditSeverity
};

// Collect metrics
fn track_request(collector: &impl MetricsCollector) {
    collector.increment_counter(
        "api_requests",
        &[("endpoint", "/classify"), ("status", "success")]
    );
    
    collector.record_timing(
        "request_duration",
        125,
        &[("endpoint", "/classify")]
    );
}

// Audit logging
async fn log_action(logger: &impl AuditLogger) {
    let entry = AuditLogEntry::new("user_login", AuditSeverity::Info)
        .with_user("user123")
        .with_ip("192.168.1.1");
    
    logger.log(entry).await;
}
```

---

## Part 4: Platform Tier Modules

Platform tier modules provide system integrations including OAuth, RBAC, encrypted storage, and compliance features.

### Auth (`auth/`)

OAuth 2.0 with PKCE for secure authentication.

#### Complete OAuth Flow

```rust
use pulsearc_common::auth::{OAuthConfig, OAuthService};
use pulsearc_common::security::KeychainProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OAuth provider
    let config = OAuthConfig::new(
        "auth.example.com".into(),
        "client-id".into(),
        "http://localhost:14251/callback".into(),
        vec!["openid".into(), "profile".into(), "offline_access".into()],
        Some("https://api.example.com".into()),
    );
    
    // Create service
    let keychain = Arc::new(KeychainProvider::new("MyApp.oauth"));
    let service = OAuthService::new(
        config,
        keychain,
        "MyApp".into(),
        "user@example.com".into(),
        300,  // Refresh 5 min before expiry
    );
    
    // Try to restore saved tokens
    if service.initialize().await? {
        service.start_auto_refresh();
        println!("Restored saved credentials");
        
        let token = service.get_access_token().await?;
        println!("Access token: {}", token);
        return Ok(());
    }
    
    // Start login flow
    let (auth_url, state) = service.start_login().await?;
    println!("Open: {}", auth_url);
    
    // Handle callback (pseudo-code)
    // let (code, returned_state) = wait_for_callback();
    // service.complete_login(&code, &returned_state).await?;
    
    // Start auto-refresh
    service.start_auto_refresh();
    
    Ok(())
}
```

### Security (`security/`)

Security primitives including RBAC and encryption key management.

#### RBAC System

```rust
use pulsearc_common::security::{
    RBACManager, Role, Permission, RBACPolicy, PolicyCondition
};

let mut rbac = RBACManager::new();

// Define roles
let admin = Role::new("admin")
    .with_permission(Permission::new("*"));

let user = Role::new("user")
    .with_permission(Permission::new("read"))
    .with_permission(Permission::new("write:own"));

rbac.add_role(admin);
rbac.add_role(user);

// Check permissions
let user_ctx = UserContext::new("user123")
    .with_role("user")
    .with_attribute("department", "engineering");

if rbac.check_permission(&user_ctx, "read").await? {
    println!("User can read");
}

// Dynamic policies
let policy = RBACPolicy::new("business_hours")
    .condition(PolicyCondition::TimeRange {
        start: "09:00".into(),
        end: "17:00".into(),
    });

rbac.add_policy(policy);
```

#### Key Management

```rust
use pulsearc_common::security::encryption::{
    StorageKeyManager, KeyRotationSchedule
};
use std::time::Duration;

let key_manager = StorageKeyManager::new()?;

// Generate and store encryption key
let key = key_manager.generate_key()?;
key_manager.store_current_key(&key)?;

// Rotate keys on schedule
key_manager.rotate_key()?;

// Configure auto-rotation
let schedule = KeyRotationSchedule::new(Duration::from_secs(86400 * 30)); // 30 days
key_manager.set_rotation_schedule(schedule)?;
```

### Storage (`storage/`)

Encrypted database infrastructure with SQLCipher.

#### SQLCipher Connection

```rust
use pulsearc_common::storage::{
    SqlCipherConnection, SqlCipherPoolConfig, SqlCipherPool
};

// Single connection
let conn = SqlCipherConnection::open(
    "database.db",
    "encryption-key".as_bytes()
)?;

conn.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT)", [])?;

// Connection pool
let pool_config = SqlCipherPoolConfig::builder()
    .max_size(10)
    .min_idle(Some(2))
    .build();

let pool = SqlCipherPool::new("database.db", "encryption-key".as_bytes(), pool_config)?;

let conn = pool.get()?;
conn.execute("INSERT INTO users (id, name) VALUES (?1, ?2)", [1, "Alice"])?;
```

### Compliance (`compliance/`)

Compliance infrastructure for audit logging and feature flags.

#### Audit Logging

```rust
use pulsearc_common::compliance::{GlobalAuditLogger, AuditEvent, AuditSeverity};

let logger = GlobalAuditLogger::new();

// Log audit event
let event = AuditEvent::new("data_access", AuditSeverity::Info)
    .with_user("user123")
    .with_resource("customer_records")
    .with_ip("192.168.1.1")
    .with_metadata("action", "read");

logger.log(event).await;

// Query audit trail
let recent = logger.get_recent_events(100).await;
```

#### Feature Flags

```rust
use pulsearc_common::compliance::{FeatureFlagManager, FeatureFlag};

let mut flags = FeatureFlagManager::new();

// Define feature flags
flags.add_flag(FeatureFlag::new("new_ui")
    .enabled(true)
    .rollout_percentage(50));

// Check if enabled
if flags.is_enabled("new_ui", "user123").await? {
    // Show new UI
}
```

### Testing (`testing/`)

Comprehensive test utilities including mocks, fixtures, and assertions.

#### Mock Implementations

```rust
use pulsearc_common::testing::mocks::{MockHttpClient, MockStorage};

// Mock HTTP client
let client = MockHttpClient::new();
client.add_response(
    "https://api.example.com/users",
    200,
    r#"{"users": []}"#
);

let response = client.get("https://api.example.com/users")?;
assert_eq!(response.status, 200);

// Mock storage
let storage = MockStorage::new();
storage.set("key", "value")?;
assert_eq!(storage.get("key")?, Some("value".to_string()));
```

#### Test Fixtures

```rust
use pulsearc_common::testing::fixtures::{
    random_string, random_email, random_u64
};

#[test]
fn test_with_random_data() {
    let user_id = random_string(16);
    let email = random_email();
    let timestamp = random_u64();
    
    // Use in tests
    create_user(user_id, email, timestamp);
}
```

#### Temporary Files

```rust
use pulsearc_common::testing::temp::{TempFile, TempDir};

#[test]
fn test_file_operations() {
    let temp = TempFile::with_contents("test", "txt", "data")?;
    let content = temp.read()?;
    assert_eq!(content, "data");
    
    // Auto-cleanup on drop
}
```

---

## Part 5: Integration Patterns

### Cross-Module Integration

#### Complete Service Stack

```rust
use pulsearc_common::{
    error::CommonError,
    resilience::{CircuitBreaker, CircuitBreakerConfig, retry, RetryConfig},
    cache::{AsyncCache, CacheConfig},
    lifecycle::{AsyncManager, ManagerHealth},
    observability::MetricsCollector,
};
use std::sync::Arc;

pub struct ResilientService {
    cache: Arc<AsyncCache<String, String>>,
    circuit_breaker: Arc<CircuitBreaker>,
    metrics: Arc<dyn MetricsCollector>,
}

impl ResilientService {
    pub fn new(metrics: Arc<dyn MetricsCollector>) -> Self {
        Self {
            cache: Arc::new(AsyncCache::new(CacheConfig::lru(1000))),
            circuit_breaker: Arc::new(CircuitBreaker::new(
                CircuitBreakerConfig::builder()
                    .failure_threshold(5)
                    .timeout(std::time::Duration::from_secs(60))
                    .build()
                    .unwrap()
            )),
            metrics,
        }
    }
    
    pub async fn fetch_data(&self, key: &str) -> Result<String, CommonError> {
        // Check cache first
        if let Some(cached) = self.cache.get(&key.to_string()).await {
            self.metrics.increment_counter("cache_hits", &[]);
            return Ok(cached);
        }
        
        // Check circuit breaker
        if !self.circuit_breaker.should_allow_request()? {
            return Err(CommonError::circuit_breaker("data_service"));
        }
        
        // Retry with backoff
        let config = RetryConfig::builder()
            .max_attempts(3)
            .build()
            .unwrap();
        
        let result = retry(config, || async {
            self.fetch_from_backend(key).await
        }).await;
        
        match result {
            Ok(data) => {
                self.circuit_breaker.record_success()?;
                self.cache.insert(key.to_string(), data.clone()).await;
                self.metrics.increment_counter("backend_hits", &[]);
                Ok(data)
            }
            Err(e) => {
                self.circuit_breaker.record_failure()?;
                self.metrics.increment_counter("backend_errors", &[]);
                Err(e.into())
            }
        }
    }
    
    async fn fetch_from_backend(&self, key: &str) -> Result<String, CommonError> {
        // Actual backend call
        Ok(format!("data-{}", key))
    }
}
```

### Design Patterns

#### Repository Pattern with Caching

```rust
use pulsearc_common::{
    cache::{AsyncCache, CacheConfig},
    storage::SqlCipherPool,
    error::CommonResult,
};
use std::sync::Arc;

pub struct UserRepository {
    db: Arc<SqlCipherPool>,
    cache: Arc<AsyncCache<String, User>>,
}

impl UserRepository {
    pub fn new(db: Arc<SqlCipherPool>) -> Self {
        Self {
            db,
            cache: Arc::new(AsyncCache::new(
                CacheConfig::ttl_lru(
                    std::time::Duration::from_secs(300),
                    1000
                )
            )),
        }
    }
    
    pub async fn get_user(&self, id: &str) -> CommonResult<Option<User>> {
        // Check cache
        if let Some(user) = self.cache.get(&id.to_string()).await {
            return Ok(Some(user));
        }
        
        // Query database
        let conn = self.db.get()?;
        let user = query_user(&conn, id)?;
        
        // Cache result
        if let Some(ref u) = user {
            self.cache.insert(id.to_string(), u.clone()).await;
        }
        
        Ok(user)
    }
    
    pub async fn update_user(&self, user: &User) -> CommonResult<()> {
        // Update database
        let conn = self.db.get()?;
        update_user_db(&conn, user)?;
        
        // Invalidate cache
        self.cache.remove(&user.id).await;
        
        Ok(())
    }
}
```

#### Event-Driven with Message Queue

```rust
use pulsearc_common::sync::queue::{SyncQueue, SyncItem, Priority};
use std::sync::Arc;

pub struct EventBus {
    queue: Arc<SyncQueue>,
}

impl EventBus {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            queue: Arc::new(SyncQueue::new()?),
        })
    }
    
    pub async fn publish(&self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        let priority = match event.severity {
            Severity::Critical => Priority::Critical,
            Severity::High => Priority::High,
            Severity::Normal => Priority::Normal,
            Severity::Low => Priority::Low,
        };
        
        let item = SyncItem::new(
            serde_json::to_value(&event)?,
            priority
        );
        
        self.queue.push(item).await?;
        Ok(())
    }
    
    pub async fn subscribe<F>(&self, mut handler: F)
    where
        F: FnMut(Event) -> Result<(), Box<dyn std::error::Error>>,
    {
        while let Some(item) = self.queue.pop().await.unwrap() {
            let event: Event = serde_json::from_value(item.payload).unwrap();
            
            match handler(event) {
                Ok(_) => {
                    self.queue.mark_completed(&item.id).await.unwrap();
                }
                Err(e) => {
                    self.queue.mark_failed(&item.id, Some(e.to_string())).await.unwrap();
                }
            }
        }
    }
}
```

### Migration Guides

#### From Manual Retry to Resilience

```rust
// BEFORE: Manual retry logic
async fn fetch_with_manual_retry(url: &str) -> Result<String, Error> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        match fetch(url).await {
            Ok(data) => return Ok(data),
            Err(e) if attempts < 3 => {
                tokio::time::sleep(Duration::from_millis(100 * attempts)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

// AFTER: Using resilience module
use pulsearc_common::resilience::{retry, RetryConfig, BackoffStrategy};

async fn fetch_with_resilience(url: &str) -> Result<String, Error> {
    let config = RetryConfig::builder()
        .max_attempts(3)
        .initial_delay(Duration::from_millis(100))
        .backoff_strategy(BackoffStrategy::Exponential)
        .build()
        .unwrap();
    
    retry(config, || async {
        fetch(url).await
    }).await
}
```

#### Adding Observability

```rust
// BEFORE: No observability
async fn process_item(item: Item) -> Result<(), Error> {
    let result = expensive_operation(item).await;
    result
}

// AFTER: With observability
use pulsearc_common::observability::{MetricsCollector, AuditLogger};

async fn process_item_with_observability(
    item: Item,
    metrics: &impl MetricsCollector,
    audit: &impl AuditLogger,
) -> Result<(), Error> {
    let start = std::time::Instant::now();
    
    // Log audit event
    audit.log(AuditLogEntry::new("item_processing", AuditSeverity::Info)
        .with_metadata("item_id", &item.id)).await;
    
    // Process with error tracking
    let result = expensive_operation(item).await;
    
    // Record metrics
    let duration = start.elapsed();
    metrics.record_timing(
        "item_processing_duration",
        duration.as_millis() as u64,
        &[("status", if result.is_ok() { "success" } else { "error" })]
    );
    
    metrics.increment_counter(
        "items_processed",
        &[("status", if result.is_ok() { "success" } else { "error" })]
    );
    
    result
}
```

---

## Part 6: Best Practices & Performance

### Performance Optimization

#### Lock Contention Strategies

```rust
use pulsearc_common::lifecycle::SharedState;
use std::time::Duration;

// Good: Use timeouts to prevent deadlocks
async fn read_with_timeout<T: Clone>(state: &SharedState<T>) -> Option<T> {
    state.read_timeout(Duration::from_secs(1))
        .await
        .ok()
        .map(|guard| guard.clone())
}

// Good: Use try_read for non-critical operations
fn status_check<T: Clone>(state: &SharedState<T>) -> Option<T> {
    state.try_read()
        .ok()
        .map(|guard| guard.clone())
}

// Bad: Blocking indefinitely
async fn bad_read<T: Clone>(state: &SharedState<T>) -> T {
    state.read().await.clone()  // Can deadlock!
}
```

#### Memory Usage Patterns

```rust
use std::sync::Arc;

// Good: Share large data with Arc
struct Service {
    config: Arc<Config>,  // Shared, not cloned
    cache: Arc<AsyncCache<String, Arc<Data>>>,  // Values are Arc'd
}

// Bad: Cloning large data
struct BadService {
    config: Config,  // Cloned on every Service creation
    cache: AsyncCache<String, Data>,  // Data cloned on cache hits
}
```

#### Async Runtime Considerations

```rust
// Good: Spawn CPU-intensive work on blocking threads
use tokio::task;

async fn process_heavy_computation(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    task::spawn_blocking(move || {
        // Heavy CPU work won't block async runtime
        compress_data(data)
    }).await?
}

// Bad: Blocking the async runtime
async fn bad_process(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    // This blocks all other async tasks!
    Ok(compress_data(data))
}
```

### Security Best Practices

#### Secret Management

```rust
use pulsearc_common::security::encryption::SecureString;
use pulsearc_common::security::KeychainProvider;

// Good: Use SecureString for sensitive data
fn handle_password(password: SecureString) {
    // password is zeroized on drop
}

// Good: Store secrets in keychain
async fn store_api_key(key: &str) -> Result<(), Error> {
    let keychain = KeychainProvider::new("MyApp");
    keychain.set("api_key", key)?;
    Ok(())
}

// Bad: Plain strings for secrets
fn bad_handle_password(password: String) {
    // password remains in memory until garbage collected
}
```

#### Encryption Key Lifecycle

```rust
use pulsearc_common::security::encryption::StorageKeyManager;
use std::time::Duration;

// Good: Rotate keys regularly
let key_manager = StorageKeyManager::new()?;
key_manager.set_rotation_schedule(
    KeyRotationSchedule::new(Duration::from_secs(86400 * 30)) // 30 days
)?;

// Good: Use separate keys for different purposes
let user_data_key = key_manager.get_key("user_data")?;
let system_key = key_manager.get_key("system")?;
```

#### PII Handling

```rust
use pulsearc_common::privacy::{PatternMatcher, RedactionStrategy};

// Good: Redact PII before logging
fn log_user_action(action: &str) {
    let matcher = PatternMatcher::new(
        PiiDetectionConfig::default()
            .set_redaction_strategy(RedactionStrategy::Full)
    ).unwrap();
    
    let redacted = matcher.redact(action).unwrap();
    tracing::info!("User action: {}", redacted);
}

// Bad: Logging raw PII
fn bad_log(email: &str) {
    tracing::info!("User email: {}", email);  // PII in logs!
}
```

### Testing Strategies

#### Unit Testing with Mocks

```rust
use pulsearc_common::testing::mocks::MockStorage;

#[test]
fn test_repository() {
    let storage = MockStorage::new();
    let repo = UserRepository::new(storage);
    
    // Test without real database
    repo.create_user("alice", "alice@example.com").unwrap();
    let user = repo.get_user("alice").unwrap();
    
    assert_eq!(user.email, "alice@example.com");
}
```

#### Integration Testing

```rust
use pulsearc_common::testing::temp::TempDir;

#[tokio::test]
async fn test_full_flow() {
    let temp_dir = TempDir::new("test")?;
    let db_path = temp_dir.create_file("test.db", "")?;
    
    // Setup test environment
    let pool = create_pool(&db_path)?;
    let service = MyService::new(pool);
    
    // Run test
    service.initialize().await?;
    let result = service.process_data().await?;
    
    assert_eq!(result.status, "success");
    
    // Auto-cleanup on drop
}
```

#### Benchmark Testing

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pulsearc_common::cache::{Cache, CacheConfig};

fn cache_benchmark(c: &mut Criterion) {
    let cache = Cache::<String, i32>::new(CacheConfig::lru(1000));
    
    c.bench_function("cache_insert", |b| {
        let mut i = 0;
        b.iter(|| {
            cache.insert(format!("key{}", i), black_box(i));
            i += 1;
        });
    });
    
    c.bench_function("cache_get", |b| {
        b.iter(|| {
            cache.get(&"key0".to_string());
        });
    });
}

criterion_group!(benches, cache_benchmark);
criterion_main!(benches);
```

### Production Readiness

#### Health Checks

```rust
use pulsearc_common::lifecycle::{ManagerHealth, ComponentHealth};

async fn comprehensive_health_check() -> ManagerHealth {
    let mut health = ManagerHealth::healthy();
    
    // Check database
    if db.ping().await.is_err() {
        health = health.with_component(
            ComponentHealth::unhealthy("database", "Connection failed")
        );
    }
    
    // Check cache
    if cache.stats().is_none() {
        health = health.with_component(
            ComponentHealth::degraded("cache", "Metrics unavailable", 0.5)
        );
    }
    
    // Check external API
    if !api.is_reachable().await {
        health = health.with_component(
            ComponentHealth::unhealthy("api", "Unreachable")
        );
    }
    
    health
}
```

#### Graceful Degradation

```rust
async fn fetch_with_fallback(key: &str) -> Result<Data, Error> {
    // Try primary source
    match fetch_from_primary(key).await {
        Ok(data) => Ok(data),
        Err(e) => {
            tracing::warn!("Primary source failed: {}, using fallback", e);
            
            // Try cache
            if let Some(cached) = try_cache(key).await {
                return Ok(cached);
            }
            
            // Use stale data
            if let Some(stale) = try_stale(key).await {
                tracing::warn!("Using stale data for key: {}", key);
                return Ok(stale);
            }
            
            // Return error
            Err(e)
        }
    }
}
```

---

## Part 7: Reference

### API Quick Reference

#### Error Handling

```rust
// Create errors
CommonError::config("message")
CommonError::timeout("operation", duration)
CommonError::validation("field", "message")
CommonError::not_found("resource", "id")

// Check error properties
error.is_retryable()
error.severity()
error.is_critical()
error.retry_after()
```

#### Collections

```rust
// Bounded Queue
BoundedQueue::new(capacity)
queue.push(item).await
queue.pop().await

// Ring Buffer
RingBuffer::new(capacity)
buffer.push(item)
buffer.get(index)

// LRU Cache
LruCache::new(capacity)
cache.put(key, value)
cache.get(&key)

// Trie
Trie::new()
trie.insert(word)
trie.find_prefix(prefix)

// Bloom Filter
BloomFilter::new(expected, fpr)
filter.insert(item)
filter.contains(item)
```

#### Cache

```rust
// Configuration
CacheConfig::lru(size)
CacheConfig::ttl(duration)
CacheConfig::ttl_lru(duration, size)

// Operations
cache.insert(key, value)
cache.get(&key)
cache.get_or_insert_with(key, || compute())
cache.cleanup_expired()
cache.stats()
```

#### Resilience

```rust
// Circuit Breaker
CircuitBreaker::new(config)
breaker.call(|| async { ... }).await
breaker.state()

// Retry
retry(config, || async { ... }).await
RetryConfig::builder()
    .max_attempts(5)
    .build()
```

#### Lifecycle

```rust
// Manager
#[async_trait]
impl AsyncManager for MyManager { ... }

// Controller
ManagerController::new()
controller.add_manager(manager)
controller.initialize_all().await
controller.shutdown_all().await
```

### Troubleshooting Guide

#### Common Errors and Solutions

**Error**: `#![feature] may not be used on the stable release channel`
- **Solution**: Update `rust-toolchain.toml` to use nightly for features, or remove unstable features

**Error**: `CircuitBreakerOpen`
- **Solution**: Circuit breaker detected repeated failures. Wait for recovery timeout or check backend health

**Error**: `RateLimitExceeded`
- **Solution**: Implement exponential backoff and check `retry_after()` duration

**Error**: `Lock timeout`
- **Solution**: Review lock ordering, use shorter critical sections, or increase timeout

**Error**: `NoRefreshToken`
- **Solution**: OAuth provider didn't issue refresh token. Check if `offline_access` scope is included

#### Deadlock Debugging

```rust
// Enable lock tracing (dev builds)
RUST_LOG=pulsearc_common::lifecycle=trace cargo run

// Use timeouts in production
let result = state.read_timeout(Duration::from_secs(5)).await;
match result {
    Ok(guard) => { /* use guard */ }
    Err(_) => {
        tracing::error!("Lock timeout - possible deadlock");
        // Trigger alert
    }
}
```

#### Performance Issues

```rust
// Profile cache hit rates
if let Some(stats) = cache.stats() {
    if stats.hit_rate() < 0.8 {
        tracing::warn!("Low cache hit rate: {:.1}%", stats.hit_rate() * 100.0);
        // Consider increasing cache size or adjusting TTL
    }
}

// Monitor circuit breaker
let metrics = breaker.metrics();
if metrics.failure_rate() > 0.5 {
    tracing::error!("High failure rate: {:.1}%", metrics.failure_rate() * 100.0);
    // Check backend health
}
```

#### Memory Leaks

```rust
// Use Arc::strong_count to detect leaks
let strong_count = Arc::strong_count(&shared_resource);
if strong_count > expected {
    tracing::warn!("Potential memory leak: {} references", strong_count);
}

// Clear caches periodically
cache.clear();
```

### FAQ

**Q: When should I use `Cache` vs `collections::LruCache`?**  
A: Use `cache::Cache` for application-level caching with TTL, metrics, and async support. Use `collections::LruCache` for low-level, synchronous caching without TTL.

**Q: Should I use `resilience::retry` or `sync::retry`?**  
A: Use `resilience::retry` for new modules needing generic retry logic. Use `sync::retry` when working within the sync/queue domain where you need integrated metrics and tracing.

**Q: How do I choose between sync and async cache?**  
A: Use `AsyncCache` in async contexts (tokio runtime). Use synchronous `Cache` in sync contexts or when you need to cache in non-async code.

**Q: What's the difference between `CommonError` and `AppError`?**  
A: `CommonError` is for infrastructure-level errors (timeouts, locks, config). `AppError` (in observability) is for application-level errors including AI, HTTP, and UI-specific errors.

**Q: How do I enable logging from common utilities?**  
A: Enable the `observability` feature and initialize tracing in your application:
```rust
use tracing_subscriber;
tracing_subscriber::fmt::init();
```

**Q: Can I use multiple feature tiers?**  
A: Yes! Features are additive. For example, `features = ["foundation", "cache", "time"]` gives you foundation + specific runtime modules.

**Q: How do I test code that uses keychain?**  
A: Use `MockKeychainProvider` from the testing module:
```rust
use pulsearc_common::testing::mocks::MockKeychainProvider;
let keychain = MockKeychainProvider::new();
```

**Q: What's the performance impact of error classification?**  
A: Minimal - classification methods are simple match statements. The main cost is in error creation, not classification.

**Q: How often should encryption keys be rotated?**  
A: Industry standard is 30-90 days for data encryption keys. Configure with `KeyRotationSchedule`.

**Q: Can I use common crates without Tokio?**  
A: Yes, use only `foundation` feature for sync-only utilities. Runtime features require Tokio.

---

## Conclusion

The PulseArc common crates provide a comprehensive foundation for building robust, observable, and maintainable Rust applications. By following the patterns and practices outlined in this guide, you can leverage battle-tested components while maintaining the flexibility to extend and customize for your specific needs.

### Key Takeaways

1. **Start with Foundation**: Begin with foundation tier and add features as needed
2. **Compose Errors**: Use `CommonError` for infrastructure, domain errors for business logic
3. **Add Resilience**: Use circuit breakers and retry patterns for external dependencies
4. **Observe Everything**: Leverage observability traits for metrics and audit logging
5. **Test Thoroughly**: Use provided test utilities and mocks for comprehensive testing
6. **Secure by Default**: Follow security best practices for secrets and PII
7. **Plan for Scale**: Consider performance implications and use appropriate data structures

### Getting Help

- **Documentation**: Check module-specific READMEs in `crates/common/src/`
- **Examples**: Review integration tests in `crates/common/tests/`
- **Issues**: Report bugs or request features in the project repository
- **Contributing**: Follow guidelines in `docs/` before submitting changes

### Version History

- **1.0** (2025-10-31): Initial comprehensive guide

---

**License**: Dual-licensed under Apache-2.0 and MIT  
**Maintainers**: PulseArc Team  
**Last Updated**: October 31, 2025

