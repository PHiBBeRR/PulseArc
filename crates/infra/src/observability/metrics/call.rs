//! Call-related metrics for tracking API call patterns and timing
//!
//! This module tracks metrics related to API calls including total call counts,
//! time to first data (TTFD), and detailed timing statistics for P50/P95/P99
//! calculations.
//!
//! ## Design
//! - **VecDeque ring buffer** for O(1) eviction (not Vec with remove(0))
//! - **Poison-safe locking** with explicit match pattern (no .expect())
//! - **SeqCst ordering** for atomics used in derived metrics
//! - **MetricsResult returns** for future extensibility (currently always Ok)

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::observability::{MetricsError, MetricsResult};

/// Metrics for tracking API call patterns and timing
///
/// All record methods return `MetricsResult<()>` for future extensibility
/// (cardinality limits, quotas), but currently always succeed.
#[derive(Debug)]
pub struct CallMetrics {
    /// Total number of calls made
    pub total_calls: AtomicUsize,
    /// Time to first data in milliseconds
    pub first_call_time_ms: AtomicU64,
    /// Whether the first call has been made
    pub has_first_call: AtomicBool,
    /// Start time for calculating calls per minute
    pub start_time: Mutex<Option<Instant>>,
    /// Individual fetch times for percentile calculations (ring buffer, max
    /// 1000)
    pub fetch_times: Mutex<VecDeque<u64>>,
}

impl Default for CallMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl CallMetrics {
    /// Create new CallMetrics instance
    pub fn new() -> Self {
        Self {
            total_calls: AtomicUsize::new(0),
            first_call_time_ms: AtomicU64::new(0),
            has_first_call: AtomicBool::new(false),
            start_time: Mutex::new(Some(Instant::now())),
            fetch_times: Mutex::new(VecDeque::with_capacity(1000)),
        }
    }

    /// Record a call
    ///
    /// Currently always succeeds. Future versions may enforce quotas or rate
    /// limits.
    pub fn record_call(&self) -> MetricsResult<()> {
        // SeqCst for consistency with calls_per_minute calculation
        self.total_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Record time to first data (TTFD)
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn record_first_call_time(&self, ttfd_ms: u64) -> MetricsResult<()> {
        // Relaxed OK: independent writes, no derived metrics
        self.first_call_time_ms.store(ttfd_ms, Ordering::Relaxed);
        self.has_first_call.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Store a fetch time for percentile calculations
    ///
    /// Maintains ring buffer of last 1000 samples. Uses VecDeque for O(1)
    /// eviction.
    ///
    /// Currently always succeeds. Future versions may enforce cardinality
    /// limits.
    pub fn record_fetch_time(&self, duration: Duration) -> MetricsResult<()> {
        let ms = duration.as_millis() as u64;

        // Record TTFD on first call
        if !self.has_first_call.load(Ordering::Relaxed) {
            self.record_first_call_time(ms)?;
        }

        // Poison-safe locking: explicit match, no .expect()
        let mut times = match self.fetch_times.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "CallMetrics::fetch_times",
                    "Mutex poisoned during fetch_time recording, recovering data"
                );
                poison_err.into_inner()
            }
        };

        // Ring buffer: O(1) push_back + pop_front
        times.push_back(ms);
        if times.len() > 1000 {
            times.pop_front(); // O(1) eviction
        }

        Ok(())
    }

    /// Get time to first data (TTFD) in milliseconds
    pub fn get_ttfd_ms(&self) -> u64 {
        self.first_call_time_ms.load(Ordering::Relaxed)
    }

    /// Get calls per minute based on elapsed time
    ///
    /// Returns 0.0 if no time has elapsed or on poison error.
    pub fn get_calls_per_minute(&self) -> f64 {
        // SeqCst for consistent snapshot with record_call
        let total = self.total_calls.load(Ordering::SeqCst);

        let start_time_opt = match self.start_time.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "CallMetrics::start_time",
                    "Mutex poisoned during calls_per_minute read, recovering"
                );
                poison_err.into_inner()
            }
        };

        if let Some(start) = *start_time_opt {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                return (total as f64 / elapsed) * 60.0;
            }
        }

        0.0
    }

    /// Get P50 (median) fetch time in milliseconds
    ///
    /// Returns `MetricsError::EmptyData` if no samples recorded.
    pub fn get_p50_fetch_time_ms(&self) -> MetricsResult<u64> {
        self.get_percentile_fetch_time(0.50, "P50")
    }

    /// Get P95 fetch time in milliseconds
    ///
    /// Returns `MetricsError::EmptyData` if no samples recorded.
    pub fn get_p95_fetch_time_ms(&self) -> MetricsResult<u64> {
        self.get_percentile_fetch_time(0.95, "P95")
    }

    /// Get P99 fetch time in milliseconds
    ///
    /// Returns `MetricsError::EmptyData` if no samples recorded.
    pub fn get_p99_fetch_time_ms(&self) -> MetricsResult<u64> {
        self.get_percentile_fetch_time(0.99, "P99")
    }

    /// Helper to calculate percentile fetch times
    ///
    /// ## Algorithm
    /// - Clone VecDeque to Vec: O(n)
    /// - Sort: O(n log n)
    /// - Calculate index: O(1)
    ///
    /// ## Performance
    /// For n=1000: ~6µs (1µs clone + 5µs sort)
    ///
    /// ## Locking
    /// Holds lock during entire operation for consistent snapshot.
    fn get_percentile_fetch_time(
        &self,
        percentile: f64,
        metric_name: &'static str,
    ) -> MetricsResult<u64> {
        // Poison-safe locking
        let times = match self.fetch_times.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "CallMetrics::fetch_times",
                    percentile = percentile,
                    "Mutex poisoned during percentile read, recovering"
                );
                poison_err.into_inner()
            }
        };

        if times.is_empty() {
            return Err(MetricsError::EmptyData { metric: metric_name });
        }

        // Clone ring buffer to Vec: O(n)
        let mut sorted: Vec<u64> = times.iter().copied().collect();

        // Sort: O(n log n), unstable is faster for primitives
        sorted.sort_unstable();

        // Calculate percentile index: O(1)
        // For 0-indexed arrays: index = (len - 1) * percentile, clamp to valid range
        let index = ((sorted.len() as f64 * percentile) as usize).min(sorted.len() - 1);
        Ok(sorted[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_call() {
        let metrics = CallMetrics::new();
        assert_eq!(metrics.total_calls.load(Ordering::SeqCst), 0);

        metrics.record_call().unwrap();
        assert_eq!(metrics.total_calls.load(Ordering::SeqCst), 1);

        metrics.record_call().unwrap();
        metrics.record_call().unwrap();
        assert_eq!(metrics.total_calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_record_first_call_time() {
        let metrics = CallMetrics::new();
        assert!(!metrics.has_first_call.load(Ordering::Relaxed));
        assert_eq!(metrics.get_ttfd_ms(), 0);

        metrics.record_first_call_time(150).unwrap();
        assert!(metrics.has_first_call.load(Ordering::Relaxed));
        assert_eq!(metrics.get_ttfd_ms(), 150);

        // Subsequent calls should overwrite
        metrics.record_first_call_time(200).unwrap();
        assert_eq!(metrics.get_ttfd_ms(), 200);
    }

    #[test]
    fn test_record_fetch_time() {
        let metrics = CallMetrics::new();

        // First fetch time should set TTFD
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        assert!(metrics.has_first_call.load(Ordering::Relaxed));
        assert_eq!(metrics.get_ttfd_ms(), 100);

        // Additional fetch times should be stored
        metrics.record_fetch_time(Duration::from_millis(200)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(150)).unwrap();

        let times = match metrics.fetch_times.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        assert_eq!(times.len(), 3);
        assert_eq!(times[0], 100);
        assert_eq!(times[1], 200);
        assert_eq!(times[2], 150);
    }

    #[test]
    fn test_percentile_calculations() {
        let metrics = CallMetrics::new();

        // Empty should return EmptyData error
        assert!(matches!(
            metrics.get_p50_fetch_time_ms(),
            Err(MetricsError::EmptyData { metric: "P50" })
        ));
        assert!(matches!(
            metrics.get_p95_fetch_time_ms(),
            Err(MetricsError::EmptyData { metric: "P95" })
        ));
        assert!(matches!(
            metrics.get_p99_fetch_time_ms(),
            Err(MetricsError::EmptyData { metric: "P99" })
        ));

        // Add some fetch times
        for ms in [100, 200, 300, 400, 500] {
            metrics.record_fetch_time(Duration::from_millis(ms)).unwrap();
        }

        // P50 should be 300 (median of 5 values: index = 5 * 0.5 = 2.5 -> 2, sorted[2]
        // = 300)
        assert_eq!(metrics.get_p50_fetch_time_ms().unwrap(), 300);

        // P95 should be 500 (95th percentile of 5 values: index = 5 * 0.95 = 4.75 -> 4,
        // sorted[4] = 500)
        assert_eq!(metrics.get_p95_fetch_time_ms().unwrap(), 500);

        // P99 should be 500 (99th percentile of 5 values: index = 5 * 0.99 = 4.95 -> 4,
        // sorted[4] = 500)
        assert_eq!(metrics.get_p99_fetch_time_ms().unwrap(), 500);
    }

    #[test]
    fn test_fetch_times_ring_buffer() {
        let metrics = CallMetrics::new();

        // Add more than 1000 entries
        for i in 0..1100 {
            metrics.record_fetch_time(Duration::from_millis(i)).unwrap();
        }

        // Should only keep last 1000 (ring buffer with FIFO eviction)
        let times = match metrics.fetch_times.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        assert_eq!(times.len(), 1000);
        // First entry should be 100 (0-99 were evicted via pop_front)
        assert_eq!(times[0], 100);
        assert_eq!(times[999], 1099);
    }

    #[test]
    fn test_calls_per_minute() {
        let metrics = CallMetrics::new();

        // Record some calls
        for _ in 0..10 {
            metrics.record_call().unwrap();
        }

        let cpm = metrics.get_calls_per_minute();
        // Should be > 0 (exact value depends on timing)
        assert!(cpm > 0.0);
    }

    // Failure path tests

    #[test]
    fn test_empty_percentile_returns_error() {
        let metrics = CallMetrics::new();

        // No samples recorded
        let result = metrics.get_p95_fetch_time_ms();
        assert!(matches!(result, Err(MetricsError::EmptyData { .. })));
    }

    #[test]
    fn test_poison_recovery_during_record() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(CallMetrics::new());

        // Poison the mutex by panicking during lock
        let metrics_clone = Arc::clone(&metrics);
        let _ = thread::spawn(move || {
            let _guard = metrics_clone.fetch_times.lock().unwrap();
            panic!("intentional poison");
        })
        .join();

        // Subsequent calls should recover from poison
        let result = metrics.record_fetch_time(Duration::from_millis(100));
        assert!(result.is_ok(), "Should recover from poison");

        // Verify data was recorded despite poison
        let times = match metrics.fetch_times.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        assert_eq!(times.len(), 1);
        assert_eq!(times[0], 100);
    }

    #[test]
    fn test_poison_recovery_during_read() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(CallMetrics::new());

        // Add some data first
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(200)).unwrap();

        // Poison the mutex
        let metrics_clone = Arc::clone(&metrics);
        let _ = thread::spawn(move || {
            let _guard = metrics_clone.fetch_times.lock().unwrap();
            panic!("intentional poison");
        })
        .join();

        // Read should recover and return correct value
        let result = metrics.get_p50_fetch_time_ms();
        assert!(result.is_ok(), "Should recover from poison during read");
        // Median of [100, 200]: index = 2 * 0.5 = 1, sorted[1] = 200
        assert_eq!(result.unwrap(), 200);
    }
}
