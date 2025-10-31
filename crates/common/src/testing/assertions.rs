//! Custom assertions for testing
//!
//! Provides assertion macros and functions for common testing scenarios.

// Allow missing panics docs for test utilities - these assertions are designed to panic
// on failure which is their core purpose in test contexts
#![allow(clippy::missing_panics_doc)]

use std::fmt::Debug;
use std::time::Duration;

/// Assert that an error contains a specific substring
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
///
/// let result: Result<(), String> = Err("Connection timeout occurred".to_string());
/// pulsearc_common::assert_error_contains!(result, "timeout");
/// # }
/// ```
#[macro_export]
macro_rules! assert_error_contains {
    ($result:expr, $substring:expr) => {
        match &$result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(e) => {
                let error_msg = format!("{}", e);
                assert!(
                    error_msg.contains($substring),
                    "Error message '{}' does not contain '{}'",
                    error_msg,
                    $substring
                );
            }
        }
    };
}

/// Assert that an error is of a specific kind
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::io;
///
/// let result: Result<(), io::Error> =
///     Err(io::Error::new(io::ErrorKind::NotFound, "file not found"));
/// pulsearc_common::assert_error_kind!(result, io::ErrorKind::NotFound);
/// # }
/// ```
///
/// For error types that expose their "kind" via a different accessor, pass it
/// explicitly:
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// #[derive(Debug)]
/// struct CustomError {
///     kind: CustomKind,
/// }
///
/// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// enum CustomKind {
///     Foo,
///     Bar,
/// }
///
/// impl CustomError {
///     fn category(&self) -> CustomKind {
///         self.kind
///     }
/// }
///
/// let result: Result<(), CustomError> = Err(CustomError { kind: CustomKind::Foo });
/// pulsearc_common::assert_error_kind!(result, CustomKind::Foo, |err: &CustomError| err
///     .category());
/// # }
/// ```
#[macro_export]
macro_rules! assert_error_kind {
    ($result:expr, $expected_kind:expr $(,)?) => {{
        match &$result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(e) => {
                let actual_kind = e.kind();
                let expected_kind = $expected_kind;
                assert_eq!(
                    std::mem::discriminant(&actual_kind),
                    std::mem::discriminant(&expected_kind),
                    "Error kind mismatch: expected {:?}, got {:?}",
                    expected_kind,
                    actual_kind
                );
            }
        }
    }};
    ($result:expr, $expected_kind:expr, $kind_accessor:expr $(,)?) => {{
        match &$result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(e) => {
                let actual_kind = $kind_accessor(e);
                let expected_kind = $expected_kind;
                assert_eq!(
                    std::mem::discriminant(&actual_kind),
                    std::mem::discriminant(&expected_kind),
                    "Error kind mismatch: expected {:?}, got {:?}",
                    expected_kind,
                    actual_kind
                );
            }
        }
    }};
}

/// Assert that a condition eventually becomes true within a timeout
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// #[tokio::test]
/// async fn test_eventually() {
///     let flag = Arc::new(AtomicBool::new(false));
///     let flag_clone = flag.clone();
///
///     tokio::spawn(async move {
///         tokio::time::sleep(Duration::from_millis(100)).await;
///         flag_clone.store(true, Ordering::SeqCst);
///     });
///
///     pulsearc_common::assert_eventually!(Duration::from_secs(1), || flag.load(Ordering::SeqCst));
/// }
/// # }
/// ```
#[macro_export]
macro_rules! assert_eventually {
    ($timeout:expr, $condition:expr) => {{
        let start = std::time::Instant::now();
        let timeout = $timeout;
        let mut last_value = false;

        while start.elapsed() < timeout {
            last_value = $condition;
            if last_value {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(last_value, "Condition did not become true within {:?}", timeout);
    }};
}

/// Assert that a retry count matches expected value
///
/// Used with retry mechanisms to verify retry behavior
#[macro_export]
macro_rules! assert_retry_count {
    ($actual:expr, $expected:expr) => {
        assert_eq!($actual, $expected, "Expected {} retries but got {}", $expected, $actual);
    };
}

/// Assert that two values are approximately equal (for floats)
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::assertions::assert_approx_eq;
///
/// assert_approx_eq(3.14159, 3.14160, 0.001);
/// ```
pub fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff < epsilon,
        "Values not approximately equal: {} vs {} (diff: {})",
        actual,
        expected,
        diff
    );
}

/// Assert that a duration is within an acceptable range
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use pulsearc_common::testing::assertions::assert_duration_in_range;
///
/// let actual = Duration::from_millis(105);
/// assert_duration_in_range(actual, Duration::from_millis(100), Duration::from_millis(10));
/// ```
pub fn assert_duration_in_range(actual: Duration, expected: Duration, tolerance: Duration) {
    let min = expected.saturating_sub(tolerance);
    let max = expected + tolerance;

    assert!(
        actual >= min && actual <= max,
        "Duration {:?} not in range [{:?}, {:?}]",
        actual,
        min,
        max
    );
}

/// Assert that a collection contains all specified items
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::assertions::assert_contains_all;
///
/// let vec = vec![1, 2, 3, 4, 5];
/// assert_contains_all(&vec, &[2, 4]);
/// ```
pub fn assert_contains_all<T>(haystack: &[T], needles: &[T])
where
    T: PartialEq + Debug,
{
    for needle in needles {
        assert!(haystack.contains(needle), "Collection does not contain {:?}", needle);
    }
}

/// Assert that a collection is sorted
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::assertions::assert_sorted;
///
/// let vec = vec![1, 2, 3, 4, 5];
/// assert_sorted(&vec);
/// ```
pub fn assert_sorted<T>(items: &[T])
where
    T: Ord + Debug,
{
    for window in items.windows(2) {
        assert!(window[0] <= window[1], "Items not sorted: {:?} > {:?}", window[0], window[1]);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::assertions.
    use std::io;

    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum CustomKind {
        Foo,
        Bar,
    }

    #[derive(Debug)]
    struct CustomError {
        kind: CustomKind,
    }

    impl CustomError {
        fn new(kind: CustomKind) -> Self {
            Self { kind }
        }

        fn category(&self) -> CustomKind {
            self.kind
        }
    }

    /// Validates the assert error contains scenario.
    ///
    /// Assertions:
    /// - Checks `assert_error_contains!(result, "timeout")`.
    #[test]
    fn test_assert_error_contains() {
        let result: Result<(), String> = Err("Connection timeout".to_string());
        assert_error_contains!(result, "timeout");
    }

    /// Validates the assert error contains fails on ok scenario.
    ///
    /// Assertions:
    /// - Checks `assert_error_contains!(result, "timeout")`.
    #[test]
    #[should_panic(expected = "Expected error but got Ok")]
    fn test_assert_error_contains_fails_on_ok() {
        let result: Result<(), String> = Ok(());
        assert_error_contains!(result, "timeout");
    }

    /// Validates the assert approx eq scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_assert_approx_eq() {
        let base = std::f64::consts::SQRT_2;
        assert_approx_eq(base, base + 0.0005, 0.001);
    }

    /// Validates the assert approx eq fails scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    #[should_panic(expected = "Values not approximately equal")]
    fn test_assert_approx_eq_fails() {
        let base = std::f64::consts::SQRT_2;
        assert_approx_eq(base, base + 0.01, 0.001);
    }

    /// Validates `Error::new` behavior for the assert error kind io scenario.
    ///
    /// Assertions:
    /// - Checks `assert_error_kind!(result, io::ErrorKind::NotFound)`.
    #[test]
    fn test_assert_error_kind_io() {
        let result: Result<(), io::Error> =
            Err(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert_error_kind!(result, io::ErrorKind::NotFound);
    }

    /// Validates `Error::new` behavior for the assert error kind io fails
    /// scenario.
    ///
    /// Assertions:
    /// - Checks `assert_error_kind!(result, io::ErrorKind::NotFound)`.
    #[test]
    #[should_panic(expected = "Error kind mismatch")]
    fn test_assert_error_kind_io_fails() {
        let result: Result<(), io::Error> =
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "no access"));
        assert_error_kind!(result, io::ErrorKind::NotFound);
    }

    /// Validates `CustomError::new` behavior for the assert error kind with
    /// accessor scenario.
    ///
    /// Assertions:
    /// - Checks `assert_error_kind!(result, CustomKind::Foo, |err:
    ///   &CustomError| err.category())`.
    #[test]
    fn test_assert_error_kind_with_accessor() {
        let result: Result<(), CustomError> = Err(CustomError::new(CustomKind::Foo));
        assert_error_kind!(result, CustomKind::Foo, |err: &CustomError| err.category());
    }

    /// Validates `CustomError::new` behavior for the assert error kind with
    /// accessor fails scenario.
    ///
    /// Assertions:
    /// - Checks `assert_error_kind!(result, CustomKind::Foo, |err:
    ///   &CustomError| err.category())`.
    #[test]
    #[should_panic(expected = "Error kind mismatch")]
    fn test_assert_error_kind_with_accessor_fails() {
        let result: Result<(), CustomError> = Err(CustomError::new(CustomKind::Bar));
        assert_error_kind!(result, CustomKind::Foo, |err: &CustomError| err.category());
    }

    /// Validates `Duration::from_millis` behavior for the assert duration in
    /// range scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_assert_duration_in_range() {
        let actual = Duration::from_millis(105);
        assert_duration_in_range(actual, Duration::from_millis(100), Duration::from_millis(10));
    }

    /// Validates the assert contains all scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_assert_contains_all() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_contains_all(&vec, &[2, 4]);
    }

    /// Validates the assert sorted scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_assert_sorted() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_sorted(&vec);
    }

    /// Validates the assert sorted fails scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    #[should_panic(expected = "Items not sorted")]
    fn test_assert_sorted_fails() {
        let vec = vec![1, 3, 2, 4, 5];
        assert_sorted(&vec);
    }
}
