# Collections

Specialized data structures that back PulseArc's runtime services. Everything in this folder is re-exported from `pulsearc_common::collections`.

## Module Map

| Data structure | Public API | Highlights | Typical use cases |
| -------------- | ---------- | ---------- | ----------------- |
| Bounded queue | `BoundedQueue<T>` | Thread-safe FIFO, blocking + timeout semantics, graceful close | Coordinating producer/consumer pipelines without Tokio |
| Ring buffer | `RingBuffer<T>` | Fixed-capacity circular buffer, overwrite-on-full | Sliding windows, metric sampling, fixed history |
| In-house LRU | `LruCache<K, V>` | Deterministic O(1) operations, MRU iteration | Hot-path caches with predictable memory footprint |
| External LRU | `ExternalLruCache<K, V>` | Thin wrapper over the `lru` crate | When you need `lru`'s API but want a common import path |
| Priority queues | `MinHeap<T>`, `MaxHeap<T>`, `PriorityQueue<T>` trait | Binary-heap backed, ergonomic constructors | Scheduling, top-k extraction, throttling |
| Trie | `Trie` + `IterPrefix` | Unicode-aware prefix tree with lazy iteration | Autocomplete, routing tables, prefix analytics |
| Bloom filter | `BloomFilter` | Randomized hashing, deterministic seeding option, FPR estimation | Duplicate suppression, membership pre-checks |

## Getting Started

Enable the `foundation` feature if your crate does not already depend on it -- `BloomFilter` and a few helpers require the optional `rand` crate.

```toml
[dependencies]
pulsearc-common = { path = "crates/common", features = ["foundation"] }
```

Import the re-exports from the module root:

```rust
use pulsearc_common::collections::{
    BloomFilter, BoundedQueue, LruCache, MaxHeap, MinHeap, PriorityQueue, RingBuffer, Trie,
};
```

## Usage Examples

### Bounded Queue (`bounded_queue.rs`)

```rust
use pulsearc_common::collections::{BoundedQueue, TryPushTimeout};
use std::time::Duration;

let queue = BoundedQueue::new(128);

// Blocking push/pop stay on std sync primitives.
queue.push("job-1")?;
assert_eq!(queue.pop()?, Some("job-1"));

// Try variants avoid blocking the caller.
if let Err(err) = queue.try_push("job-2") {
    eprintln!("queue full: {}", err);
}

// Timeouts convert to an error you can inspect.
match queue.push_timeout("job-3", Duration::from_millis(10)) {
    Err(TryPushTimeout::Timeout(_)) => { /* retry later */ }
    other => { other?; }
}

queue.close(); // wake waiters and prevent new pushes
```

Key points:
- All methods take `&self`; clone the queue to share it (`Arc` under the hood).
- `close()` is idempotent and drains remaining items before yielding `Ok(None)` from `pop()`.
- Mutex poisoning is recovered transparently to keep the queue usable after a panic.

### Ring Buffer (`ring_buffer.rs`)

```rust
use pulsearc_common::collections::RingBuffer;

let mut samples = RingBuffer::new(5);
for value in 1..=7 {
    samples.push(value);
}

assert_eq!(samples.iter().copied().collect::<Vec<_>>(), vec![3, 4, 5, 6, 7]);
assert_eq!(samples.pop(), Some(3));
assert!(samples.is_full());
```

Highlights:
- Capacity zero is clamped to one slot to avoid panics.
- Iterators visit items oldest -> newest; `as_slices` gives contiguous views for zero-copy integrations.
- `RingBuffer<T>` is `Send`/`Sync` when `T` is, making it trivial to embed in concurrent metrics collectors.

### LRU Caches (`lru.rs`, `lru_cache.rs`)

```rust
use pulsearc_common::collections::LruCache;
use std::num::NonZeroUsize;

let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());
cache.put("alpha", 1);
cache.put("beta", 2);

assert_eq!(cache.get(&"alpha"), Some(&1)); // promotes to MRU
cache.put("gamma", 3); // evicts "beta"
assert!(cache.peek(&"beta").is_none());
```

Notes:
- `LruCache` keeps entries in a Vec-backed doubly linked list for predictable O(1) operations.
- `iter()` yields MRU -> LRU; `try_new`/`try_resize` protect against zero capacities.
- Prefer `ExternalLruCache` when you specifically need the `lru` crate API surface.

### Priority Queues (`priority_queue.rs`)

```rust
use pulsearc_common::collections::{MinHeap, PriorityQueue};

let mut heap = MinHeap::with_capacity(32);
heap.extend([9, 1, 7]);
assert_eq!(heap.peek(), Some(&1));
assert_eq!(heap.pop(), Some(1));
```

Details:
- `MinHeap<T>` and `MaxHeap<T>` wrap `BinaryHeap` with intuitive ordering and constructors.
- The shared `PriorityQueue<T>` trait lets you write generic code across heap types.
- `into_sorted_vec()` produces an ordered `Vec<T>` without extra allocations.

### Trie (`trie.rs`)

```rust
use pulsearc_common::collections::Trie;

let mut trie = Trie::new();
trie.insert("pulse");
trie.insert("pulsearc");

assert!(trie.contains("pulse"));
assert_eq!(trie.find_prefix("pul"), vec!["pulse", "pulsearc"]);

let suggestions: Vec<_> = trie.iter_prefix("pulse").take(1).collect();
assert_eq!(suggestions, vec!["pulse"]);
```

Highlights:
- Unicode-aware: each scalar value becomes a distinct edge. Normalise input upstream when needed.
- `iter_prefix` reuses buffers internally for low-allocation traversals.
- `remove` recycles nodes into a free list, keeping future inserts cheap.

### Bloom Filter (`bloom_filter.rs`)

```rust
use pulsearc_common::collections::BloomFilter;

let mut filter = BloomFilter::new(10_000, 0.01)?;
filter.insert(&"user-123");

if filter.contains(&"user-123") {
    // Probably seen before.
}
```

Capabilities:
- Validates parameters and caps the bitset at ~128 MiB (`MAX_BITS`).
- `with_seed` makes deployments deterministic; default constructor seeds from `OsRng`.
- `estimated_false_positive_rate()` lets you monitor drift as the set fills.

## Choosing the Right Structure

- Use `BoundedQueue` for std-threaded producer/consumer flows that need backpressure without Tokio.
- Reach for `RingBuffer` when you need constant memory and order-preserving iteration.
- Pick `LruCache` when you need hot-path caching with predictable eviction; `ExternalLruCache` when third-party helpers expect that type.
- Choose `MinHeap`/`MaxHeap` when priority-based ordering matters more than FIFO semantics.
- Prefer `Trie` for prefix-heavy lookups, especially when you can normalise strings up front.
- Deploy `BloomFilter` as a fast guard before hitting exact-but-expensive stores; always verify positives with a ground-truth set.

## Testing and Benchmarks

```bash
# Target the unit tests in this module (fastest feedback loop)
cargo test -p pulsearc-common collections::bounded_queue
cargo test -p pulsearc-common collections::ring_buffer

# Run every collection-related test with the feature stack used in CI
cargo test -p pulsearc-common --features foundation collections

# Criterion benchmarks (enable `foundation` to pull in rand/criterion deps)
cargo bench -p pulsearc-common --features foundation --bench collections_bench
```

`make test` (Rust) and `make ci` (full pipeline) already cover these modules; the commands above are useful when iterating locally.

## Related Modules

- `crates/common/src/cache` - higher-level caching utilities built on these primitives.
- `crates/common/src/sync` - synchronization helpers that compose with `BoundedQueue`.
- `crates/common/benches/collections_bench.rs` - performance baselines referenced by CI.

## License

PulseArc is dual-licensed under MIT and Apache 2.0. See the repository root for the full text.
