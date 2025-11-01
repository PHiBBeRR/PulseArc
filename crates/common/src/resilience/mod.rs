//! Resilience patterns for fault tolerance and error handling
//!
//! This module provides **generic, reusable** resilience patterns including:
//! - **Circuit Breaker**: Prevents cascading failures by detecting and stopping
//!   repeated failures
//! - **Retry Logic**: Configurable retry strategies with exponential backoff
//!   and jitter
//! - **Rate Limiting**: Token bucket and leaky bucket algorithms for rate
//!   control
//! - **Bulkhead**: Limits concurrent operations to prevent resource exhaustion
//!
//! These patterns help build robust systems that can handle transient failures
//! gracefully.
//!
//! # Examples
//!
//! ## Basic Retry with Default Configuration
//!
//! ```rust
//! use pulsearc_common::resilience::{policies, retry};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let result = retry(policies::AlwaysRetry, || async {
//!     // Your fallible operation
//!     Ok::<_, std::io::Error>("success")
//! })
//! .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Retry Configuration
//!
//! ```rust
//! use std::time::Duration;
//!
//! use pulsearc_common::resilience::{policies, retry_with_policy, RetryConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = RetryConfig::new()
//!     .max_attempts(5)
//!     .exponential_backoff(Duration::from_millis(100), 2.0, Duration::from_secs(30))
//!     .full_jitter()
//!     .build()?;
//!
//! let result = retry_with_policy(config, policies::AlwaysRetry, || async {
//!     // Your operation
//!     Ok::<_, std::io::Error>(42)
//! })
//! .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Circuit Breaker
//!
//! ```rust
//! use std::time::Duration;
//!
//! use pulsearc_common::resilience::{CircuitBreaker, CircuitBreakerConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let breaker = CircuitBreaker::builder()
//!     .failure_threshold(5)
//!     .success_threshold(2)
//!     .timeout(Duration::from_secs(60))
//!     .build()?;
//!
//! let circuit_breaker = CircuitBreaker::new(breaker)?;
//!
//! let result = circuit_breaker.execute(|| async { Ok::<_, std::io::Error>(42) }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Combining Circuit Breaker and Retry
//!
//! ```rust
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use pulsearc_common::resilience::{
//!     policies, retry_with_policy, CircuitBreaker, CircuitBreakerConfig, RetryConfig,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let cb_config = CircuitBreakerConfig::new()
//!     .failure_threshold(3)
//!     .timeout(Duration::from_secs(30))
//!     .build()?;
//! let breaker = Arc::new(CircuitBreaker::new(cb_config)?);
//!
//! let retry_config = RetryConfig::new()
//!     .max_attempts(3)
//!     .exponential_backoff(Duration::from_millis(100), 2.0, Duration::from_secs(5))
//!     .build()?;
//!
//! let result =
//!     retry_with_policy(retry_config, policies::AlwaysRetry, || {
//!         let breaker = Arc::clone(&breaker);
//!         async move {
//!             breaker.execute(|| async { Ok::<_, std::io::Error>("Protected operation") }).await
//!         }
//!     })
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture: Generic Library vs Domain-Specific Implementation
//!
//! This `resilience` module provides **library-quality, generic abstractions**
//! that can be reused across different domains and applications. The
//! implementations are:
//! - Generic over error types (`<E: std::error::Error>`)
//! - Flexible with multiple strategies (4 backoff types, 4 jitter types)
//! - Testable with clock abstraction (`MockClock`)
//! - Framework-agnostic with minimal dependencies
//!
//! ### Relationship to `sync::retry`
//!
//! The `agent::common::sync::retry` module contains a **domain-specific
//! implementation** optimized for the sync/queue operations. Key differences:
//!
//! | Feature | `common::resilience` | `common::sync::retry` |
//! |---------|---------------------|----------------------|
//! | **Purpose** | Generic library | Sync-specific production code |
//! | **Error Type** | Generic `<E>` | Concrete `RetryError` |
//! | **Metrics** | None (generic) | Integrated `RetryMetrics` |
//! | **Tracing** | Basic logging | Feature-gated spans |
//! | **Backoff** | 4 strategies | Exponential only |
//! | **Domain Coupling** | None | Coupled to sync module |
//! | **Current Usage** | Reserved for future | Active production use |
//!
//! ### When to Use Each
//!
//! - **Use `common::resilience`**: When adding resilience to **new modules** or
//!   **different domains** that need circuit breakers or retry logic. This
//!   provides a clean, generic foundation.
//!
//! - **Use `common::sync::retry`**: When working **within the sync/queue
//!   domain** where you need integrated metrics, tracing, and domain-specific
//!   error handling.
//!
//! ### Future Direction
//!
//! Long-term, consider migrating `sync::retry` to use `resilience` as a
//! backend, adding metrics and tracing as wrapper layers. This would eliminate
//! duplication while maintaining the specialized functionality. Priority: LOW
//! (both implementations work well, no bugs reported).

pub mod adaptive;
pub mod bulkhead;
pub mod circuit_breaker;
pub mod histogram;
pub mod rate_limiter;
pub mod retry;

// Re-export adaptive circuit breaker types
pub use adaptive::{
    AdaptiveCircuitBreaker, AdaptiveCircuitBreakerConfig, AdaptiveCircuitBreakerConfigBuilder,
    AdaptiveCircuitBreakerMetrics, AdaptiveCircuitState,
};
// Re-export bulkhead types
pub use bulkhead::{Bulkhead, BulkheadConfig, BulkheadConfigBuilder, BulkheadMetrics};
// Re-export circuit breaker types
pub use circuit_breaker::{
    BoxedError, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerConfigBuilder,
    CircuitBreakerMetrics, CircuitState, Clock, ConfigError, ConfigResult, MockClock,
    ResilienceError, ResilienceResult, SyncCircuitBreaker, SystemClock,
};
// Re-export histogram types
pub use histogram::{Histogram, HistogramSnapshot, Percentiles};
// Re-export rate limiter types
pub use rate_limiter::{
    LeakyBucket, LeakyBucketConfig, LeakyBucketConfigBuilder, TokenBucket, TokenBucketConfig,
    TokenBucketConfigBuilder,
};
// Re-export retry types
pub use retry::{
    policies, retry, retry_with_policy, BackoffStrategy, Jitter, RetryConfig, RetryConfigBuilder,
    RetryContext, RetryDecision, RetryError, RetryExecutor, RetryOutcome, RetryPolicy, RetryResult,
};
