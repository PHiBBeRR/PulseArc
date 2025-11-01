//! Time abstraction for testability
//!
//! Provides a trait-based approach to time operations that allows for
//! deterministic testing without relying on actual time passage.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//!
//! use pulsearc_common::testing::{Clock, MockClock, SystemClock};
//!
//! // Use system clock in production
//! let clock = SystemClock;
//! let now = clock.now();
//!
//! // Use mock clock in tests
//! let mock = MockClock::new();
//! let start = mock.now();
//! mock.advance(Duration::from_secs(5));
//! let end = mock.now();
//! assert_eq!(end.duration_since(start), Duration::from_secs(5));
//! ```

// Allow missing panics docs for time utilities - the unwrap_or_default usage is intentional
// and handles edge cases gracefully
#![allow(clippy::missing_panics_doc)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Trait for time operations to enable testing
///
/// This trait provides an abstraction over time operations, allowing code
/// to work with either real system time or mocked time for testing.
pub trait Clock: Send + Sync {
    /// Get current instant (monotonic time)
    ///
    /// Returns a monotonic timestamp suitable for measuring durations.
    fn now(&self) -> Instant;

    /// Get current system time (wall clock)
    ///
    /// Returns the current wall clock time.
    fn system_time(&self) -> SystemTime;

    /// Get milliseconds since UNIX epoch
    ///
    /// Convenience method for getting the current time as milliseconds
    /// since the UNIX epoch (January 1, 1970).
    fn millis_since_epoch(&self) -> u64 {
        self.system_time().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }
}

/// Real system clock implementation
///
/// This implementation uses the actual system clock for time operations.
/// Use this in production code.
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::{Clock, SystemClock};
///
/// let clock = SystemClock;
/// let now = clock.now();
/// println!("Current time: {:?}", now);
/// ```
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

/// Mock clock for deterministic testing
///
/// This implementation allows you to control time in tests, making them
/// deterministic and fast. You can advance time manually without actually
/// waiting.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use pulsearc_common::testing::{Clock, MockClock};
///
/// let clock = MockClock::new();
/// let start = clock.now();
///
/// // Simulate 5 seconds passing
/// clock.advance(Duration::from_secs(5));
///
/// let end = clock.now();
/// assert_eq!(end.duration_since(start), Duration::from_secs(5));
/// ```
#[derive(Debug, Clone)]
pub struct MockClock {
    start: Instant,
    elapsed: Arc<Mutex<Duration>>,
    base_system_time: SystemTime,
}

impl MockClock {
    /// Create a new mock clock
    ///
    /// The clock starts at the current real time but can be advanced
    /// manually without real time passing.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            elapsed: Arc::new(Mutex::new(Duration::ZERO)),
            base_system_time: SystemTime::now(),
        }
    }

    /// Advance the mock clock by a duration
    ///
    /// This simulates time passing without actually waiting.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use pulsearc_common::testing::MockClock;
    ///
    /// let clock = MockClock::new();
    /// clock.advance(Duration::from_secs(10));
    /// assert_eq!(clock.elapsed(), Duration::from_secs(10));
    /// ```
    pub fn advance(&self, duration: Duration) {
        // Test utility: panic on poisoned mutex to fail tests early
        let mut elapsed = self.elapsed.lock().expect("mutex poisoned");
        *elapsed += duration;
    }

    /// Set the mock clock to a specific elapsed time
    ///
    /// This sets the clock to an absolute elapsed time, replacing
    /// any previous elapsed time.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use pulsearc_common::testing::MockClock;
    ///
    /// let clock = MockClock::new();
    /// clock.set_elapsed(Duration::from_secs(100));
    /// assert_eq!(clock.elapsed(), Duration::from_secs(100));
    /// ```
    pub fn set_elapsed(&self, duration: Duration) {
        // Test utility: panic on poisoned mutex to fail tests early
        let mut elapsed = self.elapsed.lock().expect("mutex poisoned");
        *elapsed = duration;
    }

    /// Get the current elapsed time
    ///
    /// Returns how much time has been simulated since the clock was created.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        // Test utility: panic on poisoned mutex to fail tests early
        *self.elapsed.lock().expect("mutex poisoned")
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        // Test utility: panic on poisoned mutex to fail tests early
        self.start + *self.elapsed.lock().expect("mutex poisoned")
    }

    fn system_time(&self) -> SystemTime {
        // Test utility: panic on poisoned mutex to fail tests early
        self.base_system_time + *self.elapsed.lock().expect("mutex poisoned")
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::time.
    use super::*;

    /// Validates the system clock scenario.
    ///
    /// Assertions:
    /// - Ensures `now2 >= now1` evaluates to true.
    #[test]
    fn test_system_clock() {
        let clock = SystemClock;
        let now1 = clock.now();
        let now2 = clock.now();

        assert!(now2 >= now1);
    }

    /// Validates the system clock millis scenario.
    ///
    /// Assertions:
    /// - Ensures `millis > 0` evaluates to true.
    #[test]
    fn test_system_clock_millis() {
        let clock = SystemClock;
        let millis = clock.millis_since_epoch();
        assert!(millis > 0);
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

        assert_eq!(after.duration_since(start), Duration::from_secs(5));
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
    /// - Confirms `millis.saturating_sub(before)` equals `5000`.
    #[test]
    fn test_mock_clock_millis_since_epoch() {
        let clock = MockClock::new();
        let before = clock.millis_since_epoch();
        clock.set_elapsed(Duration::from_millis(5000));

        let millis = clock.millis_since_epoch();
        assert_eq!(millis.saturating_sub(before), 5000);
    }

    /// Validates `MockClock::new` behavior for the mock clock clone scenario.
    ///
    /// Assertions:
    /// - Confirms `clock2.elapsed()` equals `Duration::from_secs(10)`.
    /// - Confirms `clock2.elapsed()` equals `Duration::from_secs(15)`.
    #[test]
    fn test_mock_clock_clone() {
        let clock1 = MockClock::new();
        clock1.advance(Duration::from_secs(10));

        let clock2 = clock1.clone();
        assert_eq!(clock2.elapsed(), Duration::from_secs(10));

        // Cloned clocks share the same elapsed time
        clock1.advance(Duration::from_secs(5));
        assert_eq!(clock2.elapsed(), Duration::from_secs(15));
    }

    /// Validates `MockClock::new` behavior for the mock clock multiple advances
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `clock.elapsed()` equals `Duration::from_secs(6)`.
    #[test]
    fn test_mock_clock_multiple_advances() {
        let clock = MockClock::new();

        clock.advance(Duration::from_secs(1));
        clock.advance(Duration::from_secs(2));
        clock.advance(Duration::from_secs(3));

        assert_eq!(clock.elapsed(), Duration::from_secs(6));
    }
}
