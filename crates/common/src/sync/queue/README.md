# Sync Queue Module

A high-performance, enterprise-grade persistent queue implementation for Rust with support for priority ordering, encryption, compression, and automatic retry mechanisms.

## Features

- **Priority Queue**: 5-level priority system (Critical, High, Normal, Low, Background)
- **Persistence**: Atomic file-based persistence with optional compression and encryption
- **Retry Logic**: Exponential backoff with jitter and configurable retry limits
- **Circuit Breaker**: Automatic failure detection and recovery
- **Metrics**: Comprehensive performance and operational metrics
- **Thread-Safe**: Lock-free operations where possible, RwLock for shared state
- **Deduplication**: Optional duplicate detection by item ID
- **Partitioning**: Support for sharded queue operations
- **Maintenance**: Automatic cleanup of expired and orphaned items

## Quick Start

### Basic Usage

```rust
use sync::queue::{SyncQueue, SyncItem, Priority};

#[tokio::main]
async fn main() {
    // Create a new queue with default configuration
    let queue = SyncQueue::new();

    // Push an item
    let item = SyncItem::new(
        serde_json::json!({"task": "process_payment", "amount": 100.00}),
        Priority::High
    );
    queue.push(item).await.unwrap();

    // Pop and process items
    while let Some(item) = queue.pop().await.unwrap() {
        // Process the item
        process_item(&item).await;

        // Mark as completed
        queue.mark_completed(&item.id).await.unwrap();
    }
}
```

### With Persistence and Encryption

```rust
use sync::queue::{SyncQueue, QueueConfig, EncryptionService};
use std::path::PathBuf;

let config = QueueConfig {
    persistence_path: Some(PathBuf::from("/var/lib/app/queue.dat")),
    enable_encryption: true,
    encryption_key: Some(EncryptionService::generate_key()),
    enable_compression: true,
    compression_level: 6,
    ..Default::default()
};

let queue = SyncQueue::with_config(config).unwrap();
```

### Batch Operations

```rust
// Push multiple items
let items = vec![
    SyncItem::new(json!({"batch": 1}), Priority::Normal),
    SyncItem::new(json!({"batch": 2}), Priority::Normal),
    SyncItem::new(json!({"batch": 3}), Priority::Normal),
];

let added_ids = queue.push_batch(items).await.unwrap();

// Pop multiple items
let batch = queue.pop_batch(10).await.unwrap();
for item in batch {
    // Process items in parallel
    tokio::spawn(async move {
        process_item(&item).await;
    });
}
```

### Retry Handling

```rust
let item = SyncItem::new(json!({"retry": "test"}), Priority::Normal)
    .with_max_retries(5);

queue.push(item).await.unwrap();

// Process with retry
loop {
    let item = queue.pop().await.unwrap();

    match process_item(&item).await {
        Ok(_) => {
            queue.mark_completed(&item.id).await.unwrap();
            break;
        }
        Err(e) => {
            let can_retry = queue.mark_failed(&item.id, Some(e.to_string()))
                .await.unwrap();

            if !can_retry {
                log::error!("Item {} failed permanently", item.id);
                break;
            }
        }
    }
}
```

## Configuration

### Predefined Configurations

```rust
// Optimized for throughput
let config = QueueConfig::high_performance();

// Optimized for security
let config = QueueConfig::high_security();

// Custom configuration
let config = QueueConfig {
    max_capacity: 50_000,
    batch_size: 500,
    persistence_path: Some(path),
    persistence_interval: Duration::from_secs(30),
    enable_deduplication: true,
    enable_compression: true,
    compression_level: 6,
    enable_encryption: false,
    retention_period: Duration::from_days(7),
    base_retry_delay: Duration::from_secs(1),
    max_retry_delay: Duration::from_hours(1),
    cleanup_interval: Duration::from_minutes(5),
    heap_cleanup_threshold: 1000,
    enable_partitioning: false,
    partition_count: 4,
    ..Default::default()
};
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_capacity` | `usize` | 10,000 | Maximum number of items in queue |
| `batch_size` | `usize` | 100 | Default batch size for batch operations |
| `persistence_path` | `Option<PathBuf>` | None | Path to persistence file |
| `persistence_interval` | `Duration` | 30s | How often to persist queue state |
| `enable_deduplication` | `bool` | true | Prevent duplicate items by ID |
| `enable_compression` | `bool` | true | Compress persisted data |
| `compression_level` | `u32` | 6 | Compression level (0-9) |
| `enable_encryption` | `bool` | false | Encrypt persisted data |
| `encryption_key` | `Option<Vec<u8>>` | None | 32-byte encryption key |
| `retention_period` | `Duration` | 7 days | How long to keep completed items |
| `base_retry_delay` | `Duration` | 1s | Initial retry delay |
| `max_retry_delay` | `Duration` | 1 hour | Maximum retry delay |
| `cleanup_interval` | `Duration` | 5 min | How often to run maintenance |
| `heap_cleanup_threshold` | `usize` | 1000 | Items before heap cleanup |
| `enable_partitioning` | `bool` | false | Enable queue partitioning |
| `partition_count` | `usize` | 4 | Number of partitions |

## Priority Levels

Items are processed in priority order:

1. **Critical** - System-critical tasks that must be processed immediately
2. **High** - Important tasks with tight deadlines
3. **Normal** - Standard priority for regular operations
4. **Low** - Tasks that can be deferred
5. **Background** - Maintenance and cleanup tasks

Within the same priority level, items are processed in FIFO order.

## Item Lifecycle

```
┌─────────┐      ┌────────────┐      ┌────────────┐
│ Pending │ ───> │ Processing │ ───> │ Completed  │
└─────────┘      └────────────┘      └────────────┘
                        │
                        v
                  ┌──────────┐
                  │  Failed  │ ───> (Retry or Cancel)
                  └──────────┘
```

## Metrics

The queue provides comprehensive metrics:

```rust
let metrics = queue.metrics();

println!("Queue Statistics:");
println!("  Total Enqueued: {}", metrics.total_enqueued);
println!("  Total Completed: {}", metrics.total_completed);
println!("  Total Failed: {}", metrics.total_failed);
println!("  Current Size: {}", metrics.current_size);
println!("  Success Rate: {:.2}%", metrics.success_rate * 100.0);
println!("  Avg Wait Time: {:?}", metrics.avg_wait_time);
println!("  Avg Processing Time: {:?}", metrics.avg_processing_time);

let throughput = queue.throughput();
println!("Throughput:");
println!("  Enqueue: {:.2} items/sec", throughput.enqueue_per_sec);
println!("  Dequeue: {:.2} items/sec", throughput.dequeue_per_sec);
```

## Advanced Features

### Correlation and Tracing

```rust
let item = SyncItem::new(json!({"data": "value"}), Priority::Normal)
    .with_correlation_id("request-123")
    .with_metadata("user_id", "user-456")
    .with_metadata("session", "session-789");
```

### Partitioning for Scalability

```rust
let config = QueueConfig {
    enable_partitioning: true,
    partition_count: 8,
    ..Default::default()
};

let item = SyncItem::new(json!({"data": "value"}), Priority::Normal)
    .with_partition_key("user-123"); // Items with same key go to same partition
```

### Circuit Breaker

The queue includes an automatic circuit breaker that trips after repeated failures:

```rust
if let Ok(healthy) = queue.health_check() {
    if !healthy {
        log::warn!("Queue circuit breaker is open - too many failures");
        // Wait for recovery or take alternative action
    }
}
```

### Graceful Shutdown

```rust
// Shutdown gracefully, persisting all items
queue.shutdown().await.unwrap();
```

## Performance Considerations

1. **Batch Operations**: Use batch push/pop for better throughput
2. **Compression**: Reduces I/O but increases CPU usage
3. **Encryption**: Adds security but impacts performance
4. **Deduplication**: Prevents duplicates but requires memory for tracking
5. **Persistence Interval**: Balance between durability and performance

## Error Handling

All queue operations return `QueueResult<T>` which can contain:

- `QueueError::CapacityExceeded` - Queue is full
- `QueueError::ItemNotFound` - Item ID not found
- `QueueError::ShuttingDown` - Queue is shutting down
- `QueueError::Locked` - Queue is locked for maintenance
- `QueueError::DuplicateItem` - Item with ID already exists
- `QueueError::PersistenceError` - Failed to save/load queue
- `QueueError::EncryptionError` - Encryption/decryption failed
- `QueueError::CompressionError` - Compression/decompression failed

## Thread Safety

The queue is thread-safe and can be shared across tasks:

```rust
let queue = Arc::new(SyncQueue::new());

// Spawn multiple producers
for i in 0..10 {
    let q = queue.clone();
    tokio::spawn(async move {
        // Push items
    });
}

// Spawn multiple consumers
for _ in 0..5 {
    let q = queue.clone();
    tokio::spawn(async move {
        // Pop and process items
    });
}
```

## Testing

Run the comprehensive test suite:

```bash
# Unit tests
cargo test --lib sync::queue::test_unit

# Integration tests
cargo test --lib sync::queue::tests

# All tests with coverage
cargo tarpaulin --lib -p tauri-agent -- sync::queue
```

## License

This module is part of the Tauri Agent project and follows the project's licensing terms.