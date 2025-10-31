//! Cache-related metrics for tracking cache performance
//!
//! This module tracks cache hit/miss rates and provides cache performance statistics.
//!
//! ## Design
//! - **SeqCst ordering** for atomics used in hit_rate calculation (derived metric)
//! - **No locking needed** - simple atomic counters
//! - **MetricsResult returns** for future extensibility (currently always Ok)

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::observability::MetricsResult;

/// Metrics for tracking cache performance
///
/// All record methods return `MetricsResult<()>` for future extensibility
/// (quotas, validation), but currently always succeed.
#[derive(Debug, Default)]
pub struct CacheMetrics {
    /// Number of cache hits
    pub cache_hits: AtomicUsize,
    /// Number of cache misses
    pub cache_misses: AtomicUsize,
}

impl CacheMetrics {
    /// Create new CacheMetrics instance
    pub fn new() -> Self {
        Self { cache_hits: AtomicUsize::new(0), cache_misses: AtomicUsize::new(0) }
    }

    /// Record a cache hit
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_hit(&self) -> MetricsResult<()> {
        // SeqCst for consistency with get_hit_rate calculation
        self.cache_hits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Record a cache miss
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_miss(&self) -> MetricsResult<()> {
        // SeqCst for consistency with get_hit_rate calculation
        self.cache_misses.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Calculate cache hit rate as a percentage (0.0 to 100.0)
    ///
    /// Returns 0.0 if no cache operations have been recorded (zero total).
    ///
    /// ## Calculation
    /// ```text
    /// hit_rate = (hits / (hits + misses)) * 100.0
    /// ```
    ///
    /// ## Memory Ordering
    /// Uses SeqCst to ensure consistent snapshot of hits and misses.
    pub fn get_hit_rate(&self) -> f64 {
        // SeqCst for consistent snapshot of both counters
        let hits = self.cache_hits.load(Ordering::SeqCst);
        let misses = self.cache_misses.load(Ordering::SeqCst);

        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }

        (hits as f64 / total as f64) * 100.0
    }

    /// Get total number of cache hits
    pub fn get_hits(&self) -> usize {
        self.cache_hits.load(Ordering::SeqCst)
    }

    /// Get total number of cache misses
    pub fn get_misses(&self) -> usize {
        self.cache_misses.load(Ordering::SeqCst)
    }

    /// Get total number of cache operations (hits + misses)
    pub fn get_total(&self) -> usize {
        let hits = self.cache_hits.load(Ordering::SeqCst);
        let misses = self.cache_misses.load(Ordering::SeqCst);
        hits + misses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let metrics = CacheMetrics::new();
        assert_eq!(metrics.get_hits(), 0);
        assert_eq!(metrics.get_misses(), 0);
        assert_eq!(metrics.get_total(), 0);
        assert_eq!(metrics.get_hit_rate(), 0.0);
    }

    #[test]
    fn test_record_hit() {
        let metrics = CacheMetrics::new();
        assert_eq!(metrics.cache_hits.load(Ordering::SeqCst), 0);

        metrics.record_hit().unwrap();
        assert_eq!(metrics.cache_hits.load(Ordering::SeqCst), 1);

        metrics.record_hit().unwrap();
        metrics.record_hit().unwrap();
        assert_eq!(metrics.cache_hits.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_record_miss() {
        let metrics = CacheMetrics::new();
        assert_eq!(metrics.cache_misses.load(Ordering::SeqCst), 0);

        metrics.record_miss().unwrap();
        assert_eq!(metrics.cache_misses.load(Ordering::SeqCst), 1);

        metrics.record_miss().unwrap();
        assert_eq!(metrics.cache_misses.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_get_hit_rate() {
        let metrics = CacheMetrics::new();

        // No operations should return 0%
        assert_eq!(metrics.get_hit_rate(), 0.0);

        // 3 hits, 7 misses = 30% hit rate
        for _ in 0..3 {
            metrics.record_hit().unwrap();
        }
        for _ in 0..7 {
            metrics.record_miss().unwrap();
        }

        let hit_rate = metrics.get_hit_rate();
        assert!((hit_rate - 30.0).abs() < 0.01);
        assert_eq!(metrics.get_total(), 10);
    }

    #[test]
    fn test_get_hit_rate_all_hits() {
        let metrics = CacheMetrics::new();

        // 10 hits, 0 misses = 100% hit rate
        for _ in 0..10 {
            metrics.record_hit().unwrap();
        }

        let hit_rate = metrics.get_hit_rate();
        assert!((hit_rate - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_get_hit_rate_all_misses() {
        let metrics = CacheMetrics::new();

        // 0 hits, 10 misses = 0% hit rate
        for _ in 0..10 {
            metrics.record_miss().unwrap();
        }

        let hit_rate = metrics.get_hit_rate();
        assert_eq!(hit_rate, 0.0);
    }

    #[test]
    fn test_get_totals() {
        let metrics = CacheMetrics::new();

        metrics.record_hit().unwrap();
        metrics.record_hit().unwrap();
        metrics.record_miss().unwrap();

        assert_eq!(metrics.get_hits(), 2);
        assert_eq!(metrics.get_misses(), 1);
        assert_eq!(metrics.get_total(), 3);
    }

    // Failure path test: Zero total edge case

    #[test]
    fn test_hit_rate_zero_total_returns_zero() {
        let metrics = CacheMetrics::new();

        // No operations recorded
        assert_eq!(metrics.get_hit_rate(), 0.0);
        assert_eq!(metrics.get_total(), 0);
    }
}
