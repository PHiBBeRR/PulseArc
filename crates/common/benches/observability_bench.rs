//! Observability benchmark suite
//!
//! Benchmarks covering metrics tracking hot paths, unified error handling, and
//! the lightweight no-op instrumentation traits used across the workspace.
//!
//! Run with: `cargo bench --bench observability_bench -p pulsearc-common
//! --features pulsearc-common/runtime`

use std::collections::HashMap;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use pulsearc_common::observability::{
    ActionHint, AiError, AppError, AuditLogEntry, AuditLogger, AuditSeverity,
    ClassificationMetrics, HttpError, MetricsCollector, MetricsError, MetricsTracker,
    NoOpAuditLogger, NoOpMetricsCollector, NoOpTracer, TraceSpan, Tracer, UiError,
};

// ============================================================================
// Metrics Tracker Benchmarks
// ============================================================================

fn bench_metrics_record_linfa(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_tracker_record_linfa");
    for &batch in &[1usize, 32, 128, 512, 2048] {
        group.bench_function(BenchmarkId::from_parameter(batch), |b| {
            b.iter_batched(
                MetricsTracker::new,
                |tracker| {
                    for _ in 0..batch {
                        tracker.record_linfa_prediction(black_box(1.2));
                    }
                    tracker
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_metrics_record_rules(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_tracker_record_rules");
    for &batch in &[1usize, 32, 128, 512, 2048] {
        group.bench_function(BenchmarkId::from_parameter(batch), |b| {
            b.iter_batched(
                MetricsTracker::new,
                |tracker| {
                    for _ in 0..batch {
                        tracker.record_rules_fallback();
                    }
                    tracker
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_metrics_snapshot_and_reset(c: &mut Criterion) {
    let mut snapshot_group = c.benchmark_group("metrics_tracker_snapshot");
    let seeded_tracker = {
        let tracker = MetricsTracker::new();
        for idx in 0..4096 {
            tracker.record_linfa_prediction((idx % 10) as f32 + 0.5);
            if idx % 3 == 0 {
                tracker.record_rules_fallback();
            }
        }
        tracker
    };

    snapshot_group.bench_function("snapshot_clone", |b| {
        b.iter(|| {
            let snapshot = seeded_tracker.get_metrics();
            black_box(snapshot);
        });
    });
    snapshot_group.finish();

    let mut reset_group = c.benchmark_group("metrics_tracker_reset");
    for &batch in &[64usize, 256, 1024, 4096] {
        reset_group.bench_function(BenchmarkId::from_parameter(batch), |b| {
            b.iter_batched(
                || {
                    let tracker = MetricsTracker::new();
                    for idx in 0..batch {
                        tracker.record_linfa_prediction((idx % 5) as f32 + 0.75);
                        if idx % 4 == 0 {
                            tracker.record_rules_fallback();
                        }
                    }
                    tracker
                },
                |tracker| {
                    tracker.reset();
                    tracker
                },
                BatchSize::SmallInput,
            );
        });
    }
    reset_group.finish();
}

type ClassificationScenario = (&'static str, fn() -> ClassificationMetrics);

fn bench_classification_percentages(c: &mut Criterion) {
    let scenarios: [ClassificationScenario; 4] = [
        ("empty", || ClassificationMetrics {
            linfa_predictions: 0,
            rules_fallbacks: 0,
            avg_linfa_time_ms: 0.0,
            total_predictions: 0,
        }),
        ("balanced", || ClassificationMetrics {
            linfa_predictions: 500,
            rules_fallbacks: 500,
            avg_linfa_time_ms: 1.5,
            total_predictions: 1000,
        }),
        ("linfa_dominant", || ClassificationMetrics {
            linfa_predictions: 950,
            rules_fallbacks: 50,
            avg_linfa_time_ms: 0.9,
            total_predictions: 1000,
        }),
        ("rules_dominant", || ClassificationMetrics {
            linfa_predictions: 100,
            rules_fallbacks: 900,
            avg_linfa_time_ms: 2.4,
            total_predictions: 1000,
        }),
    ];

    let mut group = c.benchmark_group("classification_metrics_percentages");
    for (name, build) in scenarios {
        group.bench_function(BenchmarkId::from_parameter(name), |b| {
            b.iter(|| {
                let metrics = build();
                black_box(metrics.linfa_coverage_percent());
                black_box(metrics.rules_fallback_percent());
            });
        });
    }
    group.finish();
}

// ============================================================================
// Error Handling Benchmarks
// ============================================================================

fn bench_app_error_code_lookup(c: &mut Criterion) {
    let samples: Vec<(&'static str, AppError)> = vec![
        ("ai_timeout", AppError::Ai(AiError::Timeout)),
        (
            "ai_rate_limited",
            AppError::Ai(AiError::RateLimited { retry_after: Some(Duration::from_secs(5)) }),
        ),
        (
            "ai_token_limit",
            AppError::Ai(AiError::TokenLimitExceeded { estimated: 32_768, limit: 8_192 }),
        ),
        (
            "http_too_many_requests",
            AppError::Http(HttpError::TooManyRequests {
                retry_after: Some(Duration::from_secs(2)),
            }),
        ),
        ("http_unauthorized", AppError::Http(HttpError::Unauthorized)),
        ("metrics_tracker_unavailable", AppError::Metrics(MetricsError::TrackerUnavailable)),
        ("validation_failed", AppError::Validation("invalid input".into())),
        ("io_error", AppError::from(std::io::Error::other("disk full"))),
    ];

    let mut group = c.benchmark_group("app_error_code_lookup");
    for (name, sample) in samples.iter().cloned() {
        group.bench_function(BenchmarkId::from_parameter(name), move |b| {
            b.iter(|| {
                black_box(sample.code());
            });
        });
    }
    group.finish();
}

fn bench_app_error_actions_and_retryability(c: &mut Criterion) {
    let samples: Vec<(&'static str, AppError)> = vec![
        (
            "retry_after",
            AppError::Ai(AiError::RateLimited { retry_after: Some(Duration::from_secs(30)) }),
        ),
        ("backoff", AppError::Ai(AiError::ServerError("upstream 500".into()))),
        ("check_key", AppError::Ai(AiError::InvalidApiKey)),
        (
            "reduce_batch",
            AppError::Ai(AiError::TokenLimitExceeded { estimated: 16_000, limit: 8_000 }),
        ),
        ("check_network", AppError::Http(HttpError::Network("connection reset".into()))),
        ("none", AppError::Validation("bad payload".into())),
    ];

    let mut group = c.benchmark_group("app_error_action_retry");
    for (name, sample) in samples.iter().cloned() {
        group.bench_function(BenchmarkId::from_parameter(name), move |b| {
            b.iter(|| {
                black_box(sample.action());
                black_box(sample.is_retryable());
            });
        });
    }
    group.finish();
}

fn bench_ui_error_conversion(c: &mut Criterion) {
    let samples: Vec<(&'static str, AppError)> = vec![
        ("ai_bad_request", AppError::Ai(AiError::BadRequest("missing field".into()))),
        ("http_status", AppError::Http(HttpError::Status { status: 503 })),
        ("metrics_other", AppError::Metrics(MetricsError::Other("ingest stalled".into()))),
        ("serde_error", AppError::Serde("could not decode response".into())),
    ];

    let mut group = c.benchmark_group("ui_error_conversion");
    for (name, sample) in samples.iter().cloned() {
        group.bench_function(BenchmarkId::from_parameter(name), move |b| {
            b.iter(|| {
                let ui: UiError = sample.to_ui();
                black_box(ui);
            });
        });
    }
    group.finish();
}

fn bench_action_hint_serialization(c: &mut Criterion) {
    let hints: Vec<ActionHint> = vec![
        ActionHint::RetryAfter { duration: Duration::from_millis(750) },
        ActionHint::Backoff,
        ActionHint::CheckConfig { key: "observability.enabled".into() },
        ActionHint::CheckNetwork,
        ActionHint::SwitchModel { model: "gpt-4o-mini".into() },
    ];

    let mut group = c.benchmark_group("action_hint_serialization");
    for (idx, hint) in hints.into_iter().enumerate() {
        group.bench_function(BenchmarkId::from_parameter(idx), move |b| {
            b.iter(|| {
                let json = serde_json::to_vec(&hint).expect("serialize action hint");
                let roundtrip: ActionHint =
                    serde_json::from_slice(&json).expect("deserialize action hint");
                black_box(roundtrip);
            });
        });
    }
    group.finish();
}

// ============================================================================
// Trait Implementations Benchmarks
// ============================================================================

fn bench_audit_log_entry_builders(c: &mut Criterion) {
    let metadata_dense: Vec<(String, String)> =
        (0..16).map(|idx| (format!("key_{idx}"), format!("value_{idx}"))).collect();

    let mut group = c.benchmark_group("audit_log_entry_builders");
    group.bench_function("single_metadata", |b| {
        b.iter(|| {
            let entry = AuditLogEntry::new("login", AuditSeverity::Info)
                .with_user("user-123")
                .with_session("session-abc")
                .with_ip("192.168.1.10")
                .with_metadata("result", "success");
            black_box(entry);
        });
    });

    group.bench_function("bulk_metadata_pairs", |b| {
        b.iter_batched(
            || metadata_dense.clone(),
            |pairs| {
                let entry = AuditLogEntry::new("bulk_event", AuditSeverity::Warning)
                    .with_metadata_pairs(pairs);
                black_box(entry);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_noop_metrics_collector(c: &mut Criterion) {
    let collector = NoOpMetricsCollector;
    let mut group = c.benchmark_group("noop_metrics_collector");
    group.bench_function("increment_counter", |b| {
        b.iter(|| {
            collector.increment_counter("requests_total", &[("endpoint", "inference")]);
        });
    });
    group.bench_function("record_gauge", |b| {
        b.iter(|| {
            collector.record_gauge("queue_depth", black_box(42.0), &[("queue", "classification")]);
        });
    });
    group.bench_function("record_histogram", |b| {
        b.iter(|| {
            collector.record_histogram("latency_ms", black_box(15.2), &[("model", "linfa")]);
        });
    });
    group.finish();
}

fn bench_noop_audit_logger(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let logger = NoOpAuditLogger;
    let template_entry = AuditLogEntry::new("noop", AuditSeverity::Debug)
        .with_user("bench")
        .with_session("session")
        .with_metadata("component", "observability");

    let mut group = c.benchmark_group("noop_audit_logger");
    group.bench_function("log_event", |b| {
        let logger = logger.clone();
        let entry = template_entry.clone();
        b.to_async(&runtime).iter(|| {
            let logger = logger.clone();
            let entry = entry.clone();
            async move {
                logger.log(entry).await;
            }
        });
    });

    group.finish();
}

fn bench_noop_tracer(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let tracer = NoOpTracer;

    let mut group = c.benchmark_group("noop_tracer");
    group.bench_function("start_span", |b| {
        let tracer = tracer.clone();
        b.to_async(&runtime).iter(|| {
            let tracer = tracer.clone();
            async move {
                let span = tracer.start_span("start_only", HashMap::new()).await;
                black_box(span.span_id.clone());
                span.finish();
            }
        });
    });

    group.bench_function("start_and_finish", |b| {
        let tracer = tracer.clone();
        b.to_async(&runtime).iter(|| {
            let tracer = tracer.clone();
            async move {
                let span: TraceSpan = tracer
                    .start_span("complete", HashMap::from([("user".into(), "bench".into())]))
                    .await;
                black_box(span.elapsed());
                span.finish();
            }
        });
    });
    group.finish();
}

criterion_group!(
    observability_benches,
    bench_metrics_record_linfa,
    bench_metrics_record_rules,
    bench_metrics_snapshot_and_reset,
    bench_classification_percentages,
    bench_app_error_code_lookup,
    bench_app_error_actions_and_retryability,
    bench_ui_error_conversion,
    bench_action_hint_serialization,
    bench_audit_log_entry_builders,
    bench_noop_metrics_collector,
    bench_noop_audit_logger,
    bench_noop_tracer,
);
criterion_main!(observability_benches);
