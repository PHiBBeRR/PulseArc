// Constants for retry module
use std::time::Duration;

/// Default maximum number of retry attempts
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// Default base delay for exponential backoff
pub const DEFAULT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Default maximum delay cap
pub const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(60);

/// Default jitter factor (0.0 = no jitter, 1.0 = full jitter)
pub const DEFAULT_JITTER_FACTOR: f64 = 0.3;

/// Maximum exponent for exponential backoff calculation to prevent overflow
pub const MAX_BACKOFF_EXPONENT: u32 = 30;

/// Circuit breaker: default failure threshold
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;

/// Circuit breaker: default success threshold for recovery
pub const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;

/// Circuit breaker: default timeout before attempting recovery
pub const DEFAULT_CIRCUIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Circuit breaker: default max requests in half-open state
pub const DEFAULT_HALF_OPEN_REQUESTS: u32 = 1;

/// Retry budget: token refill check interval
pub const BUDGET_REFILL_INTERVAL: Duration = Duration::from_secs(1);

/// Minimum allowed max_attempts value
pub const MIN_MAX_ATTEMPTS: u32 = 1;

/// Maximum allowed max_attempts value
pub const MAX_MAX_ATTEMPTS: u32 = 100;
