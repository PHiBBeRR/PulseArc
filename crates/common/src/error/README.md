# Error Type Migration Guide

This guide explains how to migrate module-specific error types to use the `CommonError` infrastructure, reducing duplication and improving consistency across the codebase.

## Table of Contents

1. [Why Migrate?](#why-migrate)
2. [Migration Patterns](#migration-patterns)
3. [Step-by-Step Migration](#step-by-step-migration)
4. [Before and After Examples](#before-and-after-examples)
5. [Testing Your Migration](#testing-your-migration)
6. [Common Pitfalls](#common-pitfalls)
7. [Helper Macros](#helper-macros)

---

## Why Migrate?

**Current Problem:**
- 65+ error enums with duplicated variants (IoError, ConfigError, Timeout, etc.)
- Inconsistent error messages for the same concepts
- Manual error conversion code between modules
- Difficult to establish error handling best practices

**Benefits After Migration:**
- Consistent error handling across all modules
- Automatic error conversion via `CommonError`
- Reduced boilerplate (150+ duplicate variants → shared infrastructure)
- Better retry logic via `ErrorClassification` trait
- Unified monitoring and alerting

---

## Migration Patterns

### Pattern 1: Simple Embedding

**Use when:** Your error type has common variants that match `CommonError` directly.

```rust
// BEFORE
#[derive(Debug, thiserror::Error)]
pub enum MyModuleError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout occurred")]
    Timeout,

    #[error("Module-specific error: {0}")]
    SpecificError(String),
}

// AFTER
use crate::common::error::{CommonError, impl_error_conversion};

#[derive(Debug, thiserror::Error)]
pub enum MyModuleError {
    // Embed CommonError - handles IoError, ConfigError, Timeout automatically
    #[error(transparent)]
    Common(#[from] CommonError),

    // Keep only module-specific errors
    #[error("Module-specific error: {0}")]
    SpecificError(String),
}

// Optional: Add automatic std type conversions
impl_error_conversion!(MyModuleError, Common);
```

### Pattern 2: With ErrorClassification

**Use when:** You want consistent retry logic and error severity classification.

```rust
use crate::common::error::{CommonError, ErrorClassification, ErrorSeverity, impl_error_classification};

#[derive(Debug, thiserror::Error)]
pub enum MyModuleError {
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("Module-specific error: {0}")]
    SpecificError(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

// Implement ErrorClassification using the macro
impl_error_classification!(MyModuleError, Common,
    Self::SpecificError(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::InvalidOperation(_) => {
        retryable: false,
        severity: ErrorSeverity::Warning,
        critical: false,
    }
);
```

### Pattern 3: Bidirectional Conversion

**Use when:** Other modules need to convert your errors to `CommonError`.

```rust
use crate::common::error::CommonError;

#[derive(Debug, thiserror::Error)]
pub enum MyModuleError {
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

// Allow other modules to convert MyModuleError to CommonError
impl From<MyModuleError> for CommonError {
    fn from(err: MyModuleError) -> Self {
        match err {
            MyModuleError::Common(e) => e,
            MyModuleError::InvalidState(msg) => CommonError::internal(msg),
        }
    }
}
```

---

## Step-by-Step Migration

### Step 1: Identify Common Variants

Review your error enum and identify variants that match `CommonError`:

| Your Variant | Maps To | CommonError Builder |
|--------------|---------|---------------------|
| `IoError(io::Error)` | `Persistence` | `CommonError::persistence()` |
| `ConfigError(String)` | `Config` | `CommonError::config()` |
| `Timeout` | `Timeout` | `CommonError::timeout()` |
| `LockError` | `Lock` | `CommonError::lock()` |
| `SerializationError` | `Serialization` | `CommonError::serialization()` |
| `ValidationError` | `Validation` | `CommonError::validation()` |
| `NotFound` | `NotFound` | `CommonError::not_found()` |
| `Unauthorized` | `Unauthorized` | `CommonError::unauthorized()` |
| `CircuitBreakerOpen` | `CircuitBreakerOpen` | `CommonError::circuit_breaker()` |
| `RateLimited` | `RateLimitExceeded` | `CommonError::rate_limit()` |

### Step 2: Update Error Enum

1. Add `Common(#[from] CommonError)` variant
2. Remove duplicate variants
3. Keep module-specific variants

```rust
// BEFORE
#[derive(Debug, thiserror::Error)]
pub enum IdleError {
    #[error("System error: {0}")]
    SystemError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout occurred")]
    Timeout,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),
}

// AFTER
use crate::common::error::CommonError;

#[derive(Debug, thiserror::Error)]
pub enum IdleError {
    #[error(transparent)]
    Common(#[from] CommonError),

    // Keep platform-specific error
    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),
}

impl_error_conversion!(IdleError, Common);
```

### Step 3: Update Call Sites

Update code that creates errors:

```rust
// BEFORE
return Err(IdleError::ConfigError("invalid threshold".to_string()));
return Err(IdleError::Timeout);
let config = std::fs::read_to_string(path)?; // Creates IdleError::IoError

// AFTER
return Err(CommonError::config("invalid threshold").into());
return Err(CommonError::timeout("idle_detection", Duration::from_secs(5)).into());
let config = std::fs::read_to_string(path)?; // Automatically converts to IdleError::Common
```

### Step 4: Update Error Matching

Update code that matches on errors:

```rust
// BEFORE
match result {
    Err(IdleError::ConfigError(msg)) => println!("Config: {}", msg),
    Err(IdleError::Timeout) => println!("Timeout"),
    Err(IdleError::IoError(e)) => println!("I/O: {}", e),
    Err(e) => println!("Other: {}", e),
}

// AFTER
match result {
    Err(IdleError::Common(CommonError::Config { message, .. })) => {
        println!("Config: {}", message)
    }
    Err(IdleError::Common(CommonError::Timeout { .. })) => {
        println!("Timeout")
    }
    Err(IdleError::Common(CommonError::Persistence { .. })) => {
        println!("I/O error")
    }
    Err(e) => println!("Other: {}", e),
}

// Or use ErrorClassification for high-level handling
match result {
    Err(e) if e.is_retryable() => retry_operation(),
    Err(e) if e.is_critical() => alert_ops_team(e),
    Err(e) => log_error(e),
}
```

### Step 5: Run Tests

```bash
# Run unit tests for the module
cargo test --package agent --lib module_name::tests

# Run integration tests
cargo test --package agent --test '*'

# Check for clippy warnings
cargo clippy --package agent
```

---

## Before and After Examples

### Example 1: IdleError

**BEFORE** ([agent/idle/core/types.rs:8-54](../../agent/idle/core/types.rs)):

```rust
#[derive(Debug, Error)]
pub enum IdleError {
    #[error("Failed to get system idle time: {message}")]
    SystemError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Platform not supported: {platform}")]
    UnsupportedPlatform {
        platform: String,
        fallback_available: bool,
    },

    #[error("Failed to acquire lock after {attempts} attempts")]
    LockError {
        attempts: u32,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Configuration error: {message}")]
    ConfigError { message: String, field: String },

    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64, operation: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Platform API error: {api} - {code}")]
    PlatformApiError {
        api: String,
        code: i32,
        message: String,
    },

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
}
```

**AFTER:**

```rust
use crate::common::error::{CommonError, ErrorClassification, ErrorSeverity, impl_error_conversion};

#[derive(Debug, Error)]
pub enum IdleError {
    // Common errors - automatic via #[from]
    #[error(transparent)]
    Common(#[from] CommonError),

    // Module-specific errors only
    #[error("Platform not supported: {platform}")]
    UnsupportedPlatform {
        platform: String,
        fallback_available: bool,
    },

    #[error("Platform API error: {api} - {code}")]
    PlatformApiError {
        api: String,
        code: i32,
        message: String,
    },

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
}

// Automatic conversions from std types
impl_error_conversion!(IdleError, Common);

// Classification for module-specific errors
impl_error_classification!(IdleError, Common,
    Self::UnsupportedPlatform { .. } => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::PlatformApiError { .. } => {
        retryable: true,  // Platform APIs may be temporarily unavailable
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::InvalidStateTransition { .. } => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    }
);
```

**Call Site Updates:**

```rust
// BEFORE
if threshold > MAX {
    return Err(IdleError::ConfigError {
        message: "threshold too high".to_string(),
        field: "threshold".to_string(),
    });
}

// AFTER
if threshold > MAX {
    return Err(CommonError::config_field("threshold", "threshold too high").into());
}

// BEFORE
Err(IdleError::Timeout {
    duration_ms: 5000,
    operation: "detect_idle".to_string(),
})

// AFTER
Err(CommonError::timeout("detect_idle", Duration::from_secs(5)).into())

// BEFORE
Err(IdleError::LockError {
    attempts: 3,
    source: None,
})

// AFTER
Err(CommonError::lock_resource("idle_state", "failed after 3 attempts").into())
```

### Example 2: ReporterError

**BEFORE** ([agent/telemetry/reporter/errors.rs:6-43](../../agent/telemetry/reporter/errors.rs)):

```rust
#[derive(Error, Debug)]
pub enum ReporterError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Compression error: {0}")]
    Compression(#[from] std::io::Error),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("Batch size exceeded: {size} > {max}")]
    BatchSizeExceeded { size: usize, max: usize },

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Server error: {status}")]
    ServerError { status: StatusCode },

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("All endpoints failed")]
    AllEndpointsFailed,

    #[error("Configuration error: {0}")]
    Configuration(String),
}
```

**AFTER:**

```rust
use crate::common::error::{CommonError, ErrorClassification, ErrorSeverity, impl_error_conversion};

#[derive(Error, Debug)]
pub enum ReporterError {
    // Common errors (handles Serialization, CircuitBreaker, RateLimited, Configuration, etc.)
    #[error(transparent)]
    Common(#[from] CommonError),

    // Module-specific errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Batch size exceeded: {size} > {max}")]
    BatchSizeExceeded { size: usize, max: usize },

    #[error("Server error: {status}")]
    ServerError { status: StatusCode },

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("All endpoints failed")]
    AllEndpointsFailed,
}

// Auto-convert std types via CommonError
impl_error_conversion!(ReporterError, Common);

// Classify module-specific errors
impl_error_classification!(ReporterError, Common,
    Self::Network(_) => {
        retryable: true,
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::BatchSizeExceeded { .. } => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::ServerError { .. } => {
        retryable: true,
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Encryption(_) => {
        retryable: false,
        severity: ErrorSeverity::Critical,  // Encryption failures are critical
        critical: true,
    },
    Self::AllEndpointsFailed => {
        retryable: true,
        severity: ErrorSeverity::Error,
        critical: false,
    }
);
```

**Call Site Updates:**

```rust
// BEFORE
if batch.len() > MAX_BATCH_SIZE {
    return Err(ReporterError::Configuration(
        format!("Batch size {} exceeds max {}", batch.len(), MAX_BATCH_SIZE)
    ));
}

// AFTER
if batch.len() > MAX_BATCH_SIZE {
    return Err(ReporterError::BatchSizeExceeded {
        size: batch.len(),
        max: MAX_BATCH_SIZE,
    });
}

// BEFORE
return Err(ReporterError::CircuitBreakerOpen);

// AFTER
return Err(CommonError::circuit_breaker("telemetry_reporter").into());

// BEFORE
return Err(ReporterError::AuthenticationFailed);

// AFTER
return Err(CommonError::unauthorized("report_telemetry").into());
```

### Example 3: PiiError

**BEFORE** ([agent/privacy/patterns/error.rs:5-20](../../agent/privacy/patterns/error.rs)):

```rust
#[derive(Error, Debug)]
pub enum PiiError {
    #[error("Pattern compilation error: {0}")]
    PatternCompilation(String),

    #[error("Pattern matching error: {0}")]
    PatternMatching(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
```

**AFTER:**

```rust
use crate::common::error::{CommonError, ErrorClassification, ErrorSeverity, impl_error_conversion};

#[derive(Error, Debug)]
pub enum PiiError {
    // Common errors
    #[error(transparent)]
    Common(#[from] CommonError),

    // PII-specific errors
    #[error("Pattern compilation error: {0}")]
    PatternCompilation(String),

    #[error("Pattern matching error: {0}")]
    PatternMatching(String),

    #[error("Processing error: {0}")]
    Processing(String),
}

impl_error_conversion!(PiiError, Common);

impl_error_classification!(PiiError, Common,
    Self::PatternCompilation(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::PatternMatching(_) => {
        retryable: false,
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Processing(_) => {
        retryable: true,  // Processing may be retryable
        severity: ErrorSeverity::Warning,
        critical: false,
    }
);
```

**Call Site Updates:**

```rust
// BEFORE
if config.patterns.is_empty() {
    return Err(PiiError::Configuration("No patterns configured".to_string()));
}

// AFTER
if config.patterns.is_empty() {
    return Err(CommonError::config_field("patterns", "No patterns configured").into());
}

// BEFORE
if pattern.is_empty() {
    return Err(PiiError::Validation("Pattern cannot be empty".to_string()));
}

// AFTER
if pattern.is_empty() {
    return Err(CommonError::validation("pattern", "Pattern cannot be empty").into());
}
```

---

## Testing Your Migration

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_common_error_conversion() {
        // Test io::Error -> CommonError -> MyModuleError
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file");
        let module_err: MyModuleError = io_err.into();
        assert!(matches!(module_err, MyModuleError::Common(_)));
    }

    #[test]
    fn test_timeout_construction() {
        let err: MyModuleError = CommonError::timeout("operation", Duration::from_secs(5)).into();
        assert!(err.is_retryable());
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_module_specific_error() {
        let err = MyModuleError::SpecificError("test".to_string());
        assert!(!err.is_retryable());
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }
}
```

### Integration Tests

Create integration tests in `agent/tests/`:

```rust
// agent/tests/error_propagation.rs
use agent::idle::core::types::IdleError;
use agent::capture::SamplerError;
use agent::common::error::{CommonError, ErrorClassification};

#[test]
fn test_cross_module_error_propagation() {
    fn idle_function() -> Result<(), IdleError> {
        Err(CommonError::timeout("idle", Duration::from_secs(1)).into())
    }

    fn capture_function() -> Result<(), SamplerError> {
        // IdleError should convert to SamplerError via CommonError
        idle_function().map_err(|e| {
            let common: CommonError = e.into();
            common.into()
        })?;
        Ok(())
    }

    let result = capture_function();
    assert!(result.is_err());
    assert!(result.unwrap_err().is_retryable());
}
```

---

## Common Pitfalls

### 1. Forgetting to Remove Old Variants

**Problem:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("I/O error: {0}")]  // ❌ Duplicate! CommonError already handles io::Error
    IoError(#[from] std::io::Error),
}
```

**Solution:** Remove duplicate variants that are handled by `CommonError`.

### 2. Conflicting From Implementations

**Problem:**
```rust
// Using #[from] on Common variant
#[error(transparent)]
Common(#[from] CommonError),

// Then using macro that creates From<CommonError>
impl_error_conversion!(MyError, Common, with_common);  // ❌ Conflict!
```

**Solution:** Use the standard macro without `with_common`:
```rust
impl_error_conversion!(MyError, Common);
```

### 3. Not Updating Call Sites

**Problem:**
```rust
// Error enum migrated, but call sites still reference old variant
return Err(MyError::ConfigError("invalid".to_string()));  // ❌ No longer exists
```

**Solution:** Update all call sites to use `CommonError` builders:
```rust
return Err(CommonError::config("invalid").into());
```

### 4. Incorrect Error Classification

**Problem:**
```rust
impl_error_classification!(MyError, Common,
    Self::Specific(_) => {
        retryable: true,  // ❌ But this error type should NOT be retried!
        severity: ErrorSeverity::Error,
        critical: false,
    }
);
```

**Solution:** Carefully consider the semantics of each error:
- **Retryable**: Network timeouts, rate limits, temporary failures
- **Non-retryable**: Validation errors, authentication failures, not found

---

## Helper Macros

### `impl_error_conversion!`

Generates `From` implementations for std types via `CommonError`.

**Usage:**
```rust
impl_error_conversion!(MyError, Common);
```

**Generates:**
- `From<serde_json::Error> for MyError`
- `From<std::io::Error> for MyError`

### `impl_error_classification!`

Implements `ErrorClassification` trait by delegating to `CommonError`.

**Usage:**
```rust
impl_error_classification!(MyError, Common,
    Self::SpecificError(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    }
);
```

**Generates:**
- `is_retryable()` - delegates `Common` variant to `CommonError`
- `severity()` - delegates `Common` variant to `CommonError`
- `is_critical()` - delegates `Common` variant to `CommonError`
- `retry_after()` - delegates `Common` variant to `CommonError`

---

## Summary Checklist

Migration checklist for each module:

- [ ] Identify common error variants
- [ ] Add `Common(#[from] CommonError)` variant
- [ ] Remove duplicate variants
- [ ] Add `impl_error_conversion!` macro
- [ ] Implement `ErrorClassification` (optional but recommended)
- [ ] Update all error construction sites
- [ ] Update error matching/handling code
- [ ] Add/update unit tests
- [ ] Run `cargo test` - all tests pass
- [ ] Run `cargo clippy` - no warnings
- [ ] Update module documentation

---

## Getting Help

If you encounter issues during migration:

1. Check the `CommonError` documentation in [agent/common/error.rs](../../agent/common/error.rs)
2. Review the test examples in `agent/common/error.rs` tests module
3. Look at already-migrated modules (CalendarError, StorageError, SamplerError)
4. Consult the ticket: [tickets/open/error-type-proliferation.md](../../tickets/open/error-type-proliferation.md)
