//! In-memory telemetry metrics for idle detection and activity synchronization
//!
//! FEATURE-012: Tracks idle detection events, timer state transitions, and
//! autoStartTracker rule validation for debugging and performance monitoring.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use pulsearc_common::resilience::{Histogram, HistogramSnapshot};

// Type aliases for complex types to satisfy clippy::type_complexity
type ActivityWakeMap = Arc<RwLock<HashMap<String, u64>>>;
type TransitionDurationMap = Arc<RwLock<HashMap<(String, String), Histogram>>>;
type RuleAccuracyMap = Arc<RwLock<HashMap<u8, (u64, u64)>>>;
type TransitionSnapshots = HashMap<(String, String), HistogramSnapshot>;
type RuleAccuracyData = HashMap<u8, (u64, u64)>;

/// In-memory telemetry metrics for idle sync operations
///
/// Thread-safe counters for tracking idle detection, activity wake events,
/// timer state events, and validation metrics. Metrics are stored in-memory
/// and reset on application restart.
#[derive(Debug)]
pub struct IdleSyncMetrics {
    // Idle detection metrics
    idle_detections: AtomicU64,
    idle_detection_latency: Histogram,
    activity_wakes: AtomicU64,
    activity_wake_types: ActivityWakeMap,

    // Timer event metrics
    timer_emissions: AtomicU64,
    timer_emissions_success: AtomicU64,
    timer_emissions_failed: AtomicU64,
    timer_emission_latency: Histogram,
    timer_receptions: AtomicU64,
    timer_sync_latency: Histogram,

    // Error tracking
    invalid_payloads: AtomicU64,

    // State transition tracking
    state_transitions: AtomicU64,
    transition_durations: TransitionDurationMap,

    // AutoStartTracker validation
    auto_start_tracker_rules: AtomicU64,
    auto_start_tracker_correct: AtomicU64,
    auto_start_tracker_incorrect: AtomicU64,
    rule_accuracy: RuleAccuracyMap, // (correct, incorrect) by rule number
}

impl Default for IdleSyncMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl IdleSyncMetrics {
    /// Create a new metrics instance with all counters at zero
    pub fn new() -> Self {
        Self {
            idle_detections: AtomicU64::new(0),
            idle_detection_latency: Histogram::new(),
            activity_wakes: AtomicU64::new(0),
            activity_wake_types: Arc::new(RwLock::new(HashMap::new())),
            timer_emissions: AtomicU64::new(0),
            timer_emissions_success: AtomicU64::new(0),
            timer_emissions_failed: AtomicU64::new(0),
            timer_emission_latency: Histogram::new(),
            timer_receptions: AtomicU64::new(0),
            timer_sync_latency: Histogram::new(),
            invalid_payloads: AtomicU64::new(0),
            state_transitions: AtomicU64::new(0),
            transition_durations: Arc::new(RwLock::new(HashMap::new())),
            auto_start_tracker_rules: AtomicU64::new(0),
            auto_start_tracker_correct: AtomicU64::new(0),
            auto_start_tracker_incorrect: AtomicU64::new(0),
            rule_accuracy: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record an idle detection event with latency measurement
    ///
    /// # Arguments
    /// * `latency_ms` - Time in milliseconds to detect idle state
    pub fn record_idle_detection(&self, latency_ms: u64) {
        self.idle_detections.fetch_add(1, Ordering::Relaxed);
        self.idle_detection_latency.record(std::time::Duration::from_millis(latency_ms));
    }

    /// Record an activity wake event
    ///
    /// # Arguments
    /// * `event_type` - Type of wake event (e.g., "mouse", "keyboard")
    pub fn record_activity_wake(&self, event_type: String) {
        self.activity_wakes.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut types) = self.activity_wake_types.write() {
            *types.entry(event_type).or_insert(0) += 1;
        }
    }

    /// Record a timer-state event emission
    ///
    /// # Arguments
    /// * `latency_us` - Emission latency in microseconds
    /// * `success` - Whether emission succeeded
    pub fn record_timer_event_emission(&self, latency_us: u64, success: bool) {
        self.timer_emissions.fetch_add(1, Ordering::Relaxed);
        if success {
            self.timer_emissions_success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.timer_emissions_failed.fetch_add(1, Ordering::Relaxed);
        }
        self.timer_emission_latency.record(std::time::Duration::from_micros(latency_us));
    }

    /// Record a timer-state event reception
    ///
    /// # Arguments
    /// * `sync_latency_ms` - Synchronization latency in milliseconds
    pub fn record_timer_event_reception(&self, sync_latency_ms: u64) {
        self.timer_receptions.fetch_add(1, Ordering::Relaxed);
        self.timer_sync_latency.record(std::time::Duration::from_millis(sync_latency_ms));
    }

    /// Record an invalid payload rejection
    pub fn record_invalid_payload(&self) {
        self.invalid_payloads.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a state transition
    ///
    /// # Arguments
    /// * `from` - Previous state
    /// * `to` - New state
    /// * `duration_ms` - Transition duration in milliseconds
    pub fn record_state_transition(&self, from: &str, to: &str, duration_ms: u64) {
        self.state_transitions.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut durations) = self.transition_durations.write() {
            let key = (from.to_string(), to.to_string());
            let histogram = durations.entry(key).or_insert_with(Histogram::new);
            histogram.record(std::time::Duration::from_millis(duration_ms));
        }
    }

    /// Record an autoStartTracker rule application
    ///
    /// # Arguments
    /// * `rule_num` - Rule number being validated
    /// * `timer_state` - Current timer state
    /// * `auto_start` - Whether auto-start was triggered
    /// * `is_correct` - Whether the rule application was correct
    pub fn record_auto_start_tracker_rule(
        &self,
        rule_num: u8,
        _timer_state: &str,
        _auto_start: bool,
        is_correct: bool,
    ) {
        self.auto_start_tracker_rules.fetch_add(1, Ordering::Relaxed);
        if is_correct {
            self.auto_start_tracker_correct.fetch_add(1, Ordering::Relaxed);
        } else {
            self.auto_start_tracker_incorrect.fetch_add(1, Ordering::Relaxed);
        }
        if let Ok(mut accuracy) = self.rule_accuracy.write() {
            let (correct, incorrect) = accuracy.entry(rule_num).or_insert((0, 0));
            if is_correct {
                *correct += 1;
            } else {
                *incorrect += 1;
            }
        }
    }

    /// Get idle detection latency histogram snapshot
    pub fn idle_detection_latency_snapshot(&self) -> HistogramSnapshot {
        self.idle_detection_latency.snapshot()
    }

    /// Get timer emission latency histogram snapshot
    pub fn timer_emission_latency_snapshot(&self) -> HistogramSnapshot {
        self.timer_emission_latency.snapshot()
    }

    /// Get timer sync latency histogram snapshot
    pub fn timer_sync_latency_snapshot(&self) -> HistogramSnapshot {
        self.timer_sync_latency.snapshot()
    }

    /// Get activity wake type distribution
    pub fn activity_wake_types(&self) -> HashMap<String, u64> {
        self.activity_wake_types.read().ok().map(|types| types.clone()).unwrap_or_default()
    }

    /// Get state transition duration histogram for a specific transition
    pub fn transition_duration_snapshot(
        &self,
        from: &str,
        to: &str,
    ) -> Option<HistogramSnapshot> {
        let key = (from.to_string(), to.to_string());
        self.transition_durations
            .read()
            .ok()
            .and_then(|durations| durations.get(&key).map(|h| h.snapshot()))
    }

    /// Get all state transitions with their histograms
    pub fn all_transition_snapshots(&self) -> TransitionSnapshots {
        self.transition_durations
            .read()
            .ok()
            .map(|durations| {
                durations
                    .iter()
                    .map(|(key, histogram)| (key.clone(), histogram.snapshot()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get rule accuracy by rule number
    pub fn rule_accuracy_by_number(&self) -> RuleAccuracyData {
        self.rule_accuracy.read().ok().map(|accuracy| accuracy.clone()).unwrap_or_default()
    }

    /// Reset all counters to zero (for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.idle_detections.store(0, Ordering::Relaxed);
        self.idle_detection_latency.reset();
        self.activity_wakes.store(0, Ordering::Relaxed);
        if let Ok(mut types) = self.activity_wake_types.write() {
            types.clear();
        }
        self.timer_emissions.store(0, Ordering::Relaxed);
        self.timer_emissions_success.store(0, Ordering::Relaxed);
        self.timer_emissions_failed.store(0, Ordering::Relaxed);
        self.timer_emission_latency.reset();
        self.timer_receptions.store(0, Ordering::Relaxed);
        self.timer_sync_latency.reset();
        self.invalid_payloads.store(0, Ordering::Relaxed);
        self.state_transitions.store(0, Ordering::Relaxed);
        if let Ok(mut durations) = self.transition_durations.write() {
            durations.clear();
        }
        self.auto_start_tracker_rules.store(0, Ordering::Relaxed);
        self.auto_start_tracker_correct.store(0, Ordering::Relaxed);
        self.auto_start_tracker_incorrect.store(0, Ordering::Relaxed);
        if let Ok(mut accuracy) = self.rule_accuracy.write() {
            accuracy.clear();
        }
    }

    /// Get current idle detection count (for testing/debugging)
    #[cfg(test)]
    pub fn idle_detections(&self) -> u64 {
        self.idle_detections.load(Ordering::Relaxed)
    }

    /// Get current activity wake count (for testing/debugging)
    #[cfg(test)]
    pub fn activity_wakes(&self) -> u64 {
        self.activity_wakes.load(Ordering::Relaxed)
    }

    /// Get current timer emission count (for testing/debugging)
    #[cfg(test)]
    pub fn timer_emissions(&self) -> u64 {
        self.timer_emissions.load(Ordering::Relaxed)
    }

    /// Get successful timer emission count (for testing/debugging)
    #[cfg(test)]
    pub fn timer_emissions_success(&self) -> u64 {
        self.timer_emissions_success.load(Ordering::Relaxed)
    }

    /// Get invalid payload count (for testing/debugging)
    #[cfg(test)]
    pub fn invalid_payloads(&self) -> u64 {
        self.invalid_payloads.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_idle_detection() {
        let metrics = IdleSyncMetrics::new();
        assert_eq!(metrics.idle_detections(), 0);

        metrics.record_idle_detection(100);
        assert_eq!(metrics.idle_detections(), 1);

        metrics.record_idle_detection(200);
        assert_eq!(metrics.idle_detections(), 2);
    }

    #[test]
    fn test_record_activity_wake() {
        let metrics = IdleSyncMetrics::new();
        assert_eq!(metrics.activity_wakes(), 0);

        metrics.record_activity_wake("mouse".to_string());
        assert_eq!(metrics.activity_wakes(), 1);

        metrics.record_activity_wake("keyboard".to_string());
        assert_eq!(metrics.activity_wakes(), 2);
    }

    #[test]
    fn test_record_timer_event_emission() {
        let metrics = IdleSyncMetrics::new();
        assert_eq!(metrics.timer_emissions(), 0);
        assert_eq!(metrics.timer_emissions_success(), 0);

        metrics.record_timer_event_emission(1000, true);
        assert_eq!(metrics.timer_emissions(), 1);
        assert_eq!(metrics.timer_emissions_success(), 1);

        metrics.record_timer_event_emission(2000, false);
        assert_eq!(metrics.timer_emissions(), 2);
        assert_eq!(metrics.timer_emissions_success(), 1);
    }

    #[test]
    fn test_concurrent_recording() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(IdleSyncMetrics::new());
        let mut handles = vec![];

        // Spawn 10 threads, each recording 100 events
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_idle_detection(100);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have exactly 1000 detections (10 threads × 100 each)
        assert_eq!(metrics.idle_detections(), 1000);
    }

    #[test]
    fn test_reset() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_idle_detection(100);
        metrics.record_activity_wake("mouse".to_string());
        metrics.record_invalid_payload();

        assert_eq!(metrics.idle_detections(), 1);
        assert_eq!(metrics.activity_wakes(), 1);
        assert_eq!(metrics.invalid_payloads(), 1);

        metrics.reset();

        assert_eq!(metrics.idle_detections(), 0);
        assert_eq!(metrics.activity_wakes(), 0);
        assert_eq!(metrics.invalid_payloads(), 0);
    }

    #[test]
    fn test_idle_detection_latency_histogram() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_idle_detection(100);
        metrics.record_idle_detection(200);
        metrics.record_idle_detection(300);

        let snapshot = metrics.idle_detection_latency_snapshot();
        assert_eq!(snapshot.count(), 3);
        assert!(snapshot.mean().is_some());
        assert!(snapshot.percentile(0.5).is_some());
    }

    #[test]
    fn test_timer_emission_latency_histogram() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_timer_event_emission(1000, true);
        metrics.record_timer_event_emission(2000, true);
        metrics.record_timer_event_emission(3000, false);

        let snapshot = metrics.timer_emission_latency_snapshot();
        assert_eq!(snapshot.count(), 3);
        assert!(snapshot.min().is_some());
        assert!(snapshot.max().is_some());
    }

    #[test]
    fn test_timer_sync_latency_histogram() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_timer_event_reception(50);
        metrics.record_timer_event_reception(100);
        metrics.record_timer_event_reception(150);

        let snapshot = metrics.timer_sync_latency_snapshot();
        assert_eq!(snapshot.count(), 3);
        let p99 = snapshot.percentile(0.99);
        assert!(p99.is_some());
    }

    #[test]
    fn test_activity_wake_types_tracking() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_activity_wake("mouse".to_string());
        metrics.record_activity_wake("mouse".to_string());
        metrics.record_activity_wake("keyboard".to_string());

        let types = metrics.activity_wake_types();
        assert_eq!(types.get("mouse"), Some(&2));
        assert_eq!(types.get("keyboard"), Some(&1));
    }

    #[test]
    fn test_state_transition_durations() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_state_transition("idle", "active", 100);
        metrics.record_state_transition("idle", "active", 200);
        metrics.record_state_transition("active", "idle", 150);

        let idle_to_active = metrics.transition_duration_snapshot("idle", "active");
        assert!(idle_to_active.is_some());
        let snapshot = idle_to_active.unwrap();
        assert_eq!(snapshot.count(), 2);

        let active_to_idle = metrics.transition_duration_snapshot("active", "idle");
        assert!(active_to_idle.is_some());
        assert_eq!(active_to_idle.unwrap().count(), 1);

        let all_transitions = metrics.all_transition_snapshots();
        assert_eq!(all_transitions.len(), 2);
    }

    #[test]
    fn test_rule_accuracy_by_number() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_auto_start_tracker_rule(1, "running", true, true);
        metrics.record_auto_start_tracker_rule(1, "running", true, true);
        metrics.record_auto_start_tracker_rule(1, "running", false, false);
        metrics.record_auto_start_tracker_rule(2, "stopped", true, true);

        let accuracy = metrics.rule_accuracy_by_number();
        assert_eq!(accuracy.get(&1), Some(&(2, 1))); // 2 correct, 1 incorrect
        assert_eq!(accuracy.get(&2), Some(&(1, 0))); // 1 correct, 0 incorrect
    }

    #[test]
    fn test_reset_clears_histograms() {
        let metrics = IdleSyncMetrics::new();

        metrics.record_idle_detection(100);
        metrics.record_timer_event_emission(1000, true);
        metrics.record_timer_event_reception(50);
        metrics.record_activity_wake("mouse".to_string());
        metrics.record_state_transition("idle", "active", 100);
        metrics.record_auto_start_tracker_rule(1, "running", true, true);

        metrics.reset();

        assert_eq!(metrics.idle_detection_latency_snapshot().count(), 0);
        assert_eq!(metrics.timer_emission_latency_snapshot().count(), 0);
        assert_eq!(metrics.timer_sync_latency_snapshot().count(), 0);
        assert!(metrics.activity_wake_types().is_empty());
        assert!(metrics.all_transition_snapshots().is_empty());
        assert!(metrics.rule_accuracy_by_number().is_empty());
    }
}
