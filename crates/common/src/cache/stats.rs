//! Cache statistics and metrics tracking
//!
//! This module provides types for tracking cache performance metrics
//! including hit rates, eviction counts, and access patterns.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Statistics for cache performance monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Current number of entries
    pub size: usize,

    /// Maximum allowed entries (None = unlimited)
    pub max_size: Option<usize>,

    /// Total number of successful get operations
    pub hits: u64,

    /// Total number of failed get operations (key not found or expired)
    pub misses: u64,

    /// Total number of insert operations
    pub inserts: u64,

    /// Total number of evicted entries
    pub evictions: u64,

    /// Total number of expired entries removed
    pub expirations: u64,
}

impl CacheStats {
    /// Calculate hit rate (hits / total accesses)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate miss rate (misses / total accesses)
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// Calculate fill percentage (size / max_size)
    pub fn fill_percentage(&self) -> Option<f64> {
        self.max_size.map(|max| if max == 0 { 0.0 } else { self.size as f64 / max as f64 })
    }

    /// Total number of access operations (hits + misses)
    pub fn total_accesses(&self) -> u64 {
        self.hits + self.misses
    }
}

/// Thread-safe metrics collector for cache operations
///
/// This struct uses atomic operations to track cache metrics
/// without requiring locks, enabling low-overhead monitoring.
#[derive(Debug)]
pub(crate) struct MetricsCollector {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    inserts: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
    expirations: Arc<AtomicU64>,
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            hits: Arc::clone(&self.hits),
            misses: Arc::clone(&self.misses),
            inserts: Arc::clone(&self.inserts),
            evictions: Arc::clone(&self.evictions),
            expirations: Arc::clone(&self.expirations),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub(crate) fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            inserts: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
            expirations: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a cache hit
    pub(crate) fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub(crate) fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an insert operation
    pub(crate) fn record_insert(&self) {
        self.inserts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an eviction
    pub(crate) fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an expiration
    pub(crate) fn record_expiration(&self) {
        self.expirations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current statistics snapshot
    pub(crate) fn snapshot(&self, size: usize, max_size: Option<usize>) -> CacheStats {
        CacheStats {
            size,
            max_size,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            inserts: self.inserts.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            expirations: self.expirations.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics to zero
    pub(crate) fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.inserts.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.expirations.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for cache::stats.
    use super::*;

    /// Validates `CacheStats::default` behavior for the cache stats default
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.size` equals `0`.
    /// - Ensures `stats.max_size.is_none()` evaluates to true.
    /// - Confirms `stats.hits` equals `0`.
    /// - Confirms `stats.misses` equals `0`.
    /// - Confirms `stats.inserts` equals `0`.
    /// - Confirms `stats.evictions` equals `0`.
    /// - Confirms `stats.expirations` equals `0`.
    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.size, 0);
        assert!(stats.max_size.is_none());
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.inserts, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.expirations, 0);
    }

    /// Validates `Default::default` behavior for the hit rate calculation
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `(stats.hit_rate() - 0.8).abs() < 1e-10` evaluates to true.
    /// - Ensures `(stats.miss_rate() - 0.2).abs() < 1e-10` evaluates to true.
    /// - Confirms `stats.total_accesses()` equals `100`.
    #[test]
    fn test_hit_rate_calculation() {
        let stats = CacheStats { hits: 80, misses: 20, ..Default::default() };

        assert!((stats.hit_rate() - 0.8).abs() < 1e-10);
        assert!((stats.miss_rate() - 0.2).abs() < 1e-10);
        assert_eq!(stats.total_accesses(), 100);
    }

    /// Validates `CacheStats::default` behavior for the hit rate no accesses
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hit_rate()` equals `0.0`.
    /// - Confirms `stats.miss_rate()` equals `1.0`.
    /// - Confirms `stats.total_accesses()` equals `0`.
    #[test]
    fn test_hit_rate_no_accesses() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.miss_rate(), 1.0);
        assert_eq!(stats.total_accesses(), 0);
    }

    /// Validates `Default::default` behavior for the hit rate all hits
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hit_rate()` equals `1.0`.
    /// - Confirms `stats.miss_rate()` equals `0.0`.
    #[test]
    fn test_hit_rate_all_hits() {
        let stats = CacheStats { hits: 100, misses: 0, ..Default::default() };

        assert_eq!(stats.hit_rate(), 1.0);
        assert_eq!(stats.miss_rate(), 0.0);
    }

    /// Validates `Default::default` behavior for the fill percentage scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.fill_percentage()` equals `Some(0.5)`.
    #[test]
    fn test_fill_percentage() {
        let stats = CacheStats { size: 50, max_size: Some(100), ..Default::default() };

        assert_eq!(stats.fill_percentage(), Some(0.5));
    }

    /// Validates `Default::default` behavior for the fill percentage no limit
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.fill_percentage()` equals `None`.
    #[test]
    fn test_fill_percentage_no_limit() {
        let stats = CacheStats { size: 50, max_size: None, ..Default::default() };

        assert_eq!(stats.fill_percentage(), None);
    }

    /// Validates `Default::default` behavior for the fill percentage zero max
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.fill_percentage()` equals `Some(0.0)`.
    #[test]
    fn test_fill_percentage_zero_max() {
        let stats = CacheStats { size: 0, max_size: Some(0), ..Default::default() };

        assert_eq!(stats.fill_percentage(), Some(0.0));
    }

    /// Validates `MetricsCollector::new` behavior for the metrics collector new
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hits` equals `0`.
    /// - Confirms `stats.misses` equals `0`.
    /// - Confirms `stats.inserts` equals `0`.
    /// - Confirms `stats.evictions` equals `0`.
    /// - Confirms `stats.expirations` equals `0`.
    #[test]
    fn test_metrics_collector_new() {
        let collector = MetricsCollector::new();
        let stats = collector.snapshot(0, None);

        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.inserts, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.expirations, 0);
    }

    /// Validates `MetricsCollector::new` behavior for the metrics collector
    /// record hit scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hits` equals `2`.
    #[test]
    fn test_metrics_collector_record_hit() {
        let collector = MetricsCollector::new();
        collector.record_hit();
        collector.record_hit();

        let stats = collector.snapshot(0, None);
        assert_eq!(stats.hits, 2);
    }

    /// Validates `MetricsCollector::new` behavior for the metrics collector
    /// record miss scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.misses` equals `3`.
    #[test]
    fn test_metrics_collector_record_miss() {
        let collector = MetricsCollector::new();
        collector.record_miss();
        collector.record_miss();
        collector.record_miss();

        let stats = collector.snapshot(0, None);
        assert_eq!(stats.misses, 3);
    }

    /// Validates `MetricsCollector::new` behavior for the metrics collector
    /// record operations scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hits` equals `1`.
    /// - Confirms `stats.misses` equals `1`.
    /// - Confirms `stats.inserts` equals `1`.
    /// - Confirms `stats.evictions` equals `1`.
    /// - Confirms `stats.expirations` equals `1`.
    /// - Confirms `stats.size` equals `5`.
    /// - Confirms `stats.max_size` equals `Some(10)`.
    #[test]
    fn test_metrics_collector_record_operations() {
        let collector = MetricsCollector::new();

        collector.record_hit();
        collector.record_miss();
        collector.record_insert();
        collector.record_eviction();
        collector.record_expiration();

        let stats = collector.snapshot(5, Some(10));

        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.inserts, 1);
        assert_eq!(stats.evictions, 1);
        assert_eq!(stats.expirations, 1);
        assert_eq!(stats.size, 5);
        assert_eq!(stats.max_size, Some(10));
    }

    /// Validates `MetricsCollector::new` behavior for the metrics collector
    /// reset scenario.
    ///
    /// Assertions:
    /// - Confirms `stats_before.hits` equals `1`.
    /// - Confirms `stats_before.misses` equals `1`.
    /// - Confirms `stats_before.inserts` equals `1`.
    /// - Confirms `stats_after.hits` equals `0`.
    /// - Confirms `stats_after.misses` equals `0`.
    /// - Confirms `stats_after.inserts` equals `0`.
    #[test]
    fn test_metrics_collector_reset() {
        let collector = MetricsCollector::new();

        collector.record_hit();
        collector.record_miss();
        collector.record_insert();

        let stats_before = collector.snapshot(0, None);
        assert_eq!(stats_before.hits, 1);
        assert_eq!(stats_before.misses, 1);
        assert_eq!(stats_before.inserts, 1);

        collector.reset();

        let stats_after = collector.snapshot(0, None);
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
        assert_eq!(stats_after.inserts, 0);
    }

    /// Validates `MetricsCollector::new` behavior for the metrics collector
    /// clone scenario.
    ///
    /// Assertions:
    /// - Confirms `stats1.hits` equals `2`.
    /// - Confirms `stats2.hits` equals `2`.
    #[test]
    fn test_metrics_collector_clone() {
        let collector1 = MetricsCollector::new();
        collector1.record_hit();

        let collector2 = collector1.clone();
        collector2.record_hit();

        // Both should see the same counts (shared Arc)
        let stats1 = collector1.snapshot(0, None);
        let stats2 = collector2.snapshot(0, None);

        assert_eq!(stats1.hits, 2);
        assert_eq!(stats2.hits, 2);
    }

    /// Validates `Arc::new` behavior for the metrics collector thread safety
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hits` equals `1000`.
    #[test]
    fn test_metrics_collector_thread_safety() {
        use std::thread;

        let collector = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        // Spawn 10 threads, each recording 100 hits
        for _ in 0..10 {
            let collector_clone = Arc::clone(&collector);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    collector_clone.record_hit();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = collector.snapshot(0, None);
        assert_eq!(stats.hits, 1000);
    }
}
