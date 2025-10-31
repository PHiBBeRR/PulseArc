//! Synchronization primitives for reliable distributed systems
//!
//! This module provides domain-specific sync infrastructure for reliable
//! message queue operations.
//!
//! ## Submodules
//!
//! - **`queue`**: Enterprise-grade sync queue with persistence, compression,
//!   encryption
//! - **`retry`**: Retry logic integrated with circuit breaker, retry budget,
//!   and metrics
//!
//! ## Module Relationships
//!
//! The `sync::retry` module provides domain-specific retry implementations
//! optimized for queue operations. It integrates multiple resilience patterns:
//!
//! - Circuit breaker (via adapter to `resilience::CircuitBreaker`)
//! - Retry strategy with exponential backoff
//! - Retry budget for controlling retry load
//! - Integrated metrics and tracing
//!
//! For generic, reusable resilience patterns, see the `resilience` module which
//! provides library-quality abstractions without domain coupling.

pub mod queue;
pub mod retry;

// Re-export commonly used types from retry
// Re-export commonly used types from queue
pub use queue::{
    CompressionAlgorithm, CompressionService, ItemStatus, Priority, QueueConfig, QueueError,
    QueueMetrics, QueueMetricsSnapshot, QueueResult, SyncItem, SyncQueue,
};
// Re-export retry types
pub use retry::{
    CircuitBreaker, CircuitBreakerStats, CircuitState, RetryBudget, RetryError, RetryMetrics,
    RetryPolicies, RetryPolicyBuilder, RetryResult, RetryStrategy,
};

// Re-export time abstractions from testing module
pub use crate::testing::time::{Clock, MockClock, SystemClock};

// Backward compatibility aliases
pub type MessageQueue = SyncQueue;
pub type MessageQueueConfig = QueueConfig;
pub type QueueMessage = SyncItem;
