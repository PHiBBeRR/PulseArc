# Resilience Module

Production-grade, library-quality resilience patterns for building fault-tolerant distributed systems. This module provides comprehensive, reusable implementations of proven resilience patterns that work across different domains without coupling to specific error types or observability frameworks.

## Module Layout

```
resilience/
├── README.md
├── adaptive.rs          # Adaptive circuit breaker with self-adjusting thresholds
├── bulkhead.rs          # Bulkhead pattern for limiting concurrent operations
├── circuit_breaker.rs   # Circuit breaker with state management
├── histogram.rs         # Latency histogram for percentile tracking
├── rate_limiter.rs      # Token bucket and leaky bucket rate limiters
├── retry.rs             # Generic retry strategies with backoff and jitter
└── mod.rs               # Public re-exports
```

## Feature Highlights

### Core Patterns
- **Circuit Breaker**: Prevents cascading failures by detecting repeated failures and temporarily blocking requests (Closed → Open → Half-Open state machine).
- **Adaptive Circuit Breaker**: Self-adjusting thresholds based on observed error rates and latency patterns.
- **Retry Strategies**: Four backoff types (Fixed, Linear, Exponential, Custom) with four jitter types (None, Full, Equal, Decorrelated).
- **Rate Limiting**: Token bucket (burst-tolerant) and leaky bucket (smooth rate) algorithms.
- **Bulkhead**: Limits concurrent operations to prevent resource exhaustion.
- **Latency Histogram**: Logarithmic bucketing for efficient percentile tracking (p50, p95, p99, p999).

### Technical Excellence
- **Generic Error Handling**: Works with any `std::error::Error` type via `<E: std::error::Error>`, avoiding domain-specific coupling.
- **Testable Time Abstraction**: `Clock` trait with `SystemClock` (production) and `MockClock` (tests) for deterministic testing without actual delays.
- **Builder Pattern APIs**: Fluent configuration with compile-time validation for all components.
- **Async-First Design**: All operations return `Future`s for seamless integration with Tokio/async-std.
- **Comprehensive Metrics**: Success rates, failure rates, latency percentiles, utilization, and more.
- **Thread-Safe**: All patterns use lock-free atomics where possible for high-performance concurrent access.
- **Instant-Based Timing**: Uses `std::time::Instant` throughout for accurate latency measurements.

## Architecture: Generic Library vs Domain-Specific Implementation

This `resilience` module provides **library-quality, generic abstractions** that can be reused across different domains and applications. The implementations are:
- Generic over error types (`<E: std::error::Error>`)
- Flexible with multiple strategies (4 backoff types, 4 jitter types, 2 rate limiters)
- Testable with clock abstraction (`MockClock`)
- Framework-agnostic with minimal dependencies

### Relationship to `sync::retry`

The `common::sync::retry` module contains a **domain-specific implementation** optimized for sync/queue operations. Key differences:

| Feature | `common::resilience` | `common::sync::retry` |
|---------|---------------------|----------------------|
| **Purpose** | Generic library | Sync-specific production code |
| **Error Type** | Generic `<E>` | Concrete `RetryError` |
| **Metrics** | Rich, extensible | Integrated `RetryMetrics` |
| **Tracing** | `tracing` with spans | Feature-gated spans |
| **Backoff** | 4 strategies | Exponential only |
| **Domain Coupling** | None | Coupled to sync module |
| **Patterns** | 6 patterns | Retry only |
| **Current Usage** | Ready for adoption | Active production use |

### When to Use Each

- **Use `common::resilience`**: When adding resilience to **new modules** or **different domains** that need circuit breakers, retry logic, rate limiting, or bulkheads. This provides a clean, generic foundation.

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
- `tokio` — async runtime and semaphore for bulkhead

## Quick Start

### Circuit Breaker

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig, ResilienceError
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

    let breaker = CircuitBreaker::new(config)?;

    // Wrap operations with circuit breaker
    match breaker.execute(|| async {
        // Your fallible operation here
        external_api_call().await
    }).await {
        Ok(result) => println!("Success: {result}"),
        Err(ResilienceError::CircuitOpen) => {
            println!("Circuit open, using fallback");
        }
        Err(e) => println!("Operation failed: {e}"),
    }

    // Check circuit state and metrics
    let metrics = breaker.metrics();
    println!("Circuit state: {}", metrics.state);
    println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
    println!("Time in current state: {:?}", metrics.time_in_current_state());
    println!("{}", metrics.status_message());

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
- Enhanced metrics include success/failure rates, time in state, and human-readable status.

### Adaptive Circuit Breaker

```rust
use pulsearc_common::resilience::{
    AdaptiveCircuitBreaker, AdaptiveCircuitBreakerConfig
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure adaptive circuit breaker
    let config = AdaptiveCircuitBreakerConfig::builder()
        .initial_failure_threshold(5)
        .min_failure_threshold(2)
        .max_failure_threshold(20)
        .target_error_rate(0.05)  // Target 5% error rate
        .window_size(100)          // Track last 100 operations
        .adjustment_interval(Duration::from_secs(60))
        .build()?;

    let breaker = AdaptiveCircuitBreaker::new(config)?;

    // Execute operations - threshold auto-adjusts based on error rate
    let result = breaker.execute(|| async {
        external_api_call().await
    }).await?;

    // Check adaptive metrics
    let metrics = breaker.metrics();
    println!("Current threshold: {}", metrics.current_failure_threshold);
    println!("Recent error rate: {:.2}%", metrics.recent_error_rate * 100.0);
    println!("Threshold adjustments: {}", metrics.threshold_adjustments);
    println!("p50 latency: {:?}", metrics.latency_p50);
    println!("p99 latency: {:?}", metrics.latency_p99);

    Ok(())
}
```

How it works:
- Tracks recent operation results in a sliding window
- Calculates actual error rate from recent operations
- Compares to target error rate (e.g., 5%)
- Adjusts failure threshold automatically:
  - If error rate > target → decrease threshold (more sensitive)
  - If error rate < target/2 → increase threshold (less sensitive)
- Integrated latency histogram tracks p50, p95, p99, p999

### Retry with Exponential Backoff

```rust
use pulsearc_common::resilience::{
    retry_with_policy, RetryConfig, policies::AlwaysRetry
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure exponential backoff with jitter
    let config = RetryConfig::builder()
        .max_attempts(5)
        .exponential_backoff(
            Duration::from_millis(100),
            2.0,
            Duration::from_secs(30)
        )
        .equal_jitter()
        .max_total_time(Duration::from_secs(60))
        .build()?;

    // Execute with retry
    let result = retry_with_policy(config, AlwaysRetry, || async {
        fetch_data().await
    }).await?;

    println!("Success: {result}");

    Ok(())
}

async fn fetch_data() -> Result<String, std::io::Error> {
    // Simulate network call
    Ok("data".to_string())
}
```

Enhanced retry features:
- Backoff delay grows exponentially: 100ms → 200ms → 400ms → 800ms → 1600ms (capped at `max_delay`)
- `equal_jitter()` randomizes between 50%-100% of calculated delay to prevent thundering herd
- `max_total_time` prevents infinite retry loops
- **New**: `first_attempt_time` and `last_error` in `RetryOutcome` for debugging
- **New**: `total_elapsed()` and `average_delay()` metrics

### Rate Limiting

```rust
use pulsearc_common::resilience::{TokenBucket, LeakyBucket};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Token Bucket: allows bursts up to capacity
    let token_bucket = TokenBucket::new(
        100,                           // capacity: max 100 tokens
        10,                            // refill: 10 tokens
        Duration::from_secs(1)         // interval: per second
    )?;

    if token_bucket.try_acquire(5) {
        println!("Request allowed (5 tokens acquired)");
        println!("Available: {}", token_bucket.available_tokens());
    } else {
        println!("Rate limit exceeded");
    }

    // Leaky Bucket: enforces smooth constant rate
    let leaky_bucket = LeakyBucket::new(
        100,    // capacity: max 100 pending requests
        10.0    // leak rate: 10 requests per second
    )?;

    if leaky_bucket.try_acquire() {
        println!("Request allowed");
        println!("Current level: {:.2}", leaky_bucket.current_level());
    } else {
        println!("Bucket full");
    }

    Ok(())
}
```

Key differences:
- **Token Bucket**: Allows bursts (good for APIs with occasional spikes)
- **Leaky Bucket**: Smooth rate enforcement (good for protecting downstream services)

### Bulkhead Pattern

```rust
use pulsearc_common::resilience::{Bulkhead, BulkheadConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure bulkhead to limit concurrent operations
    let config = BulkheadConfig::builder()
        .max_concurrent(10)           // Max 10 concurrent operations
        .max_queue(20)                // Max 20 waiting in queue
        .acquire_timeout(Duration::from_secs(5))
        .build()?;

    let bulkhead = Bulkhead::new(config);

    // Execute with concurrency limiting
    let result = bulkhead.execute(|| async {
        // This operation won't run if 10 others are already running
        expensive_operation().await
    }).await?;

    // Check utilization
    let metrics = bulkhead.metrics();
    println!("Utilization: {:.1}%", metrics.utilization() * 100.0);
    println!("Rejection rate: {:.1}%", metrics.rejection_rate() * 100.0);
    println!("{}", metrics.status_message());

    Ok(())
}

async fn expensive_operation() -> Result<String, std::io::Error> {
    // Simulate expensive operation
    Ok("result".to_string())
}
```

Bulkhead benefits:
- Prevents resource exhaustion from too many concurrent operations
- Queue management for overflow traffic
- Timeout support for acquiring permits
- Rich metrics: utilization, rejection rate, queue depth

### Latency Histogram

```rust
use pulsearc_common::resilience::Histogram;
use std::time::Instant;

fn main() {
    let histogram = Histogram::new();

    // Record latencies
    for _ in 0..1000 {
        let start = Instant::now();
        // ... do work ...
        std::thread::sleep(std::time::Duration::from_millis(10));
        histogram.record_since(start);
    }

    // Get statistics
    let snapshot = histogram.snapshot();
    println!("Count: {}", snapshot.count());
    println!("Mean: {:?}", snapshot.mean());
    println!("Min: {:?}", snapshot.min());
    println!("Max: {:?}", snapshot.max());
    println!("p50: {:?}", snapshot.percentile(0.5));
    println!("p95: {:?}", snapshot.percentile(0.95));
    println!("p99: {:?}", snapshot.percentile(0.99));
    println!("p999: {:?}", snapshot.percentile(0.999));
    println!("Stddev: {:?}", snapshot.stddev());

    // Or use convenience methods
    let percentiles = snapshot.percentiles();
    println!("{}", percentiles.format());

    // Human-readable summary
    println!("{}", snapshot.summary());
    // Output: count=1000, mean=10.23ms, min=10.01ms, max=11.45ms, p50=10.15ms, p99=10.89ms
}
```

Histogram features:
- Logarithmic bucketing (1µs to ~1 hour)
- Lock-free recording using atomics
- Percentile calculation (any percentile from 0.0 to 1.0)
- Mean, min, max, standard deviation
- Thread-safe and cloneable

## Advanced Usage

### Thread-Safe Circuit Breaker with Arc

```rust
use pulsearc_common::resilience::{CircuitBreaker, CircuitBreakerConfig};
use std::sync::Arc;
use std::time::Duration;

let config = CircuitBreakerConfig::builder()
    .failure_threshold(3)
    .timeout(Duration::from_secs(30))
    .build()?;

let breaker = Arc::new(CircuitBreaker::new(config)?);

// Share across threads/tasks
let breaker_clone = breaker.clone();
tokio::spawn(async move {
    let _ = breaker_clone.execute(|| async {
        Ok::<_, std::io::Error>(())
    }).await;
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

### Combining Multiple Patterns

```rust
use pulsearc_common::resilience::{
    CircuitBreaker, CircuitBreakerConfig,
    Bulkhead, BulkheadConfig,
    TokenBucket,
    retry_with_policy, RetryConfig, policies::AlwaysRetry
};
use std::sync::Arc;
use std::time::Duration;

async fn ultra_resilient_call<T, E>(
    operation: impl Fn() -> impl Future<Output = Result<T, E>>
) -> Result<T, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static + Clone,
{
    // Layer 1: Rate limiting (protect the service)
    let rate_limiter = TokenBucket::new(100, 10, Duration::from_secs(1))?;
    if !rate_limiter.try_acquire(1) {
        return Err("Rate limit exceeded".into());
    }

    // Layer 2: Bulkhead (limit concurrency)
    let bulkhead_config = BulkheadConfig::builder()
        .max_concurrent(10)
        .build()?;
    let bulkhead = Arc::new(Bulkhead::new(bulkhead_config));

    // Layer 3: Circuit breaker (fail fast on repeated failures)
    let cb_config = CircuitBreakerConfig::builder()
        .failure_threshold(5)
        .build()?;
    let breaker = Arc::new(CircuitBreaker::new(cb_config)?);

    // Layer 4: Retry (handle transient failures)
    let retry_config = RetryConfig::builder()
        .max_attempts(3)
        .exponential_backoff(Duration::from_millis(100), 2.0, Duration::from_secs(10))
        .build()?;

    // Execute through all layers
    bulkhead.execute(|| {
        let breaker = Arc::clone(&breaker);
        async move {
            breaker.execute(|| async {
                retry_with_policy(retry_config.clone(), AlwaysRetry, || operation()).await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            }).await
        }
    }).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
```

### Monitoring with Histograms

```rust
use pulsearc_common::resilience::{Histogram, CircuitBreaker};
use std::sync::Arc;
use std::time::Instant;

let breaker = Arc::new(CircuitBreaker::with_defaults());
let latency_histogram = Arc::new(Histogram::new());

// Record operation latencies
let start = Instant::now();
match breaker.execute(|| async {
    // operation
    Ok::<_, std::io::Error>(())
}).await {
    Ok(_) => {
        latency_histogram.record_since(start);
    }
    Err(_) => {
        // Still record failed operation latencies
        latency_histogram.record_since(start);
    }
}

// Periodic metrics reporting
tokio::spawn({
    let histogram = Arc::clone(&latency_histogram);
    async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let snapshot = histogram.snapshot();
            println!("Latency stats: {}", snapshot.summary());
        }
    }
});
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

    let breaker = CircuitBreaker::with_clock(config, clock.clone()).unwrap();

    // Trigger 3 failures to open circuit
    for _ in 0..3 {
        let _ = breaker.execute(|| async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
        }).await;
    }

    assert_eq!(breaker.state(), CircuitState::Open);

    // Advance time by 60 seconds (no actual waiting!)
    clock.advance(Duration::from_secs(60));

    // Circuit should transition to Half-Open on next call
    assert!(breaker.can_execute());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
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

### Adaptive Circuit Breaker Tuning

```rust
// API Gateway (high variability)
let config = AdaptiveCircuitBreakerConfig::builder()
    .initial_failure_threshold(10)
    .min_failure_threshold(5)
    .max_failure_threshold(50)
    .target_error_rate(0.01)         // Target 1% errors
    .window_size(1000)                // Large window for stability
    .adjustment_interval(Duration::from_secs(300))  // Adjust slowly
    .build()?;

// Microservice (fast adaptation)
let config = AdaptiveCircuitBreakerConfig::builder()
    .initial_failure_threshold(5)
    .min_failure_threshold(2)
    .max_failure_threshold(20)
    .target_error_rate(0.05)         // Target 5% errors
    .window_size(100)                 // Smaller window for quick response
    .adjustment_interval(Duration::from_secs(60))   // Adjust frequently
    .build()?;
```

### Retry Configuration Patterns

```rust
// Fast fail for user-facing APIs
let config = RetryConfig::builder()
    .max_attempts(3)
    .fixed_backoff(Duration::from_millis(50))
    .max_total_time(Duration::from_millis(500))
    .build()?;

// Resilient background job
let config = RetryConfig::builder()
    .max_attempts(10)
    .exponential_backoff(
        Duration::from_secs(1),
        2.0,
        Duration::from_secs(300)
    )
    .decorrelated_jitter(Duration::from_secs(1))
    .max_total_time(Duration::from_secs(3600))
    .build()?;
```

### Rate Limiter Selection

```rust
// Burst-tolerant (API with occasional traffic spikes)
let limiter = TokenBucket::new(
    1000,                          // Allow burst up to 1000 requests
    100,                           // Refill 100 requests
    Duration::from_secs(1)         // Every second
)?;

// Smooth rate (protect downstream service)
let limiter = LeakyBucket::new(
    100,                           // Queue up to 100 requests
    50.0                           // Process 50 requests/second
)?;
```

## Backoff Strategies Comparison

| Strategy | Formula | Use Case |
|----------|---------|----------|
| **Fixed** | `constant` | Simple rate limiting, predictable intervals |
| **Linear** | `initial + (attempt × increment)` | Gradual backoff, bounded growth |
| **Exponential** | `initial × base^attempt` | Default choice, aggressive backoff |
| **Custom** | `fn(attempt) -> Duration` | Complex logic, domain-specific patterns |

## Jitter Types Comparison

| Jitter Type | Range | Best For |
|-------------|-------|----------|
| **None** | `delay` | Testing, deterministic behavior |
| **Full** | `[0, delay]` | Maximum randomization, avoid thundering herd |
| **Equal** | `[delay/2, delay]` | Balanced randomization (recommended default) |
| **Decorrelated** | `[base, prev_delay × 3]` | AWS-style jitter, sophisticated randomization |

## Performance Characteristics

| Operation | Time Complexity | Allocations | Thread-Safe | Lock-Free |
|-----------|----------------|-------------|-------------|-----------|
| Circuit breaker state check | O(1) | Zero | Yes | Yes (atomic) |
| Circuit breaker call | O(1) + operation | Minimal | Yes | Mostly |
| Adaptive threshold check | O(1) | Zero | Yes | Yes (atomic) |
| Retry delay calculation | O(1) | Zero | N/A | N/A |
| Retry with backoff | O(attempts) | Per-attempt | Yes | N/A |
| Token bucket acquire | O(1) | Zero | Yes | Yes (atomic) |
| Leaky bucket acquire | O(1) | Zero | Yes | Yes (atomic) |
| Bulkhead acquire | O(1) amortized | Minimal | Yes | No (semaphore) |
| Histogram record | O(1) | Zero | Yes | Yes (atomic) |
| Histogram percentile | O(buckets) | Snapshot | Yes | N/A |

## Error Handling

The module uses strongly-typed errors via `thiserror`:

```rust
pub enum ResilienceError<E: std::error::Error> {
    CircuitOpen,                      // Circuit breaker is open
    Timeout { timeout: Duration },    // Operation timed out
    RateLimitExceeded { requests, window }, // Rate limit exceeded
    BulkheadFull { capacity },        // Bulkhead at capacity
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

# Run specific module tests
cargo test -p pulsearc-common test_histogram
cargo test -p pulsearc-common test_adaptive_circuit_breaker
cargo test -p pulsearc-common test_bulkhead

# Run benchmarks (if available)
cargo bench -p pulsearc-common --features runtime --bench resilience_bench
```

## Metrics Reference

### Circuit Breaker Metrics

```rust
let metrics = breaker.metrics();

// Core metrics
metrics.state                    // Current state: Closed, Open, HalfOpen
metrics.total_calls              // Total operations attempted
metrics.success_count            // Successful operations
metrics.failure_count            // Failed operations
metrics.poisoned_lock_count      // Lock poisoning events (for monitoring)

// Derived metrics (methods)
metrics.success_rate()           // 0.0 to 1.0
metrics.failure_rate()           // 0.0 to 1.0
metrics.time_in_current_state()  // Duration
metrics.time_since_last_failure() // Option<Duration>
metrics.is_healthy(0.1)          // true if closed and failure_rate < 10%
metrics.status_message()         // Human-readable summary
```

### Adaptive Circuit Breaker Metrics

```rust
let metrics = breaker.metrics();

metrics.current_failure_threshold  // Dynamically adjusted threshold
metrics.recent_error_rate          // 0.0 to 1.0 from sliding window
metrics.threshold_adjustments      // Number of adjustments made
metrics.latency_p50                // Median latency
metrics.latency_p99                // 99th percentile latency
metrics.status_message()           // Human-readable summary
```

### Bulkhead Metrics

```rust
let metrics = bulkhead.metrics();

metrics.current_concurrent         // Current operations running
metrics.current_queued             // Current operations queued
metrics.total_operations           // Total executed
metrics.rejected_operations        // Total rejected
metrics.timeout_count              // Timeouts acquiring permit

// Derived metrics (methods)
metrics.utilization()              // 0.0 to 1.0
metrics.rejection_rate()           // 0.0 to 1.0
metrics.is_at_capacity()           // Boolean
metrics.status_message()           // Human-readable summary
```

### Histogram Statistics

```rust
let snapshot = histogram.snapshot();

snapshot.count()                   // Total measurements
snapshot.mean()                    // Average latency
snapshot.min()                     // Minimum latency
snapshot.max()                     // Maximum latency
snapshot.percentile(0.5)           // Median (p50)
snapshot.percentile(0.95)          // p95
snapshot.percentile(0.99)          // p99
snapshot.percentile(0.999)         // p999
snapshot.stddev()                  // Standard deviation
snapshot.percentiles()             // Struct with p50, p95, p99, p999
snapshot.summary()                 // Human-readable string
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
use pulsearc_common::resilience::{retry_with_policy, RetryConfig, policies::AlwaysRetry};
```

**When to migrate:** Only if you're building a new system and want generic, reusable abstractions. Existing `sync::retry` usage is fine and should not be changed without good reason.

## Examples

See the `tests/` directory for comprehensive integration tests demonstrating:
- Circuit breaker state transitions with real and mock clocks
- Retry with all backoff and jitter combinations
- Adaptive threshold adjustments under varying error rates
- Rate limiting with burst and smooth enforcement
- Bulkhead concurrency limiting and queueing
- Latency histogram percentile calculations
- Combining multiple patterns for layered resilience

## License

PulseArc is dual-licensed under MIT and Apache 2.0. See the repository root for the full text.
