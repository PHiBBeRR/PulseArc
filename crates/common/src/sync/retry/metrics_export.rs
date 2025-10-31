use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec, Registry,
};
use std::sync::Arc;
use std::time::Duration;

use super::{CircuitBreakerStats, CircuitState, RetryMetrics};

/// Prometheus metrics exporter for retry operations
pub struct RetryMetricsExporter {
    /// Counter for retry attempts
    retry_attempts: CounterVec,
    /// Counter for successful retries
    retry_successes: CounterVec,
    /// Counter for failed retries
    retry_failures: CounterVec,
    /// Histogram for retry delays
    retry_delays: HistogramVec,
    /// Gauge for circuit breaker state (0=closed, 1=open, 2=half-open)
    circuit_breaker_state: GaugeVec,
    /// Counter for circuit breaker transitions
    circuit_breaker_transitions: CounterVec,
    /// Gauge for retry budget available tokens
    retry_budget_available: GaugeVec,
    /// Gauge for retry budget capacity
    retry_budget_capacity: GaugeVec,
}

impl RetryMetricsExporter {
    /// Create a new metrics exporter with the given registry
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let retry_attempts = register_counter_vec!(
            "retry_attempts_total",
            "Total number of retry attempts",
            &["operation", "reason"]
        )?;
        registry.register(Box::new(retry_attempts.clone()))?;

        let retry_successes = register_counter_vec!(
            "retry_successes_total",
            "Total number of successful retry operations",
            &["operation"]
        )?;
        registry.register(Box::new(retry_successes.clone()))?;

        let retry_failures = register_counter_vec!(
            "retry_failures_total",
            "Total number of failed retry operations",
            &["operation", "reason"]
        )?;
        registry.register(Box::new(retry_failures.clone()))?;

        let retry_delays = register_histogram_vec!(
            "retry_delay_seconds",
            "Delay between retry attempts in seconds",
            &["operation"],
            vec![0.001, 0.01, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
        )?;
        registry.register(Box::new(retry_delays.clone()))?;

        let circuit_breaker_state = register_gauge_vec!(
            "circuit_breaker_state",
            "Current state of circuit breaker (0=closed, 1=open, 2=half-open)",
            &["service"]
        )?;
        registry.register(Box::new(circuit_breaker_state.clone()))?;

        let circuit_breaker_transitions = register_counter_vec!(
            "circuit_breaker_transitions_total",
            "Total number of circuit breaker state transitions",
            &["service", "from", "to"]
        )?;
        registry.register(Box::new(circuit_breaker_transitions.clone()))?;

        let retry_budget_available = register_gauge_vec!(
            "retry_budget_available_tokens",
            "Available tokens in retry budget",
            &["budget"]
        )?;
        registry.register(Box::new(retry_budget_available.clone()))?;

        let retry_budget_capacity = register_gauge_vec!(
            "retry_budget_capacity_tokens",
            "Maximum capacity of retry budget",
            &["budget"]
        )?;
        registry.register(Box::new(retry_budget_capacity.clone()))?;

        Ok(Self {
            retry_attempts,
            retry_successes,
            retry_failures,
            retry_delays,
            circuit_breaker_state,
            circuit_breaker_transitions,
            retry_budget_available,
            retry_budget_capacity,
        })
    }

    /// Record retry operation metrics
    pub fn record_retry_operation(
        &self,
        operation: &str,
        metrics: &RetryMetrics,
        error: Option<&str>,
    ) {
        // Record attempts
        self.retry_attempts
            .with_label_values(&[operation, "total"])
            .inc_by(metrics.attempts as f64);

        // Record outcome
        if metrics.succeeded {
            self.retry_successes.with_label_values(&[operation]).inc();
        } else {
            let reason = if metrics.timed_out {
                "timeout"
            } else if let Some(err) = error {
                err
            } else {
                "exhausted"
            };
            self.retry_failures
                .with_label_values(&[operation, reason])
                .inc();
        }

        // Record delay if there were retries
        if metrics.attempts > 1 {
            let avg_delay = metrics.average_delay().unwrap_or(Duration::ZERO);
            self.retry_delays
                .with_label_values(&[operation])
                .observe(avg_delay.as_secs_f64());
        }
    }

    /// Record circuit breaker state
    pub fn record_circuit_breaker_state(&self, service: &str, stats: &CircuitBreakerStats) {
        let state_value = match stats.state {
            CircuitState::Closed => 0.0,
            CircuitState::Open => 1.0,
            CircuitState::HalfOpen => 2.0,
        };
        self.circuit_breaker_state
            .with_label_values(&[service])
            .set(state_value);
    }

    /// Record circuit breaker transition
    pub fn record_circuit_breaker_transition(
        &self,
        service: &str,
        from: CircuitState,
        to: CircuitState,
    ) {
        self.circuit_breaker_transitions
            .with_label_values(&[service, &from.to_string(), &to.to_string()])
            .inc();
    }

    /// Record retry budget state
    pub fn record_retry_budget_state(&self, budget_name: &str, available: u32, capacity: u32) {
        self.retry_budget_available
            .with_label_values(&[budget_name])
            .set(available as f64);
        self.retry_budget_capacity
            .with_label_values(&[budget_name])
            .set(capacity as f64);
    }
}

/// Global metrics exporter instance
static METRICS_EXPORTER: std::sync::OnceLock<Arc<RetryMetricsExporter>> =
    std::sync::OnceLock::new();

/// Initialize the global metrics exporter
pub fn init_metrics_exporter(registry: &Registry) -> Result<(), prometheus::Error> {
    let exporter = RetryMetricsExporter::new(registry)?;
    METRICS_EXPORTER
        .set(Arc::new(exporter))
        .map_err(|_| prometheus::Error::AlreadyReg)?;
    Ok(())
}

/// Get the global metrics exporter
pub fn metrics_exporter() -> Option<Arc<RetryMetricsExporter>> {
    METRICS_EXPORTER.get().cloned()
}

/// Helper macro to record retry metrics if exporter is initialized
#[macro_export]
macro_rules! record_retry_metrics {
    ($operation:expr, $metrics:expr, $error:expr) => {
        if let Some(exporter) = $crate::sync::retry::metrics_export::metrics_exporter() {
            exporter.record_retry_operation($operation, $metrics, $error);
        }
    };
}

/// Helper macro to record circuit breaker state
#[macro_export]
macro_rules! record_circuit_breaker_state {
    ($service:expr, $stats:expr) => {
        if let Some(exporter) = $crate::sync::retry::metrics_export::metrics_exporter() {
            exporter.record_circuit_breaker_state($service, $stats);
        }
    };
}

/// Helper macro to record budget state
#[macro_export]
macro_rules! record_budget_state {
    ($name:expr, $available:expr, $capacity:expr) => {
        if let Some(exporter) = $crate::sync::retry::metrics_export::metrics_exporter() {
            exporter.record_retry_budget_state($name, $available, $capacity);
        }
    };
}
