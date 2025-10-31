# Sync Module

A production-ready synchronization infrastructure for distributed systems with retry logic, circuit breakers, message queues, and comprehensive resilience patterns.

## Overview

The sync module provides **domain-specific synchronization primitives** optimized for queue and sync operations:

- **Retry Strategy**: Exponential backoff with jitter, timeout support, and custom retry conditions
- **Circuit Breaker**: Automatic failure detection and recovery with half-open testing
- **Retry Budget**: Token-based rate limiting to prevent retry storms across operations
- **Message Queue**: Enterprise-grade persistent queue with priority ordering and retry management
- **Resilience Patterns**: Thread-safe components for building fault-tolerant systems

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      Sync Module                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │              Retry Infrastructure               │  │
│  │                                                 │  │
│  │  ┌─────────────┐  ┌──────────────────┐        │  │
│  │  │  Strategy   │  │ Circuit Breaker  │        │  │
│  │  │             │  │                  │        │  │
│  │  │ • Backoff   │  │ • State Machine  │        │  │
│  │  │ • Jitter    │  │ • Auto Recovery  │        │  │
│  │  │ • Timeout   │  │ • Half-Open Test │        │  │
│  │  │ • Metrics   │  │ • Failure Track  │        │  │
│  │  └─────────────┘  └──────────────────┘        │  │
│  │                                                 │  │
│  │  ┌─────────────┐  ┌──────────────────┐        │  │
│  │  │   Budget    │  │    Policies      │        │  │
│  │  │             │  │                  │        │  │
│  │  │ • Token Pool│  │ • Network        │        │  │
│  │  │ • Refill    │  │ • Database       │        │  │
│  │  │ • Rate Limit│  │ • API/FS/Rate    │        │  │
│  │  └─────────────┘  └──────────────────┘        │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │              Queue Infrastructure               │  │
│  │                                                 │  │
│  │  ┌───────────────┐  ┌─────────────────┐       │  │
│  │  │  Priority Q   │  │  Persistence    │       │  │
│  │  │               │  │                 │       │  │
│  │  │ • 5 Levels    │  │ • Compression   │       │  │
│  │  │ • FIFO Order  │  │ • Encryption    │       │  │
│  │  │ • Dedup       │  │ • Atomic Ops    │       │  │
│  │  └───────────────┘  └─────────────────┘       │  │
│  │                                                 │  │
│  │  ┌───────────────┐  ┌─────────────────┐       │  │
│  │  │   Retry Mgmt  │  │    Metrics      │       │  │
│  │  │               │  │                 │       │  │
│  │  │ • Exp Backoff │  │ • Throughput    │       │  │
│  │  │ • Max Retries │  │ • Success Rate  │       │  │
│  │  │ • Status Track│  │ • Queue Depth   │       │  │
│  │  └───────────────┘  └─────────────────┘       │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Features

### Retry Module
- Exponential backoff with configurable jitter (0-100%)
- Timeout support with early termination
- Custom retry conditions based on error types
- Async and sync execution support
- Comprehensive metrics collection
- Thread-safe with atomic operations

### Circuit Breaker
- Three-state machine (Closed, Open, HalfOpen)
- Automatic failure threshold detection
- Configurable recovery timeout
- Half-open testing with limited requests
- Success threshold for reclosing
- Thread-safe with mock clock support for testing

### Retry Budget
- Token bucket algorithm for rate limiting
- Configurable refill rate (tokens per second)
- Multi-token acquisition support
- Thread-safe lock-free operations
- Prevents retry storms during outages
- Automatic token refill based on time

### Message Queue
- 5-level priority system (Critical → Background)
- Atomic file persistence with optional encryption/compression
- Automatic retry with exponential backoff
- Deduplication by item ID
- Batch push/pop operations
- Graceful shutdown with state persistence
- Comprehensive metrics and monitoring

## Quick Start

### Basic Retry

```rust
use agent::common::sync::RetryStrategy;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Simple retry with defaults (5 attempts, 1s-60s backoff)
    let strategy = RetryStrategy::new();

    let result = strategy.execute(|| async {
        // Your operation here
        perform_network_request().await
    }).await;
}
```

### Retry with Metrics

```rust
let strategy = RetryStrategy::new()
    .with_max_attempts(5)?
    .with_base_delay(Duration::from_millis(100))?
    .with_max_delay(Duration::from_secs(10))?
    .with_jitter_factor(0.3)
    .with_timeout(Duration::from_secs(60));

let (result, metrics) = strategy
    .execute_with_metrics("database_query", || async {
        query_database().await
    })
    .await;

println!("Attempts: {}", metrics.attempts);
println!("Total delay: {:?}", metrics.total_delay);
println!("Succeeded: {}", metrics.succeeded);
```

### Using Predefined Policies

```rust
use agent::common::sync::RetryPolicies;

// Network operations (8 attempts, 500ms-30s backoff)
let network = RetryPolicies::network_policy();

// Database operations (5 attempts, 100ms-10s backoff)
let database = RetryPolicies::database_policy();

// Rate-limited APIs (10 attempts, fixed 60s delay)
let rate_limit = RetryPolicies::rate_limit_policy();

// API calls (3 attempts, 250ms-5s backoff)
let api = RetryPolicies::api_policy();

// File system (3 attempts, 50ms-1s backoff)
let fs = RetryPolicies::filesystem_policy();

// Idempotent operations (10 attempts, safe to retry)
let idempotent = RetryPolicies::idempotent_policy();

// Non-idempotent (2 attempts, limited retries)
let non_idempotent = RetryPolicies::non_idempotent_policy();
```

### Custom Retry Conditions

```rust
use agent::common::sync::{RetryStrategy, RetryCondition};
use std::sync::Arc;

let strategy = RetryStrategy::new()
    .with_retry_condition(RetryCondition::Custom(Arc::new(|err| {
        // Only retry specific error types
        let err_str = err.to_string().to_lowercase();
        err_str.contains("timeout")
            || err_str.contains("temporary")
            || err_str.contains("503")
    })));

let result = strategy.execute(|| async {
    make_api_call().await
}).await;
```

### Circuit Breaker Protection

```rust
use agent::common::sync::CircuitBreaker;
use std::sync::Arc;
use std::time::Duration;

let circuit_breaker = Arc::new(
    CircuitBreaker::new()
        .with_failure_threshold(5)      // Open after 5 failures
        .with_success_threshold(2)       // Close after 2 successes
        .with_timeout(Duration::from_secs(60))  // Recovery timeout
);

// Check circuit state before attempting operation
if circuit_breaker.should_allow_request()? {
    match perform_operation().await {
        Ok(result) => {
            circuit_breaker.record_success()?;
            Ok(result)
        }
        Err(err) => {
            circuit_breaker.record_failure()?;
            Err(err)
        }
    }
} else {
    // Circuit is open - fail fast
    Err(Error::ServiceUnavailable)
}

// Check circuit breaker stats
let stats = circuit_breaker.stats()?;
println!("State: {}", stats.state);
println!("Failures: {}", stats.failure_count);
```

### Retry Budget for Rate Limiting

```rust
use agent::common::sync::RetryBudget;
use std::sync::Arc;

// 100 tokens max, refill 10 tokens per second
let budget = Arc::new(RetryBudget::new(100, 10.0));

// Try to acquire tokens before retrying
if budget.try_acquire_multiple(3) {  // Request 3 tokens for up to 3 retries
    let result = strategy.execute(|| async {
        perform_operation().await
    }).await;

    // Return unused tokens if succeeded early
    if result.is_ok() {
        budget.return_tokens(2);  // Used only 1 retry
    }

    result
} else {
    // No budget available - fail fast to prevent retry storm
    Err(Error::BudgetExhausted)
}

// Check available tokens
println!("Available: {}/{}", budget.available(), budget.capacity());
```

### Message Queue Operations

```rust
use agent::common::sync::queue::{SyncQueue, SyncItem, Priority, QueueConfig};
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Create queue with custom configuration
    let config = QueueConfig {
        max_capacity: 50_000,
        enable_deduplication: true,
        enable_compression: true,
        base_retry_delay: Duration::from_secs(1),
        max_retry_delay: Duration::from_secs(3600),
        ..Default::default()
    };

    let queue = SyncQueue::with_config(config)?;

    // Push high-priority item
    let item = SyncItem::new(
        serde_json::json!({"task": "process_payment", "amount": 100.00}),
        Priority::High
    )
    .with_max_retries(5)
    .with_correlation_id("req-123".to_string())
    .with_metadata("user_id".to_string(), "user-456".to_string());

    queue.push(item).await?;

    // Process items
    while let Some(item) = queue.pop().await? {
        match process_item(&item).await {
            Ok(_) => {
                // Mark as completed
                queue.mark_completed(&item.id).await?;
            }
            Err(e) => {
                // Mark as failed - automatically retries if under max_retries
                let can_retry = queue
                    .mark_failed(&item.id, Some(e.to_string()))
                    .await?;

                if !can_retry {
                    eprintln!("Item {} failed permanently", item.id);
                }
            }
        }
    }

    // Batch operations
    let items = vec![
        SyncItem::new(json!({"batch": 1}), Priority::Normal),
        SyncItem::new(json!({"batch": 2}), Priority::Normal),
        SyncItem::new(json!({"batch": 3}), Priority::Normal),
    ];

    let added_ids = queue.push_batch(items).await?;
    let batch = queue.pop_batch(10).await?;

    // Get metrics
    let metrics = queue.metrics();
    println!("Total enqueued: {}", metrics.total_enqueued);
    println!("Success rate: {:.1}%", metrics.success_rate);
    println!("Throughput: {:.2} items/sec", metrics.throughput);

    // Graceful shutdown
    queue.shutdown().await?;
}
```

### Queue with Persistence and Encryption

```rust
use agent::common::sync::queue::{SyncQueue, QueueConfig};
use std::path::PathBuf;

let encryption_key = vec![0u8; 32]; // Generate a proper key in production

let config = QueueConfig {
    persistence_path: Some(PathBuf::from("/var/lib/app/queue.dat")),
    persistence_interval: Duration::from_secs(30),
    enable_compression: true,
    compression_level: 6,
    enable_encryption: true,
    encryption_key: Some(encryption_key),
    ..Default::default()
};

let queue = SyncQueue::with_config(config)?;
```

## Retry Policies

The module includes predefined policies for common scenarios:

| Policy | Max Attempts | Base Delay | Max Delay | Use Case |
|--------|--------------|------------|-----------|----------|
| `network_policy()` | 8 | 500ms | 30s | Network requests, HTTP calls |
| `database_policy()` | 5 | 100ms | 10s | Database queries, transactions |
| `rate_limit_policy()` | 10 | 60s | 60s | Rate-limited APIs (429 errors) |
| `api_policy()` | 3 | 250ms | 5s | External API calls |
| `filesystem_policy()` | 3 | 50ms | 1s | File I/O operations |
| `idempotent_policy()` | 10 | 100ms | 20s | Safe-to-retry operations |
| `non_idempotent_policy()` | 2 | 1s | 5s | Unsafe operations (POST, etc.) |

## Circuit Breaker States

The circuit breaker operates as a state machine:

```
      ┌─────────┐
      │ Closed  │ ←───────────────┐
      └────┬────┘                 │
           │ failures ≥ threshold │ successes ≥ threshold
           v                      │
      ┌─────────┐                 │
      │  Open   │                 │
      └────┬────┘                 │
           │ timeout elapsed      │
           v                      │
      ┌─────────┐                 │
      │HalfOpen │ ────────────────┘
      └─────────┘
           │ failure
           └───────> Open
```

- **Closed**: Normal operation, all requests pass through
- **Open**: After threshold failures, all requests rejected for timeout period
- **HalfOpen**: After timeout, limited requests allowed to test recovery

## Retry Budget Token Bucket

The retry budget uses a token bucket algorithm:

```
Capacity: 100 tokens
Refill Rate: 10 tokens/second

Time     Action              Available
─────────────────────────────────────
0s       Start               100
1s       Acquire 5           95
2s       Refill +10          100 (capped)
3s       Acquire 20          80
4s       Refill +10          90
5s       Acquire 100         0 (exhausted)
6s       Acquire 1           DENIED
7s       Refill +10          10
8s       Acquire 5           5
```

## Queue Priority Levels

Items are processed in priority order:

1. **Critical** (0) - System-critical tasks, immediate processing
2. **High** (1) - Important tasks with tight deadlines
3. **Normal** (2) - Standard priority for regular operations
4. **Low** (3) - Tasks that can be deferred
5. **Background** (4) - Maintenance and cleanup tasks

Within the same priority level, items are processed in FIFO order.

## Metrics and Monitoring

### Retry Metrics

```rust
let (result, metrics) = strategy
    .execute_with_metrics("operation", operation)
    .await;

println!("Attempts: {}", metrics.attempts);
println!("Total delay: {:?}", metrics.total_delay);
println!("Succeeded: {}", metrics.succeeded);
println!("Timed out: {}", metrics.timed_out);
println!("Success rate: {:.1}%", metrics.success_rate() * 100.0);
println!("Avg delay: {:?}", metrics.average_delay());
```

### Circuit Breaker Stats

```rust
let stats = circuit_breaker.stats()?;
println!("State: {}", stats.state);              // Closed/Open/HalfOpen
println!("Failures: {}", stats.failure_count);
println!("Successes: {}", stats.success_count);
println!("Half-open requests: {}", stats.half_open_requests);

if let Some(last_failure) = stats.last_failure_time {
    println!("Last failure: {:?} ago", last_failure.elapsed());
}
```

### Queue Metrics

```rust
let metrics = queue.metrics();
println!("Queue Statistics:");
println!("  Total Enqueued: {}", metrics.total_enqueued);
println!("  Total Dequeued: {}", metrics.total_dequeued);
println!("  Total Completed: {}", metrics.total_completed);
println!("  Total Failed: {}", metrics.total_failed);
println!("  Total Retried: {}", metrics.total_retried);
println!("  Current Size: {}", metrics.current_size);
println!("  Max Depth: {}", metrics.queue_depth_max);
println!("  Success Rate: {:.1}%", metrics.success_rate);
println!("  Avg Processing Time: {:.2}ms", metrics.average_processing_time_ms);
println!("  Throughput: {:.2} items/sec", metrics.throughput);
println!("  Deduplication Hits: {}", metrics.deduplication_hits);
println!("  Compression Saved: {} bytes", metrics.compression_bytes_saved);
```

## Error Handling

### Retry Errors

```rust
use agent::common::sync::RetryError;

match strategy.execute(operation).await {
    Ok(result) => // Success
    Err(RetryError::AttemptsExhausted { attempts }) =>
        // All retry attempts failed
    Err(RetryError::Timeout { elapsed }) =>
        // Operation timed out
    Err(RetryError::OperationFailed { source }) =>
        // The underlying operation failed
    Err(RetryError::InvalidConfiguration { message }) =>
        // Configuration error
}
```

### Queue Errors

```rust
use agent::common::sync::queue::QueueError;

match queue.push(item).await {
    Ok(_) => // Success
    Err(QueueError::CapacityExceeded(max)) =>
        // Queue is full
    Err(QueueError::DuplicateItem(id)) =>
        // Item already exists (deduplication)
    Err(QueueError::ShuttingDown) =>
        // Queue is shutting down
    Err(QueueError::Locked) =>
        // Queue is locked for maintenance
    Err(QueueError::ItemNotFound(id)) =>
        // Item ID not found
    Err(QueueError::InvalidState(msg)) =>
        // Invalid queue state
}
```

## Advanced Patterns

### Complete Resilience Stack

```rust
use agent::common::sync::{
    RetryStrategy, CircuitBreaker, RetryBudget, RetryPolicies
};
use std::sync::Arc;

pub struct ResilientClient {
    retry_strategy: RetryStrategy,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_budget: Arc<RetryBudget>,
}

impl ResilientClient {
    pub fn new() -> Self {
        Self {
            retry_strategy: RetryPolicies::api_policy(),
            circuit_breaker: Arc::new(
                CircuitBreaker::new()
                    .with_failure_threshold(10)
                    .with_timeout(Duration::from_secs(60))
            ),
            retry_budget: Arc::new(RetryBudget::new(100, 10.0)),
        }
    }

    pub async fn call_api(&self, endpoint: &str) -> Result<Response, Error> {
        // Check circuit breaker
        if !self.circuit_breaker.should_allow_request()? {
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
            Ok(_) => self.circuit_breaker.record_success()?,
            Err(_) => self.circuit_breaker.record_failure()?,
        }

        // Return unused budget if succeeded quickly
        if metrics.attempts == 1 {
            self.retry_budget.return_tokens(1);
        }

        result.map_err(Error::from)
    }
}
```

### Queue-Based Processing Pipeline

```rust
use agent::common::sync::queue::{SyncQueue, SyncItem, Priority};
use std::sync::Arc;

pub struct ProcessingPipeline {
    input_queue: Arc<SyncQueue>,
    output_queue: Arc<SyncQueue>,
}

impl ProcessingPipeline {
    pub async fn process_batch(&self, batch_size: usize) -> Result<(), Error> {
        let items = self.input_queue.pop_batch(batch_size).await?;

        for item in items {
            // Process with circuit breaker protection
            match self.process_item(&item).await {
                Ok(result) => {
                    // Mark input item as completed
                    self.input_queue.mark_completed(&item.id).await?;

                    // Push result to output queue
                    let output = SyncItem::new(
                        serde_json::to_value(result)?,
                        Priority::Normal
                    );
                    self.output_queue.push(output).await?;
                }
                Err(e) => {
                    // Mark as failed - automatic retry
                    self.input_queue
                        .mark_failed(&item.id, Some(e.to_string()))
                        .await?;
                }
            }
        }

        Ok(())
    }
}
```

## Synchronous Operations

For blocking code (not in async context):

```rust
use agent::common::sync::RetryStrategy;

// WARNING: Do not use in async context - will block the runtime
let strategy = RetryStrategy::new();

let result = strategy.execute_sync(|| {
    // Blocking operation
    std::fs::read_to_string("config.toml")
})?;

// With metrics
let (result, metrics) = strategy
    .execute_sync_with_metrics("read_config", || {
        std::fs::read_to_string("config.toml")
    });
```

## Configuration

### Default Configuration

```rust
// Retry defaults
DEFAULT_MAX_ATTEMPTS: 5
DEFAULT_BASE_DELAY: 1 second
DEFAULT_MAX_DELAY: 60 seconds
DEFAULT_JITTER_FACTOR: 0.3 (30% jitter)

// Circuit breaker defaults
DEFAULT_FAILURE_THRESHOLD: 5
DEFAULT_SUCCESS_THRESHOLD: 2
DEFAULT_CIRCUIT_TIMEOUT: 60 seconds
DEFAULT_HALF_OPEN_REQUESTS: 1

// Budget defaults
BUDGET_REFILL_INTERVAL: 1 second

// Queue defaults
max_capacity: 10,000
batch_size: 100
persistence_interval: 30 seconds
compression_level: 6
retention_period: 7 days
base_retry_delay: 1 second
max_retry_delay: 3600 seconds (1 hour)
cleanup_interval: 300 seconds (5 minutes)
```

## Comparison with common/resilience

The `agent::common::sync` module is a **domain-specific implementation** optimized for sync/queue operations, while `agent::common::resilience` provides **generic library abstractions**.

| Feature | `common::sync` | `common::resilience` |
|---------|----------------|---------------------|
| **Purpose** | Sync-specific production code | Generic library for any domain |
| **Error Type** | Concrete `RetryError` | Generic `<E: std::error::Error>` |
| **Metrics** | Integrated `RetryMetrics` | None (generic) |
| **Tracing** | Feature-gated OpenTelemetry spans | Basic logging only |
| **Backoff** | Exponential with jitter | 4 strategies (Linear, Exponential, Fibonacci, Fixed) |
| **Jitter** | Fixed 30% default | 4 types (Full, Equal, Decorrelated, None) |
| **Circuit Breaker** | Adapter wrapping unified impl | Unified implementation |
| **Queue** | Enterprise-grade persistent queue | Not provided |
| **Retry Budget** | Token bucket rate limiting | Not provided |
| **Domain Coupling** | Coupled to sync operations | Framework-agnostic |
| **Current Usage** | Active production use | Reserved for future modules |
| **Testing** | MockClock support | Full clock abstraction |

### When to Use Each

- **Use `common::sync`**: When working **within the sync/queue domain** where you need integrated metrics, tracing, and domain-specific error handling. This is the primary choice for queue operations.

- **Use `common::resilience`**: When adding resilience to **new modules** or **different domains** that need circuit breakers or retry logic. Provides clean, generic foundation.

### Migration Path

Long-term, consider migrating `sync::retry` to use `resilience` as a backend, adding metrics and tracing as wrapper layers. This would eliminate duplication while maintaining specialized functionality. **Priority: LOW** (both implementations work well, no bugs reported).

## Thread Safety

All components are thread-safe and can be shared across tasks:

```rust
use std::sync::Arc;

let strategy = Arc::new(RetryStrategy::new());
let circuit_breaker = Arc::new(CircuitBreaker::new());
let budget = Arc::new(RetryBudget::new(100, 10.0));
let queue = Arc::new(SyncQueue::new());

// Spawn multiple tasks
for i in 0..10 {
    let s = strategy.clone();
    let cb = circuit_breaker.clone();
    let b = budget.clone();
    let q = queue.clone();

    tokio::spawn(async move {
        // All components are safe to use concurrently
    });
}
```

## Testing

### Unit Tests

```bash
# Run all sync module tests
cargo test --lib common::sync

# Run specific submodule tests
cargo test --lib common::sync::retry
cargo test --lib common::sync::queue

# Run with all features
cargo test --all-features --lib common::sync
```

### Integration Tests

```bash
# Integration tests with real timing
cargo test --test '*' -- --include-ignored

# Specific integration test
cargo test --test integration_sync
```

### Mock Clock for Deterministic Tests

```rust
use agent::common::sync::{CircuitBreaker, RetryBudget};
use agent::common::sync::retry::time::MockClock;
use std::time::Duration;

let clock = MockClock::new();

// Create components with mock clock
let circuit_breaker = CircuitBreaker::with_clock(clock.clone())
    .with_timeout(Duration::from_secs(60));

let budget = RetryBudget::with_clock(100, 10.0, clock.clone());

// Advance time deterministically
clock.advance(Duration::from_secs(30));

// Test time-based behavior
assert!(circuit_breaker.should_allow_request()?);
```

## Performance Considerations

1. **Retry Strategy**: Exponential backoff with jitter prevents thundering herd
2. **Circuit Breaker**: Lock-free state transitions using atomics
3. **Retry Budget**: Token bucket uses compare-and-swap for thread-safe operations
4. **Queue**: RwLock for shared state, atomic operations for metrics
5. **Compression**: Reduces I/O but increases CPU usage
6. **Encryption**: Adds security overhead but protects sensitive data
7. **Persistence**: Balance durability vs. performance with persistence interval

## Best Practices

1. **Choose Appropriate Policies**: Use predefined policies when possible
2. **Set Timeouts**: Prevent operations from running indefinitely
3. **Use Circuit Breakers**: Protect against cascading failures
4. **Budget Retries**: Prevent retry storms during outages
5. **Add Jitter**: Avoid synchronized retries (thundering herd)
6. **Monitor Metrics**: Track behavior in production
7. **Consider Idempotency**: Ensure operations are safe to retry
8. **Batch Operations**: Use batch push/pop for better queue throughput
9. **Graceful Shutdown**: Always call shutdown() on queues
10. **Test with MockClock**: Write deterministic time-based tests

## License

This module is part of the PulsearC Agent project.
