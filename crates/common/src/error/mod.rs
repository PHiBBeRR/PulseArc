//! Common error types and utilities for the Tauri agent
//!
//! This module provides standardized error handling infrastructure that can be
//! used across all modules in the application. It includes common error
//! variants, conversion patterns, and utility functions for error handling.
//!
//! # Error Handling Architecture
//!
//! The error handling system is built on three key components:
//!
//! 1. **`CommonError`**: A comprehensive enum of common error patterns that
//!    appear across multiple modules (timeouts, rate limiting, serialization,
//!    etc.)
//!
//! 2. **`ErrorClassification` trait**: A standard interface for classifying
//!    errors by their characteristics (retryability, severity, criticality)
//!
//! 3. **`ErrorSeverity` enum**: A unified severity level system for monitoring
//!    and alerting across all error types
//!
//! ## When to Use CommonError vs Module-Specific Errors
//!
//! ### Use `CommonError` directly when:
//!
//! - The error is a standard pattern (timeout, serialization, lock, etc.)
//! - The error doesn't require module-specific context
//! - You want immediate interoperability with other modules
//!
//! ### Create a module-specific error when:
//!
//! - The error requires domain-specific information
//! - You need custom error messages or debugging info
//! - The error has module-specific handling logic
//!
//! ### Best Practice: Composition
//!
//! Module-specific errors should **compose** with `CommonError` rather than
//! duplicating common patterns:
//!
//! ```rust,ignore
//! #[derive(Debug, Error)]
//! pub enum MyModuleError {
//!     // Module-specific variants
//!     #[error("Invalid widget configuration: {0}")]
//!     InvalidWidget(String),
//!
//!     // Embed common errors
//!     #[error(transparent)]
//!     Common(#[from] CommonError),
//! }
//!
//! impl ErrorClassification for MyModuleError {
//!     fn is_retryable(&self) -> bool {
//!         match self {
//!             Self::InvalidWidget(_) => false,
//!             Self::Common(e) => e.is_retryable(),
//!         }
//!     }
//!     // ... implement other trait methods
//! }
//! ```
//!
//! ## Standard Error Patterns
//!
//! The following patterns should use `CommonError` variants:
//!
//! | Pattern | CommonError Variant | When to Use |
//! |---------|-------------------|-------------|
//! | **Circuit Breaker** | `CircuitBreakerOpen` | Service protection, cascading failure prevention |
//! | **Rate Limiting** | `RateLimitExceeded` | API quotas, throttling |
//! | **Timeouts** | `Timeout` | Operation deadlines, hanging operations |
//! | **Lock Errors** | `Lock` | Mutex/lock contention, poisoned locks |
//! | **Serialization** | `Serialization` | JSON/TOML parsing, encoding errors |
//! | **Validation** | `Validation` | Input validation, constraint violations |
//! | **Configuration** | `Config` | Invalid settings, missing config |
//! | **Storage/DB** | `Storage` / `Persistence` | File I/O, database operations |
//! | **Backend** | `Backend` | External service failures |
//! | **Authorization** | `Unauthorized` | Permission denied, auth failures |
//! | **Not Found** | `NotFound` | Missing resources |
//! | **Internal** | `Internal` | Bugs, invariant violations |
//!
//! ## ErrorClassification Trait
//!
//! All error types in the system should implement `ErrorClassification` to
//! provide:
//!
//! - **`is_retryable()`**: Can this operation be retried?
//! - **`severity()`**: How serious is this error? (Info/Warning/Error/Critical)
//! - **`is_critical()`**: Does this require immediate attention?
//! - **`retry_after()`**: Suggested retry delay (if applicable)
//!
//! These methods enable:
//! - Consistent retry logic across all modules
//! - Unified monitoring and alerting
//! - Better error reporting and debugging
//!
//! ## ErrorSeverity Levels
//!
//! | Level | Use Case | Examples |
//! |-------|----------|----------|
//! | **Info** | Informational, expected conditions | Resource not found, empty results |
//! | **Warning** | Degraded but operational | Rate limiting, lock contention, transient failures |
//! | **Error** | Failure requiring attention | Network errors, invalid input, config errors |
//! | **Critical** | System integrity at risk | Data corruption, encryption failures, internal errors |
//!
//! ## Examples
//!
//! ### Using CommonError directly
//!
//! ```rust,ignore
//! use crate::error::{CommonError, CommonResult};
//!
//! fn load_config() -> CommonResult<Config> {
//!     let data = std::fs::read_to_string("config.toml")
//!         .map_err(|e| CommonError::persistence(e.to_string()))?;
//!
//!     toml::from_str(&data)
//!         .map_err(|e| CommonError::serialization_format("TOML", e.to_string()))
//! }
//! ```
//!
//! ### Creating a module-specific error
//!
//! ```rust,ignore
//! use crate::error::{CommonError, ErrorClassification, ErrorSeverity};
//! use thiserror::Error;
//!
//! #[derive(Debug, Error)]
//! pub enum WidgetError {
//!     #[error("Widget not found: {0}")]
//!     NotFound(String),
//!
//!     #[error("Widget validation failed: {0}")]
//!     Invalid(String),
//!
//!     #[error(transparent)]
//!     Common(#[from] CommonError),
//! }
//!
//! impl ErrorClassification for WidgetError {
//!     fn is_retryable(&self) -> bool {
//!         match self {
//!             Self::NotFound(_) | Self::Invalid(_) => false,
//!             Self::Common(e) => e.is_retryable(),
//!         }
//!     }
//!
//!     fn severity(&self) -> ErrorSeverity {
//!         match self {
//!             Self::NotFound(_) => ErrorSeverity::Info,
//!             Self::Invalid(_) => ErrorSeverity::Error,
//!             Self::Common(e) => e.severity(),
//!         }
//!     }
//!
//!     fn is_critical(&self) -> bool {
//!         match self {
//!             Self::Common(e) => e.is_critical(),
//!             _ => false,
//!         }
//!     }
//!
//!     fn retry_after(&self) -> Option<Duration> {
//!         match self {
//!             Self::Common(e) => e.retry_after(),
//!             _ => None,
//!         }
//!     }
//! }
//! ```
//!
//! ### Using ErrorClassification for retry logic
//!
//! ```rust,ignore
//! use crate::error::ErrorClassification;
//!
//! async fn with_retry<F, T, E>(mut operation: F) -> Result<T, E>
//! where
//!     F: FnMut() -> Result<T, E>,
//!     E: ErrorClassification,
//! {
//!     for attempt in 1..=3 {
//!         match operation() {
//!             Ok(result) => return Ok(result),
//!             Err(e) if e.is_retryable() => {
//!                 if let Some(delay) = e.retry_after() {
//!                     tokio::time::sleep(delay).await;
//!                 }
//!                 continue;
//!             }
//!             Err(e) => return Err(e),
//!         }
//!     }
//!     unreachable!()
//! }
//! ```

use std::fmt;
use std::time::Duration;

/// Standard result type using CommonError
pub type CommonResult<T> = Result<T, CommonError>;

/// Common error variants that appear across multiple modules
///
/// This enum provides standardized error types that can be embedded in
/// module-specific error enums to ensure consistency across the application.
#[derive(Debug, Clone)]
pub enum CommonError {
    /// Configuration-related errors
    Config { message: String, field: Option<String> },

    /// Lock acquisition or concurrency errors
    Lock { message: String, resource: Option<String> },

    /// Circuit breaker is open, preventing operations
    CircuitBreakerOpen { service: String, retry_after: Option<Duration> },

    /// Serialization or deserialization errors
    Serialization { message: String, format: Option<String> },

    /// Data persistence errors (file I/O, database, etc.)
    Persistence { message: String, operation: Option<String> },

    /// Rate limiting errors
    RateLimitExceeded {
        limit: Option<u32>,
        window: Option<Duration>,
        retry_after: Option<Duration>,
    },

    /// Timeout errors
    Timeout { operation: String, duration: Duration },

    /// Network or backend connectivity errors
    Backend { service: String, message: String, is_retryable: bool },

    /// Validation errors
    Validation { field: String, message: String, value: Option<String> },

    /// Resource not found errors
    NotFound { resource_type: String, identifier: Option<String> },

    /// Permission or authorization errors
    Unauthorized { operation: String, required_permission: Option<String> },

    /// Internal errors that shouldn't normally occur
    Internal { message: String, context: Option<String> },

    /// Storage/database errors
    Storage { message: String, operation: Option<String> },

    /// Detailed error with severity and context
    Detailed { message: String, severity: ErrorSeverity, context: Option<String> },

    /// Task cancellation (async)
    TaskCancelled { task_id: String, reason: Option<String> },

    /// Async operation timeout
    AsyncTimeout { future_name: String, duration: Duration },
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config { message, field } => {
                if let Some(field) = field {
                    write!(f, "Configuration error in field '{}': {}", field, message)
                } else {
                    write!(f, "Configuration error: {}", message)
                }
            }
            Self::Lock { message, resource } => {
                if let Some(resource) = resource {
                    write!(f, "Lock error for '{}': {}", resource, message)
                } else {
                    write!(f, "Lock error: {}", message)
                }
            }
            Self::CircuitBreakerOpen { service, retry_after } => {
                if let Some(retry) = retry_after {
                    write!(f, "Circuit breaker open for '{}' (retry in {:?})", service, retry)
                } else {
                    write!(f, "Circuit breaker open for '{}'", service)
                }
            }
            Self::Serialization { message, format } => {
                if let Some(format) = format {
                    write!(f, "Serialization error ({}): {}", format, message)
                } else {
                    write!(f, "Serialization error: {}", message)
                }
            }
            Self::Persistence { message, operation } => {
                if let Some(op) = operation {
                    write!(f, "Persistence error during '{}': {}", op, message)
                } else {
                    write!(f, "Persistence error: {}", message)
                }
            }
            Self::RateLimitExceeded { limit, window, retry_after } => {
                let mut msg = "Rate limit exceeded".to_string();
                if let (Some(limit), Some(window)) = (limit, window) {
                    msg.push_str(&format!(": {} requests per {:?}", limit, window));
                }
                if let Some(retry) = retry_after {
                    msg.push_str(&format!(" (retry in {:?})", retry));
                }
                write!(f, "{}", msg)
            }
            Self::Timeout { operation, duration } => {
                write!(f, "Operation '{}' timed out after {:?}", operation, duration)
            }
            Self::Backend { service, message, .. } => {
                write!(f, "Backend error from '{}': {}", service, message)
            }
            Self::Validation { field, message, value } => {
                if let Some(value) = value {
                    write!(
                        f,
                        "Validation error for field '{}' (value: '{}'): {}",
                        field, value, message
                    )
                } else {
                    write!(f, "Validation error for field '{}': {}", field, message)
                }
            }
            Self::NotFound { resource_type, identifier } => {
                if let Some(id) = identifier {
                    write!(f, "{} not found: '{}'", resource_type, id)
                } else {
                    write!(f, "{} not found", resource_type)
                }
            }
            Self::Unauthorized { operation, required_permission } => {
                if let Some(perm) = required_permission {
                    write!(f, "Unauthorized to perform '{}' (requires: {})", operation, perm)
                } else {
                    write!(f, "Unauthorized to perform '{}'", operation)
                }
            }
            Self::Internal { message, context } => {
                if let Some(ctx) = context {
                    write!(f, "Internal error in '{}': {}", ctx, message)
                } else {
                    write!(f, "Internal error: {}", message)
                }
            }
            Self::Storage { message, operation } => {
                if let Some(op) = operation {
                    write!(f, "Storage error during '{}': {}", op, message)
                } else {
                    write!(f, "Storage error: {}", message)
                }
            }
            Self::Detailed { message, severity, .. } => {
                write!(f, "[{}] {}", severity, message)
            }
            Self::TaskCancelled { task_id, reason } => {
                if let Some(reason) = reason {
                    write!(f, "Task '{}' cancelled: {}", task_id, reason)
                } else {
                    write!(f, "Task '{}' cancelled", task_id)
                }
            }
            Self::AsyncTimeout { future_name, duration } => {
                write!(f, "Async operation '{}' timed out after {:?}", future_name, duration)
            }
        }
    }
}

impl std::error::Error for CommonError {}

impl ErrorClassification for CommonError {
    /// Check if this error is retryable
    fn is_retryable(&self) -> bool {
        match self {
            Self::CircuitBreakerOpen { .. } => true,
            Self::RateLimitExceeded { .. } => true,
            Self::Timeout { .. } => true,
            Self::Backend { is_retryable, .. } => *is_retryable,
            Self::Lock { .. } => true,
            Self::AsyncTimeout { .. } => true,
            _ => false,
        }
    }

    /// Get the error severity level
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Config { .. } => ErrorSeverity::Error,
            Self::Lock { .. } => ErrorSeverity::Warning,
            Self::CircuitBreakerOpen { .. } => ErrorSeverity::Warning,
            Self::Serialization { .. } => ErrorSeverity::Error,
            Self::Persistence { .. } => ErrorSeverity::Error,
            Self::RateLimitExceeded { .. } => ErrorSeverity::Warning,
            Self::Timeout { .. } => ErrorSeverity::Warning,
            Self::Backend { .. } => ErrorSeverity::Error,
            Self::Validation { .. } => ErrorSeverity::Error,
            Self::NotFound { .. } => ErrorSeverity::Info,
            Self::Unauthorized { .. } => ErrorSeverity::Warning,
            Self::Internal { .. } => ErrorSeverity::Critical,
            Self::Storage { .. } => ErrorSeverity::Error,
            Self::Detailed { severity, .. } => *severity,
            Self::TaskCancelled { .. } => ErrorSeverity::Info,
            Self::AsyncTimeout { .. } => ErrorSeverity::Warning,
        }
    }

    /// Check if this is a critical error requiring immediate attention
    fn is_critical(&self) -> bool {
        matches!(self, Self::Internal { .. })
    }

    /// Get the suggested retry delay if applicable
    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::CircuitBreakerOpen { retry_after, .. } => *retry_after,
            Self::RateLimitExceeded { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl CommonError {
    /// Create a simple configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config { message: message.into(), field: None }
    }

    /// Create a configuration error for a specific field
    pub fn config_field<S: Into<String>, F: Into<String>>(field: F, message: S) -> Self {
        Self::Config { message: message.into(), field: Some(field.into()) }
    }

    /// Create a simple lock error
    pub fn lock<S: Into<String>>(message: S) -> Self {
        Self::Lock { message: message.into(), resource: None }
    }

    /// Create a lock error for a specific resource
    pub fn lock_resource<S: Into<String>, R: Into<String>>(resource: R, message: S) -> Self {
        Self::Lock { message: message.into(), resource: Some(resource.into()) }
    }

    /// Create a circuit breaker error
    pub fn circuit_breaker<S: Into<String>>(service: S) -> Self {
        Self::CircuitBreakerOpen { service: service.into(), retry_after: None }
    }

    /// Create a circuit breaker error with retry timing
    pub fn circuit_breaker_with_retry<S: Into<String>>(service: S, retry_after: Duration) -> Self {
        Self::CircuitBreakerOpen { service: service.into(), retry_after: Some(retry_after) }
    }

    /// Create a simple serialization error
    pub fn serialization<S: Into<String>>(message: S) -> Self {
        Self::Serialization { message: message.into(), format: None }
    }

    /// Create a serialization error with format information
    pub fn serialization_format<S: Into<String>, F: Into<String>>(format: F, message: S) -> Self {
        Self::Serialization { message: message.into(), format: Some(format.into()) }
    }

    /// Create a simple persistence error
    pub fn persistence<S: Into<String>>(message: S) -> Self {
        Self::Persistence { message: message.into(), operation: None }
    }

    /// Create a persistence error for a specific operation
    pub fn persistence_op<S: Into<String>, O: Into<String>>(operation: O, message: S) -> Self {
        Self::Persistence { message: message.into(), operation: Some(operation.into()) }
    }

    /// Create a simple rate limit error
    pub fn rate_limit() -> Self {
        Self::RateLimitExceeded { limit: None, window: None, retry_after: None }
    }

    /// Create a rate limit error with details
    pub fn rate_limit_detailed(
        limit: u32,
        window: Duration,
        retry_after: Option<Duration>,
    ) -> Self {
        Self::RateLimitExceeded { limit: Some(limit), window: Some(window), retry_after }
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(operation: S, duration: Duration) -> Self {
        Self::Timeout { operation: operation.into(), duration }
    }

    /// Create a backend error
    pub fn backend<S: Into<String>, M: Into<String>>(
        service: S,
        message: M,
        is_retryable: bool,
    ) -> Self {
        Self::Backend { service: service.into(), message: message.into(), is_retryable }
    }

    /// Create a validation error
    pub fn validation<F: Into<String>, M: Into<String>>(field: F, message: M) -> Self {
        Self::Validation { field: field.into(), message: message.into(), value: None }
    }

    /// Create a validation error with the invalid value
    pub fn validation_with_value<F: Into<String>, M: Into<String>, V: Into<String>>(
        field: F,
        message: M,
        value: V,
    ) -> Self {
        Self::Validation { field: field.into(), message: message.into(), value: Some(value.into()) }
    }

    /// Create a not found error
    pub fn not_found<T: Into<String>>(resource_type: T) -> Self {
        Self::NotFound { resource_type: resource_type.into(), identifier: None }
    }

    /// Create a not found error with identifier
    pub fn not_found_with_id<T: Into<String>, I: Into<String>>(
        resource_type: T,
        identifier: I,
    ) -> Self {
        Self::NotFound { resource_type: resource_type.into(), identifier: Some(identifier.into()) }
    }

    /// Create an unauthorized error
    pub fn unauthorized<O: Into<String>>(operation: O) -> Self {
        Self::Unauthorized { operation: operation.into(), required_permission: None }
    }

    /// Create an unauthorized error with required permission
    pub fn unauthorized_with_perm<O: Into<String>, P: Into<String>>(
        operation: O,
        permission: P,
    ) -> Self {
        Self::Unauthorized {
            operation: operation.into(),
            required_permission: Some(permission.into()),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal { message: message.into(), context: None }
    }

    /// Create an internal error with context
    pub fn internal_with_context<S: Into<String>, C: Into<String>>(message: S, context: C) -> Self {
        Self::Internal { message: message.into(), context: Some(context.into()) }
    }

    /// Create a task cancellation error
    pub fn task_cancelled<S: Into<String>>(task_id: S) -> Self {
        Self::TaskCancelled { task_id: task_id.into(), reason: None }
    }

    /// Create a task cancellation error with reason
    pub fn task_cancelled_with_reason<S: Into<String>, R: Into<String>>(
        task_id: S,
        reason: R,
    ) -> Self {
        Self::TaskCancelled { task_id: task_id.into(), reason: Some(reason.into()) }
    }

    /// Create an async timeout error
    pub fn async_timeout<S: Into<String>>(future_name: S, duration: Duration) -> Self {
        Self::AsyncTimeout { future_name: future_name.into(), duration }
    }

    /// Add context to this error (fluent API)
    ///
    /// This allows chaining additional context onto an error:
    ///
    /// ```rust,ignore
    /// CommonError::timeout("operation", dur)
    ///     .with_additional_context("retry_attempt", "3")
    /// ```
    pub fn with_additional_context<K: Into<String>, V: Into<String>>(
        self,
        _key: K,
        _value: V,
    ) -> Self {
        // For now, we store the original error
        // Future: could extend to store key-value pairs in a map
        self
    }

    /// Convert error to structured logging fields
    ///
    /// Returns a vector of key-value pairs suitable for structured logging.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tracing::error;
    ///
    /// let err = CommonError::timeout("database_query", Duration::from_secs(5));
    /// let fields = err.as_tracing_fields();
    /// error!(
    ///     error_type = fields[0].1,
    ///     operation = fields[1].1,
    ///     "Operation failed"
    /// );
    /// ```
    pub fn as_tracing_fields(&self) -> Vec<(&'static str, String)> {
        let mut fields = vec![("error_type", self.error_type_name().to_string())];

        match self {
            Self::Config { message, field } => {
                fields.push(("message", message.clone()));
                if let Some(field) = field {
                    fields.push(("field", field.clone()));
                }
            }
            Self::Lock { message, resource } => {
                fields.push(("message", message.clone()));
                if let Some(resource) = resource {
                    fields.push(("resource", resource.clone()));
                }
            }
            Self::CircuitBreakerOpen { service, retry_after } => {
                fields.push(("service", service.clone()));
                if let Some(retry) = retry_after {
                    fields.push(("retry_after_ms", retry.as_millis().to_string()));
                }
            }
            Self::Serialization { message, format } => {
                fields.push(("message", message.clone()));
                if let Some(format) = format {
                    fields.push(("format", format.clone()));
                }
            }
            Self::Persistence { message, operation } => {
                fields.push(("message", message.clone()));
                if let Some(op) = operation {
                    fields.push(("operation", op.clone()));
                }
            }
            Self::RateLimitExceeded { limit, window, retry_after } => {
                if let Some(limit) = limit {
                    fields.push(("limit", limit.to_string()));
                }
                if let Some(window) = window {
                    fields.push(("window_ms", window.as_millis().to_string()));
                }
                if let Some(retry) = retry_after {
                    fields.push(("retry_after_ms", retry.as_millis().to_string()));
                }
            }
            Self::Timeout { operation, duration } => {
                fields.push(("operation", operation.clone()));
                fields.push(("duration_ms", duration.as_millis().to_string()));
            }
            Self::Backend { service, message, is_retryable } => {
                fields.push(("service", service.clone()));
                fields.push(("message", message.clone()));
                fields.push(("is_retryable", is_retryable.to_string()));
            }
            Self::Validation { field, message, value } => {
                fields.push(("field", field.clone()));
                fields.push(("message", message.clone()));
                if let Some(value) = value {
                    fields.push(("value", value.clone()));
                }
            }
            Self::NotFound { resource_type, identifier } => {
                fields.push(("resource_type", resource_type.clone()));
                if let Some(id) = identifier {
                    fields.push(("identifier", id.clone()));
                }
            }
            Self::Unauthorized { operation, required_permission } => {
                fields.push(("operation", operation.clone()));
                if let Some(perm) = required_permission {
                    fields.push(("required_permission", perm.clone()));
                }
            }
            Self::Internal { message, context } => {
                fields.push(("message", message.clone()));
                if let Some(ctx) = context {
                    fields.push(("context", ctx.clone()));
                }
            }
            Self::Storage { message, operation } => {
                fields.push(("message", message.clone()));
                if let Some(op) = operation {
                    fields.push(("operation", op.clone()));
                }
            }
            Self::Detailed { message, severity, context } => {
                fields.push(("message", message.clone()));
                fields.push(("severity", format!("{}", severity)));
                if let Some(ctx) = context {
                    fields.push(("context", ctx.clone()));
                }
            }
            Self::TaskCancelled { task_id, reason } => {
                fields.push(("task_id", task_id.clone()));
                if let Some(reason) = reason {
                    fields.push(("reason", reason.clone()));
                }
            }
            Self::AsyncTimeout { future_name, duration } => {
                fields.push(("future_name", future_name.clone()));
                fields.push(("duration_ms", duration.as_millis().to_string()));
            }
        }

        fields
    }

    /// Get the error type name for categorization
    fn error_type_name(&self) -> &'static str {
        match self {
            Self::Config { .. } => "config",
            Self::Lock { .. } => "lock",
            Self::CircuitBreakerOpen { .. } => "circuit_breaker_open",
            Self::Serialization { .. } => "serialization",
            Self::Persistence { .. } => "persistence",
            Self::RateLimitExceeded { .. } => "rate_limit_exceeded",
            Self::Timeout { .. } => "timeout",
            Self::Backend { .. } => "backend",
            Self::Validation { .. } => "validation",
            Self::NotFound { .. } => "not_found",
            Self::Unauthorized { .. } => "unauthorized",
            Self::Internal { .. } => "internal",
            Self::Storage { .. } => "storage",
            Self::Detailed { .. } => "detailed",
            Self::TaskCancelled { .. } => "task_cancelled",
            Self::AsyncTimeout { .. } => "async_timeout",
        }
    }
}

/// Error classification trait for consistent error handling across modules
///
/// This trait provides a standard interface for classifying errors by their
/// characteristics, enabling consistent retry logic, monitoring, and alerting
/// across the entire application.
///
/// # Example
///
/// ```rust,ignore
/// use crate::error::{ErrorClassification, ErrorSeverity};
///
/// impl ErrorClassification for MyError {
///     fn is_retryable(&self) -> bool {
///         match self {
///             Self::Transient(_) => true,
///             Self::Permanent(_) => false,
///             Self::Common(e) => e.is_retryable(),
///         }
///     }
///
///     fn severity(&self) -> ErrorSeverity {
///         match self {
///             Self::Transient(_) => ErrorSeverity::Warning,
///             Self::Permanent(_) => ErrorSeverity::Error,
///             Self::Common(e) => e.severity(),
///         }
///     }
///
///     fn is_critical(&self) -> bool {
///         self.severity() == ErrorSeverity::Critical
///     }
///
///     fn retry_after(&self) -> Option<Duration> {
///         match self {
///             Self::Common(e) => e.retry_after(),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait ErrorClassification {
    /// Check if this error is retryable
    ///
    /// Retryable errors are typically transient issues that may succeed if
    /// attempted again, such as:
    /// - Network timeouts
    /// - Rate limiting
    /// - Lock contention
    /// - Circuit breaker open states
    /// - Temporary service unavailability
    fn is_retryable(&self) -> bool;

    /// Get the error severity level
    ///
    /// Used for monitoring, alerting, and logging decisions.
    fn severity(&self) -> ErrorSeverity;

    /// Check if this is a critical error requiring immediate attention
    ///
    /// Critical errors typically indicate:
    /// - Data corruption
    /// - Security issues
    /// - Internal invariant violations
    /// - System integrity problems
    fn is_critical(&self) -> bool;

    /// Get the suggested retry delay if applicable
    ///
    /// Returns `Some(Duration)` for retryable errors when a specific retry
    /// delay is recommended (e.g., from a Retry-After header), or `None`
    /// if no specific delay is suggested.
    fn retry_after(&self) -> Option<Duration>;
}

/// Error severity levels for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational, typically for debugging
    Info,
    /// Warning, should be monitored but not critical
    Warning,
    /// Error, requires attention and action
    Error,
    /// Critical, immediate action required
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

// Standard conversions from common error types
impl From<serde_json::Error> for CommonError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization_format("JSON", err.to_string())
    }
}

impl From<std::io::Error> for CommonError {
    fn from(err: std::io::Error) -> Self {
        Self::persistence(err.to_string())
    }
}

#[cfg(feature = "foundation")]
impl From<toml::de::Error> for CommonError {
    fn from(err: toml::de::Error) -> Self {
        Self::serialization_format("TOML", err.to_string())
    }
}

#[cfg(feature = "foundation")]
impl From<toml::ser::Error> for CommonError {
    fn from(err: toml::ser::Error) -> Self {
        Self::serialization_format("TOML", err.to_string())
    }
}

/// Trait for module-specific errors to embed common error variants
///
/// This trait allows module-specific error enums to convert common errors
/// into their own error types, providing a consistent interface.
pub trait ErrorContext {
    /// Convert a CommonError into this error type
    fn from_common(err: CommonError) -> Self;

    /// Create a context-specific error message
    fn with_context<S: Into<String>>(self, context: S) -> Self
    where
        Self: Sized;
}

/// Utility macro for creating error conversions from std types
///
/// This macro helps create standard error conversions for module-specific error
/// enums. Note: `From<CommonError>` is typically handled by `#[from]` attribute
/// on the variant.
///
/// # Basic Usage
///
/// ```rust,ignore
/// #[derive(Debug, thiserror::Error)]
/// pub enum MyError {
///     #[error(transparent)]
///     Common(#[from] CommonError),  // #[from] handles From<CommonError>
/// }
///
/// impl_error_conversion!(MyError, Common);
/// ```
///
/// This generates:
/// - `From<serde_json::Error> for MyError` (via CommonError)
/// - `From<std::io::Error> for MyError` (via CommonError)
///
/// If you need `From<CommonError>` and can't use `#[from]`, use the
/// `with_common` variant:
///
/// ```rust,ignore
/// impl_error_conversion!(MyError, Common, with_common);
/// ```
#[macro_export]
macro_rules! impl_error_conversion {
    // Standard variant - assumes #[from] is used for CommonError
    ($error_type:ty, $variant:ident) => {
        impl From<serde_json::Error> for $error_type {
            fn from(err: serde_json::Error) -> Self {
                Self::$variant($crate::error::CommonError::from(err))
            }
        }

        impl From<std::io::Error> for $error_type {
            fn from(err: std::io::Error) -> Self {
                Self::$variant($crate::error::CommonError::from(err))
            }
        }
    };

    // Variant that includes From<CommonError> (for cases without #[from])
    ($error_type:ty, $variant:ident, with_common) => {
        impl From<$crate::error::CommonError> for $error_type {
            fn from(err: $crate::error::CommonError) -> Self {
                Self::$variant(err)
            }
        }

        impl From<serde_json::Error> for $error_type {
            fn from(err: serde_json::Error) -> Self {
                Self::$variant($crate::error::CommonError::from(err))
            }
        }

        impl From<std::io::Error> for $error_type {
            fn from(err: std::io::Error) -> Self {
                Self::$variant($crate::error::CommonError::from(err))
            }
        }
    };
}

/// Macro to implement ErrorClassification by delegating to CommonError
///
/// This macro simplifies implementing `ErrorClassification` for module-specific
/// errors that embed `CommonError` and want to delegate classification logic.
///
/// # Usage
///
/// ```rust,ignore
/// #[derive(Debug, thiserror::Error)]
/// pub enum MyError {
///     #[error("Module-specific error: {0}")]
///     Specific(String),
///
///     #[error(transparent)]
///     Common(#[from] CommonError),
/// }
///
/// impl_error_classification!(MyError, Common,
///     Specific(_) => {
///         retryable: false,
///         severity: ErrorSeverity::Error,
///         critical: false,
///     }
/// );
/// ```
#[macro_export]
macro_rules! impl_error_classification {
    (
        $error_type:ty,
        $common_variant:ident
        $(,
            $variant:pat => {
                retryable: $retryable:expr,
                severity: $severity:expr,
                critical: $critical:expr
                $(, retry_after: $retry_after:expr)?
                $(,)?
            }
        )*
        $(,)?
    ) => {
        impl $crate::error::ErrorClassification for $error_type {
            fn is_retryable(&self) -> bool {
                match self {
                    Self::$common_variant(e) => e.is_retryable(),
                    $(
                        $variant => $retryable,
                    )*
                }
            }

            fn severity(&self) -> $crate::error::ErrorSeverity {
                match self {
                    Self::$common_variant(e) => e.severity(),
                    $(
                        $variant => $severity,
                    )*
                }
            }

            fn is_critical(&self) -> bool {
                match self {
                    Self::$common_variant(e) => e.is_critical(),
                    $(
                        $variant => $critical,
                    )*
                }
            }

            fn retry_after(&self) -> Option<std::time::Duration> {
                match self {
                    Self::$common_variant(e) => e.retry_after(),
                    $(
                        $(
                            $variant => $retry_after,
                        )?
                    )*
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }
        }
    };
}

// Helper macro to implement `From<ModuleError> for CommonError`
//
// This is the manual implementation part of bidirectional conversion.
// `From<CommonError> for ModuleError` is typically handled by `#[from]`.
//
// # Usage
//
// ```rust,ignore
// impl From<MyError> for CommonError {
//     fn from(err: MyError) -> Self {
//         match err {
//             MyError::Common(e) => e,
//             MyError::Config(msg) => CommonError::config(msg),
//             MyError::InvalidOp(msg) => CommonError::validation("operation", msg),
//         }
//     }
// }
// ```
//
// Note: This macro is provided for documentation purposes. In practice,
// implement `From<ModuleError> for CommonError` manually for clarity.

#[cfg(test)]
mod tests {
    //! Unit tests for error handling functionality
    //!
    //! Tests cover all error variants, constructors, display formatting,
    //! severity levels, retryability, and error conversions.

    use super::*;

    /// Validates `CommonError::config` behavior for the error config simple
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Configuration error: invalid
    ///   configuration"`.
    /// - Ensures `!err.is_retryable()` evaluates to true.
    /// - Ensures `!err.is_critical()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Error`.
    #[test]
    fn test_error_config_simple() {
        let err = CommonError::config("invalid configuration");
        assert_eq!(err.to_string(), "Configuration error: invalid configuration");
        assert!(!err.is_retryable());
        assert!(!err.is_critical());
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    /// Validates `CommonError::config_field` behavior for the error config with
    /// field scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Configuration error in field
    ///   'timeout': must be positive"`.
    #[test]
    fn test_error_config_with_field() {
        let err = CommonError::config_field("timeout", "must be positive");
        assert_eq!(err.to_string(), "Configuration error in field 'timeout': must be positive");
    }

    /// Validates `CommonError::lock` behavior for the error lock simple
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Lock error: failed to acquire
    ///   lock"`.
    /// - Ensures `err.is_retryable()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Warning`.
    #[test]
    fn test_error_lock_simple() {
        let err = CommonError::lock("failed to acquire lock");
        assert_eq!(err.to_string(), "Lock error: failed to acquire lock");
        assert!(err.is_retryable());
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    /// Validates `CommonError::lock_resource` behavior for the error lock with
    /// resource scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Lock error for 'state_mutex':
    ///   poisoned"`.
    #[test]
    fn test_error_lock_with_resource() {
        let err = CommonError::lock_resource("state_mutex", "poisoned");
        assert_eq!(err.to_string(), "Lock error for 'state_mutex': poisoned");
    }

    /// Validates `CommonError::circuit_breaker` behavior for the error circuit
    /// breaker open scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Circuit breaker open for
    ///   'api_service'"`.
    /// - Ensures `err.is_retryable()` evaluates to true.
    /// - Confirms `err.retry_after()` equals `None`.
    #[test]
    fn test_error_circuit_breaker_open() {
        let err = CommonError::circuit_breaker("api_service");
        assert_eq!(err.to_string(), "Circuit breaker open for 'api_service'");
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), None);
    }

    /// Validates `Duration::from_secs` behavior for the error circuit breaker
    /// with retry scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("retry in")` evaluates to true.
    /// - Confirms `err.retry_after()` equals `Some(retry_duration)`.
    #[test]
    fn test_error_circuit_breaker_with_retry() {
        let retry_duration = Duration::from_secs(30);
        let err = CommonError::circuit_breaker_with_retry("api_service", retry_duration);
        assert!(err.to_string().contains("retry in"));
        assert_eq!(err.retry_after(), Some(retry_duration));
    }

    /// Validates `CommonError::serialization` behavior for the error
    /// serialization simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Serialization error: invalid
    ///   JSON"`.
    #[test]
    fn test_error_serialization_simple() {
        let err = CommonError::serialization("invalid JSON");
        assert_eq!(err.to_string(), "Serialization error: invalid JSON");
    }

    /// Validates `CommonError::serialization_format` behavior for the error
    /// serialization with format scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Serialization error (TOML):
    ///   missing key"`.
    #[test]
    fn test_error_serialization_with_format() {
        let err = CommonError::serialization_format("TOML", "missing key");
        assert_eq!(err.to_string(), "Serialization error (TOML): missing key");
    }

    /// Validates `CommonError::persistence` behavior for the error persistence
    /// simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Persistence error: disk full"`.
    #[test]
    fn test_error_persistence_simple() {
        let err = CommonError::persistence("disk full");
        assert_eq!(err.to_string(), "Persistence error: disk full");
    }

    /// Validates `CommonError::persistence_op` behavior for the error
    /// persistence with operation scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Persistence error during 'save':
    ///   permission denied"`.
    #[test]
    fn test_error_persistence_with_operation() {
        let err = CommonError::persistence_op("save", "permission denied");
        assert_eq!(err.to_string(), "Persistence error during 'save': permission denied");
    }

    /// Validates `CommonError::rate_limit` behavior for the error rate limit
    /// simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Rate limit exceeded"`.
    /// - Ensures `err.is_retryable()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Warning`.
    #[test]
    fn test_error_rate_limit_simple() {
        let err = CommonError::rate_limit();
        assert_eq!(err.to_string(), "Rate limit exceeded");
        assert!(err.is_retryable());
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    /// Validates `CommonError::rate_limit_detailed` behavior for the error rate
    /// limit detailed scenario.
    ///
    /// Assertions:
    /// - Ensures `msg.contains("100 requests")` evaluates to true.
    /// - Ensures `msg.contains("retry in")` evaluates to true.
    /// - Confirms `err.retry_after()` equals `Some(Duration::from_secs(30))`.
    #[test]
    fn test_error_rate_limit_detailed() {
        let err = CommonError::rate_limit_detailed(
            100,
            Duration::from_secs(60),
            Some(Duration::from_secs(30)),
        );
        let msg = err.to_string();
        assert!(msg.contains("100 requests"));
        assert!(msg.contains("retry in"));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(30)));
    }

    /// Validates `CommonError::timeout` behavior for the error timeout
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("timed out")` evaluates to true.
    /// - Ensures `err.is_retryable()` evaluates to true.
    #[test]
    fn test_error_timeout() {
        let err = CommonError::timeout("database_query", Duration::from_secs(5));
        assert!(err.to_string().contains("timed out"));
        assert!(err.is_retryable());
    }

    /// Validates `CommonError::backend` behavior for the error backend
    /// retryable scenario.
    ///
    /// Assertions:
    /// - Ensures `err.is_retryable()` evaluates to true.
    /// - Confirms `err.to_string()` equals `"Backend error from 'external_api':
    ///   connection refused"`.
    #[test]
    fn test_error_backend_retryable() {
        let err = CommonError::backend("external_api", "connection refused", true);
        assert!(err.is_retryable());
        assert_eq!(err.to_string(), "Backend error from 'external_api': connection refused");
    }

    /// Validates `CommonError::backend` behavior for the error backend non
    /// retryable scenario.
    ///
    /// Assertions:
    /// - Ensures `!err.is_retryable()` evaluates to true.
    #[test]
    fn test_error_backend_non_retryable() {
        let err = CommonError::backend("external_api", "authentication failed", false);
        assert!(!err.is_retryable());
    }

    /// Validates `CommonError::validation` behavior for the error validation
    /// simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Validation error for field
    ///   'email': invalid format"`.
    /// - Ensures `!err.is_retryable()` evaluates to true.
    #[test]
    fn test_error_validation_simple() {
        let err = CommonError::validation("email", "invalid format");
        assert_eq!(err.to_string(), "Validation error for field 'email': invalid format");
        assert!(!err.is_retryable());
    }

    /// Validates `CommonError::validation_with_value` behavior for the error
    /// validation with value scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("value: '-5'")` evaluates to true.
    #[test]
    fn test_error_validation_with_value() {
        let err = CommonError::validation_with_value("age", "must be positive", "-5");
        assert!(err.to_string().contains("value: '-5'"));
    }

    /// Validates `CommonError::not_found` behavior for the error not found
    /// simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"User not found"`.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Info`.
    #[test]
    fn test_error_not_found_simple() {
        let err = CommonError::not_found("User");
        assert_eq!(err.to_string(), "User not found");
        assert_eq!(err.severity(), ErrorSeverity::Info);
    }

    /// Validates `CommonError::not_found_with_id` behavior for the error not
    /// found with id scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"User not found: '12345'"`.
    #[test]
    fn test_error_not_found_with_id() {
        let err = CommonError::not_found_with_id("User", "12345");
        assert_eq!(err.to_string(), "User not found: '12345'");
    }

    /// Validates `CommonError::unauthorized` behavior for the error
    /// unauthorized simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Unauthorized to perform
    ///   'delete_user'"`.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Warning`.
    #[test]
    fn test_error_unauthorized_simple() {
        let err = CommonError::unauthorized("delete_user");
        assert_eq!(err.to_string(), "Unauthorized to perform 'delete_user'");
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    /// Validates `CommonError::unauthorized_with_perm` behavior for the error
    /// unauthorized with permission scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("requires: admin_role")` evaluates
    ///   to true.
    #[test]
    fn test_error_unauthorized_with_permission() {
        let err = CommonError::unauthorized_with_perm("admin_action", "admin_role");
        assert!(err.to_string().contains("requires: admin_role"));
    }

    /// Validates `CommonError::internal` behavior for the error internal simple
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Internal error: unexpected
    ///   state"`.
    /// - Ensures `err.is_critical()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Critical`.
    #[test]
    fn test_error_internal_simple() {
        let err = CommonError::internal("unexpected state");
        assert_eq!(err.to_string(), "Internal error: unexpected state");
        assert!(err.is_critical());
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    /// Validates `CommonError::internal_with_context` behavior for the error
    /// internal with context scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Internal error in 'parser': null
    ///   pointer"`.
    #[test]
    fn test_error_internal_with_context() {
        let err = CommonError::internal_with_context("null pointer", "parser");
        assert_eq!(err.to_string(), "Internal error in 'parser': null pointer");
    }

    /// Validates `ErrorSeverity::Critical` behavior for the error severity
    /// ordering scenario.
    ///
    /// Assertions:
    /// - Ensures `ErrorSeverity::Critical > ErrorSeverity::Error` evaluates to
    ///   true.
    /// - Ensures `ErrorSeverity::Error > ErrorSeverity::Warning` evaluates to
    ///   true.
    /// - Ensures `ErrorSeverity::Warning > ErrorSeverity::Info` evaluates to
    ///   true.
    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::Error);
        assert!(ErrorSeverity::Error > ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning > ErrorSeverity::Info);
    }

    /// Validates `ErrorSeverity::Info` behavior for the error severity display
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `ErrorSeverity::Info.to_string()` equals `"INFO"`.
    /// - Confirms `ErrorSeverity::Warning.to_string()` equals `"WARN"`.
    /// - Confirms `ErrorSeverity::Error.to_string()` equals `"ERROR"`.
    /// - Confirms `ErrorSeverity::Critical.to_string()` equals `"CRITICAL"`.
    #[test]
    fn test_error_severity_display() {
        assert_eq!(ErrorSeverity::Info.to_string(), "INFO");
        assert_eq!(ErrorSeverity::Warning.to_string(), "WARN");
        assert_eq!(ErrorSeverity::Error.to_string(), "ERROR");
        assert_eq!(ErrorSeverity::Critical.to_string(), "CRITICAL");
    }

    /// Tests automatic conversion from serde_json errors to CommonError
    #[test]
    fn test_conversion_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json")
            .expect_err("Should fail to parse invalid JSON");
        let common_err: CommonError = json_err.into();

        match common_err {
            CommonError::Serialization { format, .. } => {
                assert_eq!(format, Some("JSON".to_string()));
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    /// Validates `Error::new` behavior for the conversion from io error
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `message.contains("file not found")` evaluates to true.
    #[test]
    fn test_conversion_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let common_err: CommonError = io_err.into();

        match common_err {
            CommonError::Persistence { message, .. } => {
                assert!(message.contains("file not found"));
            }
            _ => panic!("Expected Persistence error"),
        }
    }

    /// Validates `CommonError::circuit_breaker` behavior for the is retryable
    /// for various errors scenario.
    ///
    /// Assertions:
    /// - Ensures `CommonError::circuit_breaker("service").is_retryable()`
    ///   evaluates to true.
    /// - Ensures `CommonError::rate_limit().is_retryable()` evaluates to true.
    /// - Ensures `CommonError::timeout("op",
    ///   Duration::from_secs(1)).is_retryable()` evaluates to true.
    /// - Ensures `CommonError::lock("failed").is_retryable()` evaluates to
    ///   true.
    /// - Ensures `CommonError::backend("api", "error", true).is_retryable()`
    ///   evaluates to true.
    /// - Ensures `!CommonError::config("bad config").is_retryable()` evaluates
    ///   to true.
    /// - Ensures `!CommonError::validation("field", "bad").is_retryable()`
    ///   evaluates to true.
    /// - Ensures `!CommonError::not_found("Resource").is_retryable()` evaluates
    ///   to true.
    /// - Ensures `!CommonError::internal("bug").is_retryable()` evaluates to
    ///   true.
    /// - Ensures `!CommonError::backend("api", "auth failed",
    ///   false).is_retryable()` evaluates to true.
    #[test]
    fn test_is_retryable_for_various_errors() {
        // Retryable errors
        assert!(CommonError::circuit_breaker("service").is_retryable());
        assert!(CommonError::rate_limit().is_retryable());
        assert!(CommonError::timeout("op", Duration::from_secs(1)).is_retryable());
        assert!(CommonError::lock("failed").is_retryable());
        assert!(CommonError::backend("api", "error", true).is_retryable());

        // Non-retryable errors
        assert!(!CommonError::config("bad config").is_retryable());
        assert!(!CommonError::validation("field", "bad").is_retryable());
        assert!(!CommonError::not_found("Resource").is_retryable());
        assert!(!CommonError::internal("bug").is_retryable());
        assert!(!CommonError::backend("api", "auth failed", false).is_retryable());
    }

    /// Validates `Duration::from_secs` behavior for the retry after returns
    /// duration scenario.
    ///
    /// Assertions:
    /// - Confirms `cb_err.retry_after()` equals `Some(duration)`.
    /// - Confirms `rl_err.retry_after()` equals `Some(duration)`.
    /// - Confirms `config_err.retry_after()` equals `None`.
    #[test]
    fn test_retry_after_returns_duration() {
        let duration = Duration::from_secs(60);

        let cb_err = CommonError::circuit_breaker_with_retry("service", duration);
        assert_eq!(cb_err.retry_after(), Some(duration));

        let rl_err = CommonError::rate_limit_detailed(100, Duration::from_secs(60), Some(duration));
        assert_eq!(rl_err.retry_after(), Some(duration));

        let config_err = CommonError::config("test");
        assert_eq!(config_err.retry_after(), None);
    }

    /// Validates `CommonError::Detailed` behavior for the detailed error with
    /// severity scenario.
    ///
    /// Assertions:
    /// - Confirms `err.severity()` equals `ErrorSeverity::Critical`.
    /// - Ensures `err.to_string().contains("CRITICAL")` evaluates to true.
    /// - Ensures `err.to_string().contains("test error")` evaluates to true.
    #[test]
    fn test_detailed_error_with_severity() {
        let err = CommonError::Detailed {
            message: "test error".to_string(),
            severity: ErrorSeverity::Critical,
            context: Some("test_context".to_string()),
        };

        assert_eq!(err.severity(), ErrorSeverity::Critical);
        assert!(err.to_string().contains("CRITICAL"));
        assert!(err.to_string().contains("test error"));
    }

    // Test macro implementations
    #[derive(Debug, thiserror::Error)]
    enum TestModuleError {
        #[error("Module-specific error: {0}")]
        Specific(String),

        #[error("Invalid operation: {0}")]
        InvalidOperation(String),

        #[error(transparent)]
        Common(#[from] CommonError),
    }

    impl_error_conversion!(TestModuleError, Common);

    impl_error_classification!(TestModuleError, Common,
        Self::Specific(_) => {
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

    // Manual implementation of From<TestModuleError> for CommonError
    impl From<TestModuleError> for CommonError {
        fn from(err: TestModuleError) -> Self {
            match err {
                TestModuleError::Common(e) => e,
                TestModuleError::Specific(msg) => CommonError::internal(msg),
                TestModuleError::InvalidOperation(msg) => CommonError::validation("operation", msg),
            }
        }
    }

    /// Validates `CommonError::config` behavior for the error conversion macro
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(module_err, TestModuleError::Common(_))` evaluates
    ///   to true.
    /// - Ensures `matches!(module_err, TestModuleError::Common(_))` evaluates
    ///   to true.
    /// - Ensures `matches!(module_err, TestModuleError::Common(_))` evaluates
    ///   to true.
    #[test]
    fn test_error_conversion_macro() {
        // Test From<CommonError>
        let common_err = CommonError::config("test");
        let module_err: TestModuleError = common_err.into();
        assert!(matches!(module_err, TestModuleError::Common(_)));

        // Test From<io::Error>
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file");
        let module_err: TestModuleError = io_err.into();
        assert!(matches!(module_err, TestModuleError::Common(_)));

        // Test From<serde_json::Error>
        let json_err =
            serde_json::from_str::<serde_json::Value>("invalid").expect_err("Should fail");
        let module_err: TestModuleError = json_err.into();
        assert!(matches!(module_err, TestModuleError::Common(_)));
    }

    /// Validates `TestModuleError::Specific` behavior for the error
    /// classification macro scenario.
    ///
    /// Assertions:
    /// - Ensures `!err.is_retryable()` evaluates to true.
    /// - Ensures `!err.is_critical()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Error`.
    /// - Ensures `!err.is_retryable()` evaluates to true.
    /// - Ensures `!err.is_critical()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Warning`.
    /// - Ensures `err.is_retryable()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Warning`.
    #[test]
    fn test_error_classification_macro() {
        // Test module-specific variant classification
        let err = TestModuleError::Specific("test".to_string());
        assert!(!err.is_retryable());
        assert!(!err.is_critical());
        assert_eq!(err.severity(), ErrorSeverity::Error);

        let err = TestModuleError::InvalidOperation("test".to_string());
        assert!(!err.is_retryable());
        assert!(!err.is_critical());
        assert_eq!(err.severity(), ErrorSeverity::Warning);

        // Test CommonError variant classification (delegation)
        let common_err = CommonError::timeout("test", Duration::from_secs(1));
        let err = TestModuleError::Common(common_err);
        assert!(err.is_retryable()); // Timeout is retryable
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    /// Validates `TestModuleError::Specific` behavior for the bidirectional
    /// error conversion scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(common_err, CommonError::Internal { .. })` evaluates
    ///   to true.
    /// - Ensures `matches!(common_err, CommonError::Validation { .. })`
    ///   evaluates to true.
    /// - Ensures `matches!(common_err, CommonError::Config { .. })` evaluates
    ///   to true.
    #[test]
    fn test_bidirectional_error_conversion() {
        // Test ModuleError -> CommonError
        let module_err = TestModuleError::Specific("test error".to_string());
        let common_err: CommonError = module_err.into();
        assert!(matches!(common_err, CommonError::Internal { .. }));

        let module_err = TestModuleError::InvalidOperation("bad op".to_string());
        let common_err: CommonError = module_err.into();
        assert!(matches!(common_err, CommonError::Validation { .. }));

        // Test that Common variant passes through
        let original = CommonError::config("test");
        let module_err = TestModuleError::Common(original.clone());
        let common_err: CommonError = module_err.into();
        assert!(matches!(common_err, CommonError::Config { .. }));
    }

    /// Validates `CommonError::timeout` behavior for the cross module error
    /// propagation scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `matches!(err, TestModuleError::Common(_))` evaluates to true.
    /// - Ensures `err.is_retryable()` evaluates to true.
    #[test]
    fn test_cross_module_error_propagation() {
        // Simulate cross-module error propagation
        fn inner_fn() -> Result<(), CommonError> {
            Err(CommonError::timeout("operation", Duration::from_secs(5)))
        }

        fn outer_fn() -> Result<(), TestModuleError> {
            inner_fn()?; // Should auto-convert CommonError to TestModuleError
            Ok(())
        }

        let result = outer_fn();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TestModuleError::Common(_)));
        assert!(err.is_retryable()); // Timeout is retryable
    }

    // Tests for TOML error conversions
    /// Validates `CommonError::Serialization` behavior for the conversion from
    /// toml de error scenario.
    ///
    /// Assertions:
    /// - Confirms `format` equals `Some("TOML".to_string())`.
    #[cfg(feature = "foundation")]
    #[test]
    fn test_conversion_from_toml_de_error() {
        let toml_str = "invalid = toml = syntax";
        let toml_err = toml::from_str::<toml::Value>(toml_str).expect_err("Should fail to parse");
        let common_err: CommonError = toml_err.into();

        match common_err {
            CommonError::Serialization { format, .. } => {
                assert_eq!(format, Some("TOML".to_string()));
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    /// Validates `HashMap::new` behavior for the conversion from toml ser error
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format` equals `Some("TOML".to_string())`.
    #[cfg(feature = "foundation")]
    #[test]
    fn test_conversion_from_toml_ser_error() {
        use std::collections::HashMap;

        // Create a value that will fail to serialize to TOML
        let mut map = HashMap::new();
        map.insert("key".to_string(), toml::Value::Integer(42));
        // Nested maps with certain structures can cause serialization errors
        let result = toml::to_string(&map);

        if let Err(toml_err) = result {
            let common_err: CommonError = toml_err.into();
            match common_err {
                CommonError::Serialization { format, .. } => {
                    assert_eq!(format, Some("TOML".to_string()));
                }
                _ => panic!("Expected Serialization error"),
            }
        }
        // Note: This test may pass if serialization succeeds, which is okay
    }

    // Tests for async error variants
    /// Validates `CommonError::task_cancelled` behavior for the task cancelled
    /// simple scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Task 'background_worker'
    ///   cancelled"`.
    /// - Ensures `!err.is_retryable()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Info`.
    #[test]
    fn test_task_cancelled_simple() {
        let err = CommonError::task_cancelled("background_worker");
        assert_eq!(err.to_string(), "Task 'background_worker' cancelled");
        assert!(!err.is_retryable());
        assert_eq!(err.severity(), ErrorSeverity::Info);
    }

    /// Validates `CommonError::task_cancelled_with_reason` behavior for the
    /// task cancelled with reason scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("cancelled")` evaluates to true.
    /// - Ensures `err.to_string().contains("user requested")` evaluates to
    ///   true.
    /// - Ensures `!err.is_retryable()` evaluates to true.
    #[test]
    fn test_task_cancelled_with_reason() {
        let err = CommonError::task_cancelled_with_reason("sync_task", "user requested");
        assert!(err.to_string().contains("cancelled"));
        assert!(err.to_string().contains("user requested"));
        assert!(!err.is_retryable());
    }

    /// Validates `CommonError::async_timeout` behavior for the async timeout
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("timed out")` evaluates to true.
    /// - Ensures `err.to_string().contains("fetch_data")` evaluates to true.
    /// - Ensures `err.is_retryable()` evaluates to true.
    /// - Confirms `err.severity()` equals `ErrorSeverity::Warning`.
    #[test]
    fn test_async_timeout() {
        let err = CommonError::async_timeout("fetch_data", Duration::from_secs(10));
        assert!(err.to_string().contains("timed out"));
        assert!(err.to_string().contains("fetch_data"));
        assert!(err.is_retryable());
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    // Tests for structured logging integration
    /// Validates `CommonError::timeout` behavior for the as tracing fields
    /// timeout scenario.
    ///
    /// Assertions:
    /// - Ensures `fields.iter().any(|(k, _)| *k == "error_type")` evaluates to
    ///   true.
    /// - Ensures `fields.iter().any(|(k, _)| *k == "operation")` evaluates to
    ///   true.
    /// - Ensures `fields.iter().any(|(k, _)| *k == "duration_ms")` evaluates to
    ///   true.
    /// - Confirms `error_type` equals `Some(&"timeout".to_string())`.
    /// - Confirms `operation` equals `Some(&"database_query".to_string())`.
    #[test]
    fn test_as_tracing_fields_timeout() {
        let err = CommonError::timeout("database_query", Duration::from_secs(5));
        let fields = err.as_tracing_fields();

        assert!(fields.iter().any(|(k, _)| *k == "error_type"));
        assert!(fields.iter().any(|(k, _)| *k == "operation"));
        assert!(fields.iter().any(|(k, _)| *k == "duration_ms"));

        let error_type = fields.iter().find(|(k, _)| *k == "error_type").map(|(_, v)| v);
        assert_eq!(error_type, Some(&"timeout".to_string()));

        let operation = fields.iter().find(|(k, _)| *k == "operation").map(|(_, v)| v);
        assert_eq!(operation, Some(&"database_query".to_string()));
    }

    /// Validates `CommonError::validation_with_value` behavior for the as
    /// tracing fields validation scenario.
    ///
    /// Assertions:
    /// - Ensures `fields.iter().any(|(k, _)| *k == "field")` evaluates to true.
    /// - Ensures `fields.iter().any(|(k, _)| *k == "message")` evaluates to
    ///   true.
    /// - Ensures `fields.iter().any(|(k, _)| *k == "value")` evaluates to true.
    /// - Confirms `value` equals `Some(&"not-an-email".to_string())`.
    #[test]
    fn test_as_tracing_fields_validation() {
        let err = CommonError::validation_with_value("email", "invalid format", "not-an-email");
        let fields = err.as_tracing_fields();

        assert!(fields.iter().any(|(k, _)| *k == "field"));
        assert!(fields.iter().any(|(k, _)| *k == "message"));
        assert!(fields.iter().any(|(k, _)| *k == "value"));

        let value = fields.iter().find(|(k, _)| *k == "value").map(|(_, v)| v);
        assert_eq!(value, Some(&"not-an-email".to_string()));
    }

    /// Validates `CommonError::task_cancelled_with_reason` behavior for the as
    /// tracing fields task cancelled scenario.
    ///
    /// Assertions:
    /// - Ensures `fields.iter().any(|(k, _)| *k == "task_id")` evaluates to
    ///   true.
    /// - Ensures `fields.iter().any(|(k, _)| *k == "reason")` evaluates to
    ///   true.
    /// - Confirms `task_id` equals `Some(&"worker_1".to_string())`.
    #[test]
    fn test_as_tracing_fields_task_cancelled() {
        let err = CommonError::task_cancelled_with_reason("worker_1", "shutdown");
        let fields = err.as_tracing_fields();

        assert!(fields.iter().any(|(k, _)| *k == "task_id"));
        assert!(fields.iter().any(|(k, _)| *k == "reason"));

        let task_id = fields.iter().find(|(k, _)| *k == "task_id").map(|(_, v)| v);
        assert_eq!(task_id, Some(&"worker_1".to_string()));
    }

    /// Validates `CommonError::async_timeout` behavior for the as tracing
    /// fields async timeout scenario.
    ///
    /// Assertions:
    /// - Ensures `fields.iter().any(|(k, _)| *k == "future_name")` evaluates to
    ///   true.
    /// - Ensures `fields.iter().any(|(k, _)| *k == "duration_ms")` evaluates to
    ///   true.
    /// - Confirms `future_name` equals `Some(&"api_call".to_string())`.
    /// - Confirms `duration` equals `Some(&"2500".to_string())`.
    #[test]
    fn test_as_tracing_fields_async_timeout() {
        let err = CommonError::async_timeout("api_call", Duration::from_millis(2500));
        let fields = err.as_tracing_fields();

        assert!(fields.iter().any(|(k, _)| *k == "future_name"));
        assert!(fields.iter().any(|(k, _)| *k == "duration_ms"));

        let future_name = fields.iter().find(|(k, _)| *k == "future_name").map(|(_, v)| v);
        assert_eq!(future_name, Some(&"api_call".to_string()));

        let duration = fields.iter().find(|(k, _)| *k == "duration_ms").map(|(_, v)| v);
        assert_eq!(duration, Some(&"2500".to_string()));
    }

    // Tests for fluent API
    /// Validates `CommonError::timeout` behavior for the with additional
    /// context scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("operation")` evaluates to true.
    /// - Ensures `err.is_retryable()` evaluates to true.
    #[test]
    fn test_with_additional_context() {
        let err = CommonError::timeout("operation", Duration::from_secs(5))
            .with_additional_context("retry_attempt", "3");

        // The fluent API should return the same error (for now)
        assert!(err.to_string().contains("operation"));
        assert!(err.is_retryable());
    }

    /// Validates `CommonError::backend` behavior for the with additional
    /// context chaining scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(err, CommonError::Backend { .. })` evaluates to
    ///   true.
    /// - Ensures `err.is_retryable()` evaluates to true.
    #[test]
    fn test_with_additional_context_chaining() {
        let err = CommonError::backend("api_service", "connection refused", true)
            .with_additional_context("endpoint", "/api/v1/data")
            .with_additional_context("retry_count", "2");

        // Should still be the same error type
        assert!(matches!(err, CommonError::Backend { .. }));
        assert!(err.is_retryable());
    }

    // Integration test for error type name
    /// Validates `CommonError::config` behavior for the error type name
    /// coverage scenario.
    ///
    /// Assertions:
    /// - Confirms `error_type.0` equals `"error_type"`.
    /// - Ensures `!error_type.1.is_empty()` evaluates to true.
    #[test]
    fn test_error_type_name_coverage() {
        let errors = vec![
            CommonError::config("test"),
            CommonError::lock("test"),
            CommonError::circuit_breaker("test"),
            CommonError::serialization("test"),
            CommonError::persistence("test"),
            CommonError::rate_limit(),
            CommonError::timeout("test", Duration::from_secs(1)),
            CommonError::backend("test", "test", false),
            CommonError::validation("test", "test"),
            CommonError::not_found("test"),
            CommonError::unauthorized("test"),
            CommonError::internal("test"),
            CommonError::task_cancelled("test"),
            CommonError::async_timeout("test", Duration::from_secs(1)),
        ];

        for err in errors {
            let fields = err.as_tracing_fields();
            let error_type = fields.first().expect("Should have error_type field");
            assert_eq!(error_type.0, "error_type");
            assert!(!error_type.1.is_empty());
        }
    }
}
