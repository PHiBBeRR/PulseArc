// Retry strategy with exponential backoff and jitter
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::Rng;
use tracing::{debug, error, warn};

use crate::error::CommonError;
use crate::resilience::retry::{
    BackoffStrategy as CoreBackoffStrategy, Jitter as CoreJitter, RetryConfig,
    RetryDecision as CoreRetryDecision, RetryError as CoreRetryError,
    RetryExecutor as CoreRetryExecutor, RetryOutcome as CoreRetryOutcome,
    RetryPolicy as CoreRetryPolicy,
};
use crate::sync::retry::constants::*;
use crate::sync::retry::error::{RetryError, RetryResult};
use crate::sync::retry::metrics::RetryMetrics;
use crate::sync::retry::tracing::{RetrySpan, RetryTracer};

/// Type alias for retry result with metrics (clippy::type_complexity)
type RetryResultWithMetrics<T> = (Result<T, RetryError>, RetryMetrics);

/// Type alias for error predicate function to reduce complexity
type ErrorPredicate = Arc<dyn Fn(&dyn std::error::Error) -> bool + Send + Sync>;

/// Retry strategy with configurable exponential backoff and jitter
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter_factor: f64,
    timeout: Option<Duration>,
    retry_on: RetryCondition,
}

/// Condition for determining if an error is retryable
pub enum RetryCondition {
    /// Retry all errors
    Always,
    /// Retry specific error types
    Custom(ErrorPredicate),
}

impl Clone for RetryCondition {
    fn clone(&self) -> Self {
        match self {
            Self::Always => Self::Always,
            Self::Custom(f) => Self::Custom(Arc::clone(f)),
        }
    }
}

impl std::fmt::Debug for RetryCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "Always"),
            Self::Custom(_) => write!(f, "Custom(<function>)"),
        }
    }
}

impl Default for RetryCondition {
    fn default() -> Self {
        Self::Always
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            base_delay: DEFAULT_BASE_DELAY,
            max_delay: DEFAULT_MAX_DELAY,
            jitter_factor: DEFAULT_JITTER_FACTOR,
            timeout: None,
            retry_on: RetryCondition::default(),
        }
    }
}

impl RetryStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a custom retry strategy with validation
    pub fn custom(
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
    ) -> RetryResult<Self> {
        if !(MIN_MAX_ATTEMPTS..=MAX_MAX_ATTEMPTS).contains(&max_attempts) {
            return Err(CommonError::config(format!(
                "max_attempts must be between {} and {}, got {}",
                MIN_MAX_ATTEMPTS, MAX_MAX_ATTEMPTS, max_attempts
            ))
            .into());
        }

        if base_delay > max_delay {
            return Err(CommonError::config(format!(
                "base_delay ({:?}) cannot be greater than max_delay ({:?})",
                base_delay, max_delay
            ))
            .into());
        }

        Ok(Self {
            max_attempts,
            base_delay,
            max_delay,
            jitter_factor: DEFAULT_JITTER_FACTOR,
            timeout: None,
            retry_on: RetryCondition::default(),
        })
    }

    /// Set timeout for entire retry operation
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set custom retry condition
    pub fn with_retry_condition(mut self, condition: RetryCondition) -> Self {
        self.retry_on = condition;
        self
    }

    /// Set the maximum number of retry attempts with validation
    pub fn with_max_attempts(mut self, attempts: u32) -> RetryResult<Self> {
        if !(MIN_MAX_ATTEMPTS..=MAX_MAX_ATTEMPTS).contains(&attempts) {
            return Err(CommonError::config(format!(
                "max_attempts must be between {} and {}, got {}",
                MIN_MAX_ATTEMPTS, MAX_MAX_ATTEMPTS, attempts
            ))
            .into());
        }
        self.max_attempts = attempts;
        Ok(self)
    }

    /// Set the base delay for exponential backoff
    pub fn with_base_delay(mut self, delay: Duration) -> RetryResult<Self> {
        if delay > self.max_delay {
            return Err(CommonError::config(format!(
                "base_delay ({:?}) cannot be greater than max_delay ({:?})",
                delay, self.max_delay
            ))
            .into());
        }
        self.base_delay = delay;
        Ok(self)
    }

    /// Set the maximum delay cap
    pub fn with_max_delay(mut self, delay: Duration) -> RetryResult<Self> {
        if delay < self.base_delay {
            return Err(CommonError::config(format!(
                "max_delay ({:?}) cannot be less than base_delay ({:?})",
                delay, self.base_delay
            ))
            .into());
        }
        self.max_delay = delay;
        Ok(self)
    }

    /// Set the jitter factor (0.0 = no jitter, 1.0 = full jitter)
    pub fn with_jitter_factor(mut self, factor: f64) -> Self {
        self.jitter_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// Calculate delay for a given attempt with exponential backoff and jitter
    pub fn get_delay(&self, attempt: u32) -> Duration {
        // Calculate base exponential delay
        let exponential_delay = self.calculate_exponential_delay(attempt);

        // Apply jitter
        self.apply_jitter(exponential_delay)
    }

    /// Calculate exponential delay without jitter
    fn calculate_exponential_delay(&self, attempt: u32) -> Duration {
        // Use checked multiplication to prevent overflow
        let base_millis = self.base_delay.as_millis() as u64;
        let max_millis = self.max_delay.as_millis() as u64;

        // Cap exponent to prevent overflow
        let exponent = attempt.min(MAX_BACKOFF_EXPONENT);
        let multiplier = 2_u64.saturating_pow(exponent);

        // Calculate delay with saturating multiplication
        let delay_millis = base_millis.saturating_mul(multiplier).min(max_millis);

        Duration::from_millis(delay_millis)
    }

    /// Apply jitter to prevent thundering herd
    fn apply_jitter(&self, delay: Duration) -> Duration {
        if self.jitter_factor == 0.0 {
            return delay;
        }

        let mut rng = rand::thread_rng();
        let delay_millis = delay.as_millis() as f64;
        let jitter_range = delay_millis * self.jitter_factor;

        // Add random jitter: -jitter_range/2 to +jitter_range/2
        let jitter = rng.gen_range(-jitter_range / 2.0..=jitter_range / 2.0);
        let final_millis = (delay_millis + jitter).max(0.0) as u64;

        Duration::from_millis(final_millis)
    }

    /// Check if retry should be attempted
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }

    /// Get the maximum number of attempts
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Execute an operation with retry logic and metrics
    pub async fn execute_with_metrics<F, Fut, T, E>(
        &self,
        operation_name: &str,
        mut operation: F,
    ) -> RetryResultWithMetrics<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static + Clone,
    {
        let tracer = RetryTracer::new();
        let span = tracer.start_retry_span(operation_name, self.max_attempts);
        let instrumentation = Arc::new(Mutex::new(RetryInstrumentation::new(span)));

        let config = RetryConfig {
            max_attempts: self.max_attempts,
            max_total_time: self.timeout,
            backoff: CoreBackoffStrategy::Fixed(Duration::ZERO),
            jitter: CoreJitter::None,
            ..RetryConfig::default()
        };

        let policy = SyncRetryPolicy::new(self.clone(), instrumentation.clone());
        let executor = CoreRetryExecutor::new(config, policy);

        let attempt_counter = Arc::new(AtomicU32::new(0));
        let instrumentation_for_closure = instrumentation.clone();

        let outcome = executor
            .execute_with_outcome(|| {
                let attempt_index = attempt_counter.fetch_add(1, Ordering::SeqCst);
                let fut = operation();
                let instrumentation = instrumentation_for_closure.clone();
                async move {
                    if let Ok(mut guard) = instrumentation.lock() {
                        let previous_delay =
                            if attempt_index > 0 { guard.last_delay } else { None };
                        if let Some(span) = guard.span.as_mut() {
                            span.record_attempt(attempt_index + 1, previous_delay);
                        }
                    }
                    fut.await
                }
            })
            .await;

        self.process_outcome(outcome, instrumentation, operation_name)
    }

    /// Execute an operation with retry logic
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, RetryError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static + Clone,
    {
        let (result, _) = self.execute_with_metrics("unnamed", operation).await;
        result
    }

    fn process_outcome<T, E>(
        &self,
        outcome: CoreRetryOutcome<T, E>,
        instrumentation: Arc<Mutex<RetryInstrumentation>>,
        operation_name: &str,
    ) -> RetryResultWithMetrics<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut metrics = RetryMetrics {
            attempts: outcome.attempts,
            total_delay: outcome.total_delay,
            timed_out: outcome.timed_out,
            succeeded: outcome.result.is_ok(),
        };

        let result = match outcome.result {
            Ok(value) => {
                self.finish_span(&instrumentation, |span| {
                    span.record_success(metrics.attempts, metrics.total_delay);
                });
                Ok(value)
            }
            Err(core_err) => {
                let mapped = match core_err {
                    CoreRetryError::AttemptsExhausted { attempts } => {
                        self.finish_span(&instrumentation, |span| {
                            span.record_exhausted(metrics.total_delay);
                        });
                        RetryError::AttemptsExhausted { attempts }
                    }
                    CoreRetryError::TimeoutExceeded { elapsed } => {
                        metrics.timed_out = true;
                        self.finish_span(&instrumentation, |span| span.record_timeout(elapsed));
                        RetryError::Common(CommonError::timeout("retry_operation", elapsed))
                    }
                    CoreRetryError::InvalidConfiguration { message } => {
                        self.finish_span(&instrumentation, |_| {});
                        RetryError::Common(CommonError::config(message))
                    }
                    CoreRetryError::NonRetryable { source } => {
                        self.finish_span(&instrumentation, |_| {});
                        RetryError::operation_failed(source)
                    }
                };
                Err(mapped)
            }
        };

        if metrics.timed_out {
            warn!(
                operation = operation_name,
                attempts = metrics.attempts,
                total_delay = ?metrics.total_delay,
                "Retry operation timed out"
            );
        } else if matches!(result, Err(RetryError::AttemptsExhausted { .. })) {
            error!(
                operation = operation_name,
                attempts = metrics.attempts,
                total_delay = ?metrics.total_delay,
                "All retry attempts failed"
            );
        }

        (result, metrics)
    }

    fn finish_span<F>(&self, instrumentation: &Arc<Mutex<RetryInstrumentation>>, recorder: F)
    where
        F: FnOnce(&mut RetrySpan),
    {
        if let Ok(mut guard) = instrumentation.lock() {
            if let Some(mut span) = guard.span.take() {
                recorder(&mut span);
                span.end();
            }
        }
    }

    /// Execute a synchronous operation with retry logic
    ///
    /// WARNING: This uses std::thread::sleep for delays. Do not call this
    /// from within an async runtime as it will block the thread.
    pub fn execute_sync<F, T, E>(&self, operation: F) -> Result<T, RetryError>
    where
        F: FnMut() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.execute_sync_with_metrics("unnamed", operation).0
    }

    /// Execute a synchronous operation with retry logic and metrics
    ///
    /// WARNING: This uses std::thread::sleep for delays. Do not call this
    /// from within an async runtime as it will block the thread.
    pub fn execute_sync_with_metrics<F, T, E>(
        &self,
        operation_name: &str,
        mut operation: F,
    ) -> RetryResultWithMetrics<T>
    where
        F: FnMut() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let start_time = Instant::now();
        let deadline = self.timeout.map(|t| start_time + t);
        let mut metrics = RetryMetrics::default();
        let mut last_error = None;
        let mut total_delay = Duration::ZERO;

        let mut retry_span = {
            use crate::sync::retry::tracing::RetryTracer;
            let tracer = RetryTracer::new();
            Some(tracer.start_retry_span(operation_name, self.max_attempts))
        };

        debug!(
            operation = operation_name,
            max_attempts = self.max_attempts,
            timeout = ?self.timeout,
            "Starting sync retry operation"
        );

        for attempt in 0..self.max_attempts {
            metrics.attempts = attempt + 1;

            // Check timeout
            if let Some(deadline) = deadline {
                if Instant::now() > deadline {
                    warn!(
                        operation = operation_name,
                        attempts = metrics.attempts,
                        elapsed = ?start_time.elapsed(),
                        "Sync retry operation timed out"
                    );
                    metrics.timed_out = true;
                    metrics.total_delay = total_delay;
                    return (
                        Err(CommonError::timeout("retry_operation", start_time.elapsed()).into()),
                        metrics,
                    );
                }
            }

            match operation() {
                Ok(result) => {
                    if attempt > 0 {
                        debug!(
                            operation = operation_name,
                            attempts = metrics.attempts,
                            total_delay = ?total_delay,
                            "Sync retry operation succeeded"
                        );
                    }
                    metrics.succeeded = true;
                    metrics.total_delay = total_delay;
                    return (Ok(result), metrics);
                }
                Err(err) => {
                    // Check if error is retryable
                    let should_retry = match &self.retry_on {
                        RetryCondition::Always => true,
                        RetryCondition::Custom(f) => f(&err as &dyn std::error::Error),
                    };

                    if let Some(ref mut span) = retry_span {
                        span.record_failure(attempt + 1, &err.to_string());
                    }

                    if !should_retry {
                        debug!(
                            operation = operation_name,
                            error = %err,
                            "Error is not retryable"
                        );
                        return (Err(RetryError::operation_failed(err)), metrics);
                    }

                    last_error = Some(err);

                    if attempt < self.max_attempts - 1 {
                        let delay = self.get_delay(attempt);
                        total_delay += delay;

                        warn!(
                            operation = operation_name,
                            attempt = attempt + 1,
                            max_attempts = self.max_attempts,
                            delay = ?delay,
                            error = %last_error.as_ref().unwrap(),
                            "Sync retry attempt failed, backing off"
                        );

                        // Check if delay would exceed deadline
                        if let Some(deadline) = deadline {
                            let time_until_deadline =
                                deadline.saturating_duration_since(Instant::now());
                            if delay > time_until_deadline {
                                warn!(
                                    operation = operation_name,
                                    "Next retry would exceed timeout, aborting"
                                );
                                metrics.timed_out = true;
                                metrics.total_delay = total_delay;
                                return (
                                    Err(CommonError::timeout(
                                        "retry_operation",
                                        start_time.elapsed(),
                                    )
                                    .into()),
                                    metrics,
                                );
                            }
                        }

                        // WARNING: Blocking sleep - should not be called from async runtime
                        std::thread::sleep(delay);
                    }
                }
            }
        }

        error!(
            operation = operation_name,
            attempts = metrics.attempts,
            total_delay = ?total_delay,
            "All sync retry attempts failed"
        );

        metrics.total_delay = total_delay;
        let error = last_error
            .map(RetryError::operation_failed)
            .unwrap_or_else(|| RetryError::AttemptsExhausted { attempts: metrics.attempts });
        (Err(error), metrics)
    }
}

struct RetryInstrumentation {
    span: Option<RetrySpan>,
    last_delay: Option<Duration>,
}

impl RetryInstrumentation {
    fn new(span: RetrySpan) -> Self {
        Self { span: Some(span), last_delay: None }
    }
}

impl fmt::Debug for RetryInstrumentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryInstrumentation")
            .field("span", &self.span.is_some())
            .field("last_delay", &self.last_delay)
            .finish()
    }
}

#[derive(Clone)]
struct SyncRetryPolicy {
    strategy: RetryStrategy,
    instrumentation: Arc<Mutex<RetryInstrumentation>>,
}

impl SyncRetryPolicy {
    fn new(strategy: RetryStrategy, instrumentation: Arc<Mutex<RetryInstrumentation>>) -> Self {
        Self { strategy, instrumentation }
    }
}

impl<E> CoreRetryPolicy<E> for SyncRetryPolicy
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn should_retry(&self, error: &E, attempt: u32) -> CoreRetryDecision {
        let err_ref = error as &dyn std::error::Error;
        let mut guard = self.instrumentation.lock().ok();

        if let Some(span) = guard.as_mut().and_then(|g| g.span.as_mut()) {
            span.record_failure(attempt + 1, &error.to_string());
        }

        if self.strategy.retry_on.allows_error(err_ref) {
            let delay = self.strategy.get_delay(attempt);
            if let Some(g) = guard.as_mut() {
                g.last_delay = Some(delay);
            }

            warn!(
                attempt = attempt + 1,
                max_attempts = self.strategy.max_attempts(),
                delay = ?delay,
                error = %error,
                "Retry attempt failed, backing off"
            );

            CoreRetryDecision::RetryAfter(delay)
        } else {
            CoreRetryDecision::Stop
        }
    }
}

impl RetryCondition {
    fn allows_error(&self, error: &dyn std::error::Error) -> bool {
        match self {
            RetryCondition::Always => true,
            RetryCondition::Custom(predicate) => predicate(error),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::retry::strategy.
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// Validates `RetryStrategy::default` behavior for the default retry
    /// strategy scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.max_attempts` equals `DEFAULT_MAX_ATTEMPTS`.
    /// - Confirms `strategy.base_delay` equals `DEFAULT_BASE_DELAY`.
    /// - Confirms `strategy.max_delay` equals `DEFAULT_MAX_DELAY`.
    #[test]
    fn test_default_retry_strategy() {
        let strategy = RetryStrategy::default();

        assert_eq!(strategy.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(strategy.base_delay, DEFAULT_BASE_DELAY);
        assert_eq!(strategy.max_delay, DEFAULT_MAX_DELAY);
    }

    /// Validates `RetryStrategy::custom` behavior for the custom strategy
    /// creation scenario.
    ///
    /// Assertions:
    /// - Ensures `strategy.is_ok()` evaluates to true.
    /// - Confirms `strategy.max_attempts` equals `3`.
    #[test]
    fn test_custom_strategy_creation() {
        let strategy = RetryStrategy::custom(3, Duration::from_millis(100), Duration::from_secs(5));

        assert!(strategy.is_ok());
        let strategy = strategy.unwrap();
        assert_eq!(strategy.max_attempts, 3);
    }

    /// Validates `RetryStrategy::custom` behavior for the custom strategy
    /// invalid attempts scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_custom_strategy_invalid_attempts() {
        let result = RetryStrategy::custom(
            0, // Invalid
            Duration::from_millis(100),
            Duration::from_secs(5),
        );

        assert!(result.is_err());
    }

    /// Validates `RetryStrategy::custom` behavior for the custom strategy base
    /// delay exceeds max scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_custom_strategy_base_delay_exceeds_max() {
        let result = RetryStrategy::custom(
            3,
            Duration::from_secs(10),
            Duration::from_secs(5), // Max less than base
        );

        assert!(result.is_err());
    }

    /// Validates `RetryStrategy::new` behavior for the with max attempts
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.max_attempts` equals `7`.
    #[test]
    fn test_with_max_attempts() {
        let strategy = RetryStrategy::new().with_max_attempts(7).unwrap();

        assert_eq!(strategy.max_attempts, 7);
    }

    /// Validates `RetryStrategy::new` behavior for the with max attempts
    /// invalid scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_with_max_attempts_invalid() {
        let result = RetryStrategy::new().with_max_attempts(0);

        assert!(result.is_err());
    }

    /// Validates `Duration::from_millis` behavior for the with base delay
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.base_delay` equals `delay`.
    #[test]
    fn test_with_base_delay() {
        let delay = Duration::from_millis(500);
        let strategy = RetryStrategy::new().with_base_delay(delay).unwrap();

        assert_eq!(strategy.base_delay, delay);
    }

    /// Validates `Duration::from_secs` behavior for the with max delay
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.max_delay` equals `delay`.
    #[test]
    fn test_with_max_delay() {
        let delay = Duration::from_secs(120);
        let strategy = RetryStrategy::new().with_max_delay(delay).unwrap();

        assert_eq!(strategy.max_delay, delay);
    }

    /// Validates `RetryStrategy::new` behavior for the with jitter factor
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.jitter_factor` equals `0.3`.
    #[test]
    fn test_with_jitter_factor() {
        let strategy = RetryStrategy::new().with_jitter_factor(0.3);

        assert_eq!(strategy.jitter_factor, 0.3);
    }

    /// Validates `RetryStrategy::new` behavior for the with jitter factor
    /// clamping scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.jitter_factor` equals `1.0`.
    #[test]
    fn test_with_jitter_factor_clamping() {
        // Values > 1.0 should be clamped to 1.0
        let strategy = RetryStrategy::new().with_jitter_factor(1.5);

        assert_eq!(strategy.jitter_factor, 1.0);
    }

    /// Validates `Duration::from_secs` behavior for the with timeout scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.timeout` equals `Some(timeout)`.
    #[test]
    fn test_with_timeout() {
        let timeout = Duration::from_secs(30);
        let strategy = RetryStrategy::new().with_timeout(timeout);

        assert_eq!(strategy.timeout, Some(timeout));
    }

    /// Validates `RetryStrategy::new` behavior for the exponential backoff
    /// calculation scenario.
    ///
    /// Assertions:
    /// - Ensures `delay1.as_secs() >= 2 * delay0.as_secs()` evaluates to true.
    /// - Ensures `delay2.as_secs() >= 2 * delay1.as_secs()` evaluates to true.
    #[test]
    fn test_exponential_backoff_calculation() {
        let strategy = RetryStrategy::new()
            .with_base_delay(Duration::from_secs(1))
            .unwrap()
            .with_jitter_factor(0.0); // No jitter for predictable test

        let delay0 = strategy.get_delay(0);
        let delay1 = strategy.get_delay(1);
        let delay2 = strategy.get_delay(2);

        // Exponential backoff: base * 2^attempt
        assert!(delay1.as_secs() >= 2 * delay0.as_secs());
        assert!(delay2.as_secs() >= 2 * delay1.as_secs());
    }

    /// Validates `RetryStrategy::new` behavior for the max delay capping
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `delay <= Duration::from_secs(5)` evaluates to true.
    #[test]
    fn test_max_delay_capping() {
        let strategy = RetryStrategy::new()
            .with_base_delay(Duration::from_secs(1))
            .unwrap()
            .with_max_delay(Duration::from_secs(5))
            .unwrap()
            .with_jitter_factor(0.0);

        // Attempt 10 should exceed max_delay without cap
        let delay = strategy.get_delay(10);

        assert!(delay <= Duration::from_secs(5));
    }

    /// Validates `RetryStrategy::new` behavior for the jitter adds randomness
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!all_same` evaluates to true.
    #[test]
    fn test_jitter_adds_randomness() {
        let strategy = RetryStrategy::new()
            .with_base_delay(Duration::from_millis(100))
            .unwrap()
            .with_jitter_factor(0.5);

        // Get multiple delays for same attempt - should vary due to jitter
        let mut delays = Vec::new();
        for _ in 0..5 {
            delays.push(strategy.get_delay(0));
        }

        // At least some should be different (very high probability)
        let all_same = delays.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same);
    }

    /// Validates `RetryStrategy::new` behavior for the should retry scenario.
    ///
    /// Assertions:
    /// - Ensures `strategy.should_retry(0)` evaluates to true.
    /// - Ensures `strategy.should_retry(1)` evaluates to true.
    /// - Ensures `strategy.should_retry(2)` evaluates to true.
    /// - Ensures `!strategy.should_retry(3)` evaluates to true.
    /// - Ensures `!strategy.should_retry(4)` evaluates to true.
    #[test]
    fn test_should_retry() {
        let strategy = RetryStrategy::new().with_max_attempts(3).unwrap();

        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(1));
        assert!(strategy.should_retry(2));
        assert!(!strategy.should_retry(3));
        assert!(!strategy.should_retry(4));
    }

    /// Validates `RetryStrategy::new` behavior for the execute success no retry
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `"success"`.
    /// - Confirms `counter.load(Ordering::SeqCst)` equals `1`.
    #[tokio::test]
    async fn test_execute_success_no_retry() {
        let strategy = RetryStrategy::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = strategy
            .execute(move || {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, std::io::Error>("success")
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    /// Validates `RetryStrategy::new` behavior for the execute retry until
    /// success scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `counter.load(Ordering::SeqCst)` equals `3`.
    #[tokio::test]
    async fn test_execute_retry_until_success() {
        let strategy = RetryStrategy::new()
            .with_max_attempts(5)
            .unwrap()
            .with_base_delay(Duration::from_millis(1))
            .unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = strategy
            .execute(move || {
                let counter = Arc::clone(&counter_clone);
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(std::io::Error::other("fail"))
                    } else {
                        Ok("success")
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    /// Validates `RetryStrategy::new` behavior for the execute exhaust attempts
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Confirms `counter.load(Ordering::SeqCst)` equals `3`.
    #[tokio::test]
    async fn test_execute_exhaust_attempts() {
        let strategy = RetryStrategy::new()
            .with_max_attempts(3)
            .unwrap()
            .with_base_delay(Duration::from_millis(1))
            .unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = strategy
            .execute(move || {
                let counter = Arc::clone(&counter_clone);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(std::io::Error::other("fail"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    /// Validates `RetryStrategy::new` behavior for the clone scenario.
    ///
    /// Assertions:
    /// - Confirms `strategy.max_attempts` equals `cloned.max_attempts`.
    /// - Confirms `strategy.base_delay` equals `cloned.base_delay`.
    /// - Confirms `strategy.max_delay` equals `cloned.max_delay`.
    #[test]
    fn test_clone() {
        let strategy = RetryStrategy::new()
            .with_max_attempts(7)
            .unwrap()
            .with_base_delay(Duration::from_millis(500))
            .unwrap();

        let cloned = strategy.clone();

        assert_eq!(strategy.max_attempts, cloned.max_attempts);
        assert_eq!(strategy.base_delay, cloned.base_delay);
        assert_eq!(strategy.max_delay, cloned.max_delay);
    }

    /// Validates `RetryStrategy::new` behavior for the delay increases with
    /// attempts scenario.
    ///
    /// Assertions:
    /// - Ensures `delay2 > delay1` evaluates to true.
    /// - Ensures `delay3 > delay2` evaluates to true.
    #[test]
    fn test_delay_increases_with_attempts() {
        let strategy = RetryStrategy::new().with_jitter_factor(0.0);

        let delay1 = strategy.get_delay(1);
        let delay2 = strategy.get_delay(2);
        let delay3 = strategy.get_delay(3);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
    }
}
