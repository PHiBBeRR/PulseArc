# Error Infrastructure

The `crates/common/src/error/` module provides the shared error handling primitives for the PulseArc Rust workspace. It ships inside the `pulsearc_common` crate and is compiled whenever the crate is built with the `foundation` feature. The goal is to keep error handling consistent across crates, reduce duplication, and make retry/telemetry decisions uniform.

## Quick Start

### Use the shared result alias

```rust
use pulsearc_common::{CommonError, CommonResult};

fn load_config(path: &str) -> CommonResult<serde_json::Value> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| CommonError::persistence_op("read_config", err.to_string()))?;

    let config: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|err| CommonError::serialization_format("JSON", err.to_string()))?;

    Ok(config)
}
```

### Wrap module-specific errors

```rust
use pulsearc_common::{CommonError, ErrorClassification, ErrorSeverity};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WidgetError {
    #[error("widget validation failed: {0}")]
    Invalid(String),

    #[error(transparent)]
    Common(#[from] CommonError),
}

pulsearc_common::impl_error_conversion!(WidgetError, Common);

pulsearc_common::impl_error_classification!(WidgetError, Common,
    WidgetError::Invalid(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    }
);
```

The generated implementations let callers convert common Rust errors into `WidgetError`, share retry/severity logic, and keep `CommonError` available as the catch-all variant.

## Core Building Blocks

### CommonError and CommonResult

`CommonError` is an enum covering the failure patterns we see across crates, while `CommonResult<T>` is a `Result<T, CommonError>` alias. Prefer using the builders below to construct variants so formatting and metadata stay consistent.

| Variant | Helper constructors | Typical usage |
| --- | --- | --- |
| `Config` | `config`, `config_field` | Invalid settings, missing configuration keys |
| `Lock` | `lock`, `lock_resource` | Mutex poisoning, contention, coordination failures |
| `CircuitBreakerOpen` | `circuit_breaker`, `circuit_breaker_with_retry` | Circuit breaker blocks downstream calls |
| `Serialization` | `serialization`, `serialization_format` | JSON/TOML parsing errors, encoding failures |
| `Persistence` | `persistence`, `persistence_op` | File, database, or cache write/read issues |
| `RateLimitExceeded` | `rate_limit`, `rate_limit_detailed` | API or service throttling, quota exhaustion |
| `Timeout` | `timeout` | Long-running operations exceeding a deadline |
| `Backend` | `backend(service, message, is_retryable)` | Upstream service failures; mark retryable accordingly |
| `Validation` | `validation`, `validation_with_value` | Input validation or business rule violations |
| `NotFound` | `not_found`, `not_found_with_id` | Missing resources, absent records |
| `Unauthorized` | `unauthorized`, `unauthorized_with_perm` | Permission or auth failures |
| `Internal` | `internal`, `internal_with_context` | Invariants broken, unexpected conditions |
| `Storage` | construct via `CommonError::Storage { .. }` | Lower-level storage engine faults |
| `Detailed` | construct via `CommonError::Detailed { .. }` | Pre-classified errors with explicit severity/context |
| `TaskCancelled` | `task_cancelled`, `task_cancelled_with_reason` | Cooperative task cancellation scenarios |
| `AsyncTimeout` | `async_timeout` | Structured timeouts for async workflows |

Additional helpers include `with_additional_context` (currently a no-op placeholder) and `as_tracing_fields`, which turns any variant into key/value pairs for structured logging.

### ErrorSeverity

`ErrorSeverity` expresses the impact of an error:

- `Info` – benign situations (e.g., cache miss, expected empty result)
- `Warning` – degraded behavior worth monitoring (e.g., throttling, transient network issues)
- `Error` – actionable failures (e.g., validation, persistence, backend problems)
- `Critical` – conditions threatening system integrity (e.g., invariants violated, data corruption)

Severity automatically propagates through `ErrorClassification` and the logging helpers.

### ErrorClassification

`ErrorClassification` gives every error a retry policy and severity signature. Implementing it (or delegating through the macro) unlocks:

- `is_retryable()` for retry loops and job runners
- `severity()` for metrics and alerting
- `is_critical()` for escalations
- `retry_after()` for backoff hints (rate limiting, circuit breaker windows)

Delegating to the `Common` variant ensures common patterns stay aligned.

### ErrorContext

Implement `ErrorContext` on module error enums when you want to capture additional context while still leveraging `CommonError`. It defines:

- `from_common` – how to wrap a `CommonError`
- `with_context` – optional context enrichment (e.g., annotate which subsystem raised the error)

See `crates/common/tests/error_integration.rs` for a full example of chaining contexts.

## Conversion Helpers & Macros

- `impl_error_conversion!(Type, Variant)` generates `From<serde_json::Error>` and `From<std::io::Error>` implementations by routing them through `CommonError`. Use `impl_error_conversion!(Type, Variant, with_common)` when you cannot add `#[from] CommonError` to the variant and need an explicit `From<CommonError>` impl.
- `impl_error_classification!(Type, Variant, …)` implements `ErrorClassification` by delegating the shared variant and letting you describe module-specific cases inline. You can add an optional `retry_after: Some(Duration)` expression per pattern when needed.

In addition, `CommonError` already implements:

- `From<serde_json::Error>`
- `From<std::io::Error>`
- `From<toml::ser::Error>` and `From<toml::de::Error>` when the `foundation` feature is enabled

These conversions keep serde/toml/io failures aligned with the common variant constructors.

## Logging & Observability

- `CommonError::as_tracing_fields()` returns a vector of static keys and stringified values, ready to pass into `tracing` spans or structured logs.
- `error_type` identifies the variant (`timeout`, `validation`, etc.) for dashboards.
- `retry_after()` and `is_retryable()` can feed backoff metrics or job orchestration.
- Forwarding `ErrorSeverity` into logs keeps severity levels consistent with alerting thresholds.

## Migration Checklist

When updating a module to use this infrastructure:

- [ ] Replace duplicate enum variants with a `Common(#[from] CommonError)` variant.
- [ ] Use the appropriate constructor helper instead of rolling custom messages.
- [ ] Call `impl_error_conversion!` so serde/io errors convert automatically.
- [ ] Implement `ErrorClassification` (typically through the macro) to expose retry/alert semantics.
- [ ] Update call sites to build `CommonError` instances rather than deleted variants.
- [ ] Adjust pattern matches to account for the `Common` wrapper where needed.
- [ ] Add or refresh tests for new behaviors, then run `cargo test -p pulsearc-common --features foundation error_integration` (or `make test`) before submitting changes.

## Tests & Further Reading

- Unit tests live alongside the implementation in `crates/common/src/error/mod.rs`.
- Integration coverage is in `crates/common/tests/error_integration.rs` and exercises the macros, classification logic, and retry helpers.
- `crates/common/src/lib.rs` re-exports `CommonError`, `CommonResult`, `ErrorClassification`, `ErrorContext`, and `ErrorSeverity` under the `foundation` feature.

Keep this document updated whenever new variants, constructors, or macros are added so downstream crates stay in sync.
