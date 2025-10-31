//! Performance metrics aggregator with hierarchical structure
//!
//! This module provides the main `PerformanceMetrics` struct which organizes
//! metrics into logical categories for better maintainability and clarity.
//!
//! ## Design
//! - **Aggregation pattern** - Holds all individual metrics types
//! - **Convenience methods** - Delegates to underlying metrics for common operations
//! - **Thread-safe** - All underlying metrics use atomics/locks appropriately
//!
//! ## Usage
//!
//! ```rust
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//! use std::time::Duration;
//!
//! let metrics = PerformanceMetrics::new();
//!
//! // Record API call
//! metrics.record_call().unwrap();
//!
//! // Record fetch timing
//! metrics.record_fetch_time(Duration::from_millis(123)).unwrap();
//!
//! // Record cache hit/miss
//! metrics.record_cache_hit().unwrap();
//! metrics.record_cache_miss().unwrap();
//!
//! // Get cache hit rate (0-100)
//! let hit_rate_pct = metrics.cache_hit_rate_pct();
//! ```

use std::time::Duration;

use super::{CacheMetrics, CallMetrics, DbMetrics, FetchMetrics, ObserverMetrics};
use crate::observability::MetricsResult;

/// Performance metrics for tracking infrastructure operations
///
/// Aggregates all individual metric types and provides convenience methods
/// for common operations.
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// API call metrics (counts, TTFD, timing)
    pub call: CallMetrics,
    /// Cache performance metrics (hits, misses, hit rate)
    pub cache: CacheMetrics,
    /// Database connection pool metrics (connections, queries, pool utilization)
    pub db: DbMetrics,
    /// HTTP fetch metrics (timing, errors, timeouts)
    pub fetch: FetchMetrics,
    /// macOS Accessibility API observer metrics (notifications, registration)
    pub observer: ObserverMetrics,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    /// Create new PerformanceMetrics instance
    ///
    /// All individual metrics are initialized to zero/empty state.
    pub fn new() -> Self {
        Self {
            call: CallMetrics::new(),
            cache: CacheMetrics::new(),
            db: DbMetrics::new(),
            fetch: FetchMetrics::new(),
            observer: ObserverMetrics::new(),
        }
    }

    // ========================================================================
    // Convenience Methods - Call Metrics
    // ========================================================================

    /// Record an API call
    ///
    /// Updates total call count and start time for rate calculations.
    pub fn record_call(&self) -> MetricsResult<()> {
        self.call.record_call()
    }

    /// Get time to first data (TTFD) in milliseconds
    ///
    /// Returns 0 if no calls have been made yet.
    pub fn ttfd_ms(&self) -> u64 {
        self.call.get_ttfd_ms()
    }

    /// Get calls per minute rate
    ///
    /// Returns 0.0 if no time has elapsed since first call.
    pub fn calls_per_minute(&self) -> f64 {
        self.call.get_calls_per_minute()
    }

    /// Get P50 (median) fetch time in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no fetch times recorded.
    pub fn p50_fetch_time_ms(&self) -> MetricsResult<u64> {
        self.call.get_p50_fetch_time_ms()
    }

    /// Get P95 fetch time in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no fetch times recorded.
    pub fn p95_fetch_time_ms(&self) -> MetricsResult<u64> {
        self.call.get_p95_fetch_time_ms()
    }

    /// Get P99 fetch time in milliseconds
    ///
    /// Returns `Err(MetricsError::EmptyData)` if no fetch times recorded.
    pub fn p99_fetch_time_ms(&self) -> MetricsResult<u64> {
        self.call.get_p99_fetch_time_ms()
    }

    // ========================================================================
    // Convenience Methods - Cache Metrics
    // ========================================================================

    /// Record a cache hit
    pub fn record_cache_hit(&self) -> MetricsResult<()> {
        self.cache.record_hit()
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) -> MetricsResult<()> {
        self.cache.record_miss()
    }

    /// Get cache hit rate as percentage (0.0 - 100.0)
    ///
    /// Returns 0.0 if no cache operations recorded.
    pub fn cache_hit_rate_pct(&self) -> f64 {
        self.cache.get_hit_rate()
    }

    // ========================================================================
    // Convenience Methods - Fetch Metrics
    // ========================================================================

    /// Record fetch timing
    ///
    /// Updates both FetchMetrics and CallMetrics (for percentile calculations).
    pub fn record_fetch_time(&self, duration: Duration) -> MetricsResult<()> {
        self.fetch.record_fetch_time(duration)?;
        self.call.record_fetch_time(duration)?;
        Ok(())
    }

    /// Record fetch error
    pub fn record_fetch_error(&self) -> MetricsResult<()> {
        self.fetch.record_error()
    }

    /// Record fetch timeout
    pub fn record_fetch_timeout(&self) -> MetricsResult<()> {
        self.fetch.record_timeout()
    }

    /// Get average fetch time in milliseconds
    ///
    /// Returns 0.0 if no fetches recorded.
    pub fn avg_fetch_time_ms(&self) -> f64 {
        self.fetch.get_avg_fetch_time_ms()
    }

    /// Get timeout count
    pub fn timeout_count(&self) -> usize {
        self.fetch.get_timeout_count()
    }

    // ========================================================================
    // Convenience Methods - Database Metrics
    // ========================================================================

    /// Record successful database connection acquisition
    pub fn record_db_connection_acquired(&self, duration_ms: u64) -> MetricsResult<()> {
        self.db.record_connection_acquired(duration_ms)
    }

    /// Record database connection timeout
    pub fn record_db_connection_timeout(&self) -> MetricsResult<()> {
        self.db.record_connection_timeout()
    }

    /// Record database connection error
    pub fn record_db_connection_error(&self) -> MetricsResult<()> {
        self.db.record_connection_error()
    }

    /// Record database query execution
    pub fn record_db_query_executed(&self, duration_ms: u64) -> MetricsResult<()> {
        self.db.record_query_executed(duration_ms)
    }

    /// Record database query error
    pub fn record_db_query_error(&self) -> MetricsResult<()> {
        self.db.record_query_error()
    }

    /// Set database pool size (call during initialization)
    pub fn set_db_pool_size(&self, size: u64) -> MetricsResult<()> {
        self.db.set_pool_size(size)
    }

    /// Update peak concurrent database connections
    pub fn update_db_peak_connections(&self, current: u64) -> MetricsResult<()> {
        self.db.update_peak_concurrent_connections(current)
    }

    /// Get database pool utilization (0.0 - 1.0)
    ///
    /// Returns 0.0 if pool size is 0.
    pub fn db_pool_utilization(&self) -> f64 {
        self.db.pool_utilization()
    }

    // ========================================================================
    // Convenience Methods - Observer Metrics
    // ========================================================================

    /// Record macOS Accessibility API observer notification
    pub fn record_observer_notification(&self) -> MetricsResult<()> {
        self.observer.record_notification()
    }

    /// Record observer registration time in milliseconds
    pub fn record_observer_registration_time(&self, ms: u64) -> MetricsResult<()> {
        self.observer.record_registration_time(ms)
    }

    /// Record observer cleanup time in milliseconds
    pub fn record_observer_cleanup_time(&self, ms: u64) -> MetricsResult<()> {
        self.observer.record_cleanup_time(ms)
    }

    /// Record Accessibility API permission status
    pub fn record_ax_permission_status(&self, granted: bool) -> MetricsResult<()> {
        self.observer.record_ax_permission_status(granted)
    }

    /// Record observer initialization failure
    pub fn record_observer_failure(&self, error: &str) -> MetricsResult<()> {
        self.observer.record_failure(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_default() {
        let metrics1 = PerformanceMetrics::new();
        let metrics2 = PerformanceMetrics::default();

        // Verify initial state
        assert_eq!(metrics1.ttfd_ms(), 0);
        assert_eq!(metrics1.cache_hit_rate_pct(), 0.0);
        assert_eq!(metrics1.avg_fetch_time_ms(), 0.0);
        assert_eq!(metrics1.db_pool_utilization(), 0.0);

        assert_eq!(metrics2.ttfd_ms(), 0);
        assert_eq!(metrics2.cache_hit_rate_pct(), 0.0);
    }

    #[test]
    fn test_call_metrics_delegation() {
        let metrics = PerformanceMetrics::new();

        // Record calls
        metrics.record_call().unwrap();
        metrics.record_call().unwrap();
        metrics.record_call().unwrap();

        // Verify calls per minute is calculated
        let cpm = metrics.calls_per_minute();
        assert!(cpm > 0.0);
    }

    #[test]
    fn test_cache_metrics_delegation() {
        let metrics = PerformanceMetrics::new();

        // Record cache operations
        metrics.record_cache_hit().unwrap();
        metrics.record_cache_hit().unwrap();
        metrics.record_cache_miss().unwrap();

        // Verify hit rate (percentage 0-100)
        let hit_rate_pct = metrics.cache_hit_rate_pct();
        assert!((hit_rate_pct - 66.666).abs() < 0.1); // 2/3 * 100 ≈ 66.666%
    }

    #[test]
    fn test_fetch_metrics_delegation() {
        let metrics = PerformanceMetrics::new();

        // Record fetches
        metrics
            .record_fetch_time(Duration::from_millis(100))
            .unwrap();
        metrics
            .record_fetch_time(Duration::from_millis(200))
            .unwrap();
        metrics
            .record_fetch_time(Duration::from_millis(300))
            .unwrap();

        // Verify average
        assert_eq!(metrics.avg_fetch_time_ms(), 200.0);

        // Verify percentiles (also recorded in call metrics)
        assert_eq!(metrics.p50_fetch_time_ms().unwrap(), 200);
        assert_eq!(metrics.p95_fetch_time_ms().unwrap(), 300);

        // Record errors/timeouts
        metrics.record_fetch_error().unwrap();
        metrics.record_fetch_timeout().unwrap();
        metrics.record_fetch_timeout().unwrap();

        assert_eq!(metrics.timeout_count(), 2);
    }

    #[test]
    fn test_db_metrics_delegation() {
        let metrics = PerformanceMetrics::new();

        // Set pool size
        metrics.set_db_pool_size(10).unwrap();

        // Record connections
        metrics.record_db_connection_acquired(10).unwrap();
        metrics.record_db_connection_acquired(20).unwrap();
        metrics.record_db_connection_timeout().unwrap();
        metrics.record_db_connection_error().unwrap();

        // Record queries
        metrics.record_db_query_executed(5).unwrap();
        metrics.record_db_query_executed(15).unwrap();
        metrics.record_db_query_error().unwrap();

        // Update pool utilization
        metrics.update_db_peak_connections(7).unwrap();
        assert_eq!(metrics.db_pool_utilization(), 0.7);

        // Verify db stats via underlying metrics
        let db_stats = metrics.db.stats();
        assert_eq!(db_stats.connections_acquired, 2);
        assert_eq!(db_stats.connection_timeouts, 1);
        assert_eq!(db_stats.connection_errors, 1);
        assert_eq!(db_stats.queries_executed, 2);
        assert_eq!(db_stats.query_errors, 1);
    }

    #[test]
    fn test_observer_metrics_delegation() {
        let metrics = PerformanceMetrics::new();

        // Record observer activity
        metrics.record_observer_notification().unwrap();
        metrics.record_observer_notification().unwrap();
        metrics.record_observer_registration_time(150).unwrap();
        metrics.record_observer_cleanup_time(50).unwrap();
        metrics.record_ax_permission_status(true).unwrap();
        metrics.record_observer_failure("test error").unwrap();

        // Verify via underlying metrics
        let observer_stats = metrics.observer.stats();
        assert_eq!(observer_stats.notifications_received, 2);
        assert_eq!(observer_stats.registration_time_ms, 150);
        assert_eq!(observer_stats.cleanup_time_ms, 50);
        assert!(observer_stats.ax_permission_granted);
        assert_eq!(observer_stats.failures, 1);
    }

    #[test]
    fn test_integrated_workflow() {
        let metrics = PerformanceMetrics::new();

        // Simulate typical API call workflow
        metrics.record_call().unwrap();

        // Check cache (miss)
        metrics.record_cache_miss().unwrap();

        // Fetch from upstream
        metrics
            .record_fetch_time(Duration::from_millis(250))
            .unwrap();

        // Check cache again (hit)
        metrics.record_call().unwrap();
        metrics.record_cache_hit().unwrap();

        // Database operation
        metrics.set_db_pool_size(5).unwrap();
        metrics.record_db_connection_acquired(15).unwrap();
        metrics.record_db_query_executed(10).unwrap();

        // Observer notification
        metrics.record_observer_notification().unwrap();

        // Verify integrated state
        assert!((metrics.cache_hit_rate_pct() - 50.0).abs() < 0.1); // 1 hit, 1 miss = 50%
        assert_eq!(metrics.avg_fetch_time_ms(), 250.0);
        assert!(metrics.calls_per_minute() > 0.0);

        let db_stats = metrics.db.stats();
        assert_eq!(db_stats.connections_acquired, 1);
        assert_eq!(db_stats.queries_executed, 1);

        let observer_stats = metrics.observer.stats();
        assert_eq!(observer_stats.notifications_received, 1);
    }
}
