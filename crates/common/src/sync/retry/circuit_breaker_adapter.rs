//! Circuit breaker adapter for sync/retry module
//!
//! This module provides backward-compatible wrappers around the unified
//! circuit breaker implementation from `agent::common::resilience`.
//!
//! The unified implementation provides all the same functionality with
//! additional features like MockClock support and improved testing.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::resilience::{
    CircuitBreaker as UnifiedCircuitBreaker, CircuitBreakerConfig as UnifiedConfig,
    CircuitState as UnifiedCircuitState, Clock,
};
// Re-export Clock types for backward compatibility
pub use crate::resilience::{Clock as ClockTrait, MockClock, SystemClock};
use crate::sync::retry::constants::*;
use crate::sync::retry::error::RetryResult;

/// Circuit breaker states - compatible with sync/retry API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, limited requests allowed for testing
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, "Closed"),
            Self::Open => write!(f, "Open"),
            Self::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

impl From<UnifiedCircuitState> for CircuitState {
    fn from(state: UnifiedCircuitState) -> Self {
        match state {
            UnifiedCircuitState::Closed => CircuitState::Closed,
            UnifiedCircuitState::Open => CircuitState::Open,
            UnifiedCircuitState::HalfOpen => CircuitState::HalfOpen,
        }
    }
}

/// Circuit breaker adapter wrapping the unified implementation
///
/// Provides backward compatibility with the original sync/retry API
/// while using the enhanced unified implementation under the hood.
#[derive(Clone)]
pub struct CircuitBreaker<C: Clock = SystemClock> {
    inner: Arc<UnifiedCircuitBreaker<C>>,
}

/// Builder for configuring CircuitBreaker with custom parameters
pub struct CircuitBreakerBuilder<C: Clock + 'static = SystemClock> {
    failure_threshold: u64,
    success_threshold: u64,
    timeout: Duration,
    half_open_max_requests: u64,
    clock: C,
}

impl Default for CircuitBreakerBuilder<SystemClock> {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreakerBuilder<SystemClock> {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD.into(),
            success_threshold: DEFAULT_SUCCESS_THRESHOLD.into(),
            timeout: DEFAULT_CIRCUIT_TIMEOUT,
            half_open_max_requests: DEFAULT_HALF_OPEN_REQUESTS.into(),
            clock: SystemClock,
        }
    }
}

impl CircuitBreakerBuilder<SystemClock> {
    /// Build the circuit breaker with a custom clock
    pub fn build_with_clock<C: Clock + 'static>(self, clock: C) -> CircuitBreaker<C> {
        let config = UnifiedConfig::new()
            .failure_threshold(self.failure_threshold)
            .success_threshold(self.success_threshold)
            .timeout(self.timeout)
            .half_open_max_calls(self.half_open_max_requests)
            .build()
            .expect("Circuit breaker config should be valid");

        CircuitBreaker {
            inner: Arc::new(
                UnifiedCircuitBreaker::with_clock(config, clock)
                    .expect("Circuit breaker creation should succeed"),
            ),
        }
    }
}

impl<C: Clock + 'static> CircuitBreakerBuilder<C> {
    /// Configure failure threshold
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold.into();
        self
    }

    /// Configure success threshold
    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold.into();
        self
    }

    /// Configure timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Configure half-open max requests
    pub fn with_half_open_requests(mut self, max_requests: u32) -> Self {
        self.half_open_max_requests = max_requests.into();
        self
    }

    /// Build the circuit breaker with the configured parameters
    pub fn build(self) -> CircuitBreaker<C> {
        let config = UnifiedConfig::new()
            .failure_threshold(self.failure_threshold)
            .success_threshold(self.success_threshold)
            .timeout(self.timeout)
            .half_open_max_calls(self.half_open_max_requests)
            .build()
            .expect("Circuit breaker config should be valid");

        CircuitBreaker {
            inner: Arc::new(
                UnifiedCircuitBreaker::with_clock(config, self.clock)
                    .expect("Circuit breaker creation should succeed"),
            ),
        }
    }
}

impl std::fmt::Debug for CircuitBreaker<SystemClock> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker").field("state", &self.state()).finish()
    }
}

impl Default for CircuitBreaker<SystemClock> {
    fn default() -> Self {
        Self::with_clock(SystemClock)
    }
}

impl CircuitBreaker<SystemClock> {
    /// Create a new circuit breaker with system clock
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure failure threshold
    pub fn with_failure_threshold(self, threshold: u32) -> Self {
        CircuitBreakerBuilder::new().with_failure_threshold(threshold).build()
    }

    /// Configure success threshold
    pub fn with_success_threshold(self, threshold: u32) -> Self {
        CircuitBreakerBuilder::new().with_success_threshold(threshold).build()
    }

    /// Configure timeout duration
    pub fn with_timeout(self, timeout: Duration) -> Self {
        CircuitBreakerBuilder::new().with_timeout(timeout).build()
    }

    /// Configure half-open max requests
    pub fn with_half_open_requests(self, max_requests: u32) -> Self {
        CircuitBreakerBuilder::new().with_half_open_requests(max_requests).build()
    }
}

impl<C: Clock + 'static> CircuitBreaker<C> {
    /// Create a new circuit breaker with a custom clock (for testing)
    pub fn with_clock(clock: C) -> Self {
        let config = UnifiedConfig::new()
            .failure_threshold(DEFAULT_FAILURE_THRESHOLD.into())
            .success_threshold(DEFAULT_SUCCESS_THRESHOLD.into())
            .timeout(DEFAULT_CIRCUIT_TIMEOUT)
            .half_open_max_calls(DEFAULT_HALF_OPEN_REQUESTS.into())
            .build()
            .expect("Default circuit breaker config should be valid");

        Self {
            inner: Arc::new(
                UnifiedCircuitBreaker::with_clock(config, clock)
                    .expect("Circuit breaker creation should succeed"),
            ),
        }
    }

    /// Check if a request should be allowed through
    pub fn should_allow_request(&self) -> RetryResult<bool> {
        Ok(self.inner.can_execute())
    }

    /// Record a successful operation
    pub fn record_success(&self) -> RetryResult<()> {
        self.inner.record_success();
        Ok(())
    }

    /// Record a failed operation
    pub fn record_failure(&self) -> RetryResult<()> {
        self.inner.record_failure();
        Ok(())
    }

    /// Get the current circuit state
    pub fn state(&self) -> RetryResult<CircuitState> {
        Ok(self.inner.get_state().into())
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) -> RetryResult<()> {
        self.inner.reset();
        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> RetryResult<CircuitBreakerStats> {
        let metrics = self.inner.get_metrics();
        Ok(CircuitBreakerStats {
            state: metrics.state.into(),
            failure_count: metrics.failure_count as u32,
            success_count: metrics.success_count as u32,
            last_failure_time: metrics.last_failure_time,
            half_open_requests: metrics.half_open_calls as u32,
        })
    }
}

/// Statistics for the circuit breaker - backward compatible
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
    pub half_open_requests: u32,
}

impl fmt::Display for CircuitBreakerStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CircuitBreaker[state={}, failures={}, successes={}",
            self.state, self.failure_count, self.success_count
        )?;
        if let Some(last_failure) = self.last_failure_time {
            write!(f, ", last_failure={:?} ago", last_failure.elapsed())?;
        }
        if self.state == CircuitState::HalfOpen {
            write!(f, ", half_open_requests={}", self.half_open_requests)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::retry::circuit_breaker_adapter.
    use super::*;

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// adapter default scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.state` equals `CircuitState::Closed`.
    /// - Confirms `stats.failure_count` equals `0`.
    /// - Confirms `stats.success_count` equals `0`.
    #[test]
    fn test_circuit_breaker_adapter_default() {
        let cb = CircuitBreaker::default();
        let stats = cb.stats().unwrap();

        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.success_count, 0);
    }

    /// Validates `CircuitBreaker::new` behavior for the circuit breaker adapter
    /// builder scenario.
    ///
    /// Assertions:
    /// - Ensures `cb.should_allow_request().unwrap()` evaluates to true.
    #[test]
    fn test_circuit_breaker_adapter_builder() {
        let cb = CircuitBreaker::new()
            .with_failure_threshold(5)
            .with_success_threshold(3)
            .with_timeout(Duration::from_secs(30))
            .with_half_open_requests(2);

        assert!(cb.should_allow_request().unwrap());
    }

    /// Validates `CircuitBreaker::new` behavior for the adapter opens on
    /// threshold scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.state` equals `CircuitState::Open`.
    /// - Confirms `stats.failure_count` equals `3`.
    #[test]
    fn test_adapter_opens_on_threshold() {
        let cb = CircuitBreaker::new().with_failure_threshold(3);

        // Record failures up to threshold
        for _ in 0..3 {
            cb.record_failure().unwrap();
        }

        let stats = cb.stats().unwrap();
        assert_eq!(stats.state, CircuitState::Open);
        assert_eq!(stats.failure_count, 3);
    }

    /// Validates `MockClock::new` behavior for the adapter with mock clock
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.state().unwrap()` equals `CircuitState::Open`.
    /// - Ensures `cb.should_allow_request().unwrap()` evaluates to true.
    /// - Confirms `cb.state().unwrap()` equals `CircuitState::HalfOpen`.
    #[test]
    fn test_adapter_with_mock_clock() {
        let clock = MockClock::new();
        let cb = CircuitBreakerBuilder::new()
            .with_failure_threshold(2)
            .with_timeout(Duration::from_millis(50))
            .build_with_clock(clock.clone());

        // Open the circuit
        cb.record_failure().unwrap();
        cb.record_failure().unwrap();

        assert_eq!(cb.state().unwrap(), CircuitState::Open);

        // Advance clock past timeout
        clock.advance(Duration::from_millis(100));

        // Should transition to half-open and allow request
        assert!(cb.should_allow_request().unwrap());
        assert_eq!(cb.state().unwrap(), CircuitState::HalfOpen);
    }

    /// Validates `CircuitBreaker::new` behavior for the adapter stats display
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `display.contains("Closed")` evaluates to true.
    /// - Ensures `display.contains("failures=1")` evaluates to true.
    #[test]
    fn test_adapter_stats_display() {
        let cb = CircuitBreaker::new();
        cb.record_failure().unwrap();

        let stats = cb.stats().unwrap();
        let display = format!("{}", stats);

        assert!(display.contains("Closed"));
        assert!(display.contains("failures=1"));
    }
}
