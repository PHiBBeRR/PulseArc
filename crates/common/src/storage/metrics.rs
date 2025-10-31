//! Storage metrics tracking
//!
//! Provides simple metric tracking for storage operations without external
//! dependencies.

use std::sync::atomic::{AtomicU64, Ordering};

/// Simple storage metrics tracker
///
/// Tracks connection pool and query execution metrics using atomic counters
/// for thread-safe operation without locks.
#[derive(Debug)]
pub struct StorageMetrics {
    /// Number of connections successfully acquired from the pool
    pub connections_acquired: AtomicU64,

    /// Number of connection acquisition timeouts
    pub connections_timeout: AtomicU64,

    /// Number of connection errors
    pub connections_error: AtomicU64,

    /// Total time spent acquiring connections (in milliseconds)
    total_connection_time_ms: AtomicU64,

    /// Number of queries executed
    pub queries_executed: AtomicU64,

    /// Number of queries that failed
    pub queries_failed: AtomicU64,

    /// Maximum pool size (for calculation purposes)
    max_pool_size: u32,
}

impl StorageMetrics {
    /// Create a new metrics tracker
    pub fn new(max_pool_size: u32) -> Self {
        Self {
            connections_acquired: AtomicU64::new(0),
            connections_timeout: AtomicU64::new(0),
            connections_error: AtomicU64::new(0),
            total_connection_time_ms: AtomicU64::new(0),
            queries_executed: AtomicU64::new(0),
            queries_failed: AtomicU64::new(0),
            max_pool_size,
        }
    }

    /// Record a successful connection acquisition
    pub fn record_connection_acquired(&self, duration_ms: u64) {
        self.connections_acquired.fetch_add(1, Ordering::Relaxed);
        self.total_connection_time_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Record a connection timeout
    pub fn record_connection_timeout(&self) {
        self.connections_timeout.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection error
    pub fn record_connection_error(&self) {
        self.connections_error.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful query execution
    pub fn record_query_executed(&self) {
        self.queries_executed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed query
    pub fn record_query_failed(&self) {
        self.queries_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get average connection acquisition time in milliseconds
    pub fn avg_connection_time_ms(&self) -> u64 {
        let total = self.total_connection_time_ms.load(Ordering::Relaxed);
        let count = self.connections_acquired.load(Ordering::Relaxed);

        if count == 0 {
            0
        } else {
            total / count
        }
    }

    /// Get the maximum pool size
    pub fn max_pool_size(&self) -> u32 {
        self.max_pool_size
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::metrics.
    use super::*;

    /// Validates `StorageMetrics::new` behavior for the metrics creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.max_pool_size()` equals `10`.
    /// - Confirms `metrics.connections_acquired.load(Ordering::Relaxed)` equals
    ///   `0`.
    #[test]
    fn test_metrics_creation() {
        let metrics = StorageMetrics::new(10);
        assert_eq!(metrics.max_pool_size(), 10);
        assert_eq!(metrics.connections_acquired.load(Ordering::Relaxed), 0);
    }

    /// Validates `StorageMetrics::new` behavior for the connection acquired
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.connections_acquired.load(Ordering::Relaxed)` equals
    ///   `2`.
    /// - Confirms `metrics.avg_connection_time_ms()` equals `150`.
    #[test]
    fn test_connection_acquired() {
        let metrics = StorageMetrics::new(10);
        metrics.record_connection_acquired(100);
        metrics.record_connection_acquired(200);

        assert_eq!(metrics.connections_acquired.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.avg_connection_time_ms(), 150);
    }

    /// Validates `StorageMetrics::new` behavior for the connection errors
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.connections_timeout.load(Ordering::Relaxed)` equals
    ///   `1`.
    /// - Confirms `metrics.connections_error.load(Ordering::Relaxed)` equals
    ///   `1`.
    #[test]
    fn test_connection_errors() {
        let metrics = StorageMetrics::new(10);
        metrics.record_connection_timeout();
        metrics.record_connection_error();

        assert_eq!(metrics.connections_timeout.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.connections_error.load(Ordering::Relaxed), 1);
    }

    /// Validates `StorageMetrics::new` behavior for the query tracking
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.queries_executed.load(Ordering::Relaxed)` equals
    ///   `2`.
    /// - Confirms `metrics.queries_failed.load(Ordering::Relaxed)` equals `1`.
    #[test]
    fn test_query_tracking() {
        let metrics = StorageMetrics::new(10);
        metrics.record_query_executed();
        metrics.record_query_executed();
        metrics.record_query_failed();

        assert_eq!(metrics.queries_executed.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.queries_failed.load(Ordering::Relaxed), 1);
    }

    /// Validates `StorageMetrics::new` behavior for the avg with no connections
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.avg_connection_time_ms()` equals `0`.
    #[test]
    fn test_avg_with_no_connections() {
        let metrics = StorageMetrics::new(10);
        assert_eq!(metrics.avg_connection_time_ms(), 0);
    }
}
