// Retry module with exponential backoff, circuit breaker, and retry budget
// Reference: PART-1-OVERVIEW-AND-FOUNDATION.md lines 447-450

pub mod budget;

// Circuit breaker uses the unified implementation from common::resilience
// via the circuit_breaker_adapter which provides backward compatibility
pub mod circuit_breaker_adapter;
pub mod constants;
pub mod error;
pub mod metrics;
pub mod policies;
pub mod strategy;
pub mod time;
pub mod tracing;

// Metrics export module removed (feature flags removed)

pub use budget::RetryBudget;
// Use the adapter which wraps the unified circuit breaker implementation
pub use circuit_breaker_adapter::{CircuitBreaker, CircuitBreakerStats, CircuitState};
// Re-export Clock types from the adapter (which gets them from common::resilience)
pub use circuit_breaker_adapter::{ClockTrait as Clock, MockClock, SystemClock};
pub use error::{RetryError, RetryResult};
pub use metrics::RetryMetrics;
pub use policies::{RetryPolicies, RetryPolicyBuilder};
pub use strategy::{RetryCondition, RetryStrategy};
