# Collections

Specialized data structures for high-performance applications.

## Overview

The collections module provides:

- **Bounded Queue**: Thread-safe queue with backpressure
- **Ring Buffer**: Fixed-size circular buffer
- **LRU Cache**: Least Recently Used cache
- **Priority Queue**: Min/max heap implementations
- **Trie**: Prefix tree for string matching
- **Bloom Filter**: Probabilistic membership testing

## Features

- ✅ Thread-safe concurrent data structures
- ✅ Zero-copy operations where possible
- ✅ Efficient memory usage
- ✅ Production-tested implementations
- ✅ Comprehensive test coverage

## Quick Start

### Bounded Queue

```rust
use agent::common::collections::BoundedQueue;

#[tokio::main]
async fn main() {
    let queue = BoundedQueue::new(100);
    
    // Push items (blocks when full)
    queue.push(42).await.unwrap();
    
    // Pop items
    let value = queue.pop().await.unwrap();
    assert_eq!(value, Some(42));
    
    // Try push without blocking
    match queue.try_push(43) {
        Ok(_) => println!("Pushed successfully"),
        Err(item) => println!("Queue full, item: {}", item),
    }
}
```

### Ring Buffer

```rust
use agent::common::collections::RingBuffer;

let mut buffer = RingBuffer::new(5);

// Push items
buffer.push(1);
buffer.push(2);
buffer.push(3);

// Get items by index
assert_eq!(buffer.get(0), Some(&1));
assert_eq!(buffer.get(1), Some(&2));

// Overwrites oldest when full
for i in 4..=7 {
    buffer.push(i);
}

assert_eq!(buffer.len(), 5);
assert_eq!(buffer.get(0), Some(&3)); // 1 and 2 were overwritten
```

### LRU Cache

```rust
use agent::common::collections::LruCache;
use std::num::NonZeroUsize;

let mut cache = LruCache::new(NonZeroUsize::new(100).unwrap());

// Insert items
cache.put("key1", "value1");
cache.put("key2", "value2");

// Get items (updates access time)
assert_eq!(cache.get(&"key1"), Some(&"value1"));

// Peek without updating access time
assert_eq!(cache.peek(&"key2"), Some(&"value2"));

// Remove items
cache.pop(&"key1");
assert_eq!(cache.get(&"key1"), None);
```

### Priority Queue

```rust
use agent::common::collections::{MinHeap, MaxHeap, PriorityQueue};

// Min heap (smallest first)
let mut min_heap = MinHeap::new();
min_heap.push(5);
min_heap.push(2);
min_heap.push(8);

assert_eq!(min_heap.pop(), Some(2));
assert_eq!(min_heap.pop(), Some(5));
assert_eq!(min_heap.pop(), Some(8));

// Max heap (largest first)
let mut max_heap = MaxHeap::new();
max_heap.push(5);
max_heap.push(2);
max_heap.push(8);

assert_eq!(max_heap.pop(), Some(8));
assert_eq!(max_heap.pop(), Some(5));
assert_eq!(max_heap.pop(), Some(2));
```

### Trie

```rust
use agent::common::collections::Trie;

let mut trie = Trie::new();

// Insert words
trie.insert("hello");
trie.insert("world");
trie.insert("help");

// Check membership
assert!(trie.contains("hello"));
assert!(!trie.contains("helloworld"));

// Check prefix
assert!(trie.starts_with("hel"));

// Find all words with prefix
let words = trie.find_prefix("hel");
assert_eq!(words, vec!["hello", "help"]);

// Remove words
trie.remove("hello");
assert!(!trie.contains("hello"));
```

### Bloom Filter

```rust
use agent::common::collections::BloomFilter;

// Create filter: 1000 expected items, 1% false positive rate
let mut filter = BloomFilter::new(1000, 0.01);

// Insert items
filter.insert("apple");
filter.insert("banana");
filter.insert("orange");

// Check membership
assert!(filter.contains("apple"));    // Definitely in set
assert!(filter.contains("banana"));   // Definitely in set
assert!(!filter.contains("grape"));   // Probably not in set (could false positive)

// Estimate false positive rate
let fpr = filter.false_positive_rate();
println!("Current FPR: {:.2}%", fpr * 100.0);
```

## API Reference

### BoundedQueue\<T\>

Thread-safe bounded queue with backpressure.

**Methods:**
- `new(capacity) -> Self` - Create new queue
- `push(item) -> Result<(), ()>` - Push item (blocks if full)
- `try_push(item) -> Result<(), T>` - Try push without blocking
- `pop() -> Result<Option<T>, ()>` - Pop item
- `try_pop() -> Option<T>` - Try pop without blocking
- `len() -> usize` - Get current length
- `is_empty() -> bool` - Check if empty
- `is_full() -> bool` - Check if full
- `capacity() -> usize` - Get capacity
- `clear()` - Clear all items

### RingBuffer\<T\>

Fixed-size circular buffer with constant-time operations.

**Methods:**
- `new(capacity) -> Self` - Create new buffer
- `push(item)` - Push item (overwrites oldest when full)
- `pop() -> Option<T>` - Pop oldest item
- `get(index) -> Option<&T>` - Get item by index
- `len() -> usize` - Get current length
- `is_empty() -> bool` - Check if empty
- `is_full() -> bool` - Check if full
- `capacity() -> usize` - Get capacity
- `clear()` - Clear all items
- `iter() -> RingBufferIter<T>` - Get iterator

### LruCache\<K, V\>

Least Recently Used cache.

**Methods:**
- `new(capacity) -> Self` - Create new cache
- `put(key, value) -> Option<V>` - Insert item
- `get(&key) -> Option<&V>` - Get item (updates access)
- `get_mut(&key) -> Option<&mut V>` - Get mutable item
- `peek(&key) -> Option<&V>` - Peek without updating access
- `contains(&key) -> bool` - Check if key exists
- `pop(&key) -> Option<V>` - Remove item
- `len() -> usize` - Get current length
- `is_empty() -> bool` - Check if empty
- `cap() -> usize` - Get capacity
- `clear()` - Clear all items
- `resize(capacity)` - Resize cache

### MinHeap\<T\> / MaxHeap\<T\>

Priority queues with min/max heap semantics.

**Methods:**
- `new() -> Self` - Create new heap
- `with_capacity(capacity) -> Self` - Create with capacity
- `push(item)` - Push item
- `pop() -> Option<T>` - Pop highest priority item
- `peek() -> Option<&T>` - Peek at highest priority
- `len() -> usize` - Get current length
- `is_empty() -> bool` - Check if empty
- `clear()` - Clear all items

### Trie

Prefix tree for efficient string storage and search.

**Methods:**
- `new() -> Self` - Create new trie
- `insert(word)` - Insert word
- `contains(word) -> bool` - Check if word exists
- `starts_with(prefix) -> bool` - Check if prefix exists
- `find_prefix(prefix) -> Vec<String>` - Find all words with prefix
- `remove(word) -> bool` - Remove word
- `count() -> usize` - Count words
- `is_empty() -> bool` - Check if empty
- `clear()` - Clear all words

### BloomFilter

Probabilistic membership testing with configurable false positive rate.

**Methods:**
- `new(expected_items, false_positive_rate) -> Self` - Create new filter
- `insert(item)` - Insert item
- `contains(item) -> bool` - Check if item might exist
- `clear()` - Clear all items
- `size() -> usize` - Get size in bits
- `num_hashes() -> usize` - Get number of hash functions
- `false_positive_rate() -> f64` - Estimate current FPR

## Examples

### Producer-Consumer with Bounded Queue

```rust
use agent::common::collections::BoundedQueue;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let queue = Arc::new(BoundedQueue::new(10));
    
    // Producer
    let producer_queue = queue.clone();
    tokio::spawn(async move {
        for i in 0..100 {
            producer_queue.push(i).await.unwrap();
            println!("Produced: {}", i);
        }
    });
    
    // Consumer
    let consumer_queue = queue.clone();
    tokio::spawn(async move {
        while let Ok(Some(item)) = consumer_queue.pop().await {
            println!("Consumed: {}", item);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });
}
```

### Autocomplete with Trie

```rust
use agent::common::collections::Trie;

fn autocomplete(dictionary: &Trie, prefix: &str) -> Vec<String> {
    let mut suggestions = dictionary.find_prefix(prefix);
    suggestions.sort();
    suggestions.truncate(10); // Limit to 10 suggestions
    suggestions
}

let mut dictionary = Trie::new();
dictionary.insert("apple");
dictionary.insert("application");
dictionary.insert("apply");
dictionary.insert("banana");

let suggestions = autocomplete(&dictionary, "app");
// Returns: ["apple", "application", "apply"]
```

### Deduplication with Bloom Filter

```rust
use agent::common::collections::BloomFilter;

struct Deduplicator {
    filter: BloomFilter,
    exact: std::collections::HashSet<String>,
}

impl Deduplicator {
    fn new(expected_items: usize) -> Self {
        Self {
            filter: BloomFilter::new(expected_items, 0.01),
            exact: std::collections::HashSet::new(),
        }
    }
    
    fn is_duplicate(&mut self, item: &str) -> bool {
        // Quick check with bloom filter
        if !self.filter.contains(item) {
            self.filter.insert(item);
            self.exact.insert(item.to_string());
            return false;
        }
        
        // Verify with exact set (handles false positives)
        if self.exact.contains(item) {
            return true;
        }
        
        self.exact.insert(item.to_string());
        false
    }
}
```

## Best Practices

### 1. Choose the Right Data Structure

```rust
// Use BoundedQueue for producer-consumer patterns
let queue = BoundedQueue::new(capacity);

// Use RingBuffer for fixed-size circular buffers
let buffer = RingBuffer::new(size);

// Use LruCache for caching with automatic eviction
let cache = LruCache::new(std::num::NonZeroUsize::new(max_size).unwrap());

// Use Trie for prefix-based string search
let trie = Trie::new();

// Use BloomFilter for space-efficient set membership
let filter = BloomFilter::new(expected, fpr);
```

### 2. Size Bounded Queues Appropriately

```rust
// Too small: frequent blocking
let queue = BoundedQueue::new(10);

// Too large: excessive memory
let queue = BoundedQueue::new(1_000_000);

// Just right: balance memory and throughput
let queue = BoundedQueue::new(1000);
```

### 3. Handle Bloom Filter False Positives

```rust
// Always verify positives from bloom filter
if filter.contains(item) {
    // Might be in set - verify with exact check
    if exact_set.contains(item) {
        // Definitely in set
    }
}
```

### 4. Use try_* Methods for Non-Blocking Operations

```rust
// Blocking (may wait)
queue.push(item).await?;

// Non-blocking (returns immediately)
match queue.try_push(item) {
    Ok(_) => { /* success */ }
    Err(item) => { /* queue full */ }
}
```

## Performance Characteristics

| Data Structure | Insert | Remove | Access | Space |
|----------------|--------|--------|--------|-------|
| BoundedQueue | O(1) | O(1) | N/A | O(n) |
| RingBuffer | O(1) | O(1) | O(1) | O(n) |
| LruCache | O(1) | O(1) | O(1) | O(n) |
| MinHeap/MaxHeap | O(log n) | O(log n) | O(1) | O(n) |
| Trie | O(m) | O(m) | O(m) | O(n×m) |
| BloomFilter | O(k) | N/A | O(k) | O(m) |

*where n = number of items, m = string length, k = number of hashes*

## Testing

```bash
# Run all tests
cargo test --lib common::collections

# Run specific module tests
cargo test --lib common::collections::bounded_queue
cargo test --lib common::collections::trie

# Run with all features
cargo test --all-features --lib common::collections
```

## Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["sync"] }
lru = "0.12"
```

## Related Modules

- **common::sync** - Synchronization primitives
- **common::cache** - Caching utilities

## License

See the root LICENSE file for licensing information.
