//! Async testing utilities
//!
//! Provides async-specific test helpers and assertions.

// Allow missing error/panic docs for test utilities - they are designed to be self-explanatory
// and are used in test contexts where comprehensive documentation is less critical
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::future::Future;
use std::time::Duration;

/// Assert that a condition eventually becomes true within a timeout (async
/// version)
///
/// This is the async variant of `assert_eventually!` that works with async
/// predicates.
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
/// #[tokio::test(flavor = "multi_thread")]
/// async fn test_eventually() {
///     let flag = Arc::new(AtomicBool::new(false));
///     let flag_clone = flag.clone();
///
///     tokio::spawn(async move {
///         tokio::time::sleep(Duration::from_millis(100)).await;
///         flag_clone.store(true, Ordering::SeqCst);
///     });
///
///     pulsearc_common::assert_eventually_async!(Duration::from_secs(1), async {
///         flag.load(Ordering::SeqCst)
///     });
/// }
/// # }
/// ```
#[macro_export]
macro_rules! assert_eventually_async {
    ($timeout:expr, $fut:expr) => {{
        let timeout_duration = $timeout;
        let result = tokio::time::timeout(timeout_duration, async {
            loop {
                if $fut.await {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await;

        assert!(result.is_ok(), "Condition did not become true within {:?}", timeout_duration);
    }};
}

/// Wait for a future to complete with a timeout, returning a Result
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::testing::async_utils::timeout_ok;
///
/// #[tokio::test]
/// async fn test_timeout() {
///     let result = timeout_ok(Duration::from_millis(100), async {
///         tokio::time::sleep(Duration::from_millis(50)).await;
///         42
///     })
///     .await;
///
///     assert!(result.is_ok());
///     assert_eq!(result.unwrap(), 42);
/// }
/// # }
/// ```
pub async fn timeout_ok<F, T>(duration: Duration, fut: F) -> Result<T, tokio::time::error::Elapsed>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(duration, fut).await
}

/// Retry an async operation with exponential backoff
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::testing::async_utils::retry_async;
///
/// #[tokio::test]
/// async fn test_retry() {
///     let mut attempts = 0;
///     let result = retry_async(3, Duration::from_millis(10), || async {
///         attempts += 1;
///         if attempts < 3 {
///             Err("Not yet")
///         } else {
///             Ok(42)
///         }
///     })
///     .await;
///
///     assert!(result.is_ok());
///     assert_eq!(result.unwrap(), 42);
/// }
/// # }
/// ```
pub async fn retry_async<F, Fut, T, E>(
    max_attempts: usize,
    initial_delay: Duration,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempt = 0;
    let mut delay = initial_delay;

    loop {
        attempt += 1;
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) if attempt >= max_attempts => return Err(e),
            Err(_) => {
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }
}

/// Poll an async condition until it returns true or times out
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
/// use pulsearc_common::testing::async_utils::poll_until;
///
/// #[tokio::test]
/// async fn test_poll() {
///     let flag = Arc::new(AtomicBool::new(false));
///     let flag_clone = flag.clone();
///
///     tokio::spawn(async move {
///         tokio::time::sleep(Duration::from_millis(50)).await;
///         flag_clone.store(true, Ordering::SeqCst);
///     });
///
///     let result = poll_until(Duration::from_secs(1), Duration::from_millis(10), || async {
///         flag.load(Ordering::SeqCst)
///     })
///     .await;
///
///     assert!(result);
/// }
/// # }
/// ```
pub async fn poll_until<F, Fut>(timeout: Duration, interval: Duration, mut condition: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(interval).await;
    }

    false
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::async_utils.
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    /// Validates `Duration::from_millis` behavior for the timeout ok succeeds
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `42`.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_timeout_ok_succeeds() {
        let result = timeout_ok(Duration::from_millis(100), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    /// Validates `Duration::from_millis` behavior for the timeout ok times out
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_timeout_ok_times_out() {
        let result = timeout_ok(Duration::from_millis(10), async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            42
        })
        .await;

        assert!(result.is_err());
    }

    /// Validates `Duration::from_millis` behavior for the retry async succeeds
    /// immediately scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `42`.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_retry_async_succeeds_immediately() {
        let result =
            retry_async(3, Duration::from_millis(10), || async { Ok::<_, String>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    /// Validates `Arc::new` behavior for the retry async succeeds after retries
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `42`.
    /// - Confirms `attempts.load(Ordering::SeqCst)` equals `3`.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_retry_async_succeeds_after_retries() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = retry_async(3, Duration::from_millis(10), move || {
            let attempts = attempts_clone.clone();
            async move {
                let count = attempts.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err("Not yet")
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    /// Validates `Duration::from_millis` behavior for the retry async fails
    /// after max attempts scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_retry_async_fails_after_max_attempts() {
        let result =
            retry_async(3, Duration::from_millis(10), || async { Err::<i32, _>("Always fails") })
                .await;

        assert!(result.is_err());
    }

    /// Validates `Arc::new` behavior for the poll until succeeds scenario.
    ///
    /// Assertions:
    /// - Ensures `result` evaluates to true.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_poll_until_succeeds() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag_clone.store(true, Ordering::SeqCst);
        });

        let result = poll_until(Duration::from_secs(1), Duration::from_millis(10), || {
            let flag = flag.clone();
            async move { flag.load(Ordering::SeqCst) }
        })
        .await;

        assert!(result);
    }

    /// Validates `Duration::from_millis` behavior for the poll until times out
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!result` evaluates to true.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_poll_until_times_out() {
        let result =
            poll_until(Duration::from_millis(50), Duration::from_millis(10), || async { false })
                .await;

        assert!(!result);
    }
}
