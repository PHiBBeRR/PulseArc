//! Comprehensive error handling benchmarks
//!
//! Benchmarks for CommonError construction, classification, conversion,
//! formatting, and structured logging operations.
//!
//! Run with: `cargo bench --bench error_bench -p pulsearc-common --features
//! pulsearc-common/foundation`

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
// Import error types
use pulsearc_common::error::{CommonError, ErrorClassification, ErrorSeverity};

// ============================================================================
// Error Construction Benchmarks
// ============================================================================

fn bench_error_construction_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_construction_simple");

    group.bench_function("config", |b| {
        b.iter(|| {
            let err = CommonError::config(black_box("invalid configuration"));
            black_box(err);
        });
    });

    group.bench_function("lock", |b| {
        b.iter(|| {
            let err = CommonError::lock(black_box("failed to acquire lock"));
            black_box(err);
        });
    });

    group.bench_function("serialization", |b| {
        b.iter(|| {
            let err = CommonError::serialization(black_box("parse error"));
            black_box(err);
        });
    });

    group.bench_function("persistence", |b| {
        b.iter(|| {
            let err = CommonError::persistence(black_box("disk write failed"));
            black_box(err);
        });
    });

    group.bench_function("not_found", |b| {
        b.iter(|| {
            let err = CommonError::not_found(black_box("User"));
            black_box(err);
        });
    });

    group.bench_function("unauthorized", |b| {
        b.iter(|| {
            let err = CommonError::unauthorized(black_box("delete_user"));
            black_box(err);
        });
    });

    group.bench_function("internal", |b| {
        b.iter(|| {
            let err = CommonError::internal(black_box("unexpected state"));
            black_box(err);
        });
    });

    group.bench_function("validation", |b| {
        b.iter(|| {
            let err = CommonError::validation(black_box("email"), black_box("invalid format"));
            black_box(err);
        });
    });

    group.finish();
}

fn bench_error_construction_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_construction_complex");

    group.bench_function("config_field", |b| {
        b.iter(|| {
            let err =
                CommonError::config_field(black_box("timeout"), black_box("must be positive"));
            black_box(err);
        });
    });

    group.bench_function("lock_resource", |b| {
        b.iter(|| {
            let err =
                CommonError::lock_resource(black_box("state_mutex"), black_box("lock poisoned"));
            black_box(err);
        });
    });

    group.bench_function("circuit_breaker_with_retry", |b| {
        b.iter(|| {
            let err = CommonError::circuit_breaker_with_retry(
                black_box("api_service"),
                black_box(Duration::from_secs(30)),
            );
            black_box(err);
        });
    });

    group.bench_function("serialization_format", |b| {
        b.iter(|| {
            let err =
                CommonError::serialization_format(black_box("JSON"), black_box("unexpected token"));
            black_box(err);
        });
    });

    group.bench_function("persistence_op", |b| {
        b.iter(|| {
            let err =
                CommonError::persistence_op(black_box("write"), black_box("permission denied"));
            black_box(err);
        });
    });

    group.bench_function("rate_limit_detailed", |b| {
        b.iter(|| {
            let err = CommonError::rate_limit_detailed(
                black_box(100),
                black_box(Duration::from_secs(60)),
                Some(black_box(Duration::from_secs(30))),
            );
            black_box(err);
        });
    });

    group.bench_function("timeout", |b| {
        b.iter(|| {
            let err = CommonError::timeout(
                black_box("database_query"),
                black_box(Duration::from_secs(5)),
            );
            black_box(err);
        });
    });

    group.bench_function("backend", |b| {
        b.iter(|| {
            let err = CommonError::backend(
                black_box("external_api"),
                black_box("connection refused"),
                black_box(true),
            );
            black_box(err);
        });
    });

    group.bench_function("validation_with_value", |b| {
        b.iter(|| {
            let err = CommonError::validation_with_value(
                black_box("age"),
                black_box("must be positive"),
                black_box("-5"),
            );
            black_box(err);
        });
    });

    group.bench_function("not_found_with_id", |b| {
        b.iter(|| {
            let err = CommonError::not_found_with_id(black_box("User"), black_box("12345"));
            black_box(err);
        });
    });

    group.bench_function("unauthorized_with_perm", |b| {
        b.iter(|| {
            let err = CommonError::unauthorized_with_perm(
                black_box("admin_action"),
                black_box("admin_role"),
            );
            black_box(err);
        });
    });

    group.bench_function("internal_with_context", |b| {
        b.iter(|| {
            let err =
                CommonError::internal_with_context(black_box("null pointer"), black_box("parser"));
            black_box(err);
        });
    });

    group.bench_function("task_cancelled_with_reason", |b| {
        b.iter(|| {
            let err = CommonError::task_cancelled_with_reason(
                black_box("sync_task"),
                black_box("user requested"),
            );
            black_box(err);
        });
    });

    group.bench_function("async_timeout", |b| {
        b.iter(|| {
            let err = CommonError::async_timeout(
                black_box("fetch_data"),
                black_box(Duration::from_secs(10)),
            );
            black_box(err);
        });
    });

    group.finish();
}

// ============================================================================
// Error Display Formatting Benchmarks
// ============================================================================

fn bench_error_display(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_display");

    let errors = vec![
        ("config_simple", CommonError::config("invalid configuration")),
        ("config_field", CommonError::config_field("timeout", "must be positive")),
        ("lock_simple", CommonError::lock("failed to acquire")),
        ("lock_resource", CommonError::lock_resource("mutex", "poisoned")),
        (
            "circuit_breaker",
            CommonError::circuit_breaker_with_retry("service", Duration::from_secs(30)),
        ),
        ("serialization", CommonError::serialization_format("JSON", "parse error")),
        ("persistence", CommonError::persistence_op("write", "disk full")),
        (
            "rate_limit",
            CommonError::rate_limit_detailed(
                100,
                Duration::from_secs(60),
                Some(Duration::from_secs(30)),
            ),
        ),
        ("timeout", CommonError::timeout("query", Duration::from_secs(5))),
        ("backend", CommonError::backend("api", "connection refused", true)),
        ("validation", CommonError::validation_with_value("email", "invalid", "not-email")),
        ("not_found", CommonError::not_found_with_id("User", "12345")),
        ("unauthorized", CommonError::unauthorized_with_perm("delete", "admin")),
        ("internal", CommonError::internal_with_context("null pointer", "parser")),
        ("task_cancelled", CommonError::task_cancelled_with_reason("task_1", "shutdown")),
        ("async_timeout", CommonError::async_timeout("api_call", Duration::from_millis(2500))),
    ];

    for (name, error) in errors {
        group.bench_with_input(BenchmarkId::from_parameter(name), &error, |b, err| {
            b.iter(|| {
                let formatted = format!("{}", black_box(err));
                black_box(formatted);
            });
        });
    }

    group.finish();
}

// ============================================================================
// ErrorClassification Benchmarks
// ============================================================================

fn bench_error_classification(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_classification");

    // Create representative errors for each category
    let retryable_errors = vec![
        ("circuit_breaker", CommonError::circuit_breaker("service")),
        ("rate_limit", CommonError::rate_limit()),
        ("timeout", CommonError::timeout("op", Duration::from_secs(1))),
        ("lock", CommonError::lock("failed")),
        ("backend_retryable", CommonError::backend("api", "temp error", true)),
        ("async_timeout", CommonError::async_timeout("future", Duration::from_secs(1))),
    ];

    let non_retryable_errors = vec![
        ("config", CommonError::config("bad config")),
        ("validation", CommonError::validation("field", "invalid")),
        ("not_found", CommonError::not_found("Resource")),
        ("internal", CommonError::internal("bug")),
        ("backend_permanent", CommonError::backend("api", "auth failed", false)),
        ("unauthorized", CommonError::unauthorized("delete")),
    ];

    // Benchmark is_retryable() for retryable errors
    for (name, error) in &retryable_errors {
        group.bench_with_input(BenchmarkId::new("is_retryable_true", name), error, |b, err| {
            b.iter(|| {
                let result = black_box(err).is_retryable();
                black_box(result);
            });
        });
    }

    // Benchmark is_retryable() for non-retryable errors
    for (name, error) in &non_retryable_errors {
        group.bench_with_input(BenchmarkId::new("is_retryable_false", name), error, |b, err| {
            b.iter(|| {
                let result = black_box(err).is_retryable();
                black_box(result);
            });
        });
    }

    // Benchmark severity()
    let all_errors = retryable_errors.iter().chain(non_retryable_errors.iter()).collect::<Vec<_>>();

    for (name, error) in all_errors {
        group.bench_with_input(BenchmarkId::new("severity", name), error, |b, err| {
            b.iter(|| {
                let severity = black_box(err).severity();
                black_box(severity);
            });
        });
    }

    // Benchmark is_critical()
    group.bench_function("is_critical_true", |b| {
        let err = CommonError::internal("critical error");
        b.iter(|| {
            let result = black_box(&err).is_critical();
            black_box(result);
        });
    });

    group.bench_function("is_critical_false", |b| {
        let err = CommonError::config("not critical");
        b.iter(|| {
            let result = black_box(&err).is_critical();
            black_box(result);
        });
    });

    // Benchmark retry_after()
    group.bench_function("retry_after_some", |b| {
        let err = CommonError::circuit_breaker_with_retry("service", Duration::from_secs(30));
        b.iter(|| {
            let delay = black_box(&err).retry_after();
            black_box(delay);
        });
    });

    group.bench_function("retry_after_none", |b| {
        let err = CommonError::config("test");
        b.iter(|| {
            let delay = black_box(&err).retry_after();
            black_box(delay);
        });
    });

    group.finish();
}

// ============================================================================
// Error Conversion Benchmarks
// ============================================================================

fn bench_error_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_conversions");

    group.bench_function("from_io_error", |b| {
        b.iter(|| {
            let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let common_err: CommonError = black_box(io_err).into();
            black_box(common_err);
        });
    });

    group.bench_function("from_serde_json_error", |b| {
        b.iter(|| {
            // Create a JSON parsing error
            let json_result = serde_json::from_str::<serde_json::Value>("invalid json");
            if let Err(json_err) = json_result {
                let common_err: CommonError = black_box(json_err).into();
                black_box(common_err);
            }
        });
    });

    #[cfg(feature = "foundation")]
    group.bench_function("from_toml_error", |b| {
        b.iter(|| {
            // Create a TOML parsing error
            let toml_result = toml::from_str::<toml::Value>("invalid = toml = syntax");
            if let Err(toml_err) = toml_result {
                let common_err: CommonError = black_box(toml_err).into();
                black_box(common_err);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Structured Logging Benchmarks
// ============================================================================

fn bench_structured_logging(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_logging");

    let errors = vec![
        ("config", CommonError::config_field("timeout", "invalid")),
        ("lock", CommonError::lock_resource("mutex", "poisoned")),
        (
            "circuit_breaker",
            CommonError::circuit_breaker_with_retry("service", Duration::from_secs(30)),
        ),
        ("serialization", CommonError::serialization_format("JSON", "parse error")),
        ("persistence", CommonError::persistence_op("write", "disk full")),
        (
            "rate_limit",
            CommonError::rate_limit_detailed(
                100,
                Duration::from_secs(60),
                Some(Duration::from_secs(30)),
            ),
        ),
        ("timeout", CommonError::timeout("query", Duration::from_secs(5))),
        ("backend", CommonError::backend("api", "error", true)),
        ("validation", CommonError::validation_with_value("field", "invalid", "value")),
        ("not_found", CommonError::not_found_with_id("User", "12345")),
        ("unauthorized", CommonError::unauthorized_with_perm("delete", "admin")),
        ("internal", CommonError::internal_with_context("error", "context")),
        ("task_cancelled", CommonError::task_cancelled_with_reason("task", "reason")),
        ("async_timeout", CommonError::async_timeout("future", Duration::from_secs(1))),
    ];

    for (name, error) in errors {
        group.bench_with_input(BenchmarkId::from_parameter(name), &error, |b, err| {
            b.iter(|| {
                let fields = black_box(err).as_tracing_fields();
                black_box(fields);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Error Cloning Benchmarks
// ============================================================================

fn bench_error_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_cloning");

    let errors = vec![
        ("config_simple", CommonError::config("test")),
        ("config_complex", CommonError::config_field("field", "message")),
        ("timeout", CommonError::timeout("op", Duration::from_secs(5))),
        (
            "rate_limit",
            CommonError::rate_limit_detailed(
                100,
                Duration::from_secs(60),
                Some(Duration::from_secs(30)),
            ),
        ),
        ("validation", CommonError::validation_with_value("field", "msg", "value")),
        ("internal", CommonError::internal_with_context("msg", "ctx")),
    ];

    for (name, error) in errors {
        group.bench_with_input(BenchmarkId::from_parameter(name), &error, |b, err| {
            b.iter(|| {
                let cloned = black_box(err).clone();
                black_box(cloned);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Fluent API Benchmarks
// ============================================================================

fn bench_fluent_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluent_api");

    group.bench_function("with_additional_context_single", |b| {
        b.iter(|| {
            let err = CommonError::timeout("operation", Duration::from_secs(5))
                .with_additional_context(black_box("retry_attempt"), black_box("3"));
            black_box(err);
        });
    });

    group.bench_function("with_additional_context_chained", |b| {
        b.iter(|| {
            let err = CommonError::backend("api", "error", true)
                .with_additional_context(black_box("endpoint"), black_box("/api/v1/data"))
                .with_additional_context(black_box("retry_count"), black_box("2"))
                .with_additional_context(black_box("user_id"), black_box("user_123"));
            black_box(err);
        });
    });

    group.finish();
}

// ============================================================================
// ErrorSeverity Benchmarks
// ============================================================================

fn bench_error_severity(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_severity");

    group.bench_function("severity_display", |b| {
        b.iter(|| {
            let formatted = format!("{}", black_box(ErrorSeverity::Critical));
            black_box(formatted);
        });
    });

    group.bench_function("severity_comparison", |b| {
        b.iter(|| {
            let result = black_box(ErrorSeverity::Critical) > black_box(ErrorSeverity::Error);
            black_box(result);
        });
    });

    group.bench_function("severity_equality", |b| {
        b.iter(|| {
            let result = black_box(ErrorSeverity::Error) == black_box(ErrorSeverity::Error);
            black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// Realistic Usage Patterns
// ============================================================================

fn bench_realistic_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_patterns");

    // Pattern: Create error, check if retryable, get severity, format
    group.bench_function("error_lifecycle_retryable", |b| {
        b.iter(|| {
            let err =
                CommonError::timeout(black_box("operation"), black_box(Duration::from_secs(5)));
            let retryable = err.is_retryable();
            let severity = err.severity();
            let formatted = format!("{}", err);
            black_box((retryable, severity, formatted));
        });
    });

    group.bench_function("error_lifecycle_permanent", |b| {
        b.iter(|| {
            let err = CommonError::validation(black_box("field"), black_box("invalid"));
            let retryable = err.is_retryable();
            let severity = err.severity();
            let critical = err.is_critical();
            let formatted = format!("{}", err);
            black_box((retryable, severity, critical, formatted));
        });
    });

    // Pattern: Create error, check if retryable with delay
    group.bench_function("retry_decision_with_delay", |b| {
        b.iter(|| {
            let err = CommonError::circuit_breaker_with_retry(
                black_box("service"),
                black_box(Duration::from_secs(30)),
            );
            let retryable = err.is_retryable();
            let delay = err.retry_after();
            black_box((retryable, delay));
        });
    });

    // Pattern: Create error, generate structured logs
    group.bench_function("error_with_structured_logging", |b| {
        b.iter(|| {
            let err = CommonError::timeout(
                black_box("database_query"),
                black_box(Duration::from_secs(5)),
            );
            let fields = err.as_tracing_fields();
            let severity = err.severity();
            black_box((fields, severity));
        });
    });

    // Pattern: Error conversion pipeline
    group.bench_function("error_conversion_pipeline", |b| {
        b.iter(|| {
            let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
            let common_err: CommonError = io_err.into();
            let retryable = common_err.is_retryable();
            let formatted = format!("{}", common_err);
            black_box((retryable, formatted));
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_error_construction_simple,
    bench_error_construction_complex,
    bench_error_display,
    bench_error_classification,
    bench_error_conversions,
    bench_structured_logging,
    bench_error_cloning,
    bench_fluent_api,
    bench_error_severity,
    bench_realistic_patterns,
);

criterion_main!(benches);
