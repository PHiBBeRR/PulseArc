//! In-memory telemetry metrics for idle detection and activity synchronization
//!
//! FEATURE-012: Tracks idle detection events, timer state transitions, and
//! autoStartTracker rule validation for debugging and performance monitoring.

use std::sync::atomic::{AtomicU64, Ordering};

/// In-memory telemetry metrics for idle sync operations
///
/// Thread-safe counters for tracking idle detection, activity wake events,
/// timer state events, and validation metrics. Metrics are stored in-memory
/// and reset on application restart.
#[derive(Debug, Default)]
pub struct IdleSyncMetrics {
    // Idle detection metrics
    idle_detections: AtomicU64,
    activity_wakes: AtomicU64,

    // Timer event metrics
    timer_emissions: AtomicU64,
    timer_emissions_success: AtomicU64,
    timer_emissions_failed: AtomicU64,
    timer_receptions: AtomicU64,

    // Error tracking
    invalid_payloads: AtomicU64,

    // State transition tracking
    state_transitions: AtomicU64,

    // AutoStartTracker validation
    auto_start_tracker_rules: AtomicU64,
    auto_start_tracker_correct: AtomicU64,
    auto_start_tracker_incorrect: AtomicU64,
}

impl IdleSyncMetrics {
    /// Create a new metrics instance with all counters at zero
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an idle detection event with latency measurement
    ///
    /// # Arguments
    /// * `latency_ms` - Time in milliseconds to detect idle state
    pub fn record_idle_detection(&self, _latency_ms: u64) {
        self.idle_detections.fetch_add(1, Ordering::Relaxed);
        // TODO: Store latency histogram for percentile calculations
    }

    /// Record an activity wake event
    ///
    /// # Arguments
    /// * `event_type` - Type of wake event (e.g., "mouse", "keyboard")
    pub fn record_activity_wake(&self, _event_type: String) {
        self.activity_wakes.fetch_add(1, Ordering::Relaxed);
        // TODO: Track event type distribution if needed
    }

    /// Record a timer-state event emission
    ///
    /// # Arguments
    /// * `latency_us` - Emission latency in microseconds
    /// * `success` - Whether emission succeeded
    pub fn record_timer_event_emission(&self, _latency_us: u64, success: bool) {
        self.timer_emissions.fetch_add(1, Ordering::Relaxed);
        if success {
            self.timer_emissions_success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.timer_emissions_failed.fetch_add(1, Ordering::Relaxed);
        }
        // TODO: Store emission latency histogram
    }

    /// Record a timer-state event reception
    ///
    /// # Arguments
    /// * `sync_latency_ms` - Synchronization latency in milliseconds
    pub fn record_timer_event_reception(&self, _sync_latency_ms: u64) {
        self.timer_receptions.fetch_add(1, Ordering::Relaxed);
        // TODO: Store sync latency histogram
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
    pub fn record_state_transition(&self, _from: &str, _to: &str, _duration_ms: u64) {
        self.state_transitions.fetch_add(1, Ordering::Relaxed);
        // TODO: Track state transition pairs and durations
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
        _rule_num: u8,
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
        // TODO: Track rule accuracy by rule number
    }

    /// Reset all counters to zero (for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.idle_detections.store(0, Ordering::Relaxed);
        self.activity_wakes.store(0, Ordering::Relaxed);
        self.timer_emissions.store(0, Ordering::Relaxed);
        self.timer_emissions_success.store(0, Ordering::Relaxed);
        self.timer_emissions_failed.store(0, Ordering::Relaxed);
        self.timer_receptions.store(0, Ordering::Relaxed);
        self.invalid_payloads.store(0, Ordering::Relaxed);
        self.state_transitions.store(0, Ordering::Relaxed);
        self.auto_start_tracker_rules.store(0, Ordering::Relaxed);
        self.auto_start_tracker_correct.store(0, Ordering::Relaxed);
        self.auto_start_tracker_incorrect.store(0, Ordering::Relaxed);
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
}
