//! Adaptive circuit breaker with self-adjusting thresholds
//!
//! This module provides a circuit breaker that automatically adjusts its
//! failure threshold based on observed error rates and latency patterns.

use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tracing::{debug, info, instrument, warn};

use super::{Clock, Histogram, ResilienceError, SystemClock};

/// Configuration for adaptive circuit breaker
#[derive(Debug, Clone)]
pub struct AdaptiveCircuitBreakerConfig {
    /// Initial failure threshold
    pub initial_failure_threshold: u64,
    /// Minimum failure threshold (won't go below this)
    pub min_failure_threshold: u64,
    /// Maximum failure threshold (won't go above this)
    pub max_failure_threshold: u64,
    /// Target error rate (0.0 to 1.0) - breaker tries to maintain this
    pub target_error_rate: f64,
    /// Window size for calculating error rate (number of recent operations)
    pub window_size: usize,
    /// How often to adjust thresholds
    pub adjustment_interval: Duration,
    /// Success threshold for closing circuit
    pub success_threshold: u64,
    /// Timeout before transitioning from open to half-open
    pub timeout: Duration,
    /// Maximum number of calls allowed in half-open state
    pub half_open_max_calls: u64,
}

impl Default for AdaptiveCircuitBreakerConfig {
    fn default() -> Self {
        Self {
            initial_failure_threshold: 5,
            min_failure_threshold: 2,
            max_failure_threshold: 20,
            target_error_rate: 0.1, // 10% target error rate
            window_size: 100,
            adjustment_interval: Duration::from_secs(60),
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

impl AdaptiveCircuitBreakerConfig {
    /// Create a new configuration builder
    pub fn builder() -> AdaptiveCircuitBreakerConfigBuilder {
        AdaptiveCircuitBreakerConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.initial_failure_threshold == 0 {
            return Err("initial_failure_threshold must be greater than 0".to_string());
        }
        if self.min_failure_threshold == 0 {
            return Err("min_failure_threshold must be greater than 0".to_string());
        }
        if self.max_failure_threshold < self.min_failure_threshold {
            return Err("max_failure_threshold must be >= min_failure_threshold".to_string());
        }
        if self.target_error_rate < 0.0 || self.target_error_rate > 1.0 {
            return Err("target_error_rate must be between 0.0 and 1.0".to_string());
        }
        if self.window_size == 0 {
            return Err("window_size must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Builder for AdaptiveCircuitBreakerConfig
#[derive(Debug)]
pub struct AdaptiveCircuitBreakerConfigBuilder {
    config: AdaptiveCircuitBreakerConfig,
}

impl Default for AdaptiveCircuitBreakerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveCircuitBreakerConfigBuilder {
    pub fn new() -> Self {
        Self { config: AdaptiveCircuitBreakerConfig::default() }
    }

    pub fn initial_failure_threshold(mut self, threshold: u64) -> Self {
        self.config.initial_failure_threshold = threshold;
        self
    }

    pub fn min_failure_threshold(mut self, threshold: u64) -> Self {
        self.config.min_failure_threshold = threshold;
        self
    }

    pub fn max_failure_threshold(mut self, threshold: u64) -> Self {
        self.config.max_failure_threshold = threshold;
        self
    }

    pub fn target_error_rate(mut self, rate: f64) -> Self {
        self.config.target_error_rate = rate;
        self
    }

    pub fn window_size(mut self, size: usize) -> Self {
        self.config.window_size = size;
        self
    }

    pub fn adjustment_interval(mut self, interval: Duration) -> Self {
        self.config.adjustment_interval = interval;
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

    pub fn build(self) -> Result<AdaptiveCircuitBreakerConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveCircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl fmt::Display for AdaptiveCircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdaptiveCircuitState::Closed => write!(f, "CLOSED"),
            AdaptiveCircuitState::Open => write!(f, "OPEN"),
            AdaptiveCircuitState::HalfOpen => write!(f, "HALF_OPEN"),
        }
    }
}

/// Metrics for adaptive circuit breaker
#[derive(Debug, Clone)]
pub struct AdaptiveCircuitBreakerMetrics {
    pub state: AdaptiveCircuitState,
    pub failure_count: u64,
    pub success_count: u64,
    pub total_calls: u64,
    pub current_failure_threshold: u64,
    pub recent_error_rate: f64,
    pub threshold_adjustments: u64,
    pub latency_p50: Option<Duration>,
    pub latency_p99: Option<Duration>,
}

impl AdaptiveCircuitBreakerMetrics {
    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        format!(
            "Adaptive Circuit Breaker: {} - {}/{} calls ({:.1}% error rate), threshold={}, adjustments={}",
            self.state,
            self.success_count,
            self.total_calls,
            self.recent_error_rate * 100.0,
            self.current_failure_threshold,
            self.threshold_adjustments
        )
    }
}

/// Adaptive circuit breaker implementation
///
/// Automatically adjusts failure thresholds based on observed error rates
/// and latency patterns. Uses a sliding window to track recent operations.
///
/// # Examples
///
/// ```rust
/// use pulsearc_common::resilience::{AdaptiveCircuitBreaker, AdaptiveCircuitBreakerConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = AdaptiveCircuitBreakerConfig::builder()
///     .target_error_rate(0.05) // Target 5% error rate
///     .window_size(100)
///     .build()?;
///
/// let breaker = AdaptiveCircuitBreaker::new(config);
///
/// let result = breaker.execute(|| async { Ok::<_, std::io::Error>("Success") }).await?;
/// # Ok(())
/// # }
/// ```
pub struct AdaptiveCircuitBreaker<C: Clock = SystemClock> {
    config: AdaptiveCircuitBreakerConfig,
    state: Arc<RwLock<AdaptiveCircuitState>>,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    total_calls: Arc<AtomicU64>,
    half_open_calls: Arc<AtomicU64>,
    half_open_successes: Arc<AtomicU64>,
    current_failure_threshold: Arc<AtomicU64>,
    threshold_adjustments: Arc<AtomicU64>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    last_adjustment_time: Arc<RwLock<Instant>>,
    recent_results: Arc<RwLock<Vec<bool>>>, // true = success, false = failure
    latency_histogram: Histogram,
    clock: Arc<C>,
}

impl<C: Clock> AdaptiveCircuitBreaker<C> {
    /// Create a new adaptive circuit breaker with custom clock
    pub fn with_clock(config: AdaptiveCircuitBreakerConfig, clock: C) -> Result<Self, String> {
        config.validate()?;

        Ok(Self {
            current_failure_threshold: Arc::new(AtomicU64::new(config.initial_failure_threshold)),
            state: Arc::new(RwLock::new(AdaptiveCircuitState::Closed)),
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            total_calls: Arc::new(AtomicU64::new(0)),
            half_open_calls: Arc::new(AtomicU64::new(0)),
            half_open_successes: Arc::new(AtomicU64::new(0)),
            threshold_adjustments: Arc::new(AtomicU64::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            last_adjustment_time: Arc::new(RwLock::new(clock.now())),
            recent_results: Arc::new(RwLock::new(Vec::with_capacity(config.window_size))),
            latency_histogram: Histogram::new(),
            clock: Arc::new(clock),
            config,
        })
    }

    /// Execute an operation with adaptive circuit breaker protection
    #[instrument(skip(self, operation), fields(state = %self.get_state()))]
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, ResilienceError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.can_execute() {
            debug!("Adaptive circuit breaker rejecting call - state: {}", self.get_state());
            return Err(ResilienceError::CircuitOpen);
        }

        self.total_calls.fetch_add(1, Ordering::Relaxed);

        let current_state = self.get_state();
        if current_state == AdaptiveCircuitState::HalfOpen {
            self.half_open_calls.fetch_add(1, Ordering::Relaxed);
        }

        let start = self.clock.now();

        match operation().await {
            Ok(result) => {
                let latency = start.elapsed();
                self.latency_histogram.record(latency);
                self.record_success();
                debug!("Adaptive circuit breaker: operation succeeded");
                Ok(result)
            }
            Err(error) => {
                let latency = start.elapsed();
                self.latency_histogram.record(latency);
                self.record_failure();
                warn!("Adaptive circuit breaker: operation failed");
                Err(ResilienceError::OperationFailed { source: error })
            }
        }
    }

    /// Check if the circuit allows execution
    fn can_execute(&self) -> bool {
        let state = self.get_state();

        match state {
            AdaptiveCircuitState::Closed => true,
            AdaptiveCircuitState::Open => {
                // Check if timeout has elapsed
                if let Ok(last_failure_guard) = self.last_failure_time.read() {
                    if let Some(failure_time) = *last_failure_guard {
                        let now = self.clock.now();
                        if now.duration_since(failure_time) >= self.config.timeout {
                            drop(last_failure_guard);
                            // Transition to half-open
                            if let Ok(mut state) = self.state.write() {
                                *state = AdaptiveCircuitState::HalfOpen;
                                self.half_open_calls.store(0, Ordering::Release);
                                self.half_open_successes.store(0, Ordering::Release);
                            }
                            return true;
                        }
                    }
                }
                false
            }
            AdaptiveCircuitState::HalfOpen => {
                let current_calls = self.half_open_calls.load(Ordering::Acquire);
                current_calls < self.config.half_open_max_calls
            }
        }
    }

    /// Record a successful operation
    fn record_success(&self) {
        let current_state = self.get_state();
        self.success_count.fetch_add(1, Ordering::Relaxed);

        // Add to sliding window
        if let Ok(mut results) = self.recent_results.write() {
            results.push(true);
            if results.len() > self.config.window_size {
                results.remove(0);
            }
        }

        match current_state {
            AdaptiveCircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::Relaxed);
                self.maybe_adjust_threshold();
            }
            AdaptiveCircuitState::HalfOpen => {
                let consecutive_successes =
                    self.half_open_successes.fetch_add(1, Ordering::AcqRel) + 1;
                if consecutive_successes >= self.config.success_threshold {
                    // Transition to closed
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = AdaptiveCircuitState::Closed;
                        self.failure_count.store(0, Ordering::Release);
                        self.half_open_successes.store(0, Ordering::Release);
                    }
                    info!(
                        "Adaptive circuit breaker closed after {} half-open successes",
                        consecutive_successes
                    );
                }
            }
            AdaptiveCircuitState::Open => {
                warn!("Received success while circuit is open");
            }
        }
    }

    /// Record a failed operation
    fn record_failure(&self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let now = self.clock.now();

        // Update last failure time
        if let Ok(mut last_failure) = self.last_failure_time.write() {
            *last_failure = Some(now);
        }

        // Add to sliding window
        if let Ok(mut results) = self.recent_results.write() {
            results.push(false);
            if results.len() > self.config.window_size {
                results.remove(0);
            }
        }

        let current_state = self.get_state();
        let current_threshold = self.current_failure_threshold.load(Ordering::Acquire);

        match current_state {
            AdaptiveCircuitState::Closed => {
                if failure_count >= current_threshold {
                    // Transition to open
                    if let Ok(mut state_guard) = self.state.write() {
                        *state_guard = AdaptiveCircuitState::Open;
                    }
                    warn!(
                        "Adaptive circuit breaker opened after {} failures (threshold: {})",
                        failure_count, current_threshold
                    );
                }
                self.maybe_adjust_threshold();
            }
            AdaptiveCircuitState::HalfOpen => {
                // Any failure in half-open state immediately opens the circuit
                if let Ok(mut state_guard) = self.state.write() {
                    *state_guard = AdaptiveCircuitState::Open;
                }
                self.half_open_successes.store(0, Ordering::Release);
                warn!("Adaptive circuit breaker opened due to failure in half-open state");
            }
            AdaptiveCircuitState::Open => {
                // Already open
            }
        }
    }

    /// Adjust failure threshold based on observed error rate
    fn maybe_adjust_threshold(&self) {
        let now = self.clock.now();

        let should_adjust = if let Ok(last_adjustment) = self.last_adjustment_time.read() {
            now.duration_since(*last_adjustment) >= self.config.adjustment_interval
        } else {
            false
        };

        if !should_adjust {
            return;
        }

        // Calculate recent error rate
        let error_rate = if let Ok(results) = self.recent_results.read() {
            if results.is_empty() {
                return;
            }

            let failures = results.iter().filter(|&&success| !success).count();
            failures as f64 / results.len() as f64
        } else {
            return;
        };

        let current_threshold = self.current_failure_threshold.load(Ordering::Acquire);
        let new_threshold = if error_rate > self.config.target_error_rate {
            // Error rate too high - decrease threshold (more sensitive)
            (current_threshold.saturating_sub(1)).max(self.config.min_failure_threshold)
        } else if error_rate < self.config.target_error_rate * 0.5 {
            // Error rate very low - increase threshold (less sensitive)
            (current_threshold.saturating_add(1)).min(self.config.max_failure_threshold)
        } else {
            current_threshold
        };

        if new_threshold != current_threshold {
            self.current_failure_threshold.store(new_threshold, Ordering::Release);
            self.threshold_adjustments.fetch_add(1, Ordering::Relaxed);
            info!(
                "Adjusted failure threshold: {} -> {} (error rate: {:.1}%)",
                current_threshold,
                new_threshold,
                error_rate * 100.0
            );
        }

        // Update last adjustment time
        if let Ok(mut last_adjustment) = self.last_adjustment_time.write() {
            *last_adjustment = now;
        }
    }

    /// Get the current state
    fn get_state(&self) -> AdaptiveCircuitState {
        match self.state.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                warn!("Adaptive circuit breaker state lock poisoned");
                *poisoned.into_inner()
            }
        }
    }

    /// Get metrics
    pub fn metrics(&self) -> AdaptiveCircuitBreakerMetrics {
        let latency_snapshot = self.latency_histogram.snapshot();
        let error_rate = if let Ok(results) = self.recent_results.read() {
            if results.is_empty() {
                0.0
            } else {
                let failures = results.iter().filter(|&&success| !success).count();
                failures as f64 / results.len() as f64
            }
        } else {
            0.0
        };

        AdaptiveCircuitBreakerMetrics {
            state: self.get_state(),
            failure_count: self.failure_count.load(Ordering::Acquire),
            success_count: self.success_count.load(Ordering::Acquire),
            total_calls: self.total_calls.load(Ordering::Acquire),
            current_failure_threshold: self.current_failure_threshold.load(Ordering::Acquire),
            recent_error_rate: error_rate,
            threshold_adjustments: self.threshold_adjustments.load(Ordering::Acquire),
            latency_p50: latency_snapshot.percentile(0.5),
            latency_p99: latency_snapshot.percentile(0.99),
        }
    }

    /// Reset the circuit breaker
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Release);
        self.success_count.store(0, Ordering::Release);
        self.total_calls.store(0, Ordering::Release);
        self.half_open_calls.store(0, Ordering::Release);
        self.half_open_successes.store(0, Ordering::Release);
        self.current_failure_threshold
            .store(self.config.initial_failure_threshold, Ordering::Release);
        self.threshold_adjustments.store(0, Ordering::Release);

        if let Ok(mut results) = self.recent_results.write() {
            results.clear();
        }

        if let Ok(mut state) = self.state.write() {
            *state = AdaptiveCircuitState::Closed;
        }

        self.latency_histogram.reset();
    }
}

impl AdaptiveCircuitBreaker<SystemClock> {
    /// Create a new adaptive circuit breaker with system clock
    pub fn new(config: AdaptiveCircuitBreakerConfig) -> Result<Self, String> {
        Self::with_clock(config, SystemClock)
    }
}

impl<C: Clock> Clone for AdaptiveCircuitBreaker<C> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            failure_count: Arc::clone(&self.failure_count),
            success_count: Arc::clone(&self.success_count),
            total_calls: Arc::clone(&self.total_calls),
            half_open_calls: Arc::clone(&self.half_open_calls),
            half_open_successes: Arc::clone(&self.half_open_successes),
            current_failure_threshold: Arc::clone(&self.current_failure_threshold),
            threshold_adjustments: Arc::clone(&self.threshold_adjustments),
            last_failure_time: Arc::clone(&self.last_failure_time),
            last_adjustment_time: Arc::clone(&self.last_adjustment_time),
            recent_results: Arc::clone(&self.recent_results),
            latency_histogram: self.latency_histogram.clone(),
            clock: Arc::clone(&self.clock),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;
    use crate::resilience::MockClock;

    #[tokio::test]
    async fn test_adaptive_circuit_breaker_basic() {
        let config = AdaptiveCircuitBreakerConfig::default();
        let breaker = AdaptiveCircuitBreaker::new(config).unwrap();

        let result = breaker.execute(|| async { Ok::<_, std::io::Error>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_adaptive_threshold_adjustment() {
        use crate::resilience::MockClock;

        let clock = MockClock::new();
        let config = AdaptiveCircuitBreakerConfig::builder()
            .initial_failure_threshold(5)
            .min_failure_threshold(2)
            .max_failure_threshold(10)
            .target_error_rate(0.1)
            .window_size(20)
            .adjustment_interval(Duration::from_millis(100))
            .build()
            .unwrap();

        let breaker = AdaptiveCircuitBreaker::with_clock(config, clock.clone()).unwrap();

        // Generate high error rate (50%)
        for i in 0..20 {
            if i % 2 == 0 {
                let _ = breaker.execute(|| async { Ok::<_, std::io::Error>(()) }).await;
            } else {
                let _ = breaker
                    .execute(|| async { Err::<(), _>(std::io::Error::other("error")) })
                    .await;
            }
        }

        // Advance the mock clock past the adjustment interval
        clock.advance(Duration::from_millis(150));

        // Trigger one more operation to invoke maybe_adjust_threshold
        let _ = breaker.execute(|| async { Err::<(), _>(std::io::Error::other("error")) }).await;

        let metrics = breaker.metrics();
        // Should have adjusted threshold down due to high error rate
        assert!(
            metrics.threshold_adjustments > 0,
            "Expected threshold adjustments but got 0. Metrics: {:?}",
            metrics
        );
    }

    #[tokio::test]
    async fn test_half_open_requires_consecutive_successes() {
        #[derive(Debug)]
        struct TestError;

        impl fmt::Display for TestError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "test error")
            }
        }

        impl std::error::Error for TestError {}

        let timeout = Duration::from_secs(1);
        let config = AdaptiveCircuitBreakerConfig::builder()
            .initial_failure_threshold(1)
            .min_failure_threshold(1)
            .max_failure_threshold(5)
            .success_threshold(2)
            .half_open_max_calls(5)
            .window_size(10)
            .adjustment_interval(Duration::from_secs(3600))
            .timeout(timeout)
            .build()
            .unwrap();

        let clock = MockClock::new();
        let breaker = AdaptiveCircuitBreaker::with_clock(config, clock.clone()).unwrap();

        // First failure should open the circuit immediately (threshold = 1)
        let result = breaker
            .execute(|| async { Err::<(), TestError>(TestError) })
            .await
            .expect_err("expected operation failure");
        assert!(matches!(result, ResilienceError::OperationFailed { .. }));
        assert_eq!(breaker.metrics().state, AdaptiveCircuitState::Open);

        // Calls while open should be rejected.
        let rejected = breaker
            .execute(|| async { Ok::<_, TestError>(()) })
            .await
            .expect_err("expected circuit to reject call");
        assert!(matches!(rejected, ResilienceError::CircuitOpen));

        // Advance time to transition to half-open.
        clock.advance(timeout);

        // First success in half-open state should keep the circuit half-open.
        breaker
            .execute(|| async { Ok::<_, TestError>(()) })
            .await
            .expect("half-open attempt should succeed");
        assert_eq!(breaker.metrics().state, AdaptiveCircuitState::HalfOpen);

        // Second consecutive success should close the circuit.
        breaker
            .execute(|| async { Ok::<_, TestError>(()) })
            .await
            .expect("second half-open success should close circuit");
        assert_eq!(breaker.metrics().state, AdaptiveCircuitState::Closed);
    }
}
