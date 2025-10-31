//! Trait abstractions for observability components
//!
//! This module defines traits that allow components to integrate with
//! different audit, metrics, and tracing implementations without tight
//! coupling.

use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde")]
mod timestamp_ms {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::ser::Error as SerError;
    use serde::{Deserialize, Deserializer, Serializer};

    type SerializerResult<S> = Result<<S as Serializer>::Ok, <S as Serializer>::Error>;

    pub fn serialize<S>(timestamp: &SystemTime, serializer: S) -> SerializerResult<S>
    where
        S: Serializer,
    {
        let duration = timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SerError::custom("timestamp predates unix epoch"))?;
        let millis = duration.as_millis();
        let millis = u64::try_from(millis).map_err(|_| {
            SerError::custom("timestamp does not fit into 64-bit millisecond representation")
        })?;
        serializer.serialize_u64(millis)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_millis(millis))
    }
}

// ============================================================================
// Audit Logging Traits
// ============================================================================

/// Trait for audit logging implementations
///
/// Allows components to log security-relevant events without depending on
/// a specific audit logging implementation.
#[async_trait]
pub trait AuditLogger: Send + Sync + Debug {
    /// Log an audit event
    async fn log(&self, event: AuditLogEntry);

    /// Get the number of audit entries
    async fn entry_count(&self) -> usize;

    /// Check if audit logging is enabled
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Generic audit log entry
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct AuditLogEntry {
    /// Event type/action
    pub event_type: String,

    /// Event severity
    pub severity: AuditSeverity,

    /// User context (optional)
    pub user_id: Option<String>,

    /// Session ID (optional)
    pub session_id: Option<String>,

    /// IP address (optional)
    pub ip_address: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Timestamp captured at creation
    #[cfg_attr(feature = "serde", serde(with = "timestamp_ms"))]
    pub timestamp: SystemTime,
}

impl Default for AuditLogEntry {
    fn default() -> Self {
        Self {
            event_type: String::new(),
            severity: AuditSeverity::default(),
            user_id: None,
            session_id: None,
            ip_address: None,
            metadata: HashMap::new(),
            timestamp: UNIX_EPOCH,
        }
    }
}

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[non_exhaustive]
pub enum AuditSeverity {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

// ============================================================================
// Metrics Collection Traits
// ============================================================================

/// Trait for metrics collection implementations
///
/// Allows components to emit metrics without depending on a specific
/// metrics collection system.
pub trait MetricsCollector: Send + Sync + Debug {
    /// Record a counter metric
    fn increment_counter(&self, name: &str, labels: &[(&str, &str)]);

    /// Record a gauge metric
    fn record_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]);

    /// Record a histogram metric
    fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);

    /// Record timing metric (in milliseconds)
    fn record_timing(&self, name: &str, duration_ms: u64, labels: &[(&str, &str)]) {
        self.record_histogram(name, duration_ms as f64, labels);
    }
}

// ============================================================================
// Distributed Tracing Traits
// ============================================================================

/// Trait for distributed tracing implementations
///
/// Allows components to create trace spans without depending on a specific
/// tracing implementation.
#[async_trait]
pub trait Tracer: Send + Sync + Debug {
    /// Start a new trace span
    async fn start_span(&self, operation: &str, metadata: HashMap<String, String>) -> TraceSpan;

    /// Get the current active span (if any)
    fn current_span(&self) -> Option<TraceSpan>;
}

/// Represents a trace span
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[must_use = "trace spans should be finished to record timing data"]
pub struct TraceSpan {
    /// Span ID
    pub span_id: String,

    /// Trace ID
    pub trace_id: String,

    /// Operation name
    pub operation: String,

    /// Start timestamp
    #[cfg_attr(feature = "serde", serde(with = "timestamp_ms"))]
    pub start_time: SystemTime,

    /// Span metadata
    pub metadata: HashMap<String, String>,
}

impl Default for TraceSpan {
    fn default() -> Self {
        Self {
            span_id: String::new(),
            trace_id: String::new(),
            operation: String::new(),
            start_time: UNIX_EPOCH,
            metadata: HashMap::new(),
        }
    }
}

impl TraceSpan {
    /// Returns the elapsed time between span start and now.
    pub fn elapsed(&self) -> Option<Duration> {
        SystemTime::now().duration_since(self.start_time).ok()
    }

    /// Mark span as complete
    pub fn finish(self) {
        #[cfg(feature = "runtime")]
        {
            if let Some(elapsed) = self.elapsed() {
                tracing::trace!(
                    span_id = %self.span_id,
                    trace_id = %self.trace_id,
                    operation = %self.operation,
                    elapsed_ms = elapsed.as_millis(),
                    "Trace span finished"
                );
            } else {
                tracing::trace!(
                    span_id = %self.span_id,
                    trace_id = %self.trace_id,
                    operation = %self.operation,
                    "Trace span finished (elapsed unavailable)"
                );
            }
        }
    }
}

// ============================================================================
// No-Op Implementations
// ============================================================================

/// No-op audit logger for testing or when audit logging is disabled
#[derive(Debug, Clone, Default)]
pub struct NoOpAuditLogger;

#[async_trait]
impl AuditLogger for NoOpAuditLogger {
    async fn log(&self, _event: AuditLogEntry) {
        // No-op
    }

    async fn entry_count(&self) -> usize {
        0
    }

    fn is_enabled(&self) -> bool {
        false
    }
}

/// No-op metrics collector for testing or when metrics are disabled
#[derive(Debug, Clone, Default)]
pub struct NoOpMetricsCollector;

impl MetricsCollector for NoOpMetricsCollector {
    fn increment_counter(&self, _name: &str, _labels: &[(&str, &str)]) {
        // No-op
    }

    fn record_gauge(&self, _name: &str, _value: f64, _labels: &[(&str, &str)]) {
        // No-op
    }

    fn record_histogram(&self, _name: &str, _value: f64, _labels: &[(&str, &str)]) {
        // No-op
    }
}

/// No-op tracer for testing or when tracing is disabled
#[derive(Debug, Clone, Default)]
pub struct NoOpTracer;

#[async_trait]
impl Tracer for NoOpTracer {
    async fn start_span(&self, operation: &str, metadata: HashMap<String, String>) -> TraceSpan {
        TraceSpan {
            span_id: "noop".to_string(),
            trace_id: "noop".to_string(),
            operation: operation.to_string(),
            start_time: SystemTime::now(),
            metadata,
        }
    }

    fn current_span(&self) -> Option<TraceSpan> {
        None
    }
}

// ============================================================================
// Helper Builders
// ============================================================================

impl AuditLogEntry {
    /// Create a new audit log entry
    pub fn new(event_type: impl Into<String>, severity: AuditSeverity) -> Self {
        Self {
            event_type: event_type.into(),
            severity,
            user_id: None,
            session_id: None,
            ip_address: None,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set IP address
    pub fn with_ip(mut self, ip_address: impl Into<String>) -> Self {
        self.ip_address = Some(ip_address.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Bulk metadata insertion from any key/value iterator
    pub fn with_metadata_pairs<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in pairs {
            self.metadata.insert(key.into(), value.into());
        }
        self
    }

    /// Access the creation timestamp
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for observability::traits.
    use super::*;

    /// Validates `AuditLogEntry::new` behavior for the noop audit logger
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `logger.entry_count().await` equals `0`.
    /// - Ensures `!logger.is_enabled()` evaluates to true.
    #[tokio::test]
    async fn test_noop_audit_logger() {
        let logger = NoOpAuditLogger;
        let entry = AuditLogEntry::new("test_event", AuditSeverity::Info);
        logger.log(entry).await;
        assert_eq!(logger.entry_count().await, 0);
        assert!(!logger.is_enabled());
    }

    /// Validates the noop metrics collector scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_noop_metrics_collector() {
        let collector = NoOpMetricsCollector;
        collector.increment_counter("test_counter", &[("label", "value")]);
        collector.record_gauge("test_gauge", 42.0, &[]);
        collector.record_histogram("test_histogram", 100.0, &[]);
        // Should not panic
    }

    /// Validates `HashMap::new` behavior for the noop tracer scenario.
    ///
    /// Assertions:
    /// - Confirms `span.operation` equals `"test_operation"`.
    /// - Confirms `span.span_id` equals `"noop"`.
    /// - Ensures `tracer.current_span().is_none()` evaluates to true.
    #[tokio::test]
    async fn test_noop_tracer() {
        let tracer = NoOpTracer;
        let span = tracer.start_span("test_operation", HashMap::new()).await;
        assert_eq!(span.operation, "test_operation");
        assert_eq!(span.span_id, "noop");
        assert!(tracer.current_span().is_none());
    }

    /// Validates `AuditLogEntry::new` behavior for the audit log entry builder
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `entry.event_type` equals `"login"`.
    /// - Confirms `entry.severity` equals `AuditSeverity::Info`.
    /// - Confirms `entry.user_id` equals `Some("user123".to_string())`.
    /// - Confirms `entry.session_id` equals `Some("session456".to_string())`.
    /// - Confirms `entry.ip_address` equals `Some("192.168.1.1".to_string())`.
    /// - Confirms `entry.metadata.get("action")` equals
    ///   `Some(&"successful".to_string())`.
    /// - Ensures `elapsed <= Duration::from_secs(1)` evaluates to true.
    #[test]
    fn test_audit_log_entry_builder() {
        let entry = AuditLogEntry::new("login", AuditSeverity::Info)
            .with_user("user123")
            .with_session("session456")
            .with_ip("192.168.1.1")
            .with_metadata("action", "successful");

        assert_eq!(entry.event_type, "login");
        assert_eq!(entry.severity, AuditSeverity::Info);
        assert_eq!(entry.user_id, Some("user123".to_string()));
        assert_eq!(entry.session_id, Some("session456".to_string()));
        assert_eq!(entry.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(entry.metadata.get("action"), Some(&"successful".to_string()));
        let elapsed = entry.timestamp().elapsed().expect("timestamp should not be in the future");
        assert!(elapsed <= Duration::from_secs(1));
    }

    /// Validates `AuditLogEntry::new` behavior for the audit log entry bulk
    /// metadata scenario.
    ///
    /// Assertions:
    /// - Confirms `entry.metadata.get("k1")` equals `Some(&"v1".to_string())`.
    /// - Confirms `entry.metadata.get("k2")` equals `Some(&"v2".to_string())`.
    #[test]
    fn test_audit_log_entry_bulk_metadata() {
        let entry = AuditLogEntry::new("bulk", AuditSeverity::Warning)
            .with_metadata_pairs(vec![("k1", "v1"), ("k2", "v2")]);

        assert_eq!(entry.metadata.get("k1"), Some(&"v1".to_string()));
        assert_eq!(entry.metadata.get("k2"), Some(&"v2".to_string()));
    }

    /// Validates `HashMap::new` behavior for the trace span elapsed scenario.
    ///
    /// Assertions:
    /// - Ensures `span.elapsed().is_some()` evaluates to true.
    #[tokio::test]
    async fn test_trace_span_elapsed() {
        let tracer = NoOpTracer;
        let span = tracer.start_span("operation", HashMap::new()).await;
        assert!(span.elapsed().is_some());
        span.finish();
    }
}
