//! Resilience patterns for fault tolerance and error handling
//!
//! This module provides **generic, reusable** resilience patterns including:
//! - **Circuit Breaker**: Prevents cascading failures by detecting and stopping
//!   repeated failures
//! - **Retry Logic**: Configurable retry strategies with exponential backoff
//!   and jitter
//!
//! These patterns help build robust systems that can handle transient failures
//! gracefully.
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

pub mod circuit_breaker;
pub mod retry;

// Re-export circuit breaker types
pub use circuit_breaker::{
    BoxedError, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerConfigBuilder,
    CircuitBreakerMetrics, CircuitState, Clock, ConfigError, ConfigResult, MockClock,
    ResilienceError, ResilienceResult, SyncCircuitBreaker, SystemClock,
};
// Re-export retry types
pub use retry::{
    policies, retry, retry_with_policy, BackoffStrategy, Jitter, RetryConfig, RetryConfigBuilder,
    RetryContext, RetryDecision, RetryError, RetryExecutor, RetryOutcome, RetryPolicy, RetryResult,
};
