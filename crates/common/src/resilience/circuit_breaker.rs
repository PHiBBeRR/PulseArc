//! Resilience patterns for building fault-tolerant systems
//!
//! This module provides generic implementations of common resilience patterns
//! including circuit breakers, bulkheads, timeouts, and rate limiting.
//! These patterns help prevent cascading failures and improve system
//! reliability.

use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::{debug, info, instrument, warn};

//==============================================================================
// Time Abstraction for Testability
//==============================================================================

/// Trait for time operations to enable deterministic testing
///
/// This trait allows circuit breakers to use real system time in production
/// and controlled mock time in tests, enabling deterministic testing of
/// timeout-based behavior without actual time delays.
pub trait Clock: Send + Sync + 'static {
    /// Get current instant (monotonic time)
    fn now(&self) -> Instant;

    /// Get current system time (wall clock)
    fn system_time(&self) -> SystemTime;

    /// Get milliseconds since UNIX epoch
    fn millis_since_epoch(&self) -> u64 {
        self.system_time().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }
}

/// Real system clock implementation for production use
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn system_time(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Implement Clock for Arc<T> where T: Clock for convenient cloning
impl<T: Clock> Clock for Arc<T> {
    fn now(&self) -> Instant {
        (**self).now()
    }

    fn system_time(&self) -> SystemTime {
        (**self).system_time()
    }
}

/// Mock clock for deterministic testing
///
/// Allows tests to control time progression without actual delays,
/// enabling fast and reliable testing of timeout-based behavior.
#[derive(Debug, Clone)]
pub struct MockClock {
    start: Instant,
    elapsed: Arc<Mutex<Duration>>,
}

impl MockClock {
    /// Create a new mock clock starting at the current instant
    pub fn new() -> Self {
        Self { start: Instant::now(), elapsed: Arc::new(Mutex::new(Duration::ZERO)) }
    }

    /// Create a new mock clock with a specific start time
    ///
    /// This is useful for tests that need deterministic start times.
    pub fn with_current_time(start: Instant) -> Self {
        Self { start, elapsed: Arc::new(Mutex::new(Duration::ZERO)) }
    }

    /// Advance the mock clock by a duration
    ///
    /// This simulates the passage of time without actual delays,
    /// useful for testing timeout behavior.
    pub fn advance(&self, duration: Duration) {
        if let Ok(mut elapsed) = self.elapsed.lock() {
            *elapsed += duration;
        }
    }

    /// Advance the mock clock by milliseconds (convenience method)
    ///
    /// Equivalent to `advance(Duration::from_millis(millis))`.
    pub fn advance_millis(&self, millis: u64) {
        self.advance(Duration::from_millis(millis));
    }

    /// Set the mock clock to a specific elapsed time
    pub fn set_elapsed(&self, duration: Duration) {
        if let Ok(mut elapsed) = self.elapsed.lock() {
            *elapsed = duration;
        }
    }

    /// Get the current elapsed time
    pub fn elapsed(&self) -> Duration {
        self.elapsed.lock().map(|e| *e).unwrap_or(Duration::ZERO)
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        let elapsed = self.elapsed.lock().map(|e| *e).unwrap_or(Duration::ZERO);
        self.start + elapsed
    }

    fn system_time(&self) -> SystemTime {
        let elapsed = self.elapsed.lock().map(|e| *e).unwrap_or(Duration::ZERO);
        SystemTime::UNIX_EPOCH + elapsed
    }
}

//==============================================================================
// Error Types
//==============================================================================

/// Simple configuration error for validation
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {message}")]
    Invalid { message: String },
}

/// Errors that can occur in resilience operations
///
/// This error type is generic over the underlying operation error type `E`,
/// allowing it to wrap and preserve the original error information while
/// providing resilience-specific error variants.
#[derive(Debug, Error)]
pub enum ResilienceError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Circuit breaker is open, rejecting calls
    #[error("Circuit breaker is open, rejecting calls")]
    CircuitOpen,

    /// Operation timed out
    #[error("Operation timed out after {timeout:?}")]
    Timeout { timeout: Duration },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {requests} requests in {window:?}")]
    RateLimitExceeded { requests: u64, window: Duration },

    /// Bulkhead capacity exceeded
    #[error("Bulkhead capacity exceeded: {capacity} concurrent operations")]
    BulkheadFull { capacity: usize },

    /// The underlying operation failed
    #[error("Operation failed")]
    OperationFailed {
        #[source]
        source: E,
    },

    /// Configuration error
    #[error("Invalid configuration: {message}")]
    InvalidConfiguration { message: String },
}

/// Boxed error type for configuration and simple errors
pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Result type for resilience operations
pub type ResilienceResult<T, E> = Result<T, ResilienceError<E>>;

/// Configuration result type using simple config errors
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, allowing requests
    Closed,
    /// Circuit is open, rejecting requests
    Open,
    /// Circuit is half-open, allowing limited requests to test recovery
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "CLOSED"),
            CircuitState::Open => write!(f, "OPEN"),
            CircuitState::HalfOpen => write!(f, "HALF_OPEN"),
        }
    }
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u64,
    /// Number of successes needed to close the circuit from half-open
    pub success_threshold: u64,
    /// Time to wait before transitioning from open to half-open
    pub timeout: Duration,
    /// Maximum number of calls allowed in half-open state
    pub half_open_max_calls: u64,
    /// Whether to reset failure count on success in closed state
    pub reset_on_success: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
            reset_on_success: true,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new configuration with validation
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::new()
    }

    /// Create a configuration builder (alias for `new()`)
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> ConfigResult<()> {
        if self.failure_threshold == 0 {
            return Err(ConfigError::Invalid {
                message: "failure_threshold must be greater than 0".to_string(),
            });
        }

        if self.success_threshold == 0 {
            return Err(ConfigError::Invalid {
                message: "success_threshold must be greater than 0".to_string(),
            });
        }

        if self.half_open_max_calls == 0 {
            return Err(ConfigError::Invalid {
                message: "half_open_max_calls must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Builder for CircuitBreakerConfig
#[derive(Debug)]
pub struct CircuitBreakerConfigBuilder {
    config: CircuitBreakerConfig,
}

impl Default for CircuitBreakerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreakerConfigBuilder {
    pub fn new() -> Self {
        Self { config: CircuitBreakerConfig::default() }
    }

    pub fn failure_threshold(mut self, threshold: u64) -> Self {
        self.config.failure_threshold = threshold;
        self
    }

    pub fn success_threshold(mut self, threshold: u64) -> Self {
        self.config.success_threshold = threshold;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn half_open_max_calls(mut self, max_calls: u64) -> Self {
        self.config.half_open_max_calls = max_calls;
        self
    }

    pub fn reset_on_success(mut self, reset: bool) -> Self {
        self.config.reset_on_success = reset;
        self
    }

    /// Set a custom clock for the circuit breaker (useful for testing)
    pub fn clock<C: Clock>(self, clock: C) -> CircuitBreakerBuilderWithClock<C> {
        CircuitBreakerBuilderWithClock { config: self.config, clock }
    }

    pub fn build(self) -> ConfigResult<CircuitBreakerConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Builder with custom clock that builds a CircuitBreaker directly
pub struct CircuitBreakerBuilderWithClock<C: Clock> {
    config: CircuitBreakerConfig,
    clock: C,
}

impl<C: Clock> CircuitBreakerBuilderWithClock<C> {
    pub fn failure_threshold(mut self, threshold: u64) -> Self {
        self.config.failure_threshold = threshold;
        self
    }

    pub fn success_threshold(mut self, threshold: u64) -> Self {
        self.config.success_threshold = threshold;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn half_open_max_calls(mut self, max_calls: u64) -> Self {
        self.config.half_open_max_calls = max_calls;
        self
    }

    pub fn reset_on_success(mut self, reset: bool) -> Self {
        self.config.reset_on_success = reset;
        self
    }

    pub fn build(self) -> ConfigResult<CircuitBreaker<C>> {
        CircuitBreaker::with_clock(self.config, self.clock)
    }
}

/// Circuit breaker metrics for monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub failure_count: u64,
    pub success_count: u64,
    pub half_open_calls: u64,
    pub total_calls: u64,
    pub last_failure_time: Option<Instant>,
    pub state_change_time: Instant,
}

/// Generic circuit breaker implementation
///
/// The circuit breaker prevents cascading failures by monitoring operation
/// failures and temporarily blocking calls when a failure threshold is reached.
///
/// Supports both async and sync operations, with configurable Clock for
/// testing.
pub struct CircuitBreaker<C: Clock = SystemClock> {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    half_open_calls: Arc<AtomicU64>,
    total_calls: Arc<AtomicU64>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    state_change_time: Arc<RwLock<Instant>>,
    clock: Arc<C>,
}

impl<C: Clock> fmt::Debug for CircuitBreaker<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("config", &self.config)
            .field("state", &self.get_state())
            .field("failure_count", &self.failure_count.load(Ordering::Acquire))
            .field("success_count", &self.success_count.load(Ordering::Acquire))
            .finish()
    }
}

impl<C: Clock> Clone for CircuitBreaker<C> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            failure_count: Arc::clone(&self.failure_count),
            success_count: Arc::clone(&self.success_count),
            half_open_calls: Arc::clone(&self.half_open_calls),
            total_calls: Arc::clone(&self.total_calls),
            last_failure_time: Arc::clone(&self.last_failure_time),
            state_change_time: Arc::clone(&self.state_change_time),
            clock: Arc::clone(&self.clock),
        }
    }
}

// Type aliases for common use cases
pub type SyncCircuitBreaker = CircuitBreaker<SystemClock>;

impl CircuitBreaker<SystemClock> {
    /// Create a new circuit breaker with the given configuration using system
    /// clock
    pub fn new(config: CircuitBreakerConfig) -> ConfigResult<Self> {
        Self::with_clock(config, SystemClock)
    }

    /// Create a circuit breaker with default configuration (convenience method)
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default()).expect("Default config should be valid")
    }

    /// Create a circuit breaker using the builder pattern
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::new()
    }
}

impl<C: Clock> CircuitBreaker<C> {
    /// Create a new circuit breaker with a custom clock (useful for testing)
    pub fn with_clock(config: CircuitBreakerConfig, clock: C) -> ConfigResult<Self> {
        config.validate()?;

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            half_open_calls: Arc::new(AtomicU64::new(0)),
            total_calls: Arc::new(AtomicU64::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            state_change_time: Arc::new(RwLock::new(clock.now())),
            clock: Arc::new(clock),
        })
    }

    /// Fast check if circuit is available (lock-free when possible)
    ///
    /// This is a lightweight check useful for high-frequency polling scenarios.
    /// For complete state transition logic, use `can_execute()`.
    pub fn is_available(&self) -> bool {
        match self.state.read() {
            Ok(guard) => *guard != CircuitState::Open,
            Err(poisoned) => {
                warn!("Circuit breaker state lock poisoned in is_available");
                *poisoned.into_inner() != CircuitState::Open
            }
        }
    }

    /// Check if the circuit breaker allows execution
    ///
    /// Returns `false` if the circuit is open and the timeout hasn't elapsed,
    /// or if we're in half-open state and have reached the maximum calls.
    /// Returns `true` otherwise, potentially transitioning from open to
    /// half-open.
    pub fn can_execute(&self) -> bool {
        let state = match self.state.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                // If the lock is poisoned, we still want to try to recover
                warn!("Circuit breaker state lock poisoned: {}", poisoned);
                // Clear the poison and return the data
                *poisoned.into_inner()
            }
        };

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed to potentially transition to half-open
                if let Ok(last_failure_guard) = self.last_failure_time.read() {
                    if let Some(failure_time) = *last_failure_guard {
                        let now = self.clock.now();
                        if now.duration_since(failure_time) >= self.config.timeout {
                            drop(last_failure_guard); // Release lock before state transition
                                                      // Transition to half-open
                            if let Ok(mut state) = self.state.write() {
                                *state = CircuitState::HalfOpen;
                                self.half_open_calls.store(0, Ordering::Release);
                            }
                            return true;
                        }
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                let current_calls = self.half_open_calls.load(Ordering::Acquire);
                current_calls < self.config.half_open_max_calls
            }
        }
    }

    /// Execute an operation with circuit breaker protection
    ///
    /// This method checks if the circuit allows execution, runs the operation
    /// if allowed, and records the result to update the circuit state.
    #[instrument(skip(self, operation), fields(state = %self.get_state()))]
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> ResilienceResult<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.can_execute() {
            debug!("Circuit breaker rejecting call - state: {}", self.get_state());
            return Err(ResilienceError::CircuitOpen);
        }

        self.total_calls.fetch_add(1, Ordering::Relaxed);

        let current_state = self.get_state();
        if current_state == CircuitState::HalfOpen {
            self.half_open_calls.fetch_add(1, Ordering::Relaxed);
        }

        match operation().await {
            Ok(result) => {
                self.record_success();
                debug!("Circuit breaker: operation succeeded");
                Ok(result)
            }
            Err(error) => {
                self.record_failure();
                warn!("Circuit breaker: operation failed - {:?}", error);
                Err(ResilienceError::OperationFailed { source: error })
            }
        }
    }

    /// Execute a synchronous operation with circuit breaker protection
    ///
    /// This method provides a synchronous alternative to `execute()`, useful
    /// for non-async contexts or when wrapping sync operations.
    #[instrument(skip(self, operation), fields(state = %self.get_state()))]
    pub fn call<F, T, E>(&self, operation: F) -> ResilienceResult<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.can_execute() {
            debug!("Circuit breaker rejecting call - state: {}", self.get_state());
            return Err(ResilienceError::CircuitOpen);
        }

        self.total_calls.fetch_add(1, Ordering::Relaxed);

        let current_state = self.get_state();
        if current_state == CircuitState::HalfOpen {
            self.half_open_calls.fetch_add(1, Ordering::Relaxed);
        }

        match operation() {
            Ok(result) => {
                self.record_success();
                debug!("Circuit breaker: operation succeeded");
                Ok(result)
            }
            Err(error) => {
                self.record_failure();
                warn!("Circuit breaker: operation failed");
                Err(ResilienceError::OperationFailed { source: error })
            }
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        let current_state = self.get_state();
        self.success_count.fetch_add(1, Ordering::Relaxed);

        match current_state {
            CircuitState::Closed => {
                if self.config.reset_on_success {
                    self.failure_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::HalfOpen => {
                let success_count = self.success_count.load(Ordering::Acquire);
                if success_count >= self.config.success_threshold {
                    // Transition to closed
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = CircuitState::Closed;
                        self.failure_count.store(0, Ordering::Release);
                    }
                    info!("Circuit breaker closed after {} successes", success_count);
                }
            }
            CircuitState::Open => {
                // This shouldn't happen, but handle it gracefully
                warn!("Received success while circuit is open");
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let now = self.clock.now();

        // Update last failure time
        if let Ok(mut last_failure) = self.last_failure_time.write() {
            *last_failure = Some(now);
        }

        let current_state = self.get_state();

        match current_state {
            CircuitState::Closed => {
                if failure_count >= self.config.failure_threshold {
                    // Transition to open
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = CircuitState::Open;
                        if let Ok(mut last_failure) = self.last_failure_time.write() {
                            *last_failure = Some(now);
                        }
                    }
                    warn!("Circuit breaker opened after {} failures", failure_count);
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state immediately opens the circuit
                // Transition to open
                if let Ok(mut state_guard) = self.state.write() {
                    *state_guard = CircuitState::Open;
                    if let Ok(mut last_failure) = self.last_failure_time.write() {
                        *last_failure = Some(now);
                    }
                }
                warn!("Circuit breaker opened due to failure in half-open state");
            }
            CircuitState::Open => {
                // Already open, just update metrics
            }
        }
    }

    /// Get the current state of the circuit breaker
    ///
    /// Returns the current circuit state, handling poisoned locks gracefully
    /// by returning the state from the poisoned data (fail-safe behavior).
    pub fn get_state(&self) -> CircuitState {
        match self.state.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                warn!("Circuit breaker state lock poisoned during get_state");
                *poisoned.into_inner()
            }
        }
    }

    /// Get circuit breaker metrics
    pub fn get_metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            state: self.get_state(),
            failure_count: self.failure_count.load(Ordering::Acquire),
            success_count: self.success_count.load(Ordering::Acquire),
            half_open_calls: self.half_open_calls.load(Ordering::Acquire),
            total_calls: self.total_calls.load(Ordering::Acquire),
            last_failure_time: self.last_failure_time.read().ok().and_then(|guard| *guard),
            state_change_time: self
                .state_change_time
                .read()
                .ok()
                .map(|guard| *guard)
                .unwrap_or_else(Instant::now),
        }
    }

    /// Get the current circuit state (alias for `get_state()`)
    ///
    /// Convenience method that returns the current state of the circuit
    /// breaker.
    pub fn state(&self) -> CircuitState {
        self.get_state()
    }

    /// Get current metrics snapshot (alias for `get_metrics()`)
    ///
    /// Convenience method that returns a snapshot of current circuit breaker
    /// metrics.
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        self.get_metrics()
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.half_open_calls.store(0, Ordering::Relaxed);

        if let Ok(mut last_failure) = self.last_failure_time.write() {
            *last_failure = None;
        }

        // Reset to closed state
        if let Ok(mut state_guard) = self.state.write() {
            *state_guard = CircuitState::Closed;
            self.failure_count.store(0, Ordering::Release);
            self.success_count.store(0, Ordering::Release);
        }
        info!("Circuit breaker manually reset to closed state");
    }
}

impl Default for CircuitBreaker<SystemClock> {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for resilience patterns
    //!
    //! Tests cover circuit breaker state transitions, configuration validation,
    //! failure/success thresholds, timeout behavior, and concurrent access
    //! patterns.

    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

    use super::*;

    // =========================================================================
    // Clock Tests
    // =========================================================================

    /// Validates the system clock now scenario.
    ///
    /// Assertions:
    /// - Ensures `now2 >= now1` evaluates to true.
    #[test]
    fn test_system_clock_now() {
        let clock = SystemClock;
        let now1 = clock.now();
        let now2 = clock.now();
        assert!(now2 >= now1, "System clock should advance");
    }

    /// Validates `SystemTime::UNIX_EPOCH` behavior for the system clock system
    /// time scenario.
    ///
    /// Assertions:
    /// - Ensures `time > SystemTime::UNIX_EPOCH` evaluates to true.
    #[test]
    fn test_system_clock_system_time() {
        let clock = SystemClock;
        let time = clock.system_time();
        assert!(time > SystemTime::UNIX_EPOCH, "System time should be after Unix epoch");
    }

    /// Validates `MockClock::new` behavior for the mock clock new scenario.
    ///
    /// Assertions:
    /// - Confirms `clock.elapsed()` equals `Duration::ZERO`.
    #[test]
    fn test_mock_clock_new() {
        let clock = MockClock::new();
        assert_eq!(clock.elapsed(), Duration::ZERO, "New mock clock should start at zero");
    }

    /// Validates `MockClock::new` behavior for the mock clock advance scenario.
    ///
    /// Assertions:
    /// - Confirms `after.duration_since(start)` equals
    ///   `Duration::from_secs(5)`.
    #[test]
    fn test_mock_clock_advance() {
        let clock = MockClock::new();
        let start = clock.now();

        clock.advance(Duration::from_secs(5));
        let after = clock.now();

        assert_eq!(
            after.duration_since(start),
            Duration::from_secs(5),
            "Mock clock should advance by specified duration"
        );
    }

    /// Validates `MockClock::new` behavior for the mock clock set elapsed
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `clock.elapsed()` equals `Duration::from_secs(10)`.
    /// - Confirms `clock.elapsed()` equals `Duration::from_secs(20)`.
    #[test]
    fn test_mock_clock_set_elapsed() {
        let clock = MockClock::new();

        clock.set_elapsed(Duration::from_secs(10));
        assert_eq!(clock.elapsed(), Duration::from_secs(10));

        clock.set_elapsed(Duration::from_secs(20));
        assert_eq!(clock.elapsed(), Duration::from_secs(20));
    }

    /// Validates `MockClock::new` behavior for the mock clock millis since
    /// epoch scenario.
    ///
    /// Assertions:
    /// - Confirms `millis` equals `5000`.
    #[test]
    fn test_mock_clock_millis_since_epoch() {
        let clock = MockClock::new();
        clock.set_elapsed(Duration::from_millis(5000));

        let millis = clock.millis_since_epoch();
        assert_eq!(millis, 5000);
    }

    /// Validates `MockClock::new` behavior for the mock clock clone scenario.
    ///
    /// Assertions:
    /// - Confirms `clock2.elapsed()` equals `Duration::from_secs(10)`.
    /// - Confirms `clock1.elapsed()` equals `Duration::from_secs(15)`.
    /// - Confirms `clock2.elapsed()` equals `Duration::from_secs(15)`.
    #[test]
    fn test_mock_clock_clone() {
        let clock1 = MockClock::new();
        clock1.advance(Duration::from_secs(10));

        let clock2 = clock1.clone();
        assert_eq!(clock2.elapsed(), Duration::from_secs(10));

        clock2.advance(Duration::from_secs(5));
        assert_eq!(clock1.elapsed(), Duration::from_secs(15));
        assert_eq!(clock2.elapsed(), Duration::from_secs(15));
    }

    // =========================================================================
    // Circuit Breaker Config Tests
    // =========================================================================

    /// Validates `CircuitState::Closed` behavior for the circuit state display
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `CircuitState::Closed.to_string()` equals `"CLOSED"`.
    /// - Confirms `CircuitState::Open.to_string()` equals `"OPEN"`.
    /// - Confirms `CircuitState::HalfOpen.to_string()` equals `"HALF_OPEN"`.
    #[test]
    fn test_circuit_state_display() {
        assert_eq!(CircuitState::Closed.to_string(), "CLOSED");
        assert_eq!(CircuitState::Open.to_string(), "OPEN");
        assert_eq!(CircuitState::HalfOpen.to_string(), "HALF_OPEN");
    }

    /// Validates `CircuitBreakerConfig::default` behavior for the circuit
    /// breaker config default scenario.
    ///
    /// Assertions:
    /// - Confirms `config.failure_threshold` equals `5`.
    /// - Confirms `config.success_threshold` equals `2`.
    /// - Confirms `config.timeout` equals `Duration::from_secs(60)`.
    /// - Confirms `config.half_open_max_calls` equals `3`.
    /// - Ensures `config.reset_on_success` evaluates to true.
    #[test]
    fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.half_open_max_calls, 3);
        assert!(config.reset_on_success);
    }

    /// Validates `CircuitBreakerConfig::default` behavior for the circuit
    /// breaker config validation scenario.
    ///
    /// Assertions:
    /// - Ensures `config.validate().is_ok()` evaluates to true.
    /// - Ensures `config.validate().is_err()` evaluates to true.
    /// - Ensures `config.validate().is_err()` evaluates to true.
    /// - Ensures `config.validate().is_err()` evaluates to true.
    #[test]
    fn test_circuit_breaker_config_validation() {
        let mut config = CircuitBreakerConfig::default();
        assert!(config.validate().is_ok());

        config.failure_threshold = 0;
        assert!(config.validate().is_err());

        config.failure_threshold = 5;
        config.success_threshold = 0;
        assert!(config.validate().is_err());

        config.success_threshold = 2;
        config.half_open_max_calls = 0;
        assert!(config.validate().is_err());
    }

    /// Tests builder pattern for circuit breaker configuration
    #[test]
    fn test_circuit_breaker_config_builder() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(10)
            .success_threshold(3)
            .timeout(Duration::from_secs(30))
            .half_open_max_calls(5)
            .reset_on_success(false)
            .build();

        assert!(config.is_ok(), "Valid config should build successfully");
        let config = config.expect("Builder should create valid config");
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.half_open_max_calls, 5);
        assert!(!config.reset_on_success);
    }

    /// Validates `CircuitBreakerConfig::new` behavior for the circuit breaker
    /// config builder validation fails scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_circuit_breaker_config_builder_validation_fails() {
        let result = CircuitBreakerConfig::new().failure_threshold(0).build();

        assert!(result.is_err());
    }

    /// Validates `CircuitBreakerConfig::default` behavior for the circuit
    /// breaker creation scenario.
    ///
    /// Assertions:
    /// - Ensures `cb.is_ok()` evaluates to true.
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    #[test]
    fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new(config);
        assert!(cb.is_ok());

        let cb = cb.unwrap();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// default scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    #[test]
    fn test_circuit_breaker_default() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    /// Validates `CircuitBreaker::builder` behavior for the circuit breaker
    /// builder scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    #[test]
    fn test_circuit_breaker_builder() {
        let cb = CircuitBreaker::builder().failure_threshold(3).build().unwrap();

        let cb = CircuitBreaker::new(cb).unwrap();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker can
    /// execute closed scenario.
    ///
    /// Assertions:
    /// - Ensures `cb.can_execute()` evaluates to true.
    #[test]
    fn test_circuit_breaker_can_execute_closed() {
        let cb = CircuitBreaker::default();
        assert!(cb.can_execute());
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// record success in closed scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    /// - Confirms `cb.success_count.load(AtomicOrdering::Acquire)` equals `1`.
    #[test]
    fn test_circuit_breaker_record_success_in_closed() {
        let cb = CircuitBreaker::default();
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert_eq!(cb.success_count.load(AtomicOrdering::Acquire), 1);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// record failure in closed scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    /// - Confirms `cb.failure_count.load(AtomicOrdering::Acquire)` equals `1`.
    #[test]
    fn test_circuit_breaker_record_failure_in_closed() {
        let cb = CircuitBreaker::default();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert_eq!(cb.failure_count.load(AtomicOrdering::Acquire), 1);
    }

    /// Tests that circuit opens when failure threshold is reached
    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(3)
            .build()
            .expect("Should build valid config");
        let cb =
            CircuitBreaker::new(config).expect("Should create circuit breaker with valid config");

        // Record failures below threshold
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Closed, "Should remain closed below threshold");

        // Hit threshold - should open
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open, "Should open at threshold");
        assert!(!cb.can_execute(), "Should reject requests when open");
    }

    /// Tests that open circuit prevents execution
    #[test]
    fn test_circuit_breaker_open_prevents_execution() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(1)
            .build()
            .expect("Should build valid config");
        let cb = CircuitBreaker::new(config).expect("Should create circuit breaker");

        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert!(!cb.can_execute(), "Open circuit should block execution");
    }

    /// Tests automatic transition from Open to HalfOpen after timeout
    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(1)
            .timeout(Duration::from_millis(10))
            .build()
            .expect("Should build valid config");
        let cb = CircuitBreaker::new(config).expect("Should create circuit breaker");

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open, "Should be open after failure");

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should transition to half-open on next can_execute
        assert!(cb.can_execute(), "Should allow execution after timeout");
        assert_eq!(cb.get_state(), CircuitState::HalfOpen, "Should transition to half-open");
    }

    /// Validates `CircuitBreakerConfig::new` behavior for the circuit breaker
    /// half open allows limited calls scenario.
    ///
    /// Assertions:
    /// - Ensures `cb.can_execute()` evaluates to true.
    /// - Ensures `cb.can_execute()` evaluates to true.
    /// - Ensures `!cb.can_execute()` evaluates to true.
    #[test]
    fn test_circuit_breaker_half_open_allows_limited_calls() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(1)
            .half_open_max_calls(2)
            .timeout(Duration::from_millis(1))
            .build()
            .unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        // Manually set to half-open
        if let Ok(mut state) = cb.state.write() {
            *state = CircuitState::HalfOpen;
        }

        // Should allow up to max_calls
        assert!(cb.can_execute());
        cb.half_open_calls.fetch_add(1, AtomicOrdering::Release);

        assert!(cb.can_execute());
        cb.half_open_calls.fetch_add(1, AtomicOrdering::Release);

        // Should block after max_calls
        assert!(!cb.can_execute());
    }

    /// Validates `CircuitBreakerConfig::new` behavior for the circuit breaker
    /// half open closes on success scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::HalfOpen`.
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    #[test]
    fn test_circuit_breaker_half_open_closes_on_success() {
        let config = CircuitBreakerConfig::new().success_threshold(2).build().unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        // Set to half-open
        if let Ok(mut state) = cb.state.write() {
            *state = CircuitState::HalfOpen;
        }

        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// half open opens on failure scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Open`.
    #[test]
    fn test_circuit_breaker_half_open_opens_on_failure() {
        let cb = CircuitBreaker::default();

        // Set to half-open
        if let Ok(mut state) = cb.state.write() {
            *state = CircuitState::HalfOpen;
        }

        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
    }

    // =========================================================================
    // MockClock-based Circuit Breaker Tests
    // =========================================================================

    /// Validates `MockClock::new` behavior for the circuit breaker with mock
    /// clock scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    /// - Confirms `cb.get_state()` equals `CircuitState::Open`.
    /// - Ensures `cb.can_execute()` evaluates to true.
    /// - Confirms `cb.get_state()` equals `CircuitState::HalfOpen`.
    #[test]
    fn test_circuit_breaker_with_mock_clock() {
        let clock = MockClock::new();
        let config = CircuitBreakerConfig::new()
            .failure_threshold(2)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        let cb = CircuitBreaker::with_clock(config, clock.clone()).unwrap();

        assert_eq!(cb.get_state(), CircuitState::Closed);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Advance time past timeout
        clock.advance(Duration::from_secs(70));

        // Should transition to half-open on next check
        assert!(cb.can_execute());
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
    }

    /// Validates `MockClock::new` behavior for the circuit breaker mock clock
    /// timeout not elapsed scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Open`.
    /// - Ensures `!cb.can_execute()` evaluates to true.
    /// - Confirms `cb.get_state()` equals `CircuitState::Open`.
    #[test]
    fn test_circuit_breaker_mock_clock_timeout_not_elapsed() {
        let clock = MockClock::new();
        let config = CircuitBreakerConfig::new()
            .failure_threshold(1)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        let cb = CircuitBreaker::with_clock(config, clock.clone()).unwrap();

        // Open circuit
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Advance time but not past timeout
        clock.advance(Duration::from_secs(30));

        // Should still be open
        assert!(!cb.can_execute());
        assert_eq!(cb.get_state(), CircuitState::Open);
    }

    /// Validates `MockClock::new` behavior for the circuit breaker mock clock
    /// recovery flow scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Open`.
    /// - Ensures `cb.can_execute()` evaluates to true.
    /// - Confirms `cb.get_state()` equals `CircuitState::HalfOpen`.
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    #[test]
    fn test_circuit_breaker_mock_clock_recovery_flow() {
        let clock = MockClock::new();
        let config = CircuitBreakerConfig::new()
            .failure_threshold(2)
            .success_threshold(2)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let cb = CircuitBreaker::with_clock(config, clock.clone()).unwrap();

        // Open circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Advance past timeout
        clock.advance(Duration::from_secs(35));
        assert!(cb.can_execute());
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);

        // Record successes to close circuit
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// call sync scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `42`.
    /// - Confirms `counter.load(AtomicOrdering::SeqCst)` equals `1`.
    #[test]
    fn test_circuit_breaker_call_sync() {
        let cb = CircuitBreaker::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = cb.call(|| {
            counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
            Ok::<_, std::io::Error>(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// call sync failure scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_circuit_breaker_call_sync_failure() {
        let cb = CircuitBreaker::default();

        let result = cb.call(|| Err::<(), _>(std::io::Error::other("test error")));

        assert!(result.is_err());
        match result {
            Err(ResilienceError::OperationFailed { .. }) => (),
            _ => panic!("Expected OperationFailed error"),
        }
    }

    /// Validates `CircuitBreakerConfig::new` behavior for the circuit breaker
    /// call rejects when open scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_circuit_breaker_call_rejects_when_open() {
        let config = CircuitBreakerConfig::new().failure_threshold(1).build().unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        // Open the circuit
        cb.record_failure();

        let result = cb.call(|| Ok::<_, std::io::Error>(42));

        assert!(result.is_err());
        match result {
            Err(ResilienceError::CircuitOpen) => (),
            _ => panic!("Expected CircuitOpen error"),
        }
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker is
    /// available scenario.
    ///
    /// Assertions:
    /// - Ensures `cb.is_available()` evaluates to true.
    /// - Ensures `!cb.is_available()` evaluates to true.
    /// - Ensures `cb.is_available()` evaluates to true.
    #[test]
    fn test_circuit_breaker_is_available() {
        let cb = CircuitBreaker::default();
        assert!(cb.is_available(), "Closed circuit should be available");

        // Open circuit
        let config = CircuitBreakerConfig::new().failure_threshold(1).build().unwrap();
        let cb = CircuitBreaker::new(config).unwrap();
        cb.record_failure();
        assert!(!cb.is_available(), "Open circuit should not be available");

        // Half-open circuit
        if let Ok(mut state) = cb.state.write() {
            *state = CircuitState::HalfOpen;
        }
        assert!(cb.is_available(), "Half-open circuit should be available");
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// execute success scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `42`.
    /// - Confirms `counter.load(AtomicOrdering::SeqCst)` equals `1`.
    #[tokio::test]
    async fn test_circuit_breaker_execute_success() {
        let cb = CircuitBreaker::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = cb
            .execute(|| async move {
                counter_clone.fetch_add(1, AtomicOrdering::SeqCst);
                Ok::<_, std::io::Error>(42)
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 1);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// execute failure scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[tokio::test]
    async fn test_circuit_breaker_execute_failure() {
        let cb = CircuitBreaker::default();

        let result =
            cb.execute(|| async { Err::<(), _>(std::io::Error::other("test error")) }).await;

        assert!(result.is_err());
        match result {
            Err(ResilienceError::OperationFailed { .. }) => (),
            _ => panic!("Expected OperationFailed error"),
        }
    }

    /// Validates `CircuitBreakerConfig::new` behavior for the circuit breaker
    /// execute rejects when open scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[tokio::test]
    async fn test_circuit_breaker_execute_rejects_when_open() {
        let config = CircuitBreakerConfig::new().failure_threshold(1).build().unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        // Open the circuit
        cb.record_failure();

        let result = cb.execute(|| async { Ok::<_, std::io::Error>(42) }).await;

        assert!(result.is_err());
        match result {
            Err(ResilienceError::CircuitOpen) => (),
            _ => panic!("Expected CircuitOpen error"),
        }
    }

    /// Validates `CircuitBreakerConfig::new` behavior for the circuit breaker
    /// reset scenario.
    ///
    /// Assertions:
    /// - Confirms `cb.get_state()` equals `CircuitState::Open`.
    /// - Confirms `cb.get_state()` equals `CircuitState::Closed`.
    /// - Confirms `cb.failure_count.load(AtomicOrdering::Acquire)` equals `0`.
    /// - Confirms `cb.success_count.load(AtomicOrdering::Acquire)` equals `0`.
    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig::new().failure_threshold(1).build().unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Reset
        cb.reset();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert_eq!(cb.failure_count.load(AtomicOrdering::Acquire), 0);
        assert_eq!(cb.success_count.load(AtomicOrdering::Acquire), 0);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker get
    /// metrics scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.state` equals `CircuitState::Closed`.
    /// - Confirms `metrics.success_count` equals `1`.
    /// - Confirms `metrics.failure_count` equals `1`.
    /// - Confirms `metrics.total_calls` equals `2`.
    #[test]
    fn test_circuit_breaker_get_metrics() {
        let cb = CircuitBreaker::default();

        cb.record_success();
        cb.record_failure();
        cb.total_calls.fetch_add(2, AtomicOrdering::Release);

        let metrics = cb.get_metrics();
        assert_eq!(metrics.state, CircuitState::Closed);
        assert_eq!(metrics.success_count, 1);
        assert_eq!(metrics.failure_count, 1);
        assert_eq!(metrics.total_calls, 2);
    }

    /// Tests success resets failure count when reset_on_success is enabled.
    ///
    /// Verifies:
    /// - reset_on_success configuration flag works correctly
    /// - Success clears accumulated failure count
    /// - Prevents circuit from opening after transient failures
    /// - Allows circuit to recover from partial failure sequences
    #[test]
    fn test_circuit_breaker_reset_on_success() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(5)
            .reset_on_success(true)
            .build()
            .unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count.load(AtomicOrdering::Acquire), 2);

        cb.record_success();
        assert_eq!(cb.failure_count.load(AtomicOrdering::Acquire), 0);
    }

    /// Tests failure count persists when reset_on_success is disabled.
    ///
    /// Verifies:
    /// - reset_on_success=false preserves failure count on success
    /// - Accumulated failures remain tracked
    /// - Provides stricter circuit breaker behavior
    /// - Useful for services requiring longer recovery periods
    #[test]
    fn test_circuit_breaker_no_reset_on_success() {
        let config = CircuitBreakerConfig::new()
            .failure_threshold(5)
            .reset_on_success(false)
            .build()
            .unwrap();
        let cb = CircuitBreaker::new(config).unwrap();

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count.load(AtomicOrdering::Acquire), 2);

        cb.record_success();
        // Failure count should not reset
        assert_eq!(cb.failure_count.load(AtomicOrdering::Acquire), 2);
    }

    /// Validates `CircuitBreaker::default` behavior for the circuit breaker
    /// clone scenario.
    ///
    /// Assertions:
    /// - Confirms `cb2.failure_count.load(AtomicOrdering::Acquire)` equals `1`.
    /// - Confirms `cb2.get_state()` equals `cb1.get_state()`.
    #[test]
    fn test_circuit_breaker_clone() {
        let cb1 = CircuitBreaker::default();
        cb1.record_failure();

        let cb2 = cb1.clone();
        assert_eq!(cb2.failure_count.load(AtomicOrdering::Acquire), 1);
        assert_eq!(cb2.get_state(), cb1.get_state());
    }

    /// Tests circuit breaker handles concurrent async access correctly.
    ///
    /// Verifies:
    /// - Circuit breaker is safe for concurrent async tasks
    /// - Success count is accurately tracked across tasks
    /// - No data races in async environment
    /// - Atomic operations work correctly in tokio runtime
    #[tokio::test]
    async fn test_circuit_breaker_concurrent_access() {
        let cb = Arc::new(CircuitBreaker::default());
        let mut handles = vec![];

        for _ in 0..10 {
            let cb_clone = Arc::clone(&cb);
            let handle = tokio::spawn(async move {
                cb_clone.record_success();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(cb.success_count.load(AtomicOrdering::Acquire), 10);
    }

    /// Validates `ConfigError::Invalid` behavior for the config error display
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("bad value")` evaluates to true.
    #[test]
    fn test_config_error_display() {
        let err = ConfigError::Invalid { message: "bad value".to_string() };
        assert!(err.to_string().contains("bad value"));
    }
}
