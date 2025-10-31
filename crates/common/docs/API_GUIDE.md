# PulseArc Common Crate - Comprehensive API Guide

**Version:** 1.0
**Last Updated:** October 30, 2025

## Table of Contents

1. [Introduction](#introduction)
2. [Feature Tier System](#feature-tier-system)
3. [Getting Started](#getting-started)
4. [Foundation Tier](#foundation-tier)
   - [Error Handling](#error-handling)
   - [Validation Framework](#validation-framework)
   - [Utilities](#utilities)
   - [Collections](#collections)
5. [Runtime Tier](#runtime-tier)
   - [Cache](#cache)
   - [Cryptography](#cryptography)
   - [Privacy](#privacy)
   - [Time Utilities](#time-utilities)
   - [Resilience Patterns](#resilience-patterns)
   - [Sync Infrastructure](#sync-infrastructure)
   - [Lifecycle Management](#lifecycle-management)
   - [Observability](#observability)
6. [Platform Tier](#platform-tier)
   - [Authentication](#authentication)
   - [Security](#security)
   - [Storage](#storage)
   - [Compliance](#compliance)
7. [Testing Utilities](#testing-utilities)
8. [Common Patterns](#common-patterns)
9. [Migration Guide](#migration-guide)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

The `pulsearc-common` crate provides shared utilities and infrastructure for the PulseArc Rust workspace. It follows a **tiered feature system** that allows you to opt-in to only the functionality you need, minimizing dependencies and compilation time.

### Key Design Principles

- **Opt-in Features**: No default features - choose what you need
- **Type Safety**: Strong typing with compile-time guarantees
- **Composability**: Components work together seamlessly
- **Observability**: Built-in metrics and health checks
- **Resilience**: Automatic retry and failure protection
- **Thread Safety**: Safe concurrent access patterns
- **No Unsafe Code**: `#![forbid(unsafe_code)]` enforced

---

## Feature Tier System

The crate is organized into three tiers, each building on the previous one:

### Foundation Tier (`foundation`)

**Purpose:** Core utilities without side effects
**Dependencies:** Minimal (no logging, no async runtime)
**Use When:** You need basic utilities in a library or minimal environment

**Includes:**
- `error` - Error handling infrastructure
- `validation` - Field validators and validation framework
- `utils` - Macros and serialization helpers
- `collections` - Specialized data structures

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["foundation"] }
```

### Runtime Tier (`runtime`)

**Purpose:** Async infrastructure with observability
**Dependencies:** Includes `foundation` + `tokio` + `tracing`
**Use When:** Building async services or components

**Includes:**
- Everything from `foundation`
- `cache` - Thread-safe caching with TTL
- `crypto` - AES-256-GCM encryption primitives
- `privacy` - Data hashing and pattern detection
- `time` - Duration formatting, intervals, timers
- `resilience` - Circuit breakers and retry logic
- `sync` - Message queue with persistence
- `lifecycle` - Component lifecycle management
- `observability` - Metrics and error reporting

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["runtime"] }
```

### Platform Tier (`platform`)

**Purpose:** Platform integrations (keychain, OAuth, database)
**Dependencies:** Includes `runtime` + platform-specific libs
**Use When:** Building full applications with storage and auth

**Includes:**
- Everything from `runtime`
- `auth` - OAuth 2.0 + PKCE implementation
- `security` - Keychain, RBAC, key management
- `storage` - SQLCipher integration
- `compliance` - Audit logging and feature flags

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["platform"] }
```

### Special Features

#### `observability`

Opt-in feature that enables the `tracing` dependency for structured logging. The `observability` module is available in the `runtime` tier, but the `tracing` dependency is only added if you explicitly enable this feature.

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["runtime", "observability"] }
```

#### `test-utils`

Testing utilities for writing robust tests with mocks and fixtures.

```toml
[dev-dependencies]
pulsearc-common = { workspace = true, features = ["runtime", "test-utils"] }
```

---

## Getting Started

### Basic Setup

1. **Add to Cargo.toml** with the appropriate feature tier:

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["runtime"] }
```

2. **Import types you need**:

```rust
use pulsearc_common::{
    CommonError, CommonResult,
    Cache, CacheConfig,
    CircuitBreaker, RetryExecutor,
};
```

3. **Start using the APIs**:

```rust
fn example() -> CommonResult<()> {
    let cache = Cache::new(CacheConfig::lru(100));
    cache.insert("key".to_string(), "value".to_string());
    Ok(())
}
```

### Import Patterns

The crate re-exports commonly used types at the root for convenience:

```rust
// ✅ Recommended: Use root re-exports
use pulsearc_common::{CommonError, Validator, CircuitBreaker};

// ✅ Also valid: Direct module imports
use pulsearc_common::error::CommonError;
use pulsearc_common::validation::Validator;
use pulsearc_common::resilience::CircuitBreaker;
```

---

## Foundation Tier

### Error Handling

The error handling system provides three key components:

1. **`CommonError`**: Standard error patterns (timeouts, rate limiting, etc.)
2. **`ErrorClassification` trait**: Interface for error classification
3. **`ErrorSeverity` enum**: Severity levels for monitoring

#### CommonError

A comprehensive enum covering common error patterns that appear across modules.

**Type Alias:**
```rust
pub type CommonResult<T> = Result<T, CommonError>;
```

**Common Variants:**

| Variant | Use Case | Retryable | Severity |
|---------|----------|-----------|----------|
| `Timeout` | Operation deadlines | ✅ | Warning |
| `RateLimitExceeded` | API quotas | ✅ | Warning |
| `CircuitBreakerOpen` | Service protection | ✅ | Warning |
| `Lock` | Mutex contention | ✅ | Warning |
| `Serialization` | JSON/TOML parsing | ❌ | Error |
| `Validation` | Input validation | ❌ | Error |
| `Config` | Invalid settings | ❌ | Error |
| `Storage`/`Persistence` | File/DB I/O | ✅ | Error |
| `Backend` | External services | ✅ | Error |
| `Unauthorized` | Auth failures | ❌ | Warning |
| `NotFound` | Missing resources | ❌ | Info |
| `Internal` | Bugs/invariants | ❌ | Critical |

**Creating Errors:**

```rust
use pulsearc_common::{CommonError, CommonResult};

fn example() -> CommonResult<String> {
    // Timeout with custom duration
    return Err(CommonError::timeout(Duration::from_secs(30)));

    // Rate limiting with retry hint
    return Err(CommonError::rate_limit_with_retry(
        "API quota exceeded".to_string(),
        Duration::from_secs(60),
    ));

    // Configuration error
    return Err(CommonError::config("Invalid setting: max_connections"));

    // Storage error
    return Err(CommonError::storage("Failed to write file"));

    // Not found (informational)
    return Err(CommonError::not_found("user", "123"));
}
```

#### ErrorClassification Trait

All errors should implement this trait for consistent handling:

```rust
pub trait ErrorClassification {
    fn is_retryable(&self) -> bool;
    fn severity(&self) -> ErrorSeverity;
    fn is_critical(&self) -> bool;
    fn retry_after(&self) -> Option<Duration>;
}
```

**Implementing for Custom Errors:**

```rust
use pulsearc_common::{
    CommonError, ErrorClassification, ErrorSeverity
};
use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum MyModuleError {
    #[error("Module-specific error: {0}")]
    Specific(String),

    #[error(transparent)]
    Common(#[from] CommonError),
}

impl ErrorClassification for MyModuleError {
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

#### ErrorSeverity Levels

```rust
pub enum ErrorSeverity {
    Info,     // Informational (e.g., resource not found)
    Warning,  // Degraded but operational (e.g., rate limiting)
    Error,    // Failure requiring attention (e.g., network errors)
    Critical, // System integrity at risk (e.g., data corruption)
}
```

**Using Severity for Monitoring:**

```rust
use pulsearc_common::{ErrorClassification, ErrorSeverity};

fn handle_error<E: ErrorClassification>(error: E) {
    match error.severity() {
        ErrorSeverity::Critical => {
            // Alert on-call engineer
            send_pagerduty_alert(&error);
        }
        ErrorSeverity::Error => {
            // Log to error tracking
            log_to_sentry(&error);
        }
        ErrorSeverity::Warning => {
            // Log warning
            tracing::warn!(?error, "Degraded operation");
        }
        ErrorSeverity::Info => {
            // Debug logging
            tracing::debug!(?error, "Informational error");
        }
    }
}
```

#### Best Practices

**✅ DO:**
- Use `CommonError` for standard patterns
- Implement `ErrorClassification` for all error types
- Compose module errors with `CommonError` using `#[from]`
- Bubble errors upward with `?` operator
- Use `thiserror` for custom error types

**❌ DON'T:**
- Use `unwrap()` or `expect()` in production code
- Swallow errors silently
- Use `panic!()` for recoverable errors
- Duplicate `CommonError` variants in custom errors
- Use `anyhow` in library code (only at app boundaries)

---

### Validation Framework

Enterprise-grade validation with field-level errors and composable rules.

#### Core Types

**Validator** - Main validation coordinator:

```rust
use pulsearc_common::validation::{Validator, ValidationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct UserInput {
    email: String,
    age: u32,
    username: String,
}

fn validate_user(input: &UserInput) -> ValidationResult<()> {
    let mut validator = Validator::new();

    // String validation
    validator.string_field("email", &input.email)
        .required()
        .email()
        .max_length(255);

    // Range validation
    validator.range_field("age", input.age)
        .min(18)
        .max(120);

    // Custom validation
    validator.string_field("username", &input.username)
        .required()
        .min_length(3)
        .max_length(50)
        .matches(r"^[a-zA-Z0-9_]+$", "Username must be alphanumeric");

    validator.validate()
}
```

**Pre-built Validators:**

```rust
use pulsearc_common::validation::{
    EmailValidator, UrlValidator, IpValidator,
    StringValidator, RangeValidator, CollectionValidator
};

// Email validation
let email_validator = EmailValidator::new();
email_validator.validate("user@example.com")?;

// URL validation
let url_validator = UrlValidator::new()
    .require_https()
    .allowed_hosts(vec!["example.com".to_string()]);
url_validator.validate("https://example.com/path")?;

// IP address validation
let ip_validator = IpValidator::new().ipv4_only();
ip_validator.validate("192.168.1.1")?;

// String validation
let string_validator = StringValidator::new()
    .min_length(5)
    .max_length(100)
    .pattern(r"^\w+$");
string_validator.validate("hello_world")?;

// Range validation
let range_validator = RangeValidator::new()
    .min(0)
    .max(100);
range_validator.validate(&50)?;

// Collection validation
let collection_validator = CollectionValidator::new()
    .min_items(1)
    .max_items(10)
    .unique_items();
let items = vec![1, 2, 3];
collection_validator.validate(&items)?;
```

#### RuleSet - Complex Validation Logic

```rust
use pulsearc_common::validation::{RuleSet, RuleBuilder, CustomValidator};

// Define custom validation rules
let rules = RuleSet::new()
    .add_rule("password_strength",
        CustomValidator::new(|value: &String| {
            let has_uppercase = value.chars().any(|c| c.is_uppercase());
            let has_lowercase = value.chars().any(|c| c.is_lowercase());
            let has_number = value.chars().any(|c| c.is_numeric());
            let has_special = value.chars().any(|c| "!@#$%^&*".contains(c));

            if has_uppercase && has_lowercase && has_number && has_special {
                Ok(())
            } else {
                Err("Password must contain uppercase, lowercase, number, and special character".to_string())
            }
        })
    );

// Use the rule set
rules.validate("field", "MyP@ssw0rd")?;
```

#### ValidationError

```rust
use pulsearc_common::validation::{ValidationError, FieldError};

// Create validation error
let mut error = ValidationError::new();
error.add_field_error("email", "Invalid email format");
error.add_error_with_code("age", "Must be 18 or older", "AGE_MINIMUM");

// Check for errors
if !error.is_empty() {
    println!("Found {} validation errors", error.error_count());

    // Get errors for specific field
    for field_error in error.field_errors("email") {
        println!("Email error: {}", field_error.message);
    }

    return error.to_result();
}
```

---

### Utilities

#### Macros

**`bail_common!`** - Early return with `CommonError`:

```rust
use pulsearc_common::{bail_common, CommonResult};

fn check_input(value: i32) -> CommonResult<()> {
    if value < 0 {
        bail_common!(validation, "Value must be non-negative");
    }
    Ok(())
}
```

**`ensure_common!`** - Assert with `CommonError`:

```rust
use pulsearc_common::{ensure_common, CommonResult};

fn process(value: Option<i32>) -> CommonResult<i32> {
    ensure_common!(value.is_some(), not_found, "Value", "N/A");
    Ok(value.unwrap())
}
```

#### Serde Helpers

**Duration Serialization:**

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;
use pulsearc_common::duration_millis;

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(with = "duration_millis")]
    timeout: Duration,
}

// Serializes to: {"timeout": 5000} (milliseconds)
let config = Config {
    timeout: Duration::from_secs(5),
};
```

---

### Collections

Specialized high-performance data structures.

#### BoundedQueue

Async bounded queue with backpressure:

```rust
use pulsearc_common::collections::BoundedQueue;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = BoundedQueue::new(100);

    // Push with blocking
    queue.push(42).await?;

    // Try push with timeout
    queue.try_push_timeout(43, Duration::from_secs(1)).await?;

    // Pop item
    let item = queue.pop().await?;

    // Try pop with timeout
    if let Some(item) = queue.try_pop_timeout(Duration::from_millis(100)).await {
        println!("Got item: {}", item);
    }

    // Check size
    println!("Queue size: {}/{}", queue.len(), queue.capacity());

    Ok(())
}
```

#### LRU Cache

Least-recently-used cache:

```rust
use pulsearc_common::collections::LruCache;

let mut cache = LruCache::new(100);

// Insert items
cache.put("key1".to_string(), "value1".to_string());
cache.put("key2".to_string(), "value2".to_string());

// Get items (updates LRU order)
if let Some(value) = cache.get(&"key1".to_string()) {
    println!("Found: {}", value);
}

// Check size
println!("Cache size: {}/{}", cache.len(), cache.capacity());
```

#### RingBuffer

Fixed-size circular buffer:

```rust
use pulsearc_common::collections::RingBuffer;

let mut buffer = RingBuffer::new(5);

// Push items (overwrites oldest when full)
buffer.push(1);
buffer.push(2);
buffer.push(3);

// Iterate over items
for item in buffer.iter() {
    println!("Item: {}", item);
}

// Get by index
if let Some(item) = buffer.get(0) {
    println!("First item: {}", item);
}
```

#### BloomFilter

Probabilistic membership testing:

```rust
use pulsearc_common::collections::BloomFilter;

// Create with expected items and false positive rate
let mut filter = BloomFilter::new(1000, 0.01);

// Insert items
filter.insert("item1");
filter.insert("item2");

// Check membership
if filter.contains("item1") {
    println!("Probably contains item1");
}

if !filter.contains("item3") {
    println!("Definitely does not contain item3");
}
```

#### Trie

Prefix tree for efficient string matching:

```rust
use pulsearc_common::collections::Trie;

let mut trie = Trie::new();

// Insert strings
trie.insert("apple");
trie.insert("application");
trie.insert("apply");

// Search
assert!(trie.contains("apple"));
assert!(!trie.contains("app")); // Prefix but not complete word

// Find all words with prefix
let words = trie.words_with_prefix("app");
// Returns: ["apple", "application", "apply"]
```

#### Priority Queue

Min-heap and max-heap implementations:

```rust
use pulsearc_common::collections::{MinHeap, MaxHeap};

// Min-heap (smallest element first)
let mut min_heap = MinHeap::new();
min_heap.push(5);
min_heap.push(3);
min_heap.push(7);
assert_eq!(min_heap.pop(), Some(3));

// Max-heap (largest element first)
let mut max_heap = MaxHeap::new();
max_heap.push(5);
max_heap.push(3);
max_heap.push(7);
assert_eq!(max_heap.pop(), Some(7));
```

---

## Runtime Tier

### Cache

Thread-safe caching with TTL, eviction policies, and metrics.

#### Basic Usage

```rust
use pulsearc_common::cache::{Cache, CacheConfig};

// Simple LRU cache
let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));

cache.insert("key".to_string(), 42);
assert_eq!(cache.get(&"key".to_string()), Some(42));

// Check if key exists
assert!(cache.contains_key(&"key".to_string()));

// Remove item
cache.remove(&"key".to_string());

// Clear all
cache.clear();
```

#### TTL-based Cache

```rust
use std::time::Duration;
use pulsearc_common::cache::{Cache, CacheConfig};

// Cache with 1-hour TTL
let cache: Cache<String, String> =
    Cache::new(CacheConfig::ttl(Duration::from_secs(3600)));

cache.insert("session".to_string(), "user_data".to_string());

// Item expires after 1 hour
tokio::time::sleep(Duration::from_secs(3601)).await;
assert_eq!(cache.get(&"session".to_string()), None);
```

#### Combined TTL + LRU

```rust
use std::time::Duration;
use pulsearc_common::cache::{Cache, CacheConfig};

// Cache with TTL and size limit
let cache: Cache<String, Vec<u8>> =
    Cache::new(CacheConfig::ttl_lru(
        Duration::from_secs(300),
        1000
    ));
```

#### Custom Configuration

```rust
use std::time::Duration;
use pulsearc_common::cache::{Cache, CacheConfig, EvictionPolicy};

let config = CacheConfig::builder()
    .max_size(500)
    .ttl(Duration::from_secs(1800))
    .eviction_policy(EvictionPolicy::LFU) // Least Frequently Used
    .track_metrics(true)
    .build();

let cache: Cache<String, i32> = Cache::new(config);
```

#### Eviction Policies

```rust
use pulsearc_common::cache::EvictionPolicy;

// LRU - Least Recently Used (default)
let policy = EvictionPolicy::LRU;

// LFU - Least Frequently Used
let policy = EvictionPolicy::LFU;

// FIFO - First In First Out
let policy = EvictionPolicy::FIFO;

// Random - Random eviction
let policy = EvictionPolicy::Random;

// None - No automatic eviction (manual only)
let policy = EvictionPolicy::None;
```

#### Lazy Computation

```rust
use pulsearc_common::cache::{Cache, CacheConfig};

let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));

// Compute value only if not in cache
let value = cache.get_or_insert_with("expensive_key".to_string(), || {
    // Expensive computation
    expensive_function()
});

fn expensive_function() -> i32 {
    // Simulate expensive operation
    std::thread::sleep(std::time::Duration::from_secs(1));
    42
}
```

#### Cache Statistics

```rust
use pulsearc_common::cache::{Cache, CacheConfig};

let config = CacheConfig::builder()
    .max_size(100)
    .track_metrics(true)
    .build();

let cache: Cache<String, i32> = Cache::new(config);

// Use the cache
cache.insert("key1".to_string(), 1);
cache.get(&"key1".to_string());
cache.get(&"missing".to_string());

// Get statistics
let stats = cache.stats();
println!("Hits: {}", stats.hits);
println!("Misses: {}", stats.misses);
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Evictions: {}", stats.evictions);
```

---

### Cryptography

AES-256-GCM encryption primitives.

#### EncryptionService

```rust
use pulsearc_common::crypto::{EncryptionService, EncryptedData};

// Create service
let service = EncryptionService::new()?;

// Encrypt data
let plaintext = b"sensitive data";
let encrypted: EncryptedData = service.encrypt(plaintext)?;

// Decrypt data
let decrypted = service.decrypt(&encrypted)?;
assert_eq!(decrypted, plaintext);
```

#### Working with Keys

```rust
use pulsearc_common::crypto::EncryptionService;

// Create with specific key
let key = b"32-byte-key-for-aes256-gcm-here"; // Must be 32 bytes
let service = EncryptionService::from_key(key)?;

// Or generate random key
let service = EncryptionService::new()?;
let key = service.key();
```

---

### Privacy

Data hashing and pattern detection for privacy-preserving operations.

#### Secure Hashing

```rust
use pulsearc_common::privacy::{SecureHasher, HashConfig, HashAlgorithm};

// Create hasher with configuration
let config = HashConfig::new()?
    .with_algorithm(HashAlgorithm::Sha256);
let hasher = SecureHasher::with_config(config)?;

// Hash domain names
let hash = hasher.hash_domain("example.com")?;
println!("Hashed domain: {}", hash);
```

#### PII Detection

```rust
use pulsearc_common::privacy::{PatternMatcher, PiiDetectionConfig};

// Create pattern matcher
let config = PiiDetectionConfig::default();
let matcher = PatternMatcher::new(config);

// Detect PII in text
let text = "My email is user@example.com and SSN is 123-45-6789";
let findings = matcher.detect(text)?;

for finding in findings {
    println!("Found {} at position {}", finding.pii_type, finding.start);
}
```

---

### Time Utilities

Comprehensive time handling with formatting, parsing, and abstractions.

#### Duration Formatting

```rust
use std::time::Duration;
use pulsearc_common::time::format_duration;

let duration = Duration::from_secs(3665);
let formatted = format_duration(duration);
assert_eq!(formatted, "1h 1m 5s");
```

#### Duration Parsing

```rust
use pulsearc_common::time::parse_duration;

let duration = parse_duration("2h 30m 15s")?;
assert_eq!(duration.as_secs(), 9015);

// Supports various formats
let d1 = parse_duration("1h")?;         // 1 hour
let d2 = parse_duration("30m")?;        // 30 minutes
let d3 = parse_duration("45s")?;        // 45 seconds
let d4 = parse_duration("1h 30m 45s")?; // Combined
```

#### Intervals

```rust
use std::time::Duration;
use pulsearc_common::time::{Interval, IntervalConfig};

// Create interval with jitter
let config = IntervalConfig::new(Duration::from_secs(60))
    .with_jitter_percent(10); // ±10% jitter

let mut interval = Interval::new(config);

loop {
    interval.tick().await;
    // Execute periodic task
    println!("Tick!");
}
```

#### Timers

```rust
use std::time::Duration;
use pulsearc_common::time::Timer;

// One-shot timer
let timer = Timer::after(Duration::from_secs(5));
timer.await;
println!("5 seconds elapsed!");

// Recurring timer
let mut timer = Timer::recurring(Duration::from_secs(10));
loop {
    timer.tick().await;
    println!("10 seconds elapsed!");
}
```

#### Cron Expressions

```rust
use pulsearc_common::time::{CronExpression, CronSchedule};
use chrono::Utc;

// Parse cron expression
let expr = CronExpression::parse("0 0 * * *")?; // Daily at midnight

// Create schedule
let schedule = CronSchedule::new(expr);

// Get next occurrence
let now = Utc::now();
if let Some(next) = schedule.next_after(now) {
    println!("Next run: {}", next);
}

// Check if time matches
if schedule.matches(now) {
    println!("Time to run!");
}
```

#### Clock Abstraction

```rust
use pulsearc_common::time::{Clock, SystemClock, MockClock};
use std::time::Duration;

// Production: Use system clock
let clock = SystemClock;
let now = clock.now();

// Testing: Use mock clock
let clock = MockClock::new();
let before = clock.now();
clock.advance(Duration::from_secs(60));
let after = clock.now();
assert_eq!(after.duration_since(before).unwrap(), Duration::from_secs(60));
```

---

### Resilience Patterns

Generic fault tolerance patterns for building robust systems.

#### Circuit Breaker

Prevents cascading failures by detecting and stopping repeated failures.

**Basic Usage:**

```rust
use pulsearc_common::resilience::{CircuitBreaker, CircuitBreakerConfig};

// Create circuit breaker
let config = CircuitBreakerConfig::builder()
    .failure_threshold(5)      // Open after 5 failures
    .success_threshold(2)      // Close after 2 successes
    .timeout_duration(Duration::from_secs(60))
    .build()?;

let breaker = CircuitBreaker::new(config);

// Execute operation through breaker
let result = breaker.call(|| async {
    external_api_call().await
}).await;

match result {
    Ok(value) => println!("Success: {:?}", value),
    Err(e) => eprintln!("Failed: {}", e),
}
```

**Circuit States:**

```rust
use pulsearc_common::resilience::{CircuitBreaker, CircuitState};

let breaker = CircuitBreaker::new(config);

match breaker.state() {
    CircuitState::Closed => {
        // Normal operation - all requests pass through
    }
    CircuitState::Open => {
        // Failures detected - requests fail immediately
    }
    CircuitState::HalfOpen => {
        // Testing if service recovered - limited requests allowed
    }
}
```

**Monitoring:**

```rust
let metrics = breaker.metrics();
println!("Total calls: {}", metrics.total_calls);
println!("Failures: {}", metrics.failure_count);
println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
```

#### Retry Logic

Configurable retry with exponential backoff and jitter.

**Basic Retry:**

```rust
use pulsearc_common::resilience::{retry, RetryConfig, BackoffStrategy};

let config = RetryConfig::builder()
    .max_attempts(3)
    .backoff_strategy(BackoffStrategy::exponential(
        Duration::from_millis(100),  // Initial delay
        2.0,                          // Multiplier
        Duration::from_secs(10),      // Max delay
    ))
    .build()?;

let result = retry(config, || async {
    risky_operation().await
}).await?;
```

**Custom Retry Policy:**

```rust
use pulsearc_common::resilience::{
    retry_with_policy, RetryPolicy, RetryDecision, RetryContext
};

struct CustomPolicy {
    max_attempts: usize,
}

impl<E> RetryPolicy<E> for CustomPolicy
where
    E: std::error::Error,
{
    fn should_retry(&self, ctx: &RetryContext<E>) -> RetryDecision {
        if ctx.attempt >= self.max_attempts {
            return RetryDecision::DoNotRetry;
        }

        // Retry only on specific errors
        if let Some(error) = &ctx.last_error {
            if error.to_string().contains("timeout") {
                return RetryDecision::RetryAfter(Duration::from_secs(1));
            }
        }

        RetryDecision::DoNotRetry
    }
}

let policy = CustomPolicy { max_attempts: 5 };
let result = retry_with_policy(policy, || async {
    fallible_operation().await
}).await?;
```

**Backoff Strategies:**

```rust
use pulsearc_common::resilience::{BackoffStrategy, Jitter};
use std::time::Duration;

// Exponential backoff: 100ms, 200ms, 400ms, ...
let backoff = BackoffStrategy::exponential(
    Duration::from_millis(100),
    2.0,
    Duration::from_secs(10),
);

// Linear backoff: 100ms, 200ms, 300ms, ...
let backoff = BackoffStrategy::linear(
    Duration::from_millis(100),
    Duration::from_secs(5),
);

// Constant delay: 1s, 1s, 1s, ...
let backoff = BackoffStrategy::constant(Duration::from_secs(1));

// Immediate retry: 0ms, 0ms, 0ms, ...
let backoff = BackoffStrategy::immediate();

// Add jitter to prevent thundering herd
let jitter = Jitter::full();  // ±50% randomization
let jitter = Jitter::equal(); // ±50% of backoff
let jitter = Jitter::decorrelated(); // Decorrelated jitter
```

---

### Sync Infrastructure

Enterprise-grade message queue with persistence, compression, and encryption.

#### SyncQueue

```rust
use pulsearc_common::sync::{
    SyncQueue, QueueConfig, SyncItem, Priority
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create queue
    let config = QueueConfig::builder()
        .storage_path(PathBuf::from("./queue_data"))
        .max_items(10000)
        .enable_compression()
        .enable_encryption()
        .build()?;

    let queue = SyncQueue::new(config).await?;

    // Enqueue item
    let item = SyncItem::new("payload_data".to_string())
        .with_priority(Priority::High);
    queue.enqueue(item).await?;

    // Dequeue item
    if let Some(item) = queue.dequeue().await? {
        println!("Processing: {}", item.payload);

        // Mark as completed
        queue.mark_completed(&item.id).await?;
    }

    // Get metrics
    let metrics = queue.metrics();
    println!("Queue size: {}", metrics.total_items);
    println!("Completed: {}", metrics.completed_items);

    Ok(())
}
```

#### Retry Strategy

Domain-specific retry with circuit breaker integration:

```rust
use pulsearc_common::sync::{
    RetryStrategy, RetryPolicyBuilder, CircuitBreaker
};

let policy = RetryPolicyBuilder::default()
    .max_attempts(3)
    .initial_delay(Duration::from_millis(100))
    .max_delay(Duration::from_secs(5))
    .build();

let breaker = CircuitBreaker::new(/* config */);
let strategy = RetryStrategy::new(policy, breaker);

let result = strategy.execute(|| async {
    sync_operation().await
}).await?;
```

---

### Lifecycle Management

Standardized async component lifecycle with health checks.

#### AsyncManager Trait

```rust
use pulsearc_common::lifecycle::{
    AsyncManager, ManagerStatus, ManagerHealth, ManagerMetadata
};
use async_trait::async_trait;

struct MyService {
    status: ManagerStatus,
}

#[async_trait]
impl AsyncManager for MyService {
    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting service...");
        self.status = ManagerStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Stopping service...");
        self.status = ManagerStatus::Stopped;
        Ok(())
    }

    fn status(&self) -> ManagerStatus {
        self.status
    }

    async fn health(&self) -> ManagerHealth {
        ManagerHealth::healthy("Service is running")
    }

    fn metadata(&self) -> ManagerMetadata {
        ManagerMetadata {
            name: "MyService".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}
```

#### Shared State Management

```rust
use pulsearc_common::lifecycle::{SharedState, StateConfig};

#[derive(Clone, Debug)]
struct AppState {
    counter: i32,
}

// Create shared state
let state = SharedState::new(AppState { counter: 0 });

// Read state
let value = state.read(|s| s.counter).await;

// Write state
state.write(|s| {
    s.counter += 1;
}).await;

// With configuration
let config = StateConfig::builder()
    .max_read_contention(100)
    .max_write_wait(Duration::from_secs(5))
    .build();
let state = SharedState::with_config(AppState { counter: 0 }, config);
```

#### State Registry

```rust
use pulsearc_common::lifecycle::StateRegistry;

let registry = StateRegistry::new();

// Register state
registry.register("app_config", AppConfig::default()).await;
registry.register("user_session", UserSession::default()).await;

// Get state
if let Some(config) = registry.get::<AppConfig>("app_config").await {
    let value = config.read(|c| c.setting.clone()).await;
}

// List all registered states
let keys = registry.list().await;
for key in keys {
    println!("Registered state: {}", key);
}
```

---

### Observability

Monitoring, metrics, and error handling infrastructure.

#### Application Errors

```rust
use pulsearc_common::observability::{
    AppError, AppResult, ErrorCode, ActionHint
};

fn example() -> AppResult<String> {
    // Create error with code and hint
    Err(AppError::new(
        "Database connection failed",
        ErrorCode::DatabaseError,
        ActionHint::RetryLater,
    ))
}

// Convert to user-friendly error
let ui_error = app_error.to_ui_error();
println!("Message: {}", ui_error.message);
println!("Suggested action: {:?}", ui_error.action_hint);
```

#### AI-specific Errors

```rust
use pulsearc_common::observability::{AiError, ErrorCode};

fn call_ai_api() -> Result<String, AiError> {
    Err(AiError::api_key_missing())
}

// Check error type
match call_ai_api() {
    Err(AiError::ApiKeyMissing) => {
        println!("Please configure your API key");
    }
    Err(AiError::RateLimitExceeded { retry_after }) => {
        println!("Rate limited. Retry after: {:?}", retry_after);
    }
    Ok(response) => {
        println!("Success: {}", response);
    }
}
```

#### Metrics Tracking

```rust
use pulsearc_common::observability::{
    MetricsTracker, ClassificationMetrics, PerformanceMetrics
};

let mut tracker = MetricsTracker::new();

// Record classification
tracker.record_classification("work");
tracker.record_classification("personal");

// Record performance
tracker.record_duration(Duration::from_millis(150));
tracker.record_success();

// Get metrics
let classification = tracker.classification_metrics();
println!("Total classifications: {}", classification.total);
println!("Unique categories: {}", classification.unique_categories());

let performance = tracker.performance_metrics();
println!("Avg duration: {:?}", performance.average_duration());
println!("Success rate: {:.2}%", performance.success_rate() * 100.0);
```

---

## Platform Tier

### Authentication

OAuth 2.0 + PKCE implementation with token management and keychain storage.

#### OAuth Service

```rust
use pulsearc_common::auth::{OAuthService, OAuthConfig};
use pulsearc_common::security::KeychainProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OAuth
    let config = OAuthConfig::new(
        "auth0-tenant.us.auth0.com".to_string(),
        "client_id".to_string(),
        "http://localhost:8888/callback".to_string(),
        vec!["openid".to_string(), "profile".to_string()],
        Some("https://api.example.com".to_string()),
    );

    // Create service
    let keychain = Arc::new(KeychainProvider::new("MyApp".to_string()));
    let service = OAuthService::new(
        config,
        keychain,
        "MyApp.auth".to_string(),
        "main".to_string(),
        300, // Refresh 5 min before expiry
    );

    // Initialize (load existing tokens)
    service.initialize().await?;

    // Start login flow
    let (auth_url, state) = service.start_login().await?;
    println!("Open in browser: {}", auth_url);

    // Complete login (after user authorizes)
    let tokens = service.complete_login("auth_code", &state).await?;

    // Start auto-refresh
    service.start_auto_refresh();

    // Get access token (refreshes automatically if needed)
    let token = service.get_access_token().await?;
    println!("Token: {}", token);

    Ok(())
}
```

#### Token Management

```rust
use pulsearc_common::auth::{TokenManager, TokenSet, OAuthConfig};
use pulsearc_common::security::KeychainProvider;
use std::sync::Arc;

let keychain = Arc::new(KeychainProvider::new("MyApp".to_string()));
let manager = TokenManager::new(
    config,
    keychain,
    "MyApp.tokens".to_string(),
    "main".to_string(),
    300,
);

// Store tokens
let tokens = TokenSet {
    access_token: "access".to_string(),
    refresh_token: Some("refresh".to_string()),
    expires_in: 3600,
    token_type: "Bearer".to_string(),
    id_token: None,
};
manager.store_tokens(&tokens).await?;

// Get access token (auto-refreshes if expired)
let token = manager.get_access_token().await?;

// Check if expired
if manager.is_expired() {
    println!("Token expired, will refresh on next access");
}
```

#### PKCE Flow

```rust
use pulsearc_common::auth::PKCEChallenge;

// Generate PKCE challenge
let challenge = PKCEChallenge::generate()?;

println!("Code verifier: {}", challenge.verifier);
println!("Code challenge: {}", challenge.challenge);

// Verify challenge
assert!(challenge.verify(&challenge.verifier));
```

---

### Security

Keychain storage, RBAC, and encryption key management.

#### Keychain Provider

```rust
use pulsearc_common::security::{KeychainProvider, KeychainError};

// Create provider
let keychain = KeychainProvider::new("MyApp".to_string());

// Store credential
keychain.set_password(
    "api_token",
    "user@example.com",
    "secret_token_value",
)?;

// Retrieve credential
let password = keychain.get_password("api_token", "user@example.com")?;

// Delete credential
keychain.delete_password("api_token", "user@example.com")?;
```

#### RBAC (Role-Based Access Control)

```rust
use pulsearc_common::security::{
    RBACManager, Role, Permission, UserContext
};

// Create RBAC manager
let mut rbac = RBACManager::new();

// Define roles
let admin_role = Role::new("admin")
    .with_permission(Permission::new("users:read"))
    .with_permission(Permission::new("users:write"))
    .with_permission(Permission::new("users:delete"));

let user_role = Role::new("user")
    .with_permission(Permission::new("users:read"));

rbac.add_role(admin_role);
rbac.add_role(user_role);

// Assign roles to users
rbac.assign_role("user123", "user")?;
rbac.assign_role("admin456", "admin")?;

// Check permissions
let user_ctx = UserContext::new("user123".to_string());
if rbac.has_permission(&user_ctx, &Permission::new("users:read")).await? {
    println!("User can read users");
}

if !rbac.has_permission(&user_ctx, &Permission::new("users:delete")).await? {
    println!("User cannot delete users");
}
```

#### Dynamic RBAC Policies

```rust
use pulsearc_common::security::{
    RBACPolicy, PolicyCondition, PolicyEffect, UserContext
};
use std::collections::HashMap;

// Create policy with conditions
let mut attributes = HashMap::new();
attributes.insert("time".to_string(), "business_hours".to_string());

let policy = RBACPolicy::new(
    "business_hours_only",
    PolicyEffect::Allow,
    vec![Permission::new("data:read")],
    PolicyCondition::attribute("time", "business_hours"),
);

// Add to RBAC manager
rbac.add_policy(policy)?;

// Check with context
let mut user_ctx = UserContext::new("user123".to_string());
user_ctx.set_attribute("time", "business_hours");

if rbac.check_policy(&user_ctx, &Permission::new("data:read")).await? {
    println!("Access allowed during business hours");
}
```

---

### Storage

SQLCipher integration for encrypted databases.

#### SQLCipher Connection

```rust
use pulsearc_common::storage::{
    SqlCipherConnection, StorageConfig, KeySource
};

// Create configuration
let config = StorageConfig::builder()
    .database_path("./data.db")
    .key_source(KeySource::EnvVar("DB_KEY".to_string()))
    .build()?;

// Open connection
let conn = SqlCipherConnection::open(&config)?;

// Execute query
conn.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT)", [])?;

// Query rows
let mut stmt = conn.prepare("SELECT id, name FROM users")?;
let users = stmt.query_map([], |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
})?;

for user in users {
    let (id, name) = user?;
    println!("User {}: {}", id, name);
}
```

#### Connection Pool

```rust
use pulsearc_common::storage::{
    SqlCipherPool, SqlCipherPoolConfig, StorageConfig
};

// Create pool configuration
let storage_config = StorageConfig::builder()
    .database_path("./data.db")
    .key_source(KeySource::EnvVar("DB_KEY".to_string()))
    .build()?;

let pool_config = SqlCipherPoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(Duration::from_secs(30))
    .build();

// Create pool
let pool = SqlCipherPool::new(storage_config, pool_config)?;

// Get connection from pool
let conn = pool.get()?;

// Use connection
conn.execute("INSERT INTO users (id, name) VALUES (?1, ?2)", [1, "Alice"])?;

// Connection automatically returns to pool when dropped
```

#### Health Checks

```rust
let health = pool.health()?;
println!("Pool size: {}", health.pool_size);
println!("Active connections: {}", health.active_connections);
println!("Status: {:?}", health.status);
```

---

### Compliance

Audit logging, configuration management, and feature flags.

#### Audit Logging

```rust
use pulsearc_common::compliance::{
    GlobalAuditLogger, AuditEvent, AuditContext, AuditSeverity
};

// Get global logger
let logger = GlobalAuditLogger::instance();

// Create audit context
let context = AuditContext::new()
    .with_user_id("user123")
    .with_ip_address("192.168.1.100")
    .with_session_id("session456");

// Log audit event
let event = AuditEvent::new(
    "user_login",
    AuditSeverity::Info,
    context,
)
.with_metadata("login_method", "oauth");

logger.log(event)?;

// Log with convenience methods
logger.log_info("user_logout", context.clone())?;
logger.log_warning("suspicious_activity", context.clone())?;
logger.log_critical("data_breach_attempt", context)?;
```

#### Configuration Management

```rust
use pulsearc_common::compliance::{ConfigManager, RemoteConfig};

// Create config manager
let manager = ConfigManager::new("https://config.example.com")?;

// Fetch configuration
let config: RemoteConfig = manager.fetch("app_settings").await?;

// Get values
let max_retries: i32 = config.get("max_retries")?;
let api_endpoint: String = config.get("api_endpoint")?;

// Watch for updates
let receiver = manager.watch("app_settings").await?;
tokio::spawn(async move {
    while let Ok(updated_config) = receiver.recv().await {
        println!("Config updated: version {}", updated_config.version);
    }
});
```

#### Feature Flags

```rust
use pulsearc_common::compliance::{FeatureFlagManager, FeatureFlag};

// Create feature flag manager
let mut manager = FeatureFlagManager::new();

// Add feature flags
manager.add_flag(FeatureFlag::new("new_ui", false));
manager.add_flag(FeatureFlag::new("beta_features", true));

// Check if feature is enabled
if manager.is_enabled("new_ui") {
    println!("New UI is enabled");
}

// Enable/disable features
manager.enable("new_ui")?;
manager.disable("beta_features")?;

// Conditional feature checks
if manager.is_enabled_for_user("new_ui", "user123") {
    // User-specific feature enablement
}
```

---

## Testing Utilities

Comprehensive testing helpers for robust test suites.

### Mock Clock

```rust
use pulsearc_common::testing::{MockClock, Clock};
use std::time::Duration;

#[tokio::test]
async fn test_with_mock_time() {
    let clock = MockClock::new();

    let before = clock.now();

    // Advance time
    clock.advance(Duration::from_secs(60));

    let after = clock.now();
    assert_eq!(
        after.duration_since(before).unwrap(),
        Duration::from_secs(60)
    );
}
```

### Assertions

```rust
use pulsearc_common::testing::*;

#[test]
fn test_assertions() {
    // Approximate equality for floats
    assert_approx_eq!(3.14159, 3.14, 0.01);

    // Check collection contains all items
    let items = vec![1, 2, 3, 4, 5];
    assert_contains_all!(items, &[2, 4]);

    // Check duration in range
    let duration = Duration::from_millis(150);
    assert_duration_in_range!(
        duration,
        Duration::from_millis(100),
        Duration::from_millis(200)
    );

    // Check sorted
    let sorted = vec![1, 2, 3, 4, 5];
    assert_sorted!(sorted);
}
```

### Test Builders

```rust
use pulsearc_common::testing::TestBuilder;

#[derive(Debug)]
struct User {
    id: i64,
    name: String,
    email: String,
}

impl TestBuilder for User {
    fn test_default() -> Self {
        Self {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        }
    }
}

#[test]
fn test_user() {
    let user = User::test_default();
    assert_eq!(user.name, "Test User");

    let custom_user = User {
        name: "Custom".to_string(),
        ..User::test_default()
    };
    assert_eq!(custom_user.name, "Custom");
}
```

### Fixtures

```rust
use pulsearc_common::testing::*;

#[test]
fn test_fixtures() {
    // Random string
    let random = random_string(10);
    assert_eq!(random.len(), 10);

    // Random email
    let email = random_email();
    assert!(email.contains("@"));

    // Random number
    let num = random_u64();
    assert!(num > 0);
}
```

### Mocks

```rust
use pulsearc_common::testing::{MockHttpClient, MockStorage};

#[tokio::test]
async fn test_with_mocks() {
    // Mock HTTP client
    let mut client = MockHttpClient::new();
    client.expect_get("https://api.example.com/data")
        .returning(|| Ok("response data".to_string()));

    let response = client.get("https://api.example.com/data").await?;
    assert_eq!(response, "response data");

    // Mock storage
    let mut storage = MockStorage::new();
    storage.expect_write("key", "value")
        .returning(|| Ok(()));

    storage.write("key", "value").await?;
}
```

### Temporary Files

```rust
use pulsearc_common::testing::{TempDir, TempFile};

#[test]
fn test_temp_files() {
    // Temporary directory (auto-cleaned on drop)
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create file in temp dir
    let file_path = dir_path.join("test.txt");
    std::fs::write(&file_path, "content").unwrap();
    assert!(file_path.exists());

    // Temporary file (auto-cleaned on drop)
    let temp_file = TempFile::new().unwrap();
    let file_path = temp_file.path();
    std::fs::write(file_path, "data").unwrap();
    assert!(file_path.exists());
}
// Files and directories are automatically deleted here
```

### Async Test Utilities

```rust
use pulsearc_common::testing::*;
use std::time::Duration;

#[tokio::test]
async fn test_async_utils() {
    // Retry async operation
    let result = retry_async(3, || async {
        Ok::<_, String>(42)
    }).await;
    assert_eq!(result.unwrap(), 42);

    // Poll until condition is true
    let mut counter = 0;
    poll_until(Duration::from_millis(100), || async {
        counter += 1;
        counter >= 5
    }).await;
    assert!(counter >= 5);

    // Timeout with Ok result
    let result = timeout_ok(Duration::from_secs(1), async {
        42
    }).await;
    assert_eq!(result, Some(42));
}
```

---

## Common Patterns

### Error Handling Pattern

```rust
use pulsearc_common::{CommonError, CommonResult, ErrorClassification};
use thiserror::Error;

// Module-specific error
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service-specific: {0}")]
    Specific(String),

    #[error(transparent)]
    Common(#[from] CommonError),
}

impl ErrorClassification for ServiceError {
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

// Usage
fn service_operation() -> Result<(), ServiceError> {
    // Can return either error type
    if condition_a {
        return Err(ServiceError::Specific("reason".to_string()));
    }

    if condition_b {
        return Err(CommonError::timeout(Duration::from_secs(30)).into());
    }

    Ok(())
}
```

### Retry with Circuit Breaker

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig,
    retry, RetryConfig, BackoffStrategy,
};
use std::time::Duration;

async fn resilient_operation() -> Result<String, Box<dyn std::error::Error>> {
    // Create circuit breaker
    let breaker_config = CircuitBreakerConfig::builder()
        .failure_threshold(5)
        .timeout_duration(Duration::from_secs(60))
        .build()?;
    let breaker = CircuitBreaker::new(breaker_config);

    // Create retry config
    let retry_config = RetryConfig::builder()
        .max_attempts(3)
        .backoff_strategy(BackoffStrategy::exponential(
            Duration::from_millis(100),
            2.0,
            Duration::from_secs(10),
        ))
        .build()?;

    // Combine retry with circuit breaker
    let result = breaker.call(|| async {
        retry(retry_config.clone(), || async {
            external_api_call().await
        }).await
    }).await?;

    Ok(result)
}
```

### Caching with Validation

```rust
use pulsearc_common::{
    cache::{Cache, CacheConfig},
    validation::Validator,
};
use std::time::Duration;

struct UserService {
    cache: Cache<String, User>,
}

impl UserService {
    fn new() -> Self {
        Self {
            cache: Cache::new(CacheConfig::ttl_lru(
                Duration::from_secs(300),
                1000,
            )),
        }
    }

    async fn get_user(&self, id: &str) -> Result<User, Box<dyn std::error::Error>> {
        // Validate input
        let mut validator = Validator::new();
        validator.string_field("id", id)
            .required()
            .min_length(1)
            .max_length(50);
        validator.validate()?;

        // Check cache
        if let Some(user) = self.cache.get(&id.to_string()) {
            return Ok(user);
        }

        // Fetch from database
        let user = fetch_user_from_db(id).await?;

        // Store in cache
        self.cache.insert(id.to_string(), user.clone());

        Ok(user)
    }
}
```

### Managed Component Lifecycle

```rust
use pulsearc_common::lifecycle::{
    AsyncManager, ManagerStatus, ManagerHealth, SharedState
};
use async_trait::async_trait;

struct DataProcessor {
    status: ManagerStatus,
    state: SharedState<ProcessorState>,
}

#[derive(Clone)]
struct ProcessorState {
    processed_count: usize,
}

impl DataProcessor {
    fn new() -> Self {
        Self {
            status: ManagerStatus::Stopped,
            state: SharedState::new(ProcessorState { processed_count: 0 }),
        }
    }
}

#[async_trait]
impl AsyncManager for DataProcessor {
    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.status = ManagerStatus::Starting;

        // Initialize resources
        initialize_processor().await?;

        self.status = ManagerStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.status = ManagerStatus::Stopping;

        // Cleanup resources
        cleanup_processor().await?;

        self.status = ManagerStatus::Stopped;
        Ok(())
    }

    fn status(&self) -> ManagerStatus {
        self.status
    }

    async fn health(&self) -> ManagerHealth {
        let count = self.state.read(|s| s.processed_count).await;

        if count > 0 {
            ManagerHealth::healthy(&format!("Processed {} items", count))
        } else {
            ManagerHealth::degraded("No items processed yet")
        }
    }

    fn metadata(&self) -> ManagerMetadata {
        ManagerMetadata {
            name: "DataProcessor".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}
```

---

## Migration Guide

### From Local Error Types to CommonError

**Before:**
```rust
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error("Rate limit exceeded")]
    RateLimited,
}
```

**After:**
```rust
use pulsearc_common::{CommonError, ErrorClassification};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service-specific: {0}")]
    Specific(String),

    #[error(transparent)]
    Common(#[from] CommonError),
}

// Return CommonError variants
return Err(CommonError::timeout(Duration::from_secs(30)).into());
return Err(CommonError::rate_limit_exceeded("API quota").into());
```

### From Custom Validation to Validation Framework

**Before:**
```rust
fn validate_email(email: &str) -> Result<(), String> {
    if !email.contains('@') {
        return Err("Invalid email".to_string());
    }
    Ok(())
}
```

**After:**
```rust
use pulsearc_common::validation::{Validator, EmailValidator};

fn validate_user(email: &str) -> ValidationResult<()> {
    let mut validator = Validator::new();
    validator.string_field("email", email)
        .required()
        .email();
    validator.validate()
}
```

### From Manual Retry to RetryExecutor

**Before:**
```rust
async fn with_retry() -> Result<(), Error> {
    for attempt in 0..3 {
        match operation().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt < 2 => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

**After:**
```rust
use pulsearc_common::resilience::{retry, RetryConfig, BackoffStrategy};

async fn with_retry() -> Result<(), Error> {
    let config = RetryConfig::builder()
        .max_attempts(3)
        .backoff_strategy(BackoffStrategy::constant(Duration::from_secs(1)))
        .build()?;

    retry(config, || async {
        operation().await
    }).await
}
```

---

## Troubleshooting

### Feature Not Available

**Problem:** Module or type not found.

```rust
// Error: cannot find type `Cache` in this scope
use pulsearc_common::cache::Cache;
```

**Solution:** Enable the required feature tier:

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["runtime"] }
```

### Tracing Not Working

**Problem:** No tracing output even though using observability module.

**Solution:** Enable the `observability` feature flag:

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["runtime", "observability"] }
```

### Compilation Errors with Optional Dependencies

**Problem:** Dependency not available when feature is enabled.

**Solution:** Ensure feature dependencies are correctly specified in your `Cargo.toml`. Check that all required features are enabled:

```toml
[dependencies]
pulsearc-common = { workspace = true, features = ["platform"] }  # Includes runtime + foundation
```

### Mock Clock Not Advancing Time

**Problem:** Tests using `MockClock` are not seeing time changes.

**Solution:** Ensure you're using the same `MockClock` instance:

```rust
use pulsearc_common::testing::MockClock;

let clock = MockClock::new();
let service = MyService::with_clock(clock.clone());  // Pass same instance

clock.advance(Duration::from_secs(60));  // Now service sees time change
```

### Circuit Breaker Always Open

**Problem:** Circuit breaker immediately opens and never recovers.

**Solution:** Check your failure threshold and timeout configuration:

```rust
let config = CircuitBreakerConfig::builder()
    .failure_threshold(5)  // Need 5 failures before opening
    .success_threshold(2)  // Need 2 successes to close
    .timeout_duration(Duration::from_secs(60))  // Wait before trying again
    .build()?;
```

### Validation Errors Not Helpful

**Problem:** Validation errors lack context.

**Solution:** Use validation codes and metadata:

```rust
let mut error = ValidationError::new();
error.add_error_with_code(
    "email",
    "Email format is invalid",
    "EMAIL_INVALID"
);
```

### Performance Issues with Cache

**Problem:** Cache is slow or uses too much memory.

**Solution:** Adjust cache configuration:

```rust
let config = CacheConfig::builder()
    .max_size(1000)  // Limit size
    .ttl(Duration::from_secs(300))  // Add TTL
    .eviction_policy(EvictionPolicy::LRU)  // Use efficient eviction
    .track_metrics(false)  // Disable metrics if not needed
    .build();
```

---

## Additional Resources

- **CLAUDE.md**: Workspace rules and coding standards
- **Module READMEs**: Detailed documentation in each module directory
- **Integration Tests**: Examples in `crates/common/tests/`
- **Benchmarks**: Performance examples in `crates/common/benches/`

---

**End of API Guide**
