//! Classification metrics tracking
//!
//! Tracks classification performance to measure ML classifier coverage and
//! performance. Ported from macos-production/src-tauri/src/inference/metrics.rs

use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

/// Metrics for classification performance
///
/// Tracks which classifiers are used and performance metrics.
///
/// # Example
/// ```
/// use pulsearc_common::observability::metrics::classification::ClassificationMetrics;
///
/// let metrics = ClassificationMetrics::default();
/// println!("Linfa coverage: {}%", metrics.linfa_coverage_percent());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ClassificationMetrics {
    /// Number of blocks classified by linfa
    pub linfa_predictions: u64,
    /// Number of blocks that fell back to rules
    pub rules_fallbacks: u64,
    /// Average linfa prediction time in milliseconds
    pub avg_linfa_time_ms: f32,
    /// Total number of classifications
    pub total_predictions: u64,
}

impl ClassificationMetrics {
    /// Calculate linfa coverage percentage
    ///
    /// # Returns
    /// Percentage of blocks classified by linfa (0.0-100.0)
    pub fn linfa_coverage_percent(&self) -> f32 {
        if self.total_predictions == 0 {
            return 0.0;
        }
        (self.linfa_predictions as f32 / self.total_predictions as f32) * 100.0
    }

    /// Calculate rules fallback percentage
    pub fn rules_fallback_percent(&self) -> f32 {
        if self.total_predictions == 0 {
            return 0.0;
        }
        (self.rules_fallbacks as f32 / self.total_predictions as f32) * 100.0
    }
}

/// Thread-safe metrics tracker
///
/// Used by HybridClassifier to track classification performance.
///
/// # Example
/// ```no_run
/// use pulsearc_common::observability::metrics::classification::MetricsTracker;
///
/// let tracker = MetricsTracker::new();
/// tracker.record_linfa_prediction(0.5); // 0.5ms
/// let metrics = tracker.get_metrics();
/// println!("Linfa predictions: {}", metrics.linfa_predictions);
/// ```
#[derive(Debug)]
pub struct MetricsTracker {
    metrics: Arc<Mutex<ClassificationMetrics>>,
}

impl MetricsTracker {
    /// Create new metrics tracker with zero initial state
    pub fn new() -> Self {
        Self { metrics: Arc::new(Mutex::new(ClassificationMetrics::default())) }
    }

    /// Record a successful linfa prediction
    ///
    /// # Arguments
    /// * `time_ms` - Time taken for prediction in milliseconds
    pub fn record_linfa_prediction(&self, time_ms: f32) {
        let mut guard = self.lock_metrics();
        guard.linfa_predictions += 1;
        guard.total_predictions += 1;

        // Running average: avg_new = (avg_old * count_old + new_value) / count_new
        let count = guard.linfa_predictions as f32;
        guard.avg_linfa_time_ms = ((guard.avg_linfa_time_ms * (count - 1.0)) + time_ms) / count;
    }

    /// Record a fallback to rules classifier
    pub fn record_rules_fallback(&self) {
        let mut guard = self.lock_metrics();
        guard.rules_fallbacks += 1;
        guard.total_predictions += 1;
    }

    /// Get current metrics snapshot
    ///
    /// # Returns
    /// Copy of current metrics (safe for cross-thread access)
    pub fn get_metrics(&self) -> ClassificationMetrics {
        let guard = self.lock_metrics();
        guard.clone()
    }

    /// Reset all metrics to zero
    pub fn reset(&self) {
        let mut guard = self.lock_metrics();
        *guard = ClassificationMetrics::default();
    }

    fn lock_metrics(&self) -> MutexGuard<'_, ClassificationMetrics> {
        self.metrics.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for MetricsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MetricsTracker {
    fn clone(&self) -> Self {
        Self { metrics: Arc::clone(&self.metrics) }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for observability::metrics::classification.
    use super::*;

    /// Validates `ClassificationMetrics::default` behavior for the metrics
    /// initialization scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.linfa_predictions` equals `0`.
    /// - Confirms `metrics.rules_fallbacks` equals `0`.
    /// - Confirms `metrics.total_predictions` equals `0`.
    /// - Confirms `metrics.avg_linfa_time_ms` equals `0.0`.
    #[test]
    fn test_metrics_initialization() {
        let metrics = ClassificationMetrics::default();
        assert_eq!(metrics.linfa_predictions, 0);
        assert_eq!(metrics.rules_fallbacks, 0);
        assert_eq!(metrics.total_predictions, 0);
        assert_eq!(metrics.avg_linfa_time_ms, 0.0);
    }

    /// Validates `MetricsTracker::new` behavior for the record linfa prediction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.linfa_predictions` equals `1`.
    /// - Confirms `metrics.total_predictions` equals `1`.
    /// - Confirms `metrics.avg_linfa_time_ms` equals `0.5`.
    /// - Confirms `metrics.rules_fallbacks` equals `0`.
    #[test]
    fn test_record_linfa_prediction() {
        let tracker = MetricsTracker::new();
        tracker.record_linfa_prediction(0.5);

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.linfa_predictions, 1);
        assert_eq!(metrics.total_predictions, 1);
        assert_eq!(metrics.avg_linfa_time_ms, 0.5);
        assert_eq!(metrics.rules_fallbacks, 0);
    }

    /// Validates `MetricsTracker::new` behavior for the record rules fallback
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.rules_fallbacks` equals `1`.
    /// - Confirms `metrics.total_predictions` equals `1`.
    /// - Confirms `metrics.linfa_predictions` equals `0`.
    #[test]
    fn test_record_rules_fallback() {
        let tracker = MetricsTracker::new();
        tracker.record_rules_fallback();

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.rules_fallbacks, 1);
        assert_eq!(metrics.total_predictions, 1);
        assert_eq!(metrics.linfa_predictions, 0);
    }

    /// Validates `MetricsTracker::new` behavior for the metrics reset scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.linfa_predictions` equals `0`.
    /// - Confirms `metrics.rules_fallbacks` equals `0`.
    /// - Confirms `metrics.total_predictions` equals `0`.
    /// - Confirms `metrics.avg_linfa_time_ms` equals `0.0`.
    #[test]
    fn test_metrics_reset() {
        let tracker = MetricsTracker::new();
        tracker.record_linfa_prediction(0.5);
        tracker.record_linfa_prediction(1.0);
        tracker.record_rules_fallback();

        tracker.reset();

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.linfa_predictions, 0);
        assert_eq!(metrics.rules_fallbacks, 0);
        assert_eq!(metrics.total_predictions, 0);
        assert_eq!(metrics.avg_linfa_time_ms, 0.0);
    }

    /// Validates `MetricsTracker::new` behavior for the avg time calculation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.linfa_predictions` equals `3`.
    /// - Ensures `(metrics.avg_linfa_time_ms - expected_avg).abs() < 0.01`
    ///   evaluates to true.
    #[test]
    fn test_avg_time_calculation() {
        let tracker = MetricsTracker::new();
        tracker.record_linfa_prediction(0.5);
        tracker.record_linfa_prediction(1.5);
        tracker.record_linfa_prediction(2.0);

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.linfa_predictions, 3);
        let expected_avg = (0.5 + 1.5 + 2.0) / 3.0;
        assert!((metrics.avg_linfa_time_ms - expected_avg).abs() < 0.01);
    }

    /// Validates `ClassificationMetrics::default` behavior for the linfa
    /// coverage calculation scenario.
    ///
    /// Assertions:
    /// - Confirms `coverage` equals `95.0`.
    /// - Confirms `empty_coverage` equals `0.0`.
    #[test]
    fn test_linfa_coverage_calculation() {
        let metrics = ClassificationMetrics {
            linfa_predictions: 95,
            rules_fallbacks: 5,
            avg_linfa_time_ms: 0.8,
            total_predictions: 100,
        };

        let coverage = metrics.linfa_coverage_percent();
        assert_eq!(coverage, 95.0);

        let empty_metrics = ClassificationMetrics::default();
        let empty_coverage = empty_metrics.linfa_coverage_percent();
        assert_eq!(empty_coverage, 0.0);
    }

    /// Validates `Arc::new` behavior for the thread safety scenario.
    ///
    /// Assertions:
    /// - Confirms `metrics.linfa_predictions` equals `100`.
    /// - Confirms `metrics.total_predictions` equals `100`.
    #[test]
    fn test_thread_safety() {
        use std::thread;

        let tracker = Arc::new(MetricsTracker::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let tracker_clone = Arc::clone(&tracker);
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    tracker_clone.record_linfa_prediction(0.5);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.linfa_predictions, 100);
        assert_eq!(metrics.total_predictions, 100);
    }

    /// Validates `MetricsTracker::new` behavior for the poison recovery
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Confirms `metrics.rules_fallbacks` equals `1`.
    #[test]
    fn test_poison_recovery() {
        use std::panic;

        let tracker = MetricsTracker::new();
        let tracker_for_panic = tracker.clone();
        let result = panic::catch_unwind(move || {
            let _lock = tracker_for_panic.metrics.lock().unwrap();
            panic!("force poison");
        });
        assert!(result.is_err());

        tracker.record_rules_fallback();
        let metrics = tracker.get_metrics();
        assert_eq!(metrics.rules_fallbacks, 1);
    }
}
