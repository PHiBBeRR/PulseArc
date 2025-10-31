//! Database connection pool metrics for monitoring SqlCipher performance
//!
//! Tracks connection pool performance, query execution timing, and pool
//! utilization to validate concurrent connection pooling implementation.
//!
//! ## Design
//! - **VecDeque ring buffer** for O(1) eviction (not Vec with remove(0))
//! - **Poison-safe locking** with explicit match pattern (no .expect())
//! - **SeqCst ordering** for atomics used in derived metrics (rates,
//!   percentiles)
//! - **Relaxed ordering** for independent counters
//! - **MetricsResult returns** for future extensibility (currently always Ok)

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::observability::{MetricsError, MetricsResult};

/// Database connection pool performance metrics
///
/// All record methods return `MetricsResult<()>` for future extensibility
/// (cardinality limits, quotas), but currently always succeed.
#[derive(Debug)]
pub struct DbMetrics {
    // Connection Acquisition Metrics
    /// Total connections successfully acquired
    connections_acquired: AtomicU64,
    /// Connection acquisition times in milliseconds (ring buffer, max 1000)
    connection_acquisition_times_ms: Mutex<VecDeque<u64>>,
    /// Total connection acquisition timeouts
    connection_timeouts: AtomicU64,
    /// Total connection acquisition errors
    connection_errors: AtomicU64,

    // Query Execution Metrics
    /// Total queries executed
    queries_executed: AtomicU64,
    /// Query execution times in milliseconds (ring buffer, max 1000)
    query_execution_times_ms: Mutex<VecDeque<u64>>,
    /// Total query execution errors
    query_errors: AtomicU64,

    // Pool Utilization Metrics
    /// Peak concurrent connections observed
    peak_concurrent_connections: AtomicU64,
    /// Total connections in the pool (set during initialization)
    total_connections_in_pool: AtomicU64,

    // Fallback Tracking (for dual-path strategy during migration)
    /// Successful database manager operations
    dbmanager_successes: AtomicU64,
    /// Fallbacks to legacy LocalDatabase
    localdatabase_fallbacks: AtomicU64,

    // Timing
    /// Timestamp of first connection in milliseconds since start
    first_connection_epoch_ms: AtomicU64,
}

impl Default for DbMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl DbMetrics {
    /// Create new DbMetrics instance
    pub fn new() -> Self {
        Self {
            connections_acquired: AtomicU64::new(0),
            connection_acquisition_times_ms: Mutex::new(VecDeque::with_capacity(1000)),
            connection_timeouts: AtomicU64::new(0),
            connection_errors: AtomicU64::new(0),

            queries_executed: AtomicU64::new(0),
            query_execution_times_ms: Mutex::new(VecDeque::with_capacity(1000)),
            query_errors: AtomicU64::new(0),

            peak_concurrent_connections: AtomicU64::new(0),
            total_connections_in_pool: AtomicU64::new(0),

            dbmanager_successes: AtomicU64::new(0),
            localdatabase_fallbacks: AtomicU64::new(0),

            first_connection_epoch_ms: AtomicU64::new(0),
        }
    }

    // ========================================================================
    // Connection Acquisition
    // ========================================================================

    /// Record successful connection acquisition with timing
    ///
    /// Maintains ring buffer of last 1000 samples. Uses VecDeque for O(1)
    /// eviction.
    ///
    /// Currently always succeeds. Future versions may enforce cardinality
    /// limits.
    pub fn record_connection_acquired(&self, duration_ms: u64) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.connections_acquired.fetch_add(1, Ordering::Relaxed);

        // Poison-safe locking: explicit match, no .expect()
        let mut times = match self.connection_acquisition_times_ms.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "DbMetrics::connection_acquisition_times",
                    "Mutex poisoned during connection time recording, recovering data"
                );
                poison_err.into_inner()
            }
        };

        // Ring buffer: O(1) push_back + pop_front
        times.push_back(duration_ms);
        if times.len() > 1000 {
            times.pop_front();
        }

        // Record first connection time for TTFD tracking (CAS to avoid race condition)
        // Only the first thread to see 0 will successfully set the value
        let _ = self.first_connection_epoch_ms.compare_exchange(
            0,
            duration_ms,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );

        Ok(())
    }

    /// Record connection acquisition timeout
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_connection_timeout(&self) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.connection_timeouts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Record connection acquisition error
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_connection_error(&self) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get P50 connection acquisition latency in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no connections recorded.
    pub fn p50_connection_time_ms(&self) -> MetricsResult<u64> {
        self.get_connection_percentile(50)
    }

    /// Get P95 connection acquisition latency in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no connections recorded.
    pub fn p95_connection_time_ms(&self) -> MetricsResult<u64> {
        self.get_connection_percentile(95)
    }

    /// Get P99 connection acquisition latency in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no connections recorded.
    pub fn p99_connection_time_ms(&self) -> MetricsResult<u64> {
        self.get_connection_percentile(99)
    }

    /// Calculate connection acquisition time percentile
    fn get_connection_percentile(&self, percentile: u8) -> MetricsResult<u64> {
        // Poison-safe locking
        let times = match self.connection_acquisition_times_ms.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "DbMetrics::connection_acquisition_times",
                    "Mutex poisoned during percentile calculation, recovering data"
                );
                poison_err.into_inner()
            }
        };

        if times.is_empty() {
            return Err(MetricsError::EmptyData { metric: "connection_time_percentile" });
        }

        let mut sorted: Vec<u64> = times.iter().copied().collect();
        sorted.sort_unstable();

        // Corrected percentile formula: (len * percentile / 100) rounded up, minus 1
        // for zero-indexing
        let index = (sorted.len() * usize::from(percentile)).div_ceil(100).saturating_sub(1);
        Ok(sorted[index.min(sorted.len() - 1)])
    }

    // ========================================================================
    // Query Execution
    // ========================================================================

    /// Record query execution with timing
    ///
    /// Maintains ring buffer of last 1000 samples. Uses VecDeque for O(1)
    /// eviction.
    ///
    /// Currently always succeeds. Future versions may enforce cardinality
    /// limits.
    pub fn record_query_executed(&self, duration_ms: u64) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.queries_executed.fetch_add(1, Ordering::Relaxed);

        // Poison-safe locking
        let mut times = match self.query_execution_times_ms.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "DbMetrics::query_execution_times",
                    "Mutex poisoned during query time recording, recovering data"
                );
                poison_err.into_inner()
            }
        };

        // Ring buffer: O(1) push_back + pop_front
        times.push_back(duration_ms);
        if times.len() > 1000 {
            times.pop_front();
        }

        Ok(())
    }

    /// Record query execution error
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_query_error(&self) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.query_errors.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get average query execution time in milliseconds
    ///
    /// Returns 0.0 if no queries recorded (not an error condition).
    pub fn avg_query_time_ms(&self) -> f64 {
        // Poison-safe locking
        let times = match self.query_execution_times_ms.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "DbMetrics::query_execution_times",
                    "Mutex poisoned during average calculation, recovering data"
                );
                poison_err.into_inner()
            }
        };

        if times.is_empty() {
            return 0.0;
        }

        times.iter().sum::<u64>() as f64 / times.len() as f64
    }

    /// Get P95 query execution time in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no queries recorded.
    pub fn p95_query_time_ms(&self) -> MetricsResult<u64> {
        // Poison-safe locking
        let times = match self.query_execution_times_ms.lock() {
            Ok(guard) => guard,
            Err(poison_err) => {
                tracing::warn!(
                    metric = "DbMetrics::query_execution_times",
                    "Mutex poisoned during percentile calculation, recovering data"
                );
                poison_err.into_inner()
            }
        };

        if times.is_empty() {
            return Err(MetricsError::EmptyData { metric: "query_time_p95" });
        }

        let mut sorted: Vec<u64> = times.iter().copied().collect();
        sorted.sort_unstable();

        // Corrected percentile formula
        let index = (sorted.len() * 95).div_ceil(100).saturating_sub(1);
        Ok(sorted[index.min(sorted.len() - 1)])
    }

    // ========================================================================
    // Pool Utilization
    // ========================================================================

    /// Set the total number of connections in the pool (call during
    /// initialization)
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn set_pool_size(&self, size: u64) -> MetricsResult<()> {
        // Relaxed OK: set once during initialization
        self.total_connections_in_pool.store(size, Ordering::Relaxed);
        Ok(())
    }

    /// Update peak concurrent connections (should be called periodically)
    ///
    /// Uses compare-exchange loop to atomically update peak value.
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn update_peak_concurrent_connections(&self, current: u64) -> MetricsResult<()> {
        // SeqCst for derived metric (pool utilization calculation)
        let mut peak = self.peak_concurrent_connections.load(Ordering::SeqCst);
        while current > peak {
            match self.peak_concurrent_connections.compare_exchange(
                peak,
                current,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
        Ok(())
    }

    /// Get current pool utilization as percentage (0.0 - 1.0)
    ///
    /// Returns 0.0 if pool size is 0 (not an error condition).
    pub fn pool_utilization(&self) -> f64 {
        // SeqCst for consistency with update_peak_concurrent_connections
        let peak = self.peak_concurrent_connections.load(Ordering::SeqCst);
        let total = self.total_connections_in_pool.load(Ordering::Relaxed);

        if total == 0 {
            return 0.0;
        }

        peak as f64 / total as f64
    }

    // ========================================================================
    // Dual-Path Fallback Tracking (Phase 3 - migration monitoring)
    // ========================================================================

    /// Record successful database manager operation
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_dbmanager_success(&self) -> MetricsResult<()> {
        // SeqCst for derived metric (fallback rate calculation)
        self.dbmanager_successes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Record fallback to legacy LocalDatabase singleton
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_localdatabase_fallback(&self) -> MetricsResult<()> {
        // SeqCst for derived metric (fallback rate calculation)
        self.localdatabase_fallbacks.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Get fallback rate (0.0 = no fallbacks, 1.0 = all fallbacks)
    ///
    /// Returns 0.0 if no operations recorded (not an error condition).
    pub fn fallback_rate(&self) -> f64 {
        // SeqCst for consistency with record methods
        let successes = self.dbmanager_successes.load(Ordering::SeqCst);
        let fallbacks = self.localdatabase_fallbacks.load(Ordering::SeqCst);
        let total = successes + fallbacks;

        if total == 0 {
            return 0.0;
        }

        fallbacks as f64 / total as f64
    }

    // ========================================================================
    // Statistics Summary
    // ========================================================================

    /// Get comprehensive database metrics snapshot
    ///
    /// Percentile calculations return `Err(MetricsError::EmptyData)` if no data
    /// recorded.
    pub fn stats(&self) -> DbStats {
        DbStats {
            // Connection metrics
            connections_acquired: self.connections_acquired.load(Ordering::Relaxed),
            connection_timeouts: self.connection_timeouts.load(Ordering::Relaxed),
            connection_errors: self.connection_errors.load(Ordering::Relaxed),
            p50_connection_time_ms: self.p50_connection_time_ms().ok(),
            p95_connection_time_ms: self.p95_connection_time_ms().ok(),
            p99_connection_time_ms: self.p99_connection_time_ms().ok(),

            // Query metrics
            queries_executed: self.queries_executed.load(Ordering::Relaxed),
            query_errors: self.query_errors.load(Ordering::Relaxed),
            avg_query_time_ms: self.avg_query_time_ms(),
            p95_query_time_ms: self.p95_query_time_ms().ok(),

            // Pool metrics
            peak_concurrent_connections: self.peak_concurrent_connections.load(Ordering::SeqCst),
            total_connections_in_pool: self.total_connections_in_pool.load(Ordering::Relaxed),
            pool_utilization: self.pool_utilization(),

            // Fallback metrics
            dbmanager_successes: self.dbmanager_successes.load(Ordering::SeqCst),
            localdatabase_fallbacks: self.localdatabase_fallbacks.load(Ordering::SeqCst),
            fallback_rate: self.fallback_rate(),
        }
    }

    /// Reset metrics (useful for benchmarking)
    ///
    /// Note: Does not reset `total_connections_in_pool` as it's set during
    /// initialization.
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn reset(&self) -> MetricsResult<()> {
        // Clear connection metrics
        self.connections_acquired.store(0, Ordering::Relaxed);
        {
            let mut times = match self.connection_acquisition_times_ms.lock() {
                Ok(guard) => guard,
                Err(poison_err) => {
                    tracing::warn!(
                        metric = "DbMetrics::connection_acquisition_times",
                        "Mutex poisoned during reset, recovering data"
                    );
                    poison_err.into_inner()
                }
            };
            times.clear();
        }
        self.connection_timeouts.store(0, Ordering::Relaxed);
        self.connection_errors.store(0, Ordering::Relaxed);

        // Clear query metrics
        self.queries_executed.store(0, Ordering::Relaxed);
        {
            let mut times = match self.query_execution_times_ms.lock() {
                Ok(guard) => guard,
                Err(poison_err) => {
                    tracing::warn!(
                        metric = "DbMetrics::query_execution_times",
                        "Mutex poisoned during reset, recovering data"
                    );
                    poison_err.into_inner()
                }
            };
            times.clear();
        }
        self.query_errors.store(0, Ordering::Relaxed);

        // Clear pool utilization
        self.peak_concurrent_connections.store(0, Ordering::SeqCst);

        // Clear fallback tracking
        self.dbmanager_successes.store(0, Ordering::SeqCst);
        self.localdatabase_fallbacks.store(0, Ordering::SeqCst);

        // Clear timing
        self.first_connection_epoch_ms.store(0, Ordering::Relaxed);

        Ok(())
    }
}

/// Database metrics statistics snapshot
///
/// Percentile fields are `Option<u64>` to handle empty data case gracefully.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbStats {
    // Connection metrics
    pub connections_acquired: u64,
    pub connection_timeouts: u64,
    pub connection_errors: u64,
    pub p50_connection_time_ms: Option<u64>,
    pub p95_connection_time_ms: Option<u64>,
    pub p99_connection_time_ms: Option<u64>,

    // Query metrics
    pub queries_executed: u64,
    pub query_errors: u64,
    pub avg_query_time_ms: f64,
    pub p95_query_time_ms: Option<u64>,

    // Pool metrics
    pub peak_concurrent_connections: u64,
    pub total_connections_in_pool: u64,
    pub pool_utilization: f64,

    // Fallback metrics
    pub dbmanager_successes: u64,
    pub localdatabase_fallbacks: u64,
    pub fallback_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_acquisition_metrics() {
        let metrics = DbMetrics::new();

        // Record some connection acquisitions
        metrics.record_connection_acquired(10).unwrap();
        metrics.record_connection_acquired(20).unwrap();
        metrics.record_connection_acquired(30).unwrap();

        assert_eq!(metrics.connections_acquired.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.p50_connection_time_ms().unwrap(), 20);
        assert_eq!(metrics.p95_connection_time_ms().unwrap(), 30);
        assert_eq!(metrics.p99_connection_time_ms().unwrap(), 30);
    }

    #[test]
    fn test_connection_errors_and_timeouts() {
        let metrics = DbMetrics::new();

        metrics.record_connection_timeout().unwrap();
        metrics.record_connection_error().unwrap();
        metrics.record_connection_error().unwrap();

        assert_eq!(metrics.connection_timeouts.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.connection_errors.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_query_execution_metrics() {
        let metrics = DbMetrics::new();

        metrics.record_query_executed(5).unwrap();
        metrics.record_query_executed(10).unwrap();
        metrics.record_query_executed(15).unwrap();

        assert_eq!(metrics.queries_executed.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.avg_query_time_ms(), 10.0);
        assert_eq!(metrics.p95_query_time_ms().unwrap(), 15);
    }

    #[test]
    fn test_pool_utilization() {
        let metrics = DbMetrics::new();

        metrics.set_pool_size(10).unwrap();
        assert_eq!(metrics.pool_utilization(), 0.0);

        metrics.update_peak_concurrent_connections(5).unwrap();
        assert_eq!(metrics.pool_utilization(), 0.5);

        metrics.update_peak_concurrent_connections(8).unwrap();
        assert_eq!(metrics.pool_utilization(), 0.8);

        // Shouldn't decrease
        metrics.update_peak_concurrent_connections(3).unwrap();
        assert_eq!(metrics.pool_utilization(), 0.8);
    }

    #[test]
    fn test_fallback_tracking() {
        let metrics = DbMetrics::new();

        // All DbManager successes
        metrics.record_dbmanager_success().unwrap();
        metrics.record_dbmanager_success().unwrap();
        metrics.record_dbmanager_success().unwrap();
        assert_eq!(metrics.fallback_rate(), 0.0);

        // Add one fallback
        metrics.record_localdatabase_fallback().unwrap();
        assert_eq!(metrics.fallback_rate(), 0.25); // 1 / 4 = 0.25

        // Add more fallbacks
        metrics.record_localdatabase_fallback().unwrap();
        metrics.record_localdatabase_fallback().unwrap();
        assert_eq!(metrics.fallback_rate(), 0.5); // 3 / 6 = 0.5
    }

    #[test]
    fn test_reset() {
        let metrics = DbMetrics::new();

        // Set some values
        metrics.record_connection_acquired(10).unwrap();
        metrics.record_query_executed(5).unwrap();
        metrics.set_pool_size(10).unwrap();
        metrics.update_peak_concurrent_connections(5).unwrap();
        metrics.record_dbmanager_success().unwrap();
        metrics.record_localdatabase_fallback().unwrap();

        // Reset
        metrics.reset().unwrap();

        // Verify all cleared except pool size (set during init, not reset)
        assert_eq!(metrics.connections_acquired.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.queries_executed.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.peak_concurrent_connections.load(Ordering::SeqCst), 0);
        assert_eq!(metrics.dbmanager_successes.load(Ordering::SeqCst), 0);
        assert_eq!(metrics.localdatabase_fallbacks.load(Ordering::SeqCst), 0);
        assert_eq!(metrics.total_connections_in_pool.load(Ordering::Relaxed), 10);
        // Not reset
    }

    #[test]
    fn test_get_stats() {
        let metrics = DbMetrics::new();

        metrics.set_pool_size(10).unwrap();
        metrics.record_connection_acquired(10).unwrap();
        metrics.record_connection_acquired(20).unwrap();
        metrics.record_query_executed(5).unwrap();
        metrics.update_peak_concurrent_connections(3).unwrap();
        metrics.record_dbmanager_success().unwrap();
        metrics.record_dbmanager_success().unwrap();
        metrics.record_localdatabase_fallback().unwrap();

        let stats = metrics.stats();

        assert_eq!(stats.connections_acquired, 2);
        assert_eq!(stats.queries_executed, 1);
        assert_eq!(stats.peak_concurrent_connections, 3);
        assert_eq!(stats.total_connections_in_pool, 10);
        assert_eq!(stats.pool_utilization, 0.3);
        assert_eq!(stats.dbmanager_successes, 2);
        assert_eq!(stats.localdatabase_fallbacks, 1);
        assert!((stats.fallback_rate - 0.333).abs() < 0.01); // Approximately
                                                             // 1/3
    }

    #[test]
    fn test_empty_data_returns_error() {
        let metrics = DbMetrics::new();

        // Percentiles should error on empty data
        assert!(matches!(metrics.p50_connection_time_ms(), Err(MetricsError::EmptyData { .. })));
        assert!(matches!(metrics.p95_connection_time_ms(), Err(MetricsError::EmptyData { .. })));
        assert!(matches!(metrics.p99_connection_time_ms(), Err(MetricsError::EmptyData { .. })));
        assert!(matches!(metrics.p95_query_time_ms(), Err(MetricsError::EmptyData { .. })));

        // Average should return 0.0 (not an error)
        assert_eq!(metrics.avg_query_time_ms(), 0.0);

        // Rates should return 0.0 (not an error)
        assert_eq!(metrics.pool_utilization(), 0.0);
        assert_eq!(metrics.fallback_rate(), 0.0);
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let metrics = DbMetrics::new();

        // Add 1001 samples to trigger eviction
        for i in 0..=1000 {
            metrics.record_connection_acquired(i).unwrap();
        }

        // Should only keep last 1000
        let times = metrics.connection_acquisition_times_ms.lock().unwrap();
        assert_eq!(times.len(), 1000);
        // First element should be 1 (0 was evicted)
        assert_eq!(times[0], 1);
        // Last element should be 1000
        assert_eq!(times[999], 1000);
    }

    #[test]
    fn test_first_connection_time_cas() {
        let metrics = DbMetrics::new();

        // First connection should set TTFD via CAS
        metrics.record_connection_acquired(123).unwrap();
        assert_eq!(metrics.first_connection_epoch_ms.load(Ordering::Relaxed), 123);

        // Subsequent connections should not change TTFD (CAS fails)
        metrics.record_connection_acquired(456).unwrap();
        assert_eq!(metrics.first_connection_epoch_ms.load(Ordering::Relaxed), 123);
    }

    #[test]
    fn test_concurrent_first_connection() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(DbMetrics::new());
        let mut handles = vec![];

        // Spawn 10 threads, each trying to record a "first" connection
        for i in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            handles.push(thread::spawn(move || {
                metrics_clone.record_connection_acquired(100 + i).unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify only one value was set (could be any of 100-109)
        let first_time = metrics.first_connection_epoch_ms.load(Ordering::Relaxed);
        assert!((100..110).contains(&first_time));

        // Verify all 10 connections were recorded
        assert_eq!(metrics.connections_acquired.load(Ordering::Relaxed), 10);
    }
}
