//! Distributed tracing support for retry operations
//!
//! This module provides tracing instrumentation for retry operations using
//! the standard `tracing` crate instead of OpenTelemetry to avoid external
//! dependencies.

use std::time::Duration;

use tracing::{info, warn};

/// Distributed tracing support for retry operations
pub struct RetryTracer;

impl RetryTracer {
    /// Create a new retry tracer
    pub fn new() -> Self {
        Self
    }

    /// Start a retry operation span
    pub fn start_retry_span(&self, operation_name: &str, max_attempts: u32) -> RetrySpan {
        info!(operation = operation_name, max_attempts = max_attempts, "Starting retry operation");

        RetrySpan { operation_name: operation_name.to_string(), max_attempts }
    }
}

impl Default for RetryTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// A span representing a retry operation
pub struct RetrySpan {
    operation_name: String,
    max_attempts: u32,
}

impl RetrySpan {
    /// Record an attempt
    pub fn record_attempt(&mut self, attempt: u32, delay: Option<Duration>) {
        if let Some(delay) = delay {
            info!(
                operation = %self.operation_name,
                attempt = attempt,
                delay_ms = delay.as_millis(),
                "Retry attempt with delay"
            );
        } else {
            info!(
                operation = %self.operation_name,
                attempt = attempt,
                "Retry attempt"
            );
        }
    }

    /// Record a successful retry
    pub fn record_success(&mut self, attempts: u32, total_delay: Duration) {
        info!(
            operation = %self.operation_name,
            attempts = attempts,
            total_delay_ms = total_delay.as_millis(),
            "Retry operation succeeded"
        );
    }

    /// Record a failure
    pub fn record_failure(&mut self, attempt: u32, error: &str) {
        warn!(
            operation = %self.operation_name,
            attempt = attempt,
            error = %error,
            "Retry attempt failed"
        );
    }

    /// Record that all attempts have been exhausted
    pub fn record_exhausted(&mut self, total_delay: Duration) {
        warn!(
            operation = %self.operation_name,
            max_attempts = self.max_attempts,
            total_delay_ms = total_delay.as_millis(),
            "All retry attempts exhausted"
        );
    }

    /// Record timeout
    pub fn record_timeout(&mut self, elapsed: Duration) {
        warn!(
            operation = %self.operation_name,
            elapsed_ms = elapsed.as_millis(),
            "Retry operation timed out"
        );
    }

    /// Record circuit breaker open
    pub fn record_circuit_breaker_open(&mut self) {
        warn!(
            operation = %self.operation_name,
            "Circuit breaker is open, aborting retry"
        );
    }

    /// Record budget exhausted
    pub fn record_budget_exhausted(&mut self) {
        warn!(
            operation = %self.operation_name,
            "Retry budget exhausted, aborting retry"
        );
    }

    /// End the span
    pub fn end(self) {
        // Span will be dropped, which ends it in tracing
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::retry::tracing.
    use super::*;

    /// Validates `RetryTracer::new` behavior for the retry tracer creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `span.operation_name` equals `"test_operation"`.
    /// - Confirms `span.max_attempts` equals `3`.
    #[test]
    fn test_retry_tracer_creation() {
        let tracer = RetryTracer::new();
        let span = tracer.start_retry_span("test_operation", 3);
        assert_eq!(span.operation_name, "test_operation");
        assert_eq!(span.max_attempts, 3);
    }

    /// Validates the default tracer scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_default_tracer() {
        let tracer = RetryTracer;
        let _span = tracer.start_retry_span("test", 5);
    }

    /// Validates `RetryTracer::new` behavior for the span recording scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_span_recording() {
        let tracer = RetryTracer::new();
        let mut span = tracer.start_retry_span("test_operation", 3);

        span.record_attempt(1, None);
        span.record_attempt(2, Some(Duration::from_millis(100)));
        span.record_failure(2, "test error");
        span.record_success(3, Duration::from_secs(1));
        span.end();
    }

    /// Validates `RetryTracer::new` behavior for the span exhausted scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_span_exhausted() {
        let tracer = RetryTracer::new();
        let mut span = tracer.start_retry_span("test_operation", 3);
        span.record_exhausted(Duration::from_secs(5));
        span.end();
    }

    /// Validates `RetryTracer::new` behavior for the span timeout scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_span_timeout() {
        let tracer = RetryTracer::new();
        let mut span = tracer.start_retry_span("test_operation", 3);
        span.record_timeout(Duration::from_secs(10));
        span.end();
    }

    /// Validates `RetryTracer::new` behavior for the span circuit breaker
    /// scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_span_circuit_breaker() {
        let tracer = RetryTracer::new();
        let mut span = tracer.start_retry_span("test_operation", 3);
        span.record_circuit_breaker_open();
        span.end();
    }

    /// Validates `RetryTracer::new` behavior for the span budget exhausted
    /// scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_span_budget_exhausted() {
        let tracer = RetryTracer::new();
        let mut span = tracer.start_retry_span("test_operation", 3);
        span.record_budget_exhausted();
        span.end();
    }
}
