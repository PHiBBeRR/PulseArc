# Common

Shared utilities and infrastructure used across the agent codebase.

## Feature Flags

This crate uses **opt-in features** with no defaults enabled. Choose the tier you need:

- **`foundation`** - Core utilities without side effects (errors, validation, collections, privacy)
  - No logging, no async runtime, minimal dependencies
- **`runtime`** - Async infrastructure (cache, time, resilience, sync, lifecycle)
  - Includes foundation + tokio/futures + observability (tracing)
- **`platform`** - Platform integrations (auth, security, storage, compliance)
  - Includes runtime + platform-specific dependencies
- **`observability`** - Opt-in tracing/metrics (NOT included by default)
  - Adds `tracing` dependency for logging
  - Required if you want structured logging from common utilities

**Example `Cargo.toml`:**
```toml
# Minimal: just foundation utilities (no logging, no async)
pulsearc-common = { workspace = true, features = ["foundation"] }

# Runtime: async infrastructure with logging
pulsearc-common = { workspace = true, features = ["runtime"] }

# Full platform: all features
pulsearc-common = { workspace = true, features = ["platform"] }
```

## Overview

This module provides reusable components including error handling, configuration management, retry logic, circuit breakers, and common patterns used throughout the application.

## Directory Structure

```
agent/common/
├── auth/                       # Authentication primitives
│   ├── client.rs              # OAuth HTTP client
│   ├── mod.rs                 # Module exports
│   ├── pkce.rs                # PKCE challenge generation
│   ├── service.rs             # High-level orchestrator
│   ├── token_manager.rs       # Token lifecycle + auto-refresh
│   └── types.rs               # OAuth types (TokenSet, OAuthConfig)
├── cache/                      # Caching utilities with TTL and metrics
│   ├── async_core.rs          # Async cache implementation
│   ├── config.rs              # Cache configuration
│   ├── core.rs                # Synchronous cache
│   ├── mod.rs                 # Module exports
│   └── stats.rs               # Cache statistics
├── compliance/                 # Compliance primitives
│   ├── audit.rs               # Audit logging infrastructure
│   ├── config.rs              # Remote configuration management
│   ├── feature_flags.rs       # Feature flag system
│   └── mod.rs                 # Module exports
├── error/                      # Error handling infrastructure
│   ├── mod.rs                 # CommonError, ErrorClassification, ErrorSeverity
│   └── ERROR_MIGRATION_GUIDE.md # Error migration documentation
├── observability/              # Monitoring, metrics, and error handling
│   ├── errors/                # Structured error types
│   │   ├── app.rs             # Unified error types (AppError, AiError, etc.)
│   │   └── mod.rs             # Error exports
│   ├── metrics/               # Performance and classification metrics
│   │   ├── classification.rs  # Classification metrics tracking
│   │   └── mod.rs             # Metrics exports
│   ├── mod.rs                 # Module exports
│   └── README.md              # Observability documentation
├── patterns/                   # Reusable design patterns
│   ├── debounce/              # Context stability debouncing
│   │   ├── core.rs            # Debouncer implementation
│   │   ├── distributed/       # Distributed coordination backends
│   │   │   ├── etcd.rs        # Etcd backend
│   │   │   ├── memory.rs      # In-memory backend
│   │   │   ├── redis.rs       # Redis backend
│   │   │   ├── mod.rs         # Module exports
│   │   │   └── README.md      # Distributed coordination docs
│   │   ├── error.rs           # Debounce errors
│   │   ├── mod.rs             # Module exports
│   │   └── README.md          # Debounce documentation
│   └── mod.rs                 # Patterns module
├── resilience/                 # Fault tolerance patterns
│   ├── circuit_breaker.rs     # Circuit breaker implementation
│   ├── mod.rs                 # Re-exports
│   └── retry.rs               # Retry logic with backoff
├── security/                   # Security primitives
│   ├── rbac.rs                # Role-based access control
│   └── mod.rs                 # Security module
├── storage/                    # Encrypted database infrastructure
│   ├── encryption/            # Encryption and key management
│   │   ├── cache.rs           # Key caching
│   │   ├── cipher.rs          # SQLCipher integration
│   │   ├── key_rotation.rs    # Automated key rotation
│   │   ├── keychain.rs        # Platform keychain integration
│   │   ├── keys.rs            # Key generation and management
│   │   ├── mod.rs             # Module exports
│   │   ├── rotation.rs        # Rotation scheduling
│   │   ├── secure_string.rs   # Memory-safe string handling
│   │   └── README.md          # Encryption documentation
│   ├── sqlcipher/             # SQLCipher connection management
│   │   ├── config.rs          # SQLCipher configuration
│   │   ├── connection.rs      # Connection handling
│   │   ├── mod.rs             # Module exports
│   │   ├── pool.rs            # Connection pooling
│   │   ├── pragmas.rs         # SQLite pragma management
│   │   └── README.md          # SQLCipher documentation
│   └── mod.rs                 # Storage module exports
├── validation/                 # Enterprise validation framework
│   ├── mod.rs                 # ValidationError, Validator
│   ├── rules.rs               # Validation rules and rule sets
│   ├── validators.rs          # Pre-built validators
│   └── README.md              # Validation documentation
├── keychain.rs                 # Secure credential storage
├── macros.rs                   # Utility macros
├── manager.rs                  # Async component lifecycle management
├── mod.rs                      # Root module with re-exports
├── serde_utils.rs              # Serialization helpers
├── state.rs                    # Thread-safe state management
└── README.md                   # This file
```

## Components

### Error Handling ([`error/`](error/))

A comprehensive error handling system with three key components:

- **`CommonError`** - Unified error enum for common patterns (timeouts, rate limiting, serialization, etc.)
- **`ErrorClassification` trait** - Standard interface for error classification (retryability, severity, criticality)
- **`ErrorSeverity`** - Severity levels (Info, Warning, Error, Critical) for monitoring and alerting
- **`ErrorContext` trait** - Additional context for errors

#### Error Handling Best Practices

**Use `CommonError` for standard patterns:**
```rust
use crate::common::error::{CommonError, CommonResult};

fn load_data() -> CommonResult<Data> {
    // Use CommonError for standard cases
    let json = std::fs::read_to_string("data.json")
        .map_err(|e| CommonError::persistence(e.to_string()))?;
    
    serde_json::from_str(&json)
        .map_err(|e| CommonError::serialization_format("JSON", e.to_string()))
}
```

**Create module-specific errors that compose with CommonError:**
```rust
use crate::common::error::{CommonError, ErrorClassification, ErrorSeverity};
use thiserror::Error;

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

**Standard Error Patterns:**

| Use Case | CommonError Variant |
|----------|-------------------|
| Circuit breakers | `CircuitBreakerOpen` |
| Rate limiting | `RateLimitExceeded` |
| Timeouts | `Timeout` |
| Lock contention | `Lock` |
| Serialization | `Serialization` |
| Validation | `Validation` |
| Configuration | `Config` |
| Storage/DB | `Storage`, `Persistence` |
| Backend services | `Backend` |
| Authorization | `Unauthorized` |
| Not found | `NotFound` |
| Internal bugs | `Internal` |

**ErrorSeverity Levels:**

- **Info**: Informational, expected (e.g., resource not found)
- **Warning**: Degraded but operational (e.g., rate limiting, transient failures)
- **Error**: Failure requiring attention (e.g., network errors, invalid input)
- **Critical**: System integrity at risk (e.g., data corruption, encryption failures)

### Resilience Patterns ([`resilience/`](resilience/))

Fault tolerance patterns for building robust distributed systems:

#### Circuit Breaker ([`circuit_breaker.rs`](resilience/circuit_breaker.rs))
- `CircuitBreaker` - Prevents cascading failures by detecting and stopping repeated failures
- `CircuitState` - Tracks breaker state (Open/Closed/HalfOpen)
- `CircuitBreakerMetrics` - Performance and failure tracking
- `CircuitBreakerConfig` - Configuration with builder pattern

#### Retry Logic ([`retry.rs`](resilience/retry.rs))
- `RetryExecutor` - Configurable retry execution
- `RetryPolicy` - Retry decision logic with custom policies
- `BackoffStrategy` - Exponential/linear/constant backoff strategies
- `Jitter` - Randomization to prevent thundering herd
- `retry()` / `retry_with_policy()` - Convenience functions

### Manager Pattern ([`manager.rs`](manager.rs))
- `AsyncManager` - Base trait for async component lifecycle
- `ManagerStatus` - Component status tracking
- `ManagerHealth` - Health check infrastructure
- `SharedState` - Thread-safe state management

### State Management ([`state.rs`](state.rs))
- `SharedState` - Thread-safe shared state
- `ManagedState` - Lifecycle-managed state
- `AtomicCounter` - Lock-free counters
- `StateRegistry` - State registry for storing multiple states by key

### Cache ([`cache/`](cache/))
- `Cache` - Synchronous cache with TTL and size limits
- `AsyncCache` - Async cache for concurrent workloads
- `CacheStats` - Hit/miss/eviction metrics
- `CacheConfig` - Configuration with builder pattern

### Validation Framework ([`validation/`](validation/))

Enterprise-grade validation with field-level errors and custom validators:

- `Validator` - Main validation coordinator
- `ValidationError` - Detailed field-level error reporting
- `StringValidator`, `RangeValidator`, `CollectionValidator` - Pre-built validators
- `EmailValidator`, `UrlValidator`, `IpValidator` - Format validators
- `RuleSet` - Complex validation rule composition
- `CustomValidator` - Define custom validation logic

See [`validation/README.md`](validation/README.md) for detailed documentation.

### Design Patterns ([`patterns/`](patterns/))

Reusable design patterns for production systems:

#### Debouncing ([`patterns/debounce/`](patterns/debounce/))
- `ContextDebouncer` - Stability detection for rapidly changing values
- `DebouncerConfig` - Configurable thresholds and rate limiting
- **Distributed coordination** - Redis, Etcd, or in-memory backends
- **Circuit breaker integration** - Protects against system overload
- **Metrics & health monitoring** - Real-time performance tracking

See [`patterns/debounce/README.md`](patterns/debounce/README.md) for detailed documentation.

### Authentication ([`auth/`](auth/))

Authentication primitives for OAuth 2.0 and related protocols:

#### OAuth 2.0 + PKCE ([`auth/oauth/`](auth/oauth/))
- `OAuthService` - High-level OAuth orchestrator
- `OAuthClient` - OAuth HTTP flows (authorization, token exchange)
- `TokenManager` - Token lifecycle with auto-refresh
- `PKCEChallenge` - RFC 7636 compliant PKCE implementation
- **Multi-provider support** - Auth0, Google, Microsoft, custom servers
- **Automatic token refresh** - Background refresh with configurable thresholds
- **Secure storage** - Platform keychain integration (macOS/Windows/Linux)
- **State validation** - CSRF protection with constant-time comparison
- **OpenID Connect** - ID token and user claims support

### Security ([`security/`](security/))

Security primitives for authorization and access control:

#### RBAC ([`security/rbac.rs`](security/rbac.rs))
- `RBACManager` - Role-based access control system
- `Role` - Role hierarchy with inheritance
- `Permission` - Fine-grained permission management
- `RBACPolicy` - Dynamic policies (time-based, IP-based, attribute-based)
- `PolicyCondition` - Complex condition logic (And/Or/Not)
- **Permission caching** - High-performance authorization checks
- **Async evaluation** - Non-blocking policy evaluation

### Compliance ([`compliance/`](compliance/))

Enterprise compliance infrastructure for audit logging, configuration management, and feature control:

#### Audit Logging ([`compliance/audit.rs`](compliance/audit.rs))
- `GlobalAuditLogger` - Thread-safe audit logging system
- `AuditEvent` - Structured audit event recording
- `AuditContext` - Contextual information (user, IP, session)
- `AuditSeverity` - Event severity levels (Info, Warning, Critical)

#### Configuration Management ([`compliance/config.rs`](compliance/config.rs))
- `ConfigManager` - Remote configuration management
- `RemoteConfig` - Dynamic configuration updates
- Version tracking and rollback support

#### Feature Flags ([`compliance/feature_flags.rs`](compliance/feature_flags.rs))
- `FeatureFlagManager` - Feature flag system
- `FeatureFlag` - Individual feature toggles
- Dynamic feature enablement without deployments

### Observability ([`observability/`](observability/))

Unified observability with metrics, error handling, and monitoring:

#### Error Handling ([`observability/errors/`](observability/errors/))
- `AppError` - Unified application error type
- `AiError` - AI/ML specific errors (API keys, rate limits)
- `HttpError` - HTTP client errors
- `UiError` - Frontend-friendly error representation
- `ErrorCode` - Structured error codes for telemetry
- `ActionHint` - Recovery action suggestions

#### Metrics ([`observability/metrics/`](observability/metrics/))
- `MetricsTracker` - Performance and classification metrics
- `ClassificationMetrics` - Classification coverage tracking
- `PerformanceMetrics` - Timing and throughput metrics

See [`observability/README.md`](observability/README.md) for detailed documentation.

### Storage ([`storage/`](storage/))

Encrypted database infrastructure with SQLCipher integration:

#### Encryption ([`storage/encryption/`](storage/encryption/))
- `StorageKeyManager` - Encryption key lifecycle management
- `SecureString` - Memory-safe sensitive string handling
- `KeyRotationSchedule` - Automated key rotation
- `SqlCipherConfig` - SQLCipher configuration
- **Platform keychain integration** - Secure key storage
- **Automatic key rotation** - Scheduled key updates
- **Key caching** - Performance optimization

#### SQLCipher ([`storage/sqlcipher/`](storage/sqlcipher/))
- `SqlCipherConnection` - Encrypted database connections
- `SqlCipherPool` - Connection pooling for encrypted databases
- `SqlCipherPoolConfig` - Pool configuration with builder pattern
- **Pragma management** - SQLite optimization settings
- **Connection validation** - Encryption verification

### Keychain ([`keychain.rs`](keychain.rs))
- `KeychainProvider` - Secure credential storage abstraction
- Platform-specific implementations for macOS Keychain and other systems

### Serialization ([`serde_utils.rs`](serde_utils.rs))
- Custom serialization helpers
- Duration formatting utilities

## Usage

```rust
use agent::common::{
    CommonError, CommonResult, ErrorClassification,
    RetryExecutor, RetryConfig,
    CircuitBreaker, CircuitBreakerConfig,
};

// Error handling with classification
fn example() -> CommonResult<()> {
    let err = CommonError::config("Invalid setting");
    
    if err.is_retryable() {
        println!("This error can be retried");
    }
    
    if err.is_critical() {
        println!("Critical error! Severity: {}", err.severity());
    }
    
    Err(err)
}

// Retry with error classification
async fn with_smart_retry<F, T, E>(mut op: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: ErrorClassification,
{
    for _ in 0..3 {
        match op() {
            Ok(val) => return Ok(val),
            Err(e) if e.is_retryable() => {
                if let Some(delay) = e.retry_after() {
                    tokio::time::sleep(delay).await;
                }
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

// Circuit breaker
let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
breaker.call(|| async { Ok(()) }).await?;
```

## Design Principles

- **Type Safety**: Strong typing with compile-time guarantees
- **Composability**: Components work together seamlessly
- **Observability**: Built-in metrics and health checks
- **Resilience**: Automatic retry and failure protection
- **Thread Safety**: Safe concurrent access patterns
