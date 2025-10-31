# Observability Module

`pulsearc_common::observability` centralizes error modeling, metrics tracking, and vendor-neutral instrumentation primitives used across the PulseArc agent. Keeping these capabilities in one place lets components emit telemetry, surface actionable feedback for users, and remain testable without binding to concrete backends.

## Directory Layout

```
observability/
├── errors/            # strongly typed error system and UI bridge
├── metrics/           # metric types and thread-safe trackers
├── traits.rs          # audit/metrics/tracing traits + no-op adapters
└── mod.rs             # re-exports and module wiring
```

## Error Model

The error system in `errors/app.rs` provides a consistent surface for anything that can fail inside the agent.

| Type | Purpose |
| --- | --- |
| `AppError` | Top-level error with ergonomic `From` impls and telemetry helpers |
| `AiError`, `HttpError`, `MetricsError` | Domain-specific variants with stable codes |
| `ErrorCode` | SCREAMING_SNAKE_CASE identifiers used in logs, metrics, and UI payloads |
| `ActionHint` | Optional operator guidance (retry, check config, etc.) |
| `UiError` | Frontend-safe struct containing the code, message, and hint |
| `AppResult<T>` | Convenience alias for `Result<T, AppError>` |

Key helpers:

- `AppError::code()` and `AppError::action()` map every variant to an `ErrorCode` and `ActionHint`.
- `AppError::is_retryable()` flags transient failures for backoff loops or job schedulers.
- `AppError::to_ui()` converts directly into `UiError`, making serialization trivial when the `serde` feature is enabled.

```rust
use pulsearc_common::observability::{AppError, AppResult, UiError};

fn fetch_embeddings() -> AppResult<Vec<f32>> {
    // ...
    Err(AppError::from(
        std::io::Error::other("upstream embeddings store unavailable"),
    ))
}

fn to_frontend(error: AppError) -> UiError {
    let ui = error.to_ui();
    tracing::warn!(code = ?ui.code, action = ?ui.action, "operation failed");
    ui
}
```

## Metrics Tracking

Classification-specific metrics live in `metrics/classification.rs`. `MetricsTracker` wraps an `Arc<Mutex<ClassificationMetrics>>`, offering cheap cloning and safe sharing across async tasks.

- `ClassificationMetrics` keeps totals for LINFA invocations, rule fallbacks, and running averages.
- `MetricsTracker` exposes helpers like `record_linfa_prediction`, `record_rules_fallback`, `get_metrics`, and `reset`.
- `PerformanceMetrics` in `metrics/mod.rs` is the expansion point for broader performance telemetry.

```rust
use pulsearc_common::observability::metrics::MetricsTracker;

let metrics = MetricsTracker::new();
metrics.record_linfa_prediction(0.9);
metrics.record_rules_fallback();

let snapshot = metrics.get_metrics();
assert_eq!(snapshot.linfa_predictions, 1);
assert_eq!(snapshot.rules_fallbacks, 1);
```

## Trait Abstractions

`traits.rs` defines interfaces that let the rest of the codebase depend on behavior instead of concrete telemetry SDKs.

- `AuditLogger` accepts `AuditLogEntry` structs and reports competence via `entry_count()` / `is_enabled()`.
- `MetricsCollector` supports counters, gauges, and histograms. `record_timing` is implemented in terms of histograms for convenience.
- `Tracer` creates `TraceSpan` values that can later be finished or inspected; `TraceSpan::elapsed()` is available even without tracing backends.
- `AuditLogEntry` and `TraceSpan` derive `Serialize`/`Deserialize` when the `serde` feature is enabled, so they can be shipped to the UI or a remote collector.

The module also ships testing-friendly implementations:

- `NoOpAuditLogger`, `NoOpMetricsCollector`, and `NoOpTracer` do nothing and report deterministic results, perfect for unit tests.
- Builders on `AuditLogEntry` (`with_user`, `with_session`, `with_metadata*`) keep call sites concise.

```rust
use pulsearc_common::observability::{
    AuditLogEntry, AuditLogger, AuditSeverity, MetricsCollector, NoOpAuditLogger,
    NoOpMetricsCollector,
};
use std::sync::Arc;

struct Worker<A: AuditLogger, M: MetricsCollector> {
    audit: Arc<A>,
    metrics: Arc<M>,
}

impl<A: AuditLogger, M: MetricsCollector> Worker<A, M> {
    async fn handle(&self, job_id: &str) {
        self.audit
            .log(
                AuditLogEntry::new("job.started", AuditSeverity::Info)
                    .with_metadata("job_id", job_id),
            )
            .await;
        self.metrics.increment_counter("job.started", &[("status", "ok")]);
    }
}

// Tests can opt into no-op implementations without touching production code:
let worker = Worker {
    audit: Arc::new(NoOpAuditLogger),
    metrics: Arc::new(NoOpMetricsCollector),
};
```

When the crate is compiled with the `runtime` feature, `TraceSpan::finish()` emits a `tracing::trace!` event containing duration metadata, allowing adapters to hook into OpenTelemetry or other exporters later.

## Integration Surface

An end-to-end example lives in `crates/common/tests/observability_integration.rs` (requires the `runtime` and `serde` features). It wires together the audit logger, metrics collector, tracer, and error conversion to mimic a classification workflow. The test doubles (`TestAuditLogger`, `TestMetricsCollector`, `TestTracer`) are a good reference if you need richer mocks.

## Feature Flags

| Feature | What it enables |
| --- | --- |
| `serde` | JSON (de)serialization for `ActionHint`, `AuditLogEntry`, `TraceSpan`, and `UiError` |
| `observability` | Pulls in the `tracing` dependency and bundles with the `foundation` feature set |
| `runtime` | Extends the module with async-friendly utilities (Tokio in tests, `TraceSpan::finish` logging) and is required for the integration test |

Enable features when running examples or tests, e.g. `cargo test -p pulsearc-common --features "runtime serde"`.

## Testing & Benchmarks

- Library/unit tests: `cargo test -p pulsearc-common --lib observability`
- Integration test (requires additional features): `cargo test -p pulsearc-common --test observability_integration --features "runtime serde"`
- Benchmarks (optional): `cargo bench -p pulsearc-common observability_bench --features "runtime"`

CI runs these automatically via `make test` / `make ci`, but the commands above are useful when iterating locally.

## Related Material

- `crates/common/src/observability/TRAIT_ABSTRACTIONS.md` dives deeper into designing components around the trait interfaces.
- `crates/common/tests/observability_integration.rs` demonstrates composite usage with realistic test doubles.

## Extending the Module

- Add new `ErrorCode` values in `errors/app.rs` to keep telemetry identifiers stable.
- Expand `PerformanceMetrics` or add new metric families under `metrics/` as additional instrumentation is ported over.
- Build adapters that implement the `AuditLogger`, `MetricsCollector`, or `Tracer` traits for real telemetry backends, and keep the no-op versions available for tests.
