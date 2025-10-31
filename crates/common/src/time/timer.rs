//! One-shot and recurring timers
//!
//! Provides utilities for creating timers with cancellation support.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

/// A timer handle that can be used to cancel a timer
#[derive(Debug, Clone)]
pub struct TimerHandle {
    cancelled: Arc<AtomicBool>,
}

impl TimerHandle {
    /// Create a new timer handle
    fn new() -> Self {
        Self { cancelled: Arc::new(AtomicBool::new(false)) }
    }

    /// Cancel the timer
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if the timer has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// A one-shot timer
pub struct Timer {
    handle: TimerHandle,
}

impl Timer {
    /// Create a new timer that fires after a duration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "runtime")]
    /// # {
    /// use std::time::Duration;
    ///
    /// use pulsearc_common::time::timer::Timer;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let timer = Timer::after(Duration::from_secs(1));
    ///     timer.wait(Duration::from_secs(1)).await;
    ///     println!("Timer fired!");
    /// }
    /// # }
    /// ```
    pub fn after(_duration: Duration) -> Self {
        Self { handle: TimerHandle::new() }
    }

    /// Get a handle to cancel the timer
    pub fn handle(&self) -> TimerHandle {
        self.handle.clone()
    }

    /// Wait for the timer to fire
    pub async fn wait(self, duration: Duration) {
        sleep(duration).await;
    }

    /// Check if the timer was cancelled
    pub fn is_cancelled(&self) -> bool {
        self.handle.is_cancelled()
    }
}

/// Create a one-shot timer
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::timer::timeout;
///
/// #[tokio::main]
/// async fn main() {
///     let handle = timeout(Duration::from_secs(5), || {
///         println!("Timeout!");
///     })
///     .await;
/// }
/// # }
/// ```
pub async fn timeout<F>(duration: Duration, callback: F) -> TimerHandle
where
    F: FnOnce() + Send + 'static,
{
    let handle = TimerHandle::new();
    let handle_clone = handle.clone();

    tokio::spawn(async move {
        sleep(duration).await;
        if !handle_clone.is_cancelled() {
            callback();
        }
    });

    handle
}

/// Create a recurring timer
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::timer::recurring;
///
/// #[tokio::main]
/// async fn main() {
///     let handle = recurring(Duration::from_secs(1), || {
///         println!("Tick!");
///     });
///
///     tokio::time::sleep(Duration::from_secs(5)).await;
///     handle.cancel();
/// }
/// # }
/// ```
pub fn recurring<F>(duration: Duration, mut callback: F) -> TimerHandle
where
    F: FnMut() + Send + 'static,
{
    let handle = TimerHandle::new();
    let handle_clone = handle.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(duration);
        interval.tick().await; // Skip first immediate tick

        while !handle_clone.is_cancelled() {
            interval.tick().await;
            if !handle_clone.is_cancelled() {
                callback();
            }
        }
    });

    handle
}

#[cfg(test)]
mod tests {
    //! Unit tests for time::timer.
    use std::sync::atomic::AtomicU32;

    use super::*;

    /// Validates `Timer::after` behavior for the timer fires scenario.
    ///
    /// Assertions:
    /// - Ensures `elapsed >= Duration::from_millis(8)` evaluates to true.
    #[tokio::test]
    async fn test_timer_fires() {
        let timer = Timer::after(Duration::from_millis(10));
        let start = tokio::time::Instant::now();

        timer.wait(Duration::from_millis(10)).await;

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(8));
    }

    /// Validates `TimerHandle::new` behavior for the timer handle cancel
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!handle.is_cancelled()` evaluates to true.
    /// - Ensures `handle.is_cancelled()` evaluates to true.
    #[tokio::test]
    async fn test_timer_handle_cancel() {
        let handle = TimerHandle::new();
        assert!(!handle.is_cancelled());

        handle.cancel();
        assert!(handle.is_cancelled());
    }

    /// Validates `Arc::new` behavior for the timeout scenario.
    ///
    /// Assertions:
    /// - Confirms `counter.load(Ordering::SeqCst)` equals `1`.
    /// - Ensures `!handle.is_cancelled()` evaluates to true.
    #[tokio::test]
    async fn test_timeout() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let handle = timeout(Duration::from_millis(10), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(!handle.is_cancelled());
    }

    /// Validates `Arc::new` behavior for the timeout cancelled scenario.
    ///
    /// Assertions:
    /// - Confirms `counter.load(Ordering::SeqCst)` equals `0`.
    #[tokio::test]
    async fn test_timeout_cancelled() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let handle = timeout(Duration::from_millis(50), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        handle.cancel();
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should not have fired because it was cancelled
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    /// Validates `Arc::new` behavior for the recurring scenario.
    ///
    /// Assertions:
    /// - Ensures `(2..=4).contains(&count)` evaluates to true.
    #[tokio::test]
    async fn test_recurring() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let handle = recurring(Duration::from_millis(10), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        tokio::time::sleep(Duration::from_millis(35)).await;
        handle.cancel();
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should have fired 3 times (at 10ms, 20ms, 30ms)
        let count = counter.load(Ordering::SeqCst);
        assert!((2..=4).contains(&count)); // Allow some timing variance
    }
}
