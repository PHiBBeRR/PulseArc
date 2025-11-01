//! Rate limiting implementations for controlling request rates
//!
//! This module provides two common rate limiting algorithms:
//! - **Token Bucket**: Allows bursts up to a maximum capacity
//! - **Leaky Bucket**: Enforces a smooth, constant rate
//!
//! Both algorithms support time-based token refill and concurrent access.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use super::{Clock, SystemClock};

/// Configuration for token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucketConfig {
    /// Maximum number of tokens the bucket can hold
    pub capacity: u64,
    /// Number of tokens to refill per interval
    pub refill_amount: u64,
    /// Time interval for token refill
    pub refill_interval: Duration,
}

impl Default for TokenBucketConfig {
    fn default() -> Self {
        Self { capacity: 100, refill_amount: 10, refill_interval: Duration::from_secs(1) }
    }
}

impl TokenBucketConfig {
    /// Create a new configuration builder
    pub fn builder() -> TokenBucketConfigBuilder {
        TokenBucketConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.capacity == 0 {
            return Err("capacity must be greater than 0".to_string());
        }
        if self.refill_amount == 0 {
            return Err("refill_amount must be greater than 0".to_string());
        }
        if self.refill_interval.is_zero() {
            return Err("refill_interval must be greater than zero".to_string());
        }
        Ok(())
    }
}

/// Builder for TokenBucketConfig
#[derive(Debug)]
pub struct TokenBucketConfigBuilder {
    config: TokenBucketConfig,
}

impl Default for TokenBucketConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenBucketConfigBuilder {
    pub fn new() -> Self {
        Self { config: TokenBucketConfig::default() }
    }

    pub fn capacity(mut self, capacity: u64) -> Self {
        self.config.capacity = capacity;
        self
    }

    pub fn refill_amount(mut self, amount: u64) -> Self {
        self.config.refill_amount = amount;
        self
    }

    pub fn refill_interval(mut self, interval: Duration) -> Self {
        self.config.refill_interval = interval;
        self
    }

    pub fn build(self) -> Result<TokenBucketConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Token bucket rate limiter
///
/// Allows bursts of requests up to the capacity, then refills tokens at a fixed
/// rate. Good for scenarios where occasional bursts are acceptable.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
///
/// use pulsearc_common::resilience::TokenBucket;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let limiter = TokenBucket::new(10, 5, Duration::from_secs(1))?;
///
/// // Try to acquire 3 tokens
/// if limiter.try_acquire(3) {
///     println!("Request allowed");
/// } else {
///     println!("Rate limit exceeded");
/// }
/// # Ok(())
/// # }
/// ```
pub struct TokenBucket<C: Clock = SystemClock> {
    config: TokenBucketConfig,
    tokens: Arc<AtomicU64>,
    last_refill: Arc<RwLock<Instant>>,
    clock: Arc<C>,
}

impl<C: Clock> TokenBucket<C> {
    /// Create a new token bucket with custom clock
    pub fn with_clock(
        capacity: u64,
        refill_amount: u64,
        refill_interval: Duration,
        clock: C,
    ) -> Result<Self, String> {
        let config = TokenBucketConfig { capacity, refill_amount, refill_interval };
        config.validate()?;

        Ok(Self {
            tokens: Arc::new(AtomicU64::new(capacity)),
            last_refill: Arc::new(RwLock::new(clock.now())),
            clock: Arc::new(clock),
            config,
        })
    }

    /// Refill tokens based on elapsed time
    fn refill(&self) {
        let now = self.clock.now();

        let last_refill = match self.last_refill.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                warn!("Token bucket last_refill lock poisoned");
                *poisoned.into_inner()
            }
        };

        let elapsed = now.duration_since(last_refill);
        let refills = elapsed.as_millis() / self.config.refill_interval.as_millis();

        if refills > 0 {
            let tokens_to_add = (refills as u64).saturating_mul(self.config.refill_amount);
            let current = self.tokens.load(Ordering::Acquire);
            let new_tokens = current.saturating_add(tokens_to_add).min(self.config.capacity);

            self.tokens.store(new_tokens, Ordering::Release);

            // Update last_refill time
            if let Ok(mut guard) = self.last_refill.write() {
                *guard = now;
            }

            debug!(
                "Refilled {} tokens (now {})",
                tokens_to_add,
                self.tokens.load(Ordering::Acquire)
            );
        }
    }

    /// Try to acquire the specified number of tokens
    ///
    /// Returns `true` if tokens were acquired, `false` if not enough tokens
    /// available.
    pub fn try_acquire(&self, tokens: u64) -> bool {
        self.refill();

        let mut current = self.tokens.load(Ordering::Acquire);

        loop {
            if current < tokens {
                debug!("Rate limit: insufficient tokens ({} < {})", current, tokens);
                return false;
            }

            let new_value = current - tokens;
            match self.tokens.compare_exchange_weak(
                current,
                new_value,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    debug!("Acquired {} tokens ({} remaining)", tokens, new_value);
                    return true;
                }
                Err(actual) => {
                    current = actual;
                }
            }
        }
    }

    /// Get the current number of available tokens
    pub fn available_tokens(&self) -> u64 {
        self.refill();
        self.tokens.load(Ordering::Acquire)
    }

    /// Reset the limiter to full capacity
    pub fn reset(&self) {
        self.tokens.store(self.config.capacity, Ordering::Release);
        if let Ok(mut guard) = self.last_refill.write() {
            *guard = self.clock.now();
        }
    }
}

impl TokenBucket<SystemClock> {
    /// Create a new token bucket with system clock
    pub fn new(
        capacity: u64,
        refill_amount: u64,
        refill_interval: Duration,
    ) -> Result<Self, String> {
        Self::with_clock(capacity, refill_amount, refill_interval, SystemClock)
    }
}

impl<C: Clock> Clone for TokenBucket<C> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tokens: Arc::clone(&self.tokens),
            last_refill: Arc::clone(&self.last_refill),
            clock: Arc::clone(&self.clock),
        }
    }
}

/// Configuration for leaky bucket rate limiter
#[derive(Debug, Clone)]
pub struct LeakyBucketConfig {
    /// Maximum number of requests the bucket can hold
    pub capacity: u64,
    /// Rate at which requests leak out (requests per second)
    pub leak_rate: f64,
}

impl Default for LeakyBucketConfig {
    fn default() -> Self {
        Self { capacity: 100, leak_rate: 10.0 }
    }
}

impl LeakyBucketConfig {
    /// Create a new configuration builder
    pub fn builder() -> LeakyBucketConfigBuilder {
        LeakyBucketConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.capacity == 0 {
            return Err("capacity must be greater than 0".to_string());
        }
        if self.leak_rate <= 0.0 {
            return Err("leak_rate must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Builder for LeakyBucketConfig
#[derive(Debug)]
pub struct LeakyBucketConfigBuilder {
    config: LeakyBucketConfig,
}

impl Default for LeakyBucketConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LeakyBucketConfigBuilder {
    pub fn new() -> Self {
        Self { config: LeakyBucketConfig::default() }
    }

    pub fn capacity(mut self, capacity: u64) -> Self {
        self.config.capacity = capacity;
        self
    }

    pub fn leak_rate(mut self, rate: f64) -> Self {
        self.config.leak_rate = rate;
        self
    }

    pub fn build(self) -> Result<LeakyBucketConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Leaky bucket rate limiter
///
/// Enforces a smooth, constant rate by leaking requests at a fixed rate.
/// Requests that would overflow the bucket are rejected.
///
/// # Examples
///
/// ```rust
/// use pulsearc_common::resilience::LeakyBucket;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let limiter = LeakyBucket::new(10, 5.0)?; // 5 requests per second
///
/// if limiter.try_acquire() {
///     println!("Request allowed");
/// } else {
///     println!("Rate limit exceeded");
/// }
/// # Ok(())
/// # }
/// ```
pub struct LeakyBucket<C: Clock = SystemClock> {
    config: LeakyBucketConfig,
    level: Arc<AtomicU64>, // Current level in millirequests (level * 1000)
    last_leak: Arc<RwLock<Instant>>,
    clock: Arc<C>,
}

impl<C: Clock> LeakyBucket<C> {
    /// Create a new leaky bucket with custom clock
    pub fn with_clock(capacity: u64, leak_rate: f64, clock: C) -> Result<Self, String> {
        let config = LeakyBucketConfig { capacity, leak_rate };
        config.validate()?;

        Ok(Self {
            config,
            level: Arc::new(AtomicU64::new(0)),
            last_leak: Arc::new(RwLock::new(clock.now())),
            clock: Arc::new(clock),
        })
    }

    /// Leak requests based on elapsed time
    fn leak(&self) {
        let now = self.clock.now();

        let last_leak = match self.last_leak.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                warn!("Leaky bucket last_leak lock poisoned");
                *poisoned.into_inner()
            }
        };

        let elapsed = now.duration_since(last_leak);
        let elapsed_secs = elapsed.as_secs_f64();

        // Calculate how much should have leaked
        let leak_amount_milli = (elapsed_secs * self.config.leak_rate * 1000.0) as u64;

        if leak_amount_milli > 0 {
            let current_milli = self.level.load(Ordering::Acquire);
            let new_level_milli = current_milli.saturating_sub(leak_amount_milli);

            self.level.store(new_level_milli, Ordering::Release);

            // Update last_leak time
            if let Ok(mut guard) = self.last_leak.write() {
                *guard = now;
            }

            debug!(
                "Leaked {:.2} requests (level now {:.2})",
                leak_amount_milli as f64 / 1000.0,
                new_level_milli as f64 / 1000.0
            );
        }
    }

    /// Try to add a request to the bucket
    ///
    /// Returns `true` if the request was accepted, `false` if bucket is full.
    pub fn try_acquire(&self) -> bool {
        self.leak();

        let capacity_milli = self.config.capacity * 1000;
        let current_milli = self.level.load(Ordering::Acquire);

        if current_milli >= capacity_milli {
            debug!("Rate limit: bucket full ({} >= {})", current_milli, capacity_milli);
            return false;
        }

        // Add 1 request (1000 millirequests)
        let new_level_milli = current_milli + 1000;
        self.level.store(new_level_milli, Ordering::Release);

        debug!(
            "Request accepted (level {:.2}/{:.2})",
            new_level_milli as f64 / 1000.0,
            capacity_milli as f64 / 1000.0
        );
        true
    }

    /// Get the current level of the bucket (as a fraction of capacity)
    pub fn current_level(&self) -> f64 {
        self.leak();
        let level_milli = self.level.load(Ordering::Acquire);
        level_milli as f64 / 1000.0
    }

    /// Reset the limiter to empty
    pub fn reset(&self) {
        self.level.store(0, Ordering::Release);
        if let Ok(mut guard) = self.last_leak.write() {
            *guard = self.clock.now();
        }
    }
}

impl LeakyBucket<SystemClock> {
    /// Create a new leaky bucket with system clock
    pub fn new(capacity: u64, leak_rate: f64) -> Result<Self, String> {
        Self::with_clock(capacity, leak_rate, SystemClock)
    }
}

impl<C: Clock> Clone for LeakyBucket<C> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            level: Arc::clone(&self.level),
            last_leak: Arc::clone(&self.last_leak),
            clock: Arc::clone(&self.clock),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::MockClock;
    use super::*;

    #[test]
    fn test_token_bucket_basic() {
        let bucket = TokenBucket::new(10, 5, Duration::from_secs(1)).unwrap();

        assert!(bucket.try_acquire(5));
        assert_eq!(bucket.available_tokens(), 5);

        assert!(bucket.try_acquire(5));
        assert_eq!(bucket.available_tokens(), 0);

        assert!(!bucket.try_acquire(1));
    }

    #[test]
    fn test_token_bucket_refill() {
        let clock = MockClock::new();
        let bucket =
            TokenBucket::with_clock(10, 5, Duration::from_millis(100), clock.clone()).unwrap();

        assert!(bucket.try_acquire(10));
        assert_eq!(bucket.available_tokens(), 0);

        // Advance time and refill
        clock.advance_millis(100);
        assert_eq!(bucket.available_tokens(), 5);

        clock.advance_millis(100);
        assert_eq!(bucket.available_tokens(), 10); // Capped at capacity
    }

    #[test]
    fn test_leaky_bucket_basic() {
        let bucket = LeakyBucket::new(10, 5.0).unwrap();

        for _ in 0..10 {
            assert!(bucket.try_acquire());
        }

        // Bucket is full
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn test_leaky_bucket_leak() {
        let clock = MockClock::new();
        let bucket = LeakyBucket::with_clock(10, 5.0, clock.clone()).unwrap();

        // Fill bucket
        for _ in 0..10 {
            assert!(bucket.try_acquire());
        }
        assert!(!bucket.try_acquire());

        // Wait 1 second - should leak 5 requests
        clock.advance(Duration::from_secs(1));
        assert_eq!(bucket.current_level(), 5.0);

        // Should be able to add more
        assert!(bucket.try_acquire());
    }

    #[test]
    fn test_token_bucket_config_validation() {
        assert!(TokenBucketConfig::builder().capacity(0).build().is_err());
        assert!(TokenBucketConfig::builder().refill_amount(0).build().is_err());
        assert!(TokenBucketConfig::builder().refill_interval(Duration::ZERO).build().is_err());
    }

    #[test]
    fn test_leaky_bucket_config_validation() {
        assert!(LeakyBucketConfig::builder().capacity(0).build().is_err());
        assert!(LeakyBucketConfig::builder().leak_rate(0.0).build().is_err());
        assert!(LeakyBucketConfig::builder().leak_rate(-1.0).build().is_err());
    }
}
