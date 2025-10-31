//! Integration tests for observability module.
//!
//! Validates end-to-end behavior for audit logging, metrics tracking, tracing,
//! and error conversion utilities exposed by `pulsearc_common::observability`.

#![cfg(all(feature = "runtime", feature = "serde"))]

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use pulsearc_common::observability::{
    ActionHint, AiError, AppError, AuditLogEntry, AuditLogger, AuditSeverity, HttpError,
    MetricsCollector, MetricsError, MetricsTracker, TraceSpan, Tracer, UiError,
};
use tokio::time::sleep;
use uuid::Uuid;

type LabelPairs = Vec<(String, String)>;
type CounterStore = Vec<(String, LabelPairs)>;
type NumericStore = Vec<(String, f64, LabelPairs)>;

/// In-memory audit logger used for verifying emitted audit events.
#[derive(Debug, Default, Clone)]
struct TestAuditLogger {
    entries: Arc<Mutex<Vec<AuditLogEntry>>>,
}

impl TestAuditLogger {
    fn entries(&self) -> Vec<AuditLogEntry> {
        self.entries.lock().expect("audit entries mutex poisoned").clone()
    }
}

#[async_trait]
impl AuditLogger for TestAuditLogger {
    async fn log(&self, event: AuditLogEntry) {
        self.entries.lock().expect("audit entries mutex poisoned").push(event);
    }

    async fn entry_count(&self) -> usize {
        self.entries.lock().expect("audit entries mutex poisoned").len()
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

/// In-memory metrics collector that captures all emitted measurements.
#[derive(Debug, Default, Clone)]
struct TestMetricsCollector {
    counters: Arc<Mutex<CounterStore>>,
    gauges: Arc<Mutex<NumericStore>>,
    histograms: Arc<Mutex<NumericStore>>,
}

impl TestMetricsCollector {
    fn counter_hits(&self, name: &str) -> usize {
        self.counters
            .lock()
            .expect("counter mutex poisoned")
            .iter()
            .filter(|(counter_name, _)| counter_name == name)
            .count()
    }

    fn histogram_samples(&self, name: &str) -> Vec<f64> {
        self.histograms
            .lock()
            .expect("histogram mutex poisoned")
            .iter()
            .filter(|(metric_name, _, _)| metric_name == name)
            .map(|(_, value, _)| *value)
            .collect()
    }
}

impl MetricsCollector for TestMetricsCollector {
    fn increment_counter(&self, name: &str, labels: &[(&str, &str)]) {
        let label_pairs =
            labels.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        self.counters.lock().expect("counter mutex poisoned").push((name.to_string(), label_pairs));
    }

    fn record_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let label_pairs =
            labels.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        self.gauges.lock().expect("gauge mutex poisoned").push((
            name.to_string(),
            value,
            label_pairs,
        ));
    }

    fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let label_pairs =
            labels.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        self.histograms.lock().expect("histogram mutex poisoned").push((
            name.to_string(),
            value,
            label_pairs,
        ));
    }
}

/// In-memory tracer that records started spans for verification.
#[derive(Debug, Default, Clone)]
struct TestTracer {
    spans: Arc<Mutex<Vec<TraceSpan>>>,
    current: Arc<Mutex<Option<TraceSpan>>>,
}

impl TestTracer {
    fn spans(&self) -> Vec<TraceSpan> {
        self.spans.lock().expect("span mutex poisoned").clone()
    }

    fn clear_current(&self) {
        self.current.lock().expect("current span mutex poisoned").take();
    }
}

#[async_trait]
impl Tracer for TestTracer {
    async fn start_span(&self, operation: &str, metadata: HashMap<String, String>) -> TraceSpan {
        let span = TraceSpan {
            span_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4().to_string(),
            operation: operation.to_string(),
            start_time: SystemTime::now(),
            metadata,
        };

        {
            let mut current = self.current.lock().expect("current span mutex poisoned");
            *current = Some(span.clone());
        }
        self.spans.lock().expect("span mutex poisoned").push(span.clone());
        span
    }

    fn current_span(&self) -> Option<TraceSpan> {
        self.current.lock().expect("current span mutex poisoned").clone()
    }
}

/// Validates that observability primitives can be composed to track a full
/// classification workflow, including metrics tracking, audit logging, and
/// distributed tracing metadata.
#[tokio::test(flavor = "multi_thread")]
async fn test_observability_workflow_integration() {
    let audit_logger = TestAuditLogger::default();
    let metrics_collector = TestMetricsCollector::default();
    let tracer = TestTracer::default();
    let metrics_tracker = MetricsTracker::new();

    let mut span_metadata = HashMap::new();
    span_metadata.insert("component".to_string(), "classifier".to_string());
    span_metadata.insert("model".to_string(), "linfa".to_string());

    let span = tracer.start_span("classification.run", span_metadata.clone()).await;
    let active_span = tracer.current_span().expect("span should be active after start");
    assert_eq!(active_span.operation, "classification.run");
    assert_eq!(active_span.metadata.get("component"), Some(&"classifier".to_string()));

    metrics_tracker.record_linfa_prediction(0.8);
    metrics_collector.increment_counter("classification.success", &[("model", "linfa")]);
    metrics_collector.record_histogram("classification.latency_ms", 0.8, &[("model", "linfa")]);

    audit_logger
        .log(
            AuditLogEntry::new("classification.completed", AuditSeverity::Info)
                .with_user("integration-user")
                .with_metadata("model", "linfa")
                .with_metadata("latency_ms", "0.8"),
        )
        .await;

    sleep(Duration::from_millis(5)).await;
    span.finish();
    tracer.clear_current();

    let classification_metrics = metrics_tracker.get_metrics();
    assert_eq!(classification_metrics.linfa_predictions, 1);
    assert_eq!(classification_metrics.rules_fallbacks, 0);
    assert_eq!(classification_metrics.total_predictions, 1);
    assert!((classification_metrics.avg_linfa_time_ms - 0.8).abs() < f32::EPSILON);

    assert_eq!(metrics_collector.counter_hits("classification.success"), 1);
    let histogram_samples = metrics_collector.histogram_samples("classification.latency_ms");
    assert_eq!(histogram_samples.len(), 1);
    assert!((histogram_samples[0] - 0.8).abs() < f64::EPSILON);

    let recorded_spans = tracer.spans();
    assert_eq!(recorded_spans.len(), 1);
    let recorded_span = &recorded_spans[0];
    assert_eq!(recorded_span.metadata.get("model"), Some(&"linfa".to_string()));
    assert!(
        recorded_span.elapsed().expect("elapsed measurement available") >= Duration::from_millis(5)
    );
    assert!(tracer.current_span().is_none(), "clearing current span should remove active span");

    let audit_entries = audit_logger.entries();
    assert_eq!(audit_entries.len(), 1);
    let entry = &audit_entries[0];
    assert_eq!(entry.event_type, "classification.completed");
    assert_eq!(entry.user_id.as_deref(), Some("integration-user"));
    assert_eq!(entry.metadata.get("latency_ms"), Some(&"0.8".to_string()));
}

/// Validates error classification, UI conversion, and retry semantics for
/// diverse observability error variants.
#[test]
fn test_app_error_ui_conversion_and_retry_semantics() {
    let retry_after = Duration::from_secs(2);
    let ai_error = AppError::from(AiError::RateLimited { retry_after: Some(retry_after) });
    let ai_ui = ai_error.to_ui();
    assert_eq!(ai_ui.code, pulsearc_common::observability::ErrorCode::AiRateLimited);
    match ai_ui.action {
        ActionHint::RetryAfter { duration } => assert_eq!(duration, retry_after),
        other => panic!("expected retry-after action hint, got {other:?}"),
    }
    assert!(ai_error.is_retryable());

    let http_error = AppError::from(HttpError::Network("gateway unavailable".into()));
    assert_eq!(http_error.code(), pulsearc_common::observability::ErrorCode::HttpNetwork);
    match http_error.action() {
        ActionHint::CheckNetwork => {}
        other => panic!("expected network hint, got {other:?}"),
    }
    assert!(http_error.to_ui().message.contains("Network error"));

    let metrics_error = AppError::from(MetricsError::TrackerUnavailable);
    assert_eq!(
        metrics_error.code(),
        pulsearc_common::observability::ErrorCode::MetricsTrackerUnavailable
    );
    assert!(matches!(metrics_error.action(), ActionHint::None));
    assert!(metrics_error.is_retryable());

    let io_error = AppError::from(io::Error::other("disk offline while collecting metrics"));
    assert_eq!(io_error.code(), pulsearc_common::observability::ErrorCode::Io);
    assert!(matches!(io_error.action(), ActionHint::None));

    let serde_error: AppError =
        serde_json::from_str::<serde_json::Value>("not-json").unwrap_err().into();
    assert_eq!(serde_error.code(), pulsearc_common::observability::ErrorCode::Serialization);
    assert!(matches!(serde_error.action(), ActionHint::None));

    let ui_error = UiError::from_message("human readable problem");
    assert_eq!(ui_error.code, pulsearc_common::observability::ErrorCode::Unknown);
    assert_eq!(ui_error.message, "human readable problem");
    assert!(matches!(ui_error.action, ActionHint::None));
}

/// Ensures observability hints and audit log entries serialize cleanly for
/// forwarding to UI or external telemetry sinks.
#[test]
fn test_action_hint_and_audit_log_entry_serialization() {
    let hint = ActionHint::CheckConfig { key: "observability.enabled".to_string() };
    let serialized_hint = serde_json::to_value(&hint).expect("serialize action hint");
    assert_eq!(serialized_hint["kind"], "CHECK_CONFIG");
    assert_eq!(serialized_hint["key"], "observability.enabled");

    let roundtrip_hint: ActionHint =
        serde_json::from_value(serialized_hint).expect("deserialize action hint");
    match roundtrip_hint {
        ActionHint::CheckConfig { key } => assert_eq!(key, "observability.enabled"),
        other => panic!("expected check-config hint, got {other:?}"),
    }

    let entry = AuditLogEntry::new("config.audit", AuditSeverity::Warning)
        .with_session("session-123")
        .with_metadata("config_key", "observability.enabled");

    let serialized_entry =
        serde_json::to_string(&entry).expect("serialize audit entry to json string");
    let entry_json: serde_json::Value =
        serde_json::from_str(&serialized_entry).expect("parse serialized audit entry");
    let metadata_json = entry_json
        .get("metadata")
        .and_then(|value| value.as_object())
        .expect("metadata object present");
    assert_eq!(
        metadata_json.get("config_key").and_then(|v| v.as_str()),
        Some("observability.enabled")
    );
    assert!(entry_json.get("timestamp").is_some(), "timestamp should be serialized in millis");

    let deserialized: AuditLogEntry =
        serde_json::from_value(entry_json).expect("deserialize audit entry");
    assert_eq!(deserialized.event_type, "config.audit");
    assert_eq!(deserialized.session_id.as_deref(), Some("session-123"));
    assert_eq!(deserialized.metadata.get("config_key"), Some(&"observability.enabled".to_string()));
    assert!(deserialized.timestamp() <= SystemTime::now());
}
