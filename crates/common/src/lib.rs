//! Modular common utilities shared across PulseArc crates.
//!
//! # Safety and Quality
//!
//! This crate enforces strict safety and quality standards to ensure
//! reliability across all PulseArc components.
//!
//! # Feature Tiers
//!
//! Enable cargo features to opt into the tiers you need:
//! - `foundation`: errors, validation, utilities, collections, privacy
//! - `runtime`: async infrastructure (cache, time, resilience, sync, lifecycle)
//! - `platform`: platform integrations (auth, security, storage)
//! - `observability`: optional tracing and metrics (not included by default)

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(clippy::all, clippy::perf, clippy::complexity, clippy::suspicious)]

// Foundation tier
// -----------------------------------------------------------------
#[cfg(feature = "foundation")]
pub mod collections;
#[cfg(feature = "foundation")]
pub mod error;
#[cfg(feature = "foundation")]
pub mod validation;
#[cfg(feature = "foundation")]
#[macro_use]
pub mod utils;

// Runtime tier
// --------------------------------------------------------------------
#[cfg(feature = "runtime")]
pub mod cache;
#[cfg(feature = "runtime")]
pub mod crypto;
#[cfg(feature = "runtime")]
pub mod lifecycle;
#[cfg(feature = "runtime")]
pub mod observability;
#[cfg(feature = "runtime")]
pub mod privacy;
#[cfg(feature = "runtime")]
pub mod resilience;
#[cfg(feature = "runtime")]
pub mod sync;
#[cfg(feature = "runtime")]
pub mod time;

// Platform tier
// -------------------------------------------------------------------
#[cfg(feature = "platform")]
pub mod auth;
#[cfg(feature = "platform")]
pub mod compliance;
#[cfg(feature = "platform")]
pub mod security;
#[cfg(feature = "platform")]
pub mod storage;

// Testing utilities
// ---------------------------------------------------------------
#[cfg(any(feature = "runtime", feature = "test-utils", test))]
pub mod testing;

// Note: configuration helpers live in pulsearc-core.

// Re-export commonly used types and traits for convenience
// ------------------------
#[cfg(feature = "runtime")]
pub use crypto::{EncryptedData, EncryptionService as SymmetricEncryptionService};
#[cfg(feature = "foundation")]
pub use error::{CommonError, CommonResult, ErrorClassification, ErrorContext, ErrorSeverity};
#[cfg(feature = "runtime")]
pub use lifecycle::manager::{
    AsyncManager, ComponentHealth, ManagerController, ManagerHealth, ManagerLifecycle,
    ManagerMetadata, ManagerStatus, SharedState,
};
#[cfg(feature = "runtime")]
pub use lifecycle::state::{
    shared_state, AtomicCounter, ManagedState, SafeShare, SharedState as AsyncSharedState,
    StateBuilder, StateConfig, StateRegistry,
};
#[cfg(feature = "runtime")]
pub use resilience::{
    retry, retry_with_policy, BackoffStrategy, CircuitBreaker, CircuitBreakerConfig,
    CircuitBreakerConfigBuilder, CircuitBreakerMetrics, CircuitState, Clock, Jitter, MockClock,
    ResilienceError, ResilienceResult, RetryConfig, RetryConfigBuilder, RetryDecision, RetryError,
    RetryExecutor, RetryPolicy, RetryResult, SystemClock,
};
#[cfg(feature = "platform")]
pub use security::{KeychainError, KeychainProvider};
#[cfg(feature = "foundation")]
pub use utils::serde::duration_millis;
#[cfg(feature = "foundation")]
pub use validation::{
    CollectionValidator, CustomValidator, EmailValidator, FieldValidator, IpValidator,
    RangeValidator, RuleBuilder, RuleSet, StringValidator, UrlValidator, ValidationError,
    ValidationResult, ValidationRule, Validator,
};
