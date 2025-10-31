# Observability Infrastructure

Unified observability primitives for monitoring, error handling, and metrics tracking across the agent platform.

## Overview

This module consolidates all observability concerns into a cohesive system that provides comprehensive monitoring capabilities, structured error handling with actionable hints, and flexible metric collection. It enables developers to track application health, diagnose issues, and measure performance systematically.

## Features

- **Structured Error Handling**: Hierarchical error types with error codes and recovery hints
- **Metrics Collection**: Performance and classification metrics with trait abstractions
- **Trait-Based Integration**: Pluggable audit logging, metrics collectors, and distributed tracing
- **No-Op Implementations**: Built-in test doubles for development and testing
- **Error Recovery Hints**: Actionable suggestions for error resolution
- **Frontend-Safe Errors**: UiError type for safe serialization to frontend
- **Retry Classification**: `AppError::is_retryable()` flags transient failures for automated recovery

## Architecture

```text
┌─────────────────────────────────────┐
│       Observability Module          │
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐   │
│  │   Errors (errors/)          │   │
│  │  • AppError (top-level)     │   │
│  │  • AiError (OpenAI)         │   │
│  │  • HttpError (network)      │   │
│  │  • MetricsError             │   │
│  │  • ErrorCode (enum)         │   │
│  │  • ActionHint (recovery)    │   │
│  │  • UiError (frontend-safe)  │   │
│  └─────────────────────────────┘   │
│                                     │
│  ┌─────────────────────────────┐   │
│  │   Metrics (metrics/)        │   │
│  │  • ClassificationMetrics    │   │
│  │  • MetricsTracker           │   │
│  │  • PerformanceMetrics       │   │
│  └─────────────────────────────┘   │
│                                     │
│  ┌─────────────────────────────┐   │
│  │   Traits (traits.rs)        │   │
│  │  • AuditLogger              │   │
│  │  • MetricsCollector         │   │
│  │  • Tracer                   │   │
│  │  • No-op implementations    │   │
│  └─────────────────────────────┘   │
│                                     │
└─────────────────────────────────────┘
```

## Components

### 1. Error System (`errors/`)

Comprehensive error handling with hierarchical error types, stable error codes for telemetry, and actionable recovery hints.

#### Error Types

**AppError** - Top-level application error that wraps all other error types:
- `Ai(AiError)` - AI/OpenAI related errors
- `Http(HttpError)` - HTTP/network errors
- `Metrics(MetricsError)` - Metrics collection errors
- `Serde(String)` - Serialization errors
- `Io(String)` - I/O errors
- `Validation(String)` - Validation errors
- `Other(String)` - Unexpected errors

**AiError** - OpenAI and AI service errors:
- `RateLimited` - Rate limit exceeded (with retry timing)
- `InvalidApiKey` - Invalid API credentials
- `QuotaExceeded` - Quota/billing limit reached
- `ModelNotFound` - Requested model unavailable
- `BadRequest` - Invalid API request
- `Timeout` - Request timeout
- `ServerError` - AI service error
- `ContentPolicyViolation` - Content policy violation
- `ParseResponse` - Failed to parse response
- `OutputInvalidSchema` - Output validation failed
- `TokenLimitExceeded` - Token limit exceeded

**HttpError** - Network and HTTP errors:
- `Network` - Network connectivity issue
- `Timeout` - HTTP timeout
- `Unauthorized` - 401 Unauthorized
- `Forbidden` - 403 Forbidden
- `TooManyRequests` - 429 Rate limited
- `ServerError` - 5xx server error
- `Status` - Other HTTP status

**MetricsError** - Metrics collection errors:
- `CollectionFailed` - Metrics collection failed
- `TrackerUnavailable` - Metrics tracker unavailable
- `Other` - Other metrics error

#### Error Codes

Stable error codes for telemetry and monitoring:

```rust
pub enum ErrorCode {
    // Database
    DbOpenFailed,
    DbQueryFailed,
    DbBusy,
    DbTimeout,
    DbIntegrityFailed,

    // AI
    AiRateLimited,
    AiInvalidApiKey,
    AiQuotaExceeded,
    AiModelNotFound,
    AiBadRequest,
    AiTimeout,
    AiServerError,
    AiContentPolicyViolation,
    AiParseResponseFailed,
    AiOutputInvalidSchema,
    AiTokenLimitExceeded,

    // HTTP
    HttpNetwork,
    HttpTimeout,
    HttpUnauthorized,
    HttpForbidden,
    HttpTooManyRequests,
    HttpServerError,
    HttpStatus,

    // Metrics
    MetricsCollectionFailed,
    MetricsTrackerUnavailable,

    // Generic
    Serialization,
    Io,
    ValidationFailed,
    Unknown,
}
```

#### Action Hints

Actionable recovery suggestions for errors:

```rust
pub enum ActionHint {
    None,
    RetryAfter { duration: Duration },
    Backoff,
    CheckConfig { key: String },
    CheckNetwork,
    CheckOpenAiKey,
    ReduceBatchSize,
    SwitchModel { model: String },
}
```

> `RetryAfter` uses `std::time::Duration` to avoid unit ambiguity. Serialization continues to emit millisecond values for compatibility.

### 2. Metrics System (`metrics/`)

Performance tracking and classification metrics for monitoring application behavior.

**ClassificationMetrics** - Track AI classification operations
**MetricsTracker** - Collect and aggregate metrics
**PerformanceMetrics** - Performance measurement (placeholder for future expansion)

### 3. Trait Abstractions (`traits.rs`)

Pluggable implementations for audit logging, metrics collection, and distributed tracing.

#### AuditLogger

Log security-relevant events:

```rust
pub trait AuditLogger: Send + Sync + Debug {
    async fn log(&self, event: AuditLogEntry);
    async fn entry_count(&self) -> usize;
    fn is_enabled(&self) -> bool;
}
```

#### MetricsCollector

Emit metrics without depending on specific systems:

```rust
pub trait MetricsCollector: Send + Sync + Debug {
    fn increment_counter(&self, name: &str, labels: &[(&str, &str)]);
    fn record_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    fn record_timing(&self, name: &str, duration_ms: u64, labels: &[(&str, &str)]);
}
```

#### Tracer

Distributed tracing support:

```rust
pub trait Tracer: Send + Sync + Debug {
    async fn start_span(&self, operation: &str, metadata: HashMap<String, String>) -> TraceSpan;
    fn current_span(&self) -> Option<TraceSpan>;
}
```

## Usage Examples

### Error Handling

```rust
use std::time::Duration;

use agent::common::observability::{ActionHint, AppError, AppResult, AiError, ErrorCode};

// Function that returns structured errors
async fn classify_activity(text: &str) -> AppResult<String> {
    // Check rate limits
    if is_rate_limited() {
        return Err(AppError::Ai(AiError::RateLimited {
            retry_after: Some(Duration::from_secs(5)),
        }));
    }

    // Perform classification
    let result = ai_service.classify(text).await?;
    Ok(result)
}

// Handle errors with recovery hints
async fn handle_classification() {
    match classify_activity("Working on project").await {
        Ok(result) => println!("Classification: {}", result),
        Err(error) => {
            // Get error code for telemetry
            let code = error.code();
            println!("Error code: {:?}", code);

            // Get actionable recovery hint
            let action = error.action();
            match action {
                ActionHint::RetryAfter { duration } => {
                    println!("Retry after {:?}", duration);
                    tokio::time::sleep(duration).await;
                    // Retry operation
                }
                ActionHint::CheckOpenAiKey => {
                    println!("Please verify your OpenAI API key");
                }
                ActionHint::ReduceBatchSize => {
                    println!("Reduce batch size and retry");
                }
                _ => println!("Error: {}", error),
            }

            if error.is_retryable() {
                println!("Transient failure detected, scheduling retry");
            }
        }
    }
}
```

### Converting Errors for UI

```rust
use agent::common::observability::{AppError, UiError};

// Convert backend errors to frontend-safe errors
#[tauri::command]
fn handle_command(input: String) -> Result<String, UiError> {
    match process_input(input) {
        Ok(result) => Ok(result),
        Err(app_error) => Err(app_error.to_ui()),
    }
}

// UiError includes:
// - code: ErrorCode for classification
// - message: Human-readable message
// - action: ActionHint for recovery
```

### Audit Logging

```rust
use agent::common::observability::{AuditLogger, AuditLogEntry, AuditSeverity};
use std::collections::HashMap;

async fn log_security_event(logger: &impl AuditLogger) {
    let entry = AuditLogEntry::new("user_login", AuditSeverity::Info)
        .with_user("user123")
        .with_session("session456")
        .with_ip("192.168.1.1")
        .with_metadata("method", "oauth");

    logger.log(entry).await;
}
```

### Metrics Collection

```rust
use agent::common::observability::{MetricsCollector, NoOpMetricsCollector};

fn track_operation_metrics(collector: &impl MetricsCollector) {
    // Increment counter
    collector.increment_counter(
        "classification_requests",
        &[("model", "gpt-4"), ("status", "success")]
    );

    // Record gauge
    collector.record_gauge(
        "active_connections",
        42.0,
        &[("service", "ai")]
    );

    // Record histogram
    collector.record_histogram(
        "response_time_ms",
        125.5,
        &[("endpoint", "/classify")]
    );

    // Record timing
    collector.record_timing(
        "db_query_duration",
        45,
        &[("query", "select")]
    );
}

// Use no-op collector for testing
let collector = NoOpMetricsCollector;
track_operation_metrics(&collector);
```

### Distributed Tracing

```rust
use agent::common::observability::{Tracer, NoOpTracer};
use std::collections::HashMap;

async fn trace_operation(tracer: &impl Tracer) {
    let mut metadata = HashMap::new();
    metadata.insert("user_id".to_string(), "user123".to_string());
    metadata.insert("operation".to_string(), "classification".to_string());

    let span = tracer.start_span("classify_activity", metadata).await;

    // Perform operation
    // ...

    // Mark span complete
    span.finish();
}

// Use no-op tracer for testing
let tracer = NoOpTracer;
trace_operation(&tracer).await;
```

### No-Op Implementations for Testing

```rust
use agent::common::observability::{
    NoOpAuditLogger,
    NoOpMetricsCollector,
    NoOpTracer
};

#[tokio::test]
async fn test_with_noop_observability() {
    // Use no-op implementations that don't perform side effects
    let audit_logger = NoOpAuditLogger;
    let metrics_collector = NoOpMetricsCollector;
    let tracer = NoOpTracer;

    // Test your code without real observability infrastructure
    assert!(!audit_logger.is_enabled());
    assert_eq!(audit_logger.entry_count().await, 0);

    metrics_collector.increment_counter("test", &[]);
    tracer.start_span("test", HashMap::new()).await;
}
```

## API Reference

### Error Types

**AppError**
- `code() -> ErrorCode` - Get stable error code
- `action() -> ActionHint` - Get recovery hint
- `to_ui() -> UiError` - Convert to frontend-safe error

**UiError**
- `from_app_error(error: AppError) -> Self` - Convert from AppError
- `from_message(message: &str) -> Self` - Create from message string

### Trait Methods

**AuditLogger**
- `log(event: AuditLogEntry)` - Log audit event
- `entry_count() -> usize` - Get entry count
- `is_enabled() -> bool` - Check if enabled

**MetricsCollector**
- `increment_counter(name, labels)` - Increment counter
- `record_gauge(name, value, labels)` - Record gauge
- `record_histogram(name, value, labels)` - Record histogram
- `record_timing(name, duration_ms, labels)` - Record timing

**Tracer**
- `start_span(operation, metadata) -> TraceSpan` - Start trace span
- `current_span() -> Option<TraceSpan>` - Get current span

## Testing

### Unit Tests

```bash
# Run all observability tests
cargo test --package agent --lib common::observability

# Run specific module tests
cargo test --package agent --lib common::observability::errors
cargo test --package agent --lib common::observability::metrics
cargo test --package agent --lib common::observability::traits
```

### Integration Testing

Use no-op implementations for integration tests:

```rust
use agent::common::observability::{NoOpAuditLogger, NoOpMetricsCollector};

#[tokio::test]
async fn test_service_integration() {
    let service = MyService::new(
        NoOpAuditLogger,
        NoOpMetricsCollector,
    );

    // Test service without real observability
}
```

## Best Practices

### Error Handling

1. **Use Specific Error Types**: Return specific error variants for better error handling
2. **Provide Context**: Include relevant details in error messages
3. **Use Error Codes**: Leverage stable error codes for telemetry
4. **Check Action Hints**: Use action hints to guide error recovery
5. **Convert for Frontend**: Always use `to_ui()` before sending errors to frontend

### Metrics Collection

1. **Use Consistent Labels**: Standardize label names across metrics
2. **Choose Right Metric Type**:
   - Counters for cumulative values
   - Gauges for current values
   - Histograms for distributions
   - Timings for durations
3. **Avoid High Cardinality**: Limit label value combinations

### Audit Logging

1. **Log Security Events**: Track authentication, authorization, data access
2. **Include Context**: Add user ID, session ID, IP address
3. **Use Appropriate Severity**: Debug, Info, Warning, Error, Critical
4. **Add Metadata**: Include relevant context in metadata map

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
async-trait = "0.1"
chrono = "0.4"
```

## Related Modules

- **agent/common/auth**: Authentication and OAuth handling
- **agent/common/security**: RBAC and encryption
- **agent/common/validation**: Input validation framework
- **agent/storage**: Database and storage operations

## Roadmap

- [ ] Add OpenTelemetry integration
- [ ] Expand PerformanceMetrics with detailed tracking
- [ ] Add structured logging support (tracing crate)
- [ ] Implement metrics aggregation and reporting
- [ ] Add error rate tracking and alerting

## License

See the root LICENSE file for licensing information.
