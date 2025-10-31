use std::sync::Arc;
use std::time::Duration;

use super::{RetryCondition, RetryStrategy};

/// Common retry policies for different error scenarios
pub struct RetryPolicies;

impl RetryPolicies {
    /// Network-related retry policy with longer delays and more attempts
    pub fn network_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(8)
            .unwrap()
            .with_base_delay(Duration::from_millis(500))
            .unwrap()
            .with_max_delay(Duration::from_secs(30))
            .unwrap()
            .with_jitter_factor(0.3)
            .with_timeout(Duration::from_secs(300))
            .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
                Self::is_network_error(err)
            })))
    }

    /// Database retry policy with exponential backoff
    pub fn database_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(5)
            .unwrap()
            .with_base_delay(Duration::from_millis(100))
            .unwrap()
            .with_max_delay(Duration::from_secs(10))
            .unwrap()
            .with_jitter_factor(0.5)
            .with_timeout(Duration::from_secs(60))
            .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
                Self::is_database_transient_error(err)
            })))
    }

    /// Rate limit retry policy with fixed delay
    pub fn rate_limit_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(10)
            .unwrap()
            .with_base_delay(Duration::from_secs(60))
            .unwrap()
            .with_max_delay(Duration::from_secs(60))
            .unwrap()
            .with_jitter_factor(0.1)
            .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
                Self::is_rate_limit_error(err)
            })))
    }

    /// API retry policy for external service calls
    pub fn api_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(3)
            .unwrap()
            .with_base_delay(Duration::from_millis(250))
            .unwrap()
            .with_max_delay(Duration::from_secs(5))
            .unwrap()
            .with_jitter_factor(0.2)
            .with_timeout(Duration::from_secs(30))
            .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
                Self::is_api_retryable_error(err)
            })))
    }

    /// File system retry policy for I/O operations
    pub fn filesystem_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(3)
            .unwrap()
            .with_base_delay(Duration::from_millis(50))
            .unwrap()
            .with_max_delay(Duration::from_secs(1))
            .unwrap()
            .with_jitter_factor(0.1)
            .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
                Self::is_filesystem_transient_error(err)
            })))
    }

    /// Idempotent operation retry policy (safe to retry many times)
    pub fn idempotent_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(10)
            .unwrap()
            .with_base_delay(Duration::from_millis(100))
            .unwrap()
            .with_max_delay(Duration::from_secs(20))
            .unwrap()
            .with_jitter_factor(0.3)
            .with_timeout(Duration::from_secs(120))
    }

    /// Non-idempotent operation retry policy (limited retries)
    pub fn non_idempotent_policy() -> RetryStrategy {
        RetryStrategy::new()
            .with_max_attempts(2)
            .unwrap()
            .with_base_delay(Duration::from_secs(1))
            .unwrap()
            .with_max_delay(Duration::from_secs(5))
            .unwrap()
            .with_jitter_factor(0.1)
    }

    /// Check if error is network-related and retryable
    fn is_network_error(err: &dyn std::error::Error) -> bool {
        // Check error string for common network error patterns
        let err_str = err.to_string().to_lowercase();
        err_str.contains("connection")
            || err_str.contains("timeout")
            || err_str.contains("network")
            || err_str.contains("dns")
            || err_str.contains("refused")
            || err_str.contains("reset")
            || err_str.contains("broken pipe")
            || err_str.contains("unreachable")

        // Note: Cannot use downcast_ref here due to lifetime constraints
    }

    /// Check if error is a transient database error
    fn is_database_transient_error(err: &dyn std::error::Error) -> bool {
        let err_str = err.to_string().to_lowercase();
        err_str.contains("deadlock")
            || err_str.contains("lock")
            || err_str.contains("busy")
            || err_str.contains("database is locked")
            || err_str.contains("transaction")
            || err_str.contains("serialization failure")
            || err_str.contains("could not connect")
            || err_str.contains("connection pool")
    }

    /// Check if error is rate limit related
    fn is_rate_limit_error(err: &dyn std::error::Error) -> bool {
        let err_str = err.to_string().to_lowercase();
        err_str.contains("rate limit")
            || err_str.contains("too many requests")
            || err_str.contains("429")
            || err_str.contains("throttl")
            || err_str.contains("quota exceeded")
    }

    /// Check if API error is retryable
    fn is_api_retryable_error(err: &dyn std::error::Error) -> bool {
        let err_str = err.to_string().to_lowercase();

        // Don't retry client errors (4xx) except rate limits
        if err_str.contains("400")
            || err_str.contains("401")
            || err_str.contains("403")
            || err_str.contains("404")
            || err_str.contains("405")
            || err_str.contains("409")
        {
            return false;
        }

        // Retry server errors and network issues
        err_str.contains("500")
            || err_str.contains("502")
            || err_str.contains("503")
            || err_str.contains("504")
            || err_str.contains("timeout")
            || err_str.contains("gateway")
            || err_str.contains("service unavailable")
            || Self::is_network_error(err)
    }

    /// Check if filesystem error is transient
    fn is_filesystem_transient_error(err: &dyn std::error::Error) -> bool {
        // Note: Cannot use downcast_ref here due to lifetime constraints
        // Check error string for common transient filesystem patterns
        let err_str = err.to_string().to_lowercase();
        err_str.contains("temporarily unavailable")
            || err_str.contains("resource busy")
            || err_str.contains("locked")
            || err_str.contains("in use")
            || err_str.contains("interrupted")
            || err_str.contains("would block")
            || err_str.contains("timed out")
            || err_str.contains("write zero")
            || err_str.contains("unexpected eof")
    }
}

/// Builder for custom retry policies
pub struct RetryPolicyBuilder {
    strategy: RetryStrategy,
}

impl Default for RetryPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryPolicyBuilder {
    /// Create a new policy builder
    pub fn new() -> Self {
        Self { strategy: RetryStrategy::new() }
    }

    /// Set max attempts
    pub fn max_attempts(mut self, attempts: u32) -> Result<Self, super::RetryError> {
        self.strategy = self.strategy.with_max_attempts(attempts)?;
        Ok(self)
    }

    /// Set base delay
    pub fn base_delay(mut self, delay: Duration) -> Result<Self, super::RetryError> {
        self.strategy = self.strategy.with_base_delay(delay)?;
        Ok(self)
    }

    /// Set max delay
    pub fn max_delay(mut self, delay: Duration) -> Result<Self, super::RetryError> {
        self.strategy = self.strategy.with_max_delay(delay)?;
        Ok(self)
    }

    /// Set jitter factor
    pub fn jitter(mut self, factor: f64) -> Self {
        self.strategy = self.strategy.with_jitter_factor(factor);
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.strategy = self.strategy.with_timeout(timeout);
        self
    }

    /// Add custom retry condition
    pub fn when<F>(mut self, condition: F) -> Self
    where
        F: Fn(&dyn std::error::Error) -> bool + Send + Sync + 'static,
    {
        self.strategy =
            self.strategy.with_retry_condition(RetryCondition::Custom(Arc::new(condition)));
        self
    }

    /// Build the retry strategy
    pub fn build(self) -> RetryStrategy {
        self.strategy
    }
}
