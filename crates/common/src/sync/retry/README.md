# Retry Module

A production-ready retry mechanism for Rust with exponential backoff, circuit breaker, retry budget, and comprehensive observability support.

## Features

- **Exponential Backoff with Jitter**: Prevents thundering herd problems
- **Circuit Breaker**: Automatic failure detection and recovery
- **Retry Budget**: Token-based rate limiting to prevent retry storms
- **Distributed Tracing**: OpenTelemetry integration for observability
- **Prometheus Metrics**: Built-in metrics export for monitoring
- **Predefined Policies**: Ready-to-use policies for common scenarios
- **Thread-Safe**: All components are safe for concurrent use
- **Async/Sync Support**: Works with both async and blocking code

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Retry Module                   │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌──────────────┐  ┌──────────────────┐       │
│  │   Strategy   │  │ Circuit Breaker  │       │
│  │              │  │                  │       │
│  │ • Backoff    │  │ • State Machine  │       │
│  │ • Jitter     │  │ • Auto Recovery  │       │
│  │ • Timeout    │  │ • Half-Open Test │       │
│  └──────────────┘  └──────────────────┘       │
│                                                 │
│  ┌──────────────┐  ┌──────────────────┐       │
│  │    Budget    │  │    Policies      │       │
│  │              │  │                  │       │
│  │ • Token Pool │  │ • Network        │       │
│  │ • Refill     │  │ • Database       │       │
│  │ • Rate Limit │  │ • API/Rate Limit │       │
│  └──────────────┘  └──────────────────┘       │
│                                                 │
│  ┌──────────────────────────────────────┐      │
│  │         Observability Layer          │      │
│  │                                      │      │
│  │ • OpenTelemetry Tracing             │      │
│  │ • Prometheus Metrics                │      │
│  │ • Structured Logging                │      │
│  └──────────────────────────────────────┘      │
└─────────────────────────────────────────────────┘
```

## Quick Start

### Basic Retry

```rust
use tauri_agent::sync::retry::RetryStrategy;

// Simple retry with defaults
let strategy = RetryStrategy::new();
let result = strategy.execute(|| async {
    // Your operation here
    perform_network_request().await
}).await;
```

### Using Predefined Policies

```rust
use tauri_agent::sync::retry::RetryPolicies;

// Network operations (8 attempts, 500ms-30s backoff)
let strategy = RetryPolicies::network_policy();

// Database operations (5 attempts, 100ms-10s backoff)
let strategy = RetryPolicies::database_policy();

// Rate-limited APIs (10 attempts, fixed 60s delay)
let strategy = RetryPolicies::rate_limit_policy();

// File system operations (3 attempts, 50ms-1s backoff)
let strategy = RetryPolicies::filesystem_policy();

// Idempotent operations (safe to retry many times)
let strategy = RetryPolicies::idempotent_policy();

// Non-idempotent operations (limited retries)
let strategy = RetryPolicies::non_idempotent_policy();
```

### Custom Retry Policy

```rust
use tauri_agent::sync::retry::{RetryStrategy, RetryCondition};
use std::sync::Arc;
use std::time::Duration;

let strategy = RetryStrategy::new()
    .with_max_attempts(5)?
    .with_base_delay(Duration::from_millis(100))?
    .with_max_delay(Duration::from_secs(10))?
    .with_jitter_factor(0.3)
    .with_timeout(Duration::from_secs(60))
    .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
        // Only retry specific errors
        err.to_string().contains("temporary")
    })));
```

### With Circuit Breaker

```rust
use tauri_agent::sync::retry::{RetryStrategy, CircuitBreaker};
use std::sync::Arc;

let circuit_breaker = Arc::new(CircuitBreaker::new()
    .with_failure_threshold(5)      // Open after 5 failures
    .with_success_threshold(2)       // Close after 2 successes
    .with_timeout(Duration::from_secs(60)));  // Recovery timeout

let strategy = RetryStrategy::new();

// Check circuit breaker before attempting
if circuit_breaker.should_allow_request() {
    match strategy.execute(|| async {
        perform_operation().await
    }).await {
        Ok(result) => {
            circuit_breaker.record_success();
            Ok(result)
        }
        Err(err) => {
            circuit_breaker.record_failure();
            Err(err)
        }
    }
} else {
    // Circuit is open, fail fast
    Err(RetryError::CircuitBreakerOpen)
}
```

### With Retry Budget

```rust
use tauri_agent::sync::retry::RetryBudget;
use std::sync::Arc;

// 100 tokens, refill 10 tokens per second
let budget = Arc::new(RetryBudget::new(100, 10.0));

// Try to acquire retry tokens
if budget.try_acquire_multiple(3) {  // Request 3 tokens for up to 3 retries
    let result = strategy.execute(|| async {
        perform_operation().await
    }).await;

    // Return unused tokens if succeeded quickly
    if result.is_ok() {
        budget.return_tokens(2);  // Used only 1 retry
    }
    result
} else {
    // No budget available, fail fast
    Err(RetryError::BudgetExhausted)
}
```

## Observability

### Distributed Tracing (OpenTelemetry)

When the `tracing` feature is enabled, retry operations automatically create OpenTelemetry spans:

```toml
[dependencies]
tauri-agent = { version = "1.0", features = ["tracing"] }
```

Traced information includes:
- Retry attempts with timing
- Failure reasons
- Success/timeout/exhaustion status
- Total delay accumulated

### Prometheus Metrics

When the `prometheus` feature is enabled, metrics are automatically exported:

```toml
[dependencies]
tauri-agent = { version = "1.0", features = ["prometheus"] }
```

```rust
use tauri_agent::sync::retry::metrics_export;
use prometheus::Registry;

// Initialize metrics exporter
let registry = Registry::new();
metrics_export::init_metrics_exporter(&registry)?;

// Metrics are automatically collected during retry operations
```

Available metrics:
- `retry_attempts_total`: Total retry attempts by operation
- `retry_successes_total`: Successful retry operations
- `retry_failures_total`: Failed retry operations with reasons
- `retry_delay_seconds`: Histogram of retry delays
- `circuit_breaker_state`: Current circuit breaker state (0=closed, 1=open, 2=half-open)
- `circuit_breaker_transitions_total`: State transition counter
- `retry_budget_available_tokens`: Available retry budget tokens
- `retry_budget_capacity_tokens`: Maximum budget capacity

### With Metrics Collection

```rust
let (result, metrics) = strategy.execute_with_metrics("api_call", || async {
    call_external_api().await
}).await;

println!("Attempts: {}", metrics.attempts);
println!("Total delay: {:?}", metrics.total_delay);
println!("Success rate: {}", metrics.success_rate());
println!("Average delay: {:?}", metrics.average_delay());
```

## Advanced Usage

### Custom Policy Builder

```rust
use tauri_agent::sync::retry::RetryPolicyBuilder;

let policy = RetryPolicyBuilder::new()
    .max_attempts(5)?
    .base_delay(Duration::from_millis(200))?
    .max_delay(Duration::from_secs(30))?
    .jitter(0.2)
    .timeout(Duration::from_secs(120))
    .when(|err| {
        // Custom retry logic
        match err.downcast_ref::<MyError>() {
            Some(MyError::Temporary) => true,
            Some(MyError::RateLimit) => true,
            _ => false,
        }
    })
    .build();
```

### Synchronous Operations

For blocking/synchronous code:

```rust
let strategy = RetryStrategy::new();

// WARNING: Do not use in async context (will block the runtime)
let result = strategy.execute_sync(|| {
    std::fs::read_to_string("config.toml")
})?;
```

### Circuit Breaker Stats

```rust
let stats = circuit_breaker.stats();
println!("State: {}", stats.state);
println!("Failures: {}", stats.failure_count);
println!("Successes: {}", stats.success_count);
if let Some(last_failure) = stats.last_failure_time {
    println!("Last failure: {:?} ago", last_failure.elapsed());
}
```

## Error Handling

The module provides detailed error types:

```rust
use tauri_agent::sync::retry::RetryError;

match strategy.execute(operation).await {
    Ok(result) => // Success
    Err(RetryError::AttemptsExhausted { attempts }) =>
        // All retry attempts failed
    Err(RetryError::Timeout { elapsed }) =>
        // Operation timed out
    Err(RetryError::CircuitBreakerOpen) =>
        // Circuit breaker is preventing retries
    Err(RetryError::BudgetExhausted) =>
        // No retry budget available
    Err(RetryError::InvalidConfiguration { message }) =>
        // Configuration error
    Err(RetryError::OperationFailed { source }) =>
        // The underlying operation failed
}
```

## Configuration Constants

Default values (can be overridden):

```rust
DEFAULT_MAX_ATTEMPTS: 5
DEFAULT_BASE_DELAY: 1 second
DEFAULT_MAX_DELAY: 60 seconds
DEFAULT_JITTER_FACTOR: 0.3 (30% jitter)
DEFAULT_FAILURE_THRESHOLD: 5 (circuit breaker)
DEFAULT_SUCCESS_THRESHOLD: 2 (circuit breaker)
DEFAULT_CIRCUIT_TIMEOUT: 60 seconds
DEFAULT_HALF_OPEN_REQUESTS: 1
BUDGET_REFILL_INTERVAL: 1 second
```

## Best Practices

1. **Choose the Right Policy**: Use predefined policies when possible
2. **Set Appropriate Timeouts**: Prevent operations from running indefinitely
3. **Use Circuit Breakers**: Protect against cascading failures
4. **Monitor with Metrics**: Track retry behavior in production
5. **Budget Your Retries**: Prevent retry storms during outages
6. **Add Jitter**: Prevent synchronized retries (thundering herd)
7. **Consider Idempotency**: Ensure operations are safe to retry
8. **Log Failures**: Use structured logging for debugging

## Performance Considerations

- **Thread Safety**: All components use atomic operations and minimize lock contention
- **Overflow Protection**: Exponential backoff calculations prevent integer overflow
- **Zero-Cost Abstractions**: Features like tracing have zero overhead when disabled
- **Efficient Token Management**: Retry budget uses lock-free algorithms where possible

## Testing

The module includes comprehensive tests:

```bash
# Run all tests
cargo test --package tauri-agent --lib sync::retry

# Run with all features
cargo test --all-features --package tauri-agent --lib sync::retry

# Run specific test
cargo test --package tauri-agent --lib sync::retry::tests::test_exponential_backoff
```

## Integration Example

```rust
use tauri_agent::sync::retry::{
    RetryPolicies, CircuitBreaker, RetryBudget, RetryMetrics
};
use std::sync::Arc;

pub struct ApiClient {
    retry_strategy: RetryStrategy,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_budget: Arc<RetryBudget>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            retry_strategy: RetryPolicies::api_policy(),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            retry_budget: Arc::new(RetryBudget::new(100, 10.0)),
        }
    }

    pub async fn call_api(&self, endpoint: &str) -> Result<Response, Error> {
        // Check circuit breaker
        if !self.circuit_breaker.should_allow_request() {
            return Err(Error::ServiceUnavailable);
        }

        // Check retry budget
        if !self.retry_budget.try_acquire() {
            return Err(Error::TooManyRequests);
        }

        // Execute with retry
        let (result, metrics) = self.retry_strategy
            .execute_with_metrics(endpoint, || async {
                self.make_http_request(endpoint).await
            })
            .await;

        // Update circuit breaker
        match &result {
            Ok(_) => self.circuit_breaker.record_success(),
            Err(_) => self.circuit_breaker.record_failure(),
        }

        // Return unused budget if succeeded quickly
        if metrics.attempts == 1 {
            self.retry_budget.return_tokens(1);
        }

        // Log metrics
        tracing::info!(
            endpoint = endpoint,
            attempts = metrics.attempts,
            delay = ?metrics.total_delay,
            success = metrics.succeeded,
            "API call completed"
        );

        result.map_err(|e| Error::from(e))
    }
}
```

## Contributing

When adding new features:

1. Maintain thread safety
2. Add comprehensive tests
3. Update documentation
4. Consider backward compatibility
5. Add metrics/tracing support

## License

This module is part of the Tauri Agent project.