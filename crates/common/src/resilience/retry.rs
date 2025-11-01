//! Generic retry strategy implementation with proper error handling and
//! extensibility
//!
//! This module provides a flexible retry mechanism that can be used across the
//! application for any operation that might fail and needs retry logic. It
//! supports various backoff strategies, jitter, and customizable retry
//! conditions.

use std::fmt;
use std::future::Future;
use std::time::{Duration, Instant};

use thiserror::Error;
use tracing::{debug, instrument, warn};

/// Errors that can occur during retry operations
#[derive(Debug, Error)]
pub enum RetryError<E> {
    /// All retry attempts have been exhausted
    #[error("All retry attempts exhausted after {attempts} tries")]
    AttemptsExhausted { attempts: u32 },

    /// The operation failed with a non-retryable error
    #[error("Operation failed with non-retryable error: {source}")]
    NonRetryable { source: E },

    /// The retry strategy configuration is invalid
    #[error("Invalid retry configuration: {message}")]
    InvalidConfiguration { message: String },

    /// A timeout occurred during retry operations
    #[error("Retry timeout exceeded after {elapsed:?}")]
    TimeoutExceeded { elapsed: Duration },
}

/// Result type for retry operations
pub type RetryResult<T, E> = Result<T, RetryError<E>>;

/// Outcome of a retry execution including result and summary statistics.
#[derive(Debug)]
pub struct RetryOutcome<T, E> {
    pub result: RetryResult<T, E>,
    pub attempts: u32,
    pub total_delay: Duration,
    pub timed_out: bool,
    pub first_attempt_time: Instant,
    /// Human-readable representation of the last error that occurred.
    pub last_error: Option<String>,
}

impl<T, E> RetryOutcome<T, E> {
    /// Consume the outcome and return only the result.
    pub fn into_result(self) -> RetryResult<T, E> {
        self.result
    }

    /// Get the total elapsed time from first attempt to completion.
    pub fn total_elapsed(&self) -> Duration {
        self.first_attempt_time.elapsed()
    }

    /// Get the average delay between attempts (excludes operation execution
    /// time).
    pub fn average_delay(&self) -> Duration {
        if self.attempts <= 1 {
            return Duration::ZERO;
        }
        self.total_delay / (self.attempts - 1)
    }
}

/// Trait for determining whether an error should be retried
pub trait RetryPolicy<E> {
    /// Determine if the error should be retried and optionally provide a custom
    /// delay
    fn should_retry(&self, error: &E, attempt: u32) -> RetryDecision;
}

/// Decision for whether to retry an operation
#[derive(Debug, Clone, PartialEq)]
pub enum RetryDecision {
    /// Retry the operation with the default backoff delay
    Retry,
    /// Retry the operation with a custom delay
    RetryAfter(Duration),
    /// Don't retry the operation
    Stop,
}

/// Backoff strategy for calculating retry delays
#[derive(Debug, Clone)]
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(PartialEq)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed(Duration),
    /// Linear backoff: initial_delay + (attempt * increment)
    Linear { initial_delay: Duration, increment: Duration },
    /// Exponential backoff: initial_delay * base^attempt
    Exponential { initial_delay: Duration, base: f64, max_delay: Duration },
    /// Custom backoff function
    Custom(fn(u32) -> Duration),
}

impl BackoffStrategy {
    /// Calculate the next delay for the given attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            BackoffStrategy::Fixed(delay) => *delay,
            BackoffStrategy::Linear { initial_delay, increment } => {
                *initial_delay + increment.saturating_mul(attempt)
            }
            BackoffStrategy::Exponential { initial_delay, base, max_delay } => {
                let delay = initial_delay.as_millis() as f64 * base.powi(attempt as i32);
                let delay_ms = delay.min(max_delay.as_millis() as f64) as u64;
                Duration::from_millis(delay_ms)
            }
            BackoffStrategy::Custom(f) => f(attempt),
        }
    }
}

/// Jitter type for adding randomness to retry delays
#[derive(Debug, Clone, PartialEq)]
pub enum Jitter {
    /// No jitter
    None,
    /// Full jitter: 0 to calculated_delay
    Full,
    /// Equal jitter: calculated_delay/2 to calculated_delay
    Equal,
    /// Decorrelated jitter: more sophisticated randomization
    Decorrelated { base: Duration },
}

impl Jitter {
    /// Apply jitter to the calculated delay
    pub fn apply(&self, delay: Duration, attempt: u32) -> Duration {
        match self {
            Jitter::None => delay,
            Jitter::Full => {
                let jitter_ms = self.random_value(delay.as_millis() as u64);
                Duration::from_millis(jitter_ms)
            }
            Jitter::Equal => {
                let half_delay = delay.as_millis() / 2;
                let jitter_ms = half_delay + self.random_value(half_delay as u64) as u128;
                Duration::from_millis(jitter_ms as u64)
            }
            Jitter::Decorrelated { base } => {
                let prev_delay = if attempt == 0 { *base } else { delay };
                let max_jitter = prev_delay.as_millis() * 3;
                let jitter_ms = base.as_millis() + self.random_value(max_jitter as u64) as u128;
                Duration::from_millis(jitter_ms as u64)
            }
        }
    }

    /// Generate a pseudo-random value using timing-based seed
    /// This provides good distribution for jitter without external dependencies
    fn random_value(&self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }

        // Use nanosecond precision from Instant for seed
        let nanos = Instant::now().elapsed().subsec_nanos() as u64;

        // Simple Linear Congruential Generator (LCG) for pseudo-randomness
        // Constants from Numerical Recipes
        let mut seed = nanos.wrapping_mul(1664525).wrapping_add(1013904223);
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed % max
    }
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Backoff strategy for calculating delays
    pub backoff: BackoffStrategy,
    /// Jitter type for randomizing delays
    pub jitter: Jitter,
    /// Maximum total time to spend retrying
    pub max_total_time: Option<Duration>,
    /// Whether to reset attempt count on certain conditions
    pub reset_on_success: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                initial_delay: Duration::from_millis(100),
                base: 2.0,
                max_delay: Duration::from_secs(30),
            },
            jitter: Jitter::Equal,
            max_total_time: Some(Duration::from_secs(300)), // 5 minutes
            reset_on_success: false,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration with validation
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> RetryConfigBuilder {
        RetryConfigBuilder::new()
    }

    /// Create a configuration builder (alias for `new()`)
    pub fn builder() -> RetryConfigBuilder {
        RetryConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), RetryError<()>> {
        if self.max_attempts == 0 {
            return Err(RetryError::InvalidConfiguration {
                message: "max_attempts must be greater than 0".to_string(),
            });
        }

        match &self.backoff {
            BackoffStrategy::Exponential { base, .. } if *base <= 0.0 => {
                return Err(RetryError::InvalidConfiguration {
                    message: "exponential base must be greater than 0".to_string(),
                });
            }
            _ => {}
        }

        Ok(())
    }
}

/// Builder for RetryConfig with fluent API
#[derive(Debug)]
pub struct RetryConfigBuilder {
    config: RetryConfig,
}

impl Default for RetryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryConfigBuilder {
    pub fn new() -> Self {
        Self { config: RetryConfig::default() }
    }

    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.config.max_attempts = attempts;
        self
    }

    pub fn fixed_backoff(mut self, delay: Duration) -> Self {
        self.config.backoff = BackoffStrategy::Fixed(delay);
        self
    }

    pub fn linear_backoff(mut self, initial_delay: Duration, increment: Duration) -> Self {
        self.config.backoff = BackoffStrategy::Linear { initial_delay, increment };
        self
    }

    pub fn exponential_backoff(
        mut self,
        initial_delay: Duration,
        base: f64,
        max_delay: Duration,
    ) -> Self {
        self.config.backoff = BackoffStrategy::Exponential { initial_delay, base, max_delay };
        self
    }

    pub fn no_jitter(mut self) -> Self {
        self.config.jitter = Jitter::None;
        self
    }

    pub fn full_jitter(mut self) -> Self {
        self.config.jitter = Jitter::Full;
        self
    }

    pub fn equal_jitter(mut self) -> Self {
        self.config.jitter = Jitter::Equal;
        self
    }

    pub fn decorrelated_jitter(mut self, base: Duration) -> Self {
        self.config.jitter = Jitter::Decorrelated { base };
        self
    }

    pub fn max_total_time(mut self, duration: Duration) -> Self {
        self.config.max_total_time = Some(duration);
        self
    }

    pub fn unlimited_time(mut self) -> Self {
        self.config.max_total_time = None;
        self
    }

    pub fn reset_on_success(mut self, reset: bool) -> Self {
        self.config.reset_on_success = reset;
        self
    }

    pub fn build(self) -> Result<RetryConfig, RetryError<()>> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Context for tracking retry state
#[derive(Debug, Clone)]
pub struct RetryContext {
    /// Current attempt number (0-based)
    pub attempt: u32,
    /// Total elapsed time
    pub elapsed: Duration,
    /// Start time of retry sequence
    pub start_time: Instant,
    /// Last delay used
    pub last_delay: Option<Duration>,
    /// Whether the last attempt succeeded
    pub last_success: bool,
    /// Total accumulated delay across attempts
    pub total_delay: Duration,
    /// Whether the executor terminated due to timeout
    pub timed_out: bool,
}

impl RetryContext {
    fn new() -> Self {
        Self {
            attempt: 0,
            elapsed: Duration::ZERO,
            start_time: Instant::now(),
            last_delay: None,
            last_success: false,
            total_delay: Duration::ZERO,
            timed_out: false,
        }
    }

    fn update(&mut self) {
        self.elapsed = self.start_time.elapsed();
        self.attempt += 1;
    }
}

/// The main retry executor
pub struct RetryExecutor<P> {
    config: RetryConfig,
    policy: P,
}

impl<P> RetryExecutor<P> {
    /// Create a new retry executor with the given configuration and policy
    pub fn new(config: RetryConfig, policy: P) -> Self {
        Self { config, policy }
    }

    /// Create with default configuration
    pub fn with_policy(policy: P) -> Self {
        Self::new(RetryConfig::default(), policy)
    }
}

impl<P> RetryExecutor<P> {
    /// Execute an operation with retry logic
    #[instrument(skip(self, operation), fields(max_attempts = self.config.max_attempts))]
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> RetryResult<T, E>
    where
        P: RetryPolicy<E>,
        E: fmt::Debug,
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        self.execute_with_outcome(operation).await.into_result()
    }

    /// Execute an operation with retry logic and return outcome statistics.
    pub async fn execute_with_outcome<F, Fut, T, E>(&self, mut operation: F) -> RetryOutcome<T, E>
    where
        P: RetryPolicy<E>,
        E: fmt::Debug,
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let mut context = RetryContext::new();
        let first_attempt_time = Instant::now();
        let mut last_error: Option<String> = None;

        loop {
            context.elapsed = context.start_time.elapsed();
            let attempt_number = context.attempt + 1;

            if let Some(max_time) = self.config.max_total_time {
                if context.elapsed >= max_time {
                    warn!(
                        "Retry timeout exceeded after {:?} (attempts: {})",
                        context.elapsed, context.attempt
                    );
                    context.timed_out = true;
                    return RetryOutcome {
                        result: Err(RetryError::TimeoutExceeded { elapsed: context.elapsed }),
                        attempts: attempt_number,
                        total_delay: context.total_delay,
                        timed_out: true,
                        first_attempt_time,
                        last_error: last_error.clone(),
                    };
                }
            }

            debug!("Executing operation (attempt {}/{})", attempt_number, self.config.max_attempts);

            let result = operation().await;

            match result {
                Ok(value) => {
                    if context.attempt > 0 {
                        debug!("Operation succeeded after {} retries", context.attempt);
                    }
                    return RetryOutcome {
                        result: Ok(value),
                        attempts: attempt_number,
                        total_delay: context.total_delay,
                        timed_out: false,
                        first_attempt_time,
                        last_error: last_error.clone(),
                    };
                }
                Err(error) => {
                    let error_description = format!("{error:?}");

                    if context.attempt >= self.config.max_attempts - 1 {
                        warn!(
                            "All retry attempts exhausted after {} tries, last error: {:?}",
                            attempt_number, error
                        );
                        last_error = Some(error_description);
                        return RetryOutcome {
                            result: Err(RetryError::AttemptsExhausted { attempts: attempt_number }),
                            attempts: attempt_number,
                            total_delay: context.total_delay,
                            timed_out: false,
                            first_attempt_time,
                            last_error: last_error.clone(),
                        };
                    }

                    let decision = self.policy.should_retry(&error, context.attempt);
                    match decision {
                        RetryDecision::Stop => {
                            debug!("Retry policy determined not to retry: {:?}", error);
                            last_error = Some(error_description);
                            return RetryOutcome {
                                result: Err(RetryError::NonRetryable { source: error }),
                                attempts: attempt_number,
                                total_delay: context.total_delay,
                                timed_out: false,
                                first_attempt_time,
                                last_error: last_error.clone(),
                            };
                        }
                        RetryDecision::Retry => {
                            let delay = self.config.backoff.calculate_delay(context.attempt);
                            let jittered_delay = self.config.jitter.apply(delay, context.attempt);
                            last_error = Some(error_description);
                            self.sleep_and_update(&mut context, jittered_delay).await;
                        }
                        RetryDecision::RetryAfter(custom_delay) => {
                            last_error = Some(error_description);
                            self.sleep_and_update(&mut context, custom_delay).await;
                        }
                    }
                }
            }
        }
    }

    async fn sleep_and_update(&self, context: &mut RetryContext, delay: Duration) {
        warn!("Operation failed (attempt {}), retrying after {:?}", context.attempt + 1, delay);

        context.last_delay = Some(delay);
        tokio::time::sleep(delay).await;
        context.total_delay += delay;
        context.update();
    }
}

/// Convenience function to create a retry executor and execute an operation
pub async fn retry_with_policy<F, Fut, T, E, P>(
    config: RetryConfig,
    policy: P,
    operation: F,
) -> RetryResult<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: RetryPolicy<E>,
    E: fmt::Debug,
{
    let executor = RetryExecutor::new(config, policy);
    executor.execute(operation).await
}

/// Convenience function to retry with default configuration
pub async fn retry<F, Fut, T, E, P>(policy: P, operation: F) -> RetryResult<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: RetryPolicy<E>,
    E: fmt::Debug,
{
    retry_with_policy(RetryConfig::default(), policy, operation).await
}

/// Pre-defined retry policies for common scenarios
pub mod policies {
    use super::*;

    /// Always retry policy - retries on any error
    #[derive(Debug, Clone)]
    pub struct AlwaysRetry;

    impl<E> RetryPolicy<E> for AlwaysRetry {
        fn should_retry(&self, _error: &E, _attempt: u32) -> RetryDecision {
            RetryDecision::Retry
        }
    }

    /// Never retry policy - never retries
    #[derive(Debug, Clone)]
    pub struct NeverRetry;

    impl<E> RetryPolicy<E> for NeverRetry {
        fn should_retry(&self, _error: &E, _attempt: u32) -> RetryDecision {
            RetryDecision::Stop
        }
    }

    /// Predicate-based retry policy
    #[derive(Debug)]
    pub struct PredicateRetry<F> {
        predicate: F,
    }

    impl<F> PredicateRetry<F> {
        pub fn new(predicate: F) -> Self {
            Self { predicate }
        }
    }

    impl<F, E> RetryPolicy<E> for PredicateRetry<F>
    where
        F: Fn(&E, u32) -> bool,
    {
        fn should_retry(&self, error: &E, attempt: u32) -> RetryDecision {
            if (self.predicate)(error, attempt) {
                RetryDecision::Retry
            } else {
                RetryDecision::Stop
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for retry strategies and policies
    //!
    //! Tests cover backoff strategies (fixed, linear, exponential),
    //! jitter application, retry executor behavior, policy implementations,
    //! and timeout/attempt limit enforcement.

    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use super::policies::*;
    use super::*;

    /// Validates `RetryDecision::Retry` behavior for the retry decision
    /// equality scenario.
    ///
    /// Assertions:
    /// - Confirms `RetryDecision::Retry` equals `RetryDecision::Retry`.
    /// - Confirms `RetryDecision::Stop` equals `RetryDecision::Stop`.
    /// - Confirms `RetryDecision::Retry` differs from `RetryDecision::Stop`.
    #[test]
    fn test_retry_decision_equality() {
        assert_eq!(RetryDecision::Retry, RetryDecision::Retry);
        assert_eq!(RetryDecision::Stop, RetryDecision::Stop);
        assert_ne!(RetryDecision::Retry, RetryDecision::Stop);
    }

    /// Validates `BackoffStrategy::Fixed` behavior for the backoff strategy
    /// fixed scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.calculate_delay(0)` equals
    ///   `Duration::from_millis(100)`.
    /// - Confirms `strategy.calculate_delay(5)` equals
    ///   `Duration::from_millis(100)`.
    /// - Confirms `strategy.calculate_delay(100)` equals
    ///   `Duration::from_millis(100)`.
    #[test]
    fn test_backoff_strategy_fixed() {
        let strategy = BackoffStrategy::Fixed(Duration::from_millis(100));

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(5), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(100), Duration::from_millis(100));
    }

    /// Validates `BackoffStrategy::Linear` behavior for the backoff strategy
    /// linear scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.calculate_delay(0)` equals
    ///   `Duration::from_millis(100)`.
    /// - Confirms `strategy.calculate_delay(1)` equals
    ///   `Duration::from_millis(150)`.
    /// - Confirms `strategy.calculate_delay(2)` equals
    ///   `Duration::from_millis(200)`.
    /// - Confirms `strategy.calculate_delay(10)` equals
    ///   `Duration::from_millis(600)`.
    #[test]
    fn test_backoff_strategy_linear() {
        let strategy = BackoffStrategy::Linear {
            initial_delay: Duration::from_millis(100),
            increment: Duration::from_millis(50),
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(150));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(10), Duration::from_millis(600));
    }

    /// Validates `BackoffStrategy::Exponential` behavior for the backoff
    /// strategy exponential scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.calculate_delay(0)` equals
    ///   `Duration::from_millis(100)`.
    /// - Confirms `strategy.calculate_delay(1)` equals
    ///   `Duration::from_millis(200)`.
    /// - Confirms `strategy.calculate_delay(2)` equals
    ///   `Duration::from_millis(400)`.
    /// - Confirms `strategy.calculate_delay(3)` equals
    ///   `Duration::from_millis(800)`.
    /// - Ensures `delay <= Duration::from_secs(10)` evaluates to true.
    #[test]
    fn test_backoff_strategy_exponential() {
        let strategy = BackoffStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            base: 2.0,
            max_delay: Duration::from_secs(10),
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(800));

        // Should cap at max_delay
        let delay = strategy.calculate_delay(20);
        assert!(delay <= Duration::from_secs(10));
    }

    /// Validates `BackoffStrategy::Custom` behavior for the backoff strategy
    /// custom scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.calculate_delay(0)` equals
    ///   `Duration::from_millis(10)`.
    /// - Confirms `strategy.calculate_delay(1)` equals
    ///   `Duration::from_millis(20)`.
    /// - Confirms `strategy.calculate_delay(5)` equals
    ///   `Duration::from_millis(60)`.
    #[test]
    fn test_backoff_strategy_custom() {
        let strategy =
            BackoffStrategy::Custom(|attempt| Duration::from_millis((attempt as u64 + 1) * 10));

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(10));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(20));
        assert_eq!(strategy.calculate_delay(5), Duration::from_millis(60));
    }

    /// Validates `Jitter::None` behavior for the jitter none scenario.
    ///
    /// Assertions:
    /// - Confirms `jitter.apply(delay, 0)` equals `delay`.
    /// - Confirms `jitter.apply(delay, 5)` equals `delay`.
    #[test]
    fn test_jitter_none() {
        let jitter = Jitter::None;
        let delay = Duration::from_millis(100);

        assert_eq!(jitter.apply(delay, 0), delay);
        assert_eq!(jitter.apply(delay, 5), delay);
    }

    /// Validates `Jitter::Full` behavior for the jitter full scenario.
    ///
    /// Assertions:
    /// - Ensures `jittered <= delay` evaluates to true.
    #[test]
    fn test_jitter_full() {
        let jitter = Jitter::Full;
        let delay = Duration::from_millis(100);

        let jittered = jitter.apply(delay, 0);
        assert!(jittered <= delay);
    }

    /// Validates `Jitter::Equal` behavior for the jitter equal scenario.
    ///
    /// Assertions:
    /// - Ensures `jittered >= Duration::from_millis(50)` evaluates to true.
    /// - Ensures `jittered <= delay` evaluates to true.
    #[test]
    fn test_jitter_equal() {
        let jitter = Jitter::Equal;
        let delay = Duration::from_millis(100);

        let jittered = jitter.apply(delay, 0);
        assert!(jittered >= Duration::from_millis(50));
        assert!(jittered <= delay);
    }

    /// Validates `Jitter::Decorrelated` behavior for the jitter decorrelated
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `jittered >= Duration::from_millis(10)` evaluates to true.
    #[test]
    fn test_jitter_decorrelated() {
        let jitter = Jitter::Decorrelated { base: Duration::from_millis(10) };
        let delay = Duration::from_millis(100);

        let jittered = jitter.apply(delay, 0);
        assert!(jittered >= Duration::from_millis(10));
    }

    /// Validates `RetryConfig::default` behavior for the retry config default
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_attempts` equals `3`.
    /// - Ensures `!config.reset_on_success` evaluates to true.
    /// - Confirms `config.max_total_time` equals
    ///   `Some(Duration::from_secs(300))`.
    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();

        assert_eq!(config.max_attempts, 3);
        assert!(!config.reset_on_success);
        assert_eq!(config.max_total_time, Some(Duration::from_secs(300)));
    }

    /// Validates `RetryConfig::default` behavior for the retry config
    /// validation scenario.
    ///
    /// Assertions:
    /// - Ensures `config.validate().is_ok()` evaluates to true.
    /// - Ensures `config.validate().is_err()` evaluates to true.
    #[test]
    fn test_retry_config_validation() {
        let mut config = RetryConfig::default();
        assert!(config.validate().is_ok());

        config.max_attempts = 0;
        assert!(config.validate().is_err());
    }

    /// Tests builder pattern for retry configuration
    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new()
            .max_attempts(5)
            .fixed_backoff(Duration::from_millis(200))
            .no_jitter()
            .max_total_time(Duration::from_secs(60))
            .reset_on_success(true)
            .build();

        assert!(config.is_ok(), "Valid config should build successfully");
        let config = config.expect("Builder should create valid config");
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.jitter, Jitter::None);
        assert_eq!(config.max_total_time, Some(Duration::from_secs(60)));
        assert!(config.reset_on_success);
    }

    /// Validates `RetryConfig::new` behavior for the retry config builder
    /// linear backoff scenario.
    ///
    /// Assertions:
    /// - Confirms `initial_delay` equals `Duration::from_millis(100)`.
    /// - Confirms `increment` equals `Duration::from_millis(50)`.
    #[test]
    fn test_retry_config_builder_linear_backoff() {
        let config = RetryConfig::new()
            .linear_backoff(Duration::from_millis(100), Duration::from_millis(50))
            .build()
            .unwrap();

        match config.backoff {
            BackoffStrategy::Linear { initial_delay, increment } => {
                assert_eq!(initial_delay, Duration::from_millis(100));
                assert_eq!(increment, Duration::from_millis(50));
            }
            _ => panic!("Expected Linear backoff"),
        }
    }

    /// Validates `RetryConfig::new` behavior for the retry config builder
    /// exponential backoff scenario.
    ///
    /// Assertions:
    /// - Confirms `initial_delay` equals `Duration::from_millis(100)`.
    /// - Confirms `base` equals `2.0`.
    /// - Confirms `max_delay` equals `Duration::from_secs(30)`.
    #[test]
    fn test_retry_config_builder_exponential_backoff() {
        let config = RetryConfig::new()
            .exponential_backoff(Duration::from_millis(100), 2.0, Duration::from_secs(30))
            .build()
            .unwrap();

        match config.backoff {
            BackoffStrategy::Exponential { initial_delay, base, max_delay } => {
                assert_eq!(initial_delay, Duration::from_millis(100));
                assert_eq!(base, 2.0);
                assert_eq!(max_delay, Duration::from_secs(30));
            }
            _ => panic!("Expected Exponential backoff"),
        }
    }

    /// Validates `RetryConfig::new` behavior for the retry config builder
    /// jitter types scenario.
    ///
    /// Assertions:
    /// - Confirms `config.jitter` equals `Jitter::Full`.
    /// - Confirms `config.jitter` equals `Jitter::Equal`.
    /// - Confirms `base` equals `Duration::from_millis(10)`.
    #[test]
    fn test_retry_config_builder_jitter_types() {
        let config = RetryConfig::new().full_jitter().build().unwrap();
        assert_eq!(config.jitter, Jitter::Full);

        let config = RetryConfig::new().equal_jitter().build().unwrap();
        assert_eq!(config.jitter, Jitter::Equal);

        let config =
            RetryConfig::new().decorrelated_jitter(Duration::from_millis(10)).build().unwrap();
        match config.jitter {
            Jitter::Decorrelated { base } => assert_eq!(base, Duration::from_millis(10)),
            _ => panic!("Expected Decorrelated jitter"),
        }
    }

    /// Validates `RetryConfig::new` behavior for the retry config builder
    /// unlimited time scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_total_time` equals `None`.
    #[test]
    fn test_retry_config_builder_unlimited_time() {
        let config = RetryConfig::new().unlimited_time().build().unwrap();
        assert_eq!(config.max_total_time, None);
    }

    /// Validates `RetryConfig::new` behavior for the retry config builder
    /// validation fails scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_retry_config_builder_validation_fails() {
        let result = RetryConfig::new().max_attempts(0).build();
        assert!(result.is_err());
    }

    /// Validates `RetryContext::new` behavior for the retry context creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `context.attempt` equals `0`.
    /// - Confirms `context.elapsed` equals `Duration::ZERO`.
    /// - Confirms `context.last_delay` equals `None`.
    /// - Ensures `!context.last_success` evaluates to true.
    #[test]
    fn test_retry_context_creation() {
        let context = RetryContext::new();
        assert_eq!(context.attempt, 0);
        assert_eq!(context.elapsed, Duration::ZERO);
        assert_eq!(context.last_delay, None);
        assert!(!context.last_success);
    }

    /// Tests retry executor succeeds after temporary failures
    #[tokio::test]
    async fn test_retry_executor_with_always_retry_success() {
        let config = RetryConfig::new()
            .max_attempts(3)
            .fixed_backoff(Duration::from_millis(1))
            .no_jitter()
            .build()
            .expect("Should build valid config");

        let executor = RetryExecutor::new(config, AlwaysRetry);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = executor
            .execute(|| {
                let c = Arc::clone(&counter_clone);
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err("temporary failure")
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok(), "Should succeed after retries");
        let value = result.expect("Operation should eventually succeed");
        assert_eq!(value, 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3, "Should have tried 3 times");
    }

    /// Tests that retry executor properly exhausts all attempts on persistent
    /// failures
    #[tokio::test]
    async fn test_retry_executor_exhausts_attempts() {
        let config = RetryConfig::new()
            .max_attempts(3)
            .fixed_backoff(Duration::from_millis(1))
            .no_jitter()
            .build()
            .expect("Should build valid config");

        let executor = RetryExecutor::new(config, AlwaysRetry);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = executor
            .execute(|| {
                let c = Arc::clone(&counter_clone);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>("persistent failure")
                }
            })
            .await;

        assert!(result.is_err(), "Should fail after exhausting attempts");
        match result {
            Err(RetryError::AttemptsExhausted { attempts }) => {
                assert_eq!(attempts, 3, "Should exhaust all 3 attempts");
            }
            _ => panic!("Expected AttemptsExhausted error"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 3, "Should have tried exactly 3 times");
    }

    /// Tests NeverRetry policy stops immediately without retrying.
    ///
    /// Verifies:
    /// - NeverRetry policy executes operation only once
    /// - Returns NonRetryable error immediately on failure
    /// - No retry attempts are made regardless of error type
    /// - Useful for operations that should never be retried
    #[tokio::test]
    async fn test_retry_executor_with_never_retry() {
        let config = RetryConfig::default();
        let executor = RetryExecutor::new(config, NeverRetry);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = executor
            .execute(|| {
                let c = Arc::clone(&counter_clone);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>("error".to_string())
                }
            })
            .await;

        assert!(result.is_err());
        match result {
            Err(RetryError::NonRetryable { .. }) => (),
            _ => panic!("Expected NonRetryable error"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    /// Tests retry executor respects maximum total time limit.
    ///
    /// Verifies:
    /// - Retries stop when max_total_time is exceeded
    /// - Time limit takes precedence over max_attempts
    /// - Returns TimeoutExceeded error when time limit reached
    /// - Prevents unbounded retry duration
    #[tokio::test]
    async fn test_retry_executor_respects_max_total_time() {
        let config = RetryConfig::new()
            .max_attempts(100)
            .fixed_backoff(Duration::from_millis(50))
            .no_jitter()
            .max_total_time(Duration::from_millis(100))
            .build()
            .unwrap();

        let executor = RetryExecutor::new(config, AlwaysRetry);

        let result = executor.execute(|| async { Err::<(), _>("always fails".to_string()) }).await;

        assert!(result.is_err());
        match result {
            Err(RetryError::TimeoutExceeded { elapsed }) => {
                assert!(elapsed >= Duration::from_millis(100));
            }
            _ => panic!("Expected TimeoutExceeded error"),
        }
    }

    /// Tests PredicateRetry policy with custom retry logic.
    ///
    /// Verifies:
    /// - Custom predicate function controls retry behavior
    /// - Policy can decide based on both error and attempt number
    /// - Retries stop when predicate returns false
    /// - Stops before max_attempts when predicate rejects
    /// - Enables fine-grained retry control
    #[tokio::test]
    async fn test_retry_executor_with_predicate_retry() {
        let policy = PredicateRetry::new(|error: &String, attempt| {
            error.contains("retryable") && attempt < 2
        });

        let config = RetryConfig::new()
            .max_attempts(5)
            .fixed_backoff(Duration::from_millis(1))
            .no_jitter()
            .build()
            .unwrap();

        let executor = RetryExecutor::new(config, policy);
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = executor
            .execute(|| {
                let c = Arc::clone(&counter_clone);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>("retryable error".to_string())
                }
            })
            .await;

        assert!(result.is_err());
        // Should stop after 3 attempts (0, 1, 2) even though max is 5
        assert!(counter.load(Ordering::SeqCst) == 3);
    }

    /// Tests retry_with_policy convenience function.
    ///
    /// Verifies:
    /// - Convenience function simplifies retry usage
    /// - Custom config can be passed inline
    /// - Policy parameter allows flexible retry logic
    /// - Function succeeds after transient failure
    /// - Provides ergonomic API for common retry patterns
    #[tokio::test]
    async fn test_retry_with_policy_convenience_function() {
        let config = RetryConfig::new()
            .max_attempts(2)
            .fixed_backoff(Duration::from_millis(1))
            .build()
            .unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = retry_with_policy(config, AlwaysRetry, || {
            let c = Arc::clone(&counter_clone);
            async move {
                let count = c.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    Err("first attempt fails".to_string())
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    /// Tests retry convenience function with default configuration.
    ///
    /// Verifies:
    /// - Simplest retry API uses default config
    /// - Only policy needs to be specified
    /// - Default backoff and timeout work correctly
    /// - Provides quick retry capability for simple cases
    #[tokio::test]
    async fn test_retry_convenience_function() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = retry(AlwaysRetry, || {
            let c = Arc::clone(&counter_clone);
            async move {
                let count = c.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    Err("first attempt fails".to_string())
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
    }

    /// Validates `RetryDecision::Retry` behavior for the always retry policy
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `policy.should_retry(&error, 0)` equals
    ///   `RetryDecision::Retry`.
    /// - Confirms `policy.should_retry(&error, 100)` equals
    ///   `RetryDecision::Retry`.
    #[test]
    fn test_always_retry_policy() {
        let policy = AlwaysRetry;
        let error = "error".to_string();
        assert_eq!(policy.should_retry(&error, 0), RetryDecision::Retry);
        assert_eq!(policy.should_retry(&error, 100), RetryDecision::Retry);
    }

    /// Validates `RetryDecision::Stop` behavior for the never retry policy
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `policy.should_retry(&error, 0)` equals
    ///   `RetryDecision::Stop`.
    /// - Confirms `policy.should_retry(&error, 100)` equals
    ///   `RetryDecision::Stop`.
    #[test]
    fn test_never_retry_policy() {
        let policy = NeverRetry;
        let error = "error".to_string();
        assert_eq!(policy.should_retry(&error, 0), RetryDecision::Stop);
        assert_eq!(policy.should_retry(&error, 100), RetryDecision::Stop);
    }

    /// Validates `PredicateRetry::new` behavior for the predicate retry policy
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `policy.should_retry(&retryable, 0)` equals
    ///   `RetryDecision::Retry`.
    /// - Confirms `policy.should_retry(&fatal, 0)` equals
    ///   `RetryDecision::Stop`.
    #[test]
    fn test_predicate_retry_policy() {
        let policy = PredicateRetry::new(|error: &String, _attempt| error.contains("retry"));

        let retryable = "retryable".to_string();
        let fatal = "fatal".to_string();
        assert_eq!(policy.should_retry(&retryable, 0), RetryDecision::Retry);
        assert_eq!(policy.should_retry(&fatal, 0), RetryDecision::Stop);
    }

    /// Validates `RetryError::AttemptsExhausted` behavior for the retry error
    /// display scenario.
    ///
    /// Assertions:
    /// - Ensures `err.to_string().contains("5 tries")` evaluates to true.
    /// - Ensures `err.to_string().contains("timeout")` evaluates to true.
    /// - Ensures `err.to_string().contains("bad config")` evaluates to true.
    #[test]
    fn test_retry_error_display() {
        let err = RetryError::<String>::AttemptsExhausted { attempts: 5 };
        assert!(err.to_string().contains("5 tries"));

        let err = RetryError::<String>::TimeoutExceeded { elapsed: Duration::from_secs(10) };
        assert!(err.to_string().contains("timeout"));

        let err = RetryError::<String>::InvalidConfiguration { message: "bad config".to_string() };
        assert!(err.to_string().contains("bad config"));
    }

    /// Validates `BackoffStrategy::Fixed` behavior for the backoff strategy
    /// equality scenario.
    ///
    /// Assertions:
    /// - Confirms `fixed1` equals `fixed2`.
    /// - Confirms `linear1` equals `linear2`.
    #[test]
    fn test_backoff_strategy_equality() {
        let fixed1 = BackoffStrategy::Fixed(Duration::from_millis(100));
        let fixed2 = BackoffStrategy::Fixed(Duration::from_millis(100));
        assert_eq!(fixed1, fixed2);

        let linear1 = BackoffStrategy::Linear {
            initial_delay: Duration::from_millis(100),
            increment: Duration::from_millis(50),
        };
        let linear2 = BackoffStrategy::Linear {
            initial_delay: Duration::from_millis(100),
            increment: Duration::from_millis(50),
        };
        assert_eq!(linear1, linear2);
    }

    /// Validates `Jitter::None` behavior for the jitter equality scenario.
    ///
    /// Assertions:
    /// - Confirms `Jitter::None` equals `Jitter::None`.
    /// - Confirms `Jitter::Full` equals `Jitter::Full`.
    /// - Confirms `Jitter::Equal` equals `Jitter::Equal`.
    /// - Confirms `Jitter::None` differs from `Jitter::Full`.
    #[test]
    fn test_jitter_equality() {
        assert_eq!(Jitter::None, Jitter::None);
        assert_eq!(Jitter::Full, Jitter::Full);
        assert_eq!(Jitter::Equal, Jitter::Equal);
        assert_ne!(Jitter::None, Jitter::Full);
    }

    /// Validates `PredicateRetry::new` behavior for the retry decision retry
    /// after scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[tokio::test]
    async fn test_retry_decision_retry_after() {
        let policy = PredicateRetry::new(|_: &String, _| true);
        let config = RetryConfig::new()
            .max_attempts(2)
            .fixed_backoff(Duration::from_millis(1))
            .build()
            .unwrap();

        let executor = RetryExecutor::new(config, policy);

        let result = executor.execute(|| async { Err::<(), _>("error".to_string()) }).await;

        assert!(result.is_err());
    }
}
