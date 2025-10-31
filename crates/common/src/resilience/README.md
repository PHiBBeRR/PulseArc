# Resilience Module

Generic, library-quality resilience patterns for building fault-tolerant systems. This module provides reusable circuit breakers and retry strategies that work across different domains without coupling to specific error types or observability frameworks.

## Module Layout

```
resilience/
├── README.md
├── circuit_breaker.rs   # Circuit breaker implementation with state management
├── retry.rs             # Generic retry strategies with backoff and jitter
└── mod.rs               # Public re-exports
```

## Feature Highlights

- **Circuit Breaker**: Prevents cascading failures by detecting repeated failures and temporarily blocking requests (Closed → Open → Half-Open state machine).
- **Retry Strategies**: Four backoff types (Fixed, Linear, Exponential, Custom) with four jitter types (None, Full, Equal, Decorrelated).
- **Generic Error Handling**: Works with any `std::error::Error` type via `<E: std::error::Error>`, avoiding domain-specific coupling.
- **Testable Time Abstraction**: `Clock` trait with `SystemClock` (production) and `MockClock` (tests) for deterministic testing without actual delays.
- **Builder Pattern APIs**: Fluent configuration with compile-time validation (`RetryConfigBuilder`, `CircuitBreakerConfigBuilder`).
- **Async-First Design**: All operations return `Future`s for seamless integration with Tokio/async-std.
- **Metrics & Observability**: Circuit breaker metrics (success/failure/rejection counts) and tracing integration.

## Architecture: Generic Library vs Domain-Specific Implementation

This `resilience` module provides **library-quality, generic abstractions** that can be reused across different domains and applications. The implementations are:
- Generic over error types (`<E: std::error::Error>`)
- Flexible with multiple strategies (4 backoff types, 4 jitter types)
- Testable with clock abstraction (`MockClock`)
- Framework-agnostic with minimal dependencies

### Relationship to `sync::retry`

The `common::sync::retry` module contains a **domain-specific implementation** optimized for sync/queue operations. Key differences:

| Feature | `common::resilience` | `common::sync::retry` |
|---------|---------------------|----------------------|
| **Purpose** | Generic library | Sync-specific production code |
| **Error Type** | Generic `<E>` | Concrete `RetryError` |
| **Metrics** | None (generic) | Integrated `RetryMetrics` |
| **Tracing** | Basic logging | Feature-gated spans |
| **Backoff** | 4 strategies | Exponential only |
| **Domain Coupling** | None | Coupled to sync module |
| **Current Usage** | Reserved for future | Active production use |

### When to Use Each

- **Use `common::resilience`**: When adding resilience to **new modules** or **different domains** that need circuit breakers or retry logic. This provides a clean, generic foundation.

- **Use `common::sync::retry`**: When working **within the sync/queue domain** where you need integrated metrics, tracing, and domain-specific error handling.

### Future Direction

Long-term, consider migrating `sync::retry` to use `resilience` as a backend, adding metrics and tracing as wrapper layers. This would eliminate duplication while maintaining the specialized functionality. Priority: LOW (both implementations work well, no bugs reported).

## Crate Features & Dependencies

The resilience module lives inside `pulsearc-common` and is included in the `runtime` feature:

```toml
[dependencies]
pulsearc-common = { path = "crates/common", features = ["runtime"] }
```

Core dependencies:
- `thiserror` — ergonomic error types
- `tracing` — structured logging and instrumentation

## Quick Start

### Circuit Breaker

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig, SystemClock, ResilienceError
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure circuit breaker with 5 failures opening the circuit
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(5)
        .success_threshold(2)
        .timeout(Duration::from_secs(60))
        .half_open_max_calls(3)
        .build()?;

    let breaker = CircuitBreaker::new(config, SystemClock);

    // Wrap operations with circuit breaker
    match breaker.call(|| async {
        // Your fallible operation here
        external_api_call().await
    }).await {
        Ok(result) => println!("Success: {result}"),
        Err(ResilienceError::CircuitOpen) => {
            println!("Circuit open, using fallback");
        }
        Err(e) => println!("Operation failed: {e}"),
    }

    // Check circuit state
    let metrics = breaker.metrics();
    println!("Circuit state: {:?}", metrics.state);
    println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);

    Ok(())
}

async fn external_api_call() -> Result<String, std::io::Error> {
    // Simulate external API call
    Ok("data".to_string())
}
```

Key points:
- Circuit opens after `failure_threshold` consecutive failures.
- After `timeout` duration, circuit transitions to Half-Open state.
- `success_threshold` consecutive successes in Half-Open closes the circuit.
- `half_open_max_calls` limits concurrent requests during recovery testing.

### Retry with Exponential Backoff

```rust
use pulsearc_common::resilience::{
    retry_with_policy, RetryConfig, BackoffStrategy, Jitter, RetryPolicy, RetryDecision
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure exponential backoff with jitter
    let config = RetryConfig::builder()
        .max_attempts(5)
        .backoff(BackoffStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            base: 2.0,
            max_delay: Duration::from_secs(30),
        })
        .jitter(Jitter::Equal)
        .max_total_time(Duration::from_secs(60))
        .build()?;

    // Define retry policy
    struct MyRetryPolicy;
    impl RetryPolicy<std::io::Error> for MyRetryPolicy {
        fn should_retry(&self, error: &std::io::Error, attempt: u32) -> RetryDecision {
            match error.kind() {
                std::io::ErrorKind::TimedOut => RetryDecision::Retry,
                std::io::ErrorKind::ConnectionRefused => RetryDecision::Retry,
                _ => RetryDecision::Stop,
            }
        }
    }

    // Execute with retry
    let outcome = retry_with_policy(
        config,
        MyRetryPolicy,
        || async {
            fetch_data().await
        }
    ).await;

    match outcome.result {
        Ok(data) => {
            println!("Success after {} attempts", outcome.attempts);
            println!("Total delay: {:?}", outcome.total_delay);
        }
        Err(e) => println!("Failed: {e}"),
    }

    Ok(())
}

async fn fetch_data() -> Result<String, std::io::Error> {
    // Simulate network call
    Ok("data".to_string())
}
```

Highlights:
- Backoff delay grows exponentially: 100ms → 200ms → 400ms → 800ms → 1600ms (capped at `max_delay`).
- `Jitter::Equal` randomizes between 50%-100% of calculated delay to prevent thundering herd.
- `RetryPolicy` trait lets you customize which errors are retryable.
- `max_total_time` prevents infinite retry loops.

### Backoff Strategies Comparison

| Strategy | Formula | Use Case |
|----------|---------|----------|
| **Fixed** | `constant` | Simple rate limiting, predictable intervals |
| **Linear** | `initial + (attempt × increment)` | Gradual backoff, bounded growth |
| **Exponential** | `initial × base^attempt` | Default choice, aggressive backoff |
| **Custom** | `fn(attempt) -> Duration` | Complex logic, domain-specific patterns |

### Jitter Types Comparison

| Jitter Type | Range | Best For |
|-------------|-------|----------|
| **None** | `delay` | Testing, deterministic behavior |
| **Full** | `[0, delay]` | Maximum randomization, avoid thundering herd |
| **Equal** | `[delay/2, delay]` | Balanced randomization (recommended default) |
| **Decorrelated** | `[base, prev_delay × 3]` | AWS-style jitter, sophisticated randomization |

## Advanced Usage

### Thread-Safe Circuit Breaker with Arc

```rust
use pulsearc_common::resilience::{SyncCircuitBreaker, CircuitBreakerConfig, SystemClock};
use std::sync::Arc;

let config = CircuitBreakerConfig::builder()
    .failure_threshold(3)
    .timeout(Duration::from_secs(30))
    .build()?;

let breaker = Arc::new(SyncCircuitBreaker::new(config, SystemClock));

// Share across threads/tasks
let breaker_clone = breaker.clone();
tokio::spawn(async move {
    breaker_clone.call(|| async {
        // operation
        Ok::<_, std::io::Error>(())
    }).await
});
```

### Custom Retry Policy with Contextual Logic

```rust
use pulsearc_common::resilience::{RetryPolicy, RetryDecision};
use std::time::Duration;

struct AdaptiveRetryPolicy {
    max_retries: u32,
}

impl<E: std::error::Error> RetryPolicy<E> for AdaptiveRetryPolicy {
    fn should_retry(&self, error: &E, attempt: u32) -> RetryDecision {
        if attempt >= self.max_retries {
            return RetryDecision::Stop;
        }

        // Inspect error message for retry hints
        let error_msg = error.to_string().to_lowercase();

        if error_msg.contains("rate limit") {
            // Longer delay for rate limits
            return RetryDecision::RetryAfter(Duration::from_secs(60));
        }

        if error_msg.contains("temporary") || error_msg.contains("timeout") {
            return RetryDecision::Retry;
        }

        // Don't retry on permanent failures
        RetryDecision::Stop
    }
}
```

### Combining Circuit Breaker + Retry

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig, retry, RetryConfig, SystemClock
};

async fn resilient_call<T, E>(operation: impl Fn() -> impl Future<Output = Result<T, E>>)
    -> Result<T, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let breaker_config = CircuitBreakerConfig::builder()
        .failure_threshold(5)
        .build()?;
    let breaker = CircuitBreaker::new(breaker_config, SystemClock);

    let retry_config = RetryConfig::builder()
        .max_attempts(3)
        .build()?;

    // Circuit breaker wraps retry logic
    breaker.call(|| async {
        retry(retry_config.clone(), || operation()).await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
```

## Testing with MockClock

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig, MockClock, CircuitState
};
use std::time::Duration;

#[tokio::test]
async fn test_circuit_breaker_timeout() {
    let clock = MockClock::new();
    let config = CircuitBreakerConfig::builder()
        .failure_threshold(3)
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();

    let breaker = CircuitBreaker::new(config, clock.clone());

    // Trigger 3 failures to open circuit
    for _ in 0..3 {
        let _ = breaker.call(|| async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
        }).await;
    }

    assert_eq!(breaker.metrics().state, CircuitState::Open);

    // Advance time by 60 seconds
    clock.advance(Duration::from_secs(60));

    // Circuit should transition to Half-Open
    assert_eq!(breaker.metrics().state, CircuitState::HalfOpen);
}
```

## Configuration Best Practices

### Circuit Breaker Tuning

```rust
// High-throughput service
let config = CircuitBreakerConfig::builder()
    .failure_threshold(50)           // Tolerate more failures
    .success_threshold(10)           // Require more successes to close
    .timeout(Duration::from_secs(30))  // Shorter recovery window
    .half_open_max_calls(10)         // More concurrent tests
    .build()?;

// Critical low-latency service
let config = CircuitBreakerConfig::builder()
    .failure_threshold(3)            // Fail fast
    .success_threshold(2)            // Quick recovery
    .timeout(Duration::from_secs(5)) // Rapid retry
    .half_open_max_calls(1)          // Conservative testing
    .build()?;
```

### Retry Configuration Patterns

```rust
// Fast fail for user-facing APIs
let config = RetryConfig::builder()
    .max_attempts(3)
    .backoff(BackoffStrategy::Fixed(Duration::from_millis(50)))
    .max_total_time(Duration::from_millis(500))
    .build()?;

// Resilient background job
let config = RetryConfig::builder()
    .max_attempts(10)
    .backoff(BackoffStrategy::Exponential {
        initial_delay: Duration::from_secs(1),
        base: 2.0,
        max_delay: Duration::from_secs(300),
    })
    .jitter(Jitter::Decorrelated {
        base: Duration::from_secs(1)
    })
    .max_total_time(Duration::from_secs(3600))
    .build()?;
```

## Performance Characteristics

| Operation | Time Complexity | Allocations | Thread-Safe |
|-----------|----------------|-------------|-------------|
| Circuit breaker state check | O(1) | Zero | Yes (atomic) |
| Circuit breaker call | O(1) + operation | Minimal | Yes |
| Retry delay calculation | O(1) | Zero | N/A |
| Retry with backoff | O(attempts) | Per-attempt | Yes |

## Error Handling

The module uses strongly-typed errors via `thiserror`:

```rust
pub enum ResilienceError<E: std::error::Error> {
    CircuitOpen,                      // Circuit breaker is open
    Timeout { timeout: Duration },    // Operation timed out
    OperationFailed { source: E },    // Underlying operation failed
    InvalidConfiguration { message }, // Config validation failed
}

pub enum RetryError<E> {
    AttemptsExhausted { attempts: u32 },  // All retries failed
    NonRetryable { source: E },           // Error is not retryable
    InvalidConfiguration { message },     // Config validation failed
    TimeoutExceeded { elapsed: Duration }, // Max total time exceeded
}
```

## Testing and Benchmarks

```bash
# Run all resilience tests
cargo test -p pulsearc-common resilience

# Run with coverage
cargo test -p pulsearc-common resilience --features runtime

# Run benchmarks (if available)
cargo bench -p pulsearc-common --features runtime --bench resilience_bench
```

## Related Modules

- `crates/common/src/sync/retry` — domain-specific retry implementation for sync/queue operations with integrated metrics.
- `crates/common/src/observability` — metrics and tracing utilities that complement resilience patterns.
- `crates/common/src/time` — duration formatting and timer utilities.

## Migration Guide

### From `sync::retry` to `resilience::retry`

If you're starting a new module and considering which retry implementation to use:

```rust
// OLD: Domain-specific sync retry (keep for sync/queue operations)
use pulsearc_common::sync::retry::{RetryExecutor, RetryConfig};

// NEW: Generic resilience retry (use for new modules)
use pulsearc_common::resilience::{retry_with_policy, RetryConfig, RetryPolicy};
```

**When to migrate:** Only if you're building a new system and want generic, reusable abstractions. Existing `sync::retry` usage is fine and should not be changed without good reason.

## License

PulseArc is dual-licensed under MIT and Apache 2.0. See the repository root for the full text.
