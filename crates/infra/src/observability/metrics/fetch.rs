//! Fetch-related metrics for tracking data retrieval performance
//!
//! This module tracks metrics related to data fetching including timing,
//! errors, and timeouts.
//!
//! ## Design
//! - **SeqCst ordering** for atomics used in derived metrics (avg_fetch_time)
//! - **No locking needed** - simple atomic counters
//! - **MetricsResult returns** for future extensibility (currently always Ok)
//! - **Microsecond storage** - stores raw durations in µs, reporting helpers
//!   convert to ms

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use crate::observability::MetricsResult;

/// Metrics for tracking data fetch performance
///
/// All record methods return `MetricsResult<()>` for future extensibility
/// (quotas, validation), but currently always succeed.
#[derive(Debug, Default)]
pub struct FetchMetrics {
    /// Total time spent fetching data in microseconds
    pub total_fetch_time_micros: AtomicU64,
    /// Last fetch time in microseconds
    pub last_fetch_time_micros: AtomicU64,
    /// Number of fetch operations recorded
    pub fetch_count: AtomicUsize,
    /// Number of errors encountered
    pub errors: AtomicUsize,
    /// Number of timeouts encountered
    pub timeouts: AtomicUsize,
}

impl FetchMetrics {
    /// Create new FetchMetrics instance
    pub fn new() -> Self {
        Self {
            total_fetch_time_micros: AtomicU64::new(0),
            last_fetch_time_micros: AtomicU64::new(0),
            fetch_count: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            timeouts: AtomicUsize::new(0),
        }
    }

    /// Record a fetch time
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_fetch_time(&self, duration: Duration) -> MetricsResult<()> {
        let micros = duration.as_micros() as u64;

        // SeqCst for consistency with avg_fetch_time calculation
        self.total_fetch_time_micros.fetch_add(micros, Ordering::SeqCst);
        self.fetch_count.fetch_add(1, Ordering::SeqCst);

        // Relaxed OK: last_fetch_time is not used in derived metrics
        self.last_fetch_time_micros.store(micros, Ordering::Relaxed);

        Ok(())
    }

    /// Record an error
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_error(&self) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.errors.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Record a timeout
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_timeout(&self) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.timeouts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get the average fetch time in milliseconds
    ///
    /// Returns 0.0 if no fetch operations have been recorded.
    ///
    /// ## Memory Ordering
    /// Uses SeqCst to ensure consistent snapshot of total time and count.
    pub fn get_avg_fetch_time_ms(&self) -> f64 {
        // SeqCst for consistent snapshot
        let total_time = self.total_fetch_time_micros.load(Ordering::SeqCst);
        let count = self.fetch_count.load(Ordering::SeqCst);

        if count == 0 {
            return 0.0;
        }

        (total_time as f64 / count as f64) / 1_000.0
    }

    /// Get the last fetch time in milliseconds
    pub fn get_last_fetch_time_ms(&self) -> u64 {
        self.last_fetch_time_micros.load(Ordering::Relaxed) / 1_000
    }

    /// Get the total fetch time in milliseconds
    pub fn get_total_fetch_time_ms(&self) -> u64 {
        self.total_fetch_time_micros.load(Ordering::SeqCst) / 1_000
    }

    /// Get the fetch count
    pub fn get_fetch_count(&self) -> usize {
        self.fetch_count.load(Ordering::SeqCst)
    }

    /// Get the error count
    pub fn get_error_count(&self) -> usize {
        self.errors.load(Ordering::Relaxed)
    }

    /// Get the timeout count
    pub fn get_timeout_count(&self) -> usize {
        self.timeouts.load(Ordering::Relaxed)
    }

    /// Get the true average response time including cache hits
    ///
    /// Assumes cache hits take ~2ms on average.
    ///
    /// ## Parameters
    /// - `total_calls`: Total number of calls (hits + misses)
    /// - `cache_hits`: Number of cache hits
    ///
    /// ## Calculation
    /// ```text
    /// cache_misses = total_calls - cache_hits
    /// avg_fetch_time = total_fetch_time / cache_misses
    /// hit_rate = cache_hits / total_calls
    /// miss_rate = cache_misses / total_calls
    /// true_avg = (hit_rate * 2ms) + (miss_rate * avg_fetch_time)
    /// ```
    pub fn get_true_avg_response_time_ms(&self, total_calls: usize, cache_hits: usize) -> f64 {
        if total_calls == 0 {
            return 0.0;
        }

        let cache_misses = total_calls.saturating_sub(cache_hits);
        let avg_fetch_time = if cache_misses > 0 {
            let total_time = self.total_fetch_time_micros.load(Ordering::SeqCst);
            (total_time as f64 / cache_misses as f64) / 1_000.0
        } else {
            0.0
        };

        // Assume cache hits take ~2ms on average
        let cache_hit_time = 2.0;
        let hit_rate = cache_hits as f64 / total_calls as f64;
        let miss_rate = cache_misses as f64 / total_calls as f64;

        (hit_rate * cache_hit_time) + (miss_rate * avg_fetch_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let metrics = FetchMetrics::new();
        assert_eq!(metrics.get_total_fetch_time_ms(), 0);
        assert_eq!(metrics.get_last_fetch_time_ms(), 0);
        assert_eq!(metrics.get_fetch_count(), 0);
        assert_eq!(metrics.get_error_count(), 0);
        assert_eq!(metrics.get_timeout_count(), 0);
        assert_eq!(metrics.get_avg_fetch_time_ms(), 0.0);
    }

    #[test]
    fn test_record_fetch_time() {
        let metrics = FetchMetrics::new();
        assert_eq!(metrics.total_fetch_time_micros.load(Ordering::SeqCst), 0);
        assert_eq!(metrics.last_fetch_time_micros.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.fetch_count.load(Ordering::SeqCst), 0);

        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        assert_eq!(metrics.total_fetch_time_micros.load(Ordering::SeqCst), 100_000);
        assert_eq!(metrics.last_fetch_time_micros.load(Ordering::Relaxed), 100_000);
        assert_eq!(metrics.fetch_count.load(Ordering::SeqCst), 1);

        metrics.record_fetch_time(Duration::from_millis(200)).unwrap();
        assert_eq!(metrics.total_fetch_time_micros.load(Ordering::SeqCst), 300_000);
        assert_eq!(metrics.last_fetch_time_micros.load(Ordering::Relaxed), 200_000);
        assert_eq!(metrics.fetch_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_record_error() {
        let metrics = FetchMetrics::new();
        assert_eq!(metrics.errors.load(Ordering::Relaxed), 0);

        metrics.record_error().unwrap();
        assert_eq!(metrics.errors.load(Ordering::Relaxed), 1);

        metrics.record_error().unwrap();
        assert_eq!(metrics.errors.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.get_error_count(), 2);
    }

    #[test]
    fn test_record_timeout() {
        let metrics = FetchMetrics::new();
        assert_eq!(metrics.timeouts.load(Ordering::Relaxed), 0);

        metrics.record_timeout().unwrap();
        assert_eq!(metrics.timeouts.load(Ordering::Relaxed), 1);

        assert_eq!(metrics.get_timeout_count(), 1);
    }

    #[test]
    fn test_get_avg_fetch_time_ms() {
        let metrics = FetchMetrics::new();

        // No fetches should return 0
        assert_eq!(metrics.get_avg_fetch_time_ms(), 0.0);

        // Total 300ms over 3 fetches = 100ms average
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();

        assert_eq!(metrics.get_avg_fetch_time_ms(), 100.0);
        assert_eq!(metrics.get_fetch_count(), 3);
    }

    #[test]
    fn test_get_true_avg_response_time_ms() {
        let metrics = FetchMetrics::new();

        // No calls should return 0
        assert_eq!(metrics.get_true_avg_response_time_ms(0, 0), 0.0);

        // Scenario: 7 cache hits, 3 cache misses (avg 100ms)
        // Expected: (0.7 * 2ms) + (0.3 * 100ms) = 1.4 + 30 = 31.4ms
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();

        let avg_response = metrics.get_true_avg_response_time_ms(10, 7);
        assert!((avg_response - 31.4).abs() < 0.01);
    }

    #[test]
    fn test_get_true_avg_all_cache_hits() {
        let metrics = FetchMetrics::new();

        // All cache hits: 10 calls, 10 hits, 0 fetches
        // Expected: (1.0 * 2ms) + (0.0 * 0) = 2.0ms
        let avg_response = metrics.get_true_avg_response_time_ms(10, 10);
        assert_eq!(avg_response, 2.0);
    }

    #[test]
    fn test_get_true_avg_all_cache_misses() {
        let metrics = FetchMetrics::new();

        // All cache misses: 3 calls, 0 hits, 3 fetches (avg 100ms)
        // Expected: (0.0 * 2ms) + (1.0 * 100ms) = 100.0ms
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();

        let avg_response = metrics.get_true_avg_response_time_ms(3, 0);
        assert_eq!(avg_response, 100.0);
    }

    // Failure path test: Zero total edge case

    #[test]
    fn test_avg_fetch_time_zero_count_returns_zero() {
        let metrics = FetchMetrics::new();

        // No fetches recorded
        assert_eq!(metrics.get_avg_fetch_time_ms(), 0.0);
        assert_eq!(metrics.get_fetch_count(), 0);
    }

    #[test]
    fn test_true_avg_zero_total_calls_returns_zero() {
        let metrics = FetchMetrics::new();

        // Record some fetches but report 0 total calls
        metrics.record_fetch_time(Duration::from_millis(100)).unwrap();

        assert_eq!(metrics.get_true_avg_response_time_ms(0, 0), 0.0);
    }
}
