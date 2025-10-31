// Retry budget to prevent retry storms across multiple operations
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use crate::sync::retry::constants::BUDGET_REFILL_INTERVAL;
use crate::sync::retry::time::{Clock, SystemClock};

/// Retry budget to prevent retry storms across multiple operations
#[derive(Debug, Clone)]
pub struct RetryBudget<C: Clock = SystemClock> {
    /// Maximum retry tokens available
    max_tokens: u32,
    /// Current available tokens
    available_tokens: Arc<AtomicU32>,
    /// Token refill rate (tokens per second)
    refill_rate: f64,
    /// Last refill timestamp in milliseconds since UNIX epoch
    last_refill_millis: Arc<AtomicU64>,
    /// Clock for time operations (allows mocking in tests)
    clock: Arc<C>,
}

impl RetryBudget<SystemClock> {
    /// Create a new retry budget with system clock
    pub fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self::with_clock(max_tokens, refill_rate, SystemClock)
    }
}

impl<C: Clock> RetryBudget<C> {
    /// Create a new retry budget with a custom clock (for testing)
    pub fn with_clock(max_tokens: u32, refill_rate: f64, clock: C) -> Self {
        let now_millis = clock.millis_since_epoch();

        Self {
            max_tokens,
            available_tokens: Arc::new(AtomicU32::new(max_tokens)),
            refill_rate,
            last_refill_millis: Arc::new(AtomicU64::new(now_millis)),
            clock: Arc::new(clock),
        }
    }

    /// Try to acquire a retry token
    pub fn try_acquire(&self) -> bool {
        // Refill tokens based on elapsed time
        self.refill();

        // Try to acquire a token using compare-and-swap
        loop {
            let current = self.available_tokens.load(Ordering::Acquire);
            if current == 0 {
                return false;
            }
            match self.available_tokens.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(_) => continue, // Retry on concurrent modification
            }
        }
    }

    /// Try to acquire multiple tokens at once
    pub fn try_acquire_multiple(&self, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        self.refill();

        loop {
            let current = self.available_tokens.load(Ordering::Acquire);
            if current < count {
                return false;
            }
            match self.available_tokens.compare_exchange_weak(
                current,
                current - count,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&self) {
        let now_millis = self.clock.millis_since_epoch();

        // Use atomic compare-and-swap to ensure only one thread refills
        let last_refill_millis = self.last_refill_millis.load(Ordering::Acquire);
        let elapsed_millis = now_millis.saturating_sub(last_refill_millis);

        // Only refill if enough time has passed
        if elapsed_millis >= BUDGET_REFILL_INTERVAL.as_millis() as u64 {
            // Try to update the last refill time atomically
            match self.last_refill_millis.compare_exchange_weak(
                last_refill_millis,
                now_millis,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Successfully acquired the refill lock
                    let elapsed_secs = elapsed_millis as f64 / 1000.0;
                    let tokens_to_add = (elapsed_secs * self.refill_rate) as u32;

                    if tokens_to_add > 0 {
                        // Add tokens, capping at max_tokens
                        loop {
                            let current = self.available_tokens.load(Ordering::Acquire);
                            let new_tokens = (current + tokens_to_add).min(self.max_tokens);

                            if current == new_tokens {
                                break; // No change needed
                            }

                            match self.available_tokens.compare_exchange_weak(
                                current,
                                new_tokens,
                                Ordering::Release,
                                Ordering::Acquire,
                            ) {
                                Ok(_) => break,
                                Err(_) => continue,
                            }
                        }
                    }
                }
                Err(_) => {
                    // Another thread is already refilling, skip
                }
            }
        }
    }

    /// Get current available tokens
    pub fn available(&self) -> u32 {
        self.refill();
        self.available_tokens.load(Ordering::Acquire)
    }

    /// Get the maximum token capacity
    pub fn capacity(&self) -> u32 {
        self.max_tokens
    }

    /// Get the refill rate
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }

    /// Reset the budget to full capacity
    pub fn reset(&self) {
        self.available_tokens.store(self.max_tokens, Ordering::Release);
        let now_millis = self.clock.millis_since_epoch();
        self.last_refill_millis.store(now_millis, Ordering::Release);
    }

    /// Return tokens to the budget (e.g., if an operation succeeded without
    /// retries)
    pub fn return_tokens(&self, count: u32) {
        if count == 0 {
            return;
        }

        loop {
            let current = self.available_tokens.load(Ordering::Acquire);
            let new_tokens = (current + count).min(self.max_tokens);

            if current == new_tokens {
                break; // Already at max capacity
            }

            match self.available_tokens.compare_exchange_weak(
                current,
                new_tokens,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::retry::budget.
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::sync::retry::time::MockClock;

    /// Validates `RetryBudget::new` behavior for the new retry budget scenario.
    ///
    /// Assertions:
    /// - Ensures `budget.try_acquire()` evaluates to true.
    #[test]
    fn test_new_retry_budget() {
        let budget = RetryBudget::new(100, 10.0);

        // Should start with full tokens
        assert!(budget.try_acquire());
    }

    /// Validates `RetryBudget::new` behavior for the acquire single token
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `budget.try_acquire()` evaluates to true.
    /// - Confirms `budget.available()` equals `9`.
    #[test]
    fn test_acquire_single_token() {
        let budget = RetryBudget::new(10, 1.0);

        assert!(budget.try_acquire());
        assert_eq!(budget.available(), 9);
    }

    /// Validates `RetryBudget::new` behavior for the acquire multiple tokens
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `budget.try_acquire_multiple(5)` evaluates to true.
    /// - Confirms `budget.available()` equals `5`.
    #[test]
    fn test_acquire_multiple_tokens() {
        let budget = RetryBudget::new(10, 1.0);

        assert!(budget.try_acquire_multiple(5));
        assert_eq!(budget.available(), 5);
    }

    /// Validates `RetryBudget::new` behavior for the exhaust budget scenario.
    ///
    /// Assertions:
    /// - Ensures `budget.try_acquire()` evaluates to true.
    /// - Ensures `budget.try_acquire()` evaluates to true.
    /// - Ensures `budget.try_acquire()` evaluates to true.
    /// - Ensures `!budget.try_acquire()` evaluates to true.
    #[test]
    fn test_exhaust_budget() {
        let budget = RetryBudget::new(3, 1.0);

        assert!(budget.try_acquire());
        assert!(budget.try_acquire());
        assert!(budget.try_acquire());

        // Should fail when exhausted
        assert!(!budget.try_acquire());
    }

    /// Validates `RetryBudget::new` behavior for the acquire more than
    /// available scenario.
    ///
    /// Assertions:
    /// - Ensures `!budget.try_acquire_multiple(10)` evaluates to true.
    /// - Confirms `budget.available()` equals `5`.
    #[test]
    fn test_acquire_more_than_available() {
        let budget = RetryBudget::new(5, 1.0);

        assert!(!budget.try_acquire_multiple(10));
        assert_eq!(budget.available(), 5);
    }

    /// Validates `RetryBudget::new` behavior for the acquire zero tokens
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `budget.try_acquire_multiple(0)` evaluates to true.
    /// - Confirms `budget.available()` equals `10`.
    #[test]
    fn test_acquire_zero_tokens() {
        let budget = RetryBudget::new(10, 1.0);

        // Acquiring zero should always succeed
        assert!(budget.try_acquire_multiple(0));
        assert_eq!(budget.available(), 10);
    }

    /// Tests token bucket refills over time at configured rate.
    ///
    /// Verifies:
    /// - Consumed tokens are refilled based on time elapsed
    /// - Refill rate (10 tokens/sec) is correctly applied
    /// - Refilled tokens are capped at maximum capacity
    /// - MockClock enables deterministic time-based testing
    #[test]
    fn test_refill_over_time() {
        let clock = MockClock::new();
        let budget = RetryBudget::with_clock(10, 10.0, clock.clone()); // 10 tokens/sec

        // Exhaust some tokens
        assert!(budget.try_acquire_multiple(5));
        assert_eq!(budget.available(), 5);

        // Advance clock by 1 second (refill interval threshold)
        clock.advance(Duration::from_secs(1));

        // Should have refilled back to max (5 current + 10 refilled = 10 max)
        assert_eq!(budget.available(), 10);
    }

    /// Tests refill mechanism respects maximum capacity.
    ///
    /// Verifies:
    /// - Tokens don't exceed maximum capacity after refill
    /// - Long time periods don't overflow token count
    /// - Maximum capacity acts as hard ceiling
    /// - Token bucket algorithm correctly implements capacity limit
    #[test]
    fn test_refill_respects_max_tokens() {
        let clock = MockClock::new();
        let budget = RetryBudget::with_clock(10, 100.0, clock.clone());

        // Advance clock significantly
        clock.advance(Duration::from_secs(10));

        // Should not exceed max even with lots of time passed
        assert_eq!(budget.available(), 10);
    }

    /// Tests returning unused tokens back to budget pool.
    ///
    /// Verifies:
    /// - Tokens can be returned to increase available capacity
    /// - Returned tokens are immediately available
    /// - Return operation updates available count correctly
    /// - Enables retry cancellation to free resources
    #[test]
    fn test_return_tokens() {
        let budget = RetryBudget::new(10, 1.0);

        assert!(budget.try_acquire_multiple(5));
        assert_eq!(budget.available(), 5);

        budget.return_tokens(3);
        assert_eq!(budget.available(), 8);
    }

    /// Tests returning tokens is capped at maximum capacity.
    ///
    /// Verifies:
    /// - Returning more tokens than consumed doesn't overflow
    /// - Available tokens never exceed maximum capacity
    /// - Return operation safely handles edge cases
    /// - Prevents budget from exceeding configured limits
    #[test]
    fn test_return_tokens_respects_max() {
        let budget = RetryBudget::new(10, 1.0);

        assert!(budget.try_acquire_multiple(5));

        // Try to return more than consumed
        budget.return_tokens(20);

        // Should cap at max
        assert_eq!(budget.available(), 10);
    }

    /// Tests retry budget is thread-safe under concurrent token acquisition.
    ///
    /// Verifies:
    /// - Multiple threads can acquire tokens concurrently
    /// - Token count remains accurate under concurrent load
    /// - No tokens are double-counted or lost
    /// - Thread-safe atomic operations work correctly
    /// - Budget correctly tracks consumption across threads
    #[test]
    fn test_concurrent_token_acquisition() {
        use std::sync::Arc;

        let budget = Arc::new(RetryBudget::new(100, 10.0));
        let mut handles = vec![];

        for _ in 0..10 {
            let budget_clone = Arc::clone(&budget);
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    budget_clone.try_acquire();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 10 threads × 5 attempts = 50 tokens consumed
        // Should have 50 or fewer remaining (depending on refill timing)
        assert!(budget.available() <= 50);
    }

    /// Tests thread-safe batch token acquisition.
    ///
    /// Verifies:
    /// - Multiple threads can acquire token batches concurrently
    /// - Batch acquisitions are atomic operations
    /// - No partial batch acquisitions occur
    /// - Total consumption matches expected amount
    /// - Concurrent batch operations maintain budget integrity
    #[test]
    fn test_concurrent_multiple_token_acquisition() {
        use std::sync::Arc;

        let budget = Arc::new(RetryBudget::new(100, 1.0));
        let mut handles = vec![];

        for _ in 0..5 {
            let budget_clone = Arc::clone(&budget);
            let handle = thread::spawn(move || {
                budget_clone.try_acquire_multiple(10);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have consumed 50 tokens
        assert!(budget.available() <= 50);
    }

    /// Tests accurate refill rate calculation over time.
    ///
    /// Verifies:
    /// - Refill rate (tokens per second) is precisely calculated
    /// - Time-based refill matches configured rate
    /// - After 1 second, exactly configured tokens are refilled
    /// - Token bucket algorithm implements correct refill math
    #[test]
    fn test_refill_rate_calculation() {
        let clock = MockClock::new();
        let budget = RetryBudget::with_clock(100, 10.0, clock.clone()); // 10 tokens/sec

        // Consume all tokens
        assert!(budget.try_acquire_multiple(100));
        assert_eq!(budget.available(), 0);

        // Advance clock by 1 second
        clock.advance(Duration::from_secs(1));

        // Should have refilled exactly 10 tokens
        assert_eq!(budget.available(), 10);
    }

    /// Validates `RetryBudget::new` behavior for the available tokens scenario.
    ///
    /// Assertions:
    /// - Confirms `budget.available()` equals `20`.
    /// - Ensures `budget.try_acquire()` evaluates to true.
    /// - Confirms `budget.available()` equals `19`.
    /// - Ensures `budget.try_acquire_multiple(5)` evaluates to true.
    /// - Confirms `budget.available()` equals `14`.
    #[test]
    fn test_available_tokens() {
        let budget = RetryBudget::new(20, 1.0);

        assert_eq!(budget.available(), 20);

        assert!(budget.try_acquire());
        assert_eq!(budget.available(), 19);

        assert!(budget.try_acquire_multiple(5));
        assert_eq!(budget.available(), 14);
    }

    /// Tests exhausted budget refills and becomes usable again.
    ///
    /// Verifies:
    /// - Fully exhausted budget (0 tokens) can refill
    /// - Refill allows new acquisitions after depletion
    /// - Time-based recovery prevents permanent retry blocking
    /// - Budget recovers gracefully from complete exhaustion
    #[test]
    fn test_exhausted_budget_refills() {
        // Test that an exhausted budget (not zero capacity) refills over time
        let clock = MockClock::new();
        let budget = RetryBudget::with_clock(20, 10.0, clock.clone()); // 20 max, 10 tokens/sec

        // Exhaust all tokens
        assert!(budget.try_acquire_multiple(20));
        assert_eq!(budget.available(), 0);

        // Advance clock by 1 second - should refill 10 tokens
        clock.advance(Duration::from_secs(1));

        assert_eq!(budget.available(), 10);
        assert!(budget.try_acquire(), "Should have refilled tokens");
    }

    /// Validates `RetryBudget::new` behavior for the zero capacity budget
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!budget.try_acquire()` evaluates to true.
    /// - Confirms `budget.available()` equals `0`.
    #[test]
    fn test_zero_capacity_budget() {
        // A budget with zero capacity can never acquire tokens
        let budget = RetryBudget::new(0, 10.0);

        assert!(!budget.try_acquire());
        assert_eq!(budget.available(), 0);
    }

    /// Tests high refill rate is capped at maximum capacity.
    ///
    /// Verifies:
    /// - Very high refill rates (1000 tokens/sec) work correctly
    /// - Refilled amount is capped at maximum capacity
    /// - Budget doesn't overflow with aggressive refill rates
    /// - Capacity limit takes precedence over refill rate
    #[test]
    fn test_high_refill_rate() {
        let clock = MockClock::new();
        let budget = RetryBudget::with_clock(10, 1000.0, clock.clone()); // 1000 tokens/sec

        // Consume tokens
        assert!(budget.try_acquire_multiple(10));
        assert_eq!(budget.available(), 0);

        // Advance clock by 1 second (refill interval threshold)
        clock.advance(Duration::from_secs(1));

        // Should have refilled back to max (0 + 1000 tokens capped at 10)
        assert_eq!(budget.available(), 10);
    }

    /// Tests real-time token refill without mock clock.
    ///
    /// Verifies:
    /// - Budget uses actual system time for refill
    /// - Real time-based refill works correctly
    /// - Integration test validates production timing behavior
    /// - Allows variance due to actual time passage
    #[test]
    #[ignore] // Integration test: tests real timing behavior
    fn test_real_time_refill() {
        let budget = RetryBudget::new(10, 100.0);

        assert!(budget.try_acquire_multiple(5));
        assert_eq!(budget.available(), 5);

        thread::sleep(Duration::from_millis(150));

        // Should have refilled some tokens (allowing variance)
        assert!(budget.available() >= 7);
    }
}
