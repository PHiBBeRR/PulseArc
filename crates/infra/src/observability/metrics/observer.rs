//! Observer-related metrics for tracking macOS Accessibility API observer
//! behavior
//!
//! This module tracks metrics related to OS-level observers including
//! notifications, registration timing, and permission status for macOS
//! Accessibility API integration.
//!
//! ## Design
//! - **Simple atomic counters** - No locking needed
//! - **Relaxed ordering** - Independent counters, no derived metrics
//! - **MetricsResult returns** for future extensibility (currently always Ok)

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::observability::MetricsResult;

/// Snapshot of observer statistics at a point in time
///
/// Provides a consistent view of all observer metrics without holding locks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ObserverStats {
    /// Total observer notifications received
    pub notifications_received: u64,
    /// Observer registration time in milliseconds
    pub registration_time_ms: u64,
    /// Observer cleanup time in milliseconds
    pub cleanup_time_ms: u64,
    /// Whether Accessibility API permission was granted
    pub ax_permission_granted: bool,
    /// Number of observer initialization failures
    pub failures: u64,
}

/// Metrics for tracking macOS Accessibility API observer behavior
///
/// All record methods return `MetricsResult<()>` for future extensibility
/// (quotas, validation), but currently always succeed.
#[derive(Debug)]
pub struct ObserverMetrics {
    /// Number of observer notifications received
    observer_notifications_received: AtomicU64,
    /// Observer registration time in milliseconds
    observer_registration_time_ms: AtomicU64,
    /// Observer cleanup time in milliseconds
    observer_cleanup_time_ms: AtomicU64,
    /// Whether Accessibility API permission was granted
    ax_permission_granted: AtomicBool,
    /// Number of observer initialization failures
    observer_failures: AtomicU64,
}

impl Default for ObserverMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ObserverMetrics {
    /// Create new ObserverMetrics instance
    pub fn new() -> Self {
        Self {
            observer_notifications_received: AtomicU64::new(0),
            observer_registration_time_ms: AtomicU64::new(0),
            observer_cleanup_time_ms: AtomicU64::new(0),
            ax_permission_granted: AtomicBool::new(false),
            observer_failures: AtomicU64::new(0),
        }
    }

    /// Record an observer notification received
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_notification(&self) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.observer_notifications_received.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Record observer registration time in milliseconds
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn record_registration_time(&self, ms: u64) -> MetricsResult<()> {
        // Relaxed OK: independent value
        self.observer_registration_time_ms.store(ms, Ordering::Relaxed);
        Ok(())
    }

    /// Record observer cleanup time in milliseconds
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn record_cleanup_time(&self, ms: u64) -> MetricsResult<()> {
        // Relaxed OK: independent value
        self.observer_cleanup_time_ms.store(ms, Ordering::Relaxed);
        Ok(())
    }

    /// Record Accessibility API permission status
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn record_ax_permission_status(&self, granted: bool) -> MetricsResult<()> {
        // Relaxed OK: independent boolean
        self.ax_permission_granted.store(granted, Ordering::Relaxed);
        Ok(())
    }

    /// Record an observer initialization failure
    ///
    /// The error message is logged via tracing but not stored in metrics.
    ///
    /// Currently always succeeds. Future versions may enforce quotas.
    pub fn record_failure(&self, error: &str) -> MetricsResult<()> {
        // Relaxed OK: independent counter
        self.observer_failures.fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            error = %error,
            total_failures = self.observer_failures.load(Ordering::Relaxed),
            "Observer initialization failure"
        );

        Ok(())
    }

    /// Get observer statistics as a structured snapshot
    ///
    /// Provides a consistent view of all metrics at a point in time.
    pub fn stats(&self) -> ObserverStats {
        ObserverStats {
            notifications_received: self.observer_notifications_received.load(Ordering::Relaxed),
            registration_time_ms: self.observer_registration_time_ms.load(Ordering::Relaxed),
            cleanup_time_ms: self.observer_cleanup_time_ms.load(Ordering::Relaxed),
            ax_permission_granted: self.ax_permission_granted.load(Ordering::Relaxed),
            failures: self.observer_failures.load(Ordering::Relaxed),
        }
    }

    /// Reset observer counters to zero
    ///
    /// Useful for benchmarking or testing. Resets all metrics to initial state.
    ///
    /// Currently always succeeds. Future versions may enforce validation.
    pub fn reset(&self) -> MetricsResult<()> {
        self.observer_notifications_received.store(0, Ordering::Relaxed);
        self.observer_registration_time_ms.store(0, Ordering::Relaxed);
        self.observer_cleanup_time_ms.store(0, Ordering::Relaxed);
        self.ax_permission_granted.store(false, Ordering::Relaxed);
        self.observer_failures.store(0, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_notification() {
        let metrics = ObserverMetrics::new();
        assert_eq!(metrics.observer_notifications_received.load(Ordering::Relaxed), 0);

        metrics.record_notification().unwrap();
        assert_eq!(metrics.observer_notifications_received.load(Ordering::Relaxed), 1);

        metrics.record_notification().unwrap();
        metrics.record_notification().unwrap();
        assert_eq!(metrics.observer_notifications_received.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_record_registration_time() {
        let metrics = ObserverMetrics::new();
        assert_eq!(metrics.observer_registration_time_ms.load(Ordering::Relaxed), 0);

        metrics.record_registration_time(150).unwrap();
        assert_eq!(metrics.observer_registration_time_ms.load(Ordering::Relaxed), 150);

        // Should overwrite
        metrics.record_registration_time(200).unwrap();
        assert_eq!(metrics.observer_registration_time_ms.load(Ordering::Relaxed), 200);
    }

    #[test]
    fn test_record_cleanup_time() {
        let metrics = ObserverMetrics::new();
        assert_eq!(metrics.observer_cleanup_time_ms.load(Ordering::Relaxed), 0);

        metrics.record_cleanup_time(50).unwrap();
        assert_eq!(metrics.observer_cleanup_time_ms.load(Ordering::Relaxed), 50);

        metrics.record_cleanup_time(75).unwrap();
        assert_eq!(metrics.observer_cleanup_time_ms.load(Ordering::Relaxed), 75);
    }

    #[test]
    fn test_record_ax_permission_status() {
        let metrics = ObserverMetrics::new();
        assert!(!metrics.ax_permission_granted.load(Ordering::Relaxed));

        metrics.record_ax_permission_status(true).unwrap();
        assert!(metrics.ax_permission_granted.load(Ordering::Relaxed));

        metrics.record_ax_permission_status(false).unwrap();
        assert!(!metrics.ax_permission_granted.load(Ordering::Relaxed));
    }

    #[test]
    fn test_record_failure() {
        let metrics = ObserverMetrics::new();
        assert_eq!(metrics.observer_failures.load(Ordering::Relaxed), 0);

        metrics.record_failure("test error 1").unwrap();
        assert_eq!(metrics.observer_failures.load(Ordering::Relaxed), 1);

        metrics.record_failure("test error 2").unwrap();
        metrics.record_failure("test error 3").unwrap();
        assert_eq!(metrics.observer_failures.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_get_stats() {
        let metrics = ObserverMetrics::new();

        metrics.record_notification().unwrap();
        metrics.record_notification().unwrap();
        metrics.record_registration_time(150).unwrap();
        metrics.record_cleanup_time(50).unwrap();
        metrics.record_ax_permission_status(true).unwrap();
        metrics.record_failure("test error").unwrap();

        let stats = metrics.stats();

        assert_eq!(stats.notifications_received, 2);
        assert_eq!(stats.registration_time_ms, 150);
        assert_eq!(stats.cleanup_time_ms, 50);
        assert!(stats.ax_permission_granted);
        assert_eq!(stats.failures, 1);
    }

    #[test]
    fn test_reset() {
        let metrics = ObserverMetrics::new();

        // Set some values
        metrics.record_notification().unwrap();
        metrics.record_registration_time(150).unwrap();
        metrics.record_cleanup_time(50).unwrap();
        metrics.record_ax_permission_status(true).unwrap();
        metrics.record_failure("test").unwrap();

        assert_eq!(metrics.observer_notifications_received.load(Ordering::Relaxed), 1);

        // Reset
        metrics.reset().unwrap();

        assert_eq!(metrics.observer_notifications_received.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.observer_registration_time_ms.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.observer_cleanup_time_ms.load(Ordering::Relaxed), 0);
        assert!(!metrics.ax_permission_granted.load(Ordering::Relaxed));
        assert_eq!(metrics.observer_failures.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_default() {
        let metrics = ObserverMetrics::default();
        let stats = metrics.stats();

        assert_eq!(stats.notifications_received, 0);
        assert_eq!(stats.registration_time_ms, 0);
        assert_eq!(stats.cleanup_time_ms, 0);
        assert!(!stats.ax_permission_granted);
        assert_eq!(stats.failures, 0);
    }
}
